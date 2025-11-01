
## 新增功能概览（相比旧版本）

### 多平台支持

- ✅ **Claude Console账户**: 支持Claude Console类型账户
- ✅ **AWS Bedrock**: 完整的AWS Bedrock API支持
- ✅ **Azure OpenAI**: Azure OpenAI服务支持
- ✅ **Droid (Factory.ai)**: Factory.ai API支持
- ✅ **CCR账户**: CCR凭据支持
- ✅ **OpenAI兼容**: OpenAI格式转换和Responses格式支持

### 用户和权限系统

- ✅ **用户管理**: 完整的用户注册、登录、API Key管理系统
- ✅ **LDAP认证**: 企业级LDAP/Active Directory集成
- ✅ **权限控制**: API Key级别的服务权限（all/claude/gemini/openai）
- ✅ **客户端限制**: 基于User-Agent的客户端识别和限制
- ✅ **模型黑名单**: API Key级别的模型访问控制

### 统一调度和会话管理

- ✅ **统一调度器**: 跨账户类型的智能调度系统
- ✅ **粘性会话**: 会话级账户绑定，支持自动续期
- ✅ **并发控制**: Redis Sorted Set实现的并发限制
- ✅ **负载均衡**: 自动账户选择和故障转移

### 成本和监控

- ✅ **定价服务**: 模型价格管理和自动成本计算
- ✅ **成本统计**: 详细的token使用和费用统计
- ✅ **缓存监控**: 全局缓存统计和命中率分析
- ✅ **实时指标**: 可配置窗口的实时统计（METRICS_WINDOW）

### Webhook和通知

- ✅ **Webhook系统**: 事件通知和Webhook配置管理
- ✅ **多URL支持**: 支持多个Webhook URL（逗号分隔）

### 高级功能

- ✅ **529错误处理**: 自动识别Claude过载状态并暂时排除账户
- ✅ **HTTP调试**: DEBUG_HTTP_TRAFFIC模式详细记录HTTP请求/响应
- ✅ **数据迁移**: 完整的数据导入导出工具（含加密/脱敏）
- ✅ **自动清理**: 并发计数、速率限制、临时错误状态自动清理

## 常用命令

### 基本开发命令

````bash
# 安装依赖和初始化
npm install
npm run setup                  # 生成配置和管理员凭据
npm run install:web           # 安装Web界面依赖

# 开发和运行
npm run dev                   # 开发模式（热重载）
npm start                     # 生产模式
npm test                      # 运行测试
npm run lint                  # 代码检查

# Docker部署
docker-compose up -d          # 推荐方式
docker-compose --profile monitoring up -d  # 包含监控

# 服务管理
npm run service:start:daemon  # 后台启动（推荐）
npm run service:status        # 查看服务状态
npm run service:logs          # 查看日志
npm run service:stop          # 停止服务

### 开发环境配置

#### 必须配置的环境变量
- `JWT_SECRET`: JWT密钥（32字符以上随机字符串）
- `ENCRYPTION_KEY`: 数据加密密钥（32字符固定长度）
- `REDIS_HOST`: Redis主机地址（默认localhost）
- `REDIS_PORT`: Redis端口（默认6379）
- `REDIS_PASSWORD`: Redis密码（可选）

#### 新增重要环境变量（可选）
- `USER_MANAGEMENT_ENABLED`: 启用用户管理系统（默认false）
- `LDAP_ENABLED`: 启用LDAP认证（默认false）
- `LDAP_URL`: LDAP服务器地址（如 ldaps://ldap.example.com:636）
- `LDAP_TLS_REJECT_UNAUTHORIZED`: LDAP证书验证（默认true）
- `WEBHOOK_ENABLED`: 启用Webhook通知（默认true）
- `WEBHOOK_URLS`: Webhook通知URL列表（逗号分隔）
- `CLAUDE_OVERLOAD_HANDLING_MINUTES`: Claude 529错误处理持续时间（分钟，0表示禁用）
- `STICKY_SESSION_TTL_HOURS`: 粘性会话TTL（小时，默认1）
- `STICKY_SESSION_RENEWAL_THRESHOLD_MINUTES`: 粘性会话续期阈值（分钟，默认0）
- `METRICS_WINDOW`: 实时指标统计窗口（分钟，1-60，默认5）
- `MAX_API_KEYS_PER_USER`: 每用户最大API Key数量（默认1）
- `ALLOW_USER_DELETE_API_KEYS`: 允许用户删除自己的API Keys（默认false）
- `DEBUG_HTTP_TRAFFIC`: 启用HTTP请求/响应调试日志（默认false，仅开发环境）
- `PROXY_USE_IPV4`: 代理使用IPv4（默认true）
- `REQUEST_TIMEOUT`: 请求超时时间（毫秒，默认600000即10分钟）

