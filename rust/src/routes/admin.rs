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
use crate::services::{AdminService, LoginRequest};
use crate::utils::error::AppError;

// ============================================================================
// Data Structures
// ============================================================================

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
///
pub fn create_admin_routes(admin_service: Arc<AdminService>) -> Router {
    // 认证中间件工厂函数
    let auth_layer = |service: Arc<AdminService>| {
        axum::middleware::from_fn_with_state(service, authenticate_jwt)
    };

    Router::new()
        // 公开路由 - 不需要认证
        .route("/auth/login", post(login_handler))

        // 受保护路由 - 需要JWT认证
        .route("/profile", get(get_profile_handler))
        .route("/auth/user", get(get_profile_handler))
        .route("/oem-settings", get(get_oem_settings_handler))
        .route("/oem-settings", put(update_oem_settings_handler))
        .route("/dashboard", get(get_dashboard_handler))

        // Claude账户管理
        .route("/claude-accounts", get(list_claude_accounts_handler))
        .route("/claude-accounts", post(create_claude_account_handler))
        .route("/claude-accounts/:id", put(update_claude_account_handler))
        .route("/claude-accounts/:id", delete(delete_claude_account_handler))
        .route("/claude-accounts/generate-auth-url", post(generate_auth_url_handler))
        .route("/claude-accounts/exchange-code", post(exchange_code_handler))

        // API Keys管理
        .route("/api-keys", get(list_api_keys_handler))
        .route("/api-keys", post(create_api_key_handler))
        .route("/api-keys/:id", put(update_api_key_handler))
        .route("/api-keys/:id", delete(delete_api_key_handler))
        .route("/api-keys/:id/toggle", put(toggle_api_key_handler))

        // 应用认证中间件到所有受保护路由
        .layer(auth_layer(admin_service.clone()))
        .with_state(admin_service)
}

// ============================================================================
// Authentication Handlers
// ============================================================================

/// 管理员登录处理器
async fn login_handler(
    State(service): State<Arc<AdminService>>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("🔐 Admin login attempt: {}", payload.username);

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
    Ok((StatusCode::OK, Json(json!({
        "success": true,
        "message": "OEM设置已更新",
        "settings": settings
    }))))
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
    info!("🔄 Exchanging authorization code for account: {}", request.name);

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

/// 获取API Keys列表（Mock实现）
async fn list_api_keys_handler() -> Result<impl IntoResponse, AppError> {
    info!("🔑 Listing API keys");

    // Mock数据 - 返回空列表
    let response = json!({
        "success": true,
        "apiKeys": []
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 创建API Key（Mock实现）
async fn create_api_key_handler(
    Json(key_request): Json<ApiKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("➕ Creating API key: {}", key_request.name);

    // Mock实现 - 生成示例API Key
    let api_key = format!("cr_{}", uuid::Uuid::new_v4().simple());

    let response = json!({
        "success": true,
        "message": "API Key创建成功",
        "apiKey": {
            "id": format!("key_{}", uuid::Uuid::new_v4()),
            "key": api_key,
            "name": key_request.name,
            "description": key_request.description,
            "tokenLimit": key_request.token_limit.unwrap_or(1000000),
            "permissions": key_request.permissions.unwrap_or_else(|| "all".to_string()),
            "isActive": true,
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
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
async fn delete_api_key_handler(
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    info!("🗑️  Deleting API key: {}", id);

    let response = json!({
        "success": true,
        "message": "API Key删除成功"
    });

    Ok((StatusCode::OK, Json(response)))
}

/// 启用/禁用API Key（Mock实现）
async fn toggle_api_key_handler(
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RedisPool;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_login_route() {
        let redis = Arc::new(RedisPool::new("redis://localhost:6379", 10).await.unwrap());
        let admin_service = Arc::new(AdminService::new(
            redis,
            "test_secret_key_at_least_32_chars_long".to_string(),
        ));

        let app = create_admin_routes(admin_service);

        let request = Request::builder()
            .uri("/auth/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"username":"admin","password":"password"}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::UNAUTHORIZED
        );
    }
}
