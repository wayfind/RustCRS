use claude_relay::models::UsageRecord;
mod common;

use claude_relay::models::account::{AccountType, Platform};
use claude_relay::services::{
    AccountScheduler, AccountSchedulerConfig, ClaudeAccountService, SessionMapping,
};
use common::TestContext;
use std::sync::Arc;

/// Test session mapping creation and retrieval
#[tokio::test]
async fn test_session_mapping_lifecycle() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let config = AccountSchedulerConfig::default();
    let scheduler = AccountScheduler::with_config(redis, account_service, config);

    let session_hash = "test_session_123";
    let mapping = SessionMapping {
        account_id: "account_1".to_string(),
        account_type: AccountType::Shared,
        platform: Platform::Claude,
        created_at: chrono::Utc::now().timestamp_millis(),
    };

    // Set session mapping
    scheduler
        .set_session_mapping(session_hash, mapping.clone())
        .await
        .expect("Failed to set session mapping");
    println!("✅ Session mapping created");

    // Get session mapping
    let retrieved = scheduler
        .get_session_mapping(session_hash)
        .await
        .expect("Failed to get session mapping")
        .expect("Session mapping not found");

    assert_eq!(retrieved.account_id, "account_1");
    assert_eq!(retrieved.account_type, AccountType::Shared);
    assert_eq!(retrieved.platform, Platform::Claude);
    println!("✅ Session mapping retrieved successfully");

    // Delete session mapping
    scheduler
        .delete_session_mapping(session_hash)
        .await
        .expect("Failed to delete session mapping");
    println!("✅ Session mapping deleted");

    // Verify deletion
    let deleted = scheduler
        .get_session_mapping(session_hash)
        .await
        .expect("Failed to check deleted mapping");
    assert!(deleted.is_none(), "Session mapping should be deleted");
    println!("✅ Session mapping deletion verified");

    println!("\n🎉 Session mapping lifecycle test passed!");
}

/// Test session mapping TTL extension
#[tokio::test]
async fn test_session_mapping_ttl_extension() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let mut config = AccountSchedulerConfig::default();
    config.sticky_session_ttl_hours = 1;
    config.sticky_session_renewal_threshold_minutes = 30;
    let scheduler = AccountScheduler::with_config(redis, account_service, config);

    let session_hash = "test_session_ttl";
    let mapping = SessionMapping {
        account_id: "account_ttl".to_string(),
        account_type: AccountType::Shared,
        platform: Platform::Claude,
        created_at: chrono::Utc::now().timestamp_millis(),
    };

    // Create session
    scheduler
        .set_session_mapping(session_hash, mapping)
        .await
        .expect("Failed to set session mapping");
    println!("✅ Session created with 1 hour TTL");

    // Try to extend TTL
    let extended = scheduler
        .extend_session_mapping_ttl(session_hash)
        .await
        .expect("Failed to extend TTL");
    println!("✅ TTL extension attempted: {}", extended);

    // Cleanup
    scheduler.delete_session_mapping(session_hash).await.ok();

    println!("\n🎉 Session TTL extension test passed!");
}

/// Test concurrency tracking
#[tokio::test]
async fn test_concurrency_tracking() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let config = AccountSchedulerConfig::default();
    let scheduler = AccountScheduler::with_config(redis, account_service, config);

    let account_id = "test_account_concurrency";

    // Initial concurrency should be 0
    let initial = scheduler
        .get_account_concurrency(account_id)
        .await
        .expect("Failed to get initial concurrency");
    assert_eq!(initial, 0);
    println!("✅ Initial concurrency: {}", initial);

    // Increment concurrency
    scheduler
        .increment_concurrency(account_id, "req_1", Some(60))
        .await
        .expect("Failed to increment concurrency");
    println!("✅ Incremented concurrency (req_1)");

    scheduler
        .increment_concurrency(account_id, "req_2", Some(60))
        .await
        .expect("Failed to increment concurrency");
    println!("✅ Incremented concurrency (req_2)");

    // Check concurrency count
    let count = scheduler
        .get_account_concurrency(account_id)
        .await
        .expect("Failed to get concurrency");
    assert_eq!(count, 2);
    println!("✅ Concurrency count: {}", count);

    // Decrement concurrency
    scheduler
        .decrement_concurrency(account_id, "req_1")
        .await
        .expect("Failed to decrement concurrency");
    println!("✅ Decremented concurrency (req_1)");

    let count_after = scheduler
        .get_account_concurrency(account_id)
        .await
        .expect("Failed to get concurrency");
    assert_eq!(count_after, 1);
    println!("✅ Concurrency after decrement: {}", count_after);

    // Cleanup
    scheduler
        .decrement_concurrency(account_id, "req_2")
        .await
        .ok();

    println!("\n🎉 Concurrency tracking test passed!");
}