#### HTTPS 配置（可选 - 独立 HTTPS 服务器）
**注意**：生产环境推荐使用反向代理（Nginx/Caddy）而非独立 HTTPS
- `HTTPS_ENABLED`: 启用独立 HTTPS 服务器（默认false）
- `HTTPS_PORT`: HTTPS 端口（默认3443）
- `HTTPS_CERT_PATH`: SSL 证书路径（PEM 格式）
- `HTTPS_KEY_PATH`: SSL 私钥路径（PEM 格式）
- `HTTPS_REDIRECT_HTTP`: 启用 HTTP 到 HTTPS 自动重定向（默认true，仅当 HTTPS_ENABLED=true 时有效）

**使用示例**：
```bash
# 开发环境 - 使用自签名证书
HTTPS_ENABLED=true
HTTPS_PORT=3443
HTTPS_CERT_PATH=./certs/cert.pem
HTTPS_KEY_PATH=./certs/key.pem
HTTPS_REDIRECT_HTTP=true

# 生成自签名证书（开发用）
bash scripts/generate-self-signed-cert.sh
# 或使用 Node.js 版本（跨平台）
node scripts/generate-self-signed-cert.js
```

**生产环境推荐配置**：
- 使用 Caddy（自动 HTTPS）或 Nginx + Let's Encrypt
- 服务保持 HTTP 模式（`HTTPS_ENABLED=false`）
- 反向代理处理 SSL 终止
- 详见 README.md 的 "🔒 HTTPS 配置" 章节

#### AWS Bedrock配置（可选）
- `CLAUDE_CODE_USE_BEDROCK`: 启用Bedrock（设置为1启用）
- `AWS_REGION`: AWS默认区域（默认us-east-1）
- `ANTHROPIC_MODEL`: Bedrock默认模型
- `ANTHROPIC_SMALL_FAST_MODEL`: Bedrock小型快速模型
- `ANTHROPIC_SMALL_FAST_MODEL_AWS_REGION`: 小型模型区域
- `CLAUDE_CODE_MAX_OUTPUT_TOKENS`: 最大输出tokens（默认4096）
- `MAX_THINKING_TOKENS`: 最大思考tokens（默认1024）
- `DISABLE_PROMPT_CACHING`: 禁用提示缓存（设置为1禁用）

#### 初始化命令
```bash
cp config/config.example.js config/config.js
cp .env.example .env
npm run setup  # 自动生成密钥并创建管理员账户
```

## Web界面功能

### OAuth账户添加流程

1. **基本信息和代理设置**: 配置账户名称、描述和代理参数
2. **OAuth授权**:
   - 生成授权URL → 用户打开链接并登录Claude Code账号
   - 授权后会显示Authorization Code → 复制并粘贴到输入框
   - 系统自动交换token并创建账户

### 核心管理功能

- **实时仪表板**: 系统统计、账户状态、使用量监控、实时指标（METRICS_WINDOW配置窗口）
- **API Key管理**: 创建、配额设置、使用统计查看、权限配置、客户端限制、模型黑名单
- **多平台账户管理**:
  - Claude账户（官方/Console）: OAuth账户添加、代理配置、状态监控
  - Gemini账户: Google OAuth授权、代理配置
  - OpenAI Responses (Codex)账户: API Key配置
  - AWS Bedrock账户: AWS凭据配置
  - Azure OpenAI账户: Azure凭据和端点配置
  - Droid账户: Factory.ai API Key配置
  - CCR账户: CCR凭据配置
- **用户管理**: 用户注册、登录、API Key分配（USER_MANAGEMENT_ENABLED启用时）
- **系统日志**: 实时日志查看，多级别过滤，HTTP调试日志（DEBUG_HTTP_TRAFFIC启用时）
- **Webhook配置**: Webhook URL管理、事件配置
- **主题系统**: 支持明亮/暗黑模式切换，自动保存用户偏好设置
- **成本分析**: 详细的token使用和成本统计（基于pricingService）
- **缓存监控**: 解密缓存统计和性能监控

