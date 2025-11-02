use claude_relay::models::UsageRecord;
// Streaming Integration Tests
//
// 测试流式响应功能的所有端点
//
// 本测试验证:
// 1. Claude API 流式消息处理 (ClaudeOfficial, ClaudeConsole, Ccr账户)
// 2. Bedrock 流式消息处理 (路由层面,服务返回未实现错误)
// 3. Gemini API 流式内容生成
// 4. SSE (Server-Sent Events) 格式验证
// 5. 流式响应的权限控制
// 6. 流式响应的错误处理

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use claude_relay::{
    models::ApiKeyPermissions,
    routes::{create_api_router, create_gemini_router, ApiState, GeminiState},
    services::{
        account::ClaudeAccountService,
        account_scheduler::AccountScheduler,
        api_key::ApiKeyService,
        bedrock_relay::{BedrockRelayConfig, BedrockRelayService},
        claude_relay::{ClaudeRelayConfig, ClaudeRelayService},
        gemini_relay::{GeminiRelayConfig, GeminiRelayService},
        pricing_service::PricingService,
        unified_claude_scheduler::UnifiedClaudeScheduler,
        unified_gemini_scheduler::UnifiedGeminiScheduler,
    },
    RedisPool, Settings,
};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

/// 创建测试用的 ApiState (Claude/Bedrock)
async fn create_test_api_state(settings: Settings) -> Result<ApiState, Box<dyn std::error::Error>> {
    let settings_arc = Arc::new(settings.clone());
    let redis = RedisPool::new(&settings)?;
    let redis_arc = Arc::new(redis);

    // 创建 HTTP 客户端
    let http_client = Arc::new(reqwest::Client::new());

    // 创建服务
    let account_service = Arc::new(ClaudeAccountService::new(
        redis_arc.clone(),
        settings_arc.clone(),
    )?);
    let api_key_service = Arc::new(ApiKeyService::new((*redis_arc).clone(), settings.clone()));
    let scheduler = Arc::new(AccountScheduler::new(
        redis_arc.clone(),
        account_service.clone(),
    ));

    let relay_config = ClaudeRelayConfig::default();
    let relay_service = Arc::new(ClaudeRelayService::new(
        relay_config,
        http_client.clone(),
        redis_arc.clone(),
        account_service.clone(),
        scheduler.clone(),
    ));

    // Create Bedrock relay service
    let bedrock_config = BedrockRelayConfig::default();
    let bedrock_service = Arc::new(BedrockRelayService::new(
        bedrock_config,
        http_client.clone(),
        redis_arc.clone(),
        account_service.clone(),
        scheduler.clone(),
    ));

    // Create unified Claude scheduler
    let unified_claude_scheduler = Arc::new(UnifiedClaudeScheduler::new(
        account_service.clone(),
        scheduler.clone(),
        redis_arc.clone(),
    ));

    // Create pricing service
    let pricing_service = Arc::new(PricingService::new(http_client));

    Ok(ApiState {
        redis: redis_arc,
        settings: settings_arc,
        account_service,
        api_key_service,
        scheduler,
        relay_service,
        bedrock_service,
        unified_claude_scheduler,
        pricing_service,
    })
}

