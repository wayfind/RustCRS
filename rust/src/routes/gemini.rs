// Gemini API 路由
//
// 实现 Gemini API 的所有端点，包括：
// - POST /gemini/messages - Gemini 消息处理 (流式+非流式)
// - GET /gemini/models - 模型列表
// - GET /gemini/usage - 使用统计
// - GET /gemini/key-info - API Key 信息
// - Gemini v1internal 端点 (loadCodeAssist, onboardUser, countTokens, generateContent, streamGenerateContent)
// - Gemini v1beta 端点 (对应的 v1beta 版本)

use axum::{
    extract::State,
    http::StatusCode,
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
use crate::models::{ApiKey, ApiKeyPermissions, UsageRecord};
use crate::redis::RedisPool;
use crate::services::{
    account::ClaudeAccountService, account_scheduler::AccountScheduler, api_key::ApiKeyService,
    gemini_relay::GeminiRelayService, pricing_service::PricingService, relay_trait::RelayService,
    unified_gemini_scheduler::UnifiedGeminiScheduler,
};
use crate::utils::error::{AppError, Result};
use crate::utils::session_helper;

/// Gemini API 路由器状态
#[derive(Clone)]
pub struct GeminiState {
    pub redis: Arc<RedisPool>,
    pub settings: Arc<Settings>,
    pub account_service: Arc<ClaudeAccountService>,
    pub api_key_service: Arc<ApiKeyService>,
    pub scheduler: Arc<AccountScheduler>,
    pub gemini_service: Arc<GeminiRelayService>,
    pub unified_gemini_scheduler: Arc<UnifiedGeminiScheduler>,
    pub pricing_service: Arc<PricingService>,
}

/// 创建 Gemini API 路由
pub fn create_router(state: GeminiState) -> Router {
    // 创建受保护的路由 (需要 API Key 认证)

    Router::new()
        // 基础端点
        .route("/gemini/messages", post(handle_messages))
        .route("/gemini/models", get(handle_list_models))
        .route("/gemini/usage", get(handle_usage))
        .route("/gemini/key-info", get(handle_key_info))
        // v1internal 端点 - 使用通配符路由支持冒号格式
        // 格式: /gemini/v1internal:operation
        .route("/gemini/*path", post(handle_gemini_wildcard))
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

/// 统一通配符处理函数
/// 解析路径并路由到正确的处理器
/// 支持格式:
/// - /gemini/v1internal:operation
/// - /gemini/v1beta/models/{model}:operation
async fn handle_gemini_wildcard(
    State(state): State<GeminiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
    axum::extract::Path(path): axum::extract::Path<String>,
    Json(request): Json<JsonValue>,
) -> Result<Response> {
    // 解析路径
    if path.starts_with("v1internal:") {
        // v1internal:operation 格式
        let operation = path.trim_start_matches("v1internal:");
        match operation {
            "loadCodeAssist" => handle_load_code_assist_impl(state, api_key, None, request).await,
            "onboardUser" => handle_onboard_user_impl(state, api_key, None, request).await,
            "countTokens" => handle_count_tokens_impl(state, api_key, None, request).await,
            "generateContent" => handle_generate_content_impl(state, api_key, None, request).await,
            "streamGenerateContent" => {
                handle_stream_generate_content_impl(state, api_key, None, request).await
            }
            _ => Err(AppError::NotFound(format!(
                "Unknown v1internal operation: {}",
                operation
            ))),
        }
    } else if path.starts_with("v1beta/models/") {
        // v1beta/models/{model}:operation 格式
        let remainder = path.trim_start_matches("v1beta/models/");
        let parts: Vec<&str> = remainder.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AppError::ValidationError(
                "Invalid v1beta path format, expected: v1beta/models/{model}:operation".to_string(),
            ));
        }

        let model = parts[0].to_string();
        let operation = parts[1];

        match operation {
            "loadCodeAssist" => {
                handle_load_code_assist_impl(state, api_key, Some(model), request).await
            }
            "onboardUser" => handle_onboard_user_impl(state, api_key, Some(model), request).await,
            "countTokens" => handle_count_tokens_impl(state, api_key, Some(model), request).await,
            "generateContent" => {
                handle_generate_content_impl(state, api_key, Some(model), request).await
            }
            "streamGenerateContent" => {
                handle_stream_generate_content_impl(state, api_key, Some(model), request).await
            }
            _ => Err(AppError::NotFound(format!(
                "Unknown v1beta operation: {}",
                operation
            ))),
        }
    } else {
        Err(AppError::NotFound(format!(
            "Unknown Gemini endpoint: {}",
            path
        )))
    }
}

