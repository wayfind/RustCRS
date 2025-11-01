# Phase 12: 账户类型路由需求分析

## 1. 现有 Relay Services（Rust）

### 已实现的 4 个 Relay Services

1. **ClaudeRelayService** (`src/services/claude_relay.rs:158`)
   - 处理 Claude Official API 转发
   - Constructor: `new(config, http_client, redis, account_service, scheduler)`

2. **BedrockRelayService** (`src/services/bedrock_relay.rs:117`)
   - 处理 AWS Bedrock API 转发
   - Constructor: `new(config, http_client, redis, account_service, scheduler)`
   - 包含 model_mapping (HashMap) 用于模型名称映射

3. **GeminiRelayService** (`src/services/gemini_relay.rs:119`)
   - 处理 Google Gemini API 转发
   - Constructor: `new(config, http_client, redis, account_service, scheduler)`

4. **OpenAIRelayService** (`src/services/openai_relay.rs:95`)
   - 处理 OpenAI Responses (Codex) API 转发
   - Constructor: `new(config, http_client, redis, account_service, scheduler)`

**统一 Constructor 签名**:
```rust
pub fn new(
    config: XxxRelayConfig,
    http_client: Arc<Client>,
    redis: Arc<RedisPool>,
    account_service: Arc<ClaudeAccountService>,
    account_scheduler: Arc<AccountScheduler>,
) -> Self
```

## 2. SchedulerAccountVariant 定义

**位置**: `src/services/unified_claude_scheduler.rs:28`

```rust
pub enum SchedulerAccountVariant {
    ClaudeOfficial,    // Claude 官方 API
    ClaudeConsole,     // Claude Console
    Bedrock,           // AWS Bedrock
    Ccr,               // CCR
}
```

**重要方法**:
- `from_platform(platform: Platform) -> Self` - 从 Platform enum 转换
- `from_str(s: &str) -> Option<Self>` - 从字符串解析
- `as_str(&self) -> &str` - 转为字符串

## 3. SelectedAccount 结构

```rust
pub struct SelectedAccount {
    pub account_id: String,
    pub account_variant: SchedulerAccountVariant,
    pub account: ClaudeAccount,
}
```

unified schedulers 返回这个结构，包含:
- `account_id`: 账户ID
- `account_variant`: 账户变体类型（ClaudeOfficial/Console/Bedrock/Ccr）
- `account`: 完整的账户信息

## 4. 缺失的 Relay Services

### Console 和 CCR 账户处理

**发现**: 
- ❌ 没有独立的 `ConsoleRelayService` 文件
- ❌ 没有独立的 `CcrRelayService` 文件

**推断**:
- `ClaudeConsole` 和 `Ccr` 变体可能复用 `ClaudeRelayService`
- Node.js 版本中有 `claudeConsoleRelayService.js` 和 `ccrRelayService.js`，但 Rust 版本尚未实现

**解决方案**:
```rust
match selected.account_variant {
    SchedulerAccountVariant::ClaudeOfficial => claude_relay_service.relay_request(...).await?,
    SchedulerAccountVariant::ClaudeConsole => claude_relay_service.relay_request(...).await?, // 复用
    SchedulerAccountVariant::Bedrock => bedrock_relay_service.relay_request(...).await?,
    SchedulerAccountVariant::Ccr => claude_relay_service.relay_request(...).await?, // 复用
}
```

## 5. main.rs 当前初始化状态

**已初始化** (lines 105-122):
- `ClaudeRelayService` → `relay_service`
- `GeminiRelayService` → `gemini_service`

**未初始化**:
- ❌ `BedrockRelayService`
- ❌ `OpenAIRelayService`

## 6. 实现计划

### 6.1 main.rs 初始化所有服务

```rust
// Bedrock Relay Service
let bedrock_config = BedrockRelayConfig::default();
let bedrock_service = Arc::new(BedrockRelayService::new(
    bedrock_config,
    reqwest_client.clone(),
    redis_arc.clone(),
    account_service.clone(),
    scheduler.clone(),
));

// OpenAI Relay Service (for OpenAI Responses)
let openai_config = OpenAIRelayConfig::default();
let openai_service = Arc::new(OpenAIRelayService::new(
    openai_config,
    reqwest_client.clone(),
    redis_arc.clone(),
    account_service.clone(),
    scheduler.clone(),
));
```

### 6.2 更新 ApiState 包含所有服务

```rust
pub struct ApiState {
    pub redis: Arc<RedisPool>,
    pub settings: Arc<Settings>,
    pub account_service: Arc<ClaudeAccountService>,
    pub api_key_service: Arc<ApiKeyService>,
    pub scheduler: Arc<AccountScheduler>,
    pub relay_service: Arc<ClaudeRelayService>,
    pub bedrock_service: Arc<BedrockRelayService>,  // NEW
    pub unified_claude_scheduler: Arc<UnifiedClaudeScheduler>,
}
```

### 6.3 在路由中实现账户类型路由

```rust
// src/routes/api.rs - handle_messages
let selected = state.unified_claude_scheduler
    .select_account(&api_key, session_hash.as_deref(), Some(&model))
    .await?;

// 根据账户类型选择转发服务
let relay_response = match selected.account_variant {
    SchedulerAccountVariant::ClaudeOfficial => {
        state.relay_service.relay_request(...).await?
    },
    SchedulerAccountVariant::ClaudeConsole => {
        state.relay_service.relay_request(...).await?
    },
    SchedulerAccountVariant::Bedrock => {
        state.bedrock_service.relay_request(...).await?
    },
    SchedulerAccountVariant::Ccr => {
        state.relay_service.relay_request(...).await?
    },
};
```

### 6.4 Gemini 和 OpenAI 路由

**Gemini**: 不需要多类型路由（只有 Gemini 一种类型）
**OpenAI**: 不需要多类型路由（只有 OpenAI Responses 一种类型）

## 7. 需要修改的文件

1. **src/main.rs** - 初始化 BedrockRelayService 和 OpenAIRelayService
2. **src/routes/api.rs** - 添加 bedrock_service 到 ApiState，实现账户类型路由
3. **src/routes/mod.rs** - 更新 ApiState 导出
4. **tests/api_routes_integration_test.rs** - 更新测试的 ApiState 初始化

## 8. 下一步行动

✅ **分析完成** - 已收集所有必要信息

**待办**:
1. 修改 main.rs 初始化 Bedrock/OpenAI services
2. 更新 ApiState 结构添加 bedrock_service
3. 实现 handle_messages 中的账户类型路由
4. 更新集成测试
5. 运行测试验证功能
