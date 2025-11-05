// Claude API 路由
//
// 实现 Claude API 的所有端点，包括：
// - POST /v1/messages - Claude 消息处理 (流式+非流式)
// - POST /v1/messages/count_tokens - Token 计数
// - GET /v1/models - 模型列表
// - GET /v1/key-info - API Key 信息
// - GET /v1/usage - 使用统计
// - GET /v1/me - 用户信息 (Claude Code 客户端)
// - GET /v1/organizations/:org_id/usage - 组织使用统计
//
// 注意：这些路由会被 nest 到 /api 和 /claude 前缀下，形成最终路径：
// - /api/v1/messages (主要端点)
// - /claude/v1/messages (别名)

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::stream::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, warn};

use crate::config::Settings;
use crate::middleware::auth::AuthState;
use crate::models::{ApiKey, ApiKeyPermissions, UsageRecord};
use crate::redis::RedisPool;
use crate::services::{
    account::ClaudeAccountService,
    account_scheduler::AccountScheduler,
    api_key::ApiKeyService,
    bedrock_relay::BedrockRelayService,
    claude_relay::{ClaudeRelayService, ClaudeRequest},
    pricing_service::PricingService,
    relay_trait::{RelayRequest, RelayService},
    unified_claude_scheduler::{SchedulerAccountVariant, UnifiedClaudeScheduler},
};
use crate::utils::error::{AppError, Result};
use crate::utils::session_helper;

/// Claude API 路由器状态
#[derive(Clone)]
pub struct ApiState {
    pub redis: Arc<RedisPool>,
    pub settings: Arc<Settings>,
    pub account_service: Arc<ClaudeAccountService>,
    pub api_key_service: Arc<ApiKeyService>,
    pub scheduler: Arc<AccountScheduler>,
    pub relay_service: Arc<ClaudeRelayService>,
    pub bedrock_service: Arc<BedrockRelayService>,
    pub unified_claude_scheduler: Arc<UnifiedClaudeScheduler>,
    pub pricing_service: Arc<PricingService>,
}

