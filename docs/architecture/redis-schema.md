---
category: architecture
ai_relevance: high
load_when: "Redis数据结构、数据模型、键模式、存储架构、数据关系"
related_docs:
  - ../guides/troubleshooting.md
  - ../development/cli-usage.md
  - ../README.md
---

# Redis 数据结构文档

> **导航**: [返回 CLAUDE.md](../../CLAUDE.md) | [文档索引](../README.md)

本文档详细描述 Claude Relay Service 的 Redis 数据结构和键模式。

---

## 数据结构概述

Claude Relay Service 使用 Redis 作为主要数据存储，所有数据以键值对形式存储。数据按功能模块组织，使用命名空间前缀区分不同类型的数据。

### 数据组织原则

- **命名空间隔离**: 使用 `:` 分隔不同层级（如 `api_key:{id}`）
- **类型化存储**: 使用 Redis 的不同数据类型（String, Hash, Set, Sorted Set）
- **加密存储**: 敏感数据（tokens, credentials）使用 AES 加密
- **TTL 管理**: 临时数据设置过期时间，自动清理
- **原子操作**: 使用 Redis 事务和管道保证数据一致性

---

## API Keys

### 核心数据

#### `api_key:{id}` - API Key 详细信息
**类型**: Hash
**TTL**: 永久（除非手动删除或设置过期时间）

**字段**:
```json
{
  "id": "uuid",
  "name": "MyApp",
  "key": "cr_1234567890abcdef...",  // 完整 API Key
  "hashedKey": "sha256_hash",        // SHA-256 哈希值
  "dailyLimit": 1000,                // 每日请求限制
  "requestsToday": 156,              // 今日请求数
  "totalRequests": 5432,             // 总请求数
  "status": "active",                // active/inactive/expired
  "createdAt": "2024-01-01T00:00:00Z",
  "lastUsed": "2024-01-15T12:30:00Z",
  "expiresAt": "2024-12-31T23:59:59Z",  // 可选，过期时间

  // 权限控制
  "permissions": "all",              // all/claude/gemini/openai/droid
  "allowedClients": "claude-code,gemini-cli",  // 允许的客户端（逗号分隔）
  "blockedModels": "claude-sonnet-4.0",        // 禁止的模型（逗号分隔）

  // 用户关联（USER_MANAGEMENT_ENABLED 时）
  "userId": "user_uuid",             // 关联用户ID
  "isUserKey": true,                 // 是否为用户创建的 Key

  // 统计信息
  "inputTokens": 123456,             // 总输入 tokens
  "outputTokens": 234567,            // 总输出 tokens
  "cacheCreateTokens": 12345,        // 缓存创建 tokens
  "cacheReadTokens": 23456,          // 缓存读取 tokens
  "totalCost": 12.34                 // 总成本（美元）
}
```

**访问模式**:
- 创建: `HSET api_key:{id} field value`
- 读取: `HGETALL api_key:{id}`
- 更新: `HINCRBY api_key:{id} requestsToday 1`
- 删除: `DEL api_key:{id}`

#### `api_key_hash:{hash}` - 哈希到ID映射
**类型**: String
**TTL**: 永久

**值**: API Key ID
```
"550e8400-e29b-41d4-a716-446655440000"
```

**用途**: O(1) 时间复杂度的 API Key 查找

**访问模式**:
```bash
# 验证 API Key
hash = SHA256(apiKey)
keyId = GET api_key_hash:{hash}
if keyId:
  keyData = HGETALL api_key:{keyId}
```

### 统计数据

#### `api_key_usage:{keyId}` - 使用统计
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "daily:{date}": 156,               // 每日请求数
  "model:{model}": 89,               // 各模型请求数
  "last_request_time": "2024-01-15T12:30:00Z",
  "last_request_model": "claude-3-5-sonnet-20241022"
}
```

#### `api_key_cost:{keyId}` - 成本统计
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "total_cost": 12.34,               // 总成本（美元）
  "daily:{date}": 0.56,              // 每日成本
  "model:{model}": 5.67,             // 各模型成本
  "input_tokens": 123456,
  "output_tokens": 234567,
  "cache_create_tokens": 12345,
  "cache_read_tokens": 23456
}
```

