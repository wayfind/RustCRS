---
category: development
ai_relevance: high
load_when: "CLI操作、命令行管理、账户管理、数据操作、系统维护"
related_docs:
  - ../guides/troubleshooting.md
  - ../architecture/redis-schema.md
  - ../README.md
---

# CLI 工具使用指南

> **导航**: [返回 CLAUDE.md](../../CLAUDE.md) | [文档索引](../README.md)

本指南详细介绍 Claude Relay Service 的命令行工具（CLI）使用方法。

---

## CLI 工具概述

Claude Relay Service 提供了丰富的 CLI 工具，用于账户管理、系统维护、数据操作等。所有 CLI 命令都通过 `npm run` 执行。

### CLI 工具入口
- **主 CLI**: `npm run cli` - 交互式命令行界面
- **脚本命令**: `npm run <command>` - 直接执行特定操作

---

## API Key 管理

### 创建 API Key
```bash
# 基本创建
npm run cli keys create -- --name "MyApp" --limit 1000

# 带权限创建
npm run cli keys create -- --name "ClaudeOnly" --limit 5000 --permissions "claude"

# 带客户端限制
npm run cli keys create -- --name "ClaudeCodeOnly" --limit 10000 --allowed-clients "claude-code"

# 带模型黑名单
npm run cli keys create -- --name "NoSonnet4" --limit 2000 --blocked-models "claude-sonnet-4.0"
```

**参数说明**:
- `--name`: API Key 名称（必填）
- `--limit`: 请求配额限制
- `--permissions`: 服务权限（all/claude/gemini/openai/droid）
- `--allowed-clients`: 允许的客户端列表（逗号分隔）
- `--blocked-models`: 禁止的模型列表（逗号分隔）

### 列出 API Keys
```bash
# 列出所有 API Keys
npm run cli keys list

# 查看详细信息
npm run cli keys list --verbose
```

### 删除 API Key
```bash
npm run cli keys delete -- --id <keyId>
```

### 更新 API Key
```bash
# 更新配额
npm run cli keys update -- --id <keyId> --limit 2000

# 更新权限
npm run cli keys update -- --id <keyId> --permissions "claude,gemini"

# 更新客户端限制
npm run cli keys update -- --id <keyId> --allowed-clients "claude-code,gemini-cli"
```

---

## 系统状态查看

### 基本状态
```bash
# 查看系统概况
npm run cli status

# 统一状态脚本（推荐）
npm run status

# 详细状态信息
npm run status:detail
```

**输出内容**:
- Redis 连接状态
- 活跃账户数量
- API Key 总数
- 系统资源使用
- 最近请求统计

### 实时监控
```bash
# 增强监控脚本
npm run monitor
```

**监控内容**:
- 实时请求速率
- 账户使用情况
- 错误率统计
- 内存和 CPU 使用

---

## Claude 账户管理

### 列出账户
```bash
# 列出所有 Claude 账户
npm run cli accounts list

# 查看详细信息
npm run cli accounts list --verbose
```

**输出信息**:
- 账户 ID 和名称
- 账户类型（claude-official/claude-console/bedrock/ccr）
- 状态（active/inactive/error）
- Token 过期时间
- 最后使用时间

### 刷新账户 Token
```bash
# 刷新单个账户
npm run cli accounts refresh <accountId>

# 刷新所有账户
npm run cli accounts refresh --all
```

### 添加账户
```bash
# 通过 OAuth 添加（推荐使用 Web 界面）
npm run cli accounts add -- --name "Account1"

# 交互式添加流程
npm run cli accounts add
```

### 账户状态操作
```bash
# 启用账户
npm run cli accounts enable -- --id <accountId>

# 禁用账户
npm run cli accounts disable -- --id <accountId>

# 删除账户
npm run cli accounts delete -- --id <accountId>
```

---

## Gemini 账户管理

### 列出 Gemini 账户
```bash
# 列出所有 Gemini 账户
npm run cli gemini list

# 详细信息
npm run cli gemini list --verbose
```

### 添加 Gemini 账户
```bash
# 交互式添加
npm run cli gemini add -- --name "Gemini1"

# 使用 Web 界面添加（推荐）
```

### 刷新 Gemini Token
```bash
# 刷新特定账户
npm run cli gemini refresh <accountId>

# 测试 Gemini token 刷新
node scripts/test-gemini-refresh.js
```

