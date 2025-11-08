# Claude Console 账户功能实现计划

## 目标

将 Mock 实现替换为真实的 Redis 存储和查询逻辑，参考 CCR 账户实现模式。

## 参考实现

CCR 账户实现 (`rust/src/routes/admin.rs` Line 1100-1222):
- 使用 Redis 存储：`ccr_account:{uuid}`
- 列表查询：`KEYS ccr_account:*`
- 创建流程：验证 → 生成ID → 序列化 → Redis SET
- 返回格式：统一 `{success, message, data}` 结构

## 实现步骤

### 1. 定义数据结构

**请求结构** (已存在):
```rust
#[derive(Debug, Deserialize)]
struct ClaudeAccountRequest {
    name: String,
    #[serde(rename = "type")]
    account_type: String,  // "claude-console", "claude-official"
    session_token: Option<String>,
    custom_api_endpoint: Option<String>,
    description: Option<String>,
    is_active: Option<bool>,
    is_schedulable: Option<bool>,
}
```

**存储格式** (JSON in Redis):
```json
{
  "id": "claude_acc_{uuid}",
  "name": "账户名称",
  "account_type": "claude-console",
  "session_token": "encrypted_token",
  "custom_api_endpoint": "https://api.example.com",
  "description": "描述信息",
  "platform": "Claude",
  "isActive": true,
  "schedulable": true,
  "createdAt": "2025-11-03T18:00:00Z",
  "updatedAt": "2025-11-03T18:00:00Z"
}
```

**Redis 键模式**:
- `claude_console_account:{id}` - Claude Console 账户
- 或统一使用 `claude_account:{id}`，通过 `account_type` 字段区分

### 2. 实现 `list_claude_accounts_handler`

参考 CCR 实现：
```rust
async fn list_claude_accounts_handler(
    State(state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Listing Claude accounts");

    let mut conn = state.redis.get_connection().await?;

    // 查询所有 Claude 账户
    let pattern = "claude_console_account:*";
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(pattern)
        .query_async(&mut conn)
        .await
        .map_err(|e| {
            error!("Failed to query Claude account keys: {}", e);
            AppError::InternalError("Failed to fetch accounts".to_string())
        })?;

    let mut accounts = Vec::new();
    for key in keys {
        let account_json: String = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await?;

        if let Ok(account_data) = serde_json::from_str::<serde_json::Value>(&account_json) {
            accounts.push(account_data);
        }
    }

    info!("✅ Found {} Claude accounts", accounts.len());

    Ok((StatusCode::OK, Json(json!({
        "success": true,
        "data": accounts
    }))))
}
```

### 3. 实现 `create_claude_account_handler`

参考 CCR 实现：
```rust
async fn create_claude_account_handler(
    State(state): State<Arc<AdminRouteState>>,
    Json(request): Json<ClaudeAccountRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("➕ Creating Claude account: {}", request.name);

    // 验证必需字段
    if request.name.trim().is_empty() {
        return Err(AppError::BadRequest("Account name cannot be empty".to_string()));
    }

    if request.account_type != "claude-console" && request.account_type != "claude-official" {
        return Err(AppError::BadRequest("Invalid account type".to_string()));
    }

    if request.session_token.is_none() {
        return Err(AppError::BadRequest("Session token is required".to_string()));
    }

    // 生成账户 ID
    let account_id = format!("claude_acc_{}", uuid::Uuid::new_v4());

    // TODO: 加密 session_token
    // let encrypted_token = encrypt(&request.session_token.unwrap())?;

    // 构建账户数据
    let account_data = json!({
        "id": account_id,
        "name": request.name,
        "account_type": request.account_type,
        "session_token": request.session_token.unwrap(),  // TODO: 使用加密后的
        "custom_api_endpoint": request.custom_api_endpoint,
        "description": request.description,
        "platform": "Claude",
        "isActive": request.is_active.unwrap_or(true),
        "schedulable": request.is_schedulable.unwrap_or(true),
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "updatedAt": chrono::Utc::now().to_rfc3339()
    });

    // 存储到 Redis
    let redis_key = format!("claude_console_account:{}", account_id);
    let mut conn = state.redis.get_connection().await?;

    let account_json = serde_json::to_string(&account_data)?;
    redis::cmd("SET")
        .arg(&redis_key)
        .arg(&account_json)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| {
            error!("Failed to save Claude account to Redis: {}", e);
            AppError::InternalError("Failed to create account".to_string())
        })?;

    info!("✅ Claude account created successfully: {}", account_id);

    Ok((StatusCode::OK, Json(json!({
        "success": true,
        "message": "Claude账户创建成功",
        "account": {
            "id": account_id,
            "name": request.name,
            "description": request.description,
            "status": "active",
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    }))))
}
```

