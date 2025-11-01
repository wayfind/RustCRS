# 🔄 从原始仓库迁移到 Fork 版本指南

## 背景

如果你已经通过原始仓库（Wei-Shaw/claude-relay-service）的 `crs install` 命令安装了服务，现在想要切换到你自己 fork 的版本（wayfind/claude-relay-service），本文档提供了完整的迁移步骤。

---

## 📋 迁移前准备

### 1. 备份重要数据

**必须备份的内容**:

```bash
# 进入安装根目录（默认是 ~/claude-relay-service）
cd ~/claude-relay-service

# 备份配置文件（配置文件在根目录）
cp .env .env.backup
cp config/config.js config/config.js.backup

# 备份管理员凭据
cp data/init.json data/init.json.backup

# 备份 Redis 数据（如果使用本地 Redis）
redis-cli SAVE  # 会保存到 Redis 数据目录
```

**可选备份**:
```bash
# 备份日志（如果需要）
cd ~/claude-relay-service
tar -czf logs_backup.tar.gz logs/

# 导出 Redis 数据（推荐）
cd ~/claude-relay-service/app  # app 目录包含 package.json
npm run data:export  # 会导出到 data/export/ 目录
```

**⚠️ 目录结构说明**（crs install 安装方式）:
```
~/claude-relay-service/          # 安装根目录
├── .env                          # 环境配置
├── config/                       # 配置目录
├── data/                         # 数据目录
├── logs/                         # 日志目录
└── app/                          # Git 仓库目录（实际代码）
    ├── src/
    ├── web/
    ├── package.json
    └── ...
```

---

## 🔄 迁移方案

### 方案 A: 原地更新（推荐）✅

**适用场景**: 保留现有配置和数据，只更新代码

#### 步骤 1: 停止服务

```bash
# 使用 crs 命令停止
crs stop

# 或使用 npm 脚本
cd ~/claude-relay-service
npm run service:stop
```

#### 步骤 2: 切换远程仓库

**⚠️ 重要提示**:
- 如果你通过 `crs install` 安装,实际 Git 仓库在 `~/claude-relay-service/app/` 目录
- 如果你通过 `git clone` 安装,实际 Git 仓库在 `~/claude-relay-service/` 目录

**检查你的安装目录**:
```bash
# 方法1: 检查 app 子目录是否是 Git 仓库
cd ~/claude-relay-service/app && git remote -v 2>/dev/null

# 方法2: 检查根目录是否是 Git 仓库
cd ~/claude-relay-service && git remote -v 2>/dev/null
```

**对于 crs install 安装方式**（推荐，大多数用户）:
```bash
cd ~/claude-relay-service/app  # 进入实际的 Git 仓库目录

# 查看当前远程仓库
git remote -v

# 应该显示：
# origin  https://github.com/Wei-Shaw/claude-relay-service.git (fetch)
# origin  https://github.com/Wei-Shaw/claude-relay-service.git (push)

# 修改远程仓库地址为你的 fork
git remote set-url origin https://github.com/wayfind/claude-relay-service.git

# 验证修改
git remote -v

# 应该显示：
# origin  https://github.com/wayfind/claude-relay-service.git (fetch)
# origin  https://github.com/wayfind/claude-relay-service.git (push)
```

**对于 git clone 安装方式**:
```bash
cd ~/claude-relay-service  # Git 仓库在根目录

# 查看和修改远程仓库（同上）
git remote set-url origin https://github.com/wayfind/claude-relay-service.git
```

#### 步骤 3: 拉取最新代码

**对于 crs install 安装方式** (你应该还在 `~/claude-relay-service/app/` 目录):
```bash
# 确保在 app 目录（Git 仓库所在位置）
cd ~/claude-relay-service/app

# 获取最新标签
git fetch --tags

# 查看可用版本
git tag -l | tail -10

# 切换到你的最新版本
git checkout v1.1.183

# 或切换到 main 分支获取最新代码
# git checkout main
# git pull origin main
```

#### 步骤 4: 更新依赖

```bash
# 确保在 app 目录
cd ~/claude-relay-service/app

# 更新后端依赖
npm install

# 更新前端依赖
cd web/admin-spa
npm install
cd ../..

# 重新构建前端
npm run build:web
```

#### 步骤 5: 重启服务

```bash
# 使用 crs 命令重启
crs restart

# 或使用 npm 脚本
npm run service:restart:daemon
```

#### 步骤 6: 验证

```bash
# 检查服务状态
crs status

# 查看日志
npm run service:logs

# 访问管理界面
# 浏览器打开: http://your-server:3000/admin-next/
```

---

### 方案 B: 使用 Docker 镜像（最简单）🐳

**适用场景**: 想要使用你最新发布的 Docker 镜像

#### 步骤 1: 停止现有服务

```bash
crs stop
```

#### 步骤 2: 拉取你的 Docker 镜像

