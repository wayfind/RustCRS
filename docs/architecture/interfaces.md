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