---

## 账户数据（多类型）

### Claude 官方账户

#### `claude_account:{id}` - Claude 官方账户
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "name": "Claude Account 1",
  "type": "claude-official",         // 账户类型
  "status": "active",                // active/inactive/error/overloaded
  "createdAt": "2024-01-01T00:00:00Z",
  "lastUsed": "2024-01-15T12:30:00Z",

  // OAuth 数据（加密存储）
  "claudeAiOauth": "{encrypted_json}", // AES 加密的 OAuth 数据
  // 解密后结构：
  // {
  //   "accessToken": "sk-ant-...",
  //   "refreshToken": "refresh_token...",
  //   "tokenType": "Bearer",
  //   "expiresAt": "2024-01-15T13:30:00Z",
  //   "scopes": ["read", "write"]
  // }

  // 代理配置
  "proxyType": "socks5",             // socks5/http/null
  "proxyHost": "127.0.0.1",
  "proxyPort": 1080,
  "proxyUsername": "user",
  "proxyPassword": "pass",

  // 限制和统计
  "maxConcurrent": 5,                // 最大并发数
  "currentConcurrent": 2,            // 当前并发数
  "totalRequests": 1234,             // 总请求数
  "successRequests": 1200,           // 成功请求数
  "errorCount": 34,                  // 错误次数
  "lastError": "429 Too Many Requests",
  "lastErrorTime": "2024-01-15T11:00:00Z"
}
```

### Claude Console 账户

#### `claude_console_account:{id}` - Claude Console 账户
**类型**: Hash
**TTL**: 永久

**字段**: 与 `claude_account` 类似，`type` 为 `claude-console`

### Gemini 账户

#### `gemini_account:{id}` - Gemini 账户
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "name": "Gemini Account 1",
  "type": "gemini",
  "status": "active",

  // Google OAuth 数据（加密存储）
  "googleOauth": "{encrypted_json}",
  // 解密后结构：
  // {
  //   "accessToken": "ya29.a0...",
  //   "refreshToken": "1//0g...",
  //   "tokenType": "Bearer",
  //   "expiresAt": "2024-01-15T13:30:00Z",
  //   "scopes": ["https://www.googleapis.com/auth/cloud-platform"]
  // }

  // 代理和其他配置同 claude_account
}
```

### OpenAI Responses (Codex) 账户

#### `openai_responses_account:{id}` - OpenAI Responses 账户
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "name": "Codex Account 1",
  "type": "openai-responses",
  "status": "active",

  // API Key（加密存储）
  "apiKey": "{encrypted_string}",    // 加密的 OpenAI API Key

  // 其他配置同上
}
```

### AWS Bedrock 账户

#### `bedrock_account:{id}` - AWS Bedrock 账户
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "name": "Bedrock Account 1",
  "type": "bedrock",
  "status": "active",

  // AWS 凭据（加密存储）
  "credentials": "{encrypted_json}",
  // 解密后结构：
  // {
  //   "accessKeyId": "AKIA...",
  //   "secretAccessKey": "secret...",
  //   "region": "us-east-1"
  // }

  // 其他配置同上
}
```

### Azure OpenAI 账户

#### `azure_openai_account:{id}` - Azure OpenAI 账户
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "name": "Azure OpenAI 1",
  "type": "azure-openai",
  "status": "active",

  // Azure 凭据（加密存储）
  "credentials": "{encrypted_json}",
  // 解密后结构：
  // {
  //   "apiKey": "azure_api_key...",
  //   "endpoint": "https://your-resource.openai.azure.com/",
  //   "deploymentName": "gpt-4"
  // }

  // 其他配置同上
}
```

### Droid 账户

#### `droid_account:{id}` - Droid (Factory.ai) 账户
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "name": "Droid Account 1",
  "type": "droid",
  "status": "active",

  // Droid API Key（加密存储）
  "apiKey": "{encrypted_string}",

  // 其他配置同上
}
```

