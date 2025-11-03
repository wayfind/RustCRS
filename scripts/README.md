# 启动脚本使用说明

## 概述

`start-all.sh` 脚本支持**交互模式**和**非交互模式**，适用于开发、CI/CD、自动化测试等多种场景。

## 使用方式

### 1. 交互模式（默认）

在交互式终端中运行，脚本会询问启动方式：

```bash
# 方式1: 通过 make
make rust-dev

# 方式2: 直接运行脚本
bash scripts/start-all.sh dev
```

脚本会提示选择：
- 后端启动方式：前台运行 / 后台运行
- 是否启动前端界面

### 2. 非交互模式（自动化）

在非交互环境（CI、后台任务、Claude Code等）中运行，脚本自动使用默认配置：

```bash
# 非交互模式（标准输入重定向）
bash scripts/start-all.sh dev < /dev/null

# 或通过管道
echo "" | bash scripts/start-all.sh dev
```

**默认行为**:
- ✅ 后端：后台运行（daemon模式）
- ❌ 前端：不启动（因为前端需要前台运行）

### 3. 环境变量控制（推荐用于自动化）

通过环境变量精确控制启动行为，优先级最高：

#### 控制后端启动模式

```bash
# 后台运行（推荐用于CI/自动化）
BACKEND_MODE=background bash scripts/start-all.sh dev

# 前台运行（推荐用于开发/调试）
BACKEND_MODE=foreground bash scripts/start-all.sh dev

# 也可以使用数字
BACKEND_MODE=1 bash scripts/start-all.sh dev  # 前台
BACKEND_MODE=2 bash scripts/start-all.sh dev  # 后台
```

#### 控制前端启动

```bash
# 启动前端（仅在后台模式有效）
START_FRONTEND=yes bash scripts/start-all.sh dev

# 不启动前端
START_FRONTEND=no bash scripts/start-all.sh dev
```

#### 组合使用

```bash
# 完全自动化：后台运行，不启动前端
BACKEND_MODE=background START_FRONTEND=no bash scripts/start-all.sh dev

# 完全自动化：后台运行，自动启动前端
BACKEND_MODE=background START_FRONTEND=yes bash scripts/start-all.sh dev
```

## 使用场景

### 开发环境

```bash
# 前台运行，便于查看日志
BACKEND_MODE=foreground bash scripts/start-all.sh dev

# 或者使用交互模式手动选择
make rust-dev
```

### CI/CD Pipeline

```bash
# Makefile target
make rust-dev < /dev/null

# 或使用环境变量
BACKEND_MODE=background START_FRONTEND=no bash scripts/start-all.sh dev
```

### Claude Code / 自动化工具

```bash
# 自动后台启动，无需人工干预
bash scripts/start-all.sh dev < /dev/null
```

### Docker / 容器环境

```bash
# 在 Dockerfile 或 docker-compose.yml 中
ENV BACKEND_MODE=foreground
ENV START_FRONTEND=no
CMD ["bash", "scripts/start-all.sh", "release"]
```

## 决策优先级

脚本按以下优先级决定行为：

1. **环境变量** (`BACKEND_MODE`, `START_FRONTEND`)
2. **交互输入** (如果是交互式终端)
3. **默认值** (非交互模式：后台运行，不启动前端)

## 日志和监控

### 后台模式日志

```bash
# 实时查看后端日志
tail -f logs/backend.log

# 查看最近的日志
tail -100 logs/backend.log
```

### 停止服务

```bash
# 停止后端进程
pkill -f claude-relay

# 或使用 PID（脚本启动时会显示）
kill <PID>
```

## 故障排查

### 问题：脚本在CI中卡住

**原因**: 脚本在等待交互输入

**解决方案**:
```bash
# 使用非交互模式
bash scripts/start-all.sh dev < /dev/null

# 或设置环境变量
BACKEND_MODE=background bash scripts/start-all.sh dev
```

### 问题：logs/backend.log 不存在

**原因**: logs 目录未创建

**解决方案**:
```bash
mkdir -p logs
```

### 问题：后端启动失败

**检查**:
1. Redis 是否运行: `docker ps | grep redis`
2. 环境变量是否配置: `cat .env`
3. 端口是否被占用: `lsof -i :8080`
4. 查看详细日志: `tail -f logs/backend.log`

## 高级配置

### 自定义启动脚本

如需更复杂的启动逻辑，可以创建自定义脚本：

```bash
#!/bin/bash
# custom-start.sh

export BACKEND_MODE=background
export START_FRONTEND=no

# 启动服务
bash scripts/start-all.sh release

# 等待服务就绪
sleep 5

# 运行健康检查
curl -f http://localhost:8080/health || exit 1

echo "✅ 服务启动成功"
```

### Makefile 集成

在 `Makefile` 中添加自定义 target：

```makefile
.PHONY: rust-background
rust-background:
	BACKEND_MODE=background START_FRONTEND=no bash scripts/start-all.sh dev

.PHONY: rust-foreground
rust-foreground:
	BACKEND_MODE=foreground bash scripts/start-all.sh dev
```

## 更新历史

- **2025-11-02**: 添加非交互模式支持和环境变量控制 (ISSUE-010修复)
- **2025-01-11**: 初始版本，仅支持交互模式
