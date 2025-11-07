#!/bin/bash

################################################################################
# Claude Console E2E 回归测试脚本
#
# 使用方法:
#   bash test-claudeconsole-e2e.sh [测试时长(秒)]
#
# 示例:
#   bash test-claudeconsole-e2e.sh 60    # 1分钟测试
#   bash test-claudeconsole-e2e.sh 300   # 5分钟测试
#   bash test-claudeconsole-e2e.sh       # 默认5分钟
#
# 注意: 该脚本会自动调用 setup-test-data.sh 创建测试数据
################################################################################

set -e  # Exit on error

# ============================================================================
# 自动化测试数据设置
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CREDENTIALS_FILE="${SCRIPT_DIR}/../.test-credentials"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║   自动化测试数据准备                                       ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# 检查凭据文件是否存在且新鲜（1小时内）
SETUP_NEEDED=true
if [ -f "${CREDENTIALS_FILE}" ]; then
    # 检查文件年龄
    if [ "$(uname)" == "Darwin" ]; then
        FILE_AGE=$(($(date +%s) - $(stat -f %m "${CREDENTIALS_FILE}")))
    else
        FILE_AGE=$(($(date +%s) - $(stat -c %Y "${CREDENTIALS_FILE}")))
    fi

    if [ ${FILE_AGE} -lt 3600 ]; then
        echo "✓ 发现新鲜的测试凭据文件（${FILE_AGE} 秒前生成）"
        SETUP_NEEDED=false
    else
        echo "⚠ 测试凭据文件已过期（${FILE_AGE} 秒前生成），重新生成..."
    fi
else
    echo "⚠ 测试凭据文件不存在，需要生成..."
fi

# 如果需要，运行 setup 脚本
if [ "${SETUP_NEEDED}" = true ]; then
    echo ""
    echo "→ 运行测试数据设置脚本..."
    echo "" | bash "${SCRIPT_DIR}/setup-test-data.sh"

    if [ $? -ne 0 ]; then
        echo "✗ 测试数据设置失败"
        exit 1
    fi
    echo ""
fi

# 加载测试凭据
if [ ! -f "${CREDENTIALS_FILE}" ]; then
    echo "✗ 测试凭据文件不存在: ${CREDENTIALS_FILE}"
    exit 1
fi

source "${CREDENTIALS_FILE}"

# 验证必需的环境变量
if [ -z "${TEST_API_KEY}" ] || [ "${TEST_API_KEY}" == "<existing-key-value-unavailable>" ]; then
    echo "✗ TEST_API_KEY 不可用"
    echo "  请删除现有的测试 API Key 并重新运行 setup-test-data.sh"
    echo "  或手动设置 TEST_API_KEY 环境变量"
    exit 1
fi

if [ -z "${BASE_URL}" ]; then
    echo "✗ BASE_URL 未设置"
    exit 1
fi

echo "✓ 测试凭据加载成功"
echo ""

# ============================================================================
# 测试配置 - 使用自动化凭据
# ============================================================================

# 使用自动生成的 API Key
ANTHROPIC_AUTH_TOKEN="${TEST_API_KEY}"

# 本地 Rust 后端配置
LOCAL_ENDPOINT="${BASE_URL}/api/v1/messages"

# 测试参数
TEST_DURATION=${1:-300}  # 测试时长（秒），默认300秒（5分钟）
REQUEST_INTERVAL=3       # 请求间隔（秒）
MODEL="claude-3-5-sonnet-20241022"
MAX_TOKENS=100

# ============================================================================
# E2E 测试开始
# ============================================================================

echo "╔════════════════════════════════════════════════════════════╗"
echo "║   Claude Console E2E 回归测试                              ║"
echo "╚════════════════════════════════════════════════════════════╝"

