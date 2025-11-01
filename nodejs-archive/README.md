# Node.js 版本归档

这是 Claude Relay Service 的 Node.js 实现归档。

**状态**: 已归档（仅供参考）
**主实现**: ../rust/
**归档时间**: 2025-10-31

## 说明

该目录包含原 Node.js 版本的完整代码，包括：

- `src/` - Node.js 源代码
- `scripts/` - 工具脚本
- `cli/` - 命令行工具
- `package.json` - Node.js 依赖配置

## 运行归档版本

如需运行 Node.js 版本（调试对比用）：

```bash
cd nodejs-archive/
npm install

# 需要配置环境变量（在项目根目录创建 .env）
# 参考 .env.example.nodejs

npm run dev  # 开发模式，端口 3000
```

## 注意事项

1. **Node.js 版本不再接收新功能更新**，仅作为参考实现保留
2. 安全补丁和关键Bug修复可能会应用
3. 生产环境请使用 Rust 版本（../rust/）
4. 环境变量格式与 Rust 版本不同：
   - Node.js: `JWT_SECRET`, `ENCRYPTION_KEY`
   - Rust: `CRS_SECURITY__JWT_SECRET`, `CRS_SECURITY__ENCRYPTION_KEY`

## 对比参考

如需对比 Node.js 和 Rust 实现的差异，可以：

1. 同时启动两个后端（端口 3000 和 8080）
2. 使用相同的 API 请求测试
3. 对比响应时间和结果一致性

## 文档

原 Node.js 版本的详细文档请参考：
- `../CLAUDE.md` - 项目概述和架构说明
- `../README.md` - 使用指南（已更新为 Rust 版本）
- `../docs/` - API 文档和部署指南
