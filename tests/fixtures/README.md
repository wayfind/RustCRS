# Test Credentials Injection System

## 概述

这个系统允许您使用**真实的、经过人工验证的账户凭据**进行完整的E2E自动化测试，无需修改生产代码。

## 核心理念

1. ✅ **使用真实账户** - 测试真实的API调用，而不是模拟
2. ✅ **安全存储** - 凭据文件被gitignore，永不提交
3. ✅ **简单注入** - 一键脚本自动创建测试数据
4. ✅ **完整自动化** - 支持无人值守的E2E测试

## 快速开始（5分钟）

### 第1步：准备凭据文件

```bash
# 复制模板
cp tests/fixtures/credentials.json.example tests/fixtures/credentials.json

# 编辑并填入真实凭据
vim tests/fixtures/credentials.json
```

### 第2步：填写真实凭据

#### 获取Claude Console Session Token

1. 打开浏览器访问 https://claude.ai
2. 登录您的账户
3. 打开开发者工具（F12）→ Application → Cookies
4. 找到 `sessionKey` cookie，复制其值
5. 粘贴到 `credentials.json` 的 `session_token` 字段

#### 获取Gemini API Key（可选）

1. 访问 https://aistudio.google.com/app/apikey
2. 创建新的API Key
3. 粘贴到 `credentials.json` 的 `api_key` 字段

### 第3步：注入凭据

```bash
# 确保后端正在运行
make rust-dev

# 在另一个终端，注入凭据
bash tests/fixtures/inject-credentials.sh
```

### 第4步：运行E2E测试

```bash
# 加载测试凭据
source tests/.test-credentials

# 运行E2E测试（60秒）
bash tests/e2e/test-claudeconsole-e2e.sh 60
```

## 文件结构

```
tests/fixtures/
├── README.md                      # 本文档
├── credentials.json.example       # 凭据模板（提交到git）
├── credentials.json               # 真实凭据（.gitignore）
└── inject-credentials.sh          # 注入脚本

tests/
└── .test-credentials              # 生成的测试环境变量（.gitignore）
```

## credentials.json 格式说明

```json
{
  "admin": {
    "username": "admin",
    "password": "your-admin-password"
  },

  "accounts": {
    "claude_console": [
      {
        "name": "My Claude Account",
        "session_token": "sk_ant_...",      // 从claude.ai获取
        "custom_api_endpoint": "https://api.claude.ai",
        "priority": 50,
        "active": true
      }
    ],

    "gemini": [
      {
        "name": "My Gemini Account",
        "api_key": "AIza...",                // 从Google AI Studio获取
        "priority": 50,
        "active": true
      }
    ]
  }
}
```

## 注入脚本工作原理

`inject-credentials.sh` 脚本执行以下操作：

1. **验证环境**
   - 检查 credentials.json 是否存在
   - 验证后端服务是否运行
   - 确认依赖工具（jq）已安装

2. **管理员登录**
   - 使用 credentials.json 中的管理员凭据
   - 通过 `/admin/auth/login` 获取JWT token

3. **创建账户**
   - 通过管理API创建Claude Console账户
   - 通过管理API创建Gemini账户（如有）
   - 自动加密并存储凭据到Redis

4. **创建API Key**
   - 生成测试用API Key
   - 绑定到创建的账户

5. **输出测试凭据**
   - 写入 `tests/.test-credentials` 文件
   - 供E2E测试脚本使用

## 安全注意事项

⚠️ **重要安全提醒**：

1. ✅ **credentials.json 已被.gitignore** - 永远不会提交到版本控制
2. ✅ **仅在测试环境使用** - 不要在生产环境使用这些凭据
3. ✅ **定期轮换凭据** - 使用专用的测试账户，定期更换密码
4. ✅ **限制权限** - 测试账户应该有最小必要权限
5. ❌ **不要共享** - credentials.json 包含真实凭据，不要与他人共享

## 测试数据管理