echo ""
echo "📋 测试配置:"
echo "  - API Key: ${ANTHROPIC_AUTH_TOKEN:0:20}...${ANTHROPIC_AUTH_TOKEN: -10}"
echo "  - Key ID: ${TEST_API_KEY_ID:-N/A}"
echo "  - 本地端点: $LOCAL_ENDPOINT"
echo "  - 模型: $MODEL"
echo "  - 测试时长: $TEST_DURATION 秒 ($(awk "BEGIN {printf \"%.1f\", $TEST_DURATION/60}") 分钟)"
echo "  - 请求间隔: $REQUEST_INTERVAL 秒"
echo "  - 预计请求数: $(awk "BEGIN {printf \"%.0f\", $TEST_DURATION/$REQUEST_INTERVAL}")"
echo ""

# 检查后端是否运行
echo "🔍 检查后端状态..."
if curl -s -f http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ 后端服务正常运行"
else
    echo "❌ 后端服务未运行，请先启动: make rust-dev"
    exit 1
fi

# 检查是否需要通过本地后端还是直接测试
read -p "是否通过本地 Rust 后端测试？(Y/n): " USE_LOCAL
USE_LOCAL=${USE_LOCAL:-Y}

if [[ "$USE_LOCAL" =~ ^[Yy]$ ]]; then
    echo "🔄 将通过本地 Rust 后端进行测试"
    ENDPOINT=$LOCAL_ENDPOINT

    # 检查是否已有绑定此 session_token 的 API Key
    echo "🔍 检查现有 API Key..."
    # 这里需要从 Redis 或管理界面创建 API Key
    read -p "请输入测试用 API Key (如果已创建): " EXISTING_API_KEY

    if [ -z "$EXISTING_API_KEY" ]; then
        echo "⚠️  需要先创建 API Key 并绑定到 Claude Console 账户"
        echo "   1. 访问 http://localhost:8080/admin-next"
        echo "   2. 添加 Claude Console 账户（使用上述 session_token）"
        echo "   3. 创建 API Key 并绑定到该账户"
        echo "   4. 重新运行此脚本并输入 API Key"
        exit 1
    fi

    AUTH_HEADER="Authorization: Bearer $EXISTING_API_KEY"
else
    echo "🌐 将直接测试 Claude Console 端点"
    ENDPOINT="$ANTHROPIC_BASE_URL/messages"
    AUTH_HEADER="x-api-key: $ANTHROPIC_AUTH_TOKEN"
fi

echo ""
echo "🧪 开始测试..."
echo "开始时间: $(date)"
echo "════════════════════════════════════════════════════════════"

# 统计变量
start_time=$(date +%s)
request_count=0
success_count=0
error_count=0
total_input_tokens=0
total_output_tokens=0

# 创建日志目录
mkdir -p logs

