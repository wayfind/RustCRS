# Quickstart Guide - Claude Relay Service

**⏱️ Estimated Time**: 5-10 minutes
**🎯 Goal**: Quickly set up local development environment and start debugging
**🛠️ Platform**: Rust-based implementation (migrated from Node.js)

---

## Table of Contents

1. [One-Command Startup (Recommended)](#-one-command-startup-recommended)
2. [First-Time Setup (One Time Only)](#-first-time-setup-one-time-only)
3. [Manual Startup (Full Control)](#-manual-startup-full-control)
4. [Verify Services](#-verify-services)
5. [Startup Methods Comparison](#-startup-methods-comparison)
6. [Using the Admin Interface](#-using-the-admin-interface)
7. [Quick Troubleshooting](#-quick-troubleshooting)
8. [Next Steps](#-next-steps)

---

## 🚀 One-Command Startup (Recommended)

```bash
# 1. (Optional but recommended) Verify environment
bash verify-setup.sh

# 2. Start development environment with one command
bash start-dev.sh
```

**That's it!** The script automatically:
- ✅ Checks required tools (Docker, Rust, Node.js)
- ✅ Validates environment variable configuration
- ✅ Starts Redis container
- ✅ Compiles and runs Rust backend
- ✅ Prompts to start frontend interface

**Alternative Commands**:
```bash
# Using Makefile
make rust-dev          # Complete environment: Redis + Backend + Frontend

# Using scripts
bash rust-dev.sh       # Same as start-dev.sh
bash scripts/start-all.sh dev  # Full control with options
```

---

## 📋 First-Time Setup (One Time Only)

### Step 1: Create Environment Configuration

```bash
# Copy template to create .env file
cp .env.example .env
```

### Step 2: Configure Required Variables

Edit `.env` file and set the following **required** environment variables:

```bash
# 🔐 Security Configuration (Required)
CRS_SECURITY__JWT_SECRET=your-very-long-jwt-secret-at-least-32-characters-long-please
CRS_SECURITY__ENCRYPTION_KEY=12345678901234567890123456789012  # Must be exactly 32 characters

# 📊 Redis Configuration
CRS_REDIS__HOST=localhost
CRS_REDIS__PORT=6379

# 📝 Logging Configuration
CRS_LOGGING__LEVEL=debug
CRS_LOGGING__FORMAT=pretty
RUST_LOG=debug,hyper=info,tokio=info
```

**💡 Generate Secure Keys**:
```bash
# JWT Secret (48+ characters recommended)
openssl rand -base64 48

# Encryption Key (exactly 32 characters)
openssl rand -hex 16
```

### Step 3: Configure API Keys (Optional but Recommended)

**Optional but recommended** - If you want to test actual API forwarding:

```bash
# Add your API Keys to .env file
# ⚠️ These keys are stored locally only and never committed to Git

# Claude API Key (if available)
CLAUDE_API_KEY=sk-ant-api03-xxxxxxxxxxxxxxxxxxxxxxxxxx

# Gemini API Key (if available)
GEMINI_API_KEY=AIzaSyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

# OpenAI API Key (if available)
OPENAI_API_KEY=sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

**🔒 Security Guarantee**:
- ✅ `.env` file is in `.gitignore` and never committed
- ✅ All API Keys are stored on your local machine only
- ✅ You can add/remove accounts in the Web interface anytime

---

## 🎯 Manual Startup (Full Control)

If you want manual control over each step:

### 1. Start Redis

```bash
# Create and start Redis container
docker run -d --name redis-dev -p 6379:6379 redis:7-alpine

# Verify Redis is running
redis-cli ping  # Should return PONG
```

### 2. Start Rust Backend

**Development Mode (quick startup)**:
```bash
# From project root (recommended - Cargo workspace configured)
cargo run

# Or from rust/ directory
cd rust/
cargo run
```

**Release Mode (best performance)**:
```bash
cd rust/
cargo build --release
./target/release/claude-relay
```

Backend should display after startup:
```
🚀 Server running on http://0.0.0.0:8080
✅ Redis connected
```

### 3. Start Frontend Interface

Open a **new terminal window**:

```bash
cd web/admin-spa/

# First-time setup: install dependencies
npm install

# Start development server
npm run dev
```

Vite will automatically open browser at `http://localhost:3001` (开发模式)

**注意**: 在生产环境，静态文件由 Rust 后端直接提供服务，访问 `http://localhost:8080` 即可（根路径自动跳转到 `/admin-next`）

---

## ✅ Verify Services

### Backend Health Check

```bash
curl http://localhost:8080/health
```

**Expected Output**:
```json
{
  "status": "ok",
  "redis": "connected",
  "timestamp": "2025-10-31T..."
}
```

### Frontend Access

**开发模式**: Open browser: **http://localhost:3001** (Vite 开发服务器)

**生产环境**: Open browser: **http://localhost:8080** (Rust 后端提供静态文件服务)
- 访问根路径 `/` 会自动跳转到 `/admin-next`
- 可直接访问 `http://localhost:8080/admin-next`

You should see the Claude Relay Service admin interface.

---

## 🎯 Startup Methods Comparison

### Quick Reference Table

| Command | Starts | Use Case | Time |
|---------|--------|----------|------|
| `cargo run` | **Backend only** | Backend development, API testing | ~2s |
| `make rust-backend` | **Backend only** | Same as cargo run | ~2s |
| `make rust-frontend` | **Frontend only** | Frontend development (backend must be running) | ~5s |
| `make rust-dev` | **Complete environment** | Redis + Backend + Frontend (recommended) | ~15s |
| `make start-all` | **Complete environment** | Same as rust-dev | ~15s |
| `bash rust-dev.sh` | **Complete environment** | Interactive startup with options | ~15s |

### Detailed Comparison

#### cargo run vs make rust-dev

| Feature | `cargo run` | `make rust-dev` |
|---------|------------|----------------|
| Redis | ❌ Manual start required | ✅ Auto-starts |
| Backend | ✅ Starts | ✅ Starts |
| Frontend | ❌ Not started | ✅ Optional start |
| Environment Check | ❌ None | ✅ Auto-checks |
| Best For | Backend development | Complete environment |

### When to Use Each Method

| Scenario | Recommended Command | Reason |
|----------|-------------------|---------|
| Pure backend development | `cargo run` | Fastest, focused on backend |
| Pure frontend development | `make rust-frontend` | Frontend hot reload |
| Frontend + backend testing | `make rust-dev` | Complete environment, automated |
| First-time setup | `make rust-dev` | One-command startup, interactive |
| Quick testing | `bash rust-dev.sh` | Scripted, customizable |
| Production deployment | `make rust-release` | Performance optimized |

### Important Notes

**⚠️ Why `cargo run` doesn't start frontend?**
- Backend and frontend are separate concerns
- Allows backend development without frontend overhead
- Frontend can hot reload independently
- Follows Cargo's design philosophy (managing Rust projects only)

**For complete environment**, use:
```bash
make rust-dev           # Recommended
make start-all         # Same as above
bash rust-dev.sh       # Interactive
```

---

## 🎨 Using the Admin Interface

**访问方式**:
- **开发模式**: `http://localhost:3001` (Vite 开发服务器，支持热重载)
- **生产环境**: `http://localhost:8080` 或 `http://localhost:8080/admin-next` (Rust 后端提供静态文件服务)

### 1. Login to Admin Interface

Default admin credentials are stored in `data/init.json`

### 2. Add Accounts

- **Claude Account**: OAuth authorization flow
- **Gemini Account**: Google OAuth authorization
- **OpenAI Account**: Direct API Key input
- **Other Platforms**: Bedrock, Azure, Droid, CCR

### 3. Create API Key

- Set name and quota
- Choose service permissions (all/claude/gemini/openai)
- Configure client restrictions and model blacklist

### 4. Test API Forwarding

```bash
# Use your created API Key
curl -X POST http://localhost:8080/api/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: cr_your_api_key_here" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 100
  }'
```

---

## 🐛 Quick Troubleshooting

### ❌ Redis Connection Failed

```bash
Error: "Connection refused (os error 111)"

# Solution:
docker ps | grep redis-dev  # Check if Redis is running
docker start redis-dev      # If exists but not running
# Or
docker run -d --name redis-dev -p 6379:6379 redis:7-alpine
```

### ❌ ENCRYPTION_KEY Not Set

```bash
Error: "CRS_SECURITY__ENCRYPTION_KEY must be set"

# Solution:
# Ensure .env file exists and contains a 32-character ENCRYPTION_KEY
cat .env | grep ENCRYPTION_KEY
```

### ❌ Port Already in Use

```bash
Error: "Address already in use (os error 98)"

# Check port usage:
lsof -i :8080  # Rust backend
lsof -i :3001  # Frontend
lsof -i :6379  # Redis

# Kill occupying process:
kill -9 <PID>
```

### ❌ Frontend Cannot Connect to Backend

```bash
# Confirm backend is running
curl http://localhost:8080/health

# Check frontend proxy configuration (should point to localhost:8080)
cat web/admin-spa/vite.config.js | grep apiTarget

# Restart frontend
cd web/admin-spa/
npm run dev
```

### ❌ JWT_SECRET Error

```bash
Error: "JWT_SECRET must be at least 32 characters"

# Solution:
# Ensure .env file is in project root
cat .env | grep JWT_SECRET

# JWT_SECRET should be at least 32 characters
# Generate using:
openssl rand -base64 48
```

### ❌ Docker Permission Issues

```bash
Error: "permission denied while trying to connect to Docker daemon"

# Solution:
# Add user to docker group
sudo usermod -aG docker $USER && newgrp docker

# Or restart terminal and verify:
docker ps
```

---

## 📚 Next Steps

### Learn More

- **Complete Debugging Guide**: [LOCAL_DEBUG_GUIDE.md](../LOCAL_DEBUG_GUIDE.md)
- **Migration Documentation**: [MIGRATION.md](../../MIGRATION.md)
- **API Documentation**: [INTERFACE.md](../INTERFACE.md)
- **Project Architecture**: [CLAUDE.md](../../CLAUDE.md)
- **Testing Guide**: [testing.md](../architecture/testing.md)

### Development Workflow

**Backend Development**:
```bash
# Quick iteration - unit tests only
cargo test --lib

# With auto-reload
cargo install cargo-watch
cargo watch -x run
```

**Frontend Development**:
```bash
# Backend running in background
make rust-backend  # Choose option 2 (background)

# Frontend with hot reload
make rust-frontend
```

**Full Testing**:
```bash
# Complete check before commit
make rust-check  # Format + lint + unit tests
```

---

## 🆘 Need Help?

1. **Check Logs**: Backend startup displays detailed logs
2. **Run Verification**: `bash verify-setup.sh`
3. **View Complete Guide**: [LOCAL_DEBUG_GUIDE.md](../LOCAL_DEBUG_GUIDE.md)
4. **Check Troubleshooting**: [MIGRATION.md#troubleshooting](../../MIGRATION.md#troubleshooting)

### Log Locations

- Application logs: `logs/claude-relay-*.log`
- Token refresh errors: `logs/token-refresh-error.log`
- Webhook logs: `logs/webhook-*.log`
- HTTP debug: `logs/http-debug-*.log` (requires DEBUG_HTTP_TRAFFIC=true)

### Verification Commands

```bash
bash verify-setup.sh  # Comprehensive environment check
npm run status        # System status view
npm run data:debug    # Redis data debugging
```

---

## 🎉 Success Checklist

- [ ] Environment configured (`.env` file created)
- [ ] Redis running (`docker ps | grep redis-dev`)
- [ ] Backend healthy (`curl http://localhost:8080/health`)
- [ ] Frontend accessible (`http://localhost:3001`)
- [ ] Admin login successful
- [ ] Test account added
- [ ] API Key created
- [ ] API call successful

---

**Happy debugging!** 🚀

*Last Updated: 2025-11-01*
*Rust Version: 1.1.187*
