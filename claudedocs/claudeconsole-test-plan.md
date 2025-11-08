# Claude Console 账户完整测试方案

**测试目标**: 验证 Claude Console 账户的完整功能，包括认证、消息转发、统计数据准确性
**测试时长**: 5-10 分钟持续压测
**测试日期**: 准备就绪

## 📋 测试准备清单

### 1. 测试环境准备

#### 后端服务
```bash
# 确保后端运行在 debug 模式，便于查看日志
cd /mnt/d/prj/claude-relay-service
RUST_LOG=debug ./rust/target/debug/claude-relay > logs/test-session.log 2>&1 &
echo $! > logs/backend.pid

# 等待服务启动
sleep 3

# 验证服务健康
curl -s http://localhost:8080/health | jq '.'
```

#### Redis 清理（可选）
```bash
# 如果需要从干净状态开始，清理使用统计
docker exec redis-dev redis-cli KEYS "usage:*" | xargs docker exec redis-dev redis-cli DEL
docker exec redis-dev redis-cli KEYS "api_key_usage:*" | xargs docker exec redis-dev redis-cli DEL
```

### 2. 账户准备

#### 添加测试账户（通过管理界面或 API）

**方式 1: 通过管理界面**
1. 访问 http://localhost:8080/admin-next
2. 登录管理员账户
3. 进入账户管理页面
4. 添加 Claude Console 账户：
   - 名称: "生产环境测试账户"
   - 平台: claudeconsole
   - Session Token: `[您的有效 session_token]`
   - 自定义端点: `[您的自定义端点，如有]`
   - 并发限制: 5
   - 优先级: 50
   - 状态: active

**方式 2: 通过 Redis 直接添加**
```bash
# 生成账户 ID
ACCOUNT_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

# 添加账户到 Redis
docker exec redis-dev redis-cli SET "claude_account:claude_acc_${ACCOUNT_ID}" '{
  "id": "'${ACCOUNT_ID}'",
  "name": "生产环境测试账户",
  "platform": "claudeconsole",
  "session_token": "YOUR_VALID_SESSION_TOKEN_HERE",
  "custom_api_endpoint": "YOUR_CUSTOM_ENDPOINT_IF_ANY",
  "status": "active",
  "concurrencyLimit": 5,
  "priority": 50,
  "schedulable": true,
  "isActive": true,
  "currentConcurrency": 0,
  "createdAt": "'$(date -u +%Y-%m-%dT%H:%M:%S.%NZ)'",
  "updatedAt": "'$(date -u +%Y-%m-%dT%H:%M:%S.%NZ)'"
}'
```

#### 创建测试专用 API Key

**通过管理界面**:
1. 进入 API Keys 管理页面
2. 创建新 API Key：
   - 名称: "Claude Console 测试专用"
   - 权限: claude
   - 速率限制: 100 req/min
   - 绑定账户: 选择上面创建的测试账户
   - User-Agent 匹配: 留空

**保存生成的 API Key**: 类似 `cr_xxxxxxxxxxxx...`

## 🧪 测试场景设计

### 场景 1: 基础功能验证（2 分钟）

**目标**: 验证基本的请求/响应流程

```bash
# 创建测试脚本
cat > test-basic-flow.sh << 'EOF'
#!/bin/bash

API_KEY="YOUR_API_KEY_HERE"
ENDPOINT="http://localhost:8080/api/v1/messages"

echo "=== 场景 1: 基础功能验证 ==="
echo "开始时间: $(date)"

# 测试 1: 简单问候
echo -e "\n[测试 1] 简单问候"
curl -s -X POST $ENDPOINT \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Say hello in Chinese"}]
  }' | jq -r '.content[0].text // .error'

sleep 2

# 测试 2: 带上下文的对话
echo -e "\n[测试 2] 多轮对话"
curl -s -X POST $ENDPOINT \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 150,
    "messages": [
      {"role": "user", "content": "My name is Alice"},
      {"role": "assistant", "content": "Hello Alice! Nice to meet you."},
      {"role": "user", "content": "What is my name?"}
    ]
  }' | jq -r '.content[0].text // .error'

sleep 2

# 测试 3: 代码生成
echo -e "\n[测试 3] 代码生成"
curl -s -X POST $ENDPOINT \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 200,
    "messages": [{"role": "user", "content": "Write a Python function to check if a number is prime"}]
  }' | jq -r '.content[0].text // .error'

echo -e "\n结束时间: $(date)"
EOF

chmod +x test-basic-flow.sh
bash test-basic-flow.sh
```

