# API 接口文档

**版本**: v1.0.0
**基础URL**: `http://localhost:3000` (开发环境)
**认证方式**: Bearer Token (API Key)

---

## 目录

1. [认证](#认证)
2. [Claude API 端点](#claude-api-端点)
3. [Gemini API 端点](#gemini-api-端点)
4. [OpenAI 兼容端点](#openai-兼容端点)
5. [系统管理端点](#系统管理端点)
6. [错误响应](#错误响应)
7. [速率限制](#速率限制)

---

## 认证

所有 API 请求必须包含有效的 API Key，通过 HTTP Authorization 头发送：

```http
Authorization: Bearer cr_your_api_key_here
```

### API Key 格式

- **前缀**: `cr_` (可配置)
- **长度**: 32 字符（不含前缀）
- **示例**: `cr_1234567890abcdef1234567890abcdef`

### 获取 API Key

API Key 由管理员通过 Web 管理界面或 CLI 工具创建：

```bash
cargo run --bin cli -- keys create --name "MyApp" --limit 1000
```

---

## Claude API 端点

### 1. 发送消息 (流式)

创建 Claude 对话消息并接收流式响应。

**端点**: `POST /api/v1/messages`

**请求头**:
```http
Authorization: Bearer cr_your_api_key_here
Content-Type: application/json
anthropic-version: 2023-06-01
```

**请求体**:
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "messages": [
    {
      "role": "user",
      "content": "Hello, Claude!"
    }
  ],
  "max_tokens": 1024,
  "stream": true
}
```

**请求参数**:

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `model` | string | 是 | 模型名称 (如 `claude-3-5-sonnet-20241022`) |
| `messages` | array | 是 | 对话消息数组 |
| `max_tokens` | integer | 是 | 最大输出 token 数 (1-8192) |
| `stream` | boolean | 否 | 是否启用流式响应 (默认 false) |
| `temperature` | number | 否 | 温度参数 (0.0-1.0) |
| `top_p` | number | 否 | Top-p 采样参数 (0.0-1.0) |
| `stop_sequences` | array | 否 | 停止序列 |
| `system` | string | 否 | 系统提示词 |

**流式响应** (SSE):
```
event: message_start
data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-3-5-sonnet-20241022","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"!"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":2}}

event: message_stop
data: {"type":"message_stop"}
```

**非流式响应**:
```json
{
  "id": "msg_123",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Hello! How can I help you today?"
    }
  ],
  "model": "claude-3-5-sonnet-20241022",
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 10,
    "output_tokens": 15
  }
}
```

### 2. 获取 API Key 信息

查询当前 API Key 的配额和使用情况。

**端点**: `GET /api/v1/key-info`

**请求头**:
```http
Authorization: Bearer cr_your_api_key_here
```

**响应**:
```json
{
  "id": "key_123",
  "name": "MyApp API Key",
  "rate_limit": 1000,
  "usage": {
    "total_requests": 150,
    "total_tokens": 50000,
    "input_tokens": 30000,
    "output_tokens": 20000,
    "cache_create_tokens": 10000,
    "cache_read_tokens": 5000
  },
  "costs": {
    "total_cost": 0.05,
    "input_cost": 0.02,
    "output_cost": 0.03,
    "cache_create_cost": 0.005,
    "cache_read_cost": 0.001
  },
  "permissions": ["claude", "gemini"],
  "created_at": "2025-10-15T10:30:00Z"
}
```

### 3. 获取模型列表

获取所有可用的 Claude 模型。

**端点**: `GET /api/v1/models`

**请求头**:
```http
Authorization: Bearer cr_your_api_key_here
```

**响应**:
```json
{
  "data": [
    {
      "id": "claude-3-5-sonnet-20241022",
      "type": "model",
      "display_name": "Claude 3.5 Sonnet",
      "created_at": "2024-10-22T00:00:00Z"
    },
    {
      "id": "claude-3-opus-20240229",
      "type": "model",
      "display_name": "Claude 3 Opus",
      "created_at": "2024-02-29T00:00:00Z"
    }
  ]
}
```

---

## Gemini API 端点

### 1. 生成内容 (流式)

使用 Gemini 模型生成内容。

**端点**: `POST /gemini/v1/models/{model}:streamGenerateContent`

**路径参数**:
- `{model}`: 模型名称 (如 `gemini-1.5-pro`)

**请求头**:
```http
Authorization: Bearer cr_your_api_key_here
Content-Type: application/json
```

**请求体**:
```json
{
  "contents": [
    {
      "role": "user",
      "parts": [
        {
          "text": "Hello, Gemini!"
        }
      ]
    }
  ],
  "generationConfig": {
    "maxOutputTokens": 1024,
    "temperature": 0.7
  }
}
```

**流式响应** (SSE):
```
data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]},"index":0}],"usageMetadata":{"promptTokenCount":3,"totalTokenCount":3}}