### CCR 账户

#### `ccr_account:{id}` - CCR 账户
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "name": "CCR Account 1",
  "type": "ccr",
  "status": "active",

  // CCR 凭据（加密存储）
  "credentials": "{encrypted_json}",

  // 其他配置同上
}
```

---

## 用户管理

### 用户数据

#### `user:{id}` - 用户信息
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "email": "user@example.com",
  "username": "john_doe",
  "passwordHash": "bcrypt_hash",     // bcrypt 哈希密码
  "displayName": "John Doe",
  "status": "active",                // active/inactive/suspended
  "createdAt": "2024-01-01T00:00:00Z",
  "lastLogin": "2024-01-15T12:30:00Z",

  // LDAP 用户标识
  "ldapDn": "cn=john,ou=users,dc=example,dc=com",  // LDAP DN（如果是 LDAP 用户）
  "isLdapUser": true,                // 是否为 LDAP 用户

  // API Keys 关联
  "apiKeyCount": 3,                  // 拥有的 API Key 数量
  "maxApiKeys": 5                    // 最大允许 API Key 数量
}
```

#### `user_email:{email}` - 邮箱到用户ID映射
**类型**: String
**TTL**: 永久

**值**: User ID
```
"550e8400-e29b-41d4-a716-446655440000"
```

#### `user_session:{token}` - 用户会话
**类型**: Hash
**TTL**: 可配置（默认24小时）

**字段**:
```json
{
  "userId": "uuid",
  "email": "user@example.com",
  "createdAt": "2024-01-15T12:00:00Z",
  "expiresAt": "2024-01-16T12:00:00Z",
  "lastActivity": "2024-01-15T12:30:00Z"
}
```

### 管理员数据

#### `admin:{id}` - 管理员信息
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "username": "admin",
  "passwordHash": "bcrypt_hash",
  "email": "admin@example.com",
  "createdAt": "2024-01-01T00:00:00Z",
  "lastLogin": "2024-01-15T12:30:00Z"
}
```

#### `admin_username:{username}` - 用户名映射
**类型**: String
**TTL**: 永久

**值**: Admin ID

#### `admin_credentials` - 管理员凭据
**类型**: String
**TTL**: 永久

**值**: JSON 字符串（从 `data/init.json` 同步）
```json
{
  "username": "admin",
  "password": "generated_password"
}
```

---

## 会话管理

### JWT 会话

#### `session:{token}` - JWT 会话管理
**类型**: Hash
**TTL**: 可配置（JWT 过期时间）

**字段**:
```json
{
  "userId": "uuid",               // 用户ID（普通用户）
  "adminId": "uuid",              // 管理员ID（管理员）
  "type": "user",                 // user/admin
  "createdAt": "2024-01-15T12:00:00Z",
  "expiresAt": "2024-01-16T12:00:00Z"
}
```

### 粘性会话

#### `sticky_session:{sessionHash}` - 粘性会话账户绑定
**类型**: Hash
**TTL**: 可配置（STICKY_SESSION_TTL_HOURS，默认1小时）

**字段**:
```json
{
  "accountId": "uuid",               // 绑定的账户ID
  "accountType": "claude-official",  // 账户类型
  "createdAt": "2024-01-15T12:00:00Z",
  "expiresAt": "2024-01-15T13:00:00Z",
  "requestCount": 15                 // 此会话的请求数
}
```

**会话 Hash 计算**:
```javascript
// 基于请求内容的 hash
sessionHash = SHA256(JSON.stringify({
  messages: request.messages,
  model: request.model,
  system: request.system
}))
```

**续期策略**:
- 当剩余 TTL < `STICKY_SESSION_RENEWAL_THRESHOLD_MINUTES` 时自动续期
- 每次请求更新 `requestCount`

#### `session_window:{accountId}` - 账户会话窗口
**类型**: Sorted Set
**TTL**: 自动管理

**成员**: Session Hash
**分数**: 最后使用时间戳

**用途**: 跟踪账户的活跃会话，用于负载均衡

---

## 使用统计

### 日度统计

#### `usage:daily:{date}:{key}:{model}` - 按日期、Key、模型的使用统计
**类型**: Hash
**TTL**: 永久（或设置保留期）

**字段**:
```json
{
  "requests": 156,                   // 请求次数
  "inputTokens": 12345,              // 输入 tokens
  "outputTokens": 23456,             // 输出 tokens
  "cacheCreateTokens": 1234,         // 缓存创建 tokens
  "cacheReadTokens": 2345,           // 缓存读取 tokens
  "cost": 0.56,                      // 成本（美元）
  "errors": 3                        // 错误次数
}
```

**日期格式**: `YYYY-MM-DD`
**示例**: `usage:daily:2024-01-15:key123:claude-3-5-sonnet-20241022`

#### `usage:account:{accountId}:{date}` - 按账户的使用统计
**类型**: Hash
**TTL**: 永久

**字段**: 同上

#### `usage:global:{date}` - 全局使用统计
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "totalRequests": 5432,
  "totalInputTokens": 123456,
  "totalOutputTokens": 234567,
  "totalCacheCreateTokens": 12345,
  "totalCacheReadTokens": 23456,
  "totalCost": 12.34,
  "totalErrors": 45,

  // 按模型统计
  "model:{model}:requests": 1234,
  "model:{model}:cost": 5.67
}
```