**预期结果**:
- ✅ 所有请求返回 200 OK
- ✅ 响应包含有效的 Claude 回复
- ✅ 无认证错误
- ✅ 响应时间合理（<5 秒）

### 场景 2: 并发压力测试（3 分钟）

**目标**: 验证并发处理和调度器行为

```bash
# 创建并发测试脚本
cat > test-concurrent.sh << 'EOF'
#!/bin/bash

API_KEY="YOUR_API_KEY_HERE"
ENDPOINT="http://localhost:8080/api/v1/messages"
CONCURRENT=3  # 并发数
REQUESTS=20   # 总请求数

echo "=== 场景 2: 并发压力测试 ==="
echo "并发数: $CONCURRENT, 总请求数: $REQUESTS"
echo "开始时间: $(date)"

# 创建请求函数
make_request() {
  local id=$1
  echo "[请求 $id] 开始于 $(date +%H:%M:%S)"

  response=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME:%{time_total}" \
    -X POST $ENDPOINT \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -H "anthropic-version: 2023-06-01" \
    -d '{
      "model": "claude-3-5-sonnet-20241022",
      "max_tokens": 50,
      "messages": [{"role": "user", "content": "Count from 1 to 5"}]
    }')

  http_code=$(echo "$response" | grep "HTTP_CODE:" | cut -d: -f2)
  time_total=$(echo "$response" | grep "TIME:" | cut -d: -f2)

  echo "[请求 $id] 完成于 $(date +%H:%M:%S) - 状态码: $http_code, 耗时: ${time_total}s"
}

# 导出函数供并发使用
export -f make_request
export API_KEY ENDPOINT

# 并发执行
seq 1 $REQUESTS | xargs -P $CONCURRENT -I {} bash -c 'make_request {}'

echo "结束时间: $(date)"
EOF

chmod +x test-concurrent.sh
bash test-concurrent.sh
```

**预期结果**:
- ✅ 所有请求成功完成
- ✅ 并发控制正常（不超过账户并发限制）
- ✅ 无 429 (Too Many Requests) 错误
- ✅ 响应时间稳定

### 场景 3: 流式传输测试（2 分钟）

**目标**: 验证 SSE 流式响应处理

```bash
# 创建流式测试脚本
cat > test-streaming.sh << 'EOF'
#!/bin/bash

API_KEY="YOUR_API_KEY_HERE"
ENDPOINT="http://localhost:8080/api/v1/messages"

echo "=== 场景 3: 流式传输测试 ==="
echo "开始时间: $(date)"

# 测试 1: 短文本流式
echo -e "\n[测试 1] 短文本流式响应"
curl -s -N -X POST $ENDPOINT \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "stream": true,
    "messages": [{"role": "user", "content": "Tell me a short joke"}]
  }' | while IFS= read -r line; do
    if [[ $line == data:* ]]; then
      echo "[$(date +%H:%M:%S)] $line"
    fi
  done

sleep 2

# 测试 2: 长文本流式
echo -e "\n[测试 2] 长文本流式响应"
curl -s -N -X POST $ENDPOINT \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 500,
    "stream": true,
    "messages": [{"role": "user", "content": "Write a short story about a robot"}]
  }' | while IFS= read -r line; do
    if [[ $line == data:* ]]; then
      echo -n "."
    fi
  done
echo -e "\n"

echo "结束时间: $(date)"
EOF

chmod +x test-streaming.sh
bash test-streaming.sh
```

**预期结果**:
- ✅ 流式事件正确传输
- ✅ 事件顺序正确（message_start → content_block → message_delta → message_stop）
- ✅ 无连接中断
- ✅ usage 数据正确捕获

### 场景 4: 错误处理测试（1 分钟）

**目标**: 验证各种错误场景的处理

