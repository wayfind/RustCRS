#!/bin/bash

# Claude Relay Service - 本地开发环境启动脚本
# 用法: bash start-dev.sh

set -e  # 遇到错误立即退出

echo "🚀 Claude Relay Service - 本地开发环境启动"
echo "=============================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 检查必需工具
echo "📋 检查必需工具..."

if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Docker 未安装，请先安装 Docker${NC}"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust 未安装，请运行: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo -e "${RED}❌ Node.js 未安装，请先安装 Node.js 18+${NC}"
    exit 1
fi

echo -e "${GREEN}✅ 所有必需工具已安装${NC}"
echo ""

# 检查 .env 文件
echo "🔐 检查环境变量配置..."

if [ ! -f ".env" ]; then
    echo -e "${YELLOW}⚠️  .env 文件不存在，从模板创建...${NC}"
    cp .env.example .env
    echo -e "${YELLOW}⚠️  请编辑 .env 文件，设置 CRS_SECURITY__ENCRYPTION_KEY（必须32字符）${NC}"
    echo -e "${YELLOW}   建议值: CRS_SECURITY__ENCRYPTION_KEY=12345678901234567890123456789012${NC}"
    echo ""
    read -p "按 Enter 继续（确保已配置 .env）..."
fi

# 验证 ENCRYPTION_KEY
if ! grep -q "CRS_SECURITY__ENCRYPTION_KEY=.\{32\}" .env; then
    echo -e "${RED}❌ .env 中的 ENCRYPTION_KEY 未设置或不是32字符${NC}"
    echo -e "${YELLOW}   请设置: CRS_SECURITY__ENCRYPTION_KEY=12345678901234567890123456789012${NC}"
    exit 1
fi

echo -e "${GREEN}✅ 环境变量配置正确${NC}"
echo ""

# 启动 Redis
echo "📊 启动 Redis..."

if docker ps -a | grep -q redis-dev; then
    if docker ps | grep -q redis-dev; then
        echo -e "${GREEN}✅ Redis 已经在运行${NC}"
    else
        echo "🔄 启动已存在的 Redis 容器..."
        docker start redis-dev
        sleep 2
    fi
else
    echo "🆕 创建并启动 Redis 容器..."
    docker run -d --name redis-dev -p 6379:6379 redis:7-alpine
    sleep 3
fi

# 测试 Redis 连接
if redis-cli ping &> /dev/null; then
    echo -e "${GREEN}✅ Redis 连接成功${NC}"
else
    echo -e "${RED}❌ Redis 连接失败${NC}"
    exit 1
fi

echo ""

# 提示用户选择启动模式
echo "🦀 Rust 后端启动选项:"
echo "  1) 开发模式 (cargo run - 快速启动)"
echo "  2) 发布模式 (cargo run --release - 最佳性能)"
echo "  3) 跳过 Rust 后端（手动启动）"
echo ""
read -p "请选择 [1/2/3]: " rust_mode

case $rust_mode in
    1)
        echo ""
        echo "🚀 启动 Rust 后端（开发模式）..."
        echo -e "${YELLOW}ℹ️  Rust 后端将在前台运行，按 Ctrl+C 停止${NC}"
        echo -e "${YELLOW}ℹ️  前端启动请打开新终端运行: cd web/admin-spa && npm run dev${NC}"
        echo ""
        sleep 2
        cd rust/
        cargo run
        ;;
    2)
        echo ""
        echo "🚀 构建并启动 Rust 后端（发布模式）..."
        cd rust/

        if [ ! -f "target/release/claude-relay" ]; then
            echo "📦 首次构建，需要几分钟..."
            cargo build --release
        fi

        echo -e "${YELLOW}ℹ️  Rust 后端将在前台运行，按 Ctrl+C 停止${NC}"
        echo -e "${YELLOW}ℹ️  前端启动请打开新终端运行: cd web/admin-spa && npm run dev${NC}"
        echo ""
        sleep 2
        ./target/release/claude-relay
        ;;
    3)
        echo ""
        echo -e "${YELLOW}⏭️  跳过 Rust 后端自动启动${NC}"
        echo ""
        echo "📝 手动启动命令:"
        echo "  cd rust/"
        echo "  cargo run"
        echo ""
        echo "或发布模式:"
        echo "  cd rust/"
        echo "  cargo build --release"
        echo "  ./target/release/claude-relay"
        echo ""
        ;;
    *)
        echo -e "${RED}❌ 无效选择${NC}"
        exit 1
        ;;
esac

# 检查前端是否已构建
if [ "$rust_mode" = "3" ]; then
    echo ""
    echo "📦 检查前端构建状态..."

    if [ ! -d "web/admin-spa/dist" ] || [ ! "$(ls -A web/admin-spa/dist 2>/dev/null)" ]; then
        echo -e "${YELLOW}⚠️  前端资源未构建，正在构建...${NC}"
        cd web/admin-spa/

        if [ ! -d "node_modules" ]; then
            echo "📦 首次运行，安装依赖..."
            npm install
        fi

        echo "🔨 构建前端资源..."
        npm run build
        cd ../..
        echo -e "${GREEN}✅ 前端构建完成${NC}"
    else
        echo -e "${GREEN}✅ 前端资源已存在${NC}"
    fi

    echo ""
    echo -e "${GREEN}✅ 开发环境准备完成！${NC}"
    echo ""
    echo "🌐 访问地址:"
    echo "  - 管理界面: http://localhost:8080 或 http://localhost:8080/admin-next"
    echo "  - API: http://localhost:8080/api"
    echo "  - 健康检查: curl http://localhost:8080/health"
    echo ""
    echo "💡 提示: 前端和后端现在统一运行在端口 8080"
    echo ""
fi
