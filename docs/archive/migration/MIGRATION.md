# Node.js 到 Rust 迁移指南

**迁移日期**: 2025-10-31
**状态**: ✅ 完成
**Rust 版本**: 2.0.0
**Node.js 归档版本**: 1.0.0

---

## 📋 迁移概述

Claude Relay Service 已从 Node.js 完全迁移到 Rust，实现了更高的性能、更低的内存占用和更强的并发处理能力。Node.js 代码已归档至 `nodejs-archive/` 目录作为参考。

### 核心改进

| 指标 | Node.js (旧版) | Rust (新版) | 提升 |
|------|--------------|------------|------|
| 请求延迟 (p50) | ~50ms | <20ms | **2.5x 更快** |
| 内存使用 | ~200MB | <70MB | **65% 减少** |
| 并发吞吐量 | ~500 req/s | >2000 req/s | **4x 提升** |
| CPU 效率 | 基准线 | +50% | **更高效** |

---

## 🔄 主要变化

### 1. 项目结构

#### 旧结构 (Node.js)
```
claude-relay-service/
├── src/                 # Node.js 源代码
├── scripts/             # 工具脚本
├── cli/                 # 命令行工具
├── package.json         # Node.js 依赖
└── config/              # 配置文件
```

#### 新结构 (Rust)
```
claude-relay-service/
├── rust/                # Rust 后端（主实现）
│   ├── src/
│   ├── tests/
│   └── Cargo.toml
├── nodejs-archive/      # Node.js 代码归档
│   ├── src/
│   ├── scripts/
│   └── package.json
├── web/                 # 前端界面（不变）
│   └── admin-spa/
├── config/              # 配置模板
├── .env.example         # Rust 格式环境变量
└── docker-compose.yml   # Rust 后端配置
```

### 2. 端口变化

| 服务 | Node.js 端口 | Rust 端口 |
|------|------------|----------|
| HTTP API | 3000 | **8080** |
| 前端开发服务器 | 3001 | 3001 (不变) |
| Redis | 6379 | 6379 (不变) |

**重要**: 需要更新所有客户端配置中的端口号（3000 → 8080）

### 3. 环境变量格式

#### Node.js 格式（旧）
```bash
JWT_SECRET=your_jwt_secret
ENCRYPTION_KEY=12345678901234567890123456789012
PORT=3000
REDIS_HOST=localhost
REDIS_PORT=6379
```

#### Rust 格式（新）
```bash
CRS_SECURITY__JWT_SECRET=your_jwt_secret_minimum_32_characters_long
CRS_SECURITY__ENCRYPTION_KEY=12345678901234567890123456789012
CRS_SERVER__PORT=8080
CRS_REDIS__HOST=localhost
CRS_REDIS__PORT=6379
```

**迁移步骤**:
```bash
# 1. 备份旧配置
cp .env .env.nodejs.backup

# 2. 使用新模板
cp .env.example .env

# 3. 手动迁移配置值（注意变量名格式变化）
```

### 4. Docker 部署

#### Node.js Docker 命令（旧）
```bash
docker-compose up -d
# 暴露端口: 3000
```

#### Rust Docker 命令（新）
```bash
docker-compose up -d
# 暴露端口: 8080
# 首次构建时间较长（编译 Rust 依赖）
```

**注意**: Rust Docker 镜像使用多阶段构建，最终镜像更小（~50MB vs ~150MB）

---

## 🚀 快速开始

### 本地开发环境

#### 1. 安装 Rust（如果尚未安装）
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### 2. 配置环境变量
```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，设置必填项：
# - CRS_SECURITY__JWT_SECRET (至少32字符)
# - CRS_SECURITY__ENCRYPTION_KEY (必须32字符)
```

#### 3. 启动 Redis
```bash
docker run -d --name redis-dev -p 6379:6379 redis:7-alpine
```

#### 4. 构建并运行 Rust 后端
```bash
cd rust/
cargo build --release
ENCRYPTION_KEY="12345678901234567890123456789012" ./target/release/claude-relay
```

#### 5. 启动前端界面
```bash
cd web/admin-spa/
npm install
npm run dev  # 访问 http://localhost:3001
```

### Docker 部署

```bash
# 1. 设置必需的环境变量（创建 .env 文件）
export JWT_SECRET="your-jwt-secret-minimum-32-chars"
export ENCRYPTION_KEY="12345678901234567890123456789012"

# 2. 启动所有服务
docker-compose up -d

# 3. 查看日志
docker-compose logs -f claude-relay

# 4. 检查健康状态
curl http://localhost:8080/health
```

---

## ⚙️ API 兼容性

### 完全兼容的端点

Rust 版本保持了与 Node.js 版本 100% 的 API 兼容性：

- ✅ `/api/v1/messages` - Claude 消息处理
- ✅ `/claude/v1/messages` - Claude 消息别名
- ✅ `/gemini/v1/models/:model:generateContent` - Gemini API
- ✅ `/openai/v1/chat/completions` - OpenAI 兼容
- ✅ `/admin/*` - 管理API端点
- ✅ `/health` - 健康检查
- ✅ `/metrics` - 系统指标

