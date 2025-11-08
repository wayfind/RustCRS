# Port 8080 统一架构迁移总结

## 迁移日期
2025-11-02

## 架构变更

### 旧架构（已废弃）
- **开发模式**:
  - Rust 后端：端口 8080
  - Vue 前端：端口 3001（Vite 开发服务器）
  - 前端通过代理访问后端 API

### 新架构（当前）
- **统一端口模式**:
  - Rust 后端 + Vue 前端：**统一端口 8080**
  - 前端编译为静态文件放在 `web/admin-spa/dist/`
  - Rust 后端同时提供 API 和静态文件服务

## 已完成的文档更新

### 1. docs/guides/quickstart.md ✅
- 移除所有 3001 端口引用
- 更新"手动启动"章节：前端改为 `npm run build`
- 更新"验证服务"章节：统一为 `http://localhost:8080`
- 更新"故障排除"章节：移除前端连接问题，改为前端构建问题
- 更新成功检查清单：添加前端构建检查

### 2. CLAUDE.md ✅
- 更新快速启动命令：移除 `npm run dev`
- 更新架构图：移除 Vue 3 前端独立端口
- 移除 `make rust-frontend` 命令说明
- 更新前端故障排除：改为检查静态文件构建

## 待完成的更新

### 3. docs/guides/debugging.md ⏳
需要更新的位置：
- Line 12: `Vue 3 前端管理界面（端口 3001）` → `Rust 后端 + Vue 3 前端（统一端口 8080）`
- Line 197-201: 前端启动命令改为构建命令
- Line 233: 访问地址改为 `http://localhost:8080`
- Line 293: 移除端口 3001 检查
- Line 466: 前端地址改为 `http://localhost:8080`

### 4. start-dev.sh ⏳
需要修改：
- Line 143-174: 移除前端启动询问逻辑
- 添加前端构建检查逻辑
- 更新访问地址提示信息

### 5. 其他文档
- `docs/guides/deployment.md` - 可能包含 3001 端口引用
- `docs/archive/` 下的迁移文档 - 作为历史记录可以保留

## 关键代码依赖

### Rust 后端 (rust/src/main.rs:209-227)
```rust
// 静态文件服务配置
let static_dir = PathBuf::from("../web/admin-spa/dist");
let serve_dir = ServeDir::new(&static_dir)
    .not_found_service(ServeDir::new(&static_dir).append_index_html_on_directories(true));

// 路由配置
Router::new()
    .route("/", get(|| async { Redirect::permanent("/admin-next") }))
    .nest_service("/admin-next", serve_dir)
```

### Vue 前端构建
```bash
cd web/admin-spa/
npm run build  # 输出到 dist/
```

### 前端配置 (web/admin-spa/vite.config.js)
- Line 64: `port: 3001` - 仅在开发模式使用，已废弃
- 生产构建通过 `npm run build` 生成静态文件

## 用户影响

### 开发流程变更
**旧流程**:
```bash
# 终端 1
cd rust && cargo run

# 终端 2
cd web/admin-spa && npm run dev

# 访问 http://localhost:3001
```

**新流程**:
```bash
# 一次性构建前端
cd web/admin-spa && npm run build

# 启动后端（包含前端）
cd rust && cargo run

# 访问 http://localhost:8080
```

### 优势
1. **简化部署**: 单一端口，无需 CORS 配置
2. **统一访问**: 前后端同一地址，避免混淆
3. **生产一致**: 开发环境和生产环境架构相同
4. **减少资源**: 不需要 Vite 开发服务器常驻

### 注意事项
1. **前端修改**: 需要重新运行 `npm run build`
2. **热重载**: 失去 Vite 热重载功能（可选择保留 3001 开发模式）
3. **首次启动**: 必须先构建前端才能访问管理界面

## 下一步行动

1. ✅ 完成 `docs/guides/debugging.md` 更新
2. ✅ 完成 `start-dev.sh` 脚本更新
3. ✅ 更新 `README.md`（如果存在 3001 引用）
4. ⚠️ 决策：是否完全移除 `vite.config.js` 中的端口 3001 配置？
5. ⚠️ 决策：Makefile 中的 `make rust-frontend` 目标是否需要移除？

## 回滚方案

如果需要恢复双端口模式：
1. 恢复文档中的 3001 端口说明
2. 保持 `vite.config.js` 端口配置
3. 添加开发/生产模式说明

## 相关 Issue/PR

- Feature: 统一端口架构
- 迁移原因：简化部署和开发流程
