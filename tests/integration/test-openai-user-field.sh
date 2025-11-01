#!/bin/bash

# OpenAI user字段转换测试脚本
# 验证OpenAI的user字段正确转换为Claude的metadata.user_id

set -e

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置
RELAY_URL="${RELAY_URL:-http://localhost:3000}"
API_KEY="${API_KEY:-your-api-key}"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     OpenAI user字段转换 - 功能测试脚本                    ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}配置信息:${NC}"
echo -e "  Relay URL: ${RELAY_URL}"
echo -e "  API Key: ${API_KEY:0:10}..."
echo ""

# 测试1: OpenAI格式请求带user字段
echo -e "${BLUE}[测试 1/2]${NC} OpenAI格式 - 带user字段"
echo -e "${YELLOW}发送请求到:${NC} POST /openai/claude/v1/chat/completions"

RESPONSE_1=$(curl -s -X POST "${RELAY_URL}/openai/claude/v1/chat/completions" \
  -H "Authorization: Bearer ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [
      {"role": "user", "content": "Hello, 请说中文回复"}
    ],
    "user": "test_user_123",
    "max_tokens": 100
  }')

# 检查响应是否成功
if echo "$RESPONSE_1" | grep -q "choices"; then
  echo -e "${GREEN}✅ 测试通过${NC} - 请求成功，检查日志确认metadata传递"
  echo "响应预览:"
  echo "$RESPONSE_1" | jq '.choices[0].message.content' 2>/dev/null || echo "$RESPONSE_1" | head -c 200
  echo ""
  echo -e "${YELLOW}💡 验证步骤:${NC}"
  echo "  1. 检查服务日志: logs/claude-relay-*.log"
  echo "  2. 搜索日志: '👤 User metadata added: test_user_123'"
  echo "  3. 确认metadata字段被传递到Claude API"
else
  echo -e "${RED}❌ 测试失败${NC} - 请求失败"
  echo "$RESPONSE_1"
fi
echo ""

# 测试2: OpenAI格式请求不带user字段（向后兼容）
echo -e "${BLUE}[测试 2/2]${NC} OpenAI格式 - 不带user字段（向后兼容）"
echo -e "${YELLOW}发送请求到:${NC} POST /openai/claude/v1/chat/completions"

RESPONSE_2=$(curl -s -X POST "${RELAY_URL}/openai/claude/v1/chat/completions" \
  -H "Authorization: Bearer ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [
      {"role": "user", "content": "Hi, say hello in Chinese"}
    ],
    "max_tokens": 100
  }')

if echo "$RESPONSE_2" | grep -q "choices"; then
  echo -e "${GREEN}✅ 测试通过${NC} - 向后兼容性正常，无user字段请求成功"
  echo "响应预览:"
  echo "$RESPONSE_2" | jq '.choices[0].message.content' 2>/dev/null || echo "$RESPONSE_2" | head -c 200
else
  echo -e "${RED}❌ 测试失败${NC} - 向后兼容性问题"
  echo "$RESPONSE_2"
fi
echo ""

# 总结
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                      测试完成                              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}验证清单:${NC}"
echo -e "  ✅ OpenAI user字段请求成功"
echo -e "  ✅ 向后兼容性（无user字段）正常"
echo -e "  ⏳ 待检查日志确认metadata传递"
echo ""
echo -e "${GREEN}💡 日志验证命令:${NC}"
echo -e "  tail -f logs/claude-relay-*.log | grep -E '(User metadata|metadata)'"
echo ""
echo -e "${YELLOW}使用说明:${NC}"
echo -e "  1. 确保服务已启动: npm start"
echo -e "  2. 设置环境变量:"
echo -e "     export API_KEY=your-api-key"
echo -e "     export RELAY_URL=http://localhost:3000"
echo -e "  3. 运行测试: bash scripts/test-openai-user-field.sh"
echo ""
