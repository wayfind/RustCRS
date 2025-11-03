use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::info;

use crate::middleware::{authenticate_jwt, JwtAuthState};
use crate::models::api_key::{ApiKeyCreateOptions, ApiKeyPermissions};
use crate::services::{AdminService, ApiKeyService, LoginRequest};
use crate::utils::error::AppError;

// ============================================================================
// Data Structures
// ============================================================================

/// Admin路由共享状态
#[derive(Clone)]
pub struct AdminRouteState {
    pub admin_service: Arc<AdminService>,
    pub api_key_service: Arc<ApiKeyService>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OemSettings {
    pub enabled: bool,
    #[serde(rename = "companyName")]
    pub company_name: Option<String>,
    #[serde(rename = "logoUrl")]
    pub logo_url: Option<String>,
    #[serde(rename = "themeColor")]
    pub theme_color: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClaudeAccountRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "proxyUrl")]
    pub proxy_url: Option<String>,
    #[serde(rename = "proxyUsername")]
    pub proxy_username: Option<String>,
    #[serde(rename = "proxyPassword")]
    pub proxy_password: Option<String>,
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "tokenLimit")]
    pub token_limit: Option<i64>,
    pub permissions: Option<String>,
    #[serde(rename = "rateLimitWindow")]
    pub rate_limit_window: Option<i32>,
    #[serde(rename = "rateLimitRequests")]
    pub rate_limit_requests: Option<i32>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenerateAuthUrlRequest {
    #[serde(rename = "proxyUrl")]
    pub proxy_url: Option<String>,
    #[serde(rename = "proxyUsername")]
    pub proxy_username: Option<String>,
    #[serde(rename = "proxyPassword")]
    pub proxy_password: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExchangeCodeRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "proxyUrl")]
    pub proxy_url: Option<String>,
    #[serde(rename = "proxyUsername")]
    pub proxy_username: Option<String>,
    #[serde(rename = "proxyPassword")]
    pub proxy_password: Option<String>,
}

// ============================================================================
// Router Creation
// ============================================================================