```bash
# 创建错误测试脚本
cat > test-error-handling.sh << 'EOF'
#!/bin/bash

API_KEY="YOUR_API_KEY_HERE"
ENDPOINT="http://localhost:8080/api/v1/messages"

echo "=== 场景 4: 错误处理测试 ==="
echo "开始时间: $(date)"

# 测试 1: 无效的 API Key
echo -e "\n[测试 1] 无效的 API Key"
curl -s -w "\nHTTP_CODE:%{http_code}" -X POST $ENDPOINT \
  -H "Authorization: Bearer cr_invalid_key_12345" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model": "claude-3-5-sonnet-20241022", "max_tokens": 50, "messages": [{"role": "user", "content": "Hi"}]}' \
  | tail -1

# 测试 2: 超大 max_tokens
echo -e "\n[测试 2] 超大 max_tokens"
curl -s -w "\nHTTP_CODE:%{http_code}" -X POST $ENDPOINT \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model": "claude-3-5-sonnet-20241022", "max_tokens": 999999, "messages": [{"role": "user", "content": "Hi"}]}' \
  | grep -E "HTTP_CODE|error"

# 测试 3: 空消息
echo -e "\n[测试 3] 空消息列表"
curl -s -w "\nHTTP_CODE:%{http_code}" -X POST $ENDPOINT \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model": "claude-3-5-sonnet-20241022", "max_tokens": 50, "messages": []}' \
  | grep -E "HTTP_CODE|error"

echo -e "\n结束时间: $(date)"
EOF

chmod +x test-error-handling.sh
bash test-error-handling.sh
```

**预期结果**:
- ✅ 无效 API Key → 401 Unauthorized
- ✅ 超大 max_tokens → 400 或外部 API 错误
- ✅ 空消息 → 400 Bad Request
- ✅ 错误响应格式正确

### 场景 5: 持续负载测试（可配置时长）

**目标**: 长时间运行验证稳定性和统计准确性

```bash
# 创建持续负载测试脚本
cat > test-sustained-load.sh << 'EOF'
#!/bin/bash

API_KEY="YOUR_API_KEY_HERE"
ENDPOINT="http://localhost:8080/api/v1/messages"
DURATION=${1:-300}  # 测试时长（秒），默认 300 秒（5分钟）
INTERVAL=3          # 每 3 秒一个请求

echo "=== 场景 5: 持续负载测试 ==="
echo "测试时长: ${DURATION}秒 ($(awk "BEGIN {printf \"%.1f\", $DURATION/60}")分钟)"
echo "请求间隔: ${INTERVAL}秒"
echo "预计请求数: $(awk "BEGIN {printf \"%.0f\", $DURATION/$INTERVAL}")"
echo "开始时间: $(date)"

start_time=$(date +%s)
request_count=0
success_count=0
error_count=0

while true; do
  current_time=$(date +%s)
  elapsed=$((current_time - start_time))

  if [ $elapsed -ge $DURATION ]; then
    break
  fi

  request_count=$((request_count + 1))
  echo -e "\n[请求 $request_count] 时间: $(date +%H:%M:%S), 已运行: ${elapsed}s"

  # 发送请求
  response=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST $ENDPOINT \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -H "anthropic-version: 2023-06-01" \
    -d '{
      "model": "claude-3-5-sonnet-20241022",
      "max_tokens": 100,
      "messages": [{"role": "user", "content": "What is '$(shuf -i 1-100 -n 1)' plus '$(shuf -i 1-100 -n 1)'?"}]
    }')

  http_code=$(echo "$response" | grep "HTTP_CODE:" | cut -d: -f2)

  if [ "$http_code" = "200" ]; then
    success_count=$((success_count + 1))
    # 提取 usage 信息
    input_tokens=$(echo "$response" | jq -r '.usage.input_tokens // 0')
    output_tokens=$(echo "$response" | jq -r '.usage.output_tokens // 0')
    echo "  ✅ 成功 - Input: ${input_tokens} tokens, Output: ${output_tokens} tokens"
  else
    error_count=$((error_count + 1))
    echo "  ❌ 失败 - HTTP $http_code"
  fi

  sleep $INTERVAL
done

echo -e "\n=== 测试统计 ==="
echo "总请求数: $request_count"
echo "成功: $success_count"
echo "失败: $error_count"
echo "成功率: $(awk "BEGIN {printf \"%.2f%%\", ($success_count/$request_count)*100}")"
echo "结束时间: $(date)"
EOF

chmod +x test-sustained-load.sh
bash test-sustained-load.sh
```

**预期结果**:
- ✅ 成功率 > 95%
- ✅ 无内存泄漏（后端内存稳定）
- ✅ 响应时间稳定
- ✅ 无连接池耗尽

## 📊 统计数据验证

### 1. 实时监控脚本

