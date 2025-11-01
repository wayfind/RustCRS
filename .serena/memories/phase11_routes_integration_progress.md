# Phase 11: 路由和 API 端点集成进度

## 已完成

### 1. 会话哈希生成（session_helper）✅
- 文件：`rust/src/utils/session_helper.rs` (292 lines)
- 测试：8/8 通过
- 功能：
  - 优先级 1: metadata.user_id 中的 session ID 提取
  - 优先级 2: 带 cache_control ephemeral 的内容
  - 优先级 3: system 内容
  - 优先级 4: 第一条消息内容
  - SHA256 哈希计算（取前32字符）
  - UUID 验证
  - 会话哈希格式验证

### 2. 路由结构分析 ✅

**Node.js 集成模式**:
```javascript
// 生成会话哈希
const sessionHash = sessionHelper.generateSessionHash(req.body)

// 调用统一调度器
const { accountId, accountType } = await unifiedClaudeScheduler.selectAccountForApiKey(
  req.apiKey, 
  sessionHash, 
  requestedModel
)

// 根据账户类型转发
if (accountType === 'claude-official') {
  await claudeRelayService.relay(...)
} else if (accountType === 'claude-console') {
  await claudeConsoleRelayService.relay(...)
}
```

**Rust 现有路由**:
- `src/routes/api.rs` - Claude API 路由（已有基础框架）
- `src/routes/gemini.rs` - Gemini 路由
- `src/routes/openai.rs` - OpenAI 路由
- `src/routes/health.rs` - 健康检查
- `src/routes/mod.rs` - 路由模块导出

**ApiState 结构** (需要扩展):
```rust
pub struct ApiState {
    pub redis: Arc<RedisPool>,
    pub settings: Arc<Settings>,
    pub account_service: Arc<ClaudeAccountService>,
    pub api_key_service: Arc<ApiKeyService>,
    pub scheduler: Arc<AccountScheduler>,
    pub relay_service: Arc<ClaudeRelayService>,
    // 需要添加:
    // pub unified_claude_scheduler: Arc<UnifiedClaudeScheduler>,
    // pub unified_gemini_scheduler: Arc<UnifiedGeminiScheduler>,
    // pub unified_openai_scheduler: Arc<UnifiedOpenAIScheduler>,
}
```

## 下一步（正在进行）

### 3. 集成 UnifiedClaudeScheduler 到 Claude 路由 (进行中)

**集成步骤**:
1. 扩展 ApiState 添加 unified_claude_scheduler
2. 修改 handle_messages 使用调度器：
   - 生成 session_hash: `generate_session_hash(&request_body_json)`
   - 调用调度器: `scheduler.select_account(api_key, session_hash, model)`
   - 获取账户类型: `SelectedAccount { account_type, ... }`
   - 根据账户类型选择 relay service

3. 处理错误：
   - 专属账户限流: 429 错误
   - 无可用账户: 503 错误
   - 权限错误: 403 错误

**需要实现的路由逻辑**:
```rust
// 1. 生成会话哈希
let request_body_json = serde_json::to_value(&request)?;
let session_hash = generate_session_hash(&request_body_json);

// 2. 调用调度器
let selected = state.unified_claude_scheduler
    .select_account(&api_key, session_hash.as_deref(), Some(&request.model))
    .await?;

// 3. 根据账户类型转发
match selected.account_type.as_str() {
    "claude-official" => {
        // 使用 ClaudeRelayService
    }
    "claude-console" => {
        // 使用 ClaudeConsoleRelayService
    }
    "bedrock" => {
        // 使用 BedrockRelayService
    }
    _ => {
        return Err(AppError::InternalError(format!("Unknown account type: {}", selected.account_type)));
    }
}
```

## 待完成

### 4. 集成 UnifiedGeminiScheduler 到 Gemini 路由
### 5. 集成 UnifiedOpenAIScheduler 到 OpenAI 路由
### 6. 测试完整的请求流程
### 7. 编写集成测试

## 技术要点

- **会话粘性**: 通过 session_hash 确保同一会话使用同一账户
- **调度器隔离**: 每个平台使用独立的统一调度器
- **错误处理**: 统一的错误码映射（429/503/403）
- **并发控制**: 委托给 AccountScheduler
- **速率限制**: 通过调度器自动处理
