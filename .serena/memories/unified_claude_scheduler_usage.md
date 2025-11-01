# UnifiedClaudeScheduler 使用文档

## 概述

`UnifiedClaudeScheduler` 是一个智能多账户调度器，支持跨 4 种 Claude 账户类型（Official, Console, Bedrock, CCR）的统一调度和负载均衡。

**文件**: `src/services/unified_claude_scheduler.rs` (614 lines)

**依赖**: `src/utils/model_helper.rs` (220 lines)

## 核心功能

### 1. 多账户类型支持

```rust
pub enum SchedulerAccountVariant {
    ClaudeOfficial,   // Claude 官方账户
    ClaudeConsole,    // Claude Console 账户
    Bedrock,          // AWS Bedrock 账户
    Ccr,              // CCR 账户
}
```

**优先级顺序**: Official > Console > Bedrock > CCR

### 2. 粘性会话管理

**特性**:
- Session Hash → Account Binding
- TTL: 1 hour (可配置)
- 自动过期清理
- Redis 存储

**方法**:
```rust
async fn get_session_mapping(&self, session_hash: &str) -> Result<Option<SessionMapping>>
async fn set_session_mapping(&self, session_hash: &str, account_id: &str, account_variant: &str) -> Result<()>
async fn delete_session_mapping(&self, session_hash: &str) -> Result<()>
```

### 3. 模型兼容性检查

**规则**:
- **ClaudeOfficial**: 只支持 Claude 官方模型，Opus 需要 Max 订阅
- **ClaudeConsole/CCR**: 支持所有 Claude 模型（TODO: 从 ext_info 解析 supportedModels）
- **Bedrock**: 支持所有 Claude 模型

**Vendor 前缀**:
- `ccr:claude-3-5-sonnet` → 优先选择 CCR 账户
- `claude-3-5-sonnet` → 按标准优先级选择

### 4. Rate Limiting

**Redis Key**: `rate_limit:scheduler:{account_id}`

**方法**:
```rust
pub async fn is_account_rate_limited(&self, account_id: &str) -> Result<bool>
pub async fn mark_account_rate_limited(&self, account_id: &str, duration_seconds: Option<i64>) -> Result<()>
pub async fn remove_account_rate_limit(&self, account_id: &str) -> Result<()>
```

**默认 TTL**: 5 minutes

### 5. 并发控制

**方法**:
```rust
pub async fn increment_account_concurrency(&self, account_id: &str, request_id: &str, ttl_seconds: Option<u64>) -> Result<()>
pub async fn decrement_account_concurrency(&self, account_id: &str, request_id: &str) -> Result<()>
pub async fn get_account_concurrency(&self, account_id: &str) -> Result<usize>
pub async fn is_account_concurrency_exceeded(&self, account_id: &str, max_concurrent: usize) -> Result<bool>
```

## 使用示例

### 基本用法

```rust
use std::sync::Arc;
use crate::services::{UnifiedClaudeScheduler, ClaudeAccountService, AccountScheduler};
use crate::RedisPool;

// 1. 创建调度器
let scheduler = Arc::new(UnifiedClaudeScheduler::new(
    account_service,
    account_scheduler,
    redis,
));

// 2. 选择账户
let selected = scheduler.select_account(
    Some("session_hash_123"),  // 会话哈希（粘性会话）
    Some("claude-3-5-sonnet-20241022"),  // 请求的模型
).await?;

println!("Selected account: {} (variant: {:?})", 
    selected.account.name, 
    selected.account_variant
);
```

### 完整请求流程

```rust
// 1. 选择账户
let selected = scheduler.select_account(session_hash, model).await?;
let account_id = &selected.account_id;
let request_id = uuid::Uuid::new_v4().to_string();

// 2. 增加并发计数
scheduler.on_request_start(account_id, &request_id, Some(600)).await?;

// 3. 执行请求
let result = match execute_request(&selected.account).await {
    Ok(response) => {
        // 4a. 成功：移除 rate limit（如果之前被限制）
        scheduler.on_request_success(account_id).await?;
        Ok(response)
    },
    Err(e) if is_rate_limit_error(&e) => {
        // 4b. 429错误：标记 rate limited
        scheduler.on_rate_limit_error(account_id, Some(300)).await?;
        Err(e)
    },
    Err(e) => Err(e),
};

// 5. 减少并发计数
scheduler.on_request_end(account_id, &request_id).await?;

result
```