/// 创建管理员路由
///
/// # 路由
///
/// - POST /admin/auth/login - 管理员登录
/// - GET /admin/profile - 获取管理员信息
/// - GET /admin/auth/user - 获取当前用户信息
/// - GET /admin/oem-settings - 获取OEM设置
/// - PUT /admin/oem-settings - 更新OEM设置
/// - GET /admin/dashboard - 获取仪表板数据
/// - GET /admin/claude-accounts - 获取Claude账户列表
/// - POST /admin/claude-accounts - 创建Claude账户
/// - PUT /admin/claude-accounts/:id - 更新Claude账户
/// - DELETE /admin/claude-accounts/:id - 删除Claude账户
/// - POST /admin/claude-accounts/generate-auth-url - 生成OAuth授权URL
/// - POST /admin/claude-accounts/exchange-code - 交换授权码
/// - GET /admin/api-keys - 获取API Keys列表
/// - POST /admin/api-keys - 创建API Key
/// - PUT /admin/api-keys/:id - 更新API Key
/// - DELETE /admin/api-keys/:id - 删除API Key
/// - PUT /admin/api-keys/:id/toggle - 启用/禁用API Key
/// - GET /admin/stats/overview - 获取统计概览
///
pub fn create_admin_routes(
    admin_service: Arc<AdminService>,
    api_key_service: Arc<ApiKeyService>,
) -> Router {
    // 创建共享状态
    let shared_state = Arc::new(AdminRouteState {
        admin_service: admin_service.clone(),
        api_key_service,
    });

    // 认证中间件工厂函数
    let auth_layer = |service: Arc<AdminService>| {
        axum::middleware::from_fn_with_state(service, authenticate_jwt)
    };

    // 公开路由 - 不需要认证（品牌化信息等）
    let public_routes = Router::new()
        .route("/auth/login", post(login_handler))
        .route("/oem-settings", get(get_oem_settings_handler))
        .with_state(shared_state.clone());

    // 受保护路由 - 需要JWT认证
    let protected_routes = Router::new()
        .route("/profile", get(get_profile_handler))
        .route("/auth/user", get(get_profile_handler))
        .route("/oem-settings", put(update_oem_settings_handler))
        .route("/dashboard", get(get_dashboard_handler))
        // Claude Console 账户管理（重命名以匹配前端期望）
        .route("/claude-console-accounts", get(list_claude_accounts_handler))
        .route("/claude-console-accounts", post(create_claude_account_handler))
        .route("/claude-console-accounts/:id", put(update_claude_account_handler))
        .route(
            "/claude-console-accounts/:id",
            delete(delete_claude_account_handler),
        )
        .route(
            "/claude-console-accounts/generate-auth-url",
            post(generate_auth_url_handler),
        )
        .route(
            "/claude-console-accounts/exchange-code",
            post(exchange_code_handler),
        )
        // Claude账户别名路由（前端兼容性）
        .route("/claude-accounts", get(list_claude_accounts_handler))
        .route("/claude-accounts", post(create_claude_account_handler))
        .route("/claude-accounts/:id", put(update_claude_account_handler))
        .route("/claude-accounts/:id", delete(delete_claude_account_handler))
        .route(
            "/claude-accounts/generate-auth-url",
            post(generate_auth_url_handler),
        )
        .route(
            "/claude-accounts/exchange-code",
            post(exchange_code_handler),
        )
        // 其他账户类型管理（占位实现）
        .route("/gemini-accounts", get(list_gemini_accounts_handler))
        .route("/openai-accounts", get(list_openai_accounts_handler))
        .route("/openai-responses-accounts", get(list_openai_responses_accounts_handler))
        .route("/bedrock-accounts", get(list_bedrock_accounts_handler))
        .route("/azure-openai-accounts", get(list_azure_openai_accounts_handler))
        .route("/droid-accounts", get(list_droid_accounts_handler))
        .route("/ccr-accounts", get(list_ccr_accounts_handler))
        // API Keys管理
        .route("/api-keys", get(list_api_keys_handler))
        .route("/api-keys", post(create_api_key_handler))
        .route("/api-keys/:id", get(get_api_key_handler)) // ISSUE-UI-009: 添加获取单个API Key详情
        .route("/api-keys/:id", put(update_api_key_handler))
        .route("/api-keys/:id", delete(delete_api_key_handler))
        .route("/api-keys/:id/toggle", put(toggle_api_key_handler))
        .route("/api-keys/tags", get(get_api_keys_tags_handler))
        .route("/tags", get(get_api_keys_tags_handler)) // Alias for frontend compatibility (ISSUE-UI-004)
        // 客户端和分组管理
        .route("/supported-clients", get(get_supported_clients_handler))
        .route("/account-groups", get(get_account_groups_handler))
        // Claude Code 版本管理
        .route("/claude-code-version", get(get_claude_code_version_handler))
        .route("/claude-code-version/clear", post(clear_claude_code_version_handler))
        // 用户管理
        .route("/users", get(get_users_handler))
        // 统计
        .route("/stats/overview", get(get_stats_overview_handler))
        .route("/usage-costs", get(get_usage_costs_handler))
        .route("/usage-trend", get(get_usage_trend_handler))
        .route("/model-stats", get(get_model_stats_handler))
        .route("/account-usage-trend", get(get_account_usage_trend_handler))
        .route("/api-keys-usage-trend", get(get_api_keys_usage_trend_handler))
        // 系统管理
        .route("/check-updates", get(check_updates_handler))
        // 应用认证中间件
        .layer(auth_layer(admin_service))
        .with_state(shared_state);

    // 合并公开和受保护路由
    public_routes.merge(protected_routes)
}

// ============================================================================
// Authentication Handlers
// ============================================================================

/// 管理员登录处理器
async fn login_handler(
    State(state): State<Arc<AdminRouteState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔐 Admin login attempt: {}", payload.username);
    let service = &state.admin_service;

    let response = service
        .authenticate(&payload.username, &payload.password)
        .await?;

    info!("✅ Admin login successful: {}", payload.username);

    Ok((StatusCode::OK, Json(response)))
}

/// 获取管理员资料处理器
async fn get_profile_handler(
    jwt_state: axum::Extension<JwtAuthState>,
) -> Result<impl IntoResponse, AppError> {
    let claims = &jwt_state.claims;

    Ok((
        StatusCode::OK,
        Json(json!({
            "username": claims.sub,
            "role": claims.role,
        })),
    ))
}

// ============================================================================
// OEM Settings Handlers
// ============================================================================

/// 获取OEM设置（Mock实现）
async fn get_oem_settings_handler() -> Result<impl IntoResponse, AppError> {
    info!("📝 Getting OEM settings");

    // Mock数据 - 返回默认设置
    let settings = OemSettings {
        enabled: false,
        company_name: Some("Claude Relay Service".to_string()),
        logo_url: None,
        theme_color: Some("#6366f1".to_string()),
    };

    Ok((StatusCode::OK, Json(settings)))
}

