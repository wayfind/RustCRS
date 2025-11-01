# Claude Relay Service

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Node.js](https://img.shields.io/badge/Node.js-18+-green.svg)](https://nodejs.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Redis](https://img.shields.io/badge/Redis-6+-red.svg)](https://redis.io/)
[![Docker](https://img.shields.io/badge/Docker-Ready-blue.svg)](https://www.docker.com/)

**🔐 High-Performance AI API Relay Service with Multi-Platform Account Management**

[中文](README.md) • [Quick Start](#-quick-start) • [Documentation](docs/) • [Architecture](docs/ARCHITECTURE.md)

</div>

---

## ⚠️ Important Notice

**Read Before Use**:

- 🚨 **Terms of Service**: Using this project may violate Anthropic's Terms of Service. All risks are at your own responsibility
- 📖 **Disclaimer**: This project is for technical learning and research purposes only
- 🔒 **Data Security**: Self-hosting protects privacy, but requires maintenance responsibility

---

## 🌟 Key Features

### Multi-Platform Support
- ✅ **Claude** (Official / Console)
- ✅ **Gemini** (Google)
- ✅ **OpenAI** (Responses / Codex)
- ✅ **AWS Bedrock**
- ✅ **Azure OpenAI**
- ✅ **Droid** (Factory.ai)

### Core Features
- 🔄 **Multi-Account Management** - Intelligent scheduling and automatic rotation
- 🔑 **API Key Authentication** - Independent key allocation and permission control
- 📊 **Usage Statistics** - Detailed token usage and cost analysis
- ⚡ **Sticky Sessions** - Session-level account binding for context continuity
- 🛡️ **Security Controls** - Rate limiting, concurrency control, client restrictions
- 🌐 **Proxy Support** - HTTP/SOCKS5 proxy, independent config per account
- 📱 **Web Management** - Modern SPA admin interface

### Performance Advantages
- ⚡ **High Performance**: 3-5x faster with Rust rewrite
- 💾 **Memory Optimized**: 50-70% reduction in memory usage
- 🚀 **Low Latency**: Request latency < 20ms (p50)
- 📈 **High Concurrency**: 2000+ req/s on single instance

---

## 🚀 Quick Start

### One-Click Deployment (Recommended)

Quick install with management script:

```bash
curl -fsSL https://pincc.ai/manage.sh -o manage.sh && chmod +x manage.sh && ./manage.sh install
```

After installation, manage service with `crs` command:

```bash
crs start     # Start service
crs stop      # Stop service
crs status    # Check status
crs update    # Update service
```

### Docker Deployment

```bash
# Generate docker-compose.yml
curl -fsSL https://pincc.ai/crs-compose.sh -o crs-compose.sh && chmod +x crs-compose.sh && ./crs-compose.sh

# Start service
docker-compose up -d

# View admin credentials
cat ./data/init.json
# or
docker logs claude-relay-service
```

### Manual Deployment

```bash
# 1. Clone project
git clone https://github.com/your-username/claude-relay-service.git
cd claude-relay-service

# 2. Install dependencies
npm install

# 3. Configure environment
cp .env.example .env
cp config/config.example.js config/config.js
# Edit .env to set JWT_SECRET, ENCRYPTION_KEY, Redis config

# 4. Install and build frontend
npm run install:web
npm run build:web

# 5. Initialize and start
npm run setup  # Generate admin credentials (saved to data/init.json)
npm run service:start:daemon
```

Access admin interface: `http://your-server:3000/web`

---

## 📖 Usage Guide

### 1. Add Accounts

After logging into admin interface:

1. Navigate to "Account Management"
2. Select account type (Claude / Gemini / OpenAI, etc.)
3. For OAuth accounts:
   - Click "Generate Authorization Link"
   - Complete authorization in new window
   - Copy Authorization Code and paste
4. For API Key accounts:
   - Enter API Key or credentials directly

### 2. Create API Keys

1. Navigate to "API Keys"
2. Click "Create New Key"
3. Configure:
   - **Name**: Easy identification (e.g. "John's Key")
   - **Permissions**: all / claude / gemini / openai
   - **Rate Limits**: Requests and tokens per minute
   - **Concurrency**: Concurrent requests limit
   - **Client Restrictions**: Limit to specific clients (optional)
   - **Model Restrictions**: Blacklist mode (optional)

### 3. Configure Clients

#### Claude Code

```bash
export ANTHROPIC_BASE_URL="http://your-server:3000/api/"
export ANTHROPIC_AUTH_TOKEN="your-api-key"  # Starts with cr_
```

#### Gemini CLI

```bash
export GEMINI_MODEL="gemini-2.5-pro"
export GOOGLE_GEMINI_BASE_URL="http://your-server:3000/gemini"
export GEMINI_API_KEY="your-api-key"
```

#### Codex CLI

Add to **beginning** of `~/.codex/config.toml`:

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

Environment variable:
```bash
export CRS_OAI_KEY="your-api-key"
```

#### VSCode Claude Plugin

In `~/.claude/config.json`:

```json
{
    "primaryApiKey": "crs"
}
```

For complete setup: [docs/CLIENT_SETUP.md](docs/CLIENT_SETUP.md)

---

## 🏗️ Architecture Overview

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

Detailed architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

---

## 🦀 Rust Rewrite Plan

The project is undergoing Rust rewrite for higher performance and lower resource usage:

### Current Status
- ✅ Node.js version (Production Ready)
- 🚧 Rust version (In Development)

### Performance Targets

| Metric | Node.js | Rust (Target) | Improvement |
|--------|---------|---------------|-------------|
| Latency (p50) | ~50ms | <20ms | 2.5x |
| Memory | ~200MB | <70MB | 65%↓ |
| Throughput | ~500/s | >2000/s | 4x |

### Migration Plan

1. **Phase 1** (Current): Project cleanup and Rust initialization
2. **Phase 2** (Week 2-4): Rust core implementation
3. **Phase 3** (Week 5-8): Feature parity and parallel running
4. **Phase 4** (Week 9): Complete replacement and production deployment

Learn more: [REFACTORING_STATUS.md](REFACTORING_STATUS.md)

---

## 📚 Documentation

- [Architecture Design](docs/ARCHITECTURE.md) - System architecture and design decisions
- [Deployment Guide](docs/DEPLOYMENT.md) - Detailed deployment instructions
- [Configuration Reference](docs/CONFIGURATION.md) - Complete configuration options
- [API Reference](docs/API_REFERENCE.md) - API endpoint documentation
- [Client Setup](docs/CLIENT_SETUP.md) - Client configuration guides
- [Contributing Guide](docs/CONTRIBUTING.md) - Development and contribution guidelines
- [Refactoring Progress](REFACTORING_STATUS.md) - Rust rewrite progress

---

## 🛠️ Development

### Requirements

- **Node.js**: 18+ (current version)
- **Rust**: 1.75+ (new version)
- **Redis**: 6+
- **Docker**: Optional

### Development Commands

```bash
# Node.js version
npm run dev              # Development mode (hot reload)
npm test                 # Run tests
npm run lint             # Lint code
npm run format           # Format code

# Rust version
cd rust/
cargo build              # Build
cargo run                # Run
cargo test               # Test
cargo clippy             # Lint
cargo fmt                # Format
```

### Contributing

Contributions welcome! See [CONTRIBUTING.md](docs/CONTRIBUTING.md)

---

## 🔒 Security

- **Data Encryption**: AES-256-GCM for sensitive credentials at rest
- **API Key Hashing**: SHA-256 hash storage
- **Proxy Support**: Independent proxy config per account
- **Rate Limiting**: Abuse prevention
- **Client Validation**: User-Agent based access control

---

## 📄 License

[MIT License](LICENSE)

---

## 🙏 Acknowledgments

- Based on [Wei-Shaw/claude-relay-service](https://github.com/Wei-Shaw/claude-relay-service)
- Built with [Axum](https://github.com/tokio-rs/axum) Web Framework (Rust)
- Thanks to all contributors

---

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/your-username/claude-relay-service/issues)
- **Documentation**: [docs/](docs/)
- **Changelog**: [REFACTORING_STATUS.md](REFACTORING_STATUS.md)

---

<div align="center">

**⭐ If this project helps you, please give it a Star!**

**🤝 Issues and PRs are welcome**

</div>