```bash
# 创建监控脚本
cat > monitor-stats.sh << 'EOF'
#!/bin/bash

API_KEY_ID="YOUR_API_KEY_ID"  # 从 Redis 获取
ACCOUNT_ID="YOUR_ACCOUNT_ID"

echo "=== 统计数据监控 ==="

while true; do
  clear
  echo "监控时间: $(date)"
  echo "================================"

  # API Key 使用统计
  echo -e "\n📊 API Key 使用统计:"
  docker exec redis-dev redis-cli GET "api_key_usage:${API_KEY_ID}" | jq '.'

  # 账户使用统计
  echo -e "\n📈 账户使用统计:"
  docker exec redis-dev redis-cli GET "usage:account:${ACCOUNT_ID}:$(date +%Y-%m-%d)" | jq '.'

  # 当前并发数
  echo -e "\n⚡ 当前并发:"
  docker exec redis-dev redis-cli ZCARD "concurrency:${ACCOUNT_ID}"

  # 会话窗口
  echo -e "\n🔄 会话窗口:"
  docker exec redis-dev redis-cli GET "session_window:${ACCOUNT_ID}"

  sleep 5
done
EOF

chmod +x monitor-stats.sh
```

### 2. 测试后数据验证

```bash
# 创建验证脚本
cat > verify-stats.sh << 'EOF'
#!/bin/bash

API_KEY_ID="YOUR_API_KEY_ID"
ACCOUNT_ID="YOUR_ACCOUNT_ID"
TODAY=$(date +%Y-%m-%d)

echo "=== 测试后统计数据验证 ==="
echo "日期: $TODAY"

# 1. API Key 总使用量
echo -e "\n1️⃣ API Key 使用统计:"
api_key_usage=$(docker exec redis-dev redis-cli GET "api_key_usage:${API_KEY_ID}")
echo "$api_key_usage" | jq '{
  total_requests: .request_count,
  total_input_tokens: .input_tokens,
  total_output_tokens: .output_tokens,
  total_cost: .cost
}'

# 2. 账户使用量
echo -e "\n2️⃣ 账户使用统计:"
account_usage=$(docker exec redis-dev redis-cli GET "usage:account:${ACCOUNT_ID}:${TODAY}")
echo "$account_usage" | jq '{
  requests: .count,
  input_tokens: .input_tokens,
  output_tokens: .output_tokens
}'

# 3. 全局统计
echo -e "\n3️⃣ 全局统计:"
docker exec redis-dev redis-cli GET "usage:global:${TODAY}" | jq '{
  total_requests: .total_requests,
  total_tokens: (.total_input_tokens + .total_output_tokens)
}'

# 4. 粘性会话数量
echo -e "\n4️⃣ 粘性会话数:"
docker exec redis-dev redis-cli KEYS "sticky_session:*" | wc -l

# 5. 验证一致性
echo -e "\n5️⃣ 数据一致性检查:"
api_requests=$(echo "$api_key_usage" | jq -r '.request_count // 0')
account_requests=$(echo "$account_usage" | jq -r '.count // 0')

if [ "$api_requests" = "$account_requests" ]; then
  echo "✅ API Key 和账户请求数一致: $api_requests"
else
  echo "⚠️ 数据不一致! API Key: $api_requests, 账户: $account_requests"
fi

# 6. Token 计算验证
api_input=$(echo "$api_key_usage" | jq -r '.input_tokens // 0')
api_output=$(echo "$api_key_usage" | jq -r '.output_tokens // 0')
account_input=$(echo "$account_usage" | jq -r '.input_tokens // 0')
account_output=$(echo "$account_usage" | jq -r '.output_tokens // 0')

echo -e "\nToken 统计:"
echo "  API Key - Input: $api_input, Output: $api_output"
echo "  Account - Input: $account_input, Output: $account_output"

if [ "$api_input" = "$account_input" ] && [ "$api_output" = "$account_output" ]; then
  echo "✅ Token 计数一致"
else
  echo "⚠️ Token 计数不一致"
fi
EOF

chmod +x verify-stats.sh
```

## 🔍 日志分析

### 实时日志监控

```bash
# 监控后端日志
tail -f logs/test-session.log | grep -E "session_token|claude_relay|usage|error"

# 监控特定账户的日志
tail -f logs/test-session.log | grep "YOUR_ACCOUNT_ID"

# 监控错误
tail -f logs/test-session.log | grep -i "error\|warn\|fail"
```

### 日志分析脚本

