#!/bin/bash
# 前端启动脚本

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}🎨 启动前端界面${NC}"

# 进入前端目录
cd web/admin-spa/

# 检查依赖是否已安装
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}📦 首次运行，安装依赖...${NC}"
    npm install
fi

# 启动开发服务器
echo -e "${GREEN}🚀 启动 Vite 开发服务器${NC}"
echo -e "${YELLOW}ℹ️  前端将在浏览器自动打开: http://localhost:3001${NC}"
echo -e "${YELLOW}ℹ️  确保后端已在 http://localhost:8080 运行${NC}"

npm run dev