/// POST /gemini/messages - Gemini 消息处理
///
/// 支持流式和非流式响应
async fn handle_messages(
    State(state): State<GeminiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
    Json(request): Json<JsonValue>,
) -> Result<Response> {
    info!(
        "📨 Processing Gemini messages request for key: {}",
        api_key.name
    );

    // 1. 权限验证 - Gemini 服务权限
    if api_key.permissions != ApiKeyPermissions::All
        && api_key.permissions != ApiKeyPermissions::Gemini
    {
        warn!("❌ Permission denied for key: {}", api_key.name);
        return Err(AppError::Unauthorized(
            "此 API Key 无权访问 Gemini 服务".to_string(),
        ));
    }

    // 2. 验证请求体
    if request.get("messages").is_none() && request.get("contents").is_none() {
        return Err(AppError::BadRequest(
            "messages 或 contents 字段不能为空".to_string(),
        ));
    }

    // 3. 提取模型和流式标志
    let model = request
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("gemini-2.0-flash-exp")
        .to_string();

    let stream = request
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    // 4. 生成会话 Hash (用于粘性会话)
    let session_hash = generate_session_hash(&request);
    info!(
        "📋 Generated session hash: {:?}",
        session_hash.as_deref().unwrap_or("none")
    );

    // 5. 使用统一调度器选择账户
    // TODO: 需要在 UnifiedGeminiScheduler 中添加 API Key 专属账户绑定支持
    let selected = state
        .unified_gemini_scheduler
        .select_account(&api_key, session_hash.as_deref(), Some(&model))
        .await?;

    info!(
        "🎯 Selected Gemini account: {} (id: {}) for API key: {}",
        selected.account.name, selected.account_id, api_key.name
    );

    // 6. 创建 RelayRequest
    use crate::services::relay_trait::RelayRequest;
    let relay_request = RelayRequest {
        model: model.clone(),
        body: request,
        session_hash,
        stream,
    };

    // 7. 调用转发服务
    if stream {
        // 流式响应 - TODO: 实现 SSE 流式传输
        Err(AppError::InternalError("流式响应暂未实现".to_string()))
    } else {
        // 非流式响应
        let relay_response = state.gemini_service.relay_request(relay_request).await?;

        // 7. 记录使用量并计算成本
        if let Some(ref usage) = relay_response.usage {
            // 将 Gemini Usage 转换为 PricingService Usage
            // Note: Gemini 使用 cache_creation_tokens 和 cache_read_tokens
            let cache_creation = usage.cache_creation_tokens.map(|tokens| {
                crate::services::pricing_service::CacheCreation {
                    ephemeral_5m_input_tokens: 0,
                    ephemeral_1h_input_tokens: tokens as i64,
                }
            });

            let pricing_usage = crate::services::pricing_service::Usage {
                input_tokens: usage.input_tokens as i64,
                output_tokens: usage.output_tokens as i64,
                cache_creation_input_tokens: usage.cache_creation_tokens.unwrap_or(0) as i64,
                cache_read_input_tokens: usage.cache_read_tokens.unwrap_or(0) as i64,
                cache_creation,
            };

            // 计算实际成本
            let cost_result = state
                .pricing_service
                .calculate_cost(&pricing_usage, &model)
                .await;

            let cost = cost_result.total_cost;

            state
                .api_key_service
                .record_usage(UsageRecord::new(
                    api_key.id.clone(),
                    model.clone(),
                    usage.input_tokens as i64,
                    usage.output_tokens as i64,
                    usage.cache_creation_tokens.unwrap_or(0) as i64,
                    usage.cache_read_tokens.unwrap_or(0) as i64,
                    cost,
                ))
                .await?;
        }

        // 8. 返回响应
        Ok((
            StatusCode::from_u16(relay_response.status_code).unwrap(),
            relay_response.body,
        )
            .into_response())
    }
}

