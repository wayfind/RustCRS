# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

这个文件为 Claude Code (claude.ai/code) 提供在此代码库中工作的指导。

## 项目概述

Claude Relay Service 是一个高性能 AI API 中转服务，支持多个平台（Claude、Gemini、OpenAI、Bedrock、Azure）。

**当前状态**: 🚧 正在从 Node.js 迁移到 Rust（进行中）

项目组成：
- **Rust 后端** (`rust/`): 🎯 **主要实现** - 高性能中转服务
- **Vue 3 前端** (`web/admin-spa/`): 现代化 SPA 管理界面
- **Node.js 代码** (`nodejs-archive/`): ⚠️ **仅供参考** - 原始实现，正在迁移中

**架构**: Rust + Vue 3 + Redis
- Rust 后端处理所有业务逻辑（API 中转、账户管理、OAuth、认证）
- Vue 3 前端提供管理界面
- Redis 作为数据存储和缓存层

## 快速启动命令

### 开发环境启动

```bash
# 推荐：一键启动（Redis + Rust 后端 + Vue 前端）
make rust-dev
# 或
bash start-dev.sh

# 可选：先验证环境配置
bash verify-setup.sh
```

### 环境配置（首次设置）

```bash
# 1. 复制环境变量模板
cp .env.example .env

# 2. 生成必需的密钥
openssl rand -base64 48  # 用于 CRS_SECURITY__JWT_SECRET (48+ 字符)
openssl rand -hex 16     # 用于 CRS_SECURITY__ENCRYPTION_KEY (恰好 32 字符)

# 3. 编辑 .env 并设置：
#    - CRS_SECURITY__JWT_SECRET
#    - CRS_SECURITY__ENCRYPTION_KEY
#    - CRS_REDIS__HOST=localhost
#    - CRS_REDIS__PORT=6379
```

### 常用开发命令

```bash
# Rust 后端
cd rust/
cargo run                    # 开发模式（快速编译）
cargo build --release        # 生产构建（优化）
cargo test --lib             # 仅单元测试
cargo test --test '*'        # 集成测试
cargo clippy -- -D warnings  # 代码检查
cargo fmt                    # 代码格式化

# Vue 前端
cd web/admin-spa/
npm run dev                  # 开发服务器 (http://localhost:3001)
npm run build                # 生产构建

# Docker
docker-compose up -d         # 启动所有服务
docker-compose down          # 停止所有服务

# 服务管理
make rust-backend            # 仅 Rust 后端
make rust-frontend           # 仅 Vue 前端
make stop-all                # 停止所有运行中的服务
```

### 测试

```bash
# Rust 单元测试（快速，无需 Redis）
cargo test --lib

# Rust 集成测试（需要 Redis）
bash rust/run-integration-tests.sh

# 运行特定测试
cargo test test_name

# 带日志输出
RUST_LOG=debug cargo test test_name -- --nocapture

# 性能基准测试
cargo bench
```

## 架构说明

### Rust + Vue 架构

**当前架构**: 纯 Rust 后端 + Vue 3 前端 + Redis 存储

```
┌─────────────────────────────────────────────────────────┐
│  客户端 (Claude Code, Gemini CLI, OpenAI 客户端等)      │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│           Rust 后端服务 (端口 8080)                      │
│  - API 中转 & 转发                                       │
│  - OAuth 流程 (Claude/Gemini)                           │
│  - Token 刷新 & 管理                                     │
│  - 账户管理 & 调度                                       │
│  - 流式传输 (SSE) 处理                                   │
│  - 请求路由: /api, /gemini, /openai                     │
│  - API Key 认证                                          │
│  - 使用量追踪 & 成本计算                                 │
└────────────────┬────────────────────────────────────────┘
                 │
                 ├──────────────────────────────────┐
                 ▼                                  ▼
┌────────────────────────────────┐   ┌───────────────────────────┐
│  Vue 3 前端 (端口 3001)        │   │   Redis 数据存储          │
│  - 管理界面                    │   │   - 账户 & token          │
│  - 账户配置                    │   │   - API keys & 使用量     │
│  - 统计监控                    │   │   - 速率限制 & 会话       │
└────────────────────────────────┘   └───────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│  AI 提供商 API (Anthropic, Google, OpenAI 等)           │
└─────────────────────────────────────────────────────────┘
```

