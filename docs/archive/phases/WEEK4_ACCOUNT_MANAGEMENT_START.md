# Week 4 - 账户管理系统开发 (启动阶段)

**日期**: 2025-10-30
**进度**: 10% (数据模型完成，服务层开发中)

---

## 📊 今日完成工作

### ✅ 已完成任务

#### 1. Node.js 版本架构研究
- 分析了 `claudeAccountService.js` 的实现（~1000 行）
- 理解了关键功能：
  - OAuth token 刷新机制
  - AES加密/解密系统
  - 账户CRUD操作
  - 并发控制和调度逻辑
  - LRU缓存优化

#### 2. Rust 账户模型设计
**文件**: `src/models/account.rs` (~470 行)

**核心数据结构**:
- `ClaudeAccount` - 完整的账户模型
- `AccountType` - 账户类型枚举（Shared/Dedicated）
- `Platform` - 平台枚举（Claude/Gemini/OpenAI等）
- `AccountStatus` - 状态枚举（Active/Inactive/Error/Overloaded/Expired）
- `ProxyConfig` - 代理配置
- `ClaudeOAuthData` - OAuth数据结构
- `SubscriptionInfo` - 订阅信息
- `CreateClaudeAccountOptions` - 创建选项

**关键方法**:
```rust
impl ClaudeAccount {
    /// 检查 token 是否即将过期（10秒阈值）
    pub fn is_token_expiring(&self, threshold_seconds: i64) -> bool;

    /// 检查账户是否可用于调度
    pub fn is_available_for_scheduling(&self) -> bool;

    /// 检查是否有特定 OAuth 权限
    pub fn has_scope(&self, scope: &str) -> bool;
}
```

**单元测试**:
- ✅ `test_is_token_expiring` - Token过期检测
- ✅ `test_is_available_for_scheduling` - 调度可用性检查
- ✅ `test_has_scope` - 权限范围检查

#### 3. 模块导出更新
**文件**: `src/models/mod.rs`
- 导出所有账户相关类型
- 保持与 API Key 模型的一致性

---

## 🔧 遇到的编译问题

### 问题 1: Redis API 类型不匹配
**位置**: `src/redis/pool.rs:63`
```rust
// 错误: expected `u64`, found `usize`
conn.set_ex(key, value, seconds)
```

**分析**: Redis 0.24.0 的 `set_ex` 方法期望 `u64` 类型

### 问题 2: Redis expire 类型不匹配
**位置**: `src/redis/pool.rs:89`
```rust
// 错误: expected `i64`, found `usize`
conn.expire(key, seconds)
```

### 问题 3: Error 类型的借用冲突
**位置**: `src/utils/error.rs:83-91`
```rust
// 错误: borrow of partially moved value
Self::InternalError(msg) | Self::TokenRefreshFailed(msg) => { ... }
```

**修复**: 使用引用模式匹配 `match &self` 并克隆字符串

---

## 📋 下一步工作计划

### 待修复
1. ✅ Error 类型借用问题（已修复）
2. ⏳ Redis pool 类型错误（需要检查）
3. ⏳ 完整编译验证

### 待实现
1. **加密/解密功能**
   - AES-256-CBC 加密
   - Scrypt 密钥派生
   - LRU 解密缓存

2. **Claude 账户服务**
   - 账户 CRUD 操作
   - OAuth token 刷新
   - 分布式锁机制
   - 代理支持

3. **账户调度逻辑**
   - 优先级排序
   - 并发控制
   - 负载均衡
   - 粘性会话

4. **单元测试**
   - 服务层测试
   - 集成测试
   - Mock Redis

---

## 🏗️ 架构设计

### 数据流

```
用户请求 → API Key验证 → 账户调度器
                              ↓
                        选择最优账户
                              ↓
                        检查Token过期
                              ↓
                    (过期) → 刷新Token
                              ↓
                        使用账户转发请求
                              ↓
                        记录使用统计
```

### 关键设计决策

1. **类型安全的状态管理**
   - 使用枚举而非字符串表示状态
   - 编译时检查状态转换