---

## 速率限制

### 限流计数器

#### `rate_limit:{keyId}:{window}` - 速率限制计数器
**类型**: String (Integer)
**TTL**: 窗口时长（如 1 分钟、1 小时）

**值**: 当前窗口的请求计数
```
156
```

**窗口类型**:
- `minute` - 每分钟
- `hour` - 每小时
- `day` - 每天

**示例**:
- `rate_limit:key123:minute` - 当前分钟的请求数
- `rate_limit:key123:hour` - 当前小时的请求数
- `rate_limit:key123:day` - 今日请求数

**访问模式**:
```bash
# 递增计数
count = INCR rate_limit:{keyId}:{window}
if count == 1:
  EXPIRE rate_limit:{keyId}:{window} {ttl_seconds}

# 检查限制
if count > limit:
  return 429 Too Many Requests
```

#### `rate_limit_state:{accountId}` - 账户限流状态
**类型**: Hash
**TTL**: 可配置（默认5分钟）

**字段**:
```json
{
  "limited": true,                   // 是否被限流
  "limitType": "429",                // 限流类型（429/529）
  "limitedAt": "2024-01-15T12:30:00Z",
  "expiresAt": "2024-01-15T12:35:00Z",
  "retryAfter": 300,                 // 重试等待时间（秒）
  "reason": "Too Many Requests"
}
```

**自动清理**: rateLimitCleanupService 每5分钟清理过期状态

#### `overload:{accountId}` - 账户过载状态（529错误）
**类型**: String
**TTL**: 可配置（CLAUDE_OVERLOAD_HANDLING_MINUTES，默认30分钟）

**值**: 过载时间戳
```
"2024-01-15T12:30:00Z"
```

**用途**: 临时排除过载账户，避免持续失败

**清理**: TTL 到期自动删除

---

## 并发控制

### 并发计数

#### `concurrency:{accountId}` - 并发计数
**类型**: Sorted Set
**TTL**: 无（成员自动过期）

**成员**: 请求ID
**分数**: 请求开始时间戳

**访问模式**:
```bash
# 添加请求
ZADD concurrency:{accountId} {timestamp} {requestId}

# 检查并发数
current = ZCOUNT concurrency:{accountId} -inf +inf
if current >= maxConcurrent:
  return 503 Service Unavailable

# 完成请求（移除）
ZREM concurrency:{accountId} {requestId}

# 清理过期请求（超过 REQUEST_TIMEOUT）
ZREMRANGEBYSCORE concurrency:{accountId} -inf {now - timeout}
```

