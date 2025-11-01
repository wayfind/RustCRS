# Week 3 实现启动 - API Key 服务

**日期**: 2025-10-30
**状态**: ✅ Week 2 完成, 🚀 Week 3 开始
**当前任务**: 实现 API Key 服务和认证系统

---

## ✅ 今日完成

### 1. 文档创建
- ✅ **API_DOCUMENTATION.md** - 完整的 API 参考文档 (60+ 端点)
- ✅ **API_TEST_CASES.md** - 综合测试套件 (60+ 测试用例)
- ✅ **run_tests.sh** - 自动化测试脚本
- ✅ **RUST_MIGRATION_PLAN.md** - 30 天详细迁移计划
- ✅ **PROGRESS.md** - 实时进度追踪
- ✅ **WEEK3_KICKOFF.md** - Week 3 启动文档

### 2. 数据模型实现
- ✅ **models/api_key.rs** (441 行) - API Key 完整数据模型
  - `ApiKey` 结构体 - 完整字段定义
  - `ApiKeyCreateOptions` - 创建选项
  - `ApiKeyPermissions` 枚举 - 权限系统
  - `ExpirationMode` 枚举 - 过期模式
  - `ActivationUnit` 枚举 - 激活单位
  - `ApiKeyUsageStats` - 使用统计
  - `ModelUsage` - 按模型统计
  - 4 个单元测试 ✅ 全部通过

---

## 📋 Week 3 实现计划

### Phase 1: API Key 服务核心 (Day 11-12)

#### 1.1 API Key 生成和哈希
```rust
// services/api_key.rs
impl ApiKeyService {
    pub fn new(redis: RedisPool, config: Settings) -> Self { }
    pub async fn generate_key(&self, options: ApiKeyCreateOptions) -> Result<(String, ApiKey)> { }
    fn hash_key(&self, key: &str) -> String { }
    fn generate_random_key(&self) -> String { }
}
```

**实现要点**:
- 生成随机 API Key (cr_ 前缀 + 32 字节随机)
- SHA-256 哈希存储
- UUID 生成 Key ID
- 时间戳设置 (created_at, updated_at)
- Redis 存储: `api_key:{id}` 和 `api_key_hash:{hash}`

#### 1.2 API Key 验证
```rust
impl ApiKeyService {
    pub async fn validate_key(&self, key: &str) -> Result<ApiKey> { }
    pub async fn check_permissions(&self, api_key: &ApiKey, service: &str) -> Result<bool> { }
    pub async fn check_rate_limits(&self, api_key: &ApiKey) -> Result<()> { }
    pub async fn check_model_restriction(&self, api_key: &ApiKey, model: &str) -> Result<()> { }
    pub async fn check_client_restriction(&self, api_key: &ApiKey, user_agent: &str) -> Result<()> { }
}
```

**实现要点**:
- 快速哈希查找 (O(1))
- 权限检查 (claude/gemini/openai/droid)
- 速率限制验证
- 模型黑名单检查
- 客户端限制检查
- 过期时间验证
- 激活模式处理

#### 1.3 CRUD 操作
```rust
impl ApiKeyService {
    pub async fn create_key(&self, options: ApiKeyCreateOptions) -> Result<(String, ApiKey)> { }
    pub async fn get_key(&self, key_id: &str) -> Result<ApiKey> { }
    pub async fn get_all_keys(&self, include_deleted: bool) -> Result<Vec<ApiKey>> { }
    pub async fn update_key(&self, key_id: &str, updates: ApiKeyUpdateOptions) -> Result<ApiKey> { }
    pub async fn delete_key(&self, key_id: &str, deleted_by: &str) -> Result<()> { }
    pub async fn restore_key(&self, key_id: &str, restored_by: &str) -> Result<ApiKey> { }
    pub async fn permanent_delete(&self, key_id: &str) -> Result<()> { }
}
```

