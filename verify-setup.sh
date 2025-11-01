#!/bin/bash

# Claude Relay Service - 环境验证脚本
# 用法: bash verify-setup.sh

echo "🔍 Claude Relay Service - 环境验证"
echo "===================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

ERRORS=0
WARNINGS=0

# 检查函数
check_command() {
    if command -v $1 &> /dev/null; then
        echo -e "${GREEN}✅ $1 已安装${NC}"
        if [ "$2" != "" ]; then
            VERSION=$($1 $2 2>&1 | head -1)
            echo "   版本: $VERSION"
        fi
        return 0
    else
        echo -e "${RED}❌ $1 未安装${NC}"
        ((ERRORS++))
        return 1
    fi
}

# 1. 检查必需工具
echo "📋 检查必需工具..."
echo ""

check_command "docker" "--version"
check_command "cargo" "--version"
check_command "rustc" "--version"
check_command "node" "--version"
check_command "npm" "--version"
check_command "redis-cli" "--version"

echo ""

# 2. 检查目录结构
echo "📁 检查目录结构..."
echo ""

if [ -d "rust" ]; then
    echo -e "${GREEN}✅ rust/ 目录存在${NC}"
    if [ -f "rust/Cargo.toml" ]; then
        echo -e "${GREEN}✅ rust/Cargo.toml 存在${NC}"
    else
        echo -e "${RED}❌ rust/Cargo.toml 不存在${NC}"
        ((ERRORS++))
    fi
else
    echo -e "${RED}❌ rust/ 目录不存在${NC}"
    ((ERRORS++))
fi

if [ -d "web/admin-spa" ]; then
    echo -e "${GREEN}✅ web/admin-spa/ 目录存在${NC}"
    if [ -f "web/admin-spa/package.json" ]; then
        echo -e "${GREEN}✅ web/admin-spa/package.json 存在${NC}"
    else
        echo -e "${RED}❌ web/admin-spa/package.json 不存在${NC}"
        ((ERRORS++))
    fi
else
    echo -e "${RED}❌ web/admin-spa/ 目录不存在${NC}"
    ((ERRORS++))
fi

if [ -d "nodejs-archive" ]; then
    echo -e "${GREEN}✅ nodejs-archive/ 目录存在（归档）${NC}"
else
    echo -e "${YELLOW}⚠️  nodejs-archive/ 目录不存在（非必需）${NC}"
    ((WARNINGS++))
fi

echo ""

# 3. 检查配置文件
echo "⚙️  检查配置文件..."
echo ""

if [ -f ".env.example" ]; then
    echo -e "${GREEN}✅ .env.example 存在${NC}"
else
    echo -e "${RED}❌ .env.example 不存在${NC}"
    ((ERRORS++))
fi

if [ -f ".env" ]; then
    echo -e "${GREEN}✅ .env 文件存在${NC}"

    # 检查必填环境变量
    if grep -q "CRS_SECURITY__ENCRYPTION_KEY=.\{32\}" .env; then
        echo -e "${GREEN}✅ ENCRYPTION_KEY 已设置（32字符）${NC}"
    else
        echo -e "${RED}❌ ENCRYPTION_KEY 未设置或不是32字符${NC}"
        echo "   请在 .env 中设置: CRS_SECURITY__ENCRYPTION_KEY=12345678901234567890123456789012"
        ((ERRORS++))
    fi

    if grep -q "CRS_SECURITY__JWT_SECRET=.\{32,\}" .env; then
        echo -e "${GREEN}✅ JWT_SECRET 已设置（>=32字符）${NC}"
    else
        echo -e "${YELLOW}⚠️  JWT_SECRET 未设置或少于32字符${NC}"
        ((WARNINGS++))
    fi
else
    echo -e "${YELLOW}⚠️  .env 文件不存在，将使用默认值${NC}"
    echo "   建议运行: cp .env.example .env"
    ((WARNINGS++))
fi

if [ -f "Dockerfile" ]; then
    echo -e "${GREEN}✅ Dockerfile 存在${NC}"
else
    echo -e "${RED}❌ Dockerfile 不存在${NC}"
    ((ERRORS++))
fi

if [ -f "docker-compose.yml" ]; then
    echo -e "${GREEN}✅ docker-compose.yml 存在${NC}"
