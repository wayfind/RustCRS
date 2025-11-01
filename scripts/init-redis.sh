#!/bin/bash
# Redis 初始化和启动脚本

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}📊 Redis 初始化和启动${NC}"

# 检查 Redis 容器是否存在
if docker ps -a | grep -q redis-dev; then
    if docker ps | grep -q redis-dev; then
        echo -e "${GREEN}✅ Redis 已经在运行${NC}"
    else
        echo -e "${YELLOW}🔄 启动已存在的 Redis 容器...${NC}"
        docker start redis-dev
        sleep 2
    fi
else
    echo -e "${YELLOW}🆕 创建并启动 Redis 容器...${NC}"
    docker run -d --name redis-dev -p 6379:6379 redis:7-alpine
    sleep 3
fi

# 测试 Redis 连接
if redis-cli ping &> /dev/null; then
    echo -e "${GREEN}✅ Redis 连接成功${NC}"
else
    echo -e "${YELLOW}⚠️  Redis 连接失败，但容器已启动${NC}"
    echo -e "${YELLOW}   可能需要等待几秒钟后重试${NC}"
fi

# 可选：加载初始数据
if [ -f "data/init.json" ] && [ -f "scripts/load-initial-data.sh" ]; then
    echo -e "${YELLOW}📦 加载初始数据...${NC}"
    bash scripts/load-initial-data.sh || echo -e "${YELLOW}⚠️  初始数据加载失败（可能已存在）${NC}"
fi

echo -e "${GREEN}✅ Redis 初始化完成${NC}"