## 重要端点

### API转发端点（多路由支持）

#### Claude服务路由
- `POST /api/v1/messages` - Claude消息处理（支持流式）
- `POST /claude/v1/messages` - Claude消息处理（别名路由）
- `POST /v1/messages/count_tokens` - Token计数Beta API
- `GET /api/v1/models` - 模型列表
- `GET /api/v1/usage` - 使用统计查询
- `GET /api/v1/key-info` - API Key信息
- `GET /v1/me` - 用户信息（Claude Code客户端需要）
- `GET /v1/organizations/:org_id/usage` - 组织使用统计

#### Gemini服务路由
- `POST /gemini/v1/models/:model:generateContent` - 标准Gemini API格式
- `POST /gemini/v1/models/:model:streamGenerateContent` - Gemini流式
- `GET /gemini/v1/models` - Gemini模型列表
- 其他Gemini兼容路由（保持向后兼容）

#### OpenAI兼容路由
- `POST /openai/v1/chat/completions` - OpenAI格式转发（支持responses格式）
- `POST /openai/claude/v1/chat/completions` - OpenAI格式转Claude
- `POST /openai/gemini/v1/chat/completions` - OpenAI格式转Gemini
- `GET /openai/v1/models` - OpenAI格式模型列表

#### Droid (Factory.ai) 路由
- `POST /droid/claude/v1/messages` - Droid Claude转发
- `POST /droid/openai/v1/chat/completions` - Droid OpenAI转发

#### Azure OpenAI 路由
- `POST /azure/...` - Azure OpenAI API转发

### 管理端点

#### OAuth和账户管理
- `POST /admin/claude-accounts/generate-auth-url` - 生成OAuth授权URL（含代理）
- `POST /admin/claude-accounts/exchange-code` - 交换authorization code
- `POST /admin/claude-accounts` - 创建Claude OAuth账户
- 各平台账户CRUD端点（gemini、openai、bedrock、azure、droid、ccr）

#### 用户管理（USER_MANAGEMENT_ENABLED启用时）
- `POST /users/register` - 用户注册
- `POST /users/login` - 用户登录
- `GET /users/profile` - 用户资料
- `POST /users/api-keys` - 创建用户API Key

#### Webhook管理
- `GET /admin/webhook/configs` - 获取Webhook配置
- `POST /admin/webhook/configs` - 创建Webhook配置
- `PUT /admin/webhook/configs/:id` - 更新Webhook配置
- `DELETE /admin/webhook/configs/:id` - 删除Webhook配置

### 系统端点

- `GET /health` - 健康检查（包含组件状态、版本、内存等）
- `GET /metrics` - 系统指标（使用统计、uptime、内存）
- `GET /web` - 传统Web管理界面
- `GET /admin-next/` - 新版SPA管理界面（主界面）
- `GET /admin/dashboard` - 系统概览数据

## 故障排除

详见 [故障排除指南](docs/guides/troubleshooting.md)

## 开发最佳实践

### 代码格式化要求

- **必须使用 Prettier 格式化所有代码**
- 后端代码（src/）：运行 `npx prettier --write <file>` 格式化
- 前端代码（web/admin-spa/）：已安装 `prettier-plugin-tailwindcss`，运行 `npx prettier --write <file>` 格式化
- 提交前检查格式：`npx prettier --check <file>`
- 格式化所有文件：`npm run format`（如果配置了此脚本）

### 前端开发特殊要求

- **响应式设计**: 必须兼容不同设备尺寸（手机、平板、桌面），使用 Tailwind CSS 响应式前缀（sm:、md:、lg:、xl:）
- **暗黑模式兼容**: 项目已集成完整的暗黑模式支持，所有新增/修改的UI组件都必须同时兼容明亮模式和暗黑模式
  - 使用 Tailwind CSS 的 `dark:` 前缀为暗黑模式提供样式
  - 文本颜色：`text-gray-700 dark:text-gray-200`
  - 背景颜色：`bg-white dark:bg-gray-800`
  - 边框颜色：`border-gray-200 dark:border-gray-700`
  - 状态颜色保持一致：`text-blue-500`、`text-green-600`、`text-red-500` 等
