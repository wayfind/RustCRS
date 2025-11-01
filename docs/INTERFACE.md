# Claude Relay Service API 接口文档

本文档详细描述了 Claude Relay Service 的前后端 API 接口规范。

## 目录

- [认证机制](#认证机制)
- [Claude API 转发](#claude-api-转发)
- [Gemini API 转发](#gemini-api-转发)
- [OpenAI API 转发](#openai-api-转发)
- [管理端点](#管理端点)
- [用户管理](#用户管理)
- [Webhook 通知](#webhook-通知)
- [错误处理](#错误处理)

---

## 认证机制

### API Key 认证（客户端请求）

**请求头**：
```http
Authorization: Bearer {API_KEY}
```

或使用自定义前缀（默认 `cr_`）：
```http
x-api-key: cr_xxxxxxxxxxxxx
```

### 管理员认证

**请求头**：
```http
Authorization: Bearer {ADMIN_TOKEN}
```

### 用户认证（用户管理系统）

**请求头**：
```http
Authorization: Bearer {SESSION_TOKEN}
```

---

## Claude API 转发

### 1. Claude Messages API

#### 端点
- `POST /api/v1/messages`
- `POST /claude/v1/messages` (别名)

#### 权限
- 需要 API Key 认证
- 权限：`all` 或 `claude`

#### 请求体
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "messages": [
    {
      "role": "user",
      "content": "Hello, Claude!"
    }
  ],
  "max_tokens": 4096,
  "temperature": 0.7,
  "stream": false,
  "system": "You are a helpful assistant"
}
```

**字段说明**：
- `model` (string, required): 模型名称
- `messages` (array, required): 对话消息数组
- `max_tokens` (integer, optional): 最大输出 token 数，默认 4096
- `temperature` (float, optional): 温度参数 (0-1)，默认 0.7
- `stream` (boolean, optional): 是否流式响应，默认 false
- `system` (string, optional): 系统提示词

#### 非流式响应
```json
{
  "id": "msg_xxxxx",
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
    "output_tokens": 25,
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 0
  }
}
```

#### 流式响应 (SSE)
```
Content-Type: text/event-stream

event: message_start
data: {"type":"message_start","message":{"id":"msg_xxxxx","type":"message","role":"assistant"}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}

event: message_stop
data: {"type":"message_stop"}
```

### 2. Token 计数 API

#### 端点
`POST /api/v1/messages/count_tokens`

#### 请求体
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "messages": [
    {
      "role": "user",
      "content": "Hello, Claude!"
    }
  ],
  "system": "You are a helpful assistant"
}
```

#### 响应
```json
{
  "input_tokens": 15
}
```

### 3. 模型列表

#### 端点
`GET /api/v1/models`

#### 响应
```json
{
  "object": "list",
  "data": [
    {
      "id": "claude-3-5-sonnet-20241022",
      "object": "model",
      "created": 1234567890,
      "owned_by": "anthropic"
    }
  ]
}
```

### 4. API Key 信息

#### 端点
`GET /api/v1/key-info`

#### 响应
```json
{
  "keyInfo": {
    "id": "key_xxxxx",
    "name": "My API Key",
    "tokenLimit": 1000000,
    "usage": {
      "totalTokens": 5000,
      "totalRequests": 10,
      "totalInputTokens": 3000,
      "totalOutputTokens": 2000
    }
  },
  "timestamp": "2025-10-31T12:00:00Z"
}
```

### 5. 使用统计

#### 端点
`GET /api/v1/usage`

#### 响应
```json
{
  "usage": {
    "totalTokens": 5000,
    "totalRequests": 10,
    "totalInputTokens": 3000,
    "totalOutputTokens": 2000,
    "totalCacheCreateTokens": 100,
    "totalCacheReadTokens": 200
  },
  "limits": {
    "tokens": 1000000,
    "requests": 0
  },
  "timestamp": "2025-10-31T12:00:00Z"
}
```

### 6. 用户信息（Claude Code 客户端）

#### 端点
`GET /api/v1/me`

#### 响应
```json
{
  "id": "user_xxxxx",
  "type": "user",
  "display_name": "My API Key",
  "created_at": "2025-10-31T12:00:00Z"
}
```

### 7. 组织使用统计

#### 端点
`GET /api/v1/organizations/:org_id/usage`

#### 响应
```json
{
  "object": "usage",
  "data": [
    {
      "type": "credit_balance",
      "credit_balance": 995000
    }
  ]
}
```

---

## Gemini API 转发

### 1. 简化消息 API

#### 端点
`POST /gemini/messages`

#### 请求体
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Hello, Gemini!"
    }
  ],
  "model": "gemini-2.5-flash",
  "temperature": 0.7,
  "max_tokens": 4096,
  "stream": false
}
```

#### 响应
```json
{
  "id": "gen_xxxxx",
  "model": "gemini-2.5-flash",
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 25,
    "total_tokens": 35
  }
}
```

### 2. Gemini 内部 API (v1internal)

#### Load Code Assist
- `POST /gemini/v1internal:loadCodeAssist`

#### Onboard User
- `POST /gemini/v1internal:onboardUser`

**请求体**：
```json
{
  "tierId": "free",
  "cloudaicompanionProject": "project-id",
  "metadata": {}
}
```

#### Count Tokens
- `POST /gemini/v1internal:countTokens`

**请求体**：
```json
{
  "contents": [
    {
      "role": "user",
      "parts": [{"text": "Hello"}]
    }
  ],
  "model": "gemini-2.5-flash"
}
```

**响应**：
```json
{
  "totalTokens": 10
}
```

#### Generate Content (非流式)
- `POST /gemini/v1internal:generateContent`

**请求体**：
```json
{
  "model": "gemini-2.5-flash",
  "project": "project-id",
  "user_prompt_id": "prompt-id",
  "request": {
    "contents": [
      {
        "role": "user",
        "parts": [{"text": "Hello, Gemini!"}]
      }
    ],
    "generationConfig": {
      "temperature": 0.7,
      "maxOutputTokens": 4096
    }
  }
}
```

#### Stream Generate Content (流式)
- `POST /gemini/v1internal:streamGenerateContent`

### 3. Gemini Beta API (v1beta)

所有 v1internal 端点都有对应的 v1beta 版本，格式为：
- `POST /gemini/v1beta/models/:modelName:loadCodeAssist`
- `POST /gemini/v1beta/models/:modelName:onboardUser`
- `POST /gemini/v1beta/models/:modelName:countTokens`
- `POST /gemini/v1beta/models/:modelName:generateContent`
- `POST /gemini/v1beta/models/:modelName:streamGenerateContent`

**路径参数**：
- `modelName`: 模型名称（如 `gemini-2.5-flash`）

### 4. Gemini 模型列表

#### 端点
`GET /gemini/models`

#### 响应
```json
{
  "object": "list",
  "data": [
    {
      "id": "gemini-2.5-flash",
      "object": "model",
      "created": 1234567890,
      "owned_by": "google"
    }
  ]
}
```

---

## OpenAI API 转发

### 1. OpenAI Responses API (Codex CLI)

#### 端点
- `POST /openai/responses`
- `POST /openai/v1/responses`

#### 权限
- 需要 API Key 认证
- 权限：`all` 或 `openai`

#### 请求体
```json
{
  "model": "gpt-5",
  "instructions": "You are a coding agent running in the Codex CLI...",
  "messages": [
    {
      "role": "user",
      "content": "Write a Python function to calculate fibonacci"
    }
  ],
  "stream": true
}
```

**自动适配**：
- 如果请求体不包含 Codex CLI 的 `instructions`，系统会自动添加标准的 Codex CLI instructions
- 模型名称自动标准化（如 `gpt-5-2025-08-07` → `gpt-5`）

#### 流式响应 (SSE)
```
Content-Type: text/event-stream

event: response.created
data: {"type":"response.created","response":{"id":"resp_xxxxx","model":"gpt-5"}}

event: response.output_item.added
data: {"type":"response.output_item.added","item":{"type":"message","role":"assistant"}}

event: response.content_part.delta
data: {"type":"response.content_part.delta","delta":{"text":"def fibonacci"}}

event: response.completed
data: {"type":"response.completed","response":{"usage":{"input_tokens":50,"output_tokens":150}}}
```

#### 非流式响应
```json
{
  "id": "resp_xxxxx",
  "model": "gpt-5",
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "def fibonacci(n):\n    ..."
      }
    }
  ],
  "usage": {
    "input_tokens": 50,
    "output_tokens": 150,
    "total_tokens": 200,
    "input_tokens_details": {
      "cached_tokens": 10
    }
  }
}
```

### 2. API Key 信息

#### 端点
`GET /openai/key-info`

#### 响应
```json
{
  "id": "key_xxxxx",
  "name": "My OpenAI Key",
  "permissions": "openai",
  "token_limit": 1000000,
  "tokens_used": 5000,
  "tokens_remaining": 995000,
  "rate_limit": {
    "window": 60,
    "requests": 100
  },
  "concurrency_limit": 10,
  "model_restrictions": {
    "enabled": false,
    "models": []
  }
}
```

---

## 管理端点

### 1. 系统健康检查

#### 端点
`GET /api/health`

#### 响应
```json
{
  "status": "healthy",
  "service": "claude-relay-service",
  "version": "2.0.0",
  "healthy": true,
  "redis": "connected",
  "uptime": 3600,
  "memory": {
    "used": 150000000,
    "total": 500000000
  },
  "timestamp": "2025-10-31T12:00:00Z"
}
```

### 2. 系统指标

#### 端点
`GET /metrics`

#### 响应
```json
{
  "uptime": 3600,
  "memory": {
    "rss": 150000000,
    "heapTotal": 100000000,
    "heapUsed": 80000000
  },
  "usage": {
    "totalRequests": 1000,
    "totalTokens": 500000,
    "totalCost": 2.5
  }
}
```

### 3. 仪表板数据

#### 端点
`GET /admin/dashboard`

#### 认证
需要管理员认证

#### 响应
```json
{
  "success": true,
  "data": {
    "overview": {
      "totalAccounts": 10,
      "activeAccounts": 8,
      "totalApiKeys": 50,
      "activeApiKeys": 45
    },
    "usage": {
      "last24Hours": {
        "requests": 5000,
        "tokens": 2000000,
        "cost": 10.5
      },
      "last7Days": {
        "requests": 30000,
        "tokens": 12000000,
        "cost": 65.2
      }
    },
    "accounts": [
      {
        "id": "acc_xxxxx",
        "name": "Account 1",
        "type": "claude-official",
        "status": "active",
        "isActive": true
      }
    ],
    "apiKeys": [
      {
        "id": "key_xxxxx",
        "name": "API Key 1",
        "isActive": true,
        "usage": {
          "totalTokens": 10000,
          "totalRequests": 50
        }
      }
    ]
  }
}
```

### 4. Claude 账户管理

#### 生成 OAuth 授权 URL
`POST /admin/claude-accounts/generate-auth-url`

**请求体**：
```json
{
  "proxy": {
    "protocol": "socks5",
    "host": "127.0.0.1",
    "port": 1080,
    "auth": {
      "username": "user",
      "password": "pass"
    }
  }
}
```

**响应**：
```json
{
  "success": true,
  "authUrl": "https://account.claude.ai/oauth/authorize?..."
}
```

#### 交换授权码
`POST /admin/claude-accounts/exchange-code`

**请求体**：
```json
{
  "code": "authorization_code_from_oauth",
  "proxy": {
    "protocol": "socks5",
    "host": "127.0.0.1",
    "port": 1080
  }
}
```

**响应**：
```json
{
  "success": true,
  "tokens": {
    "accessToken": "eyJhbGc...",
    "refreshToken": "refresh_token...",
    "expiresAt": "2025-11-01T12:00:00Z",
    "scopes": ["read", "write"]
  }
}
```

#### 创建 Claude 账户
`POST /admin/claude-accounts`

**请求体**：
```json
{
  "name": "My Claude Account",
  "description": "Production account",
  "accountType": "claude-official",
  "proxy": {
    "protocol": "socks5",
    "host": "127.0.0.1",
    "port": 1080
  },
  "claudeAiOauth": {
    "accessToken": "eyJhbGc...",
    "refreshToken": "refresh_token...",
    "expiresAt": "2025-11-01T12:00:00Z",
    "scopes": ["read", "write"]
  },
  "isActive": true,
  "priority": 10
}
```

#### 获取账户列表
`GET /admin/claude-accounts`

**查询参数**：
- `type`: 账户类型筛选（`claude-official`, `claude-console`, `bedrock`, `ccr`）
- `status`: 状态筛选（`active`, `inactive`, `rate_limited`）

**响应**：
```json
{
  "success": true,
  "accounts": [
    {
      "id": "acc_xxxxx",
      "name": "My Claude Account",
      "accountType": "claude-official",
      "status": "active",
      "isActive": true,
      "priority": 10,
      "createdAt": "2025-10-01T12:00:00Z",
      "lastUsedAt": "2025-10-31T11:00:00Z"
    }
  ],
  "total": 1
}
```

#### 更新账户
`PATCH /admin/claude-accounts/:accountId`

#### 删除账户
`DELETE /admin/claude-accounts/:accountId`

### 5. API Key 管理

#### 创建 API Key
`POST /admin/api-keys`

**请求体**：
```json
{
  "name": "Production Key",
  "description": "Key for production use",
  "tokenLimit": 1000000,
  "expiresAt": "2026-10-31T23:59:59Z",
  "permissions": "all",
  "allowedClients": ["ClaudeCode", "GeminiCLI"],
  "enableModelRestriction": false,
  "restrictedModels": [],
  "rateLimitWindow": 60,
  "rateLimitRequests": 100,
  "concurrencyLimit": 10,
  "dailyCostLimit": 10.0,
  "totalCostLimit": 100.0
}
```

**响应**：
```json
{
  "success": true,
  "apiKey": {
    "id": "key_xxxxx",
    "key": "cr_xxxxxxxxxxxxxxxxxx",
    "name": "Production Key",
    "createdAt": "2025-10-31T12:00:00Z"
  }
}
```

#### 获取 API Key 列表
`GET /admin/api-keys`

#### 更新 API Key
`PATCH /admin/api-keys/:keyId`

#### 删除 API Key
`DELETE /admin/api-keys/:keyId`

#### 重置 API Key 统计
`POST /admin/api-keys/:keyId/reset-stats`

**请求体**：
```json
{
  "resetType": "daily"
}
```

**resetType 选项**：
- `daily`: 重置每日统计
- `weekly`: 重置每周统计（Opus成本）
- `total`: 重置总统计

---

## 用户管理

### 1. 用户登录

#### 端点
`POST /users/login`

#### 请求体
```json
{
  "username": "john.doe",
  "password": "secure_password"
}
```

#### 响应
```json
{
  "success": true,
  "message": "Login successful",
  "user": {
    "id": "user_xxxxx",
    "username": "john.doe",
    "email": "john@example.com",
    "displayName": "John Doe",
    "firstName": "John",
    "lastName": "Doe",
    "role": "user"
  },
  "sessionToken": "session_token_xxxxx"
}
```

#### 错误响应
```json
{
  "error": "Authentication failed",
  "message": "Invalid username or password"
}
```

**速率限制**：
- 每个 IP: 30 次/15 分钟（正常限制）
- 每个 IP: 100 次/1 小时（严格限制，防暴力破解）

### 2. 用户登出

#### 端点
`POST /users/logout`

#### 认证
需要用户会话 token

#### 响应
```json
{
  "success": true,
  "message": "Logout successful"
}
```

### 3. 获取用户资料

#### 端点
`GET /users/profile`

#### 响应
```json
{
  "success": true,
  "user": {
    "id": "user_xxxxx",
    "username": "john.doe",
    "email": "john@example.com",
    "displayName": "John Doe",
    "firstName": "John",
    "lastName": "Doe",
    "role": "user",
    "isActive": true,
    "createdAt": "2025-01-01T00:00:00Z",
    "lastLoginAt": "2025-10-31T12:00:00Z",
    "apiKeyCount": 3,
    "totalUsage": {
      "totalTokens": 50000,
      "totalRequests": 100,
      "totalCost": 2.5
    }
  },
  "config": {
    "maxApiKeysPerUser": 5,
    "allowUserDeleteApiKeys": true
  }
}
```

### 4. 获取用户的 API Keys

#### 端点
`GET /users/api-keys`

#### 查询参数
- `includeDeleted`: 是否包含已删除的 Key（`true`/`false`）

#### 响应
```json
{
  "success": true,
  "apiKeys": [
    {
      "id": "key_xxxxx",
      "name": "My Key",
      "description": "Personal use key",
      "tokenLimit": 100000,
      "isActive": true,
      "createdAt": "2025-10-01T00:00:00Z",
      "lastUsedAt": "2025-10-31T11:00:00Z",
      "expiresAt": null,
      "usage": {
        "requests": 50,
        "inputTokens": 3000,
        "outputTokens": 2000,
        "totalCost": 0.25
      },
      "dailyCost": 0.05,
      "dailyCostLimit": 1.0,
      "totalCost": 0.25,
      "totalCostLimit": 10.0,
      "keyPreview": "cr_abcd...xyz"
    }
  ],
  "total": 1
}
```

### 5. 创建 API Key

#### 端点
`POST /users/api-keys`

#### 请求体
```json
{
  "name": "My New Key",
  "description": "For testing",
  "tokenLimit": 50000,
  "expiresAt": "2026-10-31T23:59:59Z",
  "dailyCostLimit": 1.0,
  "totalCostLimit": 10.0
}
```

#### 响应
```json
{
  "success": true,
  "message": "API key created successfully",
  "apiKey": {
    "id": "key_xxxxx",
    "name": "My New Key",
    "description": "For testing",
    "key": "cr_xxxxxxxxxxxxxxxxxx",
    "tokenLimit": 50000,
    "expiresAt": "2026-10-31T23:59:59Z",
    "dailyCostLimit": 1.0,
    "totalCostLimit": 10.0,
    "createdAt": "2025-10-31T12:00:00Z"
  }
}
```

**注意**：
- 完整的 API Key 仅在创建时返回一次
- 用户创建的 Key 自动拥有 `all` 权限（访问所有服务）
- 受 `maxApiKeysPerUser` 配置限制

### 6. 删除 API Key

#### 端点
`DELETE /users/api-keys/:keyId`

#### 响应
```json
{
  "success": true,
  "message": "API key deleted successfully"
}
```

**注意**：
- 需要 `allowUserDeleteApiKeys` 配置启用
- 只能删除属于当前用户的 Key

### 7. 获取使用统计

#### 端点
`GET /users/usage-stats`

#### 查询参数
- `period`: 统计周期（`day`, `week`, `month`），默认 `week`
- `model`: 按模型筛选（可选）

#### 响应
```json
{
  "success": true,
  "stats": {
    "totalRequests": 100,
    "totalInputTokens": 50000,
    "totalOutputTokens": 30000,
    "totalCost": 2.5,
    "dailyStats": [
      {
        "date": "2025-10-31",
        "requests": 20,
        "inputTokens": 10000,
        "outputTokens": 6000,
        "cost": 0.5
      }
    ],
    "modelStats": [
      {
        "model": "claude-3-5-sonnet-20241022",
        "requests": 50,
        "inputTokens": 30000,
        "outputTokens": 18000,
        "cost": 1.5
      }
    ]
  }
}
```

### 8. 管理员：获取用户列表

#### 端点
`GET /users`

#### 认证
需要管理员权限

#### 查询参数
- `page`: 页码，默认 1
- `limit`: 每页数量，默认 20
- `role`: 角色筛选（`user`, `admin`）
- `isActive`: 状态筛选（`true`, `false`）
- `search`: 搜索关键词（用户名、邮箱、显示名）

#### 响应
```json
{
  "success": true,
  "users": [
    {
      "id": "user_xxxxx",
      "username": "john.doe",
      "email": "john@example.com",
      "displayName": "John Doe",
      "role": "user",
      "isActive": true,
      "apiKeyCount": 3,
      "createdAt": "2025-01-01T00:00:00Z",
      "lastLoginAt": "2025-10-31T12:00:00Z"
    }
  ],
  "pagination": {
    "total": 50,
    "page": 1,
    "limit": 20,
    "totalPages": 3
  }
}
```

### 9. 管理员：更新用户状态

#### 端点
`PATCH /users/:userId/status`

#### 请求体
```json
{
  "isActive": false
}
```

#### 响应
```json
{
  "success": true,
  "message": "User disabled successfully",
  "user": {
    "id": "user_xxxxx",
    "username": "john.doe",
    "isActive": false,
    "updatedAt": "2025-10-31T12:00:00Z"
  }
}
```

### 10. 管理员：更新用户角色

#### 端点
`PATCH /users/:userId/role`

#### 请求体
```json
{
  "role": "admin"
}
```

**有效角色**：
- `user`: 普通用户
- `admin`: 管理员

---

## Webhook 通知

### 1. 获取 Webhook 配置

#### 端点
`GET /admin/webhook/config`

#### 认证
需要管理员认证

#### 响应
```json
{
  "success": true,
  "config": {
    "enabled": true,
    "platforms": [
      {
        "id": "platform_xxxxx",
        "name": "钉钉通知",
        "type": "dingtalk",
        "url": "https://oapi.dingtalk.com/robot/send?access_token=xxxxx",
        "secret": "SECxxxxx",
        "enableSign": true,
        "enabled": true,
        "timeout": 10000,
        "events": ["account.failed", "token.refresh.failed"]
      }
    ]
  }
}
```

### 2. 保存 Webhook 配置

#### 端点
`POST /admin/webhook/config`

#### 请求体
```json
{
  "enabled": true,
  "platforms": [
    {
      "id": "platform_xxxxx",
      "name": "钉钉通知",
      "type": "dingtalk",
      "url": "https://oapi.dingtalk.com/robot/send?access_token=xxxxx",
      "secret": "SECxxxxx",
      "enableSign": true,
      "enabled": true,
      "timeout": 10000,
      "events": ["account.failed", "token.refresh.failed"]
    }
  ]
}
```

#### 响应
```json
{
  "success": true,
  "message": "Webhook配置已保存",
  "config": { ... }
}
```

### 3. 添加 Webhook 平台

#### 端点
`POST /admin/webhook/platforms`

#### 请求体（钉钉）
```json
{
  "name": "钉钉通知",
  "type": "dingtalk",
  "url": "https://oapi.dingtalk.com/robot/send?access_token=xxxxx",
  "secret": "SECxxxxx",
  "enableSign": true,
  "enabled": true,
  "timeout": 10000,
  "events": ["account.failed"]
}
```

#### 请求体（企业微信）
```json
{
  "name": "企业微信",
  "type": "wecom",
  "url": "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxxxx",
  "enabled": true,
  "timeout": 10000,
  "events": ["account.failed"]
}
```

#### 请求体（飞书）
```json
{
  "name": "飞书",
  "type": "feishu",
  "url": "https://open.feishu.cn/open-apis/bot/v2/hook/xxxxx",
  "secret": "xxxxx",
  "enableSign": true,
  "enabled": true,
  "timeout": 10000,
  "events": ["token.refresh.failed"]
}
```

#### 请求体（Bark）
```json
{
  "name": "Bark iOS",
  "type": "bark",
  "deviceKey": "xxxxxxxxxx",
  "serverUrl": "https://api.day.app",
  "level": "active",
  "sound": "bell",
  "group": "claude-relay",
  "enabled": true,
  "timeout": 10000,
  "events": ["account.failed"]
}
```

#### 请求体（SMTP 邮件）
```json
{
  "name": "Email Notification",
  "type": "smtp",
  "host": "smtp.gmail.com",
  "port": 587,
  "secure": false,
  "user": "your-email@gmail.com",
  "pass": "your-password",
  "from": "Claude Relay <your-email@gmail.com>",
  "to": ["admin@example.com"],
  "ignoreTLS": false,
  "enabled": true,
  "timeout": 10000,
  "events": ["account.failed"]
}
```

#### 请求体（Telegram）
```json
{
  "name": "Telegram Bot",
  "type": "telegram",
  "botToken": "123456789:ABCdefGHIjklMNOpqrsTUVwxyz",
  "chatId": "-1001234567890",
  "apiBaseUrl": "https://api.telegram.org",
  "proxyUrl": "socks5://127.0.0.1:1080",
  "enabled": true,
  "timeout": 10000,
  "events": ["account.failed"]
}
```

#### 响应
```json
{
  "success": true,
  "message": "Webhook平台已添加",
  "platform": {
    "id": "platform_xxxxx",
    "name": "钉钉通知",
    "type": "dingtalk",
    "enabled": true,
    "createdAt": "2025-10-31T12:00:00Z"
  }
}
```

### 4. 更新 Webhook 平台

#### 端点
`PUT /admin/webhook/platforms/:id`

### 5. 删除 Webhook 平台

#### 端点
`DELETE /admin/webhook/platforms/:id`

### 6. 切换平台启用状态

#### 端点
`POST /admin/webhook/platforms/:id/toggle`

#### 响应
```json
{
  "success": true,
  "message": "Webhook平台已启用",
  "platform": {
    "id": "platform_xxxxx",
    "enabled": true
  }
}
```

### 7. 测试 Webhook 连通性

#### 端点
`POST /admin/webhook/test`

#### 请求体（钉钉）
```json
{
  "type": "dingtalk",
  "url": "https://oapi.dingtalk.com/robot/send?access_token=xxxxx",
  "secret": "SECxxxxx",
  "enableSign": true
}
```

#### 响应
```json
{
  "success": true,
  "message": "Webhook测试成功",
  "url": "https://oapi.dingtalk.com/robot/send?access_token=xxxxx"
}
```

### 8. 发送测试通知

#### 端点
`POST /admin/webhook/test-notification`

#### 请求体
```json
{
  "type": "test",
  "accountId": "test-account-id",
  "accountName": "测试账号",
  "platform": "claude-oauth",
  "status": "test",
  "errorCode": "TEST_NOTIFICATION",
  "reason": "手动测试通知",
  "message": "这是一条测试通知消息"
}
```

#### 响应
```json
{
  "success": true,
  "message": "测试通知已成功发送到 2 个平台",
  "data": {
    "accountId": "test-account-id",
    "accountName": "测试账号",
    "timestamp": "2025-10-31T12:00:00+08:00"
  },
  "result": {
    "succeeded": 2,
    "failed": 0
  }
}
```

### Webhook 事件类型

系统支持以下通知事件：

| 事件类型 | 描述 | 通知数据 |
|---------|------|---------|
| `account.failed` | 账户失败（如 token 失效） | accountId, accountName, platform, errorCode, reason |
| `token.refresh.failed` | Token 刷新失败 | accountId, accountName, platform, errorCode, reason |
| `rate_limit.exceeded` | API 速率限制超出 | accountId, accountName, platform, errorCode, reason |
| `account.unauthorized` | 账户认证失败（401/402） | accountId, accountName, platform, errorCode, reason |
| `test` | 测试通知 | 自定义测试数据 |

---

## 错误处理

### 标准错误响应格式

```json
{
  "error": "Error type",
  "message": "Human-readable error message",
  "code": "ERROR_CODE",
  "details": {}
}
```

### HTTP 状态码

| 状态码 | 含义 | 说明 |
|--------|------|------|
| 200 | OK | 请求成功 |
| 201 | Created | 资源创建成功 |
| 400 | Bad Request | 请求参数错误 |
| 401 | Unauthorized | 未认证或认证失败 |
| 403 | Forbidden | 权限不足 |
| 404 | Not Found | 资源不存在 |
| 429 | Too Many Requests | 速率限制超出 |
| 500 | Internal Server Error | 服务器内部错误 |
| 502 | Bad Gateway | 上游服务错误 |
| 503 | Service Unavailable | 服务不可用 |
| 504 | Gateway Timeout | 上游服务超时 |

### 常见错误码

#### 认证相关
- `INVALID_API_KEY`: 无效的 API Key
- `API_KEY_DISABLED`: API Key 已禁用
- `API_KEY_EXPIRED`: API Key 已过期
- `PERMISSION_DENIED`: 权限不足
- `UNAUTHORIZED`: 未认证

#### 速率限制
- `RATE_LIMIT_EXCEEDED`: 速率限制超出
- `TOKEN_LIMIT_EXCEEDED`: Token 配额超出
- `CONCURRENCY_LIMIT_EXCEEDED`: 并发限制超出
- `COST_LIMIT_EXCEEDED`: 成本限制超出

#### 账户相关
- `NO_AVAILABLE_ACCOUNT`: 无可用账户
- `ACCOUNT_RATE_LIMITED`: 账户被限流
- `ACCOUNT_UNAUTHORIZED`: 账户认证失败
- `ACCOUNT_DISABLED`: 账户已禁用

#### 请求相关
- `INVALID_REQUEST`: 无效请求
- `MISSING_PARAMETER`: 缺少必需参数
- `INVALID_MODEL`: 无效的模型名称
- `MODEL_RESTRICTED`: 模型访问受限

#### 系统相关
- `UPSTREAM_ERROR`: 上游服务错误
- `REDIS_ERROR`: Redis 连接错误
- `INTERNAL_ERROR`: 内部服务错误

### 错误响应示例

#### API Key 无效
```json
{
  "error": "authentication_error",
  "message": "Invalid API key",
  "code": "INVALID_API_KEY"
}
```

#### 速率限制超出
```json
{
  "error": "rate_limit_exceeded",
  "message": "Rate limit exceeded for this API key",
  "code": "RATE_LIMIT_EXCEEDED",
  "details": {
    "retryAfter": 60,
    "limit": 100,
    "window": 60
  }
}
```

#### 成本限制超出
```json
{
  "error": "cost_limit_exceeded",
  "message": "Daily cost limit exceeded",
  "code": "DAILY_COST_LIMIT_EXCEEDED",
  "details": {
    "dailyCost": 10.5,
    "dailyCostLimit": 10.0
  }
}
```

#### 无可用账户
```json
{
  "error": "service_unavailable",
  "message": "No available Claude accounts",
  "code": "NO_AVAILABLE_ACCOUNT"
}
```

#### 上游服务错误（Claude 529）
```json
{
  "error": "upstream_overloaded",
  "message": "Claude service is temporarily overloaded. This account will be retried in 30 minutes.",
  "code": "CLAUDE_OVERLOADED",
  "details": {
    "accountId": "acc_xxxxx",
    "retryAfter": 1800
  }
}
```

---

## 附录

### A. 支持的模型

#### Claude 模型
- `claude-3-5-sonnet-20241022`
- `claude-3-5-haiku-20241022`
- `claude-3-opus-20240229`
- `claude-3-sonnet-20240229`
- `claude-3-haiku-20240307`

#### Gemini 模型
- `gemini-2.5-flash`
- `gemini-2.5-pro`
- `gemini-1.5-flash`
- `gemini-1.5-pro`

#### OpenAI 模型
- `gpt-5`
- `gpt-5-codex`
- `gpt-4o`
- `gpt-4-turbo`

### B. 代理配置格式

```json
{
  "protocol": "socks5",
  "host": "127.0.0.1",
  "port": 1080,
  "auth": {
    "username": "user",
    "password": "pass"
  }
}
```

**支持的协议**：
- `socks5`: SOCKS5 代理
- `http`: HTTP 代理
- `https`: HTTPS 代理

### C. 粘性会话

系统支持粘性会话（Sticky Session），确保同一会话的请求使用同一账户，保证上下文连续性。

**会话哈希生成**：
基于请求内容（messages、system 等）生成 SHA-256 哈希值

**配置参数**：
- `STICKY_SESSION_TTL_HOURS`: 会话 TTL（小时），默认 1
- `STICKY_SESSION_RENEWAL_THRESHOLD_MINUTES`: 续期阈值（分钟），默认 0

**工作原理**：
1. 首次请求时，根据会话哈希选择账户并绑定
2. 后续相同会话的请求使用绑定的账户
3. 会话过期或账户不可用时重新选择

### D. 客户端限制

API Key 可以限制允许的客户端类型（基于 User-Agent）：

**预定义客户端**：
- `ClaudeCode`: Claude Code 客户端
- `GeminiCLI`: Gemini CLI 客户端
- `Postman`: Postman 测试工具
- `curl`: curl 命令行工具

**配置示例**：
```json
{
  "allowedClients": ["ClaudeCode", "GeminiCLI"]
}
```

### E. 模型黑名单

API Key 可以限制禁止访问的模型：

**配置示例**：
```json
{
  "enableModelRestriction": true,
  "restrictedModels": ["claude-3-opus-20240229", "gpt-5"]
}
```

**工作原理**：
- 系统自动移除模型名称中的供应商前缀（如 `ccr,gemini-2.5-pro` → `gemini-2.5-pro`）
- 检查有效模型名称是否在黑名单中
- 如果在黑名单中，返回 403 错误

---

## 更新日志

### v2.0.0 (2025-10-31)
- ✅ 完整的多平台支持（Claude、Gemini、OpenAI、Bedrock、Azure、Droid、CCR）
- ✅ 用户管理系统（LDAP 认证、会话管理）
- ✅ Webhook 通知系统（支持 8 种平台）
- ✅ 统一调度器（智能账户选择、粘性会话、并发控制）
- ✅ 成本追踪和定价服务
- ✅ 详细的使用统计和监控
- ✅ 客户端限制和模型黑名单
- ✅ 速率限制和配额管理

---

## 技术支持

- **文档**: `/docs`
- **GitHub**: https://github.com/your-org/claude-relay-service
- **问题反馈**: https://github.com/your-org/claude-relay-service/issues