**Redis 数据结构**:
```
api_key:{id}           -> JSON 序列化的 ApiKey
api_key_hash:{hash}    -> key_id (快速查找)
user:{user_id}:keys    -> Set[key_id] (用户的所有 Keys)
```

### Phase 2: 使用统计和成本追踪 (Day 12-13)

#### 2.1 使用记录
```rust
impl ApiKeyService {
    pub async fn record_usage(&self, key_id: &str, usage: UsageRecord) -> Result<()> { }
    pub async fn record_cost(&self, key_id: &str, cost: f64, model: &str) -> Result<()> { }
    pub async fn get_usage_stats(&self, key_id: &str, options: StatsOptions) -> Result<ApiKeyUsageStats> { }
}
```

**UsageRecord 结构**:
```rust
pub struct UsageRecord {
    pub request_id: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub cost: f64,
    pub account_type: String,
    pub account_id: String,
    pub timestamp: DateTime<Utc>,
}
```

**Redis 统计结构**:
```
api_key_usage:{key_id}                    -> 累计统计
usage:daily:{date}:{key_id}:{model}       -> 按日期、Key、模型
usage:model:{key_id}:{model}              -> 按模型累计
```

#### 2.2 成本限制检查
```rust
impl ApiKeyService {
    pub async fn check_daily_cost_limit(&self, key_id: &str) -> Result<bool> { }
    pub async fn check_total_cost_limit(&self, key_id: &str) -> Result<bool> { }
    pub async fn check_weekly_opus_cost(&self, key_id: &str) -> Result<bool> { }
}
```

### Phase 3: 认证中间件 (Day 13-14)

#### 3.1 中间件实现
```rust
// middleware/auth.rs
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

pub async fn authenticate_api_key(
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. 提取 Authorization header
    // 2. 验证 API Key
    // 3. 检查权限
    // 4. 检查速率限制
    // 5. 附加到请求扩展
    // 6. 继续处理
}
```

#### 3.2 请求扩展
```rust
pub struct ApiKeyContext {
    pub api_key: ApiKey,
    pub key_id: String,
}

// 在路由中使用
pub async fn handler(
    Extension(ctx): Extension<ApiKeyContext>,
) -> Result<Json<Response>> {
    // 访问 ctx.api_key
}
```

### Phase 4: 集成测试 (Day 14-15)

#### 4.1 单元测试
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_generate_api_key() { }

    #[tokio::test]
    async fn test_validate_api_key() { }

    #[tokio::test]
    async fn test_check_permissions() { }

    #[tokio::test]
    async fn test_rate_limiting() { }

    #[tokio::test]
    async fn test_usage_recording() { }
}
```

#### 4.2 集成测试
```bash
# 测试 API Key 生成
cargo test --test integration_tests test_api_key_generation

# 测试认证流程
cargo test --test integration_tests test_authentication_flow

# 测试权限系统
cargo test --test integration_tests test_permission_system
```

---

## 🎯 实现优先级

### P0 - 核心功能 (必须)
- [x] API Key 数据模型
- [ ] API Key 生成和哈希
- [ ] API Key 验证
- [ ] 基本 CRUD
- [ ] 认证中间件

### P1 - 重要功能 (Week 3)
- [ ] 权限检查
- [ ] 速率限制
- [ ] 使用统计
- [ ] 成本追踪

### P2 - 高级功能 (Week 4)
- [ ] 模型限制
- [ ] 客户端限制
- [ ] 过期处理
- [ ] 软删除/恢复

---

## 📊 技术决策

### 依赖选择

**现有依赖** (已在 Cargo.toml):
- ✅ sha2 = "0.10" - SHA-256 哈希
- ✅ uuid = "1" - UUID 生成
- ✅ chrono = "0.4" - 时间处理
- ✅ serde = "1" - 序列化
- ✅ redis = "0.24" - Redis 客户端
- ✅ deadpool-redis = "0.14" - 连接池

**需要添加**:
- rand = "0.8" - 安全随机数生成

### Redis Key 设计

```
# API Key 数据
api_key:{uuid}                 # ApiKey JSON
api_key_hash:{sha256}          # key_id 映射

