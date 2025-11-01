# 脚本工具集

本目录包含 Claude Relay Service 的各类管理和维护脚本。

## 📁 目录结构

```
scripts/
├── setup/          # 初始化和安装脚本
├── deployment/     # 部署和服务管理脚本
├── maintenance/    # 维护和数据修复脚本
├── data/          # 数据管理和迁移脚本
└── monitoring/    # 监控和日志分析脚本
```

---

## 🚀 Setup（初始化脚本）

### `setup/setup.js`
**用途**: 项目初始化脚本

**功能**:
- 生成 `JWT_SECRET` 和 `ENCRYPTION_KEY`
- 创建管理员账户
- 生成初始化凭据文件 `data/init.json`

**使用**:
```bash
npm run setup
# 或
node scripts/setup/setup.js
```

### `setup/generate-self-signed-cert.js` / `.sh`
**用途**: 生成自签名 SSL 证书（开发环境）

**功能**:
- 为 HTTPS 开发环境生成自签名证书
- 创建 `certs/cert.pem` 和 `certs/key.pem`

**使用**:
```bash
# Node.js 版本（跨平台）
node scripts/setup/generate-self-signed-cert.js

# Shell 版本（Linux/macOS）
bash scripts/setup/generate-self-signed-cert.sh
```

**注意**: 生产环境请使用正规 CA 签发的证书或反向代理处理 SSL。

---

## 📦 Deployment（部署管理）

### `deployment/manage.sh`
**用途**: 服务管理脚本（推荐方式）

**功能**:
- 安装、启动、停止、重启服务
- 查看日志和状态
- 更新服务到最新版本

**使用**:
```bash
# 安装服务（生成 crs 命令）
bash scripts/deployment/manage.sh install

# 之后可以使用 crs 命令
crs start     # 启动服务
crs stop      # 停止服务
crs restart   # 重启服务
crs status    # 查看状态
crs logs      # 查看日志
crs update    # 更新服务
```

### `deployment/manage.js`
**用途**: Node.js 服务管理脚本

**功能**:
- PM2 进程管理
- 服务启动、停止、重启
- 日志管理

**使用**:
```bash
node scripts/deployment/manage.js start
node scripts/deployment/manage.js stop
node scripts/deployment/manage.js restart
node scripts/deployment/manage.js logs
```

### `deployment/check-deployment-status.sh`
**用途**: 检查部署状态

**功能**:
- 验证服务是否正常运行
- 检查健康检查端点
- 验证关键功能

**使用**:
```bash
bash scripts/deployment/check-deployment-status.sh
```

---

## 🔧 Maintenance（维护脚本）

### `maintenance/migrate-apikey-expiry.js`
**用途**: API Key 过期时间数据迁移

**功能**:
- 迁移旧版本 API Key 数据到新格式
- 支持干跑模式（预览变更）

**使用**:
```bash
# 干跑模式（不实际修改数据）
npm run migrate:apikey-expiry:dry

# 实际执行迁移
npm run migrate:apikey-expiry
```

### `maintenance/fix-usage-stats.js`
**用途**: 修复使用统计数据

**功能**:
- 修复损坏的使用统计
- 重新计算成本数据
- 数据一致性检查

**使用**:
```bash
npm run migrate:fix-usage-stats
```

### `maintenance/update-model-pricing.js`
**用途**: 更新模型价格

**功能**:
- 更新模型定价数据到 Redis
- 同步最新价格信息

**使用**:
```bash
npm run update:pricing
```

### `maintenance/manage-session-windows.js`
**用途**: 管理会话窗口

**功能**:
- 清理过期会话
- 查看活跃会话
- 手动管理会话状态

**使用**:
```bash
node scripts/maintenance/manage-session-windows.js
```

### `maintenance/fix-inquirer.js`
**用途**: 修复 Inquirer 库问题（临时脚本）

**状态**: 可能已过时，考虑删除

---

## 💾 Data（数据管理）

### `data/data-transfer.js`
**用途**: 基础数据导入导出

**功能**:
- 导出 Redis 数据到 JSON
- 导入数据到 Redis
- 基础数据备份

**使用**:
```bash
# 导出数据
npm run data:export

# 导入数据
npm run data:import

# 导出脱敏数据
npm run data:export:sanitized
```

### `data/data-transfer-enhanced.js`
**用途**: 增强型数据导入导出