/// 创建 Claude API 路由
pub fn create_router(state: ApiState) -> Router {
    // 创建受保护的路由 (需要 API Key 认证)

    Router::new()
        // Claude Messages API - 主要端点
        .route("/v1/messages", post(handle_messages))
        // Token 计数 API
        .route("/v1/messages/count_tokens", post(handle_count_tokens))
        // 模型列表
        .route("/v1/models", get(handle_list_models))
        // API Key 信息
        .route("/v1/key-info", get(handle_key_info))
        // 使用统计
        .route("/v1/usage", get(handle_usage))
        // 用户信息 (Claude Code 客户端)
        .route("/v1/me", get(handle_me))
        // 组织使用统计
        .route(
            "/v1/organizations/:org_id/usage",
            get(handle_organization_usage),
        )
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

/// POST /api/v1/messages - Claude 消息处理
///
/// 支持流式和非流式响应
async fn handle_messages(
    State(state): State<ApiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
    Json(request): Json<ClaudeRequest>,
) -> Result<Response> {
    info!(
        "📨 Processing messages request for key: {} (stream: {})",
        api_key.name,
        request.stream.unwrap_or(false)
    );

    // 1. 权限验证 - Claude 服务权限
    if api_key.permissions != ApiKeyPermissions::All
        && api_key.permissions != ApiKeyPermissions::Claude
    {
        warn!("❌ Permission denied for key: {}", api_key.name);
        return Err(AppError::Unauthorized(
            "此 API Key 无权访问 Claude 服务".to_string(),
        ));
    }

    // 2. 验证请求体
    validate_messages_request(&request)?;

    // 3. 模型黑名单检查
    if api_key.enable_model_restriction && api_key.restricted_models.contains(&request.model) {
        warn!(
            "❌ Model restricted for key: {} (model: {})",
            api_key.name, request.model
        );
        return Err(AppError::Unauthorized("暂无该模型访问权限".to_string()));
    }

    // 4. 生成会话 Hash (用于粘性会话)
    let session_hash = generate_session_hash(&request);
    info!(
        "📋 Generated session hash: {:?}",
        session_hash.as_deref().unwrap_or("none")
    );

    // 保存 model 和 stream (之后 request 会被 move)
    let model = request.model.clone();
    let stream = request.stream.unwrap_or(false);

    // 5. 使用统一调度器选择账户
    // TODO: 需要在 UnifiedClaudeScheduler 中添加 API Key 专属账户绑定支持
    // Node.js 版本: selectAccountForApiKey(apiKeyData, sessionHash, requestedModel)
    // 当前简化版本: select_account(sessionHash, requestedModel)
    let selected = state
        .unified_claude_scheduler
        .select_account(session_hash.as_deref(), Some(&model))
        .await?;

    info!(
        "🎯 Selected account: {} (type: {}) for API key: {}",
        selected.account.name,
        selected.account_variant.as_str(),
        api_key.name
    );

    // 6. 根据账户类型和流式标志选择转发服务
    // 6.1 流式请求处理
    if stream {
        info!("🌊 Processing streaming request");
        return match selected.account_variant {
            SchedulerAccountVariant::ClaudeOfficial
            | SchedulerAccountVariant::ClaudeConsole
            | SchedulerAccountVariant::Ccr => {
                // 调用流式方法，传入已选择的账户 ID 避免二次选择
                let stream_rx = state
                    .relay_service
                    .relay_request_stream(request, session_hash, Some(format!("claude_acc_{}", selected.account.id)))
                    .await?;

                // 将 mpsc::Receiver 转换为 Stream
                let stream = ReceiverStream::new(stream_rx);

                // 将 StreamChunk 转换为 SSE 事件格式
                use crate::services::claude_relay::StreamChunk;
                let sse_stream = stream.map(|chunk_result| {
                    match chunk_result {
                        Ok(chunk) => match chunk {
                            StreamChunk::Data(data) => {
                                // 原始 SSE 数据，直接传递
                                Ok::<_, std::convert::Infallible>(data)
                            }
                            StreamChunk::Usage(_usage) => {
                                // Usage 已经在 Data 中发送，这里跳过
                                // (ClaudeRelayService 已经在流的最后发送了 message_stop 事件)
                                Ok(bytes::Bytes::new())
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
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("Connection", "keep-alive")
                    .header("X-Accel-Buffering", "no")
                    .body(Body::from_stream(sse_stream))
                    .unwrap())
            }
            SchedulerAccountVariant::Bedrock => {
                info!("🔄 Using BedrockRelayService for bedrock streaming");
                // 将 ClaudeRequest 转换为 RelayRequest
                let relay_request = RelayRequest {
                    model: model.clone(),
                    body: serde_json::to_value(&request)?,
                    session_hash: session_hash.clone(),
                    stream: true,
                };

                // 调用 Bedrock 流式方法
                let stream_rx = state
                    .bedrock_service
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
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("Connection", "keep-alive")
                    .header("X-Accel-Buffering", "no")
                    .body(Body::from_stream(sse_stream))
                    .unwrap())
            }
        };
    }

    // 6.2 非流式请求处理
    let relay_response = match selected.account_variant {
        SchedulerAccountVariant::ClaudeOfficial => {
            info!("🔄 Using ClaudeRelayService for claude-official account");
            state
                .relay_service
                .relay_request(request, session_hash, Some(format!("claude_acc_{}", selected.account.id)))
                .await?
        }
        SchedulerAccountVariant::ClaudeConsole => {
            info!("🔄 Using ClaudeRelayService for claude-console account");
            // Console 账户复用 Claude Official 转发服务，传入已选择的账户 ID
            state
                .relay_service
                .relay_request(request, session_hash, Some(format!("claude_acc_{}", selected.account.id)))
                .await?
        }
        SchedulerAccountVariant::Bedrock => {
            info!("🔄 Using BedrockRelayService for bedrock account");
            // 将 ClaudeRequest 转换为 RelayRequest
            let relay_request = RelayRequest {
                model: model.clone(),
                body: serde_json::to_value(&request)?,
                session_hash: session_hash.clone(),
                stream,
            };
            let generic_response = state.bedrock_service.relay_request(relay_request).await?;

            // 将 GenericRelayResponse 转换为 RelayResponse
            use crate::services::claude_relay::{RelayResponse, Usage};
            RelayResponse {
                status_code: generic_response.status_code,
                headers: generic_response.headers,
                body: generic_response.body,
                account_id: generic_response.account_id,
                account_type: generic_response.account_type,
                usage: generic_response.usage.map(|stats| Usage {
                    input_tokens: stats.input_tokens,
                    output_tokens: stats.output_tokens,
                    cache_creation_input_tokens: stats.cache_creation_tokens,
                    cache_read_input_tokens: stats.cache_read_tokens,
                }),
            }
        }
        SchedulerAccountVariant::Ccr => {
            info!("🔄 Using ClaudeRelayService for ccr account");
            // CCR 账户复用 Claude Official 转发服务，传入已选择的账户 ID
            state
                .relay_service
                .relay_request(request, session_hash, Some(format!("claude_acc_{}", selected.account.id)))
                .await?
        }
    };

    // 6. 记录使用量并计算成本
    if let Some(ref usage) = relay_response.usage {
        // 将 Claude Usage 转换为 PricingService Usage
        let cache_creation = usage.cache_creation_input_tokens.map(|tokens| {
            // 简化版本: 假设所有缓存创建 tokens 都是 1h ephemeral
            crate::services::pricing_service::CacheCreation {
                ephemeral_5m_input_tokens: 0,
                ephemeral_1h_input_tokens: tokens as i64,
            }
        });

        let pricing_usage = crate::services::pricing_service::Usage {
            input_tokens: usage.input_tokens as i64,
            output_tokens: usage.output_tokens as i64,
            cache_creation_input_tokens: usage.cache_creation_input_tokens.unwrap_or(0) as i64,
            cache_read_input_tokens: usage.cache_read_input_tokens.unwrap_or(0) as i64,
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
                usage.cache_creation_input_tokens.unwrap_or(0) as i64,
                usage.cache_read_input_tokens.unwrap_or(0) as i64,
                cost,
            ))
            .await?;
    }

    // 7. 返回响应
    Ok((
        StatusCode::from_u16(relay_response.status_code).unwrap(),
        relay_response.body,
    )
        .into_response())
}

/// POST /api/v1/messages/count_tokens - Token 计数
///
/// 简单的 token 估算 (4 chars ≈ 1 token)
async fn handle_count_tokens(
    State(_state): State<ApiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
    Json(request): Json<ClaudeRequest>,
) -> Result<Json<JsonValue>> {
    info!("📊 Counting tokens for key: {}", api_key.name);

    // 估算 input tokens
    let input_tokens = estimate_tokens(&request);

    Ok(Json(json!({
        "input_tokens": input_tokens,
    })))
}

/// GET /api/v1/models - 模型列表
async fn handle_list_models(
    State(_state): State<ApiState>,
    ApiKeyExtractor(_api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("📋 Listing models");

    Ok(Json(json!({
        "data": [
            {
                "id": "claude-3-5-sonnet-20241022",
                "type": "model",
                "display_name": "Claude 3.5 Sonnet (New)"
            },
            {
                "id": "claude-3-5-sonnet-20240620",
                "type": "model",
                "display_name": "Claude 3.5 Sonnet"
            },
            {
                "id": "claude-3-5-haiku-20241022",
                "type": "model",
                "display_name": "Claude 3.5 Haiku"
            },
            {
                "id": "claude-3-opus-20240229",
                "type": "model",
                "display_name": "Claude 3 Opus"
            },
            {
                "id": "claude-3-sonnet-20240229",
                "type": "model",
                "display_name": "Claude 3 Sonnet"
            }
        ]
    })))
}

/// GET /api/v1/key-info - API Key 信息
async fn handle_key_info(
    State(state): State<ApiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("🔍 Getting key info for: {}", api_key.name);

    // 获取使用统计
    let stats = state.api_key_service.get_usage_stats(&api_key.id).await?;

    Ok(Json(json!({
        "id": api_key.id,
        "name": api_key.name,
        "permissions": api_key.permissions,
        "is_active": api_key.is_active,
        "usage": {
            "input_tokens": stats.total_input_tokens,
            "output_tokens": stats.total_output_tokens,
            "cache_creation_tokens": stats.total_cache_creation_tokens,
            "cache_read_tokens": stats.total_cache_read_tokens,
        }
    })))
}

/// GET /api/v1/usage - 使用统计
async fn handle_usage(
    State(state): State<ApiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
    Query(_params): Query<UsageQuery>,
) -> Result<Json<JsonValue>> {
    info!("📊 Getting usage stats for key: {}", api_key.name);

    let stats = state.api_key_service.get_usage_stats(&api_key.id).await?;

    Ok(Json(json!({
        "data": [{
            "input_tokens": stats.total_input_tokens,
            "output_tokens": stats.total_output_tokens,
            "cache_creation_input_tokens": stats.total_cache_creation_tokens,
            "cache_read_input_tokens": stats.total_cache_read_tokens,
        }]
    })))
}

/// GET /v1/me - 用户信息 (Claude Code 客户端)
async fn handle_me(
    State(_state): State<ApiState>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("👤 Getting user info for key: {}", api_key.name);

    Ok(Json(json!({
        "id": api_key.id,
        "name": api_key.name,
        "email": format!("{}@relay.local", api_key.id),
        "display_name": api_key.name,
    })))
}

/// GET /v1/organizations/:org_id/usage - 组织使用统计
async fn handle_organization_usage(
    State(state): State<ApiState>,
    Path(org_id): Path<String>,
    ApiKeyExtractor(api_key): ApiKeyExtractor,
) -> Result<Json<JsonValue>> {
    info!("📊 Getting organization usage for: {}", org_id);

    let stats = state.api_key_service.get_usage_stats(&api_key.id).await?;

    Ok(Json(json!({
        "data": [{
            "input_tokens": stats.total_input_tokens,
            "output_tokens": stats.total_output_tokens,
        }]
    })))
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 验证 messages 请求
fn validate_messages_request(request: &ClaudeRequest) -> Result<()> {
    if request.messages.is_empty() {
        return Err(AppError::ValidationError(
            "messages 数组不能为空".to_string(),
        ));
    }

    if request.model.is_empty() {
        return Err(AppError::ValidationError("model 不能为空".to_string()));
    }

    Ok(())
}

/// 生成会话 Hash (用于粘性会话)
///
/// 使用智能会话哈希生成逻辑：
/// 1. 优先使用 metadata.user_id 中的 session ID
/// 2. 使用带 cache_control ephemeral 的内容
/// 3. 使用 system 内容
/// 4. 使用第一条消息内容
fn generate_session_hash(request: &ClaudeRequest) -> Option<String> {
    // 将 ClaudeRequest 转换为 JSON Value
    match serde_json::to_value(request) {
        Ok(request_json) => session_helper::generate_session_hash(&request_json),
        Err(e) => {
            warn!("⚠️ Failed to serialize request for session hash: {}", e);
            None
        }
    }
}

/// 简单的 token 估算 (4 chars ≈ 1 token)
fn estimate_tokens(request: &ClaudeRequest) -> u32 {
    let mut total_chars = 0;

    for message in &request.messages {
        total_chars += message.content.len();
    }

    if let Some(ref system) = request.system {
        total_chars += system.len();
    }

    (total_chars / 4) as u32
}

/// 使用统计查询参数
#[derive(Debug, Deserialize)]
struct UsageQuery {
    #[allow(dead_code)]
    start_date: Option<String>,
    #[allow(dead_code)]
    end_date: Option<String>,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::Message;

    #[test]
    fn test_generate_session_hash() {
        let request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            system: Some("You are helpful".to_string()),
            max_tokens: Some(1024),
            temperature: None,
            stream: Some(false),
            metadata: None,
        };

        let hash = generate_session_hash(&request);
        assert!(hash.is_some()); // 应该能生成 hash
        assert_eq!(hash.unwrap().len(), 32); // session_helper 返回 32 字符的 hash
    }

    #[test]
    fn test_estimate_tokens() {
        let request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello World".to_string(), // 11 chars
            }],
            system: Some("System".to_string()), // 6 chars
            max_tokens: Some(1024),
            temperature: None,
            stream: Some(false),
            metadata: None,
        };

        let tokens = estimate_tokens(&request);
        assert_eq!(tokens, 4); // (11 + 6) / 4 = 4
    }

    #[test]
    fn test_validate_messages_request() {
        let valid_request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            system: None,
            max_tokens: Some(1024),
            temperature: None,
            stream: Some(false),
            metadata: None,
        };

        assert!(validate_messages_request(&valid_request).is_ok());

        let invalid_request = ClaudeRequest {
            model: "".to_string(),
            messages: vec![],
            system: None,
            max_tokens: Some(1024),
            temperature: None,
            stream: Some(false),
            metadata: None,
        };

        assert!(validate_messages_request(&invalid_request).is_err());
    }
}
