// Gemini Routes Integration Tests
//
// 测试 Gemini API 路由层的所有端点

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use claude_relay::{
    models::ApiKeyPermissions,
    routes::{create_gemini_router, GeminiState},
    services::{
        account::ClaudeAccountService,
        account_scheduler::AccountScheduler,
        api_key::ApiKeyService,
        gemini_relay::{GeminiRelayConfig, GeminiRelayService},
        pricing_service::PricingService,
        unified_gemini_scheduler::UnifiedGeminiScheduler,
    },
    RedisPool, Settings,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

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
        None, // Use default TTL
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

#[tokio::test]
async fn test_routes_require_authentication() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();
    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Test: 无认证的请求应该返回 401
    let request = Request::builder()
        .method(Method::GET)
        .uri("/gemini/models")
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

    // Create test API key with Gemini permission
    let mut key_options = common::TestContext::create_test_key_options("test-gemini-models");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Test: GET /gemini/models
    let request = Request::builder()
        .method(Method::GET)
        .uri("/gemini/models")
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
    assert!(json["models"].is_array());
    let models = json["models"].as_array().unwrap();
    assert!(models.len() >= 3, "Should have at least 3 Gemini models");

    // Check for specific models
    let model_names: Vec<String> = models
        .iter()
        .map(|m| m["name"].as_str().unwrap().to_string())
        .collect();

    assert!(model_names.contains(&"gemini-2.0-flash-exp".to_string()));
    assert!(model_names.contains(&"gemini-1.5-pro".to_string()));
    assert!(model_names.contains(&"gemini-1.5-flash".to_string()));
}

#[tokio::test]
async fn test_usage_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-gemini-usage");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, api_key) = ctx.service.generate_key(key_options).await.unwrap();

    // Record some usage
    ctx.service
        .record_usage(&api_key.id, "gemini-2.0-flash-exp", 100, 50, 10, 5, 0.01)
        .await
        .unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Test: GET /gemini/usage
    let request = Request::builder()
        .method(Method::GET)
        .uri("/gemini/usage")
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
    assert_eq!(json["input_tokens"].as_i64().unwrap(), 100);
    assert_eq!(json["output_tokens"].as_i64().unwrap(), 50);
    assert_eq!(json["cache_creation_tokens"].as_i64().unwrap(), 10);
    assert_eq!(json["cache_read_tokens"].as_i64().unwrap(), 5);
}

#[tokio::test]
async fn test_key_info_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-gemini-key-info");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Test: GET /gemini/key-info
    let request = Request::builder()
        .method(Method::GET)
        .uri("/gemini/key-info")
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
    assert_eq!(json["name"].as_str().unwrap(), "test-gemini-key-info");
    assert_eq!(json["permissions"].as_str().unwrap(), "gemini");
    assert!(json["is_active"].as_bool().unwrap());

    // Verify usage stats structure
    assert!(json["usage"].is_object());
    assert!(json["usage"]["total_tokens"].is_number());
    assert!(json["usage"]["input_tokens"].is_number());
    assert!(json["usage"]["output_tokens"].is_number());
}

#[tokio::test]
async fn test_count_tokens_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-gemini-count-tokens");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Create request body
    let request_body = json!({
        "model": "gemini-2.0-flash-exp",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Hello, how are you?"}]
            }
        ]
    });

    // Test: POST /gemini/v1internal:countTokens
    let request = Request::builder()
        .method(Method::POST)
        .uri("/gemini/v1internal:countTokens")
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
    assert!(json["totalTokens"].is_number());
    let tokens = json["totalTokens"].as_u64().unwrap();
    assert!(tokens > 0, "Should estimate some tokens");
}

#[tokio::test]
async fn test_v1beta_count_tokens_with_model_path() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-gemini-v1beta-count");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Create request body
    let request_body = json!({
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Test message"}]
            }
        ]
    });

    // Test: POST /gemini/v1beta/models/gemini-1.5-pro:countTokens
    let request = Request::builder()
        .method(Method::POST)
        .uri("/gemini/v1beta/models/gemini-1.5-pro:countTokens")
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

    // Verify response
    assert!(json["totalTokens"].is_number());
}

#[tokio::test]
async fn test_load_code_assist_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options =
        common::TestContext::create_test_key_options("test-gemini-load-code-assist");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Test: POST /gemini/v1internal:loadCodeAssist
    let request = Request::builder()
        .method(Method::POST)
        .uri("/gemini/v1internal:loadCodeAssist")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response
    assert_eq!(json["status"].as_str().unwrap(), "success");
    assert_eq!(json["message"].as_str().unwrap(), "Code assist loaded");
}

#[tokio::test]
async fn test_onboard_user_endpoint() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options = common::TestContext::create_test_key_options("test-gemini-onboard");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Test: POST /gemini/v1internal:onboardUser
    let request = Request::builder()
        .method(Method::POST)
        .uri("/gemini/v1internal:onboardUser")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response
    assert_eq!(json["status"].as_str().unwrap(), "success");
    assert_eq!(json["message"].as_str().unwrap(), "User onboarded");
}

#[tokio::test]
async fn test_permission_enforcement() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    // Create API key with Claude permission only (should not access Gemini)
    let mut key_options = common::TestContext::create_test_key_options("test-gemini-permissions");
    key_options.permissions = ApiKeyPermissions::Claude;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state.clone());

    // Test 1: Models endpoint should be accessible (read-only, no permission check)
    let request = Request::builder()
        .method(Method::GET)
        .uri("/gemini/models")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Models endpoint should be accessible to all authenticated users"
    );

    // Test 2: Try to send a Gemini message with Claude-only key (should fail)
    let app2 = create_gemini_router(state);
    let request_body = json!({
        "model": "gemini-2.0-flash-exp",
        "contents": [{"role": "user", "parts": [{"text": "test"}]}]
    });

    let request2 = Request::builder()
        .method(Method::POST)
        .uri("/gemini/messages")
        .header(header::AUTHORIZATION, format!("Bearer {}", raw_key))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response2 = app2.oneshot(request2).await.unwrap();
    assert_eq!(
        response2.status(),
        StatusCode::UNAUTHORIZED,
        "Claude-only key should not access Gemini messages endpoint"
    );
}

#[tokio::test]
async fn test_invalid_token_format() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();
    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Test: Invalid token format
    let request = Request::builder()
        .method(Method::GET)
        .uri("/gemini/models")
        .header(header::AUTHORIZATION, "InvalidToken")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_missing_required_fields() {
    // Setup
    let ctx = common::TestContext::new().await.unwrap();

    let mut key_options =
        common::TestContext::create_test_key_options("test-gemini-missing-fields");
    key_options.permissions = ApiKeyPermissions::Gemini;
    let (raw_key, _api_key) = ctx.service.generate_key(key_options).await.unwrap();

    let state = create_test_gemini_state(ctx.settings.clone())
        .await
        .unwrap();
    let app = create_gemini_router(state);

    // Test: Request without messages or contents field
    let request_body = json!({
        "model": "gemini-2.0-flash-exp"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/gemini/messages")
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
    async fn test_gemini_state_creation() {
        let ctx = common::TestContext::new().await.unwrap();
        let state = create_test_gemini_state(ctx.settings.clone()).await;
        assert!(
            state.is_ok(),
            "Should create GeminiState successfully: {:?}",
            state.err()
        );
    }
}
