#!/bin/bash

# Gemini Tools功能测试脚本
# 用于验证Gemini CLI转发服务的Tool Calling支持

set -e

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置
RELAY_URL="${RELAY_URL:-http://localhost:3000}"
API_KEY="${GEMINI_API_KEY:-your-api-key}"
MODEL="${MODEL:-gemini-2.5-pro}"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        Gemini Tools Support - 功能测试脚本                ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}配置信息:${NC}"
echo -e "  Relay URL: ${RELAY_URL}"
echo -e "  Model: ${MODEL}"
echo -e "  API Key: ${API_KEY:0:10}..."
echo ""

# 测试1: 标准Gemini API格式（带Tools）
echo -e "${BLUE}[测试 1/4]${NC} 标准Gemini API - 带Tools参数"
echo -e "${YELLOW}发送请求到:${NC} POST /gemini/v1/models/${MODEL}:generateContent"

RESPONSE_1=$(curl -s -X POST "${RELAY_URL}/gemini/v1/models/${MODEL}:generateContent" \
  -H "x-api-key: ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{
      "role": "user",
      "parts": [{"text": "请使用create_directory工具创建一个名为test_dir的目录"}]
    }],
    "tools": [{
      "functionDeclarations": [{
        "name": "create_directory",
        "description": "Create a new directory",
        "parameters": {
          "type": "object",
          "properties": {
            "path": {
              "type": "string",
              "description": "Directory path to create"
            }
          },
          "required": ["path"]
        }
      }]
    }]
  }')

if echo "$RESPONSE_1" | grep -q "tool_calls\|functionCall"; then
  echo -e "${GREEN}✅ 测试通过${NC} - 响应包含tool_calls"
  echo "$RESPONSE_1" | jq '.choices[0].message.tool_calls // .choices[0].message' 2>/dev/null || echo "$RESPONSE_1"
else
  echo -e "${RED}❌ 测试失败${NC} - 响应不包含tool_calls"
  echo "$RESPONSE_1"
fi
echo ""

# 测试2: Gemini CLI内部API格式（带Tools）
echo -e "${BLUE}[测试 2/4]${NC} Gemini CLI内部API - generateContent带Tools"
echo -e "${YELLOW}发送请求到:${NC} POST /gemini/v1internal:generateContent"

RESPONSE_2=$(curl -s -X POST "${RELAY_URL}/gemini/v1internal:generateContent" \
  -H "x-api-key: ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "models/'${MODEL}'",
    "request": {
      "contents": [{
        "role": "user",
        "parts": [{"text": "使用write_file工具写一个hello.txt文件，内容是Hello World"}]
      }],
      "tools": [{
        "functionDeclarations": [{
          "name": "write_file",
          "description": "Write content to a file",
          "parameters": {
            "type": "object",
            "properties": {
              "filename": {"type": "string"},
              "content": {"type": "string"}
            },
            "required": ["filename", "content"]
          }
        }]
      }]
    }
  }')

if echo "$RESPONSE_2" | grep -q "tool_calls\|functionCall\|write_file"; then
  echo -e "${GREEN}✅ 测试通过${NC} - 响应包含工具调用"
  echo "$RESPONSE_2" | jq '.response // .' 2>/dev/null || echo "$RESPONSE_2"
else
  echo -e "${RED}❌ 测试失败${NC} - 响应不包含工具调用"
  echo "$RESPONSE_2"
fi
echo ""

# 测试3: OpenAI兼容格式（转换为Gemini）
echo -e "${BLUE}[测试 3/4]${NC} OpenAI兼容格式 - Tools转换测试"
echo -e "${YELLOW}发送请求到:${NC} POST /gemini/messages"

RESPONSE_3=$(curl -s -X POST "${RELAY_URL}/gemini/messages" \
  -H "x-api-key: ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "'${MODEL}'",
    "messages": [{
      "role": "user",
      "content": "列出当前目录的文件"
    }],
    "tools": [{
      "type": "function",
      "function": {
        "name": "list_files",
        "description": "List files in a directory",
        "parameters": {
          "type": "object",
          "properties": {
            "path": {"type": "string", "default": "."}
          }
        }
      }
    }]
  }')

if echo "$RESPONSE_3" | grep -q "tool_calls\|list_files"; then
  echo -e "${GREEN}✅ 测试通过${NC} - OpenAI格式转换成功"
  echo "$RESPONSE_3" | jq '.' 2>/dev/null || echo "$RESPONSE_3"
else
  echo -e "${RED}❌ 测试失败${NC} - 格式转换失败"
  echo "$RESPONSE_3"
fi
echo ""

# 测试4: 不带Tools的请求（向后兼容性）
echo -e "${BLUE}[测试 4/4]${NC} 向后兼容性 - 不带Tools的普通请求"
echo -e "${YELLOW}发送请求到:${NC} POST /gemini/v1/models/${MODEL}:generateContent"

RESPONSE_4=$(curl -s -X POST "${RELAY_URL}/gemini/v1/models/${MODEL}:generateContent" \
  -H "x-api-key: ${API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{
      "role": "user",
      "parts": [{"text": "Hello, 请回复一个简短的问候"}]
    }]
  }')

if echo "$RESPONSE_4" | grep -q "choices\|candidates"; then
  echo -e "${GREEN}✅ 测试通过${NC} - 向后兼容性正常"
  echo "$RESPONSE_4" | jq '.choices[0].message.content // .candidates[0].content' 2>/dev/null || echo "$RESPONSE_4"
else
  echo -e "${RED}❌ 测试失败${NC} - 基本请求失败"
  echo "$RESPONSE_4"
fi
echo ""

# 总结
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                      测试完成                              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}使用说明:${NC}"
echo -e "  1. 确保服务已启动: npm start"
echo -e "  2. 设置环境变量:"
echo -e "     export GEMINI_API_KEY=your-api-key"
echo -e "     export RELAY_URL=http://localhost:3000"
echo -e "  3. 运行测试: bash scripts/test-gemini-tools.sh"
echo ""
echo -e "${GREEN}💡 提示:${NC} 如果所有测试通过，说明Gemini Tools功能已正常工作"
echo ""
