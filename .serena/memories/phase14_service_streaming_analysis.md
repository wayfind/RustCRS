# Phase 14: Service Layer Streaming Implementation - Analysis

**Date**: 2025-10-31
**Status**: 🔄 In Progress

## 目标

实现 BedrockRelayService 和 GeminiRelayService 的服务层流式支持，使 Phase 13 的路由层流式功能可以端到端工作。

## 当前状态

### Claude Streaming (✅ 已完成 - 参考实现)

**文件**: `src/services/claude_relay.rs:512-723`

**实现模式**:
1. 使用 AccountScheduler 选择账户
2. 验证 token 有效性
3. 增加并发计数
4. 创建 mpsc::channel (buffer=100)
5. 启动异步任务处理流式响应
6. 在任务中:
   - 发送HTTP请求 (stream: true)
   - 读取 bytes_stream
   - 逐块转发原始数据 (StreamChunk::Data)
   - 解析 SSE 事件提取 usage 数据
   - 最终发送 StreamChunk::Usage
   - 减少并发计数
7. 返回 Receiver端给路由层

**关键数据结构**:
```rust
pub enum StreamChunk {
    Data(Bytes),         // 原始SSE数据
    Usage(Usage),        // 最终usage数据
}

pub struct Usage {
    input_tokens: i32,
    output_tokens: i32,
    cache_creation_input_tokens: Option<i32>,
    cache_read_input_tokens: Option<i32>,
}
```

### Bedrock Streaming (❌ 未实现)

**文件**: `src/services/bedrock_relay.rs:324-331`

**当前实现**:
```rust
async fn relay_request_stream(
    &self,
    _request: RelayRequest,
) -> Result<mpsc::Receiver<Result<GenericStreamChunk>>> {
    Err(AppError::BadRequest(
        "Bedrock streaming not yet implemented".to_string(),
    ))
}
```

**需要实现的功能**:
1. AWS Bedrock API流式调用 (使用 AWS SDK或HTTP直接调用)
2. 处理 Bedrock 的流式响应格式
3. 转换为 GenericStreamChunk
4. Usage数据提取和聚合

**Bedrock API 特性**:
- 使用 AWS SDK bedrockruntime client
- 模型调用格式: `invoke_model_with_response_stream`
- 响应格式: 事件流 (Event Stream)
- Usage 字段与Claude类似

### Gemini Streaming (❌ 未实现)

**文件**: `src/services/gemini_relay.rs:334-342`

**当前实现**:
```rust
async fn relay_request_stream(
    &self,
    _request: RelayRequest,
) -> Result<mpsc::Receiver<Result<GenericStreamChunk>>> {
    Err(AppError::BadRequest(
        "Gemini streaming not yet implemented".to_string(),
    ))
}
```

**需要实现的功能**:
1. Google Gemini API 流式调用
2. 处理 Gemini 的 SSE 流式响应
3. 转换为 GenericStreamChunk
4. Usage 数据提取 (tokenCount 字段)

**Gemini API 特性**:
- 端点: `/v1beta/models/{model}:streamGenerateContent`
- 使用 SSE (Server-Sent Events)
- 响应格式: JSON chunks with `candidates` array
- Usage 在 `usageMetadata` 字段

## 实现策略

### 共同模式 (参考 Claude实现)

1. **并发管理**:
   ```rust
   // 增加并发计数
   let request_id = uuid::Uuid::new_v4().to_string();
   self.account_scheduler
       .increment_concurrency(&account_id, &request_id, None)
       .await?;
   
   // 在异步任务结束时减少
   account_scheduler
       .decrement_concurrency(&account_id, &request_id)
       .await;
   ```

2. **Channel 创建**:
   ```rust
   let (tx, rx) = mpsc::channel::<Result<GenericStreamChunk>>(100);
   ```

3. **异步任务模式**:
   ```rust
   tokio::spawn(async move {
       let result = Self::process_stream_response(...).await;
       
       // 清理并发计数
       account_scheduler.decrement_concurrency(...).await;
       
       // 处理错误
       if let Err(e) = result {
           tx.send(Err(AppError::UpstreamError(e.to_string()))).await;
       }
   });
   ```

4. **SSE 解析模式**:
   ```rust
   let mut stream = response.bytes_stream();
   let mut buffer = String::new();
   let mut accumulated_usage = UsageStats::default();
   
   while let Some(chunk_result) = stream.next().await {
       match chunk_result {
           Ok(chunk) => {
               // 1. 转发原始数据
               tx.send(Ok(GenericStreamChunk::Data(chunk.clone()))).await;
               
               // 2. 解析事件提取usage
               let chunk_str = String::from_utf8_lossy(&chunk);
               buffer.push_str(&chunk_str);
               // ... parse SSE events
           }
           Err(e) => {
               tx.send(Err(AppError::UpstreamError(e.to_string()))).await;
               break;
           }
       }
   }
   
   // 3. 发送最终usage
   tx.send(Ok(GenericStreamChunk::Usage(accumulated_usage))).await;
   ```

