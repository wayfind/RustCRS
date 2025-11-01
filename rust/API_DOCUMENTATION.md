# Claude Relay Service - Complete API Documentation

**Version:** 1.1.187
**Last Updated:** 2025-10-30
**Base URL:** `http://localhost:3000` (default)

---

## Table of Contents

1. [Authentication](#authentication)
2. [Claude API Routes](#claude-api-routes)
3. [Gemini API Routes](#gemini-api-routes)
4. [OpenAI Compatible Routes](#openai-compatible-routes)
5. [Azure OpenAI Routes](#azure-openai-routes)
6. [Droid (Factory.ai) Routes](#droid-factoryai-routes)
7. [Unified Routes](#unified-routes)
8. [User Management Routes](#user-management-routes)
9. [API Stats Routes](#api-stats-routes)
10. [Admin Management Routes](#admin-management-routes)
11. [Webhook Routes](#webhook-routes)
12. [Health Check & Metrics](#health-check--metrics)

---

## Authentication

All API endpoints require authentication via API Key, User Session Token, or Admin credentials.

### API Key Authentication

**Header:**
```
Authorization: Bearer cr_your_api_key_here
```

or

```
x-api-key: cr_your_api_key_here
```

### User Session Authentication

**Header:**
```
x-session-token: your_session_token_here
```

### Admin Authentication

**Header:**
```
x-admin-token: your_admin_token_here
```

---

## Claude API Routes

Base paths: `/api/v1/*` and `/claude/v1/*`

### POST /api/v1/messages

Send messages to Claude models with support for streaming.

**Authentication:** API Key (with `all` or `claude` permissions)

**Request Body:**
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

**Required Fields:**
- `messages` (array): Conversation messages

**Optional Fields:**
- `model` (string): Claude model name (default: `claude-3-5-sonnet-20241022`)
- `max_tokens` (number): Maximum output tokens (default: 4096)
- `temperature` (number): 0.0-1.0, controls randomness
- `stream` (boolean): Enable streaming response
- `system` (string): System prompt

**Response (Non-Streaming):**
```json
{
  "id": "msg_abc123",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Hello! How can I help you today?"
    }
  ],
  "model": "claude-3-5-sonnet-20241022",
  "usage": {
    "input_tokens": 10,
    "output_tokens": 20,
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 0
  }
}
```

**Response (Streaming):**
```
event: message_start
data: {"type":"message_start","message":{"id":"msg_abc123",...}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{...}}

event: message_stop
data: {"type":"message_stop"}
```

**Error Responses:**
- `400 Bad Request`: Invalid request body or missing messages
- `403 Forbidden`: API key lacks Claude permissions or model is blacklisted
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Service error
- `503 Service Unavailable`: No available Claude accounts

**Supported Models:**
- `claude-3-5-sonnet-20241022` (recommended)
- `claude-3-5-haiku-20241022`
- `claude-opus-4-20250514`
- `claude-3-opus-20240229`
- `claude-3-sonnet-20240229`
- `claude-3-haiku-20240307`

---

### POST /v1/messages/count_tokens

Count tokens for a Claude message request (Beta API).

**Authentication:** API Key (with `all` or `claude` permissions)

**Request Body:**
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

**Response:**
```json
{
  "input_tokens": 15,
  "output_tokens": 0
}
```

**Errors:**
- `403 Forbidden`: API key lacks Claude permissions
- `501 Not Implemented`: Bedrock or CCR accounts don't support token counting

---

### GET /api/v1/models

Get list of available models.

**Authentication:** API Key

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "claude-3-5-sonnet-20241022",
      "object": "model",
      "created": 1704067200,
      "owned_by": "anthropic"
    },
    {
      "id": "gemini-2.5-flash",
      "object": "model",
      "created": 1704067200,
      "owned_by": "google"
    }
  ]
}
```

**Notes:**
- Returns models filtered by API Key permissions and restrictions
- Includes Claude, Gemini, and OpenAI models based on available accounts

---

### GET /v1/me

Get current user information (required by Claude Code client).

**Authentication:** API Key

**Response:**
```json
{
  "id": "user_abc123",
  "type": "user",
  "display_name": "My API Key",
  "created_at": "2025-10-30T00:00:00Z"
}
```

---

### GET /v1/organizations/:org_id/usage

Get organization usage and credit balance.

**Authentication:** API Key

**Response:**
```json
{
  "object": "usage",
  "data": [
    {
      "type": "credit_balance",
      "credit_balance": 9500000
    }
  ]
}
```

**Notes:**
- Returns token limit minus total usage
- Required by Claude Code client for balance display

---

### GET /v1/usage

Get detailed usage statistics for the API Key.

**Authentication:** API Key

**Response:**
```json
{
  "usage": {
    "total": {
      "requests": 100,
      "tokens": 50000,
      "allTokens": 50000,
      "inputTokens": 30000,
      "outputTokens": 20000,
      "cacheCreateTokens": 0,
      "cacheReadTokens": 0
    },
    "daily": { ... },
    "monthly": { ... }
  },
  "limits": {
    "tokens": 1000000,
    "requests": 0
  },
  "timestamp": "2025-10-30T00:00:00Z"
}
```

---

### GET /v1/key-info

Get API Key information and current usage.

**Authentication:** API Key

**Response:**
```json
{
  "keyInfo": {
    "id": "abc-123-def",
    "name": "My API Key",
    "tokenLimit": 1000000,
    "usage": {
      "total": {
        "tokens": 50000,
        "requests": 100
      }
    }
  },
  "timestamp": "2025-10-30T00:00:00Z"
}
```

---

## Gemini API Routes

Base path: `/gemini/*`

### POST /gemini/messages

Send messages to Gemini models (internal format).

**Authentication:** API Key (with `all` or `gemini` permissions)

**Request Body:**
```json
{
  "model": "gemini-2.5-flash",
  "messages": [
    {
      "role": "user",
      "content": "Hello, Gemini!"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 4096,
  "stream": false
}
```

**Response:**
```json
{
  "id": "gen_abc123",
  "object": "chat.completion",
  "created": 1704067200,
  "model": "gemini-2.5-flash",
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
    "completion_tokens": 20,
    "total_tokens": 30
  }
}
```

---

### POST /gemini/v1beta/models/:modelName:generateContent

Standard Gemini API format for content generation.

**Authentication:** API Key (with `all` or `gemini` permissions)

**Request Body:**
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
    "temperature": 0.7,
    "maxOutputTokens": 4096,
    "topP": 0.95,
    "topK": 40
  },
  "safetySettings": [],
  "systemInstruction": {
    "parts": [
      {
        "text": "You are a helpful assistant"
      }
    ]
  },
  "tools": []
}
```

**Response:**
```json
{
  "candidates": [
    {
      "content": {
        "parts": [
          {
            "text": "Hello! How can I help you today?"
          }
        ],
        "role": "model"
      },
      "finishReason": "STOP",
      "index": 0
    }
  ],
  "usageMetadata": {
    "promptTokenCount": 10,
    "candidatesTokenCount": 20,
    "totalTokenCount": 30
  }
}
```

**Supported Actions:**
- `:generateContent` - Non-streaming generation
- `:streamGenerateContent` - Streaming generation
- `:countTokens` - Token counting
- `:loadCodeAssist` - Load code assist (Gemini CLI)
- `:onboardUser` - User onboarding (Gemini CLI)

---

### POST /gemini/v1internal:generateContent

Internal Gemini API format (used by Gemini CLI).

**Authentication:** API Key (with `all` or `gemini` permissions)

**Request Body:**
```json
{
  "model": "gemini-2.5-flash",
  "request": {
    "contents": [...],
    "generationConfig": {...}
  },
  "project": "projects/your-project-id",
  "user_prompt_id": "prompt_123"
}
```

**Response:**
```json
{
  "response": {
    "candidates": [...],
    "usageMetadata": {...}
  },
  "cloudaicompanionProject": "projects/your-project-id"
}
```

---

### GET /gemini/models

Get available Gemini models.

**Authentication:** API Key (with `all` or `gemini` permissions)

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "gemini-2.5-flash",
      "object": "model",
      "created": 1704067200,
      "owned_by": "google"
    },
    {
      "id": "gemini-2.0-flash-exp",
      "object": "model",
      "created": 1704067200,
      "owned_by": "google"
    }
  ]
}
```

---

## OpenAI Compatible Routes

Base path: `/openai/*`

### POST /openai/v1/chat/completions

OpenAI-compatible chat completions endpoint.

**Authentication:** API Key (with `all` or `openai` permissions)

**Request Body:**
```json
{
  "model": "gpt-5-2025-08-07",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 4096,
  "stream": false
}
```

**Response:**
```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1704067200,
  "model": "gpt-5-2025-08-07",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you?"
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

**Supported Models:**
- `gpt-5-2025-08-07`
- `gpt-5-mini-2025-08-07`
- `gpt-4o-2024-11-20`
- `gpt-4o-mini`
- `codex-mini-2025-08-07`

---

### POST /openai/v1/responses

OpenAI Responses API endpoint (for gpt-5, codex-mini).

**Authentication:** API Key (with `all` or `openai` permissions)

**Request Body:**
```json
{
  "model": "gpt-5-2025-08-07",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "stream": false
}
```

**Response:**
Same format as `/chat/completions`

**Notes:**
- Special endpoint for OpenAI Responses format
- Supports streaming via SSE
- Returns Codex-specific usage headers (`x-codex-*`)

---

### POST /openai/claude/v1/chat/completions

Route OpenAI format to Claude backend.

**Authentication:** API Key (with `all` or `claude` permissions)

**Request Body:**
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ]
}
```

**Response:**
OpenAI-compatible format with Claude content

---

### POST /openai/gemini/v1/chat/completions

Route OpenAI format to Gemini backend.

**Authentication:** API Key (with `all` or `gemini` permissions)

**Request Body:**
```json
{
  "model": "gemini-2.5-flash",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ]
}
```

**Response:**
OpenAI-compatible format with Gemini content

---

### GET /openai/v1/models

Get available OpenAI-compatible models.

**Authentication:** API Key

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "openai/gpt-5-2025-08-07",
      "object": "model",
      "created": 1704067200,
      "owned_by": "openai"
    }
  ]
}
```

---

## Azure OpenAI Routes

Base path: `/azure/*`

### POST /azure/chat/completions

Azure OpenAI chat completions.

**Authentication:** API Key (with `all` or `azure` permissions)

**Request Body:**
```json
{
  "model": "gpt-4o",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 4096,
  "stream": false
}
```

**Response:**
OpenAI-compatible format

**Supported Models:**
- `gpt-4`, `gpt-4-turbo`, `gpt-4o`, `gpt-4o-mini`
- `gpt-5`, `gpt-5-mini`
- `gpt-35-turbo`, `gpt-35-turbo-16k`
- `codex-mini`

---

### POST /azure/responses

Azure OpenAI Responses API.

**Authentication:** API Key

**Request Body:**
Same as `/chat/completions`

**Response:**
OpenAI-compatible format

---

### POST /azure/embeddings

Azure OpenAI embeddings.

**Authentication:** API Key

**Request Body:**
```json
{
  "model": "text-embedding-3-small",
  "input": "Hello, world!"
}
```

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "embedding": [0.1, 0.2, ...],
      "index": 0
    }
  ],
  "model": "text-embedding-3-small",
  "usage": {
    "prompt_tokens": 5,
    "total_tokens": 5
  }
}
```

---

### GET /azure/models

Get available Azure OpenAI models.

**Authentication:** API Key

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "azure/gpt-4o",
      "object": "model",
      "created": 1704067200,
      "owned_by": "azure-openai"
    }
  ]
}
```

---

### GET /azure/health

Azure OpenAI health check.

**Response:**
```json
{
  "status": "healthy",
  "service": "azure-openai-relay",
  "timestamp": "2025-10-30T00:00:00Z"
}
```

---

## Droid (Factory.ai) Routes

Base path: `/droid/*`

### POST /droid/claude/v1/messages

Droid-proxied Claude messages API.

**Authentication:** API Key (with `all` or `droid` permissions)

**Request Body:**
```json
{
  "model": "claude-opus-4-1-20250805",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "stream": false
}
```

**Response:**
Standard Claude Messages API format

**Supported Models:**
- `claude-opus-4-1-20250805`
- `claude-sonnet-4-5-20250929`

---

### POST /droid/openai/v1/responses

Droid-proxied OpenAI Responses API.

**Authentication:** API Key (with `all` or `droid` permissions)

**Request Body:**
```json
{
  "model": "gpt-5-2025-08-07",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "stream": false
}
```

**Response:**
OpenAI-compatible format

---

### GET /droid/*/v1/models

Get available Droid models.

**Authentication:** API Key

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "claude-opus-4-1-20250805",
      "object": "model",
      "created": 1704067200,
      "owned_by": "anthropic"
    },
    {
      "id": "gpt-5-2025-08-07",
      "object": "model",
      "created": 1704067200,
      "owned_by": "openai"
    }
  ]
}
```

---

## Unified Routes

Base path: `/v1/*`

### POST /v1/chat/completions

Unified endpoint with intelligent backend routing.

**Authentication:** API Key

**Request Body:**
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "messages": [
    {
      "role": "user",
      "content": "Hello!"
    }
  ]
}
```

**Backend Detection:**
- `claude-*` → Claude backend
- `gpt-*`, `o1-*`, `o3-*`, `chatgpt-4o-latest` → OpenAI backend
- `gemini-*` → Gemini backend

**Response:**
Backend-specific format (auto-converted to OpenAI-compatible)

---

### POST /v1/completions

Legacy completions endpoint (auto-converts to chat format).

**Authentication:** API Key

**Request Body:**
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "prompt": "Hello!",
  "max_tokens": 100
}
```

**Response:**
OpenAI completions format

---

## User Management Routes

Base path: `/users/*`

**Note:** Requires `USER_MANAGEMENT_ENABLED=true`

### POST /users/login

User login with LDAP authentication.

**Authentication:** None (public endpoint)

**Request Body:**
```json
{
  "username": "john.doe",
  "password": "password123"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Login successful",
  "user": {
    "id": "user_123",
    "username": "john.doe",
    "email": "john@example.com",
    "displayName": "John Doe",
    "firstName": "John",
    "lastName": "Doe",
    "role": "user"
  },
  "sessionToken": "session_abc123"
}
```

**Rate Limits:**
- 30 attempts per IP per 15 minutes
- 100 attempts per IP per hour (strict limit)

**Errors:**
- `400 Bad Request`: Missing credentials or invalid input
- `401 Unauthorized`: Invalid credentials
- `429 Too Many Requests`: Rate limit exceeded
- `503 Service Unavailable`: User management or LDAP disabled

---

### POST /users/logout

User logout (invalidates session).

**Authentication:** User Session Token

**Response:**
```json
{
  "success": true,
  "message": "Logout successful"
}
```

---

### GET /users/profile

Get current user profile.

**Authentication:** User Session Token

**Response:**
```json
{
  "success": true,
  "user": {
    "id": "user_123",
    "username": "john.doe",
    "email": "john@example.com",
    "displayName": "John Doe",
    "firstName": "John",
    "lastName": "Doe",
    "role": "user",
    "createdAt": "2025-01-01T00:00:00Z"
  }
}
```

---

### GET /users/api-keys

Get user's API Keys.

**Authentication:** User Session Token

**Response:**
```json
{
  "success": true,
  "apiKeys": [
    {
      "id": "key_123",
      "name": "My Key",
      "description": "Personal API key",
      "isActive": true,
      "createdAt": "2025-01-01T00:00:00Z",
      "expiresAt": null,
      "usage": {
        "totalTokens": 50000,
        "totalRequests": 100
      }
    }
  ]
}
```

---

### POST /users/api-keys

Create a new API Key (user).

**Authentication:** User Session Token

**Request Body:**
```json
{
  "name": "My New Key",
  "description": "API key for project X",
  "tokenLimit": 1000000,
  "expiresAt": "2025-12-31T23:59:59Z"
}
```

**Response:**
```json
{
  "success": true,
  "message": "API key created",
  "apiKey": {
    "id": "key_456",
    "key": "cr_abc123def456...",
    "name": "My New Key",
    "description": "API key for project X",
    "tokenLimit": 1000000,
    "expiresAt": "2025-12-31T23:59:59Z"
  }
}
```

**Errors:**
- `400 Bad Request`: Invalid input or limit exceeded
- `403 Forbidden`: User not allowed to create API keys

---

## API Stats Routes

Base path: `/api-stats/*`

### POST /api-stats/api/user-stats

Get API Key usage statistics (self-query).

**Authentication:** None (validates via API Key in request body)

**Request Body:**
```json
{
  "apiKey": "cr_your_api_key",
  "apiId": "key_123"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "key_123",
    "name": "My API Key",
    "description": "My key description",
    "isActive": true,
    "createdAt": "2025-01-01T00:00:00Z",
    "expiresAt": null,
    "expirationMode": "fixed",
    "isActivated": false,
    "permissions": "all",
    "usage": {
      "total": {
        "requests": 100,
        "tokens": 50000,
        "allTokens": 50000,
        "inputTokens": 30000,
        "outputTokens": 20000,
        "cacheCreateTokens": 0,
        "cacheReadTokens": 0,
        "cost": 0.05,
        "formattedCost": "$0.050000"
      }
    },
    "limits": {
      "tokenLimit": 1000000,
      "concurrencyLimit": 5,
      "rateLimitWindow": 60,
      "rateLimitRequests": 100,
      "rateLimitCost": 1.0,
      "dailyCostLimit": 10.0,
      "totalCostLimit": 100.0,
      "weeklyOpusCostLimit": 50.0,
      "currentWindowRequests": 5,
      "currentWindowTokens": 2500,
      "currentWindowCost": 0.0025,
      "currentDailyCost": 0.05,
      "currentTotalCost": 0.05,
      "weeklyOpusCost": 0.0,
      "windowStartTime": 1704067200000,
      "windowEndTime": 1704070800000,
      "windowRemainingSeconds": 3600
    },
    "accounts": {
      "claudeAccountId": "claude_acc_123",
      "geminiAccountId": null,
      "openaiAccountId": null,
      "details": null
    },
    "restrictions": {
      "enableModelRestriction": false,
      "restrictedModels": [],
      "enableClientRestriction": false,
      "allowedClients": []
    }
  }
}
```

**Notes:**
- Can query by `apiKey` (actual key) or `apiId` (UUID)
- Secure endpoint - only returns data for the provided key
- Does not trigger activation

---

### POST /api-stats/api/get-key-id

Get API Key ID from API Key string.

**Authentication:** None (validates via API Key in request body)

**Request Body:**
```json
{
  "apiKey": "cr_your_api_key"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "key_123"
  }
}
```

**Errors:**
- `400 Bad Request`: Missing or invalid API key format
- `401 Unauthorized`: Invalid API key

---

### POST /api-stats/api/user-model-stats

Get per-model usage statistics.

**Authentication:** None (validates via API Key in request body)

**Request Body:**
```json
{
  "apiKey": "cr_your_api_key",
  "apiId": "key_123",
  "period": "daily"
}
```

**Query Parameters:**
- `period` (string): `daily` or `monthly`

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "model": "claude-3-5-sonnet-20241022",
      "requests": 50,
      "inputTokens": 15000,
      "outputTokens": 10000,
      "cacheCreateTokens": 0,
      "cacheReadTokens": 0,
      "allTokens": 25000,
      "costs": {
        "input": 0.045,
        "output": 0.15,
        "cacheCreate": 0.0,
        "cacheRead": 0.0,
        "total": 0.195
      },
      "formatted": {
        "input": "$0.045000",
        "output": "$0.150000",
        "cacheCreate": "$0.000000",
        "cacheRead": "$0.000000",
        "total": "$0.195000"
      },
      "pricing": {
        "input": 3.0,
        "output": 15.0,
        "cacheCreate": 3.75,
        "cacheRead": 0.3
      }
    }
  ],
  "period": "daily"
}
```

---

### POST /api-stats/api/batch-stats

Get aggregated statistics for multiple API Keys.

**Authentication:** Admin (via API Keys owned by same user)

**Request Body:**
```json
{
  "apiIds": ["key_123", "key_456", "key_789"]
}
```

**Limits:**
- Maximum 30 API keys per request

**Response:**
```json
{
  "success": true,
  "data": {
    "aggregated": {
      "totalKeys": 3,
      "activeKeys": 3,
      "usage": {
        "requests": 300,
        "inputTokens": 90000,
        "outputTokens": 60000,
        "cacheCreateTokens": 0,
        "cacheReadTokens": 0,
        "allTokens": 150000,
        "cost": 0.15,
        "formattedCost": "$0.150000"
      },
      "dailyUsage": { ... },
      "monthlyUsage": { ... }
    },
    "individual": [
      {
        "apiId": "key_123",
        "name": "Key 1",
        "isActive": true,
        "usage": { ... },
        "dailyUsage": { ... },
        "monthlyUsage": { ... }
      }
    ]
  }
}
```

---

### POST /api-stats/api/batch-model-stats

Get aggregated per-model statistics for multiple API Keys.

**Authentication:** Admin

**Request Body:**
```json
{
  "apiIds": ["key_123", "key_456"],
  "period": "daily"
}
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "model": "claude-3-5-sonnet-20241022",
      "requests": 100,
      "inputTokens": 30000,
      "outputTokens": 20000,
      "allTokens": 50000,
      "costs": { ... },
      "formatted": { ... },
      "pricing": { ... }
    }
  ],
  "period": "daily"
}
```

---

## Admin Management Routes

Base path: `/admin/*`

**Note:** All endpoints require admin authentication

### POST /admin/auth/login

Admin login.

**Authentication:** None (public endpoint)

**Request Body:**
```json
{
  "username": "admin",
  "password": "admin_password"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Login successful",
  "adminToken": "admin_token_abc123",
  "admin": {
    "username": "admin",
    "createdAt": "2025-01-01T00:00:00Z"
  }
}
```

---

### POST /admin/api-keys

Create a new API Key (admin).

**Authentication:** Admin Token

**Request Body:**
```json
{
  "name": "Customer Key",
  "description": "API key for customer X",
  "tokenLimit": 10000000,
  "concurrencyLimit": 10,
  "rateLimitWindow": 60,
  "rateLimitRequests": 1000,
  "rateLimitCost": 10.0,
  "dailyCostLimit": 100.0,
  "totalCostLimit": 1000.0,
  "weeklyOpusCostLimit": 500.0,
  "expiresAt": "2025-12-31T23:59:59Z",
  "expirationMode": "fixed",
  "activationDays": 0,
  "permissions": "all",
  "enableModelRestriction": false,
  "restrictedModels": [],
  "enableClientRestriction": false,
  "allowedClients": [],
  "claudeAccountId": null,
  "geminiAccountId": null,
  "openaiAccountId": null
}
```

**Response:**
```json
{
  "success": true,
  "apiKey": {
    "id": "key_789",
    "key": "cr_new_api_key_here",
    "name": "Customer Key",
    "description": "API key for customer X",
    "tokenLimit": 10000000,
    "concurrencyLimit": 10,
    "rateLimitWindow": 60,
    "rateLimitRequests": 1000,
    "expiresAt": "2025-12-31T23:59:59Z",
    "expirationMode": "fixed",
    "permissions": "all",
    "createdAt": "2025-10-30T00:00:00Z"
  }
}
```

---

### GET /admin/api-keys

List all API Keys.

**Authentication:** Admin Token

**Response:**
```json
{
  "success": true,
  "apiKeys": [
    {
      "id": "key_123",
      "name": "Key 1",
      "description": "Description",
      "isActive": true,
      "tokenLimit": 1000000,
      "usage": { ... },
      "createdAt": "2025-01-01T00:00:00Z"
    }
  ]
}
```

---

### PUT /admin/api-keys/:id

Update an API Key.

**Authentication:** Admin Token

**Request Body:**
```json
{
  "name": "Updated Key Name",
  "tokenLimit": 2000000,
  "isActive": true
}
```

**Response:**
```json
{
  "success": true,
  "message": "API key updated",
  "apiKey": { ... }
}
```

---

### DELETE /admin/api-keys/:id

Delete an API Key.

**Authentication:** Admin Token

**Response:**
```json
{
  "success": true,
  "message": "API key deleted"
}
```

---

### GET /admin/dashboard

Get system dashboard statistics.

**Authentication:** Admin Token

**Response:**
```json
{
  "success": true,
  "stats": {
    "totalKeys": 50,
    "activeKeys": 45,
    "totalUsage": {
      "requests": 10000,
      "tokens": 5000000,
      "cost": 50.0
    },
    "todayUsage": { ... },
    "claudeAccounts": {
      "total": 5,
      "active": 4
    },
    "geminiAccounts": { ... },
    "openaiAccounts": { ... },
    "systemInfo": {
      "version": "1.1.187",
      "uptime": 86400,
      "memory": { ... }
    }
  }
}
```

---

## Webhook Routes

Base path: `/admin/webhook/*`

**Note:** All endpoints require admin authentication

### GET /admin/webhook/config

Get webhook configuration.

**Authentication:** Admin Token

**Response:**
```json
{
  "success": true,
  "config": {
    "enabled": true,
    "urls": ["https://webhook.example.com/endpoint"],
    "events": ["api_key_created", "usage_alert"],
    "retryAttempts": 3,
    "timeout": 5000
  }
}
```

---

### POST /admin/webhook/config

Save webhook configuration.

**Authentication:** Admin Token

**Request Body:**
```json
{
  "enabled": true,
  "urls": ["https://webhook.example.com/endpoint"],
  "events": ["api_key_created", "usage_alert"],
  "retryAttempts": 3,
  "timeout": 5000
}
```

**Response:**
```json
{
  "success": true,
  "message": "Webhook配置已保存",
  "config": { ... }
}
```

---

## Health Check & Metrics

### GET /health

System health check.

**Authentication:** None (public endpoint)

**Response:**
```json
{
  "status": "healthy",
  "service": "claude-relay-service",
  "version": "1.1.187",
  "timestamp": "2025-10-30T00:00:00Z",
  "components": {
    "redis": "healthy",
    "logger": "healthy",
    "accounts": {
      "claude": 5,
      "gemini": 3,
      "openai": 2
    }
  },
  "memory": {
    "heapUsed": 50000000,
    "heapTotal": 100000000,
    "external": 1000000
  }
}
```

---

### GET /metrics

System metrics and usage statistics.

**Authentication:** None (public endpoint)

**Response:**
```json
{
  "service": "claude-relay-service",
  "version": "1.1.187",
  "uptime": 86400,
  "timestamp": "2025-10-30T00:00:00Z",
  "usage": {
    "total": {
      "requests": 10000,
      "tokens": 5000000,
      "cost": 50.0
    },
    "realtime": {
      "requestsPerMinute": 10,
      "tokensPerMinute": 5000,
      "window": 5
    }
  },
  "memory": {
    "heapUsed": 50000000,
    "heapTotal": 100000000,
    "rss": 120000000
  },
  "cache": {
    "hitRate": 0.85,
    "size": 1000,
    "maxSize": 10000
  }
}
```

---

## Error Codes Reference

| Code | Status | Description |
|------|--------|-------------|
| `400` | Bad Request | Invalid request format or missing required fields |
| `401` | Unauthorized | Invalid or missing API key/session token |
| `403` | Forbidden | API key lacks required permissions or model is blacklisted |
| `404` | Not Found | Resource not found |
| `429` | Too Many Requests | Rate limit exceeded |
| `500` | Internal Server Error | Server-side error |
| `502` | Bad Gateway | Upstream service error |
| `503` | Service Unavailable | Service temporarily unavailable or no available accounts |
| `504` | Gateway Timeout | Upstream service timeout |

---

## Common Error Response Format

```json
{
  "error": {
    "type": "error_type",
    "message": "Human-readable error message",
    "code": "error_code"
  },
  "timestamp": "2025-10-30T00:00:00Z"
}
```

---

## Rate Limiting

Rate limits are configured per API Key:

- **Request-based**: Limit requests per time window
- **Token-based**: Limit total tokens consumed
- **Cost-based**: Limit total cost per window/day/total
- **Concurrency**: Limit simultaneous requests

**Rate Limit Headers:**
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 950
X-RateLimit-Reset: 1704070800
```

---

## Sticky Sessions

The service supports sticky sessions to ensure conversation continuity:

- **Session Hash**: Generated from request content
- **TTL**: Configurable (default: 1 hour)
- **Renewal**: Automatic on subsequent requests
- **Header**: `x-session-hash` (optional, auto-generated if not provided)

---

## Streaming Response Format

All streaming endpoints use Server-Sent Events (SSE):

```
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive
X-Accel-Buffering: no
```

**Event Format:**
```
event: event_type
data: {"key": "value"}

```

**Common Events:**
- `message_start` - Stream started
- `content_block_delta` - Content chunk
- `message_delta` - Message metadata
- `message_stop` - Stream completed
- `error` - Error occurred

---

## Proxy Configuration

All account types support independent proxy configuration:

**Proxy Format:**
```json
{
  "host": "proxy.example.com",
  "port": 1080,
  "protocol": "socks5",
  "auth": {
    "username": "user",
    "password": "pass"
  }
}
```

**Supported Protocols:**
- `http`
- `https`
- `socks5`

---

## Account Types

The service supports multiple account types:

| Type | Provider | Endpoints |
|------|----------|-----------|
| `claude-official` | Anthropic | `/api/*`, `/claude/*` |
| `claude-console` | Anthropic | `/api/*`, `/claude/*` |
| `bedrock` | AWS | `/api/*`, `/claude/*` |
| `ccr` | Custom | `/api/*`, `/claude/*` |
| `gemini` | Google | `/gemini/*` |
| `openai-responses` | OpenAI | `/openai/*` |
| `azure-openai` | Microsoft | `/azure/*` |
| `droid` | Factory.ai | `/droid/*` |

---

## Pagination

Endpoints that return lists support pagination:

**Query Parameters:**
- `page` (number): Page number (default: 1)
- `limit` (number): Items per page (default: 20, max: 100)

**Response:**
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 100,
    "totalPages": 5
  }
}
```

---

## Changelog

See [CHANGELOG.md](../CHANGELOG.md) for version history and updates.

---

## Support

For issues and questions:
- GitHub Issues: https://github.com/your-repo/issues
- Documentation: https://github.com/your-repo/wiki

---

**Last Updated:** 2025-10-30
**Documentation Version:** 1.0.0