---

## 管理员操作

### 创建管理员
```bash
# 创建新管理员
npm run cli admin create -- --username admin2

# 交互式创建（会提示输入密码）
npm run cli admin create
```

### 重置管理员密码
```bash
# 重置密码
npm run cli admin reset-password -- --username admin

# 交互式重置
npm run cli admin reset-password
```

### 列出管理员
```bash
npm run cli admin list
```

### 删除管理员
```bash
npm run cli admin delete -- --username <username>
```

---

## 数据管理

### 数据导出

#### 标准导出
```bash
# 导出所有 Redis 数据
npm run data:export

# 导出到指定文件
npm run data:export -- --output ./backup/data.json
```

#### 脱敏导出
```bash
# 导出脱敏数据（用于分享或调试）
npm run data:export:sanitized

# 脱敏内容：
# - 移除敏感字段（accessToken, refreshToken, credentials）
# - 保留数据结构和关系
# - 可安全分享
```

#### 增强导出
```bash
# 增强导出（含解密数据，仅内部调试用）
npm run data:export:enhanced

# 包含：
# - 解密的 OAuth tokens
# - 完整的账户凭据
# - 敏感配置信息
# ⚠️ 仅用于本地调试，不要分享！
```

#### 加密导出
```bash
# 导出加密数据（安全传输）
npm run data:export:encrypted

# 使用场景：
# - 跨环境数据迁移
# - 安全备份
# - 加密存储
```

### 数据导入

#### 标准导入
```bash
# 导入数据
npm run data:import

# 从指定文件导入
npm run data:import -- --input ./backup/data.json
```

#### 增强导入
```bash
# 增强导入（含加密数据）
npm run data:import:enhanced

# 支持：
# - 自动解密导入数据
# - 验证数据完整性
# - 冲突处理策略
```

### Redis 键调试
```bash
# 查看所有 Redis 键
npm run data:debug

# 输出内容：
# - 所有键列表
# - 键类型和 TTL
# - 键前缀统计
# - 数据大小分析
```

---

## 数据迁移和修复

### API Key 过期时间迁移
```bash
# 执行迁移
npm run migrate:apikey-expiry

# 干跑模式（只检查，不修改）
npm run migrate:apikey-expiry:dry

# 用途：
# - 迁移旧版 API Key 数据结构
# - 添加过期时间字段
# - 修复不一致数据
```

### 使用统计修复
```bash
# 修复使用统计数据
npm run migrate:fix-usage-stats

# 修复内容：
# - 重建统计索引
# - 修复计数错误
# - 清理无效数据
```

---

## 成本和定价

### 初始化成本数据
```bash
# 初始化成本统计系统
npm run init:costs

# 操作内容：
# - 加载模型价格配置
# - 初始化成本追踪
# - 创建统计索引
```

### 更新模型价格
```bash
# 更新模型价格
npm run update:pricing

# 用途：
# - 同步最新模型价格
# - 更新定价规则
# - 刷新成本计算配置
```

### 测试价格回退
```bash
# 测试价格回退机制
npm run test:pricing-fallback

# 测试场景：
# - 未知模型的价格计算
# - 价格缺失时的回退策略
# - 成本估算准确性
```

---

## 监控和日志

### 增强监控
```bash
# 启动增强监控
npm run monitor

# 监控内容：
# - 实时请求统计
# - 账户使用情况
# - 错误率和延迟
# - 系统资源使用
```

### 查看日志
```bash
# 实时查看主日志
tail -f logs/claude-relay-$(date +%Y-%m-%d).log

# 查看 token 刷新错误
tail -f logs/token-refresh-error.log

# 查看 Webhook 日志
tail -f logs/webhook-$(date +%Y-%m-%d).log

# 查看 HTTP 调试日志（需启用 DEBUG_HTTP_TRAFFIC）
tail -f logs/http-debug-$(date +%Y-%m-%d).log
```

### 日志过滤
```bash
# 查看错误日志
grep ERROR logs/claude-relay-*.log

# 查看特定账户日志
grep "accountId:abc123" logs/claude-relay-*.log

# 查看特定 API Key 日志
grep "keyId:xyz789" logs/claude-relay-*.log
```

---

## 服务管理

### 启动服务
```bash
# 后台启动（推荐）
npm run service:start:daemon

# 前台启动（开发调试）
npm run dev

# 生产模式启动
npm start
```