data: {"candidates":[{"content":{"role":"model","parts":[{"text":"!"}]},"index":0,"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":3,"candidatesTokenCount":2,"totalTokenCount":5}}
```

### 2. 生成内容 (非流式)

**端点**: `POST /gemini/v1/models/{model}:generateContent`

**响应**:
```json
{
  "candidates": [
    {
      "content": {
        "role": "model",
        "parts": [
          {
            "text": "Hello! How can I help you today?"
          }
        ]
      },
      "finishReason": "STOP",
      "index": 0
    }
  ],
  "usageMetadata": {
    "promptTokenCount": 3,
    "candidatesTokenCount": 15,
    "totalTokenCount": 18
  }
}
```

### 3. 获取 Gemini 模型列表

**端点**: `GET /gemini/v1/models`

**响应**:
```json
{
  "models": [
    {
      "name": "models/gemini-1.5-pro",
      "displayName": "Gemini 1.5 Pro",
      "supportedGenerationMethods": ["generateContent", "streamGenerateContent"]
    },
    {
      "name": "models/gemini-1.5-flash",
      "displayName": "Gemini 1.5 Flash",
      "supportedGenerationMethods": ["generateContent", "streamGenerateContent"]
    }
  ]
}
```

---

## OpenAI 兼容端点

### 1. 聊天补全

OpenAI 格式的聊天补全接口，可转发到 Claude 或 Gemini。

**端点**: `POST /openai/v1/chat/completions`

**请求头**:
```http
Authorization: Bearer cr_your_api_key_here
Content-Type: application/json
```

**请求体**:
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "max_tokens": 1024,
  "stream": true
}
```

**流式响应** (SSE):
```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"claude-3-5-sonnet-20241022","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"claude-3-5-sonnet-20241022","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"claude-3-5-sonnet-20241022","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":2,"total_tokens":12}}

data: [DONE]
```

**非流式响应**:
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "claude-3-5-sonnet-20241022",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 15,
    "total_tokens": 25
  }
}
```

### 2. 获取 OpenAI 格式模型列表

**端点**: `GET /openai/v1/models`

**响应**:
```json
{
  "object": "list",
  "data": [
    {
      "id": "claude-3-5-sonnet-20241022",
      "object": "model",
      "created": 1729555200,
      "owned_by": "anthropic"
    },
    {
      "id": "gemini-1.5-pro",
      "object": "model",
      "created": 1704067200,
      "owned_by": "google"
    }
  ]
}
```

---

## 系统管理端点

### 1. 健康检查

检查服务健康状态。

**端点**: `GET /health`

**响应**:
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "components": {
    "redis": "connected",
    "api_keys": 10,
    "accounts": 5
  }
}
```

### 2. 系统指标

获取系统使用统计。

**端点**: `GET /metrics`

**请求头**:
```http
Authorization: Bearer cr_admin_key_here
```

