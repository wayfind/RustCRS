# OpenAI 路由完整实现分析

## 当前状态

**已完成**：
- ✅ 基础 OpenAI 路由框架（src/routes/openai.rs）
- ✅ 基础集成测试（tests/openai_routes_integration_test.rs）
- ✅ 编译通过，12个测试全部通过

**问题**：
- ❌ 当前实现只是占位符，未实现真正的 OpenAI Responses (Codex) 协议
- ❌ 缺少核心依赖服务（UnifiedOpenAIScheduler, OpenAIResponsesRelayService）
- ❌ Usage 数据结构不匹配

## Node.js 完整功能需求

### 1. POST /responses, /v1/responses (handleResponses)

**核心流程**：
```
1. 权限验证（OpenAI/All 权限）
2. 提取 sessionId 和 sessionHash
3. 模型归一化（gpt-5-* → gpt-5）
4. Codex CLI 请求检测和适配
   - 非 Codex CLI 请求：自动添加完整的 instructions（巨大的 Codex CLI 指令文本）
   - 移除不需要的字段（temperature, top_p, max_output_tokens, etc.）
5. 使用 unifiedOpenAIScheduler 选择账户
   - 支持专属账户绑定
   - 支持账户分组
   - 粘性会话支持
   - 优先级和负载均衡
6. 账户类型判断：
   - openai-responses: 调用 openaiResponsesRelayService
   - openai: 直接转发到 chatgpt.com/backend-api/codex/responses
7. 构建请求头：
   - Authorization: Bearer {accessToken}
   - chatgpt-account-id: {accountId}
   - host: chatgpt.com
   - accept: text/event-stream (流式) / application/json (非流式)
8. 代理配置
9. 流式/非流式响应处理
10. Usage 数据捕获和记录
11. 错误处理（429, 401, 402）
12. 速率限制计数器更新
```

**Codex CLI Instructions**（超长文本，~8KB）：
```
You are a coding agent running in the Codex CLI, a terminal-based coding assistant...
[完整的 Codex CLI 系统提示词]
```

**关键依赖**：
- `unifiedOpenAIScheduler.selectAccountForApiKey(apiKeyData, sessionHash, requestedModel)`
- `openaiResponsesRelayService.handleRequest(req, res, account, apiKeyData)`
- `apiKeyService.recordUsage(keyId, inputTokens, outputTokens, cacheCreate, cacheRead, model, accountId)`
- `updateRateLimitCounters(rateLimitInfo, usageSummary, model)`

**响应头提取**：
- Codex Usage Headers:
  - `x-codex-primary-used-percent`
  - `x-codex-primary-reset-after-seconds`
  - `x-codex-primary-window-minutes`
  - `x-codex-secondary-used-percent`
  - `x-codex-secondary-reset-after-seconds`
  - `x-codex-secondary-window-minutes`
  - `x-codex-primary-over-secondary-limit-percent`

**Usage 数据解析**：
- 流式响应：从 SSE 事件 `response.completed` 中提取 `response.usage`
- 非流式响应：直接从 `data.usage` 获取
- 字段映射：
  - `input_tokens` / `prompt_tokens` → actualInputTokens（减去缓存）
  - `output_tokens` / `completion_tokens` → outputTokens
  - `input_tokens_details.cached_tokens` → cacheReadTokens
  - `cache_creation_input_tokens` / `cache_creation_tokens` → cacheCreateTokens

**错误处理**：
- **429 限流**：
  - 提取 `error.resets_in_seconds`
  - 调用 `unifiedOpenAIScheduler.markAccountRateLimited(accountId, 'openai', sessionHash, resetsInSeconds)`
  - 返回错误响应（流式: SSE data事件, 非流式: JSON）
- **401/402 认证错误**：
  - 调用 `unifiedOpenAIScheduler.markAccountUnauthorized(accountId, accountType, sessionHash, reason)`
  - 返回 401/402 错误
- **成功响应（200）**：
  - 检查并移除限流状态：`unifiedOpenAIScheduler.removeAccountRateLimit(accountId, 'openai')`

### 2. GET /usage

**响应格式**：
```json
{
  "object": "usage",
  "total_tokens": usage.total.tokens,
  "total_requests": usage.total.requests,
  "daily_tokens": usage.daily.tokens,
  "daily_requests": usage.daily.requests,
  "monthly_tokens": usage.monthly.tokens,
  "monthly_requests": usage.monthly.requests
}
```

**依赖**：
- `req.apiKey.usage.{total, daily, monthly}.{tokens, requests}`

### 3. GET /key-info

