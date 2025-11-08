# 本地调试完整指南

**适用版本**: Rust 2.0.0
**更新日期**: 2025-10-31

---

## 🎯 目标

本指南帮助你在本地环境完整运行 Claude Relay Service，包括：
- ✅ Rust 后端 + Vue 3 前端（统一端口 8080）
- ✅ Redis 数据库（端口 6379）
- ✅ 配置你的 Claude/Gemini/OpenAI API Keys

---

## 📋 第一步：环境准备

### 1.1 安装必需工具

```bash
# 1. 安装 Rust（如果尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 验证安装
rustc --version  # 应显示 rustc 1.75.0 或更高

# 2. 安装 Node.js（如果尚未安装）
# 推荐使用 Node.js 18+
node --version  # 应显示 v18.x 或更高
npm --version

# 3. 安装 Docker（用于运行 Redis）
docker --version
```

### 1.2 克隆并进入项目

```bash
cd /mnt/d/prj/claude-relay-service
# 或
cd /home/david/prj/claude-relay-service
```

---

## 🔐 第二步：配置环境变量（重要！）

### 2.1 创建本地 .env 文件

```bash
# 复制模板
cp .env.example .env
```

### 2.2 编辑 .env 文件（必填项）

打开 `.env` 文件，配置以下**必填项**：

```bash
# ========================================
# 🔐 安全配置（必填！）
# ========================================

# JWT 密钥（至少 32 字符，用于生成访问令牌）
CRS_SECURITY__JWT_SECRET=your-very-long-jwt-secret-at-least-32-characters-long-please

# 加密密钥（必须恰好 32 字符，用于加密敏感数据）
CRS_SECURITY__ENCRYPTION_KEY=12345678901234567890123456789012

# ========================================
# 🌐 服务器配置
# ========================================
CRS_SERVER__HOST=0.0.0.0
CRS_SERVER__PORT=8080

# ========================================
# 📊 Redis 配置
# ========================================
CRS_REDIS__HOST=localhost
CRS_REDIS__PORT=6379
# CRS_REDIS__PASSWORD=  # 本地调试通常不需要密码
CRS_REDIS__DB=0

# ========================================
# 📝 日志配置
# ========================================
CRS_LOGGING__LEVEL=debug  # 开发环境使用 debug，生产环境用 info
CRS_LOGGING__FORMAT=pretty  # pretty 易读，json 用于生产

# Rust 日志详细程度
RUST_LOG=debug,hyper=info,tokio=info
```

### 2.3 配置 API Keys（可选，调试时需要）

如果你想测试实际的 API 转发功能，需要配置以下 API Keys：

```bash
# ========================================
# 🔑 AI API Keys（调试用，可选）
# ========================================

# Claude API Key（如果有）
# CLAUDE_API_KEY=sk-ant-api03-xxxxxxxxxx

# Gemini API Key（如果有）
# GEMINI_API_KEY=AIzaSyxxxxxxxxxx

# OpenAI API Key（如果有）
# OPENAI_API_KEY=sk-xxxxxxxxxx
```

**注意**：
- ✅ 这些 API Keys 仅存储在本地 `.env` 文件中
- ✅ `.env` 文件已被 `.gitignore` 排除，**永远不会被提交到 Git**
- ✅ 如果你只想测试系统功能（不调用真实 API），可以跳过此步骤

---

## 🚀 第三步：启动服务

### 3.1 启动 Redis（使用 Docker）

```bash
# 启动 Redis 容器（后台运行）
docker run -d \
  --name redis-dev \
  -p 6379:6379 \
  redis:7-alpine

# 验证 Redis 运行
docker ps | grep redis-dev
redis-cli ping  # 应返回 PONG
```

**停止 Redis**（不需要时）：
```bash
docker stop redis-dev
docker rm redis-dev
```

### 3.2 启动 Rust 后端

**方法 1：开发模式（快速启动，带调试符号）**

```bash
cd rust/

# 确保环境变量已加载（或手动设置）
export ENCRYPTION_KEY="12345678901234567890123456789012"

# 启动开发服务器
cargo run

# 或者使用 cargo watch 实现热重载（需要先安装）
# cargo install cargo-watch
# cargo watch -x run
```

**方法 2：发布模式（最佳性能）**

```bash
cd rust/

# 构建发布版本（首次需要几分钟）
cargo build --release

# 运行发布版本
ENCRYPTION_KEY="12345678901234567890123456789012" \
  ./target/release/claude-relay
```

**验证 Rust 后端启动成功**：

```bash
# 健康检查
curl http://localhost:8080/health

# 应返回类似：
# {"status":"ok","redis":"connected","timestamp":"2025-10-31T..."}
```

### 3.3 启动前端界面

打开**新的终端窗口**：

```bash
cd web/admin-spa/

# 首次运行需要安装依赖
npm install

# 构建前端静态资源
npm run build

# 前端资源将输出到 dist/ 目录，由 Rust 后端提供服务
```

**前端配置说明**：
- 前端编译后的静态文件位于 `web/admin-spa/dist/`
- Rust 后端在端口 8080 同时提供 API 和静态文件服务
- 访问地址：`http://localhost:8080` 或 `http://localhost:8080/admin-next`

---

## ✅ 第四步：验证系统运行

### 4.1 验证后端健康

```bash
# 健康检查
curl http://localhost:8080/health

# 系统指标
curl http://localhost:8080/metrics

# 测试 API（如果配置了 API Key）
curl -X POST http://localhost:8080/api/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: cr_your_api_key_here" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 100
  }'
```

### 4.2 验证前端界面