**架构优势**:
- **Rust**: 高性能、低延迟 (<20ms)、高吞吐量 (>2000 req/s)、低内存 (<70MB)
- **Vue 3**: 现代化响应式前端，支持暗黑模式和响应式设计
- **Redis**: 高速数据存储，支持原子操作和 TTL

### 请求流程

1. **客户端请求** → Rust 服务 (`:8080/api/v1/messages`)
2. **Rust 认证** → 从 Redis 验证 API Key
3. **Rust 调度器** → 选择最优账户，必要时刷新 token
4. **Rust 中转** → 使用账户凭据转发到提供商
5. **流式响应** → Rust 处理 SSE 流式传输回客户端
6. **使用量捕获** → Rust 更新 Redis 使用量/成本统计

### 核心目录结构

```
rust/                           🎯 主要实现（Rust 后端）
  src/
    routes/       - API 端点 (admin.rs, api.rs, gemini.rs, openai.rs, health.rs)
    services/     - 业务逻辑 (account, api_key, relay, scheduler 服务)
    middleware/   - 认证、日志、错误处理
    models/       - 数据结构
    redis/        - Redis 客户端和操作
    utils/        - 加密、验证、辅助函数
    config/       - 配置管理
  tests/          - 集成测试
  benches/        - 性能基准测试

web/admin-spa/                  🎨 前端界面（Vue 3）
  src/
    components/   - Vue 组件
    views/        - 页面视图（仪表板、账户、API Keys 等）
    stores/       - Pinia 状态管理（主题、认证）
    router/       - Vue Router 配置

docs/                           📚 项目文档
  guides/         - 用户指南（快速开始、API 参考、部署）
  architecture/   - 技术文档（概览、测试、Redis schema）
  development/    - 开发者资源（CLI 使用、故障排除）

nodejs-archive/                 ⚠️ 仅供参考（原 Node.js 实现）
  src/
    services/     - 原服务实现（参考迁移逻辑）
    routes/       - 原路由实现（参考 API 设计）
    middleware/   - 原中间件（参考认证逻辑）
    utils/        - 原工具函数（参考算法实现）
```

### 配置系统

**环境变量模式**: 所有配置使用 `CRS_*` 前缀，采用分层结构：

```bash
# 必需配置
CRS_SECURITY__JWT_SECRET="..."          # JWT 签名（48+ 字符）
CRS_SECURITY__ENCRYPTION_KEY="..."     # AES 加密（恰好 32 字符）
CRS_REDIS__HOST="localhost"
CRS_REDIS__PORT=6379

# 可选但常用
CRS_LOGGING__LEVEL="debug"
CRS_LOGGING__FORMAT="pretty"
RUST_LOG="debug,hyper=info,tokio=info"
```

**关键文件**:
- `.env` - 运行时环境变量（已忽略 git）
- `.env.example` - 包含所有可用选项的模板
- `rust/src/config/settings.rs` - Rust 配置结构和加载
- `Cargo.toml` - Rust 依赖和项目元数据
- `nodejs-archive/config/config.js` - Node.js 配置（遗留）

### Redis 数据架构

**模式**: 所有键遵循命名空间模式以实现多租户隔离：

```
api_key:{id}                    - API Key 详情（权限、限制、元数据）
api_key_hash:{hash}             - SHA-256 哈希 → ID 映射（O(1) 查找）
api_key_usage:{keyId}           - 每个 key 的使用统计
api_key_cost:{keyId}            - 每个 key 的成本追踪

claude_account:{id}             - Claude OAuth 账户（加密 token）
gemini_account:{id}             - Gemini OAuth 账户
openai_responses_account:{id}   - OpenAI 账户
bedrock_account:{id}            - AWS Bedrock 凭据
azure_openai_account:{id}       - Azure OpenAI 配置

sticky_session:{sessionHash}    - 会话 → 账户绑定（对话连续性）
session_window:{accountId}      - 账户使用窗口追踪

rate_limit:{keyId}:{window}     - 速率限制计数器
concurrency:{accountId}         - 活动请求数（Redis Sorted Set）
overload:{accountId}            - 529 错误退避状态

usage:daily:{date}:{key}:{model}  - 详细使用指标
usage:account:{accountId}:{date}  - 按账户的使用量
usage:global:{date}                - 全局统计

admin:{id}                      - 管理员用户数据
admin_username:{username}       - 用户名 → ID 映射
session:{token}                 - JWT 会话管理
```

