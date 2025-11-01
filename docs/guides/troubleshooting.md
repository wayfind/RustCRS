---
category: guides
ai_relevance: high
load_when: "问题诊断、错误排查、系统调试、配置问题、服务异常"
related_docs:
  - ../development/cli-usage.md
  - ../architecture/redis-schema.md
  - ../README.md
---

# 故障排除指南

> **导航**: [返回 CLAUDE.md](../../CLAUDE.md) | [文档索引](../README.md)

本指南提供 Claude Relay Service 常见问题的诊断和解决方案。

---

## OAuth相关问题

### 1. 代理配置错误
**问题**: OAuth token交换失败，连接超时

**解决方案**:
- 检查代理设置是否正确（SOCKS5/HTTP格式）
- OAuth token交换也需要代理配置
- 验证代理认证信息（用户名/密码）
- 检查 PROXY_USE_IPV4 设置（默认true）

### 2. 授权码无效
**问题**: Authorization Code 交换失败

**解决方案**:
- 确保复制了完整的 Authorization Code
- 检查是否有遗漏字符或额外空格
- 授权码只能使用一次，重新授权获取新码

### 3. Token刷新失败
**问题**: refreshToken 自动刷新失败

**解决方案**:
- 检查 refreshToken 有效性（是否过期或被撤销）
- 验证代理配置（token刷新也需要代理）
- 查看 `logs/token-refresh-error.log` 获取详细错误
- 手动刷新账户 token: `npm run cli accounts refresh <accountId>`

---

## Gemini Token刷新问题

### 1. 刷新失败
**问题**: Gemini refresh_token 刷新失败

**解决方案**:
- 确保 refresh_token 有效且未过期
- Google OAuth token 有效期较长，但仍会过期
- 重新授权 Gemini 账户获取新的 refresh_token

### 2. 错误日志
**问题**: 需要查看详细刷新错误

**解决方案**:
- 查看 `logs/token-refresh-error.log` 获取详细错误信息
- 日志包含完整的 HTTP 响应和错误堆栈

### 3. 测试脚本
**问题**: 需要手动测试 token 刷新

**解决方案**:
- 运行测试脚本: `node scripts/test-gemini-refresh.js`
- 脚本会测试 token 刷新流程并输出详细信息

---

## 常见开发问题

### 1. Redis连接失败
**问题**: 应用启动失败，Redis 连接错误

**解决方案**:
- 确认 Redis 服务正在运行: `redis-cli ping`
- 检查环境变量配置:
  - `REDIS_HOST` (默认 localhost)
  - `REDIS_PORT` (默认 6379)
  - `REDIS_PASSWORD` (如果设置了密码)
- 验证 Redis 连接: `redis-cli -h <host> -p <port> -a <password>`

### 2. 管理员登录失败
**问题**: Web 界面管理员登录失败

**解决方案**:
- 检查 `data/init.json` 文件是否存在
- 运行初始化脚本: `npm run setup`
- 重新生成管理员凭据
- 检查 Redis 中的 `admin_credentials` 键

### 3. API Key格式错误
**问题**: API Key 验证失败

**解决方案**:
- 确保使用正确的前缀格式（默认 `cr_`）
- 可通过 `API_KEY_PREFIX` 环境变量修改前缀
- 使用 CLI 创建 API Key: `npm run cli keys create --name "MyApp"`
- 检查 API Key 哈希映射: `npm run data:debug`

### 4. 代理连接问题
**问题**: 通过代理的请求失败

**解决方案**:
- 验证 SOCKS5/HTTP 代理配置格式
- 检查代理认证信息（用户名/密码）
- 设置 `PROXY_USE_IPV4=true` 强制使用 IPv4
- 测试代理连接: `curl -x <proxy> https://api.anthropic.com`

### 5. 粘性会话失效
**问题**: 同一会话使用了不同账户

**解决方案**:
- 检查 Redis 中 session 数据: `sticky_session:{sessionHash}`
- 确认 `STICKY_SESSION_TTL_HOURS` 配置（默认1小时）
- **Nginx 代理配置**: 添加 `underscores_in_headers on;` 支持自定义请求头
- 检查请求头中的会话标识符传递
- 调整续期阈值: `STICKY_SESSION_RENEWAL_THRESHOLD_MINUTES`

### 6. LDAP认证失败
**问题**: LDAP 用户登录失败

