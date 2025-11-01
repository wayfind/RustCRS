// OpenAI API 路由
//
// 实现 OpenAI API 的所有端点，包括：
// - POST /responses, /v1/responses - OpenAI Responses (Codex) API 处理
// - GET /usage - 使用统计
// - GET /key-info - API Key 信息

use axum::{
    extract::State,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;
use tracing::{info, warn};

use crate::config::Settings;
use crate::middleware::auth::AuthState;
use crate::models::{ApiKey, ApiKeyPermissions};
use crate::redis::RedisPool;
use crate::services::{
    account::ClaudeAccountService, account_scheduler::AccountScheduler, api_key::ApiKeyService,
    unified_openai_scheduler::UnifiedOpenAIScheduler,
};
use crate::utils::error::{AppError, Result};
use crate::utils::session_helper;

/// OpenAI API 路由器状态
#[derive(Clone)]
pub struct OpenAIState {
    pub redis: Arc<RedisPool>,
    pub settings: Arc<Settings>,
    pub account_service: Arc<ClaudeAccountService>,
    pub api_key_service: Arc<ApiKeyService>,
    pub scheduler: Arc<AccountScheduler>,
    pub unified_openai_scheduler: Arc<UnifiedOpenAIScheduler>,
}

/// 创建 OpenAI API 路由
pub fn create_router(state: OpenAIState) -> Router {
    // 创建受保护的路由 (需要 API Key 认证)

    Router::new()
        // Responses 端点 (支持两种路径)
        .route("/responses", post(handle_responses))
        .route("/v1/responses", post(handle_responses))
        // 其他端点
        .route("/usage", get(handle_usage))
        .route("/key-info", get(handle_key_info))
        // 应用认证中间件到所有路由
        .layer(middleware::from_fn_with_state(
            state.api_key_service.clone(),
            crate::middleware::auth::authenticate_api_key,
        ))
        .with_state(state)
}

/// Axum 提取器：从请求扩展中提取 API Key
pub struct ApiKeyExtractor(pub ApiKey);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for ApiKeyExtractor
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthState>()
            .map(|auth| ApiKeyExtractor(auth.api_key.clone()))
            .ok_or_else(|| AppError::Unauthorized("Missing authentication".to_string()))
    }
}

/// POST /responses, /v1/responses - OpenAI Responses (Codex) API 处理
///
/// 处理 OpenAI Responses 格式的请求
async fn handle_responses(
    State(state): State<OpenAIState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
    Json(request): Json<JsonValue>,
) -> Result<Response> {
    info!(
        "📨 Processing OpenAI Responses request for key: {}",
        api_key.name
    );

    // 1. 权限验证 - OpenAI 服务权限
    if api_key.permissions != ApiKeyPermissions::All
        && api_key.permissions != ApiKeyPermissions::OpenAI
    {
        warn!("❌ Permission denied for key: {}", api_key.name);
        return Err(AppError::Unauthorized(
            "此 API Key 无权访问 OpenAI 服务".to_string(),
        ));
    }

    // 2. 验证请求体
    if request.get("prompt").is_none() {
        return Err(AppError::BadRequest("prompt 字段不能为空".to_string()));
    }

    // 3. 提取模型
    let model = request
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("code-davinci-002")
        .to_string();

    // 4. 生成会话 Hash (用于粘性会话)
    let session_hash = generate_session_hash(&request);
    info!(
        "📋 Generated session hash: {:?}",
        session_hash.as_deref().unwrap_or("none")
    );

    // 5. 使用统一调度器选择账户
    // TODO: 需要在 UnifiedOpenAIScheduler 中添加 API Key 专属账户绑定支持
    let selected = state
        .unified_openai_scheduler
        .select_account(&api_key, session_hash.as_deref(), Some(&model))
        .await?;

    info!(
        "🎯 Selected OpenAI account: {} (type: {}) for API key: {}",
        selected.account.name, selected.account_type, api_key.name
    );

    // TODO: 实现 OpenAI Responses 转发逻辑
    // 目前先返回简单响应
    Ok(Json(json!({
        "id": "resp_123",
        "object": "response",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "text": format!("OpenAI Responses 实现中 - 使用账户: {}", selected.account.name),
            "index": 0,
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0
        }
    }))
    .into_response())
}

/// GET /usage - 使用统计
async fn handle_usage(
    State(state): State<OpenAIState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("📊 Getting OpenAI usage stats for key: {}", api_key.name);

    let stats = state.api_key_service.get_usage_stats(&api_key.id).await?;

    Ok(Json(json!({
        "object": "usage",
        "total_tokens": stats.total_input_tokens + stats.total_output_tokens,
        "prompt_tokens": stats.total_input_tokens,
        "completion_tokens": stats.total_output_tokens,
    })))
}

/// GET /key-info - API Key 信息
async fn handle_key_info(
    State(state): State<OpenAIState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("🔍 Getting OpenAI key info for: {}", api_key.name);

    let stats = state.api_key_service.get_usage_stats(&api_key.id).await?;

    Ok(Json(json!({
        "id": api_key.id,
        "name": api_key.name,
        "permissions": api_key.permissions,
        "is_active": api_key.is_active,
        "usage": {
            "total_tokens": stats.total_input_tokens + stats.total_output_tokens,
            "prompt_tokens": stats.total_input_tokens,
            "completion_tokens": stats.total_output_tokens,
        }
    })))
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 生成会话 Hash (用于粘性会话)
///
/// 使用智能会话哈希生成逻辑：
/// 1. 优先使用 metadata.user_id 中的 session ID
/// 2. 使用带 cache_control ephemeral 的内容
/// 3. 使用 system 内容
/// 4. 使用第一条消息内容
fn generate_session_hash(request: &JsonValue) -> Option<String> {
    session_helper::generate_session_hash(request)
}