```bash
# 拉取你 fork 仓库的最新镜像
docker pull ghcr.io/wayfind/claude-relay-service:v1.1.183

# 或拉取 latest
docker pull ghcr.io/wayfind/claude-relay-service:latest
```

#### 步骤 3: 修改 docker-compose.yml

如果使用 Docker Compose，修改镜像地址：

```yaml
services:
  claude-relay-service:
    # 修改前:
    # image: weishaw/claude-relay-service:latest

    # 修改后:
    image: ghcr.io/wayfind/claude-relay-service:v1.1.183

    # 或使用 latest:
    # image: ghcr.io/wayfind/claude-relay-service:latest

    ports:
      - "3000:3000"
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
      - ./.env:/app/.env
    environment:
      - NODE_ENV=production
```

#### 步骤 4: 重启服务

```bash
docker-compose down
docker-compose up -d
```

#### 步骤 5: 验证

```bash
docker-compose ps
docker-compose logs -f claude-relay-service
```

---

### 方案 C: 全新安装（最干净）🆕

**适用场景**: 想要重新安装，保持配置分离

#### 步骤 1: 备份数据（见上方"迁移前准备"）

#### 步骤 2: 卸载原有服务

```bash
# 使用 crs 命令卸载
crs uninstall

# 手动删除（如果 crs 不工作）
cd ~
rm -rf claude-relay-service  # 注意：这会删除所有数据！确保已备份
```

#### 步骤 3: 克隆你的 fork

```bash
# 克隆你的仓库
git clone https://github.com/wayfind/claude-relay-service.git
cd claude-relay-service

# 切换到特定版本
git checkout v1.1.183
```

#### 步骤 4: 恢复配置

```bash
# 恢复配置文件
cp /path/to/backup/.env .env
cp /path/to/backup/config.js config/config.js
cp /path/to/backup/init.json data/init.json

# 或者重新初始化
npm run setup
```

#### 步骤 5: 安装和启动

```bash
# 安装依赖
npm install
npm run install:web

# 构建前端
npm run build:web

# 启动服务
npm run service:start:daemon
```

#### 步骤 6: 恢复 Redis 数据（如果需要）

```bash
# 如果之前导出过数据
npm run data:import

# 或手动恢复 Redis dump.rdb 文件
```

---

## 🔍 验证迁移成功

### 1. 检查版本信息

```bash
# 查看 Git 版本
cd ~/claude-relay-service
git describe --tags
# 应该显示: v1.1.183

# 查看 VERSION 文件
cat VERSION
# 应该显示: 1.1.183
```

### 2. 检查远程仓库

```bash
git remote -v
# 应该显示你的 fork 仓库:
# origin  https://github.com/wayfind/claude-relay-service.git
```

### 3. 检查服务运行

```bash
# 检查服务状态
crs status
# 或
npm run service:status

# 检查 API 健康状态
curl http://localhost:3000/health
```

### 4. 检查 Web 界面

访问: http://your-server:3000/admin-next/

- 确认可以登录
- 检查账户数据是否完整
- 检查 API Key 是否可用

---

## 📊 迁移对比

| 方案 | 难度 | 数据迁移 | 服务中断时间 | 推荐场景 |
|------|------|---------|------------|---------|
| **方案 A: 原地更新** | ⭐⭐ | 自动保留 | ~5 分钟 | 大多数情况 |
| **方案 B: Docker 镜像** | ⭐ | 需要挂载 | ~2 分钟 | 喜欢 Docker |
| **方案 C: 全新安装** | ⭐⭐⭐ | 需要手动 | ~10 分钟 | 想要干净安装 |

---

## 🔄 后续更新

迁移到你的 fork 后，以后更新非常简单：

### 方法 1: 使用 Git 更新

```bash
cd ~/claude-relay-service

# 停止服务
crs stop

# 拉取最新代码
git fetch --tags
git checkout v1.1.184  # 新版本

# 更新依赖
npm install
npm run build:web

# 重启服务
crs restart
```

### 方法 2: 使用 Docker 更新

```bash
# 拉取新镜像
docker pull ghcr.io/wayfind/claude-relay-service:latest

# 重启服务
docker-compose down
docker-compose up -d
```

### 方法 3: 使用 `crs update` 命令 ✨ **推荐**

**⚠️ 重要**: 完成迁移步骤 2 (切换 Git remote) 后,`crs update` **已经会从你的 fork 更新**!

```bash
# ✅ 迁移完成后,直接使用即可:
crs update
```

**工作原理**:
- `crs` 命令实际上是软链接到 `~/claude-relay-service/app/scripts/manage.sh`
- `crs update` 内部使用 `git fetch origin` 拉取更新
- 因为你在步骤 2 已经修改了 `origin` 指向你的 fork
- 所以 `crs update` 自动从你的 fork 仓库获取更新 🎉

**验证配置**:
```bash
# 进入 app 目录查看 remote 配置
cd ~/claude-relay-service/app
git remote -v

# 应该显示:
# origin  https://github.com/wayfind/claude-relay-service.git (fetch)
# origin  https://github.com/wayfind/claude-relay-service.git (push)
```