**解决方案**:
- 检查环境变量配置:
  - `LDAP_URL` (如 `ldaps://ldap.example.com:636`)
  - `LDAP_BIND_DN` (绑定用户 DN)
  - `LDAP_BIND_PASSWORD` (绑定密码)
- **自签名证书问题**: 设置 `LDAP_TLS_REJECT_UNAUTHORIZED=false`
- 查看日志中的 LDAP 连接错误详情
- 测试 LDAP 连接: `ldapsearch -H <url> -D <bind_dn> -W`

### 7. 用户管理功能不可用
**问题**: 用户管理端点返回 404 或功能禁用

**解决方案**:
- 确认 `USER_MANAGEMENT_ENABLED=true`
- 检查 userService 初始化状态
- 重启服务使配置生效
- 查看启动日志确认用户管理模块加载

### 8. Webhook通知失败
**问题**: Webhook 事件未发送或失败

**解决方案**:
- 确认 `WEBHOOK_ENABLED=true`
- 检查 `WEBHOOK_URLS` 格式（逗号分隔多个URL）
- 查看 `logs/webhook-*.log` 日志
- 验证 Webhook URL 可访问性
- 检查 Webhook 配置: `/admin/webhook/configs`

### 9. 统一调度器选择账户失败
**问题**: 请求无可用账户

**解决方案**:
- 检查账户状态: `status: 'active'`
- 确认账户类型与请求路由匹配
  - `/api` 或 `/claude` → claude-official, claude-console, bedrock, ccr
  - `/gemini` → gemini
  - `/openai` → openai-responses, azure-openai
  - `/droid` → droid
- 查看粘性会话绑定情况
- 检查账户 token 有效性: `npm run cli accounts list`

### 10. 并发计数泄漏
**问题**: 并发计数未正确释放

**解决方案**:
- 系统每分钟自动清理过期并发计数（concurrency cleanup task）
- 重启服务时自动清理所有并发计数
- 手动清理: 删除 Redis 中的 `concurrency:{accountId}` 键
- 检查客户端断开时的资源清理逻辑

### 11. 速率限制未清理
**问题**: 速率限制状态未过期

**解决方案**:
- rateLimitCleanupService 每5分钟自动清理过期限流状态
- 检查 `rate_limit:{keyId}:{window}` 键的 TTL
- 手动清理: `npm run data:debug` 查看并删除过期键

### 12. 成本统计不准确
**问题**: 使用成本计算错误或缺失

**解决方案**:
- 运行初始化脚本: `npm run init:costs`
- 检查 pricingService 是否正确加载模型价格
- 更新模型价格: `npm run update:pricing`
- 测试价格回退: `npm run test:pricing-fallback`
- 查看 `model_pricing` Redis 键

### 13. 缓存命中率低
**问题**: 解密缓存或账户缓存效率低

**解决方案**:
- 查看缓存监控统计（Web 界面或 `/metrics` 端点）
- 调整 LRU 缓存大小配置
- 检查缓存失效策略
- 使用 cacheMonitor 分析缓存模式

---

## 调试工具

### 日志系统
Winston 结构化日志，支持不同级别，logs/ 目录下分类存储：

- **`logs/claude-relay-*.log`** - 应用主日志
  - 所有核心服务的运行日志
  - 请求转发、账户选择、token 刷新记录

- **`logs/token-refresh-error.log`** - Token 刷新错误
  - OAuth token 刷新失败详情
  - Google OAuth 刷新错误

- **`logs/webhook-*.log`** - Webhook 通知日志
  - Webhook 发送成功/失败记录
  - 事件详情和响应状态

- **`logs/http-debug-*.log`** - HTTP 调试日志
  - 需设置 `DEBUG_HTTP_TRAFFIC=true`
  - 记录完整的 HTTP 请求/响应（仅开发环境）

**日志查看命令**:
```bash
# 实时查看主日志
tail -f logs/claude-relay-$(date +%Y-%m-%d).log

# 查看错误日志
grep ERROR logs/claude-relay-*.log

# 查看特定账户的日志
grep "accountId:abc123" logs/claude-relay-*.log
```

### CLI工具
命令行状态查看和管理

```bash
# 系统状态
npm run cli status
npm run status
npm run status:detail

# 账户管理
npm run cli accounts list
npm run cli gemini list

# API Key 管理
npm run cli keys list
npm run cli keys create --name "Test"
```