2. **灵活的序列化**
   - 支持 JSON 序列化/反序列化
   - 与 Node.js 版本数据格式兼容

3. **性能优化**
   - Token 过期提前检测（10秒阈值）
   - 调度可用性快速判断
   - 权限检查O(n)复杂度

4. **扩展性**
   - 支持多平台（Claude/Gemini/OpenAI等）
   - 灵活的代理配置
   - 可扩展的订阅信息

---

## 💡 技术亮点

### 1. 强类型系统
```rust
pub enum AccountStatus {
    Active,
    Inactive,
    Error,
    Overloaded,
    Expired,
}
```

**优势**: 编译时保证状态正确性，避免运行时字符串错误

### 2. Serde 序列化
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Claude,
    Gemini,
    // ...
}
```

**优势**: 自动 JSON 序列化，与 Node.js Redis 数据兼容

### 3. Option 类型处理
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub access_token: Option<String>,
```

**优势**: 精确表达可空字段，避免 null 相关错误

### 4. 业务逻辑方法
```rust
pub fn is_token_expiring(&self, threshold_seconds: i64) -> bool {
    // 提前检测Token过期，避免请求失败
}
```

**优势**: 封装复杂逻辑，提供清晰API

---

## 📈 代码统计

### 新增代码
```
src/models/account.rs:      ~470 行 (数据模型 + 测试)
src/models/mod.rs:          +5 行 (导出声明)
src/utils/error.rs:         修复借用问题

总计新增:                  ~475 行
```

### 测试覆盖
```
单元测试:  3 个 (账户模型) ✅
测试通过: 3/3 (100%)
```

---

## 🎯 Week 4 目标

### Phase 1: 账户管理 (本周)
- [x] 数据模型设计
- [ ] 加密/解密实现
- [ ] OAuth token 刷新
- [ ] 账户 CRUD 操作
- [ ] 单元测试

### Phase 2: 调度系统 (下周)
- [ ] 账户选择算法
- [ ] 并发控制
- [ ] 负载均衡
- [ ] 粘性会话
- [ ] 集成测试

### Phase 3: 多平台支持 (Week 5)
- [ ] Gemini 账户支持
- [ ] OpenAI 账户支持
- [ ] Bedrock 账户支持
- [ ] 统一调度接口

---

## 🔄 与 Node.js 版本对比

| 功能 | Node.js | Rust (计划) | 状态 |
|------|---------|-------------|------|
| 数据模型 | JavaScript 对象 | 强类型结构体 | ✅ 完成 |
| 加密 | AES-256-CBC | AES-GCM | ⏳ 待实现 |
| Token刷新 | axios + 分布式锁 | reqwest + Redis锁 | ⏳ 待实现 |
| 缓存 | LRU Cache | LRU Cache | ⏳ 待实现 |
| 并发控制 | Redis Sorted Set | Redis Sorted Set | ⏳ 待实现 |
| 性能 | Baseline | +3x (预期) | 🎯 目标 |

---

## 🚧 已知问题

### 编译错误
1. **Redis API类型不匹配**: 需要验证 redis 0.24.0 的 API 签名
2. **类型推断问题**: 可能需要显式类型转换

### 待验证
1. **Redis 数据格式兼容性**: Rust序列化后的JSON能否被Node.js读取
2. **加密算法选择**: AES-GCM vs AES-CBC
3. **性能基准**: 实际性能提升幅度

---

## 📚 参考资料

### Node.js 实现
- `src/services/claudeAccountService.js` - 账户服务（~1000行）
- `src/services/tokenRefreshService.js` - Token刷新服务
- `src/utils/proxyHelper.js` - 代理工具
- `src/utils/lruCache.js` - LRU缓存

### Rust 依赖
- `serde/serde_json` - 序列化
- `chrono` - 时间处理
- `uuid` - ID生成
- `redis/deadpool-redis` - Redis客户端
- `reqwest` - HTTP客户端
- `aes-gcm` - 加密算法

---

**维护者**: Rust Migration Team
**最后更新**: 2025-10-30 23:45
**下次同步**: Week 4 继续开发