- **主题切换**: 使用 `stores/theme.js` 中的 `useThemeStore()` 来实现主题切换功能
- **玻璃态效果**: 保持现有的玻璃态设计风格，在暗黑模式下调整透明度和背景色
- **图标和交互**: 确保所有图标、按钮、交互元素在两种模式下都清晰可见且易于操作

### 代码修改原则

- 对现有文件进行修改时，首先检查代码库的现有模式和风格
- 尽可能重用现有的服务和工具函数，避免重复代码
- 遵循项目现有的错误处理和日志记录模式
- 敏感数据必须使用加密存储（参考 claudeAccountService.js 中的加密实现）

### 测试和质量保证

- 运行 `npm run lint` 进行代码风格检查（使用 ESLint）
- 运行 `npm test` 执行测试套件（Jest + SuperTest 配置）
- 在修改核心服务后，使用 CLI 工具验证功能：`npm run cli status`
- 检查日志文件 `logs/claude-relay-*.log` 确认服务正常运行
- 注意：当前项目缺少实际测试文件，建议补充单元测试和集成测试

### 开发工作流

- **功能开发**: 始终从理解现有代码开始，重用已有的服务和模式
- **调试流程**: 使用 Winston 日志 + Web 界面实时日志查看 + CLI 状态工具
- **代码审查**: 关注安全性（加密存储）、性能（异步处理）、错误处理
- **部署前检查**: 运行 lint → 测试 CLI 功能 → 检查日志 → Docker 构建

### 常见文件位置

- 核心服务逻辑：`src/services/` 目录（30+服务文件）
- 路由处理：`src/routes/` 目录（api.js、admin.js、geminiRoutes.js、openaiRoutes.js等13个路由文件）
- 中间件：`src/middleware/` 目录（auth.js、browserFallback.js、debugInterceptor.js等）
- 配置管理：`config/config.js`（完整的多平台配置）
- Redis 模型：`src/models/redis.js`
- 工具函数：`src/utils/` 目录
  - `logger.js` - 日志系统
  - `oauthHelper.js` - OAuth工具
  - `proxyHelper.js` - 代理工具
  - `sessionHelper.js` - 会话管理
  - `cacheMonitor.js` - 缓存监控
  - `costCalculator.js` - 成本计算
  - `rateLimitHelper.js` - 速率限制
  - `webhookNotifier.js` - Webhook通知
  - `tokenMask.js` - Token脱敏
  - `workosOAuthHelper.js` - WorkOS OAuth
  - `modelHelper.js` - 模型工具
  - `inputValidator.js` - 输入验证
- CLI工具：`cli/index.js` 和 `src/cli/` 目录
- 脚本目录：`scripts/` 目录
  - `setup.js` - 初始化脚本
  - `manage.js` - 服务管理
  - `migrate-apikey-expiry.js` - API Key过期迁移
  - `fix-usage-stats.js` - 使用统计修复
  - `data-transfer.js` / `data-transfer-enhanced.js` - 数据导入导出
  - `update-model-pricing.js` - 模型价格更新
  - `test-pricing-fallback.js` - 价格回退测试
  - `debug-redis-keys.js` - Redis调试
- 前端主题管理：`web/admin-spa/src/stores/theme.js`
- 前端组件：`web/admin-spa/src/components/` 目录
- 前端页面：`web/admin-spa/src/views/` 目录
- 初始化数据：`data/init.json`（管理员凭据存储）
- 日志目录：`logs/`（各类日志文件）

### 重要架构决策

- **统一调度系统**: 使用统一调度器（unifiedClaudeScheduler等）实现跨账户类型的智能调度，支持粘性会话、负载均衡、故障转移
- **多账户类型支持**: 支持8种账户类型（claude-official、claude-console、bedrock、ccr、droid、gemini、openai-responses、azure-openai）
- **加密存储**: 所有敏感数据（OAuth token、refreshToken、credentials）都使用 AES 加密存储在 Redis
- **独立代理**: 每个账户支持独立的代理配置（SOCKS5/HTTP），包括OAuth授权流程
- **API Key哈希**: 使用SHA-256哈希存储，支持自定义前缀（默认 `cr_`）
- **权限系统**: API Key支持细粒度权限控制（all/claude/gemini/openai等）
- **请求流程**: API Key验证（含权限、客户端、模型黑名单） → 统一调度器选择账户 → Token刷新（如需）→ 请求转发 → Usage捕获 → 成本计算
- **流式响应**: 支持SSE流式响应，实时捕获真实usage数据，客户端断开时自动清理资源（AbortController）
- **粘性会话**: 基于请求内容hash的会话绑定，同一会话始终使用同一账户，支持自动续期
- **自动清理**: 定时清理任务（过期Key、错误账户、临时错误、并发计数、速率限制状态）
- **缓存优化**: 多层LRU缓存（解密缓存、账户缓存），全局缓存监控和统计
- **成本追踪**: 实时token使用统计（input/output/cache_create/cache_read）和成本计算（基于pricingService）
- **并发控制**: Redis Sorted Set实现的并发计数，支持自动过期清理
- **客户端识别**: 基于User-Agent的客户端限制，支持预定义客户端（ClaudeCode、Gemini-CLI等）
- **错误处理**: 529错误自动标记账户过载状态，配置时长内自动排除该账户

