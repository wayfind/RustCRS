/// 集成测试: CCR 账户创建端点
///
/// 测试 POST /admin/ccr-accounts 端点的实现
/// 用于修复 ISSUE-UI-012

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use claude_relay::{
    routes::create_admin_routes,
    services::{AdminService, ApiKeyService},
    RedisPool, Settings,
};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

/// Create test services
async fn create_test_services(
    settings: &Settings,
) -> Result<(Arc<AdminService>, Arc<ApiKeyService>), Box<dyn std::error::Error>> {
    let redis = RedisPool::new(settings)?;
    let jwt_secret = settings.security.jwt_secret.clone();

    let admin_service = Arc::new(AdminService::new(Arc::new(redis.clone()), jwt_secret));
    let api_key_service = Arc::new(ApiKeyService::new(redis, settings.clone()));

    Ok((admin_service, api_key_service))
}

#[tokio::test]
async fn test_ccr_account_create_endpoint_exists() {
    // 设置测试环境
    let ctx = common::TestContext::new().await.unwrap();
    let (admin_service, api_key_service) = create_test_services(&ctx.settings).await.unwrap();
    let app = create_admin_routes(admin_service.clone(), api_key_service.clone());

    // 测试: POST /admin/ccr-accounts 端点应该存在
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/ccr-accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "CCR测试账户",
                        "api_url": "https://us3.pincc.ai/api/v1/messages",
                        "api_key": "test_key"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // 端点应该存在并响应 (401 未授权或 400 错误请求，但不是 405 方法不允许)
    assert_ne!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "POST /admin/ccr-accounts 不应该返回 405 Method Not Allowed. 得到 {}",
        response.status()
    );

    println!(
        "✅ ISSUE-UI-012: POST /admin/ccr-accounts 端点存在并返回 {} (不是 405 Method Not Allowed)",
        response.status()
    );
}
