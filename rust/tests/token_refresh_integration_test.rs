mod common;

use chrono::Utc;
use claude_relay::services::{ClaudeAccountService, TokenRefreshConfig, TokenRefreshService};
use claude_relay::utils::HttpClient;
use common::TestContext;
use std::sync::Arc;

/// Test token expiry detection
#[tokio::test]
async fn test_token_expiring_detection() {
    // Note: is_token_expiring expects expires_at in milliseconds

    // Test with token expiring soon (within default threshold of 10 seconds)
    let expires_soon = Utc::now().timestamp_millis() + 5000; // 5 seconds in milliseconds
    assert!(TokenRefreshService::is_token_expiring(expires_soon, None));
    println!("✅ Token expiring in 5 seconds correctly detected");

    // Test with token not expiring soon
    let expires_later = Utc::now().timestamp_millis() + 3600000; // 1 hour in milliseconds
    assert!(!TokenRefreshService::is_token_expiring(expires_later, None));
    println!("✅ Token expiring in 1 hour correctly not detected as expiring");

    // Test with custom threshold
    let expires_medium = Utc::now().timestamp_millis() + 60000; // 60 seconds in milliseconds
    assert!(TokenRefreshService::is_token_expiring(
        expires_medium,
        Some(120)
    ));
    println!("✅ Token expiring in 1 minute detected with 2-minute threshold");

    // Test with already expired token
    let expired = Utc::now().timestamp_millis() - 10000; // 10 seconds ago in milliseconds
    assert!(TokenRefreshService::is_token_expiring(expired, None));
    println!("✅ Expired token correctly detected");

    println!("\n🎉 Token expiring detection test passed!");
}

/// Test refresh lock acquisition and release
#[tokio::test]
async fn test_refresh_lock_lifecycle() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let http_client = Arc::new(HttpClient::new(&ctx.settings).unwrap());
    let config = TokenRefreshConfig::default();
    let refresh_service =
        TokenRefreshService::with_config(redis, account_service, http_client, config);

    let account_id = "test_account_lock";
    let platform = "claude";

    // Initially should not be locked
    let is_locked = refresh_service
        .is_refresh_locked(account_id, platform)
        .await
        .expect("Failed to check lock status");
    assert!(!is_locked);
    println!("✅ Initial state: not locked");

    // Acquire lock
    let lock_acquired = refresh_service
        .acquire_refresh_lock(account_id, platform)
        .await
        .expect("Failed to acquire lock");
    assert!(lock_acquired);
    println!("✅ Lock acquired successfully");

    // Verify locked
    let is_locked_after = refresh_service
        .is_refresh_locked(account_id, platform)
        .await
        .expect("Failed to check lock status after acquiring");
    assert!(is_locked_after);
    println!("✅ Lock status verified: locked");

    // Try to acquire again (should fail since already locked)
    let lock_acquired_again = refresh_service
        .acquire_refresh_lock(account_id, platform)
        .await
        .expect("Failed to acquire lock again");
    assert!(!lock_acquired_again);
    println!("✅ Second lock acquisition correctly failed");

    // Release lock
    refresh_service
        .release_refresh_lock(account_id, platform)
        .await
        .expect("Failed to release lock");
    println!("✅ Lock released successfully");

    // Verify unlocked
    let is_unlocked = refresh_service
        .is_refresh_locked(account_id, platform)
        .await
        .expect("Failed to check lock status after release");
    assert!(!is_unlocked);
    println!("✅ Lock release verified: unlocked");

    println!("\n🎉 Refresh lock lifecycle test passed!");
}

/// Test lock TTL
#[tokio::test]
async fn test_refresh_lock_ttl() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let http_client = Arc::new(HttpClient::new(&ctx.settings).unwrap());
    let mut config = TokenRefreshConfig::default();
    config.lock_ttl = 60;
    let refresh_service =
        TokenRefreshService::with_config(redis, account_service, http_client, config);

    let account_id = "test_account_ttl";
    let platform = "claude";

    // Acquire lock
    let lock_acquired = refresh_service
        .acquire_refresh_lock(account_id, platform)
        .await
        .expect("Failed to acquire lock");
    assert!(lock_acquired);
    println!("✅ Lock acquired with 60s TTL");

    // Get TTL
    let ttl = refresh_service
        .get_lock_ttl(account_id, platform)
        .await
        .expect("Failed to get TTL");

    assert!(
        ttl > 0 && ttl <= 60,
        "TTL should be between 0 and 60 seconds"
    );
    println!("✅ Lock TTL: {} seconds", ttl);

    // Cleanup
    refresh_service
        .release_refresh_lock(account_id, platform)
        .await
        .ok();

    println!("\n🎉 Refresh lock TTL test passed!");
}

/// Test concurrent lock attempts
#[tokio::test]
async fn test_concurrent_lock_attempts() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let http_client = Arc::new(HttpClient::new(&ctx.settings).unwrap());
    let config = TokenRefreshConfig::default();
    let refresh_service = Arc::new(TokenRefreshService::with_config(
        redis,
        account_service,
        http_client,
        config,
    ));

    let account_id = "test_account_concurrent";
    let platform = "claude";

    // Spawn multiple tasks trying to acquire the same lock
    let mut handles = vec![];
    for i in 0..5 {
        let service = refresh_service.clone();
        let acc_id = account_id.to_string();
        let plat = platform.to_string();

        let handle = tokio::spawn(async move {
            let result = service.acquire_refresh_lock(&acc_id, &plat).await;
            (i, result)
        });
        handles.push(handle);
    }

    // Collect results
    let mut successful_acquisitions = 0;
    for handle in handles {
        let (task_id, result) = handle.await.expect("Task panicked");
        match result {
            Ok(true) => {
                successful_acquisitions += 1;
                println!("✅ Task {} acquired lock", task_id);
            }
            Ok(false) => {
                println!("✅ Task {} correctly failed to acquire lock", task_id);
            }
            Err(e) => {
                panic!("Task {} encountered error: {:?}", task_id, e);
            }
        }
    }

    // Only one task should have acquired the lock
    assert_eq!(
        successful_acquisitions, 1,
        "Exactly one task should acquire the lock"
    );
    println!("✅ Only 1 out of 5 concurrent tasks acquired the lock");

    // Cleanup
    refresh_service
        .release_refresh_lock(account_id, platform)
        .await
        .ok();

    println!("\n🎉 Concurrent lock attempts test passed!");
}