```bash
cat > analyze-logs.sh << 'EOF'
#!/bin/bash

LOG_FILE="logs/test-session.log"

echo "=== 日志分析报告 ==="
echo "日志文件: $LOG_FILE"

# 1. 请求总数
echo -e "\n📊 请求统计:"
total_requests=$(grep -c "POST /api/v1/messages" "$LOG_FILE" || echo 0)
echo "  总请求数: $total_requests"

# 2. session_token 使用次数
echo -e "\n🔑 Session Token 使用:"
session_token_usage=$(grep -c "session_token" "$LOG_FILE" || echo 0)
echo "  Session token 提及次数: $session_token_usage"

# 3. 错误统计
echo -e "\n❌ 错误统计:"
errors=$(grep -ci "error" "$LOG_FILE" || echo 0)
warnings=$(grep -ci "warn" "$LOG_FILE" || echo 0)
echo "  Errors: $errors"
echo "  Warnings: $warnings"

# 4. 响应时间分析（如果日志包含时间信息）
echo -e "\n⏱️ 性能指标:"
grep "completed in" "$LOG_FILE" | tail -20

# 5. 最近的错误
echo -e "\n🔴 最近的错误:"
grep -i "error" "$LOG_FILE" | tail -5

echo -e "\n分析完成: $(date)"
EOF

chmod +x analyze-logs.sh
```

## 📝 测试执行流程

### 完整测试流程

```bash
# 创建主测试脚本
cat > run-full-test.sh << 'EOF'
#!/bin/bash

echo "╔════════════════════════════════════════════════╗"
echo "║   Claude Console 账户完整测试套件             ║"
echo "╚════════════════════════════════════════════════╝"

# 默认测试时长
DEFAULT_DURATION=300  # 5 分钟

# 读取配置
read -p "请输入 API Key: " API_KEY
read -p "请输入 API Key ID (从 Redis): " API_KEY_ID
read -p "请输入账户 ID: " ACCOUNT_ID
read -p "请输入持续负载测试时长（秒，默认 $DEFAULT_DURATION）: " TEST_DURATION
TEST_DURATION=${TEST_DURATION:-$DEFAULT_DURATION}

echo -e "\n📋 测试配置:"
echo "  - API Key: ${API_KEY:0:10}..."
echo "  - 持续测试时长: $TEST_DURATION 秒 ($(awk "BEGIN {printf \"%.1f\", $TEST_DURATION/60}") 分钟)"

# 更新所有脚本中的配置
for script in test-*.sh monitor-stats.sh verify-stats.sh; do
  sed -i "s/YOUR_API_KEY_HERE/$API_KEY/g" "$script"
  sed -i "s/YOUR_API_KEY_ID/$API_KEY_ID/g" "$script"
  sed -i "s/YOUR_ACCOUNT_ID/$ACCOUNT_ID/g" "$script"
done

# 启动监控（后台）
echo -e "\n🔍 启动统计监控..."
bash monitor-stats.sh > logs/monitor.log 2>&1 &
MONITOR_PID=$!
echo "监控进程 PID: $MONITOR_PID"

sleep 2

# 执行测试场景
echo -e "\n🧪 开始测试..."

echo -e "\n▶️  场景 1: 基础功能验证"
bash test-basic-flow.sh | tee logs/test-1-basic.log

echo -e "\n▶️  场景 2: 并发压力测试"
bash test-concurrent.sh | tee logs/test-2-concurrent.log

echo -e "\n▶️  场景 3: 流式传输测试"
bash test-streaming.sh | tee logs/test-3-streaming.log

echo -e "\n▶️  场景 4: 错误处理测试"
bash test-error-handling.sh | tee logs/test-4-errors.log

echo -e "\n▶️  场景 5: 持续负载测试 ($TEST_DURATION 秒)"
bash test-sustained-load.sh $TEST_DURATION | tee logs/test-5-sustained.log

# 停止监控
echo -e "\n⏹️  停止监控..."
kill $MONITOR_PID

# 等待一下让最后的统计数据写入
sleep 3

# 数据验证
echo -e "\n✅ 验证统计数据..."
bash verify-stats.sh | tee logs/verification.log

# 日志分析
echo -e "\n📋 分析日志..."
bash analyze-logs.sh | tee logs/analysis.log

# 生成测试报告
echo -e "\n📄 生成测试报告..."
cat > logs/test-report-$(date +%Y%m%d-%H%M%S).md << REPORT
# Claude Console 账户测试报告

**测试时间**: $(date)
**账户 ID**: $ACCOUNT_ID
**API Key ID**: $API_KEY_ID

## 测试结果

### 场景 1: 基础功能
$(cat logs/test-1-basic.log | tail -20)

### 场景 2: 并发测试
$(cat logs/test-2-concurrent.log | tail -20)

### 场景 5: 持续负载
$(cat logs/test-5-sustained.log | tail -30)

## 统计验证
$(cat logs/verification.log)

## 日志分析
$(cat logs/analysis.log)

---
测试完成于: $(date)
REPORT

echo -e "\n✨ 测试完成！"
echo "📊 查看完整报告: logs/test-report-*.md"
echo "📝 查看详细日志: logs/*.log"
EOF

chmod +x run-full-test.sh
```