**响应**:
```json
{
  "uptime_seconds": 3600,
  "total_requests": 1000,
  "total_tokens": {
    "input": 500000,
    "output": 300000,
    "cache_create": 100000,
    "cache_read": 50000
  },
  "total_costs": {
    "total": 5.50,
    "input": 2.50,
    "output": 2.00,
    "cache_create": 0.75,
    "cache_read": 0.25
  },
  "active_accounts": 5,
  "cache_stats": {
    "size": 450,
    "hits": 8500,
    "misses": 1500,
    "hit_rate": 85.0
  }
}
```

---

## 错误响应

所有错误响应遵循统一格式：

```json
{
  "error": {
    "type": "invalid_request_error",
    "message": "API key is invalid or has been revoked"
  }
}
```

### 错误类型

| HTTP 状态码 | 错误类型 | 描述 |
|-------------|----------|------|
| 400 | `invalid_request_error` | 请求参数无效 |
| 401 | `authentication_error` | API Key 无效或已吊销 |
| 403 | `permission_error` | 无权访问该资源 |
| 429 | `rate_limit_error` | 超过速率限制 |
| 500 | `api_error` | 服务器内部错误 |
| 502 | `upstream_error` | 上游 API 错误 |
| 503 | `overloaded_error` | 服务暂时不可用 |

### 错误示例

#### 认证错误 (401)
```json
{
  "error": {
    "type": "authentication_error",
    "message": "Invalid API key format"
  }
}
```

#### 速率限制错误 (429)
```json
{
  "error": {
    "type": "rate_limit_error",
    "message": "Rate limit exceeded: 1000 requests per hour"
  }
}
```

#### 权限错误 (403)
```json
{
  "error": {
    "type": "permission_error",
    "message": "API key does not have permission to access Claude service"
  }
}
```

#### 模型黑名单错误 (403)
```json
{
  "error": {
    "type": "permission_error",
    "message": "Model 'claude-3-opus-20240229' is in API key blacklist"
  }
}
```

---

## 速率限制

### 限制策略

- **滑动窗口**: 每小时请求数限制
- **并发控制**: 每个账户最大并发请求数
- **自动恢复**: 限流状态在窗口过期后自动清除

### 响应头

速率限制信息通过 HTTP 响应头返回：

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 950
X-RateLimit-Reset: 1234567890
```

| 响应头 | 描述 |
|--------|------|
| `X-RateLimit-Limit` | 每小时最大请求数 |
| `X-RateLimit-Remaining` | 当前窗口剩余请求数 |
| `X-RateLimit-Reset` | 限流重置时间 (Unix 时间戳) |

### 限流策略示例

```rust
// 默认限流配置
rate_limit: 1000,              // 每小时 1000 次请求
concurrent_limit: 5,           // 最大 5 个并发请求
window_duration: 3600 seconds  // 滑动窗口 1 小时
```

### 超出限流后的行为

当超出速率限制时：

1. **返回 429 错误**: 包含重试时间信息
2. **阻止请求**: 直到限流窗口重置
3. **记录日志**: 记录限流事件用于监控

**429 响应示例**:
```json
{
  "error": {
    "type": "rate_limit_error",
    "message": "Rate limit exceeded: 1000 requests per hour",
    "retry_after": 3600
  }
}
```

---

## 使用示例

### cURL 示例

#### Claude 流式请求
```bash
curl -X POST http://localhost:3000/api/v1/messages \
  -H "Authorization: Bearer cr_your_api_key_here" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 1024,
    "stream": true
  }'
```

#### Gemini 请求
```bash
curl -X POST http://localhost:3000/gemini/v1/models/gemini-1.5-pro:generateContent \
  -H "Authorization: Bearer cr_your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{"role": "user", "parts": [{"text": "Hello!"}]}],
    "generationConfig": {"maxOutputTokens": 1024}
  }'
```

#### OpenAI 兼容请求
```bash
curl -X POST http://localhost:3000/openai/v1/chat/completions \
  -H "Authorization: Bearer cr_your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 1024
  }'
```

### Python 示例

#### Claude 流式请求
```python
import requests
import json