# 用户关联
user:{user_id}:keys            # Set[key_id]

# 使用统计
api_key_usage:{key_id}         # 累计统计 JSON
usage:daily:{date}:{key_id}:{model}  # 日统计

# 速率限制
rate_limit:{key_id}:{window}   # 请求计数
rate_limit_cost:{key_id}:{window}  # 成本累计

# 并发控制
concurrency:{key_id}           # Sorted Set (活跃请求)
```

### 错误处理策略

```rust
// 添加新的错误类型到 utils/error.rs
pub enum AppError {
    // ... 现有错误

    // API Key 相关
    ApiKeyNotFound(String),
    ApiKeyInvalid(String),
    ApiKeyExpired(String),
    ApiKeyInactive(String),
    ApiKeyPermissionDenied(String),
    ApiKeyRateLimitExceeded(String),
    ApiKeyCostLimitExceeded(String),
    ApiKeyModelRestricted(String),
    ApiKeyClientRestricted(String),
}
```

---

## 🧪 测试策略

### 单元测试覆盖

**数据模型** (✅ 已完成):
- [x] 默认值测试
- [x] 权限检查逻辑
- [x] 序列化/反序列化

**API Key 服务**:
- [ ] Key 生成唯一性
- [ ] 哈希一致性
- [ ] 验证逻辑
- [ ] CRUD 操作
- [ ] 权限检查
- [ ] 速率限制
- [ ] 使用统计
- [ ] 成本计算

**认证中间件**:
- [ ] Header 提取
- [ ] 验证流程
- [ ] 错误处理
- [ ] 请求扩展

### 集成测试场景

1. **完整认证流程**
   - 生成 Key → 验证 → 发送请求 → 记录使用 → 查询统计

2. **权限测试**
   - Claude 权限访问 Gemini API (拒绝)
   - All 权限访问所有 API (允许)

3. **速率限制测试**
   - 超过请求限制 → 429 错误
   - 窗口重置后恢复

4. **成本限制测试**
   - 达到每日限制 → 拒绝请求
   - 重置时间测试

---

## 📝 下一步行动

### 立即执行 (今日)
1. ✅ 创建 API Key 数据模型
2. ✅ 编写模型测试
3. ✅ 创建项目文档

### 明日计划 (Day 11)
1. 实现 API Key Service 基础框架
2. 实现 Key 生成和哈希功能
3. 实现 Key 验证逻辑
4. 添加 rand crate 依赖
5. 编写生成和验证的测试

### 本周目标 (Day 11-15)
- 完成 API Key 服务所有核心功能
- 完成认证中间件
- 通过所有单元测试和集成测试
- 文档更新

---

## 🚧 风险和挑战

### 技术风险
1. **Redis 性能** - 高并发下的哈希查找性能
   - 缓解: 使用本地 LRU 缓存

2. **速率限制精度** - 分布式环境下的限流
   - 缓解: Redis 原子操作 + Lua 脚本

3. **成本计算准确性** - 实时 token 捕获
   - 缓解: 完整的流式响应解析

### 时间风险
1. **复杂度低估** - 功能比预期复杂
   - 缓解: MVP 优先,渐进增强

2. **测试不足** - 边界情况未覆盖
   - 缓解: TDD 方法,先写测试

---

## 📚 参考资料

### Node.js 实现
- `/home/david/prj/claude-relay-service/src/services/apiKeyService.js` (1765 行)
- 关键方法: generateApiKey, validateApiKey, recordUsage

### Rust 最佳实践
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Axum Middleware Guide](https://docs.rs/axum/latest/axum/middleware/index.html)
- [Redis Rust Client](https://docs.rs/redis/latest/redis/)

---

**维护者**: Rust Migration Team
**最后更新**: 2025-10-30 19:30
**下次同步**: Day 11 结束时