**自动清理**:
- 每分钟清理任务清理过期成员
- 服务重启时清理所有并发计数

---

## Webhook 配置

### Webhook 配置

#### `webhook_config:{id}` - Webhook 配置
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "id": "uuid",
  "name": "Slack Notification",
  "url": "https://hooks.slack.com/services/...",
  "events": "request.success,request.error,account.error",  // 事件列表（逗号分隔）
  "enabled": true,
  "headers": "{\"Authorization\":\"Bearer token\"}",  // 自定义请求头（JSON）
  "createdAt": "2024-01-01T00:00:00Z",
  "lastTriggered": "2024-01-15T12:30:00Z",
  "successCount": 1234,
  "errorCount": 5
}
```

**支持的事件**:
- `request.success` - 请求成功
- `request.error` - 请求失败
- `account.error` - 账户错误
- `account.overload` - 账户过载
- `rate_limit.exceeded` - 速率限制超出
- `token.refresh.failed` - Token 刷新失败

---

## 系统信息

### 系统缓存

#### `system_info` - 系统状态缓存
**类型**: Hash
**TTL**: 可配置（默认60秒）

**字段**:
```json
{
  "totalApiKeys": 150,
  "activeApiKeys": 145,
  "totalAccounts": 20,
  "activeAccounts": 18,
  "totalUsers": 50,               // USER_MANAGEMENT_ENABLED 时
  "requestsToday": 5432,
  "costToday": 12.34,
  "cacheHitRate": 0.85,
  "uptime": 86400,                // 秒
  "version": "1.0.0",
  "lastUpdated": "2024-01-15T12:30:00Z"
}
```

**用途**: 减少系统概览查询的计算开销

#### `model_pricing` - 模型价格数据
**类型**: Hash
**TTL**: 永久

**字段**:
```json
{
  "claude-3-5-sonnet-20241022:input": 0.003,      // 每 1K tokens 价格（美元）
  "claude-3-5-sonnet-20241022:output": 0.015,
  "claude-3-5-sonnet-20241022:cache_write": 0.00375,
  "claude-3-5-sonnet-20241022:cache_read": 0.0003,

  "gemini-2.0-flash-exp:input": 0.001,
  "gemini-2.0-flash-exp:output": 0.004
}
```

**加载来源**: pricingService 从配置文件加载

---

## 数据访问模式

### 常见查询

#### 验证 API Key
```bash
# 1. 计算哈希
hash = SHA256(apiKey)

# 2. 查找 ID
keyId = GET api_key_hash:{hash}

# 3. 获取详情
keyData = HGETALL api_key:{keyId}

# 4. 检查速率限制
count = INCR rate_limit:{keyId}:minute
if count > limit:
  return 429
```

#### 选择账户
```bash
# 1. 检查粘性会话
sessionHash = calculateHash(request)
stickyAccount = HGET sticky_session:{sessionHash} accountId

# 2. 如果有粘性会话，使用该账户
if stickyAccount:
  account = HGETALL {accountType}_account:{stickyAccount}
else:
  # 3. 否则，选择负载最低的活跃账户
  accounts = KEYS {accountType}_account:*
  for each account:
    if status == "active" and concurrent < maxConcurrent:
      select account with lowest concurrent

  # 4. 创建粘性会话
  HSET sticky_session:{sessionHash} accountId {selectedAccountId}
  EXPIRE sticky_session:{sessionHash} {ttl}
```

#### 刷新 Token
```bash
# 1. 获取账户
account = HGETALL claude_account:{id}

# 2. 解密 OAuth 数据
oauthData = decrypt(account.claudeAiOauth)

