# 集成测试

本目录包含 Claude Relay Service 的集成测试脚本。

## 测试文件说明

### 账户和认证测试
- `test-account-display.js` - 测试账户显示和格式化
- `test-dedicated-accounts.js` - 测试专用账户功能
- `test-gemini-refresh.js` - 测试 Gemini OAuth token 刷新

### API 和协议测试
- `test-api-response.js` - 测试 API 响应格式
- `test-openai-user-field.sh` - 测试 OpenAI user 字段处理
- `test-extended-thinking.sh` - 测试扩展思考功能

### 模型和定价测试
- `test-bedrock-models.js` - 测试 AWS Bedrock 模型
- `test-model-mapping.js` - 测试模型映射和别名
- `test-pricing-fallback.js` - 测试定价回退机制

### 功能测试
- `test-billing-events.js` - 测试计费事件处理
- `test-group-scheduling.js` - 测试账户组调度
- `test-window-remaining.js` - 测试会话窗口剩余时间
- `test-gemini-tools.sh` - 测试 Gemini 工具调用

### 前端测试
- `test-web-dist.sh` - 测试 Web 前端构建

### 测试数据
- `generate-test-data.js` - 生成测试数据

## 运行测试

### 运行所有测试
```bash
npm test
```

### 运行特定测试
```bash
# JavaScript 测试
node tests/integration/test-account-display.js

# Shell 测试
bash tests/integration/test-extended-thinking.sh
```

### 运行集成测试套件
```bash
# 运行所有集成测试（如果配置）
npm run test:integration
```

## 测试环境要求

- **Node.js**: 18+
- **Redis**: 运行中的 Redis 实例
- **环境变量**: 正确配置的 .env 文件
- **测试账户**: 某些测试需要有效的测试账户凭据

## 注意事项

1. **测试隔离**: 测试应使用独立的测试数据，不应影响生产数据
2. **清理**: 测试完成后应清理创建的测试数据
3. **凭据**: 不要在测试脚本中硬编码敏感凭据
4. **并行**: 某些测试可能不支持并行运行，注意资源竞争

## 添加新测试

1. 在此目录创建新的测试文件
2. 遵循现有测试的命名规范：`test-<feature-name>.js` 或 `.sh`
3. 添加适当的错误处理和清理逻辑
4. 更新本 README 文件的测试说明

## 持续集成

这些测试可以集成到 CI/CD 流程中：

```yaml
# .github/workflows/test.yml
- name: Run Integration Tests
  run: |
    npm run test:integration
```

## 故障排除

### 测试失败常见原因
1. Redis 未运行
2. 环境变量未配置
3. 测试账户凭据过期
4. 端口冲突
5. 网络连接问题

### 调试建议
```bash
# 启用详细日志
DEBUG=* node tests/integration/test-api-response.js

# 检查 Redis 连接
redis-cli ping

# 验证环境变量
node -e "require('dotenv').config(); console.log(process.env)"
```
