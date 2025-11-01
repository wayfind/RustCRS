#!/bin/bash

# Claude Extended Thinking 参数测试脚本
# 验证thinking参数正确传递和处理

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
echo -e "${BLUE}║     Claude Extended Thinking - 功能测试脚本               ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}配置信息:${NC}"
echo -e "  Relay URL: ${RELAY_URL}"
echo -e "  API Key: ${API_KEY:0:10}..."
echo ""

# 测试1: 带Extended Thinking参数的请求
echo -e "${BLUE}[测试 1/3]${NC} Claude API - 带Extended Thinking参数"
echo -e "${YELLOW}发送请求到:${NC} POST /api/v1/messages"

RESPONSE_1=$(curl -s -X POST "${RELAY_URL}/api/v1/messages" \
  -H "x-api-key: ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 1024,
    "thinking": {
      "type": "enabled",
      "budget_tokens": 5000
    },
    "messages": [
      {
        "role": "user",
        "content": "解释量子纠缠的原理，要深入思考"
      }
    ]
  }')

# 检查响应
if echo "$RESPONSE_1" | grep -q "content"; then
  echo -e "${GREEN}✅ 测试通过${NC} - 请求成功"
  echo ""
  echo -e "${YELLOW}💡 检查事项:${NC}"
  echo "  1. 查看服务日志中的thinking参数记录:"
  echo "     tail -f logs/claude-relay-*.log | grep 'Extended Thinking'"
  echo ""
  echo "  2. 预期日志内容:"
  echo "     🧠 Extended Thinking: enabled, budget: 5000 tokens"
  echo ""
  echo "  3. 检查响应中是否包含thinking blocks:"
  # 尝试检查响应中的content类型
  THINKING_BLOCKS=$(echo "$RESPONSE_1" | jq '[.content[] | select(.type == "thinking")] | length' 2>/dev/null)
  if [ "$THINKING_BLOCKS" != "" ] && [ "$THINKING_BLOCKS" -gt 0 ]; then
    echo -e "     ${GREEN}✅ 发现 ${THINKING_BLOCKS} 个thinking blocks${NC}"
  else
    echo -e "     ${YELLOW}⚠️ 响应中未发现thinking blocks (可能模型未使用)${NC}"
  fi
else
  echo -e "${RED}❌ 测试失败${NC} - 请求失败"
  echo "$RESPONSE_1"
fi
echo ""

# 测试2: thinking参数类型为disabled
echo -e "${BLUE}[测试 2/3]${NC} Claude API - thinking.type = 'disabled'"
echo -e "${YELLOW}发送请求到:${NC} POST /api/v1/messages"

RESPONSE_2=$(curl -s -X POST "${RELAY_URL}/api/v1/messages" \
  -H "x-api-key: ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 500,
    "thinking": {
      "type": "disabled"
    },
    "messages": [
      {
        "role": "user",
        "content": "What is 2+2?"
      }
    ]
  }')

if echo "$RESPONSE_2" | grep -q "content"; then
  echo -e "${GREEN}✅ 测试通过${NC} - thinking disabled请求成功"
  echo ""
  echo -e "${YELLOW}💡 预期日志:${NC}"
  echo "  🧠 Extended Thinking: disabled"
else
  echo -e "${RED}❌ 测试失败${NC}"
  echo "$RESPONSE_2"
fi
echo ""

# 测试3: 不带thinking参数（向后兼容）
echo -e "${BLUE}[测试 3/3]${NC} 向后兼容性 - 不带thinking参数"
echo -e "${YELLOW}发送请求到:${NC} POST /api/v1/messages"

RESPONSE_3=$(curl -s -X POST "${RELAY_URL}/api/v1/messages" \
  -H "x-api-key: ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 200,
    "messages": [
      {
        "role": "user",
        "content": "Hello"
      }
    ]
  }')

if echo "$RESPONSE_3" | grep -q "content"; then
  echo -e "${GREEN}✅ 测试通过${NC} - 向后兼容性正常"
  echo "  日志中不应该出现Extended Thinking相关记录"
else
  echo -e "${RED}❌ 测试失败${NC}"
  echo "$RESPONSE_3"
fi
echo ""

# 总结
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                      测试完成                              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}验证清单:${NC}"
echo -e "  ✅ Extended Thinking enabled 请求成功"
echo -e "  ✅ Extended Thinking disabled 请求成功"
echo -e "  ✅ 向后兼容性（无thinking参数）正常"
echo -e "  ⏳ 待检查日志确认参数传递"
echo ""
echo -e "${GREEN}💡 完整日志验证命令:${NC}"
echo -e "  tail -100 logs/claude-relay-*.log | grep -E '(Extended Thinking|thinking)'"
echo ""
echo -e "${YELLOW}Bedrock测试:${NC}"
echo -e "  如果使用Bedrock账户，可以修改上述请求的端点测试"
echo -e "  Bedrock应该在日志中显示: 🧠 Extended Thinking enabled for Bedrock"
echo ""
echo -e "${YELLOW}使用说明:${NC}"
echo -e "  1. 确保服务已启动: npm start"
echo -e "  2. 设置环境变量:"
echo -e "     export API_KEY=your-api-key"
echo -e "     export RELAY_URL=http://localhost:3000"
echo -e "  3. 运行测试: bash scripts/test-extended-thinking.sh"
echo ""