**功能**:
- 支持加密数据导出
- 解密数据导入
- 完整性验证
- 增量备份

**使用**:
```bash
# 增强型导出（含解密）
npm run data:export:enhanced

# 导出加密数据
npm run data:export:encrypted

# 增强型导入
npm run data:import:enhanced
```

### `data/debug-redis-keys.js`
**用途**: 调试 Redis 键值

**功能**:
- 列出所有 Redis 键
- 查看键的类型和内容
- 统计键的数量

**使用**:
```bash
npm run data:debug
```

### `data/check-redis-keys.js`
**用途**: 检查 Redis 键状态

**功能**:
- 验证数据完整性
- 检查过期键
- 数据一致性验证

**使用**:
```bash
node scripts/data/check-redis-keys.js
```

---

## 📊 Monitoring（监控脚本）

### `monitoring/status-unified.sh`
**用途**: 统一状态查看

**功能**:
- 系统概览
- 服务状态
- Redis 连接状态
- 基础统计信息

**使用**:
```bash
npm run status
# 或
bash scripts/monitoring/status-unified.sh
```

### `monitoring/monitor-enhanced.sh`
**用途**: 增强监控脚本

**功能**:
- 详细系统指标
- 实时资源使用
- 性能监控
- 错误日志监控

**使用**:
```bash
npm run monitor
# 或
bash scripts/monitoring/monitor-enhanced.sh
```

### `monitoring/analyze-log-sessions.js`
**用途**: 分析日志中的会话数据

**功能**:
- 会话统计
- 错误率分析
- 性能指标提取

**使用**:
```bash
node scripts/monitoring/analyze-log-sessions.js
```

---

## 📋 常用脚本快速参考

### 首次部署
```bash
npm run setup                        # 初始化项目
npm run install:web                  # 安装前端
npm run build:web                    # 构建前端
npm run service:start:daemon         # 启动服务
```

### 日常维护
```bash
npm run status                       # 查看状态
npm run monitor                      # 监控服务
npm run data:export                  # 备份数据
npm run update:pricing               # 更新价格
```

### 数据迁移
```bash
npm run migrate:apikey-expiry        # API Key 迁移
npm run migrate:fix-usage-stats      # 修复统计
```

### 故障排除
```bash
npm run data:debug                   # 调试 Redis
npm run service:stop                 # 停止服务
npm run service:start                # 启动服务
bash scripts/deployment/check-deployment-status.sh  # 检查状态
```

---

## ⚠️ 注意事项

### 数据安全
- 导出数据时注意保护敏感信息
- 使用 `data:export:sanitized` 导出脱敏数据
- 备份文件应妥善保管，避免泄露

### 生产环境
- 在生产环境运行脚本前，先在测试环境验证
- 数据迁移脚本建议先用干跑模式测试
- 重要操作前先备份数据

### 权限要求
- 某些脚本需要 root 权限（如 `manage.sh install`）
- 确保脚本有执行权限：`chmod +x script.sh`

### 依赖检查
- 确保 Redis 服务运行
- 确保环境变量正确配置
- 某些脚本需要额外的 npm 包

---

## 🔍 故障排除

### Redis 连接失败
```bash
# 检查 Redis 是否运行
redis-cli ping

# 检查环境变量
echo $REDIS_HOST
echo $REDIS_PORT
```

### 脚本执行失败
```bash
# 检查 Node.js 版本
node --version  # 应为 18+

# 检查权限
ls -l scripts/

# 添加执行权限
chmod +x scripts/**/*.sh
```

### 数据导出失败
```bash
# 检查磁盘空间
df -h

# 检查 data/ 目录权限
ls -ld data/

# 查看详细错误
DEBUG=* npm run data:export
```

---

## 📚 相关文档

- [贡献指南](../docs/CONTRIBUTING.md) - 开发和贡献流程
- [配置参考](../docs/CONFIGURATION.md) - 配置选项说明（待创建）
- [部署指南](../docs/DEPLOYMENT.md) - 详细部署说明（待创建）
- [架构设计](../docs/ARCHITECTURE.md) - 系统架构文档

---

## 🆘 获取帮助

如果遇到问题：

1. 查看日志：`logs/claude-relay-*.log`
2. 检查脚本输出的错误信息
3. 查阅相关文档
4. 提交 Issue: [GitHub Issues](https://github.com/your-username/claude-relay-service/issues)