**响应格式**：
```json
{
  "id": keyData.id,
  "name": keyData.name,
  "description": keyData.description,
  "permissions": keyData.permissions || 'all',
  "token_limit": keyData.tokenLimit,
  "tokens_used": keyData.usage.total.tokens,
  "tokens_remaining": keyData.tokenLimit > 0 
    ? Math.max(0, keyData.tokenLimit - keyData.usage.total.tokens) 
    : null,
  "rate_limit": {
    "window": keyData.rateLimitWindow,
    "requests": keyData.rateLimitRequests
  },
  "usage": {
    "total": keyData.usage.total,
    "daily": keyData.usage.daily,
    "monthly": keyData.usage.monthly
  }
}
```

## 缺失的 Rust 服务

### 1. UnifiedOpenAIScheduler

**功能**：
- 账户选择算法（专属账户、分组、共享池）
- 粘性会话管理（session_mapping Redis键）
- 限流状态管理（rate_limit_status Redis键）
- 账户可用性检查（isActive, status, schedulable, rateLimitStatus）
- Token过期检测和自动刷新
- 模型支持检查
- 按最后使用时间排序（最久未使用优先）

**关键方法**：
- `selectAccountForApiKey(apiKeyData, sessionHash, requestedModel)`
- `markAccountRateLimited(accountId, accountType, sessionHash, resetsInSeconds)`
- `markAccountUnauthorized(accountId, accountType, sessionHash, reason)`
- `removeAccountRateLimit(accountId, accountType)`
- `isAccountRateLimited(accountId)`

### 2. OpenAIResponsesRelayService

**功能**：
- 转发请求到第三方 OpenAI 兼容 API
- 代理支持（账户级别）
- User-Agent 处理（自定义或透传）
- 流式/非流式响应处理
- Usage 数据捕获（response.completed 事件）
- 429/401 错误处理
- AbortController 支持（客户端断开时取消请求）
- 订阅过期检查

**关键方法**：
- `handleRequest(req, res, account, apiKeyData)`
- `_handleStreamResponse(response, res, account, apiKeyData, requestedModel, handleClientDisconnect, req)`
- `_handleNormalResponse(response, res, account, apiKeyData, requestedModel)`
- `_handle429Error(account, response, isStream, sessionHash)`

### 3. Usage 统计系统增强

**需要支持**：
- `usage.total.{tokens, requests}`
- `usage.daily.{tokens, requests}`
- `usage.monthly.{tokens, requests}`

**当前 Rust 只有**：
- `total_input_tokens`
- `total_output_tokens`
- `total_cache_creation_tokens`
- `total_cache_read_tokens`

### 4. 速率限制计数器（rateLimitHelper）

**功能**：
- `updateRateLimitCounters(rateLimitInfo, usageSummary, model)`
- 更新 token 和成本计数器

## 实现优先级

### Phase 1（当前已完成）- 基础框架
- ✅ 路由文件结构
- ✅ 基础测试框架

### Phase 2（需要实现）- 核心服务
1. **UnifiedOpenAIScheduler**
   - 账户服务集成（openaiAccountService, openaiResponsesAccountService）
   - 会话管理（session_mapping）
   - 限流管理（rate_limit_status）
   
2. **OpenAIResponsesRelayService**
   - HTTP 客户端（reqwest）
   - 流式响应处理（SSE）
   - Usage 解析

3. **Usage 统计系统**
   - total/daily/monthly 分类
   - requests 计数

### Phase 3（需要实现）- 完整路由
1. **handleResponses 实现**
   - Codex CLI 适配逻辑
   - 调度器集成
   - 错误处理
   
2. **Usage/KeyInfo 端点**
   - 正确的数据结构

### Phase 4（需要实现）- 测试
1. **完整集成测试**
   - Codex CLI 适配测试
   - 流式响应测试
   - 限流测试
   - 认证错误测试

## 待办事项

- [ ] 实现 UnifiedOpenAIScheduler（Rust 版本）
- [ ] 实现 OpenAIResponsesRelayService（Rust 版本）
- [ ] 增强 Usage 统计系统（total/daily/monthly + requests）
- [ ] 实现 rateLimitHelper
- [ ] 完整重写 OpenAI 路由
- [ ] 编写完整的集成测试

## 估算工作量

- UnifiedOpenAIScheduler: 1-2 天
- OpenAIResponsesRelayService: 1 天
- Usage 统计系统增强: 0.5 天
- 完整 OpenAI 路由: 1 天
- 完整测试: 0.5 天
- **总计**: 约 4-5 天

## 备注

由于工作量巨大且依赖多个尚未实现的服务，建议：
1. 先完成其他核心 Phase（定价、成本、Webhook等）
2. 等基础设施完善后，再回来完整实现 OpenAI 路由
3. 当前的简化版本可以保留作为占位符
