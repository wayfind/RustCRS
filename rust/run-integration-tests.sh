#!/bin/bash

# E2E Integration Tests Runner
# 完整用户流程端到端测试（使用临时 Docker Redis）

set -e

echo "========================================="
echo "Phase 8.1: E2E 集成测试"
echo "========================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
IGNORED_TESTS=0

# Redis 容器信息
REDIS_CONTAINER=""
REDIS_PORT=""

# 清理函数
cleanup() {
    if [ -n "$REDIS_CONTAINER" ]; then
        echo ""
        echo -e "${BLUE}🧹 清理 Redis 容器...${NC}"
        docker stop "$REDIS_CONTAINER" > /dev/null 2>&1 || true
        docker rm "$REDIS_CONTAINER" > /dev/null 2>&1 || true
        echo -e "${GREEN}✓ Redis 容器已清理${NC}"
    fi
}

# 注册清理函数（脚本退出时自动执行）
trap cleanup EXIT

echo -e "${BLUE}📦 启动临时 Redis 容器${NC}"
echo "----------------------------------------"

# 检查 Docker 是否可用
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}✗ Docker 未运行或无权限访问${NC}"
    echo "请确保 Docker 已启动并且当前用户有权限访问 Docker"
    exit 1
fi

# 生成唯一容器名
CONTAINER_NAME="test-redis-$$"

# 启动 Redis 容器（随机端口）
echo "启动 Redis 容器: $CONTAINER_NAME"
REDIS_CONTAINER=$(docker run -d --rm --name "$CONTAINER_NAME" -p 0:6379 redis:7-alpine)

if [ -z "$REDIS_CONTAINER" ]; then
    echo -e "${RED}✗ 无法启动 Redis 容器${NC}"
    exit 1
fi

# 等待容器启动
sleep 2

# 获取映射的端口
REDIS_PORT=$(docker port "$CONTAINER_NAME" 6379 | cut -d: -f2)

if [ -z "$REDIS_PORT" ]; then
    echo -e "${RED}✗ 无法获取 Redis 端口${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Redis 容器已启动${NC}"
echo "  容器 ID: $REDIS_CONTAINER"
echo "  端口: $REDIS_PORT"
echo "  URL: redis://127.0.0.1:$REDIS_PORT"

# 验证 Redis 连接（通过 Docker exec，不依赖本地 redis-cli）
echo ""
echo "验证 Redis 连接..."
if docker exec "$CONTAINER_NAME" redis-cli ping > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Redis 连接正常${NC}"
else
    echo -e "${YELLOW}⚠ 无法验证 Redis 连接（但容器已启动）${NC}"
    echo "测试将继续，实际连接由测试代码验证"
fi

echo ""
echo -e "${BLUE}🔨 编译项目${NC}"
echo "----------------------------------------"

# 编译项目
if cargo build --release --quiet 2>&1 | grep -q "error"; then
    echo -e "${RED}✗ 编译失败${NC}"
    exit 1
else
    echo -e "${GREEN}✓ 编译成功${NC}"
fi

echo ""
echo -e "${BLUE}🧪 运行集成测试套件${NC}"
echo "----------------------------------------"

# 设置环境变量
export REDIS_URL="redis://127.0.0.1:$REDIS_PORT"
export ENCRYPTION_KEY="test-encryption-key-32chars!!"
export JWT_SECRET="test-jwt-secret-key-for-testing-only-32-chars"

echo "环境变量已设置:"
echo "  REDIS_URL=$REDIS_URL"
echo "  ENCRYPTION_KEY=***"
echo "  JWT_SECRET=***"
echo ""

# 运行所有集成测试
echo "执行所有集成测试..."
echo ""

# 捕获测试输出
TEST_OUTPUT=$(cargo test --tests --release -- --nocapture 2>&1 || true)

# 显示测试输出
echo "$TEST_OUTPUT"

# 提取测试结果
PASSED_TESTS=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= passed)' | tail -1 || echo "0")
FAILED_TESTS=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)' | tail -1 || echo "0")
IGNORED_TESTS=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= ignored)' | tail -1 || echo "0")

# 确保变量不为空
PASSED_TESTS=${PASSED_TESTS:-0}
FAILED_TESTS=${FAILED_TESTS:-0}
IGNORED_TESTS=${IGNORED_TESTS:-0}

# 计算总数
TOTAL_TESTS=$((PASSED_TESTS + FAILED_TESTS + IGNORED_TESTS))

echo ""
echo "========================================="
echo "测试总结"
echo "========================================="
echo "总计: $TOTAL_TESTS 个测试"
echo -e "${GREEN}通过: $PASSED_TESTS${NC}"
if [ "$FAILED_TESTS" -ne 0 ]; then
    echo -e "${RED}失败: $FAILED_TESTS${NC}"
fi
if [ "$IGNORED_TESTS" -ne 0 ]; then
    echo -e "${YELLOW}忽略: $IGNORED_TESTS${NC}"
fi

# 计算通过率
if [ "$TOTAL_TESTS" -gt 0 ]; then
    PASS_RATE=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    echo "通过率: ${PASS_RATE}%"
fi

if [ "$FAILED_TESTS" -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    echo ""
    echo "========================================="
    echo "Phase 8.1 完成"
    echo "========================================="
    exit 0
else
    echo ""
    echo -e "${RED}✗ 部分测试失败${NC}"
    exit 1
fi