### Web界面
实时日志查看和系统监控

- 访问 `/admin-next/` 打开管理界面
- **日志查看器**: 实时日志流，支持过滤和搜索
- **系统监控**: 账户状态、使用统计、性能指标
- **缓存监控**: 缓存命中率、大小、失效情况

### 健康检查
`/health` 端点提供系统状态

```bash
curl http://localhost:3000/health
```

返回信息包括:
- Redis 连接状态
- Logger 状态
- 内存使用情况
- 系统版本
- Uptime

### 系统指标
`/metrics` 端点提供详细的使用统计和性能指标

```bash
curl http://localhost:3000/metrics
```

返回信息包括:
- API Key 使用统计
- 账户请求次数
- Token 使用量
- 成本统计
- 缓存性能

### 缓存监控
cacheMonitor 提供全局缓存统计和命中率分析

- **解密缓存**: OAuth token 解密结果缓存
- **账户缓存**: 账户数据缓存
- **命中率分析**: 缓存效率评估

### 数据导出工具
导出 Redis 数据进行调试

```bash
# 标准导出
npm run data:export

# 脱敏导出（用于分享）
npm run data:export:sanitized

# 增强导出（含解密数据，仅调试用）
npm run data:export:enhanced

# 加密导出（安全传输）
npm run data:export:encrypted
```

### Redis Key调试
查看所有 Redis 键和数据结构

```bash
npm run data:debug
```

输出包括:
- 所有 Redis 键列表
- 键类型和 TTL
- 键前缀统计
- 数据大小分析

---

## 常见错误代码

### HTTP 错误代码

- **401 Unauthorized**: API Key 无效或已过期
- **403 Forbidden**: 权限不足或模型被黑名单禁止
- **429 Too Many Requests**: 速率限制或并发限制
- **500 Internal Server Error**: 服务内部错误，查看日志
- **502 Bad Gateway**: 上游 API 连接失败（代理或网络问题）
- **503 Service Unavailable**: 无可用账户或所有账户暂时不可用
- **529 Overloaded**: Claude API 过载（自动标记账户并暂时排除）

### 业务错误代码

- **RATE_LIMIT_EXCEEDED**: API Key 速率限制超出
- **QUOTA_EXCEEDED**: API Key 配额耗尽
- **INVALID_MODEL**: 请求的模型不可用或被禁止
- **NO_AVAILABLE_ACCOUNT**: 无可用账户处理请求
- **TOKEN_REFRESH_FAILED**: OAuth token 刷新失败
- **AUTHENTICATION_FAILED**: 认证失败（API Key 或 OAuth）

---

## 性能调优

### Redis 优化
- 调整 Redis maxmemory 和 maxmemory-policy
- 使用 Redis 持久化（RDB 或 AOF）
- 监控 Redis 内存使用: `redis-cli info memory`

### 缓存调优
- 增加 LRU 缓存大小（解密缓存、账户缓存）
- 调整缓存 TTL 策略
- 监控缓存命中率

### 并发控制
- 调整账户并发限制
- 优化粘性会话 TTL
- 使用账户组进行负载分散

### 日志优化
- 生产环境设置合理的日志级别（info 或 warn）
- 定期清理旧日志文件
- 禁用 DEBUG_HTTP_TRAFFIC（性能影响较大）

---

## 应急处理

### 服务无响应
1. 检查 Redis 连接: `redis-cli ping`
2. 查看服务状态: `npm run service:status`
3. 检查错误日志: `tail -f logs/claude-relay-*.log`
4. 重启服务: `npm run service:stop && npm run service:start:daemon`

### 账户全部不可用
1. 检查账户状态: `npm run cli accounts list`
2. 刷新所有账户 token
3. 清理过载标记: 删除 Redis `overload:{accountId}` 键
4. 验证代理配置

### 数据丢失
1. 检查 Redis 持久化配置
2. 从备份恢复: `npm run data:import`
3. 重新初始化系统: `npm run setup`

### 高负载处理
1. 检查并发控制配置
2. 增加账户数量分散负载
3. 启用速率限制保护
4. 调整 REQUEST_TIMEOUT 避免长时间占用

---

**更多信息**:
- [CLI 使用指南](../development/cli-usage.md)
- [Redis 数据结构](../architecture/redis-schema.md)
- [项目主文档](../../CLAUDE.md)