# 主测试循环
while true; do
    current_time=$(date +%s)
    elapsed=$((current_time - start_time))

    if [ $elapsed -ge $TEST_DURATION ]; then
        break
    fi

    request_count=$((request_count + 1))
    remaining=$((TEST_DURATION - elapsed))

    echo ""
    echo "┌─────────────────────────────────────────────────────────┐"
    echo "│ 请求 #$request_count"
    echo "│ 时间: $(date +%H:%M:%S)"
    echo "│ 已运行: ${elapsed}s / 剩余: ${remaining}s"
    echo "└─────────────────────────────────────────────────────────┘"

    # 生成随机问题使每个请求不同
    num1=$((RANDOM % 100 + 1))
    num2=$((RANDOM % 100 + 1))
    question="What is $num1 plus $num2? Give me just the answer."

    # 发送请求
    response=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME_TOTAL:%{time_total}" \
        -X POST "$ENDPOINT" \
        -H "$AUTH_HEADER" \
        -H "Content-Type: application/json" \
        -H "anthropic-version: 2023-06-01" \
        -d '{
            "model": "'"$MODEL"'",
            "max_tokens": '"$MAX_TOKENS"',
            "messages": [{"role": "user", "content": "'"$question"'"}]
        }')

    # 提取状态码和响应时间
    http_code=$(echo "$response" | grep "HTTP_CODE:" | cut -d: -f2)
    time_total=$(echo "$response" | grep "TIME_TOTAL:" | cut -d: -f2)

    # 分析响应
    if [ "$http_code" = "200" ]; then
        success_count=$((success_count + 1))

        # 提取 usage 信息
        body=$(echo "$response" | sed '/HTTP_CODE:/,$d')
        input_tokens=$(echo "$body" | jq -r '.usage.input_tokens // 0' 2>/dev/null || echo 0)
        output_tokens=$(echo "$body" | jq -r '.usage.output_tokens // 0' 2>/dev/null || echo 0)

        total_input_tokens=$((total_input_tokens + input_tokens))
        total_output_tokens=$((total_output_tokens + output_tokens))

        # 提取回答
        answer=$(echo "$body" | jq -r '.content[0].text // "N/A"' 2>/dev/null || echo "N/A")

        echo "✅ 成功 - 耗时: ${time_total}s"
        echo "   Input: $input_tokens tokens, Output: $output_tokens tokens"
        echo "   问题: $question"
        echo "   回答: ${answer:0:50}$([ ${#answer} -gt 50 ] && echo '...')"

        # 记录到日志
        echo "[$(date +%Y-%m-%d\ %H:%M:%S)] SUCCESS | Request #$request_count | Time: ${time_total}s | In: $input_tokens | Out: $output_tokens" >> logs/test-success.log
    else
        error_count=$((error_count + 1))

        # 提取错误信息
        error_body=$(echo "$response" | sed '/HTTP_CODE:/,$d')
        error_msg=$(echo "$error_body" | jq -r '.error.message // .error // "Unknown error"' 2>/dev/null || echo "Unknown error")
        error_type=$(echo "$error_body" | jq -r '.error.type // "unknown"' 2>/dev/null || echo "unknown")

        echo "❌ 失败 - HTTP $http_code - 耗时: ${time_total}s"
        echo "   错误类型: $error_type"
        echo "   错误信息: $error_msg"

        # 记录到日志
        echo "[$(date +%Y-%m-%d\ %H:%M:%S)] ERROR | Request #$request_count | HTTP $http_code | Type: $error_type | Msg: $error_msg" >> logs/test-errors.log
    fi

    # 显示当前统计
    if [ $request_count -gt 0 ]; then
        success_rate=$(awk "BEGIN {printf \"%.2f\", ($success_count/$request_count)*100}")
        avg_input=$(awk "BEGIN {printf \"%.1f\", $total_input_tokens/$request_count}")
        avg_output=$(awk "BEGIN {printf \"%.1f\", $total_output_tokens/$request_count}")

        echo ""
        echo "📊 当前统计:"
        echo "   成功率: $success_rate% ($success_count/$request_count)"
        echo "   平均 Token: In=$avg_input, Out=$avg_output"
    fi

    # 等待下一个请求
    sleep $REQUEST_INTERVAL
done

# ============================================================================
# 测试结束，生成报告
# ============================================================================

end_time=$(date +%s)
actual_duration=$((end_time - start_time))

echo ""
echo "════════════════════════════════════════════════════════════"
echo "🎉 测试完成！"
echo "结束时间: $(date)"
echo ""

# 计算最终统计
if [ $request_count -gt 0 ]; then
    success_rate=$(awk "BEGIN {printf \"%.2f\", ($success_count/$request_count)*100}")
    error_rate=$(awk "BEGIN {printf \"%.2f\", ($error_count/$request_count)*100}")
    avg_input=$(awk "BEGIN {printf \"%.1f\", $total_input_tokens/$request_count}")
    avg_output=$(awk "BEGIN {printf \"%.1f\", $total_output_tokens/$request_count}")
    total_tokens=$((total_input_tokens + total_output_tokens))
else
    success_rate=0
    error_rate=0
    avg_input=0
    avg_output=0
    total_tokens=0
fi

# 生成测试报告
cat > logs/test-report-$(date +%Y%m%d-%H%M%S).md << EOF
# Claude Console 测试报告

**测试时间**: $(date)
**端点**: $ENDPOINT
**模型**: $MODEL

## 测试配置

- 计划时长: $TEST_DURATION 秒
- 实际时长: $actual_duration 秒
- 请求间隔: $REQUEST_INTERVAL 秒

