use claude_relay::models::UsageRecord;
// API Routes Integration Tests
//
// 测试 Claude API 路由层的所有端点

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use claude_relay::{
    models::ApiKeyPermissions,
    routes::{create_api_router, ApiState},
    services::{
        account::ClaudeAccountService,
        account_scheduler::AccountScheduler,
        api_key::ApiKeyService,
        bedrock_relay::{BedrockRelayConfig, BedrockRelayService},
        claude_relay::{ClaudeRelayConfig, ClaudeRelayService},
        pricing_service::PricingService,
        unified_claude_scheduler::UnifiedClaudeScheduler,
    },
    RedisPool, Settings,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

/// 创建测试用的 ApiState
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

#[tokio::test]
async fn test_routes_require_authentication() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();
    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Test: 无认证的请求应该返回 401
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/models")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 without authentication"
    );
}

#[tokio::test]
async fn test_list_models_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create test API key
    let key_options = common::TestContext::create_test_key_options("test-models");
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Test: GET /api/v1/models
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/models")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response contains models
    assert!(json["data"].is_array());
    let models = json["data"].as_array().unwrap();
    assert!(models.len() >= 5, "Should have at least 5 Claude models");

    // Check for specific models
    let model_ids: Vec<String> = models
        .iter()
        .map(|m| m["id"].as_str().unwrap().to_string())
        .collect();

    assert!(model_ids.contains(&"claude-3-5-sonnet-20241022".to_string()));
    assert!(model_ids.contains(&"claude-3-5-haiku-20241022".to_string()));
}

#[tokio::test]
async fn test_key_info_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create test API key with specific permissions
    let mut key_options = common::TestContext::create_test_key_options("test-key-info");
    key_options.permissions = ApiKeyPermissions::Claude;
    let (raw_key, api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Test: GET /api/v1/key-info
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/key-info")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify key info
    assert_eq!(json["id"].as_str().unwrap(), api_key.id);
    assert_eq!(json["name"].as_str().unwrap(), "test-key-info");
    assert_eq!(json["permissions"].as_str().unwrap(), "claude");
    assert!(json["is_active"].as_bool().unwrap());

    // Verify usage stats structure
    assert!(json["usage"].is_object());
    assert!(json["usage"]["input_tokens"].is_number());
    assert!(json["usage"]["output_tokens"].is_number());
}

#[tokio::test]
async fn test_me_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let key_options = common::TestContext::create_test_key_options("test-me");
    let (raw_key, api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Test: GET /v1/me
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/me")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify user info
    assert_eq!(json["id"].as_str().unwrap(), api_key.id);
    assert_eq!(json["name"].as_str().unwrap(), "test-me");
    assert!(json["email"].as_str().unwrap().contains(&api_key.id));
    assert_eq!(json["display_name"].as_str().unwrap(), "test-me");
}

#[tokio::test]
async fn test_count_tokens_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let key_options = common::TestContext::create_test_key_options("test-count-tokens");
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Create request body
    let request_body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {
                "role": "user",
                "content": "Hello, how are you?"
            }
        ],
        "system": "You are a helpful assistant"
    });

    // Test: POST /api/v1/messages/count_tokens
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/messages/count_tokens")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify token count (estimation)
    assert!(json["input_tokens"].is_number());
    let tokens = json["input_tokens"].as_u64().unwrap();
    assert!(tokens > 0, "Should estimate some tokens");
}

#[tokio::test]
async fn test_usage_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let key_options = common::TestContext::create_test_key_options("test-usage");
    let (raw_key, api_key) = ctx.service.generate_key(key_options).await.unwrap();

    // Record some usage
    ctx.service
        .record_usage(UsageRecord::new(
                api_key.id.clone(),
                "claude-3-5-sonnet-20241022".to_string(),
                100,
                50,
                10,
                5,
                0.01,,
            ))
        .await
        .unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Test: GET /api/v1/usage
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/usage")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify usage data
    assert!(json["data"].is_array());
    let data = json["data"].as_array().unwrap();
    assert!(!data.is_empty());

    let usage = &data[0];
    assert_eq!(usage["input_tokens"].as_i64().unwrap(), 100);
    assert_eq!(usage["output_tokens"].as_i64().unwrap(), 50);
}

#[tokio::test]
async fn test_organization_usage_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let key_options = common::TestContext::create_test_key_options("test-org-usage");
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Test: GET /v1/organizations/org-123/usage
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/organizations/org-123/usage")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify structure
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_permission_enforcement() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create API key with Gemini permission only (should not access Claude messages)
    let mut key_options = common::TestContext::create_test_key_options("test-permissions");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state.clone());

    // Test 1: Models endpoint should be accessible (read-only, no permission check)
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/models")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Models endpoint should be accessible to all authenticated users"
    );

    // Test 2: Try to send a Claude message with Gemini-only key (should fail)
    let app2 = create_api_router(state);
    let request_body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "test"}],
        "max_tokens": 100
    });

    let request2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/messages")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response2 = app2.oneshot(request2).await.unwrap();
    assert_eq!(
        response2.status(),
        StatusCode::UNAUTHORIZED,
        "Gemini-only key should not access Claude messages endpoint"
    );
}

#[tokio::test]
async fn test_invalid_token_format() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();
    let state = create_test_api_state(ctx.settings.clone()).await.unwrap();
    let app = create_api_router(state);

    // Test: Invalid token format
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/models")
        .header(header::AUTHORIZATION, "InvalidToken")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// Helper tests for common module
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_connection() {
        let ctx = common::TestContext::new().await;
        assert!(ctx.is_ok(), "Should create test context successfully");
    }

    #[tokio::test]
    async fn test_context_creation() {
        let ctx = common::TestContext::new().await.unwrap();
        // Just verify context creation succeeds and encryption key is set
        assert!(!ctx.settings.security.encryption_key.is_empty());
    }
}
