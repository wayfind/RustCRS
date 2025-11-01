# Repository Guidelines

## 项目结构与模块划分
- `rust/` 是主服务（Axum），业务代码位于 `src/`，集成测试在 `tests/`，性能基准放在 `benches/`。
- `web/admin-spa/` 保存 Vue 3 管理台，`dist/` 产物保持忽略；本地调试时需单独安装依赖。
- `nodejs-archive/` 为历史实现，仅作迁移参考，不再接受新提交。
- 通用资源位于 `config/`、`resources/` 与 `docs/`；新增配置时同步更新 `.env.example` 与 `config/config.example.js`。

## 构建、测试与开发命令
- Rust：`make rust-dev` 启动全栈开发环境，`cargo build --release` 构建优化二进制，`cargo run` 快速验证局部改动。
- 前端：在 `web/admin-spa/` 下执行 `npm install`，随后用 `npm run dev` 启动 Vite，发布前执行 `npm run build`。
- 容器：`docker-compose up -d` 启动 Redis + API + UI，查看日志使用 `docker-compose logs -f claude-relay`。

## 代码风格与命名
- Rust 代码提交前必须通过 `cargo fmt` 与 `cargo clippy --all-targets --all-features`；模块目录使用蛇形命名，结构体与枚举保持帕斯卡命名，异步处理函数约定 `_handler` 结尾。
- JS/TS 代码依赖 ESLint + Prettier（2 空格缩进、单引号、无分号）；变量和方法使用 `camelCase`，Vue 组件文件采用 kebab-case。
- 配置项统一使用大写下划线，例如 `CRS_SECURITY__JWT_SECRET`。

## 测试规范
- 合并前执行 `cargo test`；涉及 Redis、账户调度或网络层改动时运行 `./rust/run-integration-tests.sh`。
- 前端与遗留 Node 工具使用 Jest：`npm test` 会扫描 `tests/` 与 `*.test.js`。
- 新增外部行为需补充 `rust/tests/` 集成用例，文件名以功能命名（例如 `accounts_rate_limit.rs`）。

## 提交与 PR 要求
- 推荐采用祈使句并指明作用域，如 `feat(rust): add claude usage tracker`；跟踪 issue 时在正文引用。
- PR 描述需包含变更摘要、迁移影响与自测结果；界面调整附截图，后端流程链接相关文档或工单。
- 提交评审前请运行格式化、Lint 与相关测试；若有跳过项，必须在 PR 中说明原因与后续计划。

## 安全与配置提示
- 禁止提交真实密钥；新增敏感配置时先更新示例文件，再在 `docs/CONFIGURATION.md` 说明用途。
- 生产环境的 Redis 凭据与 JWT 密钥暴露后需立即旋转，并通知运维团队同步更新各环境。