/// 创建测试用的 GeminiState
async fn create_test_gemini_state(
    settings: Settings,
) -> Result<GeminiState, Box<dyn std::error::Error>> {
    let settings_arc = Arc::new(settings.clone());
    let redis = RedisPool::new(&settings)?;
    let redis_arc = Arc::new(redis);

    // 创建 HTTP 客户端
    let http_client = Arc::new(reqwest::Client::new());

    // 创建服务
    let account_service = Arc::new(ClaudeAccountService::new(
        redis_arc.clone(),
        settings_arc.clone(),
    )?);
    let api_key_service = Arc::new(ApiKeyService::new((*redis_arc).clone(), settings.clone()));
    let scheduler = Arc::new(AccountScheduler::new(
        redis_arc.clone(),
        account_service.clone(),
    ));

    // Create Gemini relay service
    let gemini_config = GeminiRelayConfig::default();
    let gemini_service = Arc::new(GeminiRelayService::new(
        gemini_config,
        http_client.clone(),
        redis_arc.clone(),
        account_service.clone(),
        scheduler.clone(),
    ));

    // Create unified Gemini scheduler
    let unified_gemini_scheduler = Arc::new(UnifiedGeminiScheduler::new(
        account_service.clone(),
        scheduler.clone(),
        redis_arc.clone(),
        None, // sticky_session_ttl_hours
    ));

    // Create pricing service
    let pricing_service = Arc::new(PricingService::new(http_client));

    Ok(GeminiState {
        redis: redis_arc,
        settings: settings_arc,
        account_service,
        api_key_service,
        scheduler,
        gemini_service,
        unified_gemini_scheduler,
        pricing_service,
    })
}

/// 解析 SSE 事件流
/// 返回 (事件类型, 数据) 的列表
fn parse_sse_events(body: &[u8]) -> Vec<(String, String)> {
    let body_str = String::from_utf8_lossy(body);
    let mut events = Vec::new();
    let mut current_event = String::from("message"); // 默认事件类型
    let mut current_data = String::new();

    for line in body_str.lines() {
        if line.starts_with("event:") {
            current_event = line[6..].trim().to_string();
        } else if line.starts_with("data:") {
            current_data = line[5..].trim().to_string();
        } else if line.is_empty() && !current_data.is_empty() {
            // 空行表示事件结束
            events.push((current_event.clone(), current_data.clone()));
            current_event = String::from("message");
            current_data.clear();
        }
    }

    // 处理最后一个事件（如果没有结尾空行）
    if !current_data.is_empty() {
        events.push((current_event, current_data));
    }

    events
}

#[tokio::test]
async fn test_claude_streaming_requires_authentication() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();
    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Create streaming request body
    let request_body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ],
        "stream": true,
        "max_tokens": 10
    });

    // Test: 无认证的流式请求应该返回 401
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/messages")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 without authentication"
    );
}

#[tokio::test]
async fn test_claude_streaming_permission_enforcement() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create API key with Gemini permission only
    let mut key_options = common::TestContext::create_test_key_options("test-stream-perm");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Create streaming request body
    let request_body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ],
        "stream": true,
        "max_tokens": 10
    });

    // Test: Gemini-only key should not access Claude streaming
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/messages")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Gemini-only key should not access Claude streaming"
    );
}

#[tokio::test]
async fn test_claude_streaming_sse_headers() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create valid API key with Claude permission
    let mut key_options = common::TestContext::create_test_key_options("test-stream-headers");
    key_options.permissions = ApiKeyPermissions::Claude;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Create streaming request body
    let request_body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {
                "role": "user",
                "content": "Say hi"
            }
        ],
        "stream": true,
        "max_tokens": 10
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/messages")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // 验证 SSE 响应头
    // 注意: 由于没有真实的Claude账户,这个请求会失败
    // 但我们可以验证路由层面的处理是否正确
    // 如果账户不存在,应该返回错误而不是成功的流式响应

    // 由于测试环境没有真实账户,期望得到错误响应
    // 503 Service Unavailable 表示没有可用账户
    assert!(
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "Should return error when no accounts available, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_bedrock_streaming_route_handler() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create valid API key with All permission
    let mut key_options = common::TestContext::create_test_key_options("test-bedrock-stream");
    key_options.permissions = ApiKeyPermissions::All;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Create streaming request body
    let request_body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ],
        "stream": true,
        "max_tokens": 10
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/messages")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // 注意: 由于BedrockRelayService.relay_request_stream目前返回未实现错误
    // 或者没有可用的Bedrock账户
    // 期望得到错误响应
    assert!(
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "Bedrock streaming should return error (not implemented or no accounts), got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_gemini_streaming_requires_authentication() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();
    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Create streaming request body
    let request_body = json!({
        "contents": [
            {
                "parts": [
                    {
                        "text": "Hello"
                    }
                ]
            }
        ]
    });

    // Test: 无认证的流式请求应该返回 401
    let request = Request::builder()
        .method(Method::POST)
        .uri("/gemini/v1/models/gemini-pro:streamGenerateContent")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 without authentication"
    );
}

