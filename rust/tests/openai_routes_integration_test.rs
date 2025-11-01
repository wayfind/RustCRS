// OpenAI Routes Integration Tests
//
// 测试 OpenAI API 路由层的所有端点

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use claude_relay::{
    models::ApiKeyPermissions,
    routes::{create_openai_router, OpenAIState},
    services::{
        account::ClaudeAccountService, account_scheduler::AccountScheduler, api_key::ApiKeyService,
        unified_openai_scheduler::UnifiedOpenAIScheduler,
    },
    RedisPool, Settings,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

/// 创建测试用的 OpenAIState
async fn create_test_openai_state(
    settings: Settings,
) -> Result<OpenAIState, Box<dyn std::error::Error>> {
    let settings_arc = Arc::new(settings.clone());
    let redis = RedisPool::new(&settings)?;
    let redis_arc = Arc::new(redis);

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

    // Create unified OpenAI scheduler
    let unified_openai_scheduler = Arc::new(UnifiedOpenAIScheduler::new(
        account_service.clone(),
        scheduler.clone(),
        redis_arc.clone(),
        None, // Use default TTL
    ));

    Ok(OpenAIState {
        redis: redis_arc,
        settings: settings_arc,
        account_service,
        api_key_service,
        scheduler,
        unified_openai_scheduler,
    })
}

#[tokio::test]
async fn test_routes_require_authentication() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();
    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    // Test: 无认证的请求应该返回 401
    let request = Request::builder()
        .method(Method::POST)
        .uri("/responses")
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
#[ignore] // Requires real OpenAI account configured
async fn test_responses_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create test API key with OpenAI permission
    let mut key_options = common::TestContext::create_test_key_options("test-openai-responses");
    key_options.permissions = ApiKeyPermissions::OpenAI;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    // Create request body
    let request_body = json!({
        "prompt": "def hello_world():",
        "max_tokens": 100
    });

    // Test: POST /responses
    let request = Request::builder()
        .method(Method::POST)
        .uri("/responses")
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

    // Verify response structure
    assert!(json["id"].is_string());
    assert_eq!(json["object"].as_str().unwrap(), "response");
    assert!(json["choices"].is_array());
}

#[tokio::test]
#[ignore] // Requires real OpenAI account configured
async fn test_v1_responses_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-v1-responses");
    key_options.permissions = ApiKeyPermissions::OpenAI;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    let request_body = json!({
        "prompt": "import numpy as np",
        "max_tokens": 50
    });

    // Test: POST /v1/responses (alternate path)
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/responses")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_usage_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-openai-usage");
    key_options.permissions = ApiKeyPermissions::OpenAI;
    let (raw_key, api_key) = ctx.service.generate_key(key_options).await.unwrap();

    // Record some usage
    ctx.service
        .record_usage(&api_key.id, "code-davinci-002", 100, 50, 0, 0, 0.01)
        .await
        .unwrap();

    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    // Test: GET /usage
    let request = Request::builder()
        .method(Method::GET)
        .uri("/usage")
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
    assert_eq!(json["object"].as_str().unwrap(), "usage");
    assert_eq!(json["total_tokens"].as_i64().unwrap(), 150);
    assert_eq!(json["prompt_tokens"].as_i64().unwrap(), 100);
    assert_eq!(json["completion_tokens"].as_i64().unwrap(), 50);
}

#[tokio::test]
async fn test_key_info_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-openai-key-info");
    key_options.permissions = ApiKeyPermissions::OpenAI;
    let (raw_key, api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    // Test: GET /key-info
    let request = Request::builder()
        .method(Method::GET)
        .uri("/key-info")
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
    assert_eq!(json["name"].as_str().unwrap(), "test-openai-key-info");
    assert_eq!(json["permissions"].as_str().unwrap(), "openai");
    assert!(json["is_active"].as_bool().unwrap());

    // Verify usage stats structure
    assert!(json["usage"].is_object());
    assert!(json["usage"]["total_tokens"].is_number());
    assert!(json["usage"]["prompt_tokens"].is_number());
    assert!(json["usage"]["completion_tokens"].is_number());
}

#[tokio::test]
async fn test_permission_enforcement() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create API key with Claude permission only (should not access OpenAI endpoints)
    let mut key_options = common::TestContext::create_test_key_options("test-openai-permissions");
    key_options.permissions = ApiKeyPermissions::Claude;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    // Try to send a request with Claude-only key (should fail)
    let request_body = json!({
        "prompt": "test",
        "max_tokens": 10
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/responses")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Claude-only key should not access OpenAI endpoints"
    );
}

#[tokio::test]
async fn test_missing_required_fields() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-openai-validation");
    key_options.permissions = ApiKeyPermissions::OpenAI;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    // Test: Missing prompt field
    let request_body = json!({
        "max_tokens": 100
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/responses")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should return 400 when missing required fields"
    );
}

#[tokio::test]
#[ignore] // Requires real OpenAI account configured
async fn test_all_permission_accepts_openai() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create API key with All permission (should access OpenAI endpoints)
    let mut key_options = common::TestContext::create_test_key_options("test-openai-all-perm");
    key_options.permissions = ApiKeyPermissions::All;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    let request_body = json!({
        "prompt": "test with all permission",
        "max_tokens": 10
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/responses")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "All permission should access OpenAI endpoints"
    );
}

#[tokio::test]
async fn test_invalid_token_format() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();
    let state = create_test_openai_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_openai_router(state);

    // Test: Invalid token format
    let request = Request::builder()
        .method(Method::GET)
        .uri("/usage")
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
    async fn test_openai_state_creation() {
        let ctx = common::TestContext::new().await.unwrap();
        let state = create_test_openai_state(ctx.settings.clone()).await;
        assert!(state.is_ok(), "Should create OpenAI state successfully");
    }
}