else
    echo -e "${RED}❌ docker-compose.yml 不存在${NC}"
    ((ERRORS++))
fi

echo ""

# 4. 检查 Redis
echo "📊 检查 Redis..."
echo ""

if docker ps | grep -q redis-dev; then
    echo -e "${GREEN}✅ Redis 容器正在运行${NC}"

    if redis-cli ping &> /dev/null; then
        echo -e "${GREEN}✅ Redis 连接成功${NC}"
    else
        echo -e "${RED}❌ Redis 无法连接${NC}"
        ((ERRORS++))
    fi
elif docker ps -a | grep -q redis-dev; then
    echo -e "${YELLOW}⚠️  Redis 容器存在但未运行${NC}"
    echo "   启动命令: docker start redis-dev"
    ((WARNINGS++))
else
    echo -e "${YELLOW}⚠️  Redis 容器不存在${NC}"
    echo "   创建命令: docker run -d --name redis-dev -p 6379:6379 redis:7-alpine"
    ((WARNINGS++))
fi

echo ""

# 5. 检查端口占用
echo "🔌 检查端口占用..."
echo ""

check_port() {
    if lsof -Pi :$1 -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  端口 $1 已被占用${NC}"
        lsof -i :$1 | grep LISTEN
        ((WARNINGS++))
    else
        echo -e "${GREEN}✅ 端口 $1 可用${NC}"
    fi
}

check_port 8080  # Rust 后端
check_port 3001  # 前端
check_port 6379  # Redis

echo ""

# 6. 检查 Rust 项目
echo "🦀 检查 Rust 项目..."
echo ""

cd rust/

if cargo check --quiet &> /dev/null; then
    echo -e "${GREEN}✅ Rust 项目编译通过${NC}"
else
    echo -e "${RED}❌ Rust 项目编译失败${NC}"
    echo "   运行 'cd rust/ && cargo check' 查看详细错误"
    ((ERRORS++))
fi

cd ..

echo ""

# 7. 检查前端项目
echo "🎨 检查前端项目..."
echo ""

cd web/admin-spa/

if [ -d "node_modules" ]; then
    echo -e "${GREEN}✅ 前端依赖已安装${NC}"
else
    echo -e "${YELLOW}⚠️  前端依赖未安装${NC}"
    echo "   安装命令: cd web/admin-spa/ && npm install"
    ((WARNINGS++))
fi

cd ../..

echo ""

# 8. 检查文档
echo "📚 检查文档..."
echo ""

DOCS=(
    "README.md"
    "MIGRATION.md"
    "LOCAL_DEBUG_GUIDE.md"
    "rust/README.md"
    "docs/INTERFACE.md"
)

for doc in "${DOCS[@]}"; do
    if [ -f "$doc" ]; then
        echo -e "${GREEN}✅ $doc 存在${NC}"
    else
        echo -e "${YELLOW}⚠️  $doc 不存在${NC}"
        ((WARNINGS++))
    fi
done

echo ""

# 总结
echo "======================================"
echo "📊 验证总结"
echo "======================================"
echo ""

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}🎉 完美！所有检查通过！${NC}"
    echo ""
    echo "✅ 环境已准备就绪，可以开始开发"
    echo ""
    echo "🚀 快速启动:"
    echo "   bash start-dev.sh"
    echo ""
    echo "或手动启动:"
    echo "   1. docker start redis-dev  # 启动 Redis"
    echo "   2. cd rust/ && cargo run   # 启动 Rust 后端"
    echo "   3. cd web/admin-spa/ && npm run dev  # 启动前端"
    exit 0
elif [ $ERRORS -eq 0 ]; then
    echo -e "${YELLOW}⚠️  发现 $WARNINGS 个警告${NC}"
    echo ""
    echo "环境基本可用，但建议修复上述警告"
    echo ""
    echo "🚀 可以尝试启动:"
    echo "   bash start-dev.sh"
    exit 0
else
    echo -e "${RED}❌ 发现 $ERRORS 个错误和 $WARNINGS 个警告${NC}"
    echo ""
    echo "请修复上述错误后再启动服务"
    echo ""
    echo "📖 参考文档:"
    echo "   - LOCAL_DEBUG_GUIDE.md - 完整本地调试指南"
    echo "   - MIGRATION.md - 迁移和故障排查"
    exit 1
fi