headers = {
    "Authorization": "Bearer cr_your_api_key_here",
    "Content-Type": "application/json",
    "anthropic-version": "2023-06-01"
}

data = {
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 1024,
    "stream": True
}

response = requests.post(
    "http://localhost:3000/api/v1/messages",
    headers=headers,
    json=data,
    stream=True
)

for line in response.iter_lines():
    if line:
        line = line.decode('utf-8')
        if line.startswith('data: '):
            event_data = json.loads(line[6:])
            print(event_data)
```

#### Gemini 请求
```python
import requests

headers = {
    "Authorization": "Bearer cr_your_api_key_here",
    "Content-Type": "application/json"
}

data = {
    "contents": [{"role": "user", "parts": [{"text": "Hello!"}]}],
    "generationConfig": {"maxOutputTokens": 1024}
}

response = requests.post(
    "http://localhost:3000/gemini/v1/models/gemini-1.5-pro:generateContent",
    headers=headers,
    json=data
)

print(response.json())
```

### Node.js 示例

#### Claude 流式请求
```javascript
const axios = require('axios');

const headers = {
  'Authorization': 'Bearer cr_your_api_key_here',
  'Content-Type': 'application/json',
  'anthropic-version': '2023-06-01'
};

const data = {
  model: 'claude-3-5-sonnet-20241022',
  messages: [{ role: 'user', content: 'Hello!' }],
  max_tokens: 1024,
  stream: true
};

axios.post('http://localhost:3000/api/v1/messages', data, {
  headers,
  responseType: 'stream'
}).then(response => {
  response.data.on('data', chunk => {
    const lines = chunk.toString().split('\n');
    lines.forEach(line => {
      if (line.startsWith('data: ')) {
        const event = JSON.parse(line.slice(6));
        console.log(event);
      }
    });
  });
});
```

---

## 成本计算

系统自动计算每个请求的实时成本，基于以下因素：

### 定价模型

| Token 类型 | Claude 3.5 Sonnet ($/MTok) | Gemini 1.5 Pro ($/MTok) |
|-----------|---------------------------|------------------------|
| Input | $3.00 | $7.00 |
| Output | $15.00 | $21.00 |
| Cache Create | $3.75 | N/A |
| Cache Read | $0.30 | N/A |

### 成本计算公式

```
总成本 = (input_tokens × input_price) +
        (output_tokens × output_price) +
        (cache_create_tokens × cache_create_price) +
        (cache_read_tokens × cache_read_price)