### CCR Vendor 前缀

```rust
// 使用 ccr: 前缀优先选择 CCR 账户
let selected = scheduler.select_account(
    None,
    Some("ccr:claude-3-5-sonnet"),
).await?;

assert_eq!(selected.account_variant, SchedulerAccountVariant::Ccr);
```

### 账户可用性检查

```rust
// 综合检查：active + schedulable + rate limit + concurrency
let is_available = scheduler.is_account_available_for_scheduling(
    &account,
    Some(5),  // 最大并发数
).await?;

if is_available {
    // 账户可用
} else {
    // 账户不可用（被限流或并发超限）
}
```

## 账户选择算法

```
1. 解析 vendor 前缀
   ccr:model-name → 优先选择 CCR 账户
   
2. 检查粘性会话
   session_hash → 尝试使用绑定的账户
   
3. 获取所有可用账户
   is_active && schedulable
   
4. 按优先级顺序选择
   for variant in [Official, Console, Bedrock, CCR]:
       candidates = filter by variant + model support
       for candidate in sorted(candidates by priority):
           if is_available (rate_limit + concurrency):
               return candidate
               
5. 创建粘性会话
   session_hash → account_id (TTL: 1h)
```

## 错误处理

```rust
// NoAvailableAccounts
Err(AppError::NoAvailableAccounts("No Claude accounts available"))
Err(AppError::NoAvailableAccounts("No suitable account for model: claude-opus-4"))

// RedisError
Err(AppError::RedisError("Connection failed"))

// InternalError
Err(AppError::InternalError("JSON serialization error: ..."))
```

## 配置选项

```rust
impl UnifiedClaudeScheduler {
    pub fn new(...) -> Self {
        Self {
            session_mapping_prefix: "sticky_session:".to_string(),
            sticky_session_ttl_seconds: 3600,  // 1 hour
            rate_limit_prefix: "rate_limit:scheduler:".to_string(),
            rate_limit_ttl_seconds: 300,  // 5 minutes
            ...
        }
    }
}
```

**可自定义字段**:
- `session_mapping_prefix`: Redis 粘性会话前缀
- `sticky_session_ttl_seconds`: 会话 TTL（秒）
- `rate_limit_prefix`: Redis rate limit 前缀
- `rate_limit_ttl_seconds`: Rate limit TTL（秒）

## Redis 数据结构

### 粘性会话

```
Key: sticky_session:{session_hash}
Value: JSON
{
  "account_id": "uuid",
  "account_variant": "claude-official",
  "created_at": 1234567890,
  "expires_at": 1234571490
}
TTL: sticky_session_ttl_seconds
```

### Rate Limit

```
Key: rate_limit:scheduler:{account_id}
Value: "1"
TTL: rate_limit_ttl_seconds
```

### 并发计数

由 `AccountScheduler` 管理：
```
Key: concurrency:{account_id}
Value: Redis Sorted Set {request_id: timestamp}
```

## 测试

```bash
# 运行单元测试
cargo test --lib unified_claude_scheduler

# 测试覆盖
# - test_scheduler_account_variant_conversion: 类型转换
# - test_from_platform: Platform → SchedulerAccountVariant 转换
```

## 性能优化

1. **Redis 连接池**: 复用连接，减少连接开销
2. **批量过滤**: 先按 variant 和 model 过滤，再异步检查可用性
3. **优先级排序**: 一次排序，按序检查
4. **粘性会话**: 减少重复选择开销
5. **并发控制**: Redis Sorted Set 自动过期清理

## 已知限制

1. **supportedModels 未实现**: Console 和 CCR 账户的模型限制需要从 `ext_info` 解析
2. **max_concurrent 硬编码**: 并发限制当前未从账户配置读取，传 `None` 跳过检查
3. **list_accounts 限制**: 使用 offset=0, limit=1000，超过 1000 个账户会被截断

## 后续改进

1. ✅ 从 `ext_info` 解析 `supportedModels`
2. ✅ 添加账户级别的 `max_concurrent_requests` 字段
3. ✅ 实现账户组支持（Account Groups）
4. ✅ 添加账户健康检查和自动恢复
5. ✅ 实现调度统计和监控指标