### 核心数据流和性能优化

- **哈希映射优化**: API Key 验证从 O(n) 优化到 O(1) 查找
- **智能 Usage 捕获**: 从 SSE 流中解析真实的 token 使用数据
- **多维度统计**: 支持按时间、模型、用户的实时使用统计
- **异步处理**: 非阻塞的统计记录和日志写入
- **原子操作**: Redis 管道操作确保数据一致性

### 安全和容错机制

- **多层加密**: API Key 哈希 + OAuth Token AES 加密
- **零信任验证**: 每个请求都需要完整的认证链
- **优雅降级**: Redis 连接失败时的回退机制
- **自动重试**: 指数退避重试策略和错误隔离
- **资源清理**: 客户端断开时的自动清理机制

## 项目特定注意事项

### Redis 数据结构

详见 [Redis 数据结构文档](docs/architecture/redis-schema.md)

### 流式响应处理

- 支持 SSE (Server-Sent Events) 流式传输，实时推送响应数据
- 自动从SSE流中解析真实usage数据（input/output/cache_create/cache_read tokens）
- 客户端断开时通过 AbortController 清理资源和并发计数
- 错误时发送适当的 SSE 错误事件（带时间戳和错误类型）
- 支持大文件流式传输（REQUEST_TIMEOUT配置超时时间）
- 禁用Nagle算法确保数据立即发送（socket.setNoDelay）
- 设置 `X-Accel-Buffering: no` 禁用Nginx缓冲

### CLI 工具使用

详见 [CLI 工具使用指南](docs/development/cli-usage.md)

## 📚 详细文档索引

### 架构文档
- [Redis 数据结构](docs/architecture/redis-schema.md) - Redis key 模式和数据结构完整说明
- [架构概览](docs/architecture/overview.md) - 系统架构设计
- [API 接口](docs/architecture/interfaces.md) - 接口规范
- [安全审计](docs/architecture/security.md) - 安全最佳实践
- [测试文档](docs/architecture/testing.md) - 测试策略和用例

### 开发指南
- [CLI 工具使用](docs/development/cli-usage.md) - 命令行工具完整参考
- [快速开始](docs/guides/quickstart.md) - 快速启动指南
- [部署指南](docs/guides/deployment.md) - 生产环境部署
- [调试指南](docs/guides/debugging.md) - 调试技巧和工具
- [API 参考](docs/guides/api-reference.md) - API 端点完整文档
- [贡献指南](docs/development/contributing.md) - 如何贡献代码
- [项目路线图](docs/development/roadmap.md) - 未来规划

### 故障排除
- [故障排除指南](docs/guides/troubleshooting.md) - 常见问题和解决方案

### 按需加载指南

当你需要以下信息时，请告诉我加载对应文档：

- **调试问题、错误排查** → 加载 [故障排除指南](docs/guides/troubleshooting.md)
- **使用 CLI 工具、数据管理** → 加载 [CLI 使用指南](docs/development/cli-usage.md)
- **理解 Redis 数据结构** → 加载 [Redis Schema](docs/architecture/redis-schema.md)
- **了解系统架构** → 加载 [架构文档](docs/architecture/overview.md)
- **查找 API 端点** → 加载 [API 参考](docs/guides/api-reference.md)

# important-instruction-reminders

Do what has been asked; nothing more, nothing less.
NEVER create files unless they're absolutely necessary for achieving your goal.
ALWAYS prefer editing an existing file to creating a new one.
NEVER proactively create documentation files (\*.md) or README files. Only create documentation files if explicitly requested by the User.
````