# 3. 检查过期
if oauthData.expiresAt < now + 10s:
  # 4. 使用 refresh_token 获取新 token
  newTokens = refreshOAuthToken(oauthData.refreshToken, proxy)

  # 5. 加密并更新
  encryptedOauth = encrypt(newTokens)
  HSET claude_account:{id} claudeAiOauth {encryptedOauth}
```

#### 记录使用统计
```bash
# 1. 更新 API Key 统计
HINCRBY api_key:{keyId} totalRequests 1
HINCRBY api_key:{keyId} inputTokens {inputTokens}
HINCRBY api_key:{keyId} outputTokens {outputTokens}

# 2. 更新每日统计
date = format(now, "YYYY-MM-DD")
HINCRBY usage:daily:{date}:{keyId}:{model} requests 1
HINCRBY usage:daily:{date}:{keyId}:{model} inputTokens {inputTokens}
HINCRBY usage:daily:{date}:{keyId}:{model} outputTokens {outputTokens}
HINCRBYFLOAT usage:daily:{date}:{keyId}:{model} cost {cost}

# 3. 更新全局统计
HINCRBY usage:global:{date} totalRequests 1
HINCRBYFLOAT usage:global:{date} totalCost {cost}
```

---

## 数据维护

### 自动清理任务

#### 并发计数清理
**频率**: 每分钟
**操作**:
```bash
for each concurrency:{accountId}:
  # 清理超过 REQUEST_TIMEOUT 的请求
  cutoff = now - REQUEST_TIMEOUT
  ZREMRANGEBYSCORE concurrency:{accountId} -inf {cutoff}
```

#### 速率限制清理
**频率**: 每5分钟
**操作**:
```bash
# rateLimitCleanupService 自动清理过期的 rate_limit_state
for each rate_limit_state:{accountId}:
  if TTL < 0 or expiresAt < now:
    DEL rate_limit_state:{accountId}
```

#### 过载状态清理
**频率**: 自动（基于 TTL）
**操作**:
```bash
# overload:{accountId} 键设置 TTL 自动过期
# CLAUDE_OVERLOAD_HANDLING_MINUTES 控制持续时间
```

### 备份和恢复

#### 数据导出
```bash
# 使用 CLI 工具
npm run data:export
npm run data:export:sanitized    # 脱敏导出
npm run data:export:enhanced     # 含解密数据
```

#### 数据导入
```bash
npm run data:import
npm run data:import:enhanced     # 含加密数据
```

### 数据迁移

#### API Key 过期迁移
```bash
npm run migrate:apikey-expiry
npm run migrate:apikey-expiry:dry   # 干跑模式
```

#### 使用统计修复
```bash
npm run migrate:fix-usage-stats
```

---

## 性能优化

### 索引优化
- **哈希映射**: `api_key_hash:{hash}` 实现 O(1) 查找
- **Sorted Set**: `concurrency:{accountId}` 实现高效并发控制
- **TTL 管理**: 临时数据自动过期，减少内存占用

### 缓存策略
- **解密缓存**: 减少 AES 解密操作
- **账户缓存**: 减少 Redis 查询
- **系统信息缓存**: 减少聚合计算

### 原子操作
- **Redis 事务**: 保证数据一致性
- **Pipeline**: 批量操作减少网络往返
- **INCR/HINCRBY**: 原子递增计数器

---

## 安全考虑

### 敏感数据加密
所有敏感数据使用 AES-256-CBC 加密：
- `claudeAiOauth` - Claude OAuth tokens
- `googleOauth` - Google OAuth tokens
- `apiKey` - 第三方 API Keys
- `credentials` - AWS/Azure 凭据

### 哈希存储
- API Key: SHA-256 哈希
- 用户密码: bcrypt 哈希

### 访问控制
- JWT 会话管理
- API Key 权限验证
- 客户端白名单
- 模型黑名单

---

**相关文档**:
- [故障排除指南](../guides/troubleshooting.md)
- [CLI 使用指南](../development/cli-usage.md)
- [项目主文档](../../CLAUDE.md)