### 查看服务状态
```bash
# 查看服务状态
npm run service:status

# 查看详细状态
npm run status:detail
```

### 查看服务日志
```bash
# 查看服务日志
npm run service:logs

# 实时跟踪日志
npm run service:logs -- --follow
```

### 停止服务
```bash
# 停止服务
npm run service:stop

# 强制停止
npm run service:stop -- --force
```

---

## 开发和测试

### 开发模式
```bash
# 启动开发服务器（热重载）
npm run dev

# 启动带调试的开发服务器
DEBUG=* npm run dev
```

### 代码检查
```bash
# 运行 ESLint
npm run lint

# 自动修复代码风格问题
npm run lint:fix

# Prettier 格式化
npx prettier --write "src/**/*.js"
```

### 测试
```bash
# 运行测试套件
npm test

# 运行特定测试
npm test -- --testPathPattern=<pattern>

# 生成覆盖率报告
npm test -- --coverage
```

---

## 高级操作

### 批量账户操作
```bash
# 刷新所有账户 token
for id in $(npm run cli accounts list --json | jq -r '.[].id'); do
  npm run cli accounts refresh $id
done

# 批量启用账户
for id in $(npm run cli accounts list --json | jq -r '.[] | select(.status=="inactive") | .id'); do
  npm run cli accounts enable --id $id
done
```

### 批量 API Key 操作
```bash
# 导出所有 API Keys
npm run cli keys list --json > api-keys-backup.json

# 批量更新配额
for id in $(npm run cli keys list --json | jq -r '.[].id'); do
  npm run cli keys update --id $id --limit 5000
done
```

### 数据清理
```bash
# 清理过期数据
redis-cli --scan --pattern "rate_limit:*" | xargs redis-cli del

# 清理过载标记
redis-cli --scan --pattern "overload:*" | xargs redis-cli del

# 清理过期会话
redis-cli --scan --pattern "session:*" | xargs redis-cli del
```

---

## 故障恢复

### 数据恢复
```bash
# 从备份恢复
npm run data:import -- --input ./backup/data.json

# 增强恢复（含加密数据）
npm run data:import:enhanced -- --input ./backup/encrypted.json
```

### 系统重置
```bash
# 重新初始化系统
npm run setup

# 清空所有数据（⚠️ 危险操作）
redis-cli FLUSHALL
npm run setup
```

### 账户修复
```bash
# 刷新所有账户
npm run cli accounts refresh --all

# 重新同步账户状态
npm run cli accounts sync
```

---

## 脚本参数说明

### 通用参数
- `--verbose` / `-v`: 详细输出
- `--quiet` / `-q`: 静默模式
- `--json`: JSON 格式输出
- `--help` / `-h`: 显示帮助信息

### 数据操作参数
- `--input <file>`: 指定输入文件
- `--output <file>`: 指定输出文件
- `--force`: 强制执行（跳过确认）
- `--dry-run`: 干跑模式（不实际修改）

### 账户操作参数
- `--id <accountId>`: 指定账户 ID
- `--name <name>`: 账户名称
- `--type <type>`: 账户类型
- `--all`: 应用到所有账户

### API Key 参数
- `--id <keyId>`: 指定 API Key ID
- `--name <name>`: API Key 名称
- `--limit <number>`: 配额限制
- `--permissions <perms>`: 权限列表
- `--allowed-clients <clients>`: 允许的客户端
- `--blocked-models <models>`: 禁止的模型

---

## 最佳实践

### 定期维护
```bash
# 每日备份
npm run data:export -- --output ./backups/daily-$(date +%Y%m%d).json

# 每周账户刷新
npm run cli accounts refresh --all

# 每月数据清理
npm run data:cleanup
```

### 性能优化
```bash
# 监控系统性能
npm run monitor

# 检查缓存效率
curl http://localhost:3000/metrics | jq '.cache'

# 优化 Redis 内存
redis-cli INFO memory
```

### 安全审计
```bash
# 检查 API Key 使用
npm run cli keys list --verbose

# 检查异常登录
grep "login" logs/claude-relay-*.log

# 审计账户权限
npm run cli accounts list --json | jq '.[] | {id, permissions}'
```

---

**相关文档**:
- [故障排除指南](../guides/troubleshooting.md)
- [Redis 数据结构](../architecture/redis-schema.md)
- [项目主文档](../../CLAUDE.md)
