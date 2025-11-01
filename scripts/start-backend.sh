#!/bin/bash
# Rust 后端启动脚本

set -e

MODE=${1:-dev}  # dev 或 release

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}🦀 启动 Rust 后端 (${MODE} 模式)${NC}"

# 确保 .env 文件存在
if [ ! -f ".env" ]; then
    echo -e "${YELLOW}⚠️  .env 文件不存在，从模板创建...${NC}"
    cp .env.example .env
    echo -e "${YELLOW}⚠️  请编辑 .env 文件配置必要的环境变量${NC}"
fi

# 检查必填环境变量
if ! grep -q "CRS_SECURITY__JWT_SECRET=.\{32\}" .env || ! grep -q "CRS_SECURITY__ENCRYPTION_KEY=.\{32\}" .env; then
    echo -e "${YELLOW}⚠️  警告: JWT_SECRET 或 ENCRYPTION_KEY 可能未正确配置${NC}"
    echo -e "${YELLOW}   JWT_SECRET 需要至少 32 字符${NC}"
    echo -e "${YELLOW}   ENCRYPTION_KEY 必须恰好 32 字符${NC}"
fi

# 加载环境变量到当前 shell
echo -e "${GREEN}📝 加载环境变量...${NC}"
source scripts/load-env.sh

# 启动后端
if [ "$MODE" = "release" ]; then
    echo -e "${GREEN}📦 构建发布版本...${NC}"
    cargo build --release
    echo -e "${GREEN}🚀 启动发布版本后端${NC}"
    cargo run --release
else
    echo -e "${GREEN}🚀 启动开发版本后端${NC}"
    cargo run
fi
