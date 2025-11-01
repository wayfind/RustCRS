# 贡献指南

欢迎为 Claude Relay Service 项目做出贡献！本文档提供完整的开发、测试、发布流程说明。

## 📋 目录

- [开发环境设置](#开发环境设置)
- [开发流程](#开发流程)
- [代码规范](#代码规范)
- [测试指南](#测试指南)
- [版本发布流程](#版本发布流程)
- [CI/CD 自动化](#cicd-自动化)
- [Docker 镜像发布](#docker-镜像发布)
- [通知配置](#通知配置)
- [Fork 仓库配置](#fork-仓库配置)
- [故障排除](#故障排除)

---

## 开发环境设置

### 前置要求

- **Node.js**: 18+ (当前版本)
- **Rust**: 1.75+ (新版本开发中)
- **Redis**: 6+
- **Docker**: 可选
- **Git**: 用于版本控制

### 基本配置

```bash
# 1. 克隆项目
git clone https://github.com/your-username/claude-relay-service.git
cd claude-relay-service

# 2. 安装依赖
npm install

# 3. 配置环境
cp config/config.example.js config/config.js
cp .env.example .env
# 编辑 .env 设置必要的环境变量

# 4. 初始化
npm run setup  # 生成管理员凭据和密钥

# 5. 启动开发服务器
npm run dev
```

### 必需的环境变量

```bash
JWT_SECRET=<32字符以上随机字符串>
ENCRYPTION_KEY=<32字符固定长度>
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=<可选>
```

---

## 开发流程

### 分支策略

- `main`: 主分支，所有生产就绪的代码
- `dev`: 开发分支（如果需要）
- `feature/*`: 功能开发分支
- `fix/*`: Bug 修复分支
- `refactor/*`: 重构分支

### 提交规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范：

```bash
# 新功能
git commit -m "feat: 添加 Gemini 账户支持"

# Bug 修复
git commit -m "fix: 修复 OAuth token 刷新逻辑"

# 文档更新
git commit -m "docs: 更新部署指南"

# 代码重构
git commit -m "refactor: 优化统一调度器逻辑"

# 性能优化
git commit -m "perf: 优化 Redis 查询性能"

# 其他变更
git commit -m "chore: 更新依赖版本"
```

### 常规开发工作流

```bash
# 1. 创建功能分支
git checkout -b feature/new-feature

# 2. 进行开发
# ... 编写代码 ...

# 3. 运行测试和检查
npm run lint
npm test

# 4. 提交变更
git add .
git commit -m "feat: 添加新功能"

# 5. 推送到远程
git push origin feature/new-feature

# 6. 创建 Pull Request（可选）

# 7. 合并到 main 分支
git checkout main
git merge feature/new-feature
git push origin main  # 这会触发自动发布流程
```

---

## 代码规范

### JavaScript/Node.js 代码

- 使用 **ESLint** 进行代码检查
- 遵循项目现有的代码风格
- 使用 `async/await` 而非回调函数
- 适当添加注释说明复杂逻辑
- 敏感数据必须加密存储

### Rust 代码（开发中）

```bash
cd rust/

# 代码格式化
cargo fmt

# 代码检查
cargo clippy

# 运行测试
cargo test

# 构建
cargo build --release
```

### 命名规范

- **文件名**: kebab-case（如 `api-key-service.js`）
- **函数名**: camelCase（如 `validateApiKey`）
- **类名**: PascalCase（如 `UnifiedScheduler`）
- **常量**: UPPER_SNAKE_CASE（如 `MAX_RETRIES`）

---

## 测试指南

### 运行测试

```bash
# 运行所有测试
npm test

# 运行特定测试文件
npm test -- src/services/apiKeyService.test.js

# 运行测试覆盖率
npm run test:coverage
```

### 编写测试

```javascript
// 示例测试文件
const { validateApiKey } = require('../services/apiKeyService');

describe('API Key Service', () => {
  test('should validate correct API key', async () => {
    const result = await validateApiKey('cr_test123');
    expect(result.valid).toBe(true);
  });

  test('should reject invalid API key', async () => {
    const result = await validateApiKey('invalid_key');
    expect(result.valid).toBe(false);
  });
});
```

---

## 版本发布流程

### 自动版本发布

本项目采用**全自动化**的版本管理和发布流程。

#### 工作原理

1. **代码推送**: 推送代码到 `main` 分支
2. **自动版本递增**:
   - 检测代码变更（排除纯文档更新）
   - 自动递增 patch 版本号（如 v1.0.1 → v1.0.2）
3. **自动发布**:
   - 创建 Git tag
   - 生成 Changelog
   - 创建 GitHub Release
   - 构建 Docker 镜像
   - 发送通知（可选）

#### 版本递增规则

- **版本格式**: `v<major>.<minor>.<patch>` (如 v1.0.2)
- **自动递增**: 每次推送到 main 分支，自动递增 patch 版本
- **触发条件**:
  - 推送到 `main` 分支
  - 有实际代码变更（不包括 .md 文件、docs/ 目录等）
  - 自上次发布以来有新提交

#### 跳过自动发布

在 commit 消息中添加 `[skip ci]`：

```bash
git commit -m "docs: 更新文档 [skip ci]"
git push origin main  # 不会触发自动发布
```

#### 手动控制版本号

如果需要发布大版本或中版本更新：

```bash
# 大版本更新 (1.0.x → 2.0.0)
git tag -a v2.0.0 -m "Major release v2.0.0"
git push origin v2.0.0

# 中版本更新 (1.0.x → 1.1.0)
git tag -a v1.1.0 -m "Minor release v1.1.0"
git push origin v1.1.0
```

### Changelog 生成

使用 [git-cliff](https://github.com/orhun/git-cliff) 自动生成更新日志：

- **配置文件**: `.github/cliff.toml`
- **提交规范**: 遵循 Conventional Commits
  - `feat:` 新功能
  - `fix:` Bug 修复
  - `docs:` 文档更新
  - `chore:` 其他变更
  - `refactor:` 代码重构
  - `perf:` 性能优化

### 查看发布历史

1. **GitHub Releases 页面**: `https://github.com/<owner>/<repo>/releases`
2. **CHANGELOG.md**: 项目根目录的完整版本历史
3. **Git 命令**:
   ```bash
   # 查看最新标签
   git describe --tags --abbrev=0

   # 查看所有标签
   git tag -l
   ```

---

## CI/CD 自动化

### GitHub Actions 工作流

#### 1. 自动发布流程 (`auto-release-pipeline.yml`)

**功能**:
- 自动检测代码变更并更新版本号
- 生成 Changelog
- 构建前端并推送到 `web-dist` 分支
- 构建多平台 Docker 镜像
- 创建 GitHub Release
- 发送 Telegram 通知

**触发条件**:
- 推送到 `main` 分支（自动触发）
- 手动触发（GitHub Actions 页面）

#### 2. Docker 构建 (`docker-publish.yml`)

**功能**:
- 构建多平台镜像（amd64, arm64）
- 推送到 Docker Hub 和 GitHub Container Registry
- 安全漏洞扫描（Trivy）
- 更新 Docker Hub 描述

**触发条件**:
- 推送到 `main` 分支
- 创建版本标签（如 `v1.0.0`）
- Pull Request（仅构建，不推送）

#### 3. PR 检查 (`pr-lint-check.yml`)

**功能**:
- 检查提交消息格式
- 运行代码检查（ESLint）
- 运行测试

**触发条件**:
- 创建或更新 Pull Request

### 手动触发构建

1. 访问仓库的 **Actions** 页面
2. 选择工作流（如 "Auto Release Pipeline"）
3. 点击 **Run workflow**
4. 选择分支并运行

### 查看构建状态

- **Actions 页面**: 查看所有工作流运行历史和日志
- **README 徽章**: 实时显示构建状态
- **Docker Hub**: 查看镜像标签和拉取次数

---

## Docker 镜像发布

### 配置 Docker Hub 发布

#### 1. 创建 Docker Hub Access Token

1. 登录 [Docker Hub](https://hub.docker.com/)
2. Account Settings → Security → Access Tokens
3. 点击 **New Access Token**
4. 填写描述（如 `GitHub Actions`）
5. 选择权限：**Read, Write, Delete**
6. 生成并**立即复制** token

#### 2. 配置 GitHub Secrets

1. 进入仓库 → **Settings** → **Secrets and variables** → **Actions**
2. 点击 **New repository secret**
3. 添加以下 secrets：

| Secret 名称 | 说明 | 示例值 |
|------------|------|--------|
| `DOCKERHUB_USERNAME` | Docker Hub 用户名 | `myusername` |
| `DOCKERHUB_TOKEN` | Docker Hub Access Token | `dckr_pat_xxx...` |

### 镜像标签策略

每次发布会创建以下标签：

- `latest`: 始终指向最新版本
- `main`: main 分支的最新构建
- `v1.0.0`: 完整版本号
- `1.0`: 主次版本
- `1`: 主版本
- `main-sha-xxxxxxx`: 包含 commit SHA 的标签

### 使用发布的镜像

```bash
# Docker Hub（需配置 secrets）
docker pull <your-dockerhub-username>/claude-relay-service:latest
docker pull <your-dockerhub-username>/claude-relay-service:v1.0.0

# GitHub Container Registry（始终可用）
docker pull ghcr.io/<your-github-username>/claude-relay-service:latest
docker pull ghcr.io/<your-github-username>/claude-relay-service:v1.0.0

# 运行容器
docker run -d \
  --name claude-relay \
  -p 3000:3000 \
  -v ./data:/app/data \
  -v ./logs:/app/logs \
  <your-username>/claude-relay-service:latest
```

### 支持的平台

- `linux/amd64`: Intel/AMD 架构
- `linux/arm64`: ARM64 架构（Apple Silicon, 树莓派等）

---

## 通知配置

### Telegram 通知设置

当 GitHub Actions 自动发布新版本时，可以发送通知到 Telegram 频道。

#### 1. 创建 Telegram Bot

1. 在 Telegram 中找到 [@BotFather](https://t.me/botfather)
2. 发送 `/newbot` 命令
3. 按提示设置 Bot 名称和用户名
4. **保存 Bot Token**（格式：`1234567890:ABCdefGHIjklMNOpqrsTUVwxyz`）

#### 2. 创建或选择频道

1. 创建新频道或使用现有频道
2. 将 Bot 添加为频道管理员
3. 赋予发送消息权限

#### 3. 获取频道 Chat ID

**方法 1: Web Telegram**
1. 打开 https://web.telegram.org
2. 进入你的频道
3. 查看 URL：`https://web.telegram.org/k/#-1234567890`
4. Chat ID 是 `#` 后的数字（包括负号）：`-1234567890`

**方法 2: Bot API**
1. 在频道发送一条消息
2. 访问：`https://api.telegram.org/bot<YOUR_BOT_TOKEN>/getUpdates`
3. 查看 `chat.id` 字段

**方法 3: 公开频道**
可直接使用 `@频道用户名` 作为 Chat ID

#### 4. 添加 GitHub Secrets

添加以下两个 secrets：

| Secret 名称 | 说明 | 示例值 |
|------------|------|--------|
| `TELEGRAM_BOT_TOKEN` | Bot Token | `1234567890:ABCdefGHIjklMNOpqrsTUVwxyz` |
| `TELEGRAM_CHAT_ID` | 频道 Chat ID | `-1234567890` 或 `@your_channel` |

#### 通知消息示例

```
🚀 Claude Relay Service 新版本发布！

📦 版本号: 1.1.3

📝 更新内容:
- feat: 添加 Telegram 自动通知功能
- fix: 修复某个问题

🐳 Docker 部署:
docker pull username/claude-relay-service:v1.1.3

🔗 相关链接:
• GitHub Release
• 完整更新日志
• Docker Hub

#ClaudeRelay #Update #v1_1_3
```

---

## Fork 仓库配置

### 快速开始

如果你 fork 了这个项目，GitHub Actions 工作流会**自动适应你的仓库**，无需手动修改代码！

#### 自动适配的配置

| 配置项 | 自动适配行为 |
|-------|------------|
| **Docker Hub 镜像** | `$DOCKERHUB_USERNAME/claude-relay-service` |
| **GitHub Container Registry** | `ghcr.io/${{ github.repository_owner }}/claude-relay-service` |
| **GitHub Release** | 自动在你的仓库中创建 |
| **Changelog 链接** | 自动使用 `${{ github.repository }}` |
| **Issue 链接** | 自动指向你的仓库 |
| **前端构建分支** | 自动推送到你仓库的 `web-dist` 分支 |

### Fork 后的配置选项

#### 方式 A: 使用 Docker Hub（推荐）

配置以下 secrets 以推送到 Docker Hub：
- `DOCKERHUB_USERNAME`
- `DOCKERHUB_TOKEN`

#### 方式 B: 仅使用 GitHub Container Registry

**无需配置任何 secrets**！镜像会自动推送到：
```
ghcr.io/<your-username>/claude-relay-service
```

### 验证配置

```bash
# 推送代码测试
git add .
git commit -m "feat: test auto-release pipeline"
git push origin main

# 检查 Actions 页面
# 查看日志中的镜像名称是否正确
```

---

## 故障排除

### 版本发布问题

**问题**: 版本没有自动更新

**解决方案**:
1. 检查是否有实质性代码变更（非文档）
2. 查看 GitHub Actions 日志
3. 确认推送到 `main` 分支
4. 检查是否在 commit 消息中使用了 `[skip ci]`

**问题**: 需要手动触发发布

**解决方案**:
```bash
# 直接修改 VERSION 文件
echo "1.1.10" > VERSION
git add VERSION
git commit -m "chore: bump version to 1.1.10"
git push origin main
```

### Docker 构建问题

**问题**: Docker 构建失败

**解决方案**:
1. 检查 Docker Hub secrets 是否正确配置
2. 确认 token 权限足够（Read, Write, Delete）
3. 查看 Actions 日志详细错误
4. 本地测试：`docker build -t test .`

**问题**: 镜像推送失败

**解决方案**:
1. 确认 Docker Hub 用户名正确
2. Token 可能过期，重新生成
3. 检查是否达到免费账户限制

### Telegram 通知问题

**问题**: 通知发送失败

**解决方案**:
1. 检查 Bot Token 是否正确
2. 确认 Bot 已添加为频道管理员
3. 验证 Chat ID 格式（注意负号）
4. 检查 GitHub Secrets 配置

**注意**: 通知失败不会影响版本发布（配置了 `continue-on-error: true`）

### 代码检查问题

**问题**: ESLint 检查失败

**解决方案**:
```bash
# 运行 lint 检查
npm run lint

# 自动修复问题
npm run lint -- --fix
```

**问题**: 测试失败

**解决方案**:
```bash
# 运行测试
npm test

# 查看详细日志
npm test -- --verbose
```

### Redis 连接问题

**问题**: Redis 连接失败

**解决方案**:
1. 确认 Redis 服务运行：`redis-cli ping`
2. 检查环境变量：`REDIS_HOST`, `REDIS_PORT`, `REDIS_PASSWORD`
3. 查看日志：`logs/claude-relay-*.log`

---

## 常见问题

### Q: 如何回滚到之前的版本？

```bash
# 使用 Docker 特定版本
docker pull <username>/claude-relay-service:v1.0.0

# 或在 docker-compose.yml 中指定版本
image: <username>/claude-relay-service:v1.0.0
```

### Q: 如何跳过某次构建？

在 commit 消息中添加 `[skip ci]`：
```bash
git commit -m "docs: 更新文档 [skip ci]"
```

### Q: 可以发送到多个 Telegram 频道吗？

可以修改工作流，添加多个通知步骤，或使用逗号分隔多个 Chat ID。

### Q: 如何修改版本递增规则？

编辑 `.github/workflows/auto-release-pipeline.yml` 中的版本计算逻辑：

```yaml
# 当前是递增 patch 版本
NEW_PATCH=$((PATCH + 1))

# 改为递增 minor 版本
NEW_MINOR=$((MINOR + 1))
NEW_PATCH=0
```

### Q: 如何查看当前版本？

```bash
# 查看 VERSION 文件
cat VERSION

# 查看最新 Git tag
git describe --tags --abbrev=0

# 查看所有 tags
git tag -l
```

---

## 安全提示

- **永远不要**在代码中直接写入敏感信息
- 始终使用 GitHub Secrets 存储 tokens 和密钥
- 定期更换 API tokens 和密码
- 检查 `.gitignore` 确保敏感文件不会被提交
- 使用环境变量配置敏感信息

---

## 相关资源

### 官方文档

- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [Docker 官方文档](https://docs.docker.com/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Git Cliff](https://git-cliff.org/docs/)

### 项目文档

- [架构设计](./ARCHITECTURE.md) - Rust 系统架构
- [部署指南](./DEPLOYMENT.md) - 详细部署说明（待创建）
- [配置参考](./CONFIGURATION.md) - 完整配置选项（待创建）
- [重构进度](../REFACTORING_STATUS.md) - Rust 重写进度

### 工具链接

- [Node.js](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/)
- [Redis](https://redis.io/)
- [Docker](https://www.docker.com/)

---

## 获取帮助

- **Issues**: [GitHub Issues](https://github.com/your-username/claude-relay-service/issues)
- **Discussions**: GitHub Discussions（如果启用）
- **文档**: [docs/](../docs/)

---

**感谢你的贡献！** 🎉

如果这是你第一次为开源项目做贡献，欢迎阅读 [GitHub's guide to contributing](https://docs.github.com/en/get-started/quickstart/contributing-to-projects)。
