# 🚀 Claude Console 测试快速开始

## 一键运行

```bash
cd /mnt/d/prj/claude-relay-service

# 1分钟快速验证
bash test-claudeconsole-preconfigured.sh 60

# 5分钟标准测试（推荐）
bash test-claudeconsole-preconfigured.sh 300

# 10分钟深度测试
bash test-claudeconsole-preconfigured.sh 600
```

## 📋 脚本已内置配置

✅ Claude Console 端点: `https://us3.pincc.ai/api`
✅ Session Token: `cr_022dc9fc...`（已配置）
✅ 测试参数: 每3秒一个请求，自动生成测试问题

## 🎯 两种测试模式

### 模式 1: 直接测试（无需配置）
```
运行脚本 → 选择 "n" → 直接验证凭据
```
- ✅ 零配置，立即开始
- ✅ 验证 session_token 有效性
- ✅ 不需要 API Key

### 模式 2: 完整测试（需要API Key）
```
运行脚本 → 选择 "Y" → 输入 API Key → 测试中转流程
```
- ✅ 测试完整的 Rust 后端中转
- ✅ 验证统计数据准确性
- ⚠️ 需要先在管理界面创建 API Key

## 📊 自动生成报告

测试完成后自动生成：

```
logs/
├── test-report-YYYYMMDD-HHMMSS.md  # 详细报告
├── test-success.log                 # 成功记录
└── test-errors.log                  # 错误记录（如有）
```

## 💡 常用命令

```bash
# 快速验证（60秒）
bash test-claudeconsole-preconfigured.sh 60

# 后台运行长测试
nohup bash test-claudeconsole-preconfigured.sh 1800 > logs/test.log 2>&1 &

# 查看实时日志
tail -f logs/test.log

# 查看最新报告
cat logs/test-report-*.md | tail -100
```

## ✅ 成功标准

- **成功率**: > 95%
- **稳定性**: 错误 < 5个
- **响应**: 平均 < 3秒

## 📖 详细文档

- **完整指南**: `README-测试脚本使用.md`
- **测试方案**: `claudedocs/claudeconsole-test-plan.md`
- **快速入门**: `claudedocs/claudeconsole-test-quickstart.md`

---

**提示**: 首次使用建议先运行60秒测试验证环境正常！