**详见**: `docs/architecture/redis-schema.md` 获取完整 schema 参考。

### 多平台账户支持

该服务支持 8 种账户类型，每种都有不同的认证和中转逻辑：

| 账户类型 | 认证方式 | Rust 实现位置 | 说明 |
|---------|---------|--------------|------|
| `claude-official` | OAuth (PKCE) | `services/relay_service.rs` | Claude 官方 API |
| `claude-console` | 会话 token | `services/account_service.rs` | Claude Console |
| `gemini` | Google OAuth | `services/gemini_service.rs` | Google Gemini |
| `openai-responses` | API Key | `services/openai_service.rs` | OpenAI Codex |
| `bedrock` | AWS 凭据 | `services/bedrock_service.rs` | AWS Bedrock |
| `azure-openai` | Azure key | `services/openai_service.rs` | Azure OpenAI |
| `droid` | API Key | 🚧 迁移中 | Factory.ai |
| `ccr` | 凭据 | 🚧 迁移中 | CCR |

**统一调度器** (Rust 实现 `services/scheduler.rs`):
- 智能账户选择：负载均衡、健康检查、故障转移
- 粘性会话：同一对话使用同一账户保持上下文
- 并发控制：Redis Sorted Set 实现并发限制
- Token 管理：自动检测过期并刷新

### 流式传输 & 使用量捕获

**SSE 流式传输架构**: Rust 处理服务器发送事件（Server-Sent Events）以实现实时响应：

```rust
// rust/src/services/relay_service.rs
// 流式传输响应块，同时从 SSE 事件解析使用量元数据
async fn stream_response(provider_stream: Response) -> Result<Response> {
    // 1. 创建 SSE 流
    // 2. 解析每个块以获取使用量数据（input_tokens, output_tokens 等）
    // 3. 实时转发给客户端
    // 4. 完成时，用实际使用量更新 Redis
}
```

**使用量数据流**:
1. 提供商发送带有使用量事件的 SSE 流
2. Rust 解析 `message_start`、`content_block_delta`、`message_delta` 事件
3. 提取 token 计数：`input_tokens`、`output_tokens`、`cache_creation_input_tokens`、`cache_read_input_tokens`
4. 流完成后原子性更新 Redis
5. 基于模型定价（来自 `pricingService.js`）计算成本

**关键点**: 使用量必须从实际 API 响应中捕获，而不是从请求中估算。

## 开发模式

### 添加新的 API 路由（Rust）

1. **定义路由处理器** 在 `rust/src/routes/`:
```rust
// rust/src/routes/my_route.rs
pub async fn my_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<RequestType>,
) -> Result<Json<ResponseType>, AppError> {
    // 实现
}
```

2. **添加到路由器** 在 `rust/src/routes/mod.rs`:
```rust
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/my-endpoint", post(my_route::my_handler))
        .with_state(state)
}
```

3. **如需要，添加中间件**（认证、日志）:
```rust
Router::new()
    .route("/my-endpoint", post(my_route::my_handler))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        crate::middleware::authenticate_api_key,
    ))
```

### 使用 Redis（Rust）

```rust
use deadpool_redis::Pool;

// 从连接池获取连接
async fn get_data(pool: &Pool, key: &str) -> Result<String, Error> {
    let mut conn = pool.get().await?;
    let value: String = conn.get(key).await?;
    Ok(value)
}

// 使用管道进行原子操作
async fn update_stats(pool: &Pool, key: &str, increment: i64) -> Result<()> {
    let mut conn = pool.get().await?;
    redis::pipe()
        .atomic()
        .incr(key, increment)
        .expire(key, 86400)
        .query_async(&mut conn)
        .await?;
    Ok(())
}
```