### 行为差异

| 功能 | Node.js 行为 | Rust 行为 | 影响 |
|------|------------|----------|------|
| 错误响应格式 | JSON | JSON (相同) | ✅ 无影响 |
| 流式SSE格式 | Server-Sent Events | Server-Sent Events (相同) | ✅ 无影响 |
| 超时处理 | 10分钟 | 10分钟 (可配置) | ✅ 无影响 |
| 并发限制 | 基于内存 | 更高并发 | ✅ 性能提升 |

---

## 🔧 故障排查

### 常见问题

#### 1. 端口冲突错误
```bash
错误: "Address already in use (os error 98)"
```
**解决方案**:
- 检查端口 8080 是否被占用：`lsof -i :8080`
- 或修改 `CRS_SERVER__PORT` 环境变量

#### 2. Redis 连接失败
```bash
错误: "Connection refused"
```
**解决方案**:
```bash
# 检查 Redis 是否运行
redis-cli ping

# 或启动 Redis
docker run -d -p 6379:6379 redis:7-alpine
```

#### 3. 环境变量未设置
```bash
错误: "CRS_SECURITY__ENCRYPTION_KEY must be set"
```
**解决方案**:
```bash
# 确保 .env 文件存在且包含必填项
cp .env.example .env
# 编辑 .env 设置 ENCRYPTION_KEY (必须32字符)
```

#### 4. 前端无法连接后端
**解决方案**:
- 检查 `web/admin-spa/vite.config.js` 中的 `apiTarget`
- 默认应指向 `http://localhost:8080`
- 可通过环境变量覆盖：`VITE_API_TARGET=http://localhost:8080 npm run dev`

---

## 📊 性能对比

### 实际测试结果

基准测试环境: Intel Core i7, 16GB RAM, SSD

| 操作 | Node.js | Rust | 改进 |
|------|---------|------|------|
| **加密 (10KB)** | ~25 µs | ~20.6 µs | 1.2x 更快 |
| **解密 (10KB)** | ~12 µs | ~8.0 µs | 1.5x 更快 |
| **JSON 序列化** | ~80 ns | ~36.6 ns | 2.2x 更快 |
| **API 请求处理** | ~50 ms | ~18 ms | 2.8x 更快 |
| **内存占用** | 200 MB | 65 MB | 3x 减少 |

### 缓存性能

| 指标 | 性能 |
|------|------|
| 解密缓存命中 | **3.8x 加速** |
| 缓存命中率 | > 80% |
| LRU 缓存大小 | 500 条目 |

---

## 🔄 回退到 Node.js（紧急情况）

如需紧急回退到 Node.js 版本：

```bash
# 1. 从归档恢复 Node.js 代码
cp -r nodejs-archive/src ./
cp -r nodejs-archive/scripts ./
cp -r nodejs-archive/cli ./
cp nodejs-archive/package.json ./

# 2. 恢复环境变量格式
cp nodejs-archive/.env.example.nodejs .env

# 3. 安装依赖
npm install

# 4. 启动 Node.js 服务（端口 3000）
npm run dev

# 5. 更新前端代理（指向 3000）
cd web/admin-spa/
VITE_API_TARGET=http://localhost:3000 npm run dev
```

**注意**: 回退后需要重启所有依赖该服务的客户端。

---

## 📚 相关文档

- **Rust 实现**: `rust/README.md`
- **API 文档**: `docs/INTERFACE.md`
- **部署指南**: `rust/DEPLOYMENT_GUIDE.md`
- **安全审计**: `rust/SECURITY_AUDIT.md`
- **性能基准**: `rust/PHASE8.2_COMPLETE.md`
- **Node.js 归档**: `nodejs-archive/README.md`

---

## ✅ 迁移检查清单

### 开发环境

- [ ] 安装 Rust 1.75+
- [ ] 更新 `.env` 文件为 Rust 格式
- [ ] 更新客户端配置端口（3000 → 8080）
- [ ] 验证 Redis 连接
- [ ] 测试 Rust 后端启动
- [ ] 测试前端界面连接

### 生产环境

- [ ] 更新反向代理配置（端口 8080）
- [ ] 更新监控系统端口
- [ ] 更新部署脚本
- [ ] 更新环境变量（Rust 格式）
- [ ] 进行负载测试
- [ ] 准备回退方案

---

## 💡 最佳实践

1. **渐进式切换**: 先在测试环境验证，再逐步切换生产流量
2. **监控指标**: 密切关注内存、CPU、响应时间等关键指标
3. **保留 Node.js**: 短期内保留 `nodejs-archive/` 作为应急回退方案
4. **文档更新**: 及时更新内部文档和 API 文档中的端口信息
5. **团队培训**: 确保团队熟悉 Rust 代码维护和调试流程

---

**如有任何问题，请参考**:
- [GitHub Issues](https://github.com/your-repo/issues)
- [Rust 文档](https://doc.rust-lang.org/)
- [项目 Wiki](https://github.com/your-repo/wiki)

**迁移完成！** 🎉