### 查看已创建的测试数据

```bash
# 加载凭据
source tests/.test-credentials

# 查看账户
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/admin/claude-console-accounts | jq .

# 查看API Keys
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/admin/api-keys | jq .
```

### 清理测试数据

```bash
# 方法1: 通过管理API删除（推荐）
curl -X DELETE -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/admin/api-keys/$TEST_API_KEY_ID

# 方法2: 清空Redis（谨慎！会删除所有数据）
redis-cli FLUSHDB
```

### 重新注入

```bash
# 如果需要重新注入（会创建新的账户和API Key）
bash tests/fixtures/inject-credentials.sh
```

## 故障排除

### 问题: "credentials.json not found"

**解决方案**：
```bash
cp tests/fixtures/credentials.json.example tests/fixtures/credentials.json
vim tests/fixtures/credentials.json  # 填入真实凭据
```

### 问题: "Backend not responding"

**解决方案**：
```bash
# 启动后端
make rust-dev

# 或手动启动
cd rust && cargo run --release
```

### 问题: "Admin login failed"

**解决方案**：
1. 检查 `rust/data/init.json` 是否存在
2. 确认 credentials.json 中的管理员密码与 init.json 一致
3. 检查后端日志

### 问题: "Failed to create account"

**解决方案**：
1. 验证 session_token 是否有效（登录 claude.ai 检查）
2. 检查 session_token 格式（应该是 `sk_ant_...` 格式）
3. 查看后端日志中的详细错误信息

## 高级用法

### 添加多个账户

您可以在 credentials.json 中添加多个同类型账户：

```json
{
  "accounts": {
    "claude_console": [
      {
        "name": "Primary Account",
        "session_token": "sk_ant_xxx1...",
        "priority": 100
      },
      {
        "name": "Backup Account",
        "session_token": "sk_ant_xxx2...",
        "priority": 50
      }
    ]
  }
}
```

### 自定义API端点

如果您使用反向代理或自定义端点：

```json
{
  "accounts": {
    "claude_console": [
      {
        "name": "Through Proxy",
        "session_token": "sk_ant_xxx...",
        "custom_api_endpoint": "https://my-proxy.example.com/api"
      }
    ]
  }
}
```

### 集成到CI/CD

如果需要在CI/CD中运行E2E测试：

```yaml
# .github/workflows/e2e-test.yml
- name: Setup test credentials
  env:
    CLAUDE_SESSION_TOKEN: ${{ secrets.CLAUDE_SESSION_TOKEN }}
  run: |
    cat > tests/fixtures/credentials.json <<EOF
    {
      "admin": {"username": "admin", "password": "admin123"},
      "accounts": {
        "claude_console": [{
          "name": "CI Test Account",
          "session_token": "$CLAUDE_SESSION_TOKEN"
        }]
      }
    }
    EOF

- name: Inject credentials
  run: bash tests/fixtures/inject-credentials.sh

- name: Run E2E tests
  run: |
    source tests/.test-credentials
    bash tests/e2e/test-claudeconsole-e2e.sh 60
```

## 支持的账户类型

当前支持的账户类型：

| 类型 | 字段 | 说明 |
|------|------|------|
| `claude_console` | `session_token` | Claude Console会话令牌 |
| `gemini` | `api_key` | Google Gemini API密钥 |
| `bedrock` | `aws_access_key_id`, `aws_secret_access_key` | AWS Bedrock凭据 |

更多账户类型支持即将到来！

## 相关文档

- [E2E测试指南](../e2e/README.md)
- [API参考文档](../../docs/guides/api-reference.md)
- [故障排除指南](../../docs/guides/troubleshooting.md)

## 贡献

如果您想改进这个系统：

1. Fork项目
2. 创建功能分支
3. 提交改进（记得不要提交credentials.json！）
4. 发起Pull Request

---

**最后更新**: 2025-11-08
**维护者**: Claude Code AI