/// 更新OEM设置（Mock实现）
async fn update_oem_settings_handler(
    Json(settings): Json<OemSettings>,
) -> Result<impl IntoResponse, AppError> {
    info!("💾 Updating OEM settings: {:?}", settings);

    // Mock实现 - 直接返回接收到的设置
    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "OEM设置已更新",
            "settings": settings
        })),
    ))
}

// ============================================================================
// Dashboard Handlers
// ============================================================================

/// 获取仪表板数据（Mock实现）
async fn get_dashboard_handler() -> Result<impl IntoResponse, AppError> {
    info!("📊 Getting dashboard data");

    // Mock数据 - 返回符合前端期望的完整数据结构
    // 前端期望: data.overview, data.recentActivity, data.systemAverages, data.realtimeMetrics, data.systemHealth, data.systemTimezone
    let dashboard = json!({
        "success": true,
        "data": {
            "overview": {
                // API Keys 统计
                "totalApiKeys": 0,
                "activeApiKeys": 0,
                // 账户统计
                "totalAccounts": 0,
                "normalAccounts": 0,
                "abnormalAccounts": 0,
                "pausedAccounts": 0,
                "activeAccounts": 0,
                "rateLimitedAccounts": 0,
                "accountsByPlatform": {
                    "claude": 0,
                    "gemini": 0,
                    "openai": 0,
                    "bedrock": 0,
                    "azure": 0
                },
                // 请求统计
                "totalRequestsUsed": 0,
                // Token 统计
                "totalTokensUsed": 0,
                "totalInputTokensUsed": 0,
                "totalOutputTokensUsed": 0,
                "totalCacheCreateTokensUsed": 0,
                "totalCacheReadTokensUsed": 0
            },
            "recentActivity": {
                // 今日请求
                "requestsToday": 0,
                // 今日 Token
                "tokensToday": 0,
                "inputTokensToday": 0,
                "outputTokensToday": 0,
                "cacheCreateTokensToday": 0,
                "cacheReadTokensToday": 0
            },
            "systemAverages": {
                "rpm": 0,
                "tpm": 0
            },
            "realtimeMetrics": {
                "rpm": 0,
                "tpm": 0,
                "windowMinutes": 5,
                "isHistorical": false
            },
            "systemHealth": {
                "redisConnected": true,
                "uptime": 0
            },
            "systemTimezone": 8
        }
    });

    Ok((StatusCode::OK, Json(dashboard)))
}

// ============================================================================
// Claude Accounts Handlers
// ============================================================================