## 测试结果

### 总体统计

| 指标 | 数值 |
|-----|------|
| 总请求数 | $request_count |
| 成功请求 | $success_count |
| 失败请求 | $error_count |
| 成功率 | $success_rate% |
| 失败率 | $error_rate% |

### Token 使用统计

| 指标 | 数值 |
|-----|------|
| 总 Input Tokens | $total_input_tokens |
| 总 Output Tokens | $total_output_tokens |
| 总 Tokens | $total_tokens |
| 平均 Input Tokens | $avg_input |
| 平均 Output Tokens | $avg_output |

### 性能评估

- **成功率**: $(if (( $(echo "$success_rate > 95" | bc -l) )); then echo "✅ 优秀"; elif (( $(echo "$success_rate > 90" | bc -l) )); then echo "🟡 良好"; else echo "❌ 需改进"; fi)
- **稳定性**: $(if [ $error_count -lt 5 ]; then echo "✅ 稳定"; else echo "⚠️ 不稳定"; fi)

## 详细日志

- 成功日志: \`logs/test-success.log\`
- 错误日志: \`logs/test-errors.log\`

---
测试完成于: $(date)
EOF

# 显示最终报告
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                     最终测试报告                           ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "📊 总体统计"
echo "────────────────────────────────────────────────────────────"
echo "  总请求数:     $request_count"
echo "  成功:         $success_count"
echo "  失败:         $error_count"
echo "  成功率:       $success_rate%"
echo ""
echo "🎯 Token 使用"
echo "────────────────────────────────────────────────────────────"
echo "  总 Input:     $total_input_tokens tokens"
echo "  总 Output:    $total_output_tokens tokens"
echo "  总计:         $total_tokens tokens"
echo "  平均 Input:   $avg_input tokens/请求"
echo "  平均 Output:  $avg_output tokens/请求"
echo ""
echo "⏱️  性能"
echo "────────────────────────────────────────────────────────────"
echo "  计划时长:     $TEST_DURATION 秒"
echo "  实际时长:     $actual_duration 秒"
echo "  请求间隔:     $REQUEST_INTERVAL 秒"
echo ""

# 质量评估
echo "✨ 质量评估"
echo "────────────────────────────────────────────────────────────"
if (( $(echo "$success_rate >= 95" | bc -l) )); then
    echo "  成功率:       ✅ 优秀 ($success_rate%)"
elif (( $(echo "$success_rate >= 90" | bc -l) )); then
    echo "  成功率:       🟡 良好 ($success_rate%)"
else
    echo "  成功率:       ❌ 需改进 ($success_rate%)"
fi

if [ $error_count -lt 5 ]; then
    echo "  稳定性:       ✅ 稳定 ($error_count 个错误)"
else
    echo "  稳定性:       ⚠️ 不稳定 ($error_count 个错误)"
fi
echo ""

echo "📁 生成的文件"
echo "────────────────────────────────────────────────────────────"
echo "  详细报告:     logs/test-report-*.md"
echo "  成功日志:     logs/test-success.log"
if [ $error_count -gt 0 ]; then
    echo "  错误日志:     logs/test-errors.log"
fi
echo ""

# 如果使用本地后端，提示检查统计数据
if [[ "$USE_LOCAL" =~ ^[Yy]$ ]]; then
    echo "💡 提示"
    echo "────────────────────────────────────────────────────────────"
    echo "  可以使用以下命令验证统计数据:"
    echo "  1. 查看 API Key 使用统计:"
    echo "     docker exec redis-dev redis-cli GET \"api_key_usage:<your_key_id>\" | jq '.'"
    echo ""
    echo "  2. 查看账户使用统计:"
    echo "     docker exec redis-dev redis-cli GET \"usage:account:<account_id>:$(date +%Y-%m-%d)\" | jq '.'"
    echo ""
fi

echo "════════════════════════════════════════════════════════════"
echo "🎉 测试脚本执行完毕！"
echo "════════════════════════════════════════════════════════════"
