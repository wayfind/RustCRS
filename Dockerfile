# 🚀 Claude Relay Service - Rust 版本多阶段构建

# 📦 前端构建阶段
FROM node:18-alpine AS frontend-builder

WORKDIR /app/web/admin-spa

# 复制前端依赖文件
COPY web/admin-spa/package*.json ./

# 安装前端依赖
RUN npm ci

# 复制前端源代码
COPY web/admin-spa/ ./

# 构建前端
RUN npm run build

# 🦀 Rust 构建阶段
FROM rust:1.75 AS rust-builder

WORKDIR /app

# 复制 Cargo 配置文件（利用 Docker 缓存层）
COPY rust/Cargo.toml rust/Cargo.lock ./

# 创建虚拟 main.rs 以预编译依赖（加速后续构建）
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 复制实际源代码
COPY rust/src ./src

# 构建发布版本
RUN cargo build --release

# 🐳 最终运行阶段
FROM debian:bookworm-slim

LABEL maintainer="claude-relay-service@example.com"
LABEL description="Claude Code API Relay Service (Rust)"
LABEL version="2.0.0"

# 安装运行时依赖
RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        curl \
        dumb-init \
        && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 从 Rust 构建阶段复制二进制文件
COPY --from=rust-builder /app/target/release/claude-relay /usr/local/bin/claude-relay

# 从前端构建阶段复制前端产物
COPY --from=frontend-builder /app/web/admin-spa/dist /app/web/admin-spa/dist

# 复制配置文件和静态资源
COPY config/ /app/config/
COPY docs/ /app/docs/
COPY rust/.env.example /app/.env.example

# 创建必要目录
RUN mkdir -p /app/logs /app/data /app/certs

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 启动应用
ENTRYPOINT ["dumb-init", "--"]
CMD ["claude-relay"]