**详见**: `rust/src/redis/client.rs` 了解连接池设置和辅助函数。

### 前端开发（Vue 3）

**暗黑模式**: 所有新组件必须同时支持亮色和暗色主题：

```vue
<template>
  <!-- 使用 Tailwind dark: 前缀设置暗黑模式样式 -->
  <div class="bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100">
    <button class="bg-blue-500 hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700">
      操作
    </button>
  </div>
</template>
```

**主题存储** (`web/admin-spa/src/stores/theme.js`):
```javascript
import { useThemeStore } from '@/stores/theme';

// 在组件中
const themeStore = useThemeStore();
themeStore.toggleTheme(); // 在亮色/暗色之间切换
```

**响应式设计**: 使用 Tailwind 断点（`sm:`、`md:`、`lg:`、`xl:`）:
```vue
<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
  <!-- 自动响应式布局 -->
</div>
```

**代码格式化**: 提交前始终运行 Prettier：
```bash
npx prettier --write "src/**/*.{vue,js,ts}"
```

## 关键约束

### 安全性

- **永不提交密钥**: 所有敏感数据存放在 `.env`（已忽略 git）
- **静态加密**: Token 和凭据在 Redis 中使用 AES-256-CBC 加密
- **哈希 API keys**: 存储 SHA-256 哈希值，而非明文
- **环境隔离**: 使用 `CRS_*` 前缀避免配置冲突

### 性能

- **Rust 流式传输**: 高性能 SSE 流式处理,低延迟(<20ms)
- **Redis 管道化**: 对多键操作使用原子管道
- **连接池**: 重用 HTTP 客户端和 Redis 连接
- **LRU 缓存**: 在内存中缓存解密结果，避免重复加密操作

### 代码质量

- **Rust**: 提交前必须通过 `cargo clippy` 和 `cargo fmt --check`
- **前端**: 必须运行 Prettier 格式化并支持暗黑模式
- **测试**: 集成测试需要 Docker Redis（使用 `testcontainers`）
- **文档**: 更改架构或 API 时更新 docs/

## 常见工作流程

### OAuth 账户添加（通过 Web UI）

1. **前端** (`web/admin-spa/`) 收集账户信息和代理设置
2. **POST** 到 Rust `/admin/claude-accounts/generate-auth-url` → 获取 OAuth URL
3. **用户** 打开 URL，授权，复制授权码
4. **前端** 将授权码提交到 `/admin/claude-accounts/exchange-code`
5. **Rust** 交换授权码获取 token（如果配置了代理则通过代理）
6. **存储** 加密的 token 到 Redis 作为 `claude_account:{id}`
7. **后台服务** 在 token 即将过期时自动刷新（<10 秒阈值）

### 请求生命周期

1. **客户端** 发送请求到 Rust 端点（如 `/api/v1/messages`）
2. **Rust 认证中间件** 从 Redis 验证 API Key（`api_key_hash:{sha256}`）
3. **检查权限**: API Key 的 `permissions` 字段（all/claude/gemini/openai）
4. **检查限制**: User-Agent 匹配和模型黑名单
5. **Rust 调度器** 选择账户（或使用粘性会话）
6. **调度器** 返回账户 ID 并检查 token 新鲜度
7. **如果过期**: Rust 通过 OAuth 刷新 token（使用代理）
8. **Rust 中转服务** 使用账户凭据转发请求到提供商
9. **流式响应** 同时从 SSE 事件解析使用量
10. **更新 Redis**: 使用统计、成本计算、速率限制、并发计数器

### 粘性会话（对话连续性）

**问题**: 同一对话应使用同一账户以保持上下文连续性。

**解决方案**: 基于哈希的会话绑定：
```javascript
// ⚠️ 遗留参考 (nodejs-archive/src/utils/sessionHelper.js)
// Rust 实现位于: rust/src/utils/session.rs
const sessionHash = crypto.createHash('sha256')
  .update(JSON.stringify({
    messages: request.messages.slice(-5), // 最后 5 条消息
    model: request.model,
    apiKeyId: apiKey.id
  }))
  .digest('hex');

// 检查 Redis 中的粘性会话
const boundAccountId = await redis.get(`sticky_session:${sessionHash}`);

// 如果找到且健康，使用绑定的账户
// 否则，选择新账户并绑定：
await redis.setex(`sticky_session:${sessionHash}`, TTL, selectedAccountId);
```