1. 打开浏览器访问 `http://localhost:8080`（根路径自动跳转到 `/admin-next`）
2. 应该看到 Claude Relay Service 管理界面
3. 尝试登录（如果已配置管理员账户）
4. 检查仪表板、账户管理、API Key 管理等功能
5. 打开浏览器开发者工具，检查控制台是否有错误

### 4.3 验证 Redis 连接

```bash
# 进入 Redis CLI
redis-cli

# 查看所有键
KEYS *

# 退出
exit
```

---

## 🐛 常见问题排查

### 问题 1：Rust 后端启动失败

**错误**: `CRS_SECURITY__ENCRYPTION_KEY must be set`

**解决方案**：
```bash
# 确保 .env 文件存在且配置正确
cat .env | grep ENCRYPTION_KEY

# 或直接设置环境变量
export CRS_SECURITY__ENCRYPTION_KEY="12345678901234567890123456789012"
cargo run
```

### 问题 2：Redis 连接失败

**错误**: `Connection refused (os error 111)`

**解决方案**：
```bash
# 检查 Redis 是否运行
docker ps | grep redis-dev

# 如果没有运行，启动 Redis
docker run -d --name redis-dev -p 6379:6379 redis:7-alpine

# 测试连接
redis-cli ping
```

### 问题 3：端口被占用

**错误**: `Address already in use (os error 98)`

**解决方案**：
```bash
# 检查端口占用
lsof -i :8080  # Rust 后端（包含前端）
lsof -i :6379  # Redis

# 杀死占用进程
kill -9 <PID>

# 或修改端口
export CRS_SERVER__PORT=8081  # Rust 后端换端口
```

### 问题 4：前端界面无法加载

**错误**: 访问 `http://localhost:8080` 显示 404 或空白页面

**解决方案**：
```bash
# 1. 确认 Rust 后端正在运行
curl http://localhost:8080/health

# 2. 检查前端是否已构建
ls -la web/admin-spa/dist/

# 3. 如果 dist/ 目录为空，重新构建前端
cd web/admin-spa/
npm run build

# 4. 重启 Rust 后端
cd ../../rust/
cargo run
```

### 问题 5：编译错误

**错误**: `cargo build` 失败

**解决方案**：
```bash
# 清理并重新构建
cd rust/
cargo clean
cargo build

# 更新 Rust 工具链
rustup update

# 检查 Cargo.toml 依赖
cat Cargo.toml
```

---

## 🔄 开发工作流

### 日常开发流程

```bash
# 1. 启动 Redis（仅需一次）
docker start redis-dev || docker run -d --name redis-dev -p 6379:6379 redis:7-alpine

# 2. 启动 Rust 后端（终端 1）
cd rust/
cargo run

# 3. 启动前端（终端 2）
cd web/admin-spa/
npm run dev

# 4. 开始开发！
# - Rust 代码修改后需要重新编译（Ctrl+C 然后 cargo run）
# - 前端代码自动热重载
```

### 运行测试

```bash
# Rust 单元测试
cd rust/
cargo test

# Rust 集成测试（自动启动临时 Redis）
bash run-integration-tests.sh

# Rust 性能基准测试
cargo bench

# 前端测试（如果有）
cd web/admin-spa/
npm test
```

### 代码格式化

```bash
# Rust 代码格式化
cd rust/
cargo fmt
cargo clippy

# 前端代码格式化
cd web/admin-spa/
npx prettier --write "src/**/*.{js,vue}"
```

---

## 📊 监控和调试

### 查看日志

**Rust 后端日志**：
- 终端直接输出（pretty 格式）
- 日志级别通过 `CRS_LOGGING__LEVEL` 控制

**调整日志级别**：
```bash
# 临时调整
RUST_LOG=trace cargo run

# 或修改 .env
CRS_LOGGING__LEVEL=trace
```

### 调试技巧

```bash
# 1. 使用 Rust 调试器
cd rust/
rust-lldb target/debug/claude-relay

# 2. 查看 Redis 数据
redis-cli
KEYS *
GET api_key:some_id

# 3. 监控 HTTP 请求
# 在 Rust 代码中日志级别设为 debug，会输出所有请求详情
```

---

## 🔐 安全最佳实践

### 本地开发

1. **永远不要提交 .env 文件**
   ```bash
   # 验证 .env 被忽略
   git status | grep .env  # 不应出现
   ```

2. **使用强随机密钥**
   ```bash
   # 生成随机 JWT Secret
   openssl rand -base64 48

   # 生成随机 Encryption Key（必须32字符）
   openssl rand -hex 16
   ```

3. **定期轮换密钥**
   - 开发环境每月轮换
   - 生产环境每季度轮换

### API Keys 管理

- ✅ 仅在 `.env` 中存储真实 API Keys
- ✅ 使用环境变量而非硬编码
- ✅ 团队共享时使用密钥管理工具（如 1Password）

---

## 🎉 完成！

如果所有步骤都成功，你现在应该有：

- ✅ Rust 后端 + 前端统一运行在 `http://localhost:8080`
- ✅ Redis 运行在 `localhost:6379`
- ✅ 完整的本地调试环境

**下一步**：
1. 探索管理界面功能
2. 创建测试 API Key
3. 配置 Claude/Gemini 账户
4. 测试 API 转发功能

---

## 📚 相关文档

- [README.md](README.md) - 项目概览
- [MIGRATION.md](MIGRATION.md) - 迁移指南
- [rust/README.md](rust/README.md) - Rust 实现说明
- [docs/INTERFACE.md](docs/INTERFACE.md) - API 文档

**遇到问题？** 查看 [MIGRATION.md 故障排查章节](MIGRATION.md#故障排查)

---

**祝调试愉快！** 🚀
