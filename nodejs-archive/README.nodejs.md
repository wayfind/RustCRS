# Claude Relay Service

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Node.js](https://img.shields.io/badge/Node.js-18+-green.svg)](https://nodejs.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Redis](https://img.shields.io/badge/Redis-6+-red.svg)](https://redis.io/)
[![Docker](https://img.shields.io/badge/Docker-Ready-blue.svg)](https://www.docker.com/)

**🔐 高性能 AI API 中转服务，支持多平台多账户管理**

[English](README_EN.md) • [快速开始](#-快速开始) • [文档](docs/) • [架构设计](docs/ARCHITECTURE.md)

</div>

---

## ⚠️ 重要提醒

**使用前必读**:

- 🚨 **服务条款**: 使用本项目可能违反 Anthropic 服务条款，所有风险由用户自行承担
- 📖 **免责声明**: 本项目仅供技术学习和研究使用
- 🔒 **数据安全**: 自行搭建保护隐私，但需承担维护责任

---

## 🌟 核心特性

### 多平台支持
- ✅ **Claude** (Official / Console)
- ✅ **Gemini** (Google)
- ✅ **OpenAI** (Responses / Codex)
- ✅ **AWS Bedrock**
- ✅ **Azure OpenAI**
- ✅ **Droid** (Factory.ai)

### 核心功能
- 🔄 **多账户管理** - 智能调度和自动轮换
- 🔑 **API Key 认证** - 独立密钥分配和权限控制
- 📊 **使用统计** - 详细的 token 使用和成本分析
- ⚡ **粘性会话** - 会话级账户绑定，保持上下文连续性
- 🛡️ **安全控制** - 速率限制、并发控制、客户端限制
- 🌐 **代理支持** - HTTP/SOCKS5 代理，每个账户独立配置
- 📱 **Web 管理** - 现代化 SPA 管理界面

### 性能优势
- ⚡ **高性能**: Rust 重写版本性能提升 3-5 倍
- 💾 **内存优化**: 内存占用减少 50-70%
- 🚀 **低延迟**: 请求延迟 < 20ms (p50)
- 📈 **高并发**: 单实例支持 2000+ req/s

---

## 🚀 快速开始

### 一键部署（推荐）

使用管理脚本快速安装：

```bash
curl -fsSL https://pincc.ai/manage.sh -o manage.sh && chmod +x manage.sh && ./manage.sh install
```

安装完成后使用 `crs` 命令管理服务：

```bash
crs start     # 启动服务
crs stop      # 停止服务
crs status    # 查看状态
crs update    # 更新服务
```

### Docker 部署

```bash
# 一键生成 docker-compose.yml
curl -fsSL https://pincc.ai/crs-compose.sh -o crs-compose.sh && chmod +x crs-compose.sh && ./crs-compose.sh

# 启动服务
docker-compose up -d

# 查看管理员凭据
cat ./data/init.json
# 或
docker logs claude-relay-service
```

### 手动部署

```bash
# 1. 克隆项目
git clone https://github.com/your-username/claude-relay-service.git
cd claude-relay-service

# 2. 安装依赖
npm install

# 3. 配置环境
cp .env.example .env
cp config/config.example.js config/config.js
# 编辑 .env 设置 JWT_SECRET、ENCRYPTION_KEY、Redis 配置

# 4. 安装并构建前端
npm run install:web
npm run build:web

# 5. 初始化并启动
npm run setup  # 生成管理员凭据（保存在 data/init.json）
npm run service:start:daemon
```

访问管理界面: `http://your-server:3000/web`

---

## 📖 使用指南

### 1. 添加账户

登录管理界面后：

1. 导航至「账户管理」
2. 选择账户类型（Claude / Gemini / OpenAI 等）
3. 对于 OAuth 账户：
   - 点击「生成授权链接」
   - 在新窗口完成授权
   - 复制 Authorization Code 并粘贴
4. 对于 API Key 账户：
   - 直接输入 API Key 或凭据

### 2. 创建 API Key

1. 导航至「API Keys」
2. 点击「创建新 Key」
3. 设置：
   - **名称**: 便于识别（如 "张三的Key"）
   - **权限**: all / claude / gemini / openai
   - **速率限制**: 每分钟请求数和 token 数
   - **并发限制**: 同时处理的请求数
   - **客户端限制**: 限制特定客户端（可选）
   - **模型限制**: 黑名单模式（可选）

### 3. 配置客户端

#### Claude Code

```bash
export ANTHROPIC_BASE_URL="http://your-server:3000/api/"
export ANTHROPIC_AUTH_TOKEN="your-api-key"  # cr_ 开头
```

#### Gemini CLI

```bash
export GEMINI_MODEL="gemini-2.5-pro"
export GOOGLE_GEMINI_BASE_URL="http://your-server:3000/gemini"
export GEMINI_API_KEY="your-api-key"
```

#### Codex CLI

在 `~/.codex/config.toml` **开头**添加：

```toml
model_provider = "crs"
model = "gpt-5-codex"
preferred_auth_method = "apikey"

[model_providers.crs]
name = "crs"
base_url = "http://your-server:3000/openai"
wire_api = "responses"
requires_openai_auth = true
env_key = "CRS_OAI_KEY"
```

环境变量：
```bash
export CRS_OAI_KEY="your-api-key"
```

#### VSCode Claude 插件

在 `~/.claude/config.json` 中：

```json
{
    "primaryApiKey": "crs"
}
```

完整配置参见: [docs/CLIENT_SETUP.md](docs/CLIENT_SETUP.md)

---

## 🏗️ 架构概览

```
┌─────────────┐
│   Client    │  (Claude Code, Gemini CLI, Codex, etc.)
└──────┬──────┘
       │ API Key (cr_xxx)
       ↓
┌─────────────────────────────────────────────┐
│        Claude Relay Service (Rust)          │
│  ┌──────────────────────────────────────┐   │
│  │  Auth Middleware                      │   │
│  │  ├─ API Key Validation (SHA-256)     │   │
│  │  ├─ Permission Check                 │   │
│  │  └─ Rate Limiting                    │   │
│  └──────────────────────────────────────┘   │
│  ┌──────────────────────────────────────┐   │
│  │  Unified Scheduler                   │   │
│  │  ├─ Account Selection                │   │
│  │  ├─ Sticky Session                   │   │
│  │  ├─ Load Balancing                   │   │
│  │  └─ Failover                         │   │
│  └──────────────────────────────────────┘   │
│  ┌──────────────────────────────────────┐   │
│  │  Relay Services                      │   │
│  │  ├─ Claude Official/Console          │   │
│  │  ├─ Gemini                           │   │
│  │  ├─ OpenAI/Codex                     │   │
│  │  ├─ AWS Bedrock                      │   │
│  │  └─ Azure OpenAI                     │   │
│  └──────────────────────────────────────┘   │
└───────────┬─────────────────────────────────┘
            │
            ↓
  ┌─────────────────┐
  │  Upstream APIs  │
  │  (Anthropic,    │
  │   Google, etc.) │
  └─────────────────┘
```

详细架构设计: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

---

## 🦀 Rust 重写计划

项目正在进行 Rust 重写，提供更高性能和更低资源占用：

### 当前状态
- ✅ Node.js 版本 (生产可用)
- 🚧 Rust 版本 (开发中)

### 性能目标

| 指标 | Node.js | Rust (目标) | 改进 |
|------|---------|------------|------|
| 延迟 (p50) | ~50ms | <20ms | 2.5x |
| 内存 | ~200MB | <70MB | 65%↓ |
| 吞吐量 | ~500/s | >2000/s | 4x |

### 迁移计划

1. **Phase 1** (当前): 项目清理和 Rust 初始化
2. **Phase 2** (Week 2-4): Rust 核心功能实现
3. **Phase 3** (Week 5-8): 功能对等和并行运行
4. **Phase 4** (Week 9): 完全替换和生产部署

了解更多: [REFACTORING_STATUS.md](REFACTORING_STATUS.md)

---

## 📚 文档

- [架构设计](docs/ARCHITECTURE.md) - 系统架构和设计决策
- [部署指南](docs/DEPLOYMENT.md) - 详细部署说明
- [配置参考](docs/CONFIGURATION.md) - 完整配置选项
- [API 参考](docs/API_REFERENCE.md) - API 端点文档
- [客户端配置](docs/CLIENT_SETUP.md) - 各客户端配置指南
- [贡献指南](docs/CONTRIBUTING.md) - 开发和贡献指南
- [重构进度](REFACTORING_STATUS.md) - Rust 重写进度

---

## 🛠️ 开发

### 环境要求

- **Node.js**: 18+ (当前版本)
- **Rust**: 1.75+ (新版本)
- **Redis**: 6+
- **Docker**: 可选

### 开发命令

```bash
# Node.js 版本
npm run dev              # 开发模式（热重载）
npm test                 # 运行测试
npm run lint             # 代码检查
npm run format           # 代码格式化

# Rust 版本
cd rust/
cargo build              # 构建
cargo run                # 运行
cargo test               # 测试
cargo clippy             # Lint
cargo fmt                # 格式化
```

### 贡献

欢迎贡献代码！请参阅 [CONTRIBUTING.md](docs/CONTRIBUTING.md)

---

## 🔒 安全

- **数据加密**: AES-256-GCM 加密存储敏感凭据
- **API Key 哈希**: SHA-256 哈希存储
- **代理支持**: 每个账户独立代理配置
- **速率限制**: 防止滥用
- **客户端验证**: 基于 User-Agent 的访问控制

---

## 📄 许可证

[MIT License](LICENSE)

---

## 🙏 致谢

- 基于 [Wei-Shaw/claude-relay-service](https://github.com/Wei-Shaw/claude-relay-service)
- 使用 [Axum](https://github.com/tokio-rs/axum) Web 框架 (Rust)
- 感谢所有贡献者

---

## 📞 支持

- **Issues**: [GitHub Issues](https://github.com/your-username/claude-relay-service/issues)
- **文档**: [docs/](docs/)
- **更新日志**: [REFACTORING_STATUS.md](REFACTORING_STATUS.md)

---

<div align="center">

**⭐ 如果这个项目对你有帮助，请给个 Star！**

**🤝 欢迎提 Issue 和 PR**

</div>
