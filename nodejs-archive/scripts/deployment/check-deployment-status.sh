#!/bin/bash

# 部署状态检查脚本
# Check Deployment Status Script

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 图标
SUCCESS="✅"
FAIL="❌"
PENDING="⏳"
INFO="ℹ️"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Claude Relay Service 部署状态检查${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# 自动检测 GitHub 用户名和 Docker 用户名
# 优先级: 环境变量 > 从 Git remote 检测
detect_github_user() {
    if [ -n "$GITHUB_USER" ]; then
        echo "$GITHUB_USER"
        return
    fi

    # 从 Git remote URL 提取用户名
    if [ -d ".git" ]; then
        local remote_url=$(git config --get remote.origin.url 2>/dev/null)
        if [[ "$remote_url" =~ github\.com[:/]([^/]+)/claude-relay-service ]]; then
            echo "${BASH_REMATCH[1]}"
            return
        fi
    fi

    echo ""
}

# 检查 1: GitHub Actions 状态
echo -e "${YELLOW}${INFO} 检查 GitHub Actions 状态...${NC}"
GITHUB_USER=$(detect_github_user)
DOCKER_USER="${DOCKER_USER:-$GITHUB_USER}"

if [ -z "$GITHUB_USER" ]; then
    echo -e "${RED}${FAIL} 无法检测 GitHub 用户名。请设置 GITHUB_USER 环境变量。${NC}"
    exit 1
fi
echo -e "${INFO} 访问: ${BLUE}https://github.com/${GITHUB_USER}/claude-relay-service/actions${NC}"
echo ""

# 检查 2: GitHub Release
echo -e "${YELLOW}${INFO} 检查 GitHub Release...${NC}"
echo -e "${INFO} 访问: ${BLUE}https://github.com/${GITHUB_USER}/claude-relay-service/releases${NC}"
echo ""

# 检查 3: Docker Hub 镜像
echo -e "${YELLOW}${INFO} 检查 Docker Hub 镜像...${NC}"
DOCKER_IMAGE="${DOCKER_USER}/claude-relay-service"
if docker pull ${DOCKER_IMAGE}:latest >/dev/null 2>&1; then
    echo -e "${GREEN}${SUCCESS} Docker Hub 镜像可拉取${NC}"
    docker images ${DOCKER_IMAGE} --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}" | head -5
else
    echo -e "${RED}${FAIL} Docker Hub 镜像拉取失败${NC}"
fi
echo ""

# 检查 4: GHCR 镜像
echo -e "${YELLOW}${INFO} 检查 GHCR 镜像...${NC}"
GHCR_IMAGE="ghcr.io/${GITHUB_USER}/claude-relay-service"
if docker pull ${GHCR_IMAGE}:latest >/dev/null 2>&1; then
    echo -e "${GREEN}${SUCCESS} GHCR 镜像可拉取${NC}"
    docker images ${GHCR_IMAGE} --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}" | head -5
else
    echo -e "${YELLOW}${PENDING} GHCR 镜像可能需要认证${NC}"
fi
echo ""

# 检查 5: 本地服务状态
echo -e "${YELLOW}${INFO} 检查本地服务状态...${NC}"
if docker ps | grep -q claude-relay-service; then
    echo -e "${GREEN}${SUCCESS} 服务容器正在运行${NC}"
    docker ps --filter "name=claude-relay-service" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

    # 健康检查
    echo ""
    echo -e "${YELLOW}${INFO} 执行健康检查...${NC}"
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/health | grep -q "200"; then
        echo -e "${GREEN}${SUCCESS} 健康检查通过${NC}"

        # 显示版本信息
        echo ""
        echo -e "${YELLOW}${INFO} 版本信息:${NC}"
        curl -s http://localhost:3000/health | jq -r '.version // "未知"' || echo "无法获取版本"
    else
        echo -e "${RED}${FAIL} 健康检查失败${NC}"
    fi
else
    echo -e "${YELLOW}${PENDING} 本地服务未运行${NC}"
fi
echo ""

# 检查 6: 日志
echo -e "${YELLOW}${INFO} 最近日志（最后 10 行）:${NC}"
if [ -f "logs/claude-relay-error.log" ]; then
    echo -e "${INFO} 错误日志:"
    tail -n 10 logs/claude-relay-error.log 2>/dev/null || echo "  无错误日志"
else
    echo -e "${GREEN}${SUCCESS} 无错误日志文件${NC}"
fi
echo ""

# 检查 7: Git 状态
echo -e "${YELLOW}${INFO} Git 状态:${NC}"
git log --oneline -1
echo -e "${INFO} 当前分支: ${BLUE}$(git branch --show-current)${NC}"
echo -e "${INFO} 最新 tag: ${BLUE}$(git describe --tags --abbrev=0 2>/dev/null || echo "无 tag")${NC}"
echo ""

# 总结
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  检查完成${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${INFO} 详细部署验证请参考: ${BLUE}DEPLOYMENT_CHECKLIST.md${NC}"
echo -e "${INFO} 部署状态报告: ${BLUE}DEPLOYMENT_STATUS.md${NC}"
echo ""

# 快速测试命令
echo -e "${YELLOW}${INFO} 快速测试命令:${NC}"
echo -e "  健康检查: ${BLUE}curl http://localhost:3000/health${NC}"
echo -e "  系统指标: ${BLUE}curl http://localhost:3000/metrics${NC}"
echo -e "  Web 界面: ${BLUE}http://localhost:3000/admin-next/${NC}"
echo ""
echo -e "  Gemini Tools 测试: ${BLUE}bash scripts/test-gemini-tools.sh${NC}"
echo -e "  User 字段测试: ${BLUE}bash scripts/test-openai-user-field.sh${NC}"
echo -e "  Extended Thinking 测试: ${BLUE}bash scripts/test-extended-thinking.sh${NC}"
echo ""
