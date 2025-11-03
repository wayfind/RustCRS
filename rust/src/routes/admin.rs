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
        .route("/api-keys/:id", put(update_api_key_handler))
        .route("/api-keys/:id", delete(delete_api_key_handler))
        .route("/api-keys/:id/toggle", put(toggle_api_key_handler))
        // 客户端和分组管理
        .route("/supported-clients", get(get_supported_clients_handler))
        .route("/account-groups", get(get_account_groups_handler))
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

    // Mock数据 - 返回空的统计信息
    let dashboard = json!({
        "success": true,
        "stats": {
            "totalKeys": 0,
            "activeKeys": 0,
            "totalAccounts": 0,
            "activeAccounts": 0,
            "todayRequests": 0,
            "totalRequests": 0,
            "systemStatus": "正常",
            "uptime": 0,
            "todayTokens": {
                "total": 0,
                "input": 0,
                "output": 0,
                "cost": 0.0
            },
            "totalTokens": {
                "total": 0,
                "input": 0,
                "output": 0,
                "cost": 0.0
            },
            "realtime": {
                "rpm": 0,
                "tpm": 0,
                "window": 5
            }
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
        "apiKey": response_key
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 更新API Key（Mock实现）
async fn update_api_key_handler(
    Path(id): Path<String>,
    Json(key_request): Json<ApiKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔄 Updating API key: {}", id);

    let response = json!({
        "success": true,
        "message": "API Key更新成功",
        "apiKey": {
            "id": id,
            "name": key_request.name,
            "description": key_request.description,
            "tokenLimit": key_request.token_limit.unwrap_or(1000000)
        }
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 删除API Key（Mock实现）
async fn delete_api_key_handler(Path(id): Path<String>) -> Result<impl IntoResponse, AppError> {
    info!("🗑️  Deleting API key: {}", id);

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

// ============================================================================
// Statistics Handlers
// ============================================================================

/// 获取统计概览
async fn get_stats_overview_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("📊 Fetching stats overview");

    // 简化版统计：返回占位数据
    // TODO: 完整实现需要从 Redis 聚合 API Keys 使用量
    let stats = serde_json::json!({
        "success": true,
        "stats": {
            "totalApiKeys": 0,
            "activeApiKeys": 0,
            "totalUsage": {
                "requests": 0,
                "inputTokens": 0,
                "outputTokens": 0,
                "totalCost": 0.0
            }
        }
    });

    Ok((StatusCode::OK, Json(stats)))
}

/// 获取使用成本统计
async fn get_usage_costs_handler(
    State(_state): State<Arc<AdminRouteState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let period = params.get("period").map(|s| s.as_str()).unwrap_or("today");
    info!("📊 Fetching usage costs for period: {}", period);

    // 占位数据 - 返回基础成本结构
    // TODO: 从 Redis 聚合实际使用量和成本
    let costs = serde_json::json!({
        "success": true,
        "period": period,
        "costs": {
            "totalCost": 0.0,
            "inputTokens": 0,
            "outputTokens": 0,
            "requests": 0
        }
    });

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
    // TODO: 按模型维度聚合 Redis 数据
    let stats = serde_json::json!({
        "success": true,
        "period": period,
        "models": []
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

    // 占位数据 - 返回空账号趋势
    // TODO: 按账号维度聚合 Redis 数据
    let trend = serde_json::json!({
        "success": true,
        "group": group,
        "granularity": granularity,
        "accounts": []
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

/// 检查更新处理器（占位实现）
///
/// 返回当前版本信息，不实际检查 GitHub
/// TODO: 实现完整的版本检查功能
/// - 读取 VERSION 文件
/// - 从 GitHub API 获取最新版本
/// - 比较版本并返回更新信息
/// - 使用 Redis 缓存结果（1小时）
async fn check_updates_handler(
    State(_state): State<Arc<AdminRouteState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔄 Checking for updates (placeholder)");

    // 占位实现：返回当前版本，不检查 GitHub
    // 前端期望的响应格式：
    // {
    //   "success": true,
    //   "data": {
    //     "current": "2.0.0",
    //     "latest": "2.0.0",
    //     "hasUpdate": false,
    //     "releaseInfo": null
    //   }
    // }
    let version_info = serde_json::json!({
        "success": true,
        "data": {
            "current": "2.0.0",
            "latest": "2.0.0",
            "hasUpdate": false,
            "releaseInfo": null,
            "cached": false
        }
    });

    Ok((StatusCode::OK, Json(version_info)))
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
            redis,
            "test_secret_key_at_least_32_chars_long".to_string(),
        ));

        let app = create_admin_routes(admin_service);

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