### 4. 实现 `update_claude_account_handler`

```rust
async fn update_claude_account_handler(
    State(state): State<Arc<AdminRouteState>>,
    Path(id): Path<String>,
    Json(request): Json<ClaudeAccountRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔄 Updating Claude account: {}", id);

    let redis_key = format!("claude_console_account:{}", id);
    let mut conn = state.redis.get_connection().await?;

    // 检查账户是否存在
    let exists: bool = redis::cmd("EXISTS")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await?;

    if !exists {
        return Err(AppError::NotFound("Account not found".to_string()));
    }

    // 获取现有账户数据
    let existing_json: String = redis::cmd("GET")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await?;

    let mut account_data: serde_json::Value = serde_json::from_str(&existing_json)?;

    // 更新字段
    if !request.name.is_empty() {
        account_data["name"] = json!(request.name);
    }
    if let Some(token) = request.session_token {
        account_data["session_token"] = json!(token);  // TODO: 加密
    }
    if let Some(endpoint) = request.custom_api_endpoint {
        account_data["custom_api_endpoint"] = json!(endpoint);
    }
    if let Some(desc) = request.description {
        account_data["description"] = json!(desc);
    }
    if let Some(active) = request.is_active {
        account_data["isActive"] = json!(active);
    }
    if let Some(schedulable) = request.is_schedulable {
        account_data["schedulable"] = json!(schedulable);
    }
    account_data["updatedAt"] = json!(chrono::Utc::now().to_rfc3339());

    // 保存更新后的数据
    let updated_json = serde_json::to_string(&account_data)?;
    redis::cmd("SET")
        .arg(&redis_key)
        .arg(&updated_json)
        .query_async::<_, ()>(&mut conn)
        .await?;

    info!("✅ Claude account updated successfully: {}", id);

    Ok((StatusCode::OK, Json(json!({
        "success": true,
        "message": "Claude账户更新成功",
        "account": account_data
    }))))
}
```

### 5. 实现 `delete_claude_account_handler`

```rust
async fn delete_claude_account_handler(
    State(state): State<Arc<AdminRouteState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    info!("🗑️  Deleting Claude account: {}", id);

    let redis_key = format!("claude_console_account:{}", id);
    let mut conn = state.redis.get_connection().await?;

    let deleted: u32 = redis::cmd("DEL")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await?;

    if deleted == 0 {
        return Err(AppError::NotFound("Account not found".to_string()));
    }

    info!("✅ Claude account deleted successfully: {}", id);

    Ok((StatusCode::OK, Json(json!({
        "success": true,
        "message": "Claude账户删除成功"
    }))))
}
```

### 6. 调度器集成

确保 `UnifiedClaudeScheduler` 能够查询到 Claude Console 账户：

1. 检查 `rust/src/services/unified_claude_scheduler.rs`
2. 确认账户查询逻辑包含 `claude_console_account:*` 模式
3. 或者统一使用 `claude_account:*` 模式

### 7. API Key 关联

确保 API Key 更新时能正确关联到 Claude Console 账户：

1. 检查 `rust/src/services/api_key_service.rs`
2. 确认 `account_id` 参数处理逻辑
3. 确保正确设置 `claude_console_account_id` 字段

## 测试计划

### 单元测试
- [ ] 创建账户测试
- [ ] 查询账户列表测试
- [ ] 更新账户测试
- [ ] 删除账户测试

### 集成测试
- [ ] 创建账户 → 查询列表验证存在
- [ ] 创建账户 → API Key 关联 → 验证关联成功
- [ ] 创建账户 → 调度器查询 → 验证可调度
- [ ] 创建账户 → API 调用 → 验证中转成功

### 端到端测试
- [ ] UI 创建账户
- [ ] UI 创建 API Key 关联账户
- [ ] 使用 API Key 调用 `/api/v1/messages`
- [ ] 验证响应成功

## 优先级

**P0 - 立即实现**:
1. `list_claude_accounts_handler` - 基础查询
2. `create_claude_account_handler` - 基础创建
3. 调度器集成 - 使账户可用

**P1 - 核心功能**:
4. `update_claude_account_handler` - 编辑功能
5. `delete_claude_account_handler` - 删除功能
6. API Key 关联修复

**P2 - 增强功能**:
7. Token 加密存储
8. OAuth 流程实现 (`generate_auth_url_handler`, `exchange_code_handler`)

## 预计工作量

- 基础实现（P0）：2-3 小时
- 核心功能（P1）：1-2 小时
- 测试和验证：1-2 小时
- **总计**：4-7 小时

## 下一步

立即开始实现 P0 功能，让 API Key 测试能够通过。
