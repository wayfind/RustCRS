# GEMINI.md

## 项目概述

本项目是一个高性能的 AI API 中继服务，旨在为 Claude、Gemini 和 OpenAI 等各种 AI 平台提供统一的接口。后端采用 Rust 编写，以实现高性能和低资源消耗；前端则是一个现代化的 Vue.js 3 单页应用程序。

该服务提供多账户管理、身份验证、监控和成本跟踪等功能。它使用 Redis 进行缓存和数据存储，并可使用 Docker 进行部署。

**关键技术：**

*   **后端：** Rust、Axum、Tokio、Reqwest、Redis
*   **前端：** Vue.js 3、Vite、Pinia、Element Plus
*   **数据库：** Redis
*   **部署：** Docker

## 构建和运行

### 后端 (Rust)

1.  **先决条件：**
    *   Rust 1.75+
    *   Redis 6+

2.  **配置：**
    *   将 `.env.example` 复制为 `.env` 并填写所需的环境变量，尤其是 `CRS_SECURITY__JWT_SECRET` 和 `CRS_SECURITY__ENCRYPTION_KEY`。

3.  **运行：**
    ```bash
    # 导航到 Rust 项目目录
    cd rust/

    # 构建项目
    cargo build --release

    # 运行服务
    ./target/release/claude-relay
    ```

### 前端 (Vue.js)

1.  **先决条件：**
    *   Node.js 和 npm

2.  **运行：**
    ```bash
    # 导航到前端项目目录
    cd web/admin-spa/

    # 安装依赖
    npm install

    # 启动开发服务器
    npm run dev
    ```

### Docker (推荐)

1.  **先决条件：**
    *   Docker 和 Docker Compose

2.  **配置：**
    *   在您的 shell 或 `.env` 文件中设置所需的环境变量。

3.  **运行：**
    ```bash
    # 以分离模式启动所有服务
    docker-compose up -d
    ```

## 测试

### 后端 (Rust)

```bash
# 导航到 Rust 项目目录
cd rust/

# 运行所有测试
cargo test

# 运行集成测试 (需要 Docker)
bash run-integration-tests.sh

# 运行性能基准测试
cargo bench
```

## 开发约定

*   **代码风格：** 项目使用 `rustfmt` 来格式化 Rust 代码，使用 `prettier` 来格式化前端代码。
*   **代码检查：** 使用 `clippy` 来检查 Rust 代码，使用 `eslint` 来检查前端代码，以确保代码质量。
*   **提交信息：** 遵循常规提交标准。
*   **分支：** 对新功能开发使用功能分支。
*   **拉取请求：** 所有新代码都应通过拉取请求提交，并在合并前进行审查。