**TTL 续期**: 如果会话在续期阈值内，延长 TTL 以保持绑定活跃。

## 故障排除

### Rust 后端无法启动

**错误**: `CRS_SECURITY__ENCRYPTION_KEY must be set`
- **修复**: 确保 `.env` 存在且 `CRS_SECURITY__ENCRYPTION_KEY` 恰好为 32 字符
- **生成**: `openssl rand -hex 16`

**错误**: `Connection refused (os error 111)` (Redis)
- **修复**: 启动 Redis: `docker run -d --name redis-dev -p 6379:6379 redis:7-alpine`
- **验证**: `redis-cli ping` 应返回 `PONG`

### 前端无法连接后端

- **检查 Rust**: `curl http://localhost:8080/health` 应返回 `{"status":"ok"}`
- **检查代理**: `web/admin-spa/vite.config.js` 应将 `/api` 代理到 `localhost:8080`
- **CORS**: Rust 在 `rust/src/main.rs` 中配置 CORS（应允许 `localhost:3001`）

### OAuth Token 刷新失败

- **检查代理配置**: OAuth 流程必须使用账户的代理设置
- **检查日志**: `logs/token-refresh-error.log (⚠️ Rust实现中)`
- **手动测试**: `⚠️ 遗留脚本参考: nodejs-archive/scripts/test-gemini-refresh.js`（Gemini 示例）
- **验证刷新 token**: Redis `claude_account:{id}` 应有有效的 `refresh_token`

### 集成测试失败

- **确保 Redis 可用**: 测试使用 `testcontainers` 自动启动 Redis
- **检查 Docker**: `docker ps` 应显示 testcontainer Redis 实例
- **环境变量**: 测试需要 `ENCRYPTION_KEY` 环境变量
- **带日志运行**: `RUST_LOG=debug cargo test test_name -- --nocapture`

## 文档导航

**新主题从这里开始**:
- **快速开始**: `docs/guides/quickstart.md`
- **API 参考**: `docs/guides/api-reference.md`
- **测试**: `docs/architecture/testing.md`
- **CLI 工具**: `docs/development/cli-usage.md`
- **Redis schema**: `docs/architecture/redis-schema.md`
- **故障排除**: `docs/guides/troubleshooting.md`
- **路线图**: `docs/development/roadmap.md`

**完整文档索引**: `docs/README.md`

## 重要文件参考

**说明**: 此表格展示 Rust 实现与原 Node.js 代码的对应关系，供理解迁移逻辑使用。**当前系统仅运行 Rust 代码**，Node.js 代码仅作参考。

| 用途 | Rust 位置 | Node.js 位置 | 说明 |
|------|-----------|--------------|------|
| **主入口** | `rust/src/main.rs` | `nodejs-archive/src/app.js` | 应用启动 |
| **配置** | `rust/src/config/settings.rs` | `nodejs-archive/config/config.js` | 配置加载 |
| **API 路由** | `rust/src/routes/` | `nodejs-archive/src/routes/` | HTTP 端点 |
| **认证中间件** | `rust/src/middleware/auth.rs` | `nodejs-archive/src/middleware/auth.js` | API Key 验证 |
| **中转服务** | `rust/src/services/relay_service.rs` | `nodejs-archive/src/services/claudeRelayService.js` | 提供商转发 |
| **账户管理** | `rust/src/services/account_service.rs` | `nodejs-archive/src/services/claudeAccountService.js` | 账户 CRUD、token 刷新 |
| **调度器** | `rust/src/services/scheduler.rs` | `nodejs-archive/src/services/unifiedClaudeScheduler.js` | 账户选择 |
| **Redis 客户端** | `rust/src/redis/client.rs` | `nodejs-archive/src/models/redis.js` | 连接池 |
| **加密工具** | `rust/src/utils/crypto.rs` | `nodejs-archive/src/utils/crypto.js` | AES 加密、SHA 哈希 |