## 🎯 执行测试

### 1. 准备步骤

```bash
# 确保在项目根目录
cd /mnt/d/prj/claude-relay-service

# 创建日志目录
mkdir -p logs

# 确保后端运行
make rust-dev

# 等待服务就绪
sleep 5
curl http://localhost:8080/health
```

### 2. 获取必要信息

```bash
# 获取 API Key ID（从管理界面或 Redis）
docker exec redis-dev redis-cli KEYS "api_key:*"

# 获取账户 ID
docker exec redis-dev redis-cli KEYS "claude_account:*"

# 查看特定 API Key 详情
docker exec redis-dev redis-cli GET "api_key:YOUR_KEY_ID" | jq '.'
```

### 3. 运行完整测试

```bash
# 运行主测试脚本
bash run-full-test.sh

# 或者单独运行各个场景
bash test-basic-flow.sh
bash test-concurrent.sh
bash test-sustained-load.sh
```

## 📈 成功标准

### 功能性指标
- ✅ 所有基础请求成功率 100%
- ✅ 并发请求成功率 > 95%
- ✅ 持续负载成功率 > 95%
- ✅ 流式传输无中断
- ✅ 错误处理符合预期

### 性能指标
- ✅ 平均响应时间 < 3 秒
- ✅ P95 响应时间 < 5 秒
- ✅ 并发处理正常（不超过限制）
- ✅ 后端内存使用稳定（< 100MB）

### 数据准确性
- ✅ API Key 和账户请求数一致
- ✅ Input/Output tokens 计数准确
- ✅ 成本计算正确
- ✅ 粘性会话正常工作
- ✅ 并发计数准确

### 日志质量
- ✅ 无错误日志（除预期的测试错误）
- ✅ session_token 正确使用
- ✅ 请求路由正确
- ✅ 统计更新及时

## 🔧 故障排查

### 常见问题

**问题 1**: 所有请求返回 401
- 检查: session_token 是否有效
- 检查: API Key 是否正确绑定账户

**问题 2**: 统计数据不一致
- 检查: Redis 连接是否稳定
- 检查: 后端日志是否有统计更新失败

**问题 3**: 并发请求失败
- 检查: 并发限制设置
- 检查: Redis 连接池大小

**问题 4**: 流式传输中断
- 检查: 网络连接
- 检查: 自定义端点是否稳定

## 📊 预期输出示例

### 成功的测试输出
```
=== 场景 5: 持续负载测试 ===
测试时长: 300秒, 请求间隔: 3秒
开始时间: Thu Nov  6 12:00:00 CST 2025

[请求 1] 时间: 12:00:00, 已运行: 0s
  ✅ 成功 - Input: 15 tokens, Output: 42 tokens

[请求 2] 时间: 12:00:03, 已运行: 3s
  ✅ 成功 - Input: 18 tokens, Output: 38 tokens

...

=== 测试统计 ===
总请求数: 100
成功: 98
失败: 2
成功率: 98.00%
```

### 统计验证输出
```
=== 测试后统计数据验证 ===
日期: 2025-11-06

1️⃣ API Key 使用统计:
{
  "total_requests": 100,
  "total_input_tokens": 1500,
  "total_output_tokens": 4000,
  "total_cost": 0.025
}

2️⃣ 账户使用统计:
{
  "requests": 100,
  "input_tokens": 1500,
  "output_tokens": 4000
}

5️⃣ 数据一致性检查:
✅ API Key 和账户请求数一致: 100
✅ Token 计数一致
```

---

**测试方案版本**: v1.0
**创建日期**: 2025-11-06
**适用范围**: Claude Console 账户完整功能验证