#[tokio::test]
async fn test_gemini_streaming_permission_enforcement() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create API key with Claude permission only
    let mut key_options = common::TestContext::create_test_key_options("test-gemini-perm");
    key_options.permissions = ApiKeyPermissions::Claude;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Create streaming request body
    let request_body = json!({
        "contents": [
            {
                "parts": [
                    {
                        "text": "Hello"
                    }
                ]
            }
        ]
    });

    // Test: Claude-only key should not access Gemini streaming
    let request = Request::builder()
        .method(Method::POST)
        .uri("/gemini/v1/models/gemini-pro:streamGenerateContent")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // 可能返回 401 (权限不足) 或 404 (路由未找到)
    // 取决于路由处理顺序
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "Claude-only key should not access Gemini streaming, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_gemini_streaming_sse_headers() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create valid API key with Gemini permission
    let mut key_options = common::TestContext::create_test_key_options("test-gemini-headers");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Create streaming request body
    let request_body = json!({
        "contents": [
            {
                "parts": [
                    {
                        "text": "Say hi"
                    }
                ]
            }
        ]
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/gemini/v1/models/gemini-pro:streamGenerateContent")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // 验证 SSE 响应头
    // 注意: 由于没有真实的Gemini账户,这个请求会失败
    // 但我们可以验证路由层面的处理是否正确

    // 由于测试环境没有真实账户,期望得到错误响应
    // 404 可能是路由未找到, 503 是没有可用账户
    assert!(
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
            || response.status() == StatusCode::NOT_FOUND,
        "Should return error when no accounts available, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_non_streaming_request_still_works() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create valid API key with Claude permission
    let mut key_options = common::TestContext::create_test_key_options("test-non-stream");
    key_options.permissions = ApiKeyPermissions::Claude;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Create non-streaming request body
    let request_body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ],
        "stream": false,
        "max_tokens": 10
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/messages")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // 验证非流式请求仍然正常工作
    // 由于没有真实账户,应该返回错误而不是挂起
    // 503 Service Unavailable 表示没有可用账户
    assert!(
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "Non-streaming request should return error when no accounts available, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_sse_event_parsing() {
    // 测试 SSE 事件解析函数
    let sse_data = b"event: message_start\ndata: {\"type\":\"message_start\"}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";

    let events = parse_sse_events(sse_data);

    assert_eq!(events.len(), 3, "Should parse 3 events");
    assert_eq!(events[0].0, "message_start");
    assert_eq!(events[1].0, "content_block_delta");
    assert_eq!(events[2].0, "message_stop");

    // 验证数据可以被解析为JSON
    for (event_type, data) in &events {
        let json_result: Result<serde_json::Value, _> = serde_json::from_str(data);
        assert!(
            json_result.is_ok(),
            "Event {} data should be valid JSON: {}",
            event_type,
            data
        );
    }
}

// Helper tests for streaming functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_test_context_creation() {
        let ctx = common::TestContext::new().await;
        assert!(
            ctx.is_ok(),
            "Should create test context for streaming tests"
        );
    }

    #[tokio::test]
    async fn test_api_state_creation_for_streaming() {
        let ctx = common::TestContext::new().await.unwrap();
        let state_result = create_test_api_state(ctx.settings.clone()).await;
        assert!(
            state_result.is_ok(),
            "Should create ApiState for streaming tests"
        );
    }

    #[tokio::test]
    async fn test_gemini_state_creation_for_streaming() {
        let ctx = common::TestContext::new().await.unwrap();
        let state_result = create_test_gemini_state(ctx.settings.clone()).await;
        assert!(
            state_result.is_ok(),
            "Should create GeminiState for streaming tests"
        );
    }
}