### Bedrock 特殊考虑

**AWS SDK 集成**:
- 需要配置 AWS credentials
- 使用 `aws-sdk-bedrockruntime` crate
- 调用 `invoke_model_with_response_stream`

**流式响应格式**:
```rust
// AWS Event Stream format
{
    "chunk": {
        "bytes": "base64-encoded-data"
    }
}

// 或者直接JSON流
{
    "type": "content_block_delta",
    "delta": { "text": "..." }
}
```

**实现选项**:
1. **选项A**: 使用 AWS SDK (推荐)
   - 优点: 官方支持，处理认证和签名
   - 缺点: 增加依赖，学习曲线

2. **选项B**: HTTP直接调用
   - 优点: 与现有模式一致
   - 缺点: 需要手动处理 AWS Signature V4

### Gemini 特殊考虑

**API 端点**:
```
POST https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent
```

**请求格式**:
```json
{
  "contents": [...]
}
```

**响应格式** (SSE):
```
data: {"candidates":[{"content":{"parts":[{"text":"Hello"}]}}],"usageMetadata":{"promptTokenCount":5,"candidatesTokenCount":1}}

data: {"candidates":[{"content":{"parts":[{"text":" world"}]}}]}

data: [DONE]
```

**Usage 提取**:
```rust
// 从 usageMetadata 提取
{
    "promptTokenCount": input_tokens,
    "candidatesTokenCount": output_tokens,
    "totalTokenCount": total_tokens
}
```

## GenericStreamChunk 结构

```rust
pub enum GenericStreamChunk {
    Data(Bytes),              // 原始响应数据
    Usage(UsageStats),        // 使用统计
    Error(String),            // 错误信息
}

pub struct UsageStats {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_creation_tokens: Option<i32>,
    pub cache_read_tokens: Option<i32>,
    pub total_tokens: i32,
}
```

## 实现顺序

### Phase 14.1: Gemini Streaming (优先)
**原因**: 
- Gemini 使用标准SSE，与Claude模式接近
- 无需AWS SDK依赖
- 可以快速验证通用streaming模式

**步骤**:
1. 实现 `GeminiRelayService::relay_request_stream()`
2. 实现 `GeminiRelayService::process_stream_response()`
3. 实现 Gemini SSE 解析和usage提取
4. 编写集成测试
5. 端到端测试 (路由层→服务层→实际API)

### Phase 14.2: Bedrock Streaming
**原因**:
- 需要AWS SDK集成决策
- 可能需要更多基础设施配置

**步骤**:
1. 决定实现方案 (AWS SDK vs HTTP)
2. 实现 `BedrockRelayService::relay_request_stream()`
3. 实现 Bedrock 流式响应处理
4. Usage 数据提取
5. 集成测试

## 测试策略

### 单元测试
- SSE 事件解析 (parse_sse_line)
- Usage 数据提取 (extract_usage_from_event)
- 请求格式转换

### 集成测试 (无真实API)
- Channel 通信验证
- 并发计数管理
- 错误处理路径

### 端到端测试 (需要真实账户)
- 实际流式请求
- Usage 数据准确性
- 中断恢复

## 文件变更清单

### 需要修改的文件
1. `src/services/gemini_relay.rs` - 实现 Gemini 流式
2. `src/services/bedrock_relay.rs` - 实现 Bedrock 流式
3. `tests/gemini_streaming_service_test.rs` - 新增 Gemini 服务层测试
4. `tests/bedrock_streaming_service_test.rs` - 新增 Bedrock 服务层测试

### 可能需要的新文件
- Gemini SSE parser utilities
- Bedrock event stream parser utilities

## 成功标准

### Gemini Streaming ✅
- [ ] `relay_request_stream()` 实现完成
- [ ] SSE 解析正确
- [ ] Usage 数据准确
- [ ] 并发计数管理正确
- [ ] 集成测试通过
- [ ] 路由层→服务层端到端工作

### Bedrock Streaming ✅
- [ ] `relay_request_stream()` 实现完成
- [ ] AWS 流式响应处理正确
- [ ] Usage 数据准确
- [ ] 并发计数管理正确
- [ ] 集成测试通过
- [ ] 路由层→服务层端到端工作

## 下一步行动

1. ✅ 完成分析文档 (当前文件)
2. ⏳ 实现 Gemini streaming (Phase 14.1)
3. ⏳ 实现 Bedrock streaming (Phase 14.2)
4. ⏳ 编写服务层流式测试
5. ⏳ 更新 Phase 14 完成文档
