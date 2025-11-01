# 部署指南 - Rust 版本

**版本**: 1.0.0
**更新时间**: 2025-10-31
**目标环境**: 生产环境

---

## 📋 目录

1. [系统要求](#系统要求)
2. [环境配置](#环境配置)
3. [部署方式](#部署方式)
4. [性能调优](#性能调优)
5. [监控和日志](#监控和日志)
6. [故障排除](#故障排除)
7. [安全加固](#安全加固)

---

## 📦 系统要求

### 最低配置

| 组件 | 最低要求 | 推荐配置 |
|------|---------|---------|
| CPU | 2 核 | 4 核+ |
| 内存 | 2GB | 4GB+ |
| 磁盘 | 10GB | 50GB+ SSD |
| 网络 | 100Mbps | 1Gbps |
| OS | Linux 4.x+ | Ubuntu 22.04 LTS |

### 软件依赖

**必需**:
- Redis 6.0+
- Rust 1.75+ (编译时)

**可选**:
- Docker 20.10+
- Nginx 1.20+ (反向代理)
- systemd (服务管理)

---

## 🔧 环境配置

### 1. 环境变量

创建 `.env` 文件：

```bash
# 基础配置
PORT=3000
NODE_ENV=production

# Redis 配置
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=your_redis_password

# 安全配置 (必需)
CRS_SECURITY__JWT_SECRET=your_very_long_random_jwt_secret_at_least_32_chars
CRS_SECURITY__ENCRYPTION_KEY=12345678901234567890123456789012  # 必须32字符

# 日志配置
RUST_LOG=info
LOG_LEVEL=info
LOG_DIR=./logs

# 性能配置
MAX_CONNECTIONS=10000
REQUEST_TIMEOUT=600000  # 10分钟（毫秒）
```

### 2. 生成安全密钥

```bash
# JWT Secret (建议64字符以上)
openssl rand -base64 64

# Encryption Key (必须32字符)
openssl rand -hex 16
```

### 3. Redis 配置

**生产环境 Redis 配置** (`/etc/redis/redis.conf`):

```conf
# 绑定
bind 127.0.0.1

# 端口
port 6379

# 密码（强制）
requirepass your_strong_redis_password

# 持久化
save 900 1
save 300 10
save 60 10000
appendonly yes

# 内存限制
maxmemory 2gb
maxmemory-policy allkeys-lru

# 性能
tcp-backlog 511
tcp-keepalive 300
```

---

## 🚀 部署方式

### 方式 1: Docker 部署（推荐）

#### 1.1 准备 Dockerfile

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/claude-relay .

EXPOSE 3000

CMD ["./claude-relay"]
```

#### 1.2 Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis-data:/data
    networks:
      - app-network
    restart: unless-stopped

  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - REDIS_HOST=redis
      - REDIS_PORT=6379
      - REDIS_PASSWORD=${REDIS_PASSWORD}
      - CRS_SECURITY__JWT_SECRET=${JWT_SECRET}
      - CRS_SECURITY__ENCRYPTION_KEY=${ENCRYPTION_KEY}
      - RUST_LOG=info
    depends_on:
      - redis
    networks:
      - app-network
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  redis-data:

networks:
  app-network:
```

#### 1.3 部署命令

```bash
# 构建和启动
docker-compose up -d

# 查看日志
docker-compose logs -f app

# 重启服务
docker-compose restart app

# 停止服务
docker-compose down
```

### 方式 2: systemd 服务

#### 2.1 编译发布版本

```bash
cargo build --release
```

#### 2.2 创建 systemd 服务

```ini
# /etc/systemd/system/claude-relay.service
[Unit]
Description=Claude Relay Service (Rust)
After=network.target redis.service
Wants=redis.service

[Service]
Type=simple
User=claude-relay
Group=claude-relay
WorkingDirectory=/opt/claude-relay
ExecStart=/opt/claude-relay/target/release/claude-relay

# 环境变量
EnvironmentFile=/opt/claude-relay/.env

# 重启策略
Restart=on-failure
RestartSec=5s

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096

# 安全加固
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/claude-relay/logs

[Install]
WantedBy=multi-user.target
```

#### 2.3 安装和启动

```bash
# 创建用户
sudo useradd -r -s /bin/false claude-relay

# 复制文件
sudo mkdir -p /opt/claude-relay
sudo cp -r . /opt/claude-relay/
sudo chown -R claude-relay:claude-relay /opt/claude-relay

# 启用服务
sudo systemctl daemon-reload
sudo systemctl enable claude-relay
sudo systemctl start claude-relay

# 查看状态
sudo systemctl status claude-relay

# 查看日志
sudo journalctl -u claude-relay -f
```

### 方式 3: 直接运行（开发/测试）

```bash
# 生产模式运行
RUST_LOG=info cargo run --release
```

---

## 🔥 性能调优

### 1. Rust 编译优化

在 `Cargo.toml` 中：

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

### 2. Redis 连接池

在代码中（已配置）：

```rust
// src/redis/mod.rs
const POOL_SIZE: usize = 100;  // 调整为适合负载的值
const TIMEOUT: u64 = 30;       // 连接超时（秒）
```

### 3. 系统调优

**文件描述符限制**:

```bash
# /etc/security/limits.conf
claude-relay soft nofile 65536
claude-relay hard nofile 65536
```

**内核参数** (`/etc/sysctl.conf`):

```conf
# TCP 连接
net.core.somaxconn = 65536
net.ipv4.tcp_max_syn_backlog = 65536
net.ipv4.ip_local_port_range = 1024 65535

# TIME_WAIT 复用
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 30
```

应用配置:
```bash
sudo sysctl -p
```

### 4. Nginx 反向代理（推荐）

```nginx
# /etc/nginx/sites-available/claude-relay
upstream claude_relay {
    least_conn;
    server 127.0.0.1:3000 max_fails=3 fail_timeout=30s;
    # 如果有多个实例
    # server 127.0.0.1:3001 max_fails=3 fail_timeout=30s;
    # server 127.0.0.1:3002 max_fails=3 fail_timeout=30s;
}

server {
    listen 80;
    server_name api.example.com;

    # HTTPS 重定向
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name api.example.com;

    # SSL 证书
    ssl_certificate /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;

    # SSL 配置
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # 安全头
    add_header Strict-Transport-Security "max-age=31536000" always;
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;

    # 日志
    access_log /var/log/nginx/claude-relay-access.log;
    error_log /var/log/nginx/claude-relay-error.log;

    # 代理设置
    location / {
        proxy_pass http://claude_relay;
        proxy_http_version 1.1;

        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # 超时
        proxy_connect_timeout 60s;
        proxy_send_timeout 600s;
        proxy_read_timeout 600s;

        # 缓冲
        proxy_buffering off;
        proxy_request_buffering off;

        # SSE 支持（流式响应）
        proxy_set_header Connection '';
        proxy_cache_bypass $http_upgrade;
        chunked_transfer_encoding on;

        # 客户端最大body大小
        client_max_body_size 10M;
    }

    # 健康检查
    location /health {
        proxy_pass http://claude_relay;
        access_log off;
    }
}
```

启用配置:
```bash
sudo ln -s /etc/nginx/sites-available/claude-relay /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

---

## 📊 监控和日志

### 1. 日志配置

**日志级别**:
```bash
# 开发
RUST_LOG=debug

# 生产
RUST_LOG=info

# 详细调试
RUST_LOG=trace
```

**日志文件**:
- `logs/claude-relay-YYYY-MM-DD.log` - 应用日志
- `/var/log/nginx/claude-relay-*.log` - Nginx日志

### 2. 健康检查

```bash
# 基础健康检查
curl http://localhost:3000/health

# 完整状态检查
curl http://localhost:3000/metrics
```

**预期响应**:
```json
{
  "status": "ok",
  "version": "1.0.0",
  "uptime": 12345,
  "components": {
    "redis": "ok",
    "logger": "ok"
  }
}
```

### 3. 监控指标

**系统监控**:
- CPU 使用率 < 80%
- 内存使用率 < 75%
- 磁盘使用率 < 80%
- 网络连接数 < 文件描述符限制的50%

**应用监控**:
- 请求成功率 > 99%
- 平均响应时间 < 500ms
- Redis 连接池使用率 < 80%
- 错误率 < 1%

### 4. 日志轮转

```bash
# /etc/logrotate.d/claude-relay
/opt/claude-relay/logs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 0640 claude-relay claude-relay
    sharedscripts
    postrotate
        systemctl reload claude-relay > /dev/null 2>&1 || true
    endscript
}
```

---

## 🔧 故障排除

### 常见问题

#### 1. 服务无法启动

**症状**: `systemctl start claude-relay` 失败

**排查**:
```bash
# 查看详细日志
sudo journalctl -u claude-relay -n 50 --no-pager

# 检查配置文件
cat /opt/claude-relay/.env

# 验证密钥长度
echo -n "$CRS_SECURITY__ENCRYPTION_KEY" | wc -c  # 必须是32
```

**常见原因**:
- ENCRYPTION_KEY 不是32字符
- JWT_SECRET 少于32字符
- Redis 连接失败

#### 2. Redis 连接错误

**症状**: `Failed to connect to Redis`

**排查**:
```bash
# 测试 Redis 连接
redis-cli -h localhost -p 6379 -a your_password ping

# 检查 Redis 状态
sudo systemctl status redis

# 查看 Redis 日志
sudo journalctl -u redis -n 50
```

#### 3. 高内存使用

**症状**: 内存使用持续增长

**排查**:
```bash
# 查看进程内存
ps aux | grep claude-relay

# 检查 Redis 内存
redis-cli INFO memory
```

**解决方案**:
- 调整解密缓存大小（src/utils/crypto.rs:125）
- 配置Redis `maxmemory` 限制
- 重启服务释放内存

#### 4. 请求超时

**症状**: 请求长时间无响应

**检查**:
```bash
# 检查并发连接数
netstat -an | grep :3000 | wc -l

# 检查 Redis 延迟
redis-cli --latency

# 查看慢查询
grep "slow" logs/claude-relay-*.log
```

---

## 🔒 安全加固

### 1. 防火墙配置

```bash
# UFW (Ubuntu)
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw deny 3000/tcp   # 禁止直接访问应用端口
sudo ufw deny 6379/tcp   # 禁止直接访问Redis
sudo ufw enable
```

### 2. SSL/TLS 证书

**Let's Encrypt (推荐)**:
```bash
# 安装 Certbot
sudo apt install certbot python3-certbot-nginx

# 获取证书
sudo certbot --nginx -d api.example.com

# 自动续期
sudo systemctl enable certbot.timer
```

### 3. 密钥轮换

**定期更换密钥** (建议每6个月):

```bash
# 1. 生成新密钥
NEW_JWT_SECRET=$(openssl rand -base64 64)
NEW_ENCRYPTION_KEY=$(openssl rand -hex 16)

# 2. 更新 .env 文件
# 3. 重启服务
sudo systemctl restart claude-relay
```

**注意**: 更换 ENCRYPTION_KEY 需要重新加密所有数据！

### 4. 访问控制

**限制 API Key 创建**:
- 仅允许管理员创建 API Key
- 使用强随机API Key（推荐32字符以上）
- 设置合理的速率限制

**IP 白名单** (可选):
```nginx
# Nginx location 块中
location / {
    allow 1.2.3.4;        # 允许的IP
    allow 5.6.7.0/24;     # 允许的IP段
    deny all;

    proxy_pass http://claude_relay;
}
```

### 5. 定期安全检查

```bash
# 检查依赖漏洞
cargo audit

# 更新依赖
cargo update

# 检查系统安全
sudo apt update
sudo apt upgrade

# 检查日志异常
grep -i "error\|fail\|unauthorized" logs/claude-relay-*.log
```

---

## 📋 部署检查清单

### 部署前

- [ ] 编译发布版本 (`cargo build --release`)
- [ ] 生成强随机密钥（JWT_SECRET, ENCRYPTION_KEY）
- [ ] 配置 Redis 密码
- [ ] 设置环境变量 (`.env` 文件)
- [ ] 运行测试 (`cargo test`)
- [ ] 检查依赖漏洞 (`cargo audit`)

### 部署时

- [ ] 创建系统用户 (`claude-relay`)
- [ ] 设置文件权限
- [ ] 配置 systemd 服务
- [ ] 配置 Nginx 反向代理
- [ ] 申请 SSL 证书
- [ ] 配置防火墙规则
- [ ] 设置日志轮转

### 部署后

- [ ] 验证健康检查 (`/health`)
- [ ] 测试 API 端点
- [ ] 检查日志无错误
- [ ] 监控系统资源
- [ ] 设置监控告警
- [ ] 备份配置文件
- [ ] 文档化部署信息

---

## 🔄 更新和维护

### 滚动更新

```bash
# 1. 拉取最新代码
git pull origin main

# 2. 编译新版本
cargo build --release

# 3. 备份当前版本
sudo cp /opt/claude-relay/target/release/claude-relay \
        /opt/claude-relay/target/release/claude-relay.backup

# 4. 替换新版本
sudo cp target/release/claude-relay /opt/claude-relay/target/release/

# 5. 重启服务
sudo systemctl restart claude-relay

# 6. 验证服务
curl http://localhost:3000/health
```

### 回滚

```bash
# 恢复备份
sudo cp /opt/claude-relay/target/release/claude-relay.backup \
        /opt/claude-relay/target/release/claude-relay

# 重启服务
sudo systemctl restart claude-relay
```

---

**部署文档版本**: 1.0.0
**最后更新**: 2025-10-31
**维护者**: Rust Migration Team

**紧急联系**: 参考 `README.md` 中的联系方式