```

### 成本查询

通过 `/api/v1/key-info` 端点查询 API Key 的累计成本：

```json
{
  "costs": {
    "total_cost": 0.05,
    "input_cost": 0.02,
    "output_cost": 0.03,
    "cache_create_cost": 0.005,
    "cache_read_cost": 0.001
  }
}
```

---

## 权限系统

### API Key 权限类型

| 权限 | 描述 | 可访问端点 |
|------|------|-----------|
| `all` | 全部权限 | Claude + Gemini + OpenAI |
| `claude` | 仅 Claude | `/api/v1/*`, `/claude/*` |
| `gemini` | 仅 Gemini | `/gemini/*` |
| `openai` | 仅 OpenAI | `/openai/*` |

### 权限配置示例

```rust
// API Key 创建时配置权限
ApiKey {
    id: "key_123",
    permissions: vec!["claude".to_string(), "gemini".to_string()],
    // 允许访问 Claude 和 Gemini，拒绝 OpenAI
}
```

### 权限验证流程

1. **提取 API Key**: 从 Authorization 头获取 Bearer token
2. **验证格式**: 检查 API Key 格式是否正确
3. **查询权限**: 从 Redis 获取 API Key 权限配置
4. **检查权限**: 验证当前端点是否在允许范围内
5. **允许/拒绝**: 返回 200 或 403

### 权限错误响应

```json
{
  "error": {
    "type": "permission_error",
    "message": "API key does not have permission to access this service"
  }
}
```

---

## 模型黑名单

### 黑名单配置

API Key 可以配置模型黑名单，禁止使用特定模型：

```rust
ApiKey {
    id: "key_123",
    model_blacklist: vec![
        "claude-3-opus-20240229".to_string(),
        "gemini-1.5-pro".to_string()
    ],
}
```

### 黑名单验证

当请求包含黑名单模型时，返回 403 错误：

```json
{
  "error": {
    "type": "permission_error",
    "message": "Model 'claude-3-opus-20240229' is in API key blacklist"
  }
}
```

---

## 客户端限制

### 支持的客户端

系统通过 User-Agent 识别客户端，并支持限制特定客户端访问：

| 客户端 | User-Agent 标识 |
|--------|----------------|
| Claude Code | `Claude Code/` |
| Gemini CLI | `gemini-cli/` |
| Cherry Studio | `Cherry Studio/` |
| OpenAI SDK | `openai-python/`, `openai-node/` |

### 客户端限制配置

```rust
ApiKey {
    id: "key_123",
    client_restrictions: vec![
        "ClaudeCode".to_string(),
        "Gemini-CLI".to_string()
    ],
    // 仅允许 Claude Code 和 Gemini CLI 客户端使用
}
```

### 客户端识别逻辑

```rust
// 从 User-Agent 提取客户端类型
if user_agent.contains("Claude Code/") {
    "ClaudeCode"
} else if user_agent.contains("gemini-cli/") {
    "Gemini-CLI"
} else if user_agent.contains("Cherry Studio/") {
    "CherryStudio"
} else {
    "Unknown"
}
```

---

## 会话管理

### 粘性会话

系统支持会话级别的账户绑定，确保同一会话使用同一账户：

#### 会话绑定流程

1. **提取会话标识**: 从请求头 `x-session-hash` 或 `anthropic-client-user-id` 获取
2. **查询绑定**: 检查 Redis 中是否存在会话到账户的绑定
3. **创建绑定**: 首次请求时绑定会话到选中的账户
4. **续期**: 后续请求自动续期会话 TTL

#### 会话头示例

```http
x-session-hash: abc123def456
```

或

```http
anthropic-client-user-id: user_789
```

#### 会话 TTL

- **默认 TTL**: 1 小时
- **自动续期**: 剩余时间 < 阈值时自动续期
- **可配置**: 通过环境变量调整

```bash
STICKY_SESSION_TTL_HOURS=1
STICKY_SESSION_RENEWAL_THRESHOLD_MINUTES=10
```

---

## 附录

### 环境变量配置

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `CRS_SERVER__HOST` | 服务监听地址 | `0.0.0.0` |
| `CRS_SERVER__PORT` | 服务监听端口 | `3000` |
| `CRS_REDIS__HOST` | Redis 主机地址 | `localhost` |
| `CRS_REDIS__PORT` | Redis 端口 | `6379` |
| `CRS_SECURITY__JWT_SECRET` | JWT 密钥 | 必填 |
| `CRS_SECURITY__ENCRYPTION_KEY` | AES 加密密钥 (32字符) | 必填 |
| `CRS_RATE_LIMIT__ENABLED` | 启用速率限制 | `true` |
| `CRS_RATE_LIMIT__DEFAULT_LIMIT` | 默认速率限制 | `1000` |

### 日志级别

通过 `RUST_LOG` 环境变量配置日志级别：

```bash
# 开发环境
RUST_LOG=debug cargo run

# 生产环境
RUST_LOG=info cargo run

# 详细调试
RUST_LOG=trace cargo run
```

### 性能建议

1. **启用 Redis 持久化**: 避免重启后数据丢失
2. **配置连接池**: 调整 Redis 连接池大小以提高并发性能
3. **监控内存使用**: 定期清理过期的速率限制和会话数据
4. **使用 Nginx 反向代理**: 启用 SSL、负载均衡和缓存
5. **启用 Prometheus 监控**: 实时监控系统性能指标

---

**文档版本**: 1.0.0
**最后更新**: 2025-10-31
**维护者**: Rust Migration Team