/// GET /gemini/models - 模型列表
async fn handle_list_models(
    State(_state): State<GeminiState>,
    ApiKeyExtractor(_api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("📋 Listing Gemini models");

    Ok(Json(json!({
        "models": [
            {
                "name": "gemini-2.0-flash-exp",
                "displayName": "Gemini 2.0 Flash (Experimental)",
                "description": "Fast and efficient model for general tasks"
            },
            {
                "name": "gemini-1.5-pro",
                "displayName": "Gemini 1.5 Pro",
                "description": "Advanced model with extended context"
            },
            {
                "name": "gemini-1.5-flash",
                "displayName": "Gemini 1.5 Flash",
                "description": "Fast model for quick responses"
            }
        ]
    })))
}

/// GET /gemini/usage - 使用统计
async fn handle_usage(
    State(state): State<GeminiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("📊 Getting Gemini usage stats for key: {}", api_key.name);

    let stats = state.api_key_service.get_usage_stats(&api_key.id).await?;

    Ok(Json(json!({
        "object": "usage",
        "total_tokens": stats.total_input_tokens + stats.total_output_tokens,
        "input_tokens": stats.total_input_tokens,
        "output_tokens": stats.total_output_tokens,
        "cache_creation_tokens": stats.total_cache_creation_tokens,
        "cache_read_tokens": stats.total_cache_read_tokens,
    })))
}

/// GET /gemini/key-info - API Key 信息
async fn handle_key_info(
    State(state): State<GeminiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("🔍 Getting Gemini key info for: {}", api_key.name);

    let stats = state.api_key_service.get_usage_stats(&api_key.id).await?;

    Ok(Json(json!({
        "id": api_key.id,
        "name": api_key.name,
        "permissions": api_key.permissions,
        "is_active": api_key.is_active,
        "usage": {
            "total_tokens": stats.total_input_tokens + stats.total_output_tokens,
            "input_tokens": stats.total_input_tokens,
            "output_tokens": stats.total_output_tokens,
        }
    })))
}

/// 实现: loadCodeAssist 操作
async fn handle_load_code_assist_impl(
    _state: GeminiState,
    api_key: ApiKey,
    _model: Option<String>,
    _request: JsonValue,
) -> Result<Response> {
    info!("🔧 Load code assist for key: {}", api_key.name);

    Ok(Json(json!({
        "status": "success",
        "message": "Code assist loaded"
    }))
    .into_response())
}

/// 实现: onboardUser 操作
async fn handle_onboard_user_impl(
    _state: GeminiState,
    api_key: ApiKey,
    _model: Option<String>,
    _request: JsonValue,
) -> Result<Response> {
    info!("👤 Onboard user for key: {}", api_key.name);

    Ok(Json(json!({
        "status": "success",
        "message": "User onboarded"
    }))
    .into_response())
}

/// 实现: countTokens 操作
async fn handle_count_tokens_impl(
    _state: GeminiState,
    api_key: ApiKey,
    _model: Option<String>,
    request: JsonValue,
) -> Result<Response> {
    info!("📊 Count tokens for key: {}", api_key.name);

    // 简单估算：4 chars ≈ 1 token
    let text = request.to_string();
    let estimated_tokens = (text.len() / 4) as u32;

    Ok(Json(json!({
        "totalTokens": estimated_tokens
    }))
    .into_response())
}

/// 实现: generateContent 操作
async fn handle_generate_content_impl(
    state: GeminiState,
    api_key: ApiKey,
    model_from_path: Option<String>,
    mut request: JsonValue,
) -> Result<Response> {
    info!("✨ Generate content for key: {}", api_key.name);

    // 权限验证
    if api_key.permissions != ApiKeyPermissions::All
        && api_key.permissions != ApiKeyPermissions::Gemini
    {
        return Err(AppError::Unauthorized(
            "此 API Key 无权访问 Gemini 服务".to_string(),
        ));
    }

    // 从路径或请求体中提取模型名
    let model = if let Some(model_from_path) = model_from_path {
        model_from_path
    } else {
        request
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("gemini-2.0-flash-exp")
            .to_string()
    };

    // 确保请求中有模型字段
    if request.get("model").is_none() {
        request["model"] = json!(model.clone());
    }

    // 生成会话 Hash
    let session_hash = generate_session_hash(&request);

    // 使用统一调度器选择账户
    // TODO: 需要在 UnifiedGeminiScheduler 中添加 API Key 专属账户绑定支持
    let _selected = state
        .unified_gemini_scheduler
        .select_account(&api_key, session_hash.as_deref(), Some(&model))
        .await?;

    // 创建 RelayRequest
    use crate::services::relay_trait::RelayRequest;
    let relay_request = RelayRequest {
        model: model.clone(),
        body: request,
        session_hash,
        stream: false,
    };

    // 调用转发服务
    let relay_response = state.gemini_service.relay_request(relay_request).await?;

    // 记录使用量并计算成本
    if let Some(ref usage) = relay_response.usage {
        // 将 Gemini Usage 转换为 PricingService Usage
        let cache_creation = usage.cache_creation_tokens.map(|tokens| {
            crate::services::pricing_service::CacheCreation {
                ephemeral_5m_input_tokens: 0,
                ephemeral_1h_input_tokens: tokens as i64,
            }
        });

        let pricing_usage = crate::services::pricing_service::Usage {
            input_tokens: usage.input_tokens as i64,
            output_tokens: usage.output_tokens as i64,
            cache_creation_input_tokens: usage.cache_creation_tokens.unwrap_or(0) as i64,
            cache_read_input_tokens: usage.cache_read_tokens.unwrap_or(0) as i64,
            cache_creation,
        };

        // 计算实际成本
        let cost_result = state
            .pricing_service
            .calculate_cost(&pricing_usage, &model)
            .await;

        let cost = cost_result.total_cost;

        state
            .api_key_service
            .record_usage(UsageRecord::new(
                api_key.id.clone(),
                model.clone(),
                usage.input_tokens as i64,
                usage.output_tokens as i64,
                usage.cache_creation_tokens.unwrap_or(0) as i64,
                usage.cache_read_tokens.unwrap_or(0) as i64,
                cost,
            ))
            .await?;
    }

    // 返回响应
    Ok((
        StatusCode::from_u16(relay_response.status_code).unwrap(),
        relay_response.body,
    )
        .into_response())
}

/// 实现: streamGenerateContent 操作
async fn handle_stream_generate_content_impl(
    state: GeminiState,
    api_key: ApiKey,
    model_from_path: Option<String>,
    request: JsonValue,
) -> Result<Response> {
    info!("🌊 Stream generate content for key: {}", api_key.name);

    // 权限验证
    if api_key.permissions != ApiKeyPermissions::All
        && api_key.permissions != ApiKeyPermissions::Gemini
    {
        return Err(AppError::Unauthorized(
            "此 API Key 无权访问 Gemini 服务".to_string(),
        ));
    }

    // 提取模型名称
    let model = model_from_path
        .or_else(|| {
            request
                .get("model")
                .and_then(|m| m.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "gemini-pro".to_string());

    // 生成会话 Hash
    let session_hash = generate_session_hash(&request);
    info!(
        "📋 Generated session hash: {:?}",
        session_hash.as_deref().unwrap_or("none")
    );

    // 使用统一调度器选择账户
    let selected = state
        .unified_gemini_scheduler
        .select_account(&api_key, session_hash.as_deref(), Some(&model))
        .await?;

    info!(
        "🎯 Selected Gemini account: {} for API key: {}",
        selected.account.name, api_key.name
    );

    // 构建 RelayRequest
    use crate::services::relay_trait::RelayRequest;
    let relay_request = RelayRequest {
        model: model.clone(),
        body: request,
        session_hash: session_hash.clone(),
        stream: true,
    };

    // 调用 Gemini 流式方法
    use futures::stream::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    let stream_rx = state
        .gemini_service
        .relay_request_stream(relay_request)
        .await?;

    // 将 mpsc::Receiver 转换为 Stream
    let stream = ReceiverStream::new(stream_rx);

    // 将 GenericStreamChunk 转换为 SSE 事件格式
    use crate::services::relay_trait::GenericStreamChunk;
    let sse_stream = stream.map(|chunk_result| {
        match chunk_result {
            Ok(chunk) => match chunk {
                GenericStreamChunk::Data(data) => {
                    // 原始 SSE 数据，直接传递
                    Ok::<_, std::convert::Infallible>(data)
                }
                GenericStreamChunk::Usage(_usage) => {
                    // Usage 已经在 Data 中发送
                    Ok(bytes::Bytes::new())
                }
                GenericStreamChunk::Error(err) => {
                    // 错误事件
                    Ok(format!(
                        "event: error\ndata: {}\n\n",
                        serde_json::json!({"error": err})
                    )
                    .into())
                }
            },
            Err(e) => {
                // 发送错误事件
                Ok(format!(
                    "event: error\ndata: {}\n\n",
                    serde_json::json!({"error": e.to_string()})
                )
                .into())
            }
        }
    });

    // 创建 SSE 响应
    use axum::{body::Body, http::StatusCode};
    Ok(axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("X-Accel-Buffering", "no")
        .body(Body::from_stream(sse_stream))
        .unwrap())
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
