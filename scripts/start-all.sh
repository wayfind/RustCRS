#!/bin/bash
# 统一启动全部服务的脚本

set -e

MODE=${1:-dev}  # dev 或 release

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}🚀 Claude Relay Service - 统一启动 (${MODE} 模式)${NC}"
echo ""

# 1. 启动 Redis
echo -e "${GREEN}步骤 1/3: 启动 Redis${NC}"
bash scripts/init-redis.sh
echo ""

# 2. 确定启动方式（支持环境变量和非交互模式）
echo -e "${GREEN}步骤 2/3: 启动 Rust 后端${NC}"
echo ""

# 检测是否为交互式终端
IS_INTERACTIVE=false
if [ -t 0 ]; then
    IS_INTERACTIVE=true
fi

# 获取后端启动模式（优先级：环境变量 > 交互输入 > 默认值）
if [ -n "$BACKEND_MODE" ]; then
    # 使用环境变量
    if [ "$BACKEND_MODE" = "background" ] || [ "$BACKEND_MODE" = "2" ]; then
        backend_choice="2"
    else
        backend_choice="1"
    fi
    echo -e "${YELLOW}ℹ️  使用环境变量 BACKEND_MODE=$BACKEND_MODE${NC}"
elif [ "$IS_INTERACTIVE" = true ]; then
    # 交互模式：询问用户
    echo "请选择后端启动方式："
    echo "  1) 前台运行（推荐，便于查看日志）"
    echo "  2) 后台运行（守护进程模式）"
    echo ""
    read -p "请选择 [1/2]: " backend_choice
else
    # 非交互模式：使用默认值（后台运行）
    backend_choice="2"
    echo -e "${YELLOW}ℹ️  非交互模式，使用默认设置：后台运行${NC}"
    echo -e "${YELLOW}   提示: 可通过环境变量 BACKEND_MODE=foreground|background 控制${NC}"
fi
echo ""

if [ "$backend_choice" = "2" ]; then
    echo -e "${GREEN}🔧 后台启动 Rust 后端...${NC}"
    nohup bash scripts/start-backend.sh $MODE > logs/backend.log 2>&1 &
    BACKEND_PID=$!
    echo -e "${GREEN}✅ 后端已在后台启动 (PID: $BACKEND_PID)${NC}"
    echo -e "${YELLOW}   查看日志: tail -f logs/backend.log${NC}"
    echo ""
    sleep 3

    # 3. 确定是否启动前端（支持环境变量和非交互模式）
    echo -e "${GREEN}步骤 3/3: 启动前端界面${NC}"

    # 获取前端启动选择（优先级：环境变量 > 交互输入 > 默认值）
    if [ -n "$START_FRONTEND" ]; then
        # 使用环境变量
        start_frontend="$START_FRONTEND"
        echo -e "${YELLOW}ℹ️  使用环境变量 START_FRONTEND=$START_FRONTEND${NC}"
    elif [ "$IS_INTERACTIVE" = true ]; then
        # 交互模式：询问用户
        read -p "是否启动前端界面? [Y/n]: " start_frontend
    else
        # 非交互模式：默认不启动前端（因为前端需要前台运行）
        start_frontend="n"
        echo -e "${YELLOW}ℹ️  非交互模式，默认不启动前端${NC}"
        echo -e "${YELLOW}   提示: 可通过环境变量 START_FRONTEND=yes|no 控制${NC}"
    fi

    if [[ "$start_frontend" =~ ^[Nn]o?$ ]] || [[ "$start_frontend" == "no" ]] || [[ "$start_frontend" == "NO" ]] || [[ "$start_frontend" == "No" ]]; then
        echo -e "${GREEN}✅ 服务启动完成！${NC}"
        echo ""
        echo -e "${GREEN}🎉 所有服务已准备就绪！${NC}"
        echo ""
        echo "📝 手动启动前端："
        echo "   cd web/admin-spa && npm run dev"
        echo ""
        echo "🌐 访问地址："
        echo "   - API: http://localhost:8080"
        echo "   - 健康检查: curl http://localhost:8080/health"
        echo ""
        echo "📊 监控："
        echo "   - 后端日志: tail -f logs/backend.log"
        echo "   - 停止后端: kill $BACKEND_PID"
    else
        echo -e "${YELLOW}ℹ️  前端将在前台运行，按 Ctrl+C 停止${NC}"
        sleep 2
        bash scripts/start-frontend.sh
    fi
else
    echo -e "${YELLOW}ℹ️  后端将在前台运行${NC}"
    echo -e "${YELLOW}ℹ️  请在另一个终端运行前端: make start-frontend 或 bash scripts/start-frontend.sh${NC}"
    echo ""
    sleep 2
    bash scripts/start-backend.sh $MODE
fi