/// Test concurrent request limits
#[tokio::test]
async fn test_concurrent_request_limits() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let mut config = AccountSchedulerConfig::default();
    config.concurrent_limit_enabled = true;
    let scheduler = AccountScheduler::with_config(redis, account_service, config);

    let account_id = "test_account_limits";

    // Add multiple concurrent requests
    for i in 1..=3 {
        let request_id = format!("req_{}", i);
        scheduler
            .increment_concurrency(account_id, &request_id, Some(60))
            .await
            .unwrap_or_else(|_| panic!("Failed to increment concurrency for {}", request_id));
        println!("✅ Added concurrent request: {}", request_id);
    }

    let final_count = scheduler
        .get_account_concurrency(account_id)
        .await
        .expect("Failed to get final concurrency");
    assert_eq!(final_count, 3);
    println!("✅ Final concurrent count: {}", final_count);

    // Cleanup
    for i in 1..=3 {
        let request_id = format!("req_{}", i);
        scheduler
            .decrement_concurrency(account_id, &request_id)
            .await
            .ok();
    }

    println!("\n🎉 Concurrent request limits test passed!");
}

/// Test account overload marking
#[tokio::test]
async fn test_account_overload_marking() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let mut config = AccountSchedulerConfig::default();
    config.overload_handling_minutes = 5;
    let scheduler = AccountScheduler::with_config(redis, account_service, config);

    let account_id = "test_account_overload";

    // Initially not overloaded
    let is_overloaded = scheduler
        .is_account_overloaded(account_id)
        .await
        .expect("Failed to check overload status");
    assert!(!is_overloaded);
    println!("✅ Initial state: not overloaded");

    // Mark as overloaded
    scheduler
        .mark_account_overloaded(account_id)
        .await
        .expect("Failed to mark account as overloaded");
    println!("✅ Account marked as overloaded");

    // Verify overloaded
    let is_overloaded_after = scheduler
        .is_account_overloaded(account_id)
        .await
        .expect("Failed to check overload status after marking");
    assert!(is_overloaded_after);
    println!("✅ Overload status verified: {}", is_overloaded_after);

    // Clear overload
    scheduler
        .clear_account_overload(account_id)
        .await
        .expect("Failed to clear overload");
    println!("✅ Overload cleared");

    // Verify cleared
    let is_cleared = scheduler
        .is_account_overloaded(account_id)
        .await
        .expect("Failed to check cleared status");
    assert!(!is_cleared);
    println!("✅ Overload clear verified: not overloaded");

    println!("\n🎉 Account overload marking test passed!");
}

/// Test expired concurrency cleanup
#[tokio::test]
async fn test_expired_concurrency_cleanup() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(claude_relay::RedisPool::new(&ctx.settings).unwrap());
    let settings_arc = Arc::new(ctx.settings.clone());
    let account_service = Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
    let config = AccountSchedulerConfig::default();
    let scheduler = AccountScheduler::with_config(redis, account_service, config);

    let account_id = "test_account_cleanup";

    // Add request with very short TTL (1 second)
    scheduler
        .increment_concurrency(account_id, "short_lived_req", Some(1))
        .await
        .expect("Failed to add short-lived request");
    println!("✅ Added request with 1s TTL");

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    println!("✅ Waited 2 seconds for expiration");

    // Cleanup expired entries
    let cleaned = scheduler
        .cleanup_expired_concurrency(account_id)
        .await
        .expect("Failed to cleanup expired concurrency");
    println!("✅ Cleaned up {} expired entries", cleaned);

    // Verify concurrency is 0
    let count = scheduler
        .get_account_concurrency(account_id)
        .await
        .expect("Failed to get concurrency after cleanup");
    assert_eq!(count, 0);
    println!("✅ Concurrency after cleanup: {}", count);

    println!("\n🎉 Expired concurrency cleanup test passed!");
}