**如果 remote 还是原仓库,需要重新执行步骤 2**:
```bash
cd ~/claude-relay-service/app
git remote set-url origin https://github.com/wayfind/claude-relay-service.git
```

---

## ❓ 常见问题

### Q1: 迁移会丢失数据吗？

**A**: 不会！使用**方案 A（原地更新）**，所有配置和 Redis 数据都会保留。只是代码更新到你的版本。

### Q2: 迁移后原来的 API Key 还能用吗？

**A**: 可以！API Key 存储在 Redis 中，只要 Redis 数据没有清空，所有 API Key 都会继续有效。

### Q3: 如何回滚到原始版本？

**A**: 只需切换回原始仓库：

```bash
cd ~/claude-relay-service
git remote set-url origin https://github.com/Wei-Shaw/claude-relay-service.git
git fetch --tags
git checkout <原来的版本号>
npm install
npm run build:web
crs restart
```

### Q4: Docker Compose 如何保留数据？

**A**: 使用 volumes 挂载：

```yaml
volumes:
  - ./data:/app/data        # 持久化数据目录
  - ./logs:/app/logs        # 日志目录
  - ./.env:/app/.env        # 环境配置
  - ./config:/app/config    # 配置文件（可选）
```

这样即使容器删除，数据也不会丢失。

### Q5: 我有多个服务器，需要每台都迁移吗？

**A**: 是的。每台服务器上的安装都需要单独迁移。不过可以使用脚本批量操作：

```bash
#!/bin/bash
# migrate-all.sh

SERVERS=("server1.example.com" "server2.example.com" "server3.example.com")

for server in "${SERVERS[@]}"; do
  echo "Migrating $server..."
  ssh $server << 'EOF'
    cd ~/claude-relay-service
    crs stop
    git remote set-url origin https://github.com/wayfind/claude-relay-service.git
    git fetch --tags
    git checkout v1.1.183
    npm install
    npm run build:web
    crs restart
EOF
done
```

### Q6: 迁移后如何跟踪原始仓库的更新？

**A**: 可以添加原始仓库为 upstream：

```bash
cd ~/claude-relay-service

# 添加原始仓库为 upstream
git remote add upstream https://github.com/Wei-Shaw/claude-relay-service.git

# 查看远程仓库
git remote -v
# origin    https://github.com/wayfind/claude-relay-service.git (fetch)
# origin    https://github.com/wayfind/claude-relay-service.git (push)
# upstream  https://github.com/Wei-Shaw/claude-relay-service.git (fetch)
# upstream  https://github.com/Wei-Shaw/claude-relay-service.git (push)

# 获取 upstream 的更新
git fetch upstream

# 合并 upstream 的更新到你的 fork
git merge upstream/main
```

---

## 🎯 推荐迁移流程

对于大多数用户（**crs install 安装方式**），推荐使用 **方案 A（原地更新）**:

### 完整命令序列（复制粘贴执行）

```bash
# ========================================
# 1. 备份（安全第一）
# ========================================
cd ~/claude-relay-service
cp .env .env.backup 2>/dev/null || echo ".env not found"
cp config/config.js config/config.js.backup 2>/dev/null || echo "config.js not found"
cp data/init.json data/init.json.backup 2>/dev/null || echo "init.json not found"

# 导出 Redis 数据（可选但推荐）
cd ~/claude-relay-service/app
npm run data:export 2>/dev/null || echo "Data export skipped"

# ========================================
# 2. 停止服务
# ========================================
crs stop

# ========================================
# 3. 切换到你的 fork (进入 app 子目录)
# ========================================
cd ~/claude-relay-service/app  # ⚠️ 重要：Git 仓库在 app 目录
git remote set-url origin https://github.com/wayfind/claude-relay-service.git
git fetch --tags
git checkout v1.1.183

# ========================================
# 4. 更新依赖和构建
# ========================================
npm install
npm run build:web

# ========================================
# 5. 重启服务
# ========================================
crs restart

# ========================================
# 6. 验证
# ========================================
crs status
curl http://localhost:3000/health
```

### 简化版（如果你熟悉流程）

```bash
# 备份、停止、切换、更新、重启、验证
cd ~/claude-relay-service && \
crs stop && \
cd app && \
git remote set-url origin https://github.com/wayfind/claude-relay-service.git && \
git fetch --tags && \
git checkout v1.1.183 && \
npm install && \
npm run build:web && \
crs restart && \
crs status
```

---

## 📞 需要帮助？

如果迁移过程中遇到问题：

1. **检查日志**: `npm run service:logs` 或 `docker-compose logs`
2. **查看服务状态**: `crs status`
3. **验证配置**: 确认 `.env` 和 `config/config.js` 正确
4. **Redis 连接**: 确认 Redis 服务运行正常

如果问题仍然存在，提供以下信息：
- 使用的迁移方案
- 错误日志
- 系统信息（OS, Node.js 版本）
- 部署方式（手动 / Docker）