/// 获取Claude账户列表（Mock实现）
async fn list_claude_accounts_handler() -> Result<impl IntoResponse, AppError> {
    info!("📋 Listing Claude accounts");

    // Mock数据 - 返回空列表
    let response = json!({
        "success": true,
        "accounts": []
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 创建Claude账户（Mock实现）
async fn create_claude_account_handler(
    Json(account): Json<ClaudeAccountRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("➕ Creating Claude account: {}", account.name);

    // Mock实现 - 返回成功响应
    let response = json!({
        "success": true,
        "message": "Claude账户创建成功",
        "account": {
            "id": format!("claude_acc_{}", uuid::Uuid::new_v4()),
            "name": account.name,
            "description": account.description,
            "status": "active",
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 更新Claude账户（Mock实现）
async fn update_claude_account_handler(
    Path(id): Path<String>,
    Json(account): Json<ClaudeAccountRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔄 Updating Claude account: {}", id);

    let response = json!({
        "success": true,
        "message": "Claude账户更新成功",
        "account": {
            "id": id,
            "name": account.name,
            "description": account.description,
            "status": "active"
        }
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 删除Claude账户（Mock实现）
async fn delete_claude_account_handler(
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    info!("🗑️  Deleting Claude account: {}", id);

    let response = json!({
        "success": true,
        "message": "Claude账户删除成功"
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 生成OAuth授权URL（Mock实现）
async fn generate_auth_url_handler(
    Json(_request): Json<GenerateAuthUrlRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔗 Generating OAuth authorization URL");

    // Mock实现 - 返回示例URL
    let response = json!({
        "success": true,
        "authUrl": "https://claude.ai/oauth/authorize?client_id=example&redirect_uri=urn:ietf:wg:oauth:2.0:oob&response_type=code&scope=openid%20profile%20email",
        "message": "请在浏览器中打开此URL进行授权"
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 交换授权码（Mock实现）
async fn exchange_code_handler(
    Json(request): Json<ExchangeCodeRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "🔄 Exchanging authorization code for account: {}",
        request.name
    );

    // Mock实现 - 返回成功响应
    let response = json!({
        "success": true,
        "message": "OAuth授权成功，账户已创建",
        "account": {
            "id": format!("claude_acc_{}", uuid::Uuid::new_v4()),
            "name": request.name,
            "description": request.description,
            "status": "active",
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    });

    Ok((StatusCode::OK, Json(response)))
}

// ============================================================================
// API Keys Handlers
// ============================================================================

/// 获取API Keys列表
async fn list_api_keys_handler(
    State(state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔑 Listing API keys");

    // 使用真实服务获取所有API Keys（不包括已删除的）
    let api_keys = state.api_key_service.get_all_keys(false).await?;

    let response = json!({
        "success": true,
        "data": api_keys
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 获取单个API Key详情
///
/// 修复 ISSUE-UI-009: 编辑 API Key 时前端需要获取完整配置
async fn get_api_key_handler(
    State(state): State<Arc<AdminRouteState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔍 Getting API key detail: {}", id);

    // 使用真实服务获取API Key
    let api_key = state.api_key_service.get_key(&id).await?;

    let response = json!({
        "success": true,
        "data": api_key
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 创建API Key
async fn create_api_key_handler(
    State(state): State<Arc<AdminRouteState>>,
    Json(key_request): Json<ApiKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("➕ Creating API key: {}", key_request.name);

    // 解析permissions字符串为枚举
    let permissions = match key_request.permissions.as_deref() {
        Some("all") | None => ApiKeyPermissions::All,
        Some("claude") => ApiKeyPermissions::Claude,
        Some("gemini") => ApiKeyPermissions::Gemini,
        Some("openai") => ApiKeyPermissions::OpenAI,
        Some("droid") => ApiKeyPermissions::Droid,
        Some(other) => {
            return Err(AppError::BadRequest(format!("Invalid permissions: {}", other)))
        }
    };

    // 创建API Key选项
    let options = ApiKeyCreateOptions {
        name: key_request.name.clone(),
        description: key_request.description.clone(),
        icon: None,
        permissions,
        is_active: true,
        tags: key_request.tags.clone(),  // 传递标签
        ..Default::default()
    };

    // 使用真实服务生成API Key
    let (raw_key, api_key) = state.api_key_service.generate_key(options).await?;

    // 返回包含原始key的响应（仅在创建时返回一次）
    let mut response_key = api_key;
    response_key.key = Some(raw_key);

    let response = json!({
        "success": true,
        "message": "API Key创建成功",
        "data": response_key  // 改为 data 字段，与前端期待的字段名一致
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 更新API Key
async fn update_api_key_handler(
    State(state): State<Arc<AdminRouteState>>,
    Path(id): Path<String>,
    Json(key_request): Json<ApiKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔄 Updating API key: {} with name: {}", id, key_request.name);

    // 调用 ApiKeyService 的更新方法
    // 目前只支持更新 name 和 is_active
    // ApiKeyRequest 不包含 is_active 字段，所以传 None（保持原状态）
    let updated_key = state
        .api_key_service
        .update_key(&id, Some(key_request.name), None)
        .await?;

    let response = json!({
        "success": true,
        "message": "API Key更新成功",
        "data": updated_key  // 修复 ISSUE-UI-007: 与其他端点保持一致，使用 data 字段
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 删除API Key（软删除）
async fn delete_api_key_handler(
    State(state): State<Arc<AdminRouteState>>,
    jwt_state: axum::Extension<JwtAuthState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    info!("🗑️  Deleting API key: {} by user: {}", id, jwt_state.claims.sub);

    // 调用 ApiKeyService 的软删除方法
    state
        .api_key_service
        .delete_key(&id, &jwt_state.claims.sub)
        .await?;

    let response = json!({
        "success": true,
        "message": "API Key删除成功"
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 启用/禁用API Key（Mock实现）
async fn toggle_api_key_handler(Path(id): Path<String>) -> Result<impl IntoResponse, AppError> {
    info!("🔄 Toggling API key: {}", id);

    let response = json!({
        "success": true,
        "message": "API Key状态已切换",
        "apiKey": {
            "id": id,
            "isActive": true
        }
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 获取所有 API Keys 的标签列表
///
/// 收集所有 API Keys 的标签，去重并排序返回
async fn get_api_keys_tags_handler(
    State(state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching API keys tags");

    // 1. 获取所有 API Keys（不包括已删除）
    let api_keys = state.api_key_service.get_all_keys(false).await?;

    // 2. 收集所有标签（使用 HashSet 自动去重）
    let mut tag_set = std::collections::HashSet::new();
    for api_key in api_keys {
        for tag in api_key.tags {
            let trimmed = tag.trim();
            if !trimmed.is_empty() {
                tag_set.insert(trimmed.to_string());
            }
        }
    }

    // 3. 转换为向量并排序
    let mut tags: Vec<String> = tag_set.into_iter().collect();
    tags.sort();

    info!("📋 Retrieved {} unique tags from API keys", tags.len());

    let response = json!({
        "success": true,
        "data": tags
    });

    Ok((StatusCode::OK, Json(response)))
}

// ============================================================================
// User Management Handlers
// ============================================================================

/// 获取用户列表
///
/// 返回系统中所有用户的列表，供前端下拉选择使用
async fn get_users_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching users list");

    // 目前只返回默认的 admin 用户
    // 未来可以扩展为从 UserService 获取完整的用户列表
    let users = vec![
        serde_json::json!({
            "id": "admin",
            "username": "admin",
            "displayName": "Admin",
            "email": "",
            "role": "admin"
        })
    ];

    info!("📋 Retrieved {} users", users.len());

    let response = json!({
        "success": true,
        "data": users
    });

    Ok((StatusCode::OK, Json(response)))
}

// ============================================================================
// Statistics Handlers
// ============================================================================

/// 获取统计概览
///
/// 聚合所有 API Keys 的使用统计数据，返回总体概览
async fn get_stats_overview_handler(
    State(state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📊 Fetching stats overview");

    // 1. 获取所有 API Keys（不包括已删除）
    let all_keys = state.api_key_service.get_all_keys(false).await?;

    // 2. 统计活跃 API Keys 数量
    let total_api_keys = all_keys.len() as i64;
    let active_api_keys = all_keys.iter().filter(|k| k.is_active && !k.is_deleted).count() as i64;

    // 3. 聚合所有 API Keys 的使用量
    let mut total_requests = 0i64;
    let mut total_input_tokens = 0i64;
    let mut total_output_tokens = 0i64;
    let mut total_cache_creation_tokens = 0i64;
    let mut total_cache_read_tokens = 0i64;
    let mut total_cost = 0.0f64;

    for api_key in &all_keys {
        // 获取每个 key 的使用统计
        if let Ok(usage_stats) = state.api_key_service.get_usage_stats(&api_key.id).await {
            total_requests += usage_stats.total_requests;
            total_input_tokens += usage_stats.total_input_tokens;
            total_output_tokens += usage_stats.total_output_tokens;
            total_cache_creation_tokens += usage_stats.total_cache_creation_tokens;
            total_cache_read_tokens += usage_stats.total_cache_read_tokens;
            total_cost += usage_stats.total_cost;
        }
    }

    // 4. 构建响应
    let stats = serde_json::json!({
        "success": true,
        "stats": {
            "totalApiKeys": total_api_keys,
            "activeApiKeys": active_api_keys,
            "totalUsage": {
                "requests": total_requests,
                "inputTokens": total_input_tokens,
                "outputTokens": total_output_tokens,
                "cacheCreationTokens": total_cache_creation_tokens,
                "cacheReadTokens": total_cache_read_tokens,
                "totalCost": total_cost
            }
        }
    });

    info!("📊 Stats overview: {} total keys, {} active keys, {} total requests",
          total_api_keys, active_api_keys, total_requests);

    Ok((StatusCode::OK, Json(stats)))
}

/// 获取使用成本统计
///
/// 按时间维度（today/week/month）聚合所有 API Keys 的成本数据
async fn get_usage_costs_handler(
    State(state): State<Arc<AdminRouteState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let period = params.get("period").map(|s| s.as_str()).unwrap_or("today");
    info!("📊 Fetching usage costs for period: {}", period);

    // 1. 获取所有 API Keys（不包括已删除）
    let all_keys = state.api_key_service.get_all_keys(false).await?;

    // 2. 根据时间维度聚合数据
    let mut total_cost = 0.0f64;
    let mut total_input_tokens = 0i64;
    let mut total_output_tokens = 0i64;
    let mut total_requests = 0i64;

    for api_key in &all_keys {
        if let Ok(usage_stats) = state.api_key_service.get_usage_stats(&api_key.id).await {
            // 根据 period 参数选择对应的统计字段
            match period {
                "today" => {
                    // 使用每日成本
                    total_cost += usage_stats.daily_cost;
                    // 注意：当前 ApiKeyUsageStats 没有每日 tokens 字段，使用总量作为近似
                    // 完整实现需要在 Redis 中按日期存储 tokens
                    total_input_tokens += usage_stats.total_input_tokens;
                    total_output_tokens += usage_stats.total_output_tokens;
                    total_requests += usage_stats.total_requests;
                }
                "week" => {
                    // 使用每周成本
                    total_cost += usage_stats.weekly_opus_cost;
                    total_input_tokens += usage_stats.total_input_tokens;
                    total_output_tokens += usage_stats.total_output_tokens;
                    total_requests += usage_stats.total_requests;
                }
                _ => {
                    // 默认使用总成本（month/all）
                    total_cost += usage_stats.total_cost;
                    total_input_tokens += usage_stats.total_input_tokens;
                    total_output_tokens += usage_stats.total_output_tokens;
                    total_requests += usage_stats.total_requests;
                }
            }
        }
    }

    // 3. 构建响应（匹配前端期望的结构）
    let costs = serde_json::json!({
        "success": true,
        "period": period,
        "data": {
            "totalCosts": {
                "totalCost": total_cost,
                "inputTokens": total_input_tokens,
                "outputTokens": total_output_tokens,
                "requests": total_requests,
                "formatted": {
                    "totalCost": format!("${:.6}", total_cost)
                }
            }
        }
    });

    info!("📊 Usage costs for period '{}': ${:.4}, {} requests",
          period, total_cost, total_requests);

    Ok((StatusCode::OK, Json(costs)))
}

/// 获取使用趋势
async fn get_usage_trend_handler(
    State(_state): State<Arc<AdminRouteState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let granularity = params.get("granularity").map(|s| s.as_str()).unwrap_or("day");
    let days = params.get("days").and_then(|s| s.parse::<i32>().ok()).unwrap_or(7);
    info!("📊 Fetching usage trend: granularity={}, days={}", granularity, days);

    // 占位数据 - 返回空趋势数组
    // TODO: 从 Redis 聚合时间序列数据
    let trend = serde_json::json!({
        "success": true,
        "granularity": granularity,
        "data": []
    });

    Ok((StatusCode::OK, Json(trend)))
}

/// 获取模型统计
async fn get_model_stats_handler(
    State(_state): State<Arc<AdminRouteState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let period = params.get("period").map(|s| s.as_str()).unwrap_or("monthly");
    info!("📊 Fetching model stats for period: {}", period);

    // 占位数据 - 返回空模型统计
    // 前端期望: response.data (数组)
    // TODO: 按模型维度聚合 Redis 数据
    let stats = serde_json::json!({
        "success": true,
        "period": period,
        "data": []  // ← 字段名从 "models" 改为 "data" 以匹配前端期望
    });

    Ok((StatusCode::OK, Json(stats)))
}

/// 获取账号使用趋势
async fn get_account_usage_trend_handler(
    State(_state): State<Arc<AdminRouteState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let granularity = params.get("granularity").map(|s| s.as_str()).unwrap_or("day");
    let days = params.get("days").and_then(|s| s.parse::<i32>().ok()).unwrap_or(7);
    let group = params.get("group").map(|s| s.as_str()).unwrap_or("claude");
    info!("📊 Fetching account usage trend: group={}, granularity={}, days={}", group, granularity, days);

    // 占位数据 - 返回符合前端期望的结构
    // 前端期望: data, topAccounts, totalAccounts, group, groupLabel
    // TODO: 按账号维度聚合 Redis 数据
    let trend = serde_json::json!({
        "success": true,
        "group": group,
        "granularity": granularity,
        "data": [],           // 前端期望 response.data
        "topAccounts": [],    // 前端期望 response.topAccounts
        "totalAccounts": 0,   // 前端期望 response.totalAccounts
        "groupLabel": ""      // 前端期望 response.groupLabel
    });

    Ok((StatusCode::OK, Json(trend)))
}

/// 获取 API Keys 使用趋势
async fn get_api_keys_usage_trend_handler(
    State(_state): State<Arc<AdminRouteState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let granularity = params.get("granularity").map(|s| s.as_str()).unwrap_or("day");
    let days = params.get("days").and_then(|s| s.parse::<i32>().ok()).unwrap_or(7);
    let metric = params.get("metric").map(|s| s.as_str()).unwrap_or("requests");
    info!("📊 Fetching API keys usage trend: metric={}, granularity={}, days={}", metric, granularity, days);

    // 占位数据 - 返回空 API Key 趋势
    // TODO: 按 API Key 维度聚合 Redis 数据
    let trend = serde_json::json!({
        "success": true,
        "metric": metric,
        "granularity": granularity,
        "apiKeys": []
    });

    Ok((StatusCode::OK, Json(trend)))
}

// ============================================================================
// Client & Account Group Handlers
// ============================================================================

/// 获取支持的客户端列表
async fn get_supported_clients_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📱 Fetching supported clients list");

    // 返回支持的客户端列表（与 Node.js 实现保持一致）
    let clients = serde_json::json!({
        "success": true,
        "data": [
            {
                "id": "claude_code",
                "name": "Claude Code",
                "description": "Claude Code command-line interface",
                "icon": "🤖"
            },
            {
                "id": "gemini_cli",
                "name": "Gemini CLI",
                "description": "Google Gemini API command-line interface",
                "icon": "💎"
            },
            {
                "id": "codex_cli",
                "name": "Codex CLI",
                "description": "Cursor/Codex command-line interface",
                "icon": "🔷"
            },
            {
                "id": "droid_cli",
                "name": "Droid CLI",
                "description": "Factory Droid platform command-line interface",
                "icon": "🤖"
            }
        ]
    });

    Ok((StatusCode::OK, Json(clients)))
}

/// 获取账户分组列表
async fn get_account_groups_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("👥 Fetching account groups list");

    // 占位实现 - 返回空分组列表
    // TODO: 实现完整的账户分组功能
    let groups = serde_json::json!({
        "success": true,
        "data": []
    });

    Ok((StatusCode::OK, Json(groups)))
}

/// 获取 Claude Code 版本（统一 User-Agent）
///
/// 返回配置的 Claude Code 版本字符串，用作统一的 User-Agent
/// 前端在添加账户时会请求此端点获取版本信息
async fn get_claude_code_version_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔧 Fetching Claude Code version");

    // 从环境变量获取配置的版本号，如果未设置则使用默认值
    let version = std::env::var("CLAUDE_CODE_VERSION")
        .unwrap_or_else(|_| "1.1.0".to_string());

    let response = json!({
        "success": true,
        "data": {
            "version": version
        }
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 清除 Claude Code 版本缓存
///
/// 占位实现 - 清除版本缓存（如果有缓存机制）
/// 前端在某些情况下会调用此端点重置版本信息
async fn clear_claude_code_version_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("🧹 Clearing Claude Code version cache");

    // 占位实现 - 实际上没有缓存需要清除
    // 返回成功响应即可
    let response = json!({
        "success": true,
        "message": "Version cache cleared"
    });

    Ok((StatusCode::OK, Json(response)))
}

// ============================================================================
// Account Management Placeholder Handlers
// ============================================================================

/// Gemini 账户列表（占位）
async fn list_gemini_accounts_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching Gemini accounts (placeholder)");
    Ok((StatusCode::OK, Json(serde_json::json!({ "success": true, "data": [] }))))
}

/// OpenAI 账户列表（占位）
async fn list_openai_accounts_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching OpenAI accounts (placeholder)");
    Ok((StatusCode::OK, Json(serde_json::json!({ "success": true, "data": [] }))))
}

/// OpenAI Responses 账户列表（占位）
async fn list_openai_responses_accounts_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching OpenAI Responses accounts (placeholder)");
    Ok((StatusCode::OK, Json(serde_json::json!({ "success": true, "data": [] }))))
}

/// Bedrock 账户列表（占位）
async fn list_bedrock_accounts_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching Bedrock accounts (placeholder)");
    Ok((StatusCode::OK, Json(serde_json::json!({ "success": true, "data": [] }))))
}

/// Azure OpenAI 账户列表（占位）
async fn list_azure_openai_accounts_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching Azure OpenAI accounts (placeholder)");
    Ok((StatusCode::OK, Json(serde_json::json!({ "success": true, "data": [] }))))
}

/// Droid 账户列表（占位）
async fn list_droid_accounts_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching Droid accounts (placeholder)");
    Ok((StatusCode::OK, Json(serde_json::json!({ "success": true, "data": [] }))))
}

/// CCR 账户列表（占位）
async fn list_ccr_accounts_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📋 Fetching CCR accounts (placeholder)");
    Ok((StatusCode::OK, Json(serde_json::json!({ "success": true, "data": [] }))))
}

/// 检查更新处理器
///
/// 从 VERSION 文件读取当前版本，从 GitHub API 获取最新版本（带 Redis 缓存）
async fn check_updates_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔄 Checking for updates");

    // 1. 读取当前版本（从 VERSION 文件）
    let current_version = match tokio::fs::read_to_string("VERSION").await {
        Ok(content) => content.trim().to_string(),
        Err(e) => {
            // VERSION 文件不存在或读取失败，从 Cargo.toml 获取
            tracing::warn!("Failed to read VERSION file: {}, using Cargo.toml version", e);
            env!("CARGO_PKG_VERSION").to_string()
        }
    };

    // 2. 从 GitHub API 获取最新版本（简化版：不使用 Redis 缓存）
    // TODO: 添加 Redis 缓存以减少 GitHub API 调用
    let latest_version = match fetch_latest_version_from_github().await {
        Ok(version) => {
            info!("🔄 Fetched latest version from GitHub: {}", version);
            version
        }
        Err(e) => {
            tracing::warn!("Failed to fetch latest version from GitHub: {}, using current as fallback", e);
            // GitHub API 失败，使用当前版本作为 fallback
            current_version.clone()
        }
    };

    // 3. 比较版本
    let has_update = compare_versions(&current_version, &latest_version);

    // 4. 构建响应
    let version_info = serde_json::json!({
        "success": true,
        "data": {
            "current": current_version,
            "latest": latest_version,
            "hasUpdate": has_update,
            "releaseInfo": if has_update {
                Some(format!("New version {} is available", latest_version))
            } else {
                None
            },
            "cached": false
        }
    });

    if has_update {
        info!("🔄 Update available: {} -> {}", current_version, latest_version);
    } else {
        info!("🔄 Already on latest version: {}", current_version);
    }

    Ok((StatusCode::OK, Json(version_info)))
}

/// 从 GitHub API 获取最新版本号
///
/// 查询 GitHub Releases API 获取最新发布版本
async fn fetch_latest_version_from_github() -> Result<String, AppError> {
    // GitHub API endpoint (假设仓库为 anthropics/claude-relay-service)
    // 实际项目应该从配置中读取仓库信息
    let url = "";

    let client = reqwest::Client::builder()
        .user_agent("claude-relay-service")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::InternalError(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch from GitHub: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::InternalError(format!(
            "GitHub API returned status: {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to parse GitHub response: {}", e)))?;

    // 从响应中提取 tag_name (例如 "v1.1.187" 或 "1.1.187")
    let tag_name = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InternalError("No tag_name in GitHub response".to_string()))?;

    // 移除 "v" 前缀（如果存在）
    let version = tag_name.strip_prefix('v').unwrap_or(tag_name).to_string();

    Ok(version)
}

/// 比较版本号
///
/// 简单的版本号比较（假设格式为 "major.minor.patch"）
/// 返回 true 如果 latest > current
fn compare_versions(current: &str, latest: &str) -> bool {
    // 简单实现：按字符串比较
    // 完整实现应该使用 semver crate 进行语义化版本比较
    let current_parts: Vec<u32> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    let latest_parts: Vec<u32> = latest
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    // 逐段比较
    for i in 0..std::cmp::max(current_parts.len(), latest_parts.len()) {
        let current_part = current_parts.get(i).copied().unwrap_or(0);
        let latest_part = latest_parts.get(i).copied().unwrap_or(0);

        if latest_part > current_part {
            return true;
        } else if latest_part < current_part {
            return false;
        }
    }

    false
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use crate::RedisPool;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_login_route() {
        let settings = Settings::new().expect("Failed to create test settings");
        let redis = Arc::new(RedisPool::new(&settings).expect("Failed to create Redis pool"));
        let admin_service = Arc::new(AdminService::new(
            redis.clone(),
            "test_secret_key_at_least_32_chars_long".to_string(),
        ));
        let api_key_service = Arc::new(ApiKeyService::new((*redis).clone(), settings.clone()));

        let app = create_admin_routes(admin_service, api_key_service);

        let request = Request::builder()
            .uri("/auth/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"username":"admin","password":"password"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert!(
            response.status() == StatusCode::OK || response.status() == StatusCode::UNAUTHORIZED
        );
    }
}
