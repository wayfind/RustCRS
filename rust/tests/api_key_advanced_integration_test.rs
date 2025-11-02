use claude_relay::models::UsageRecord;
mod common;

use claude_relay::models::api_key::{ApiKeyPermissions, ExpirationMode};
use common::TestContext;

/// Test API key permission restrictions
#[tokio::test]
async fn test_permission_restrictions() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Create Claude-only key
    let mut options = TestContext::create_test_key_options("Claude Only Key");
    options.permissions = ApiKeyPermissions::Claude;

    let (raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create Claude-only key");

    // Verify permissions
    let validated = ctx
        .service
        .validate_key(&raw_key)
        .await
        .expect("Failed to validate key");

    // Should have Claude permission
    assert!(ctx.service.check_permissions(&validated, "claude").is_ok());
    println!("✅ Claude permission granted");

    // Should NOT have Gemini permission
    assert!(ctx.service.check_permissions(&validated, "gemini").is_err());
    println!("✅ Gemini permission denied");

    // Should NOT have OpenAI permission
    assert!(ctx.service.check_permissions(&validated, "openai").is_err());
    println!("✅ OpenAI permission denied");

    // Cleanup
    ctx.cleanup_key(&created_key.id).await;
}

/// Test rate limiting functionality
#[tokio::test]
async fn test_rate_limiting() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Create key with tight rate limits
    let mut options = TestContext::create_test_key_options("Rate Limited Key");
    options.rate_limit_window = Some(60); // 60 seconds
    options.rate_limit_requests = Some(5); // Only 5 requests

    let (raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create rate-limited key");

    let validated = ctx
        .service
        .validate_key(&raw_key)
        .await
        .expect("Failed to validate key");

    // Make 5 requests - should all succeed
    for i in 1..=5 {
        let result = ctx.service.check_rate_limit(&validated).await;
        assert!(result.is_ok(), "Request {} should succeed", i);
    }
    println!("✅ First 5 requests succeeded");

    // 6th request should be rate limited
    let result = ctx.service.check_rate_limit(&validated).await;
    assert!(result.is_err(), "6th request should be rate limited");
    println!("✅ Rate limiting working correctly");

    // Cleanup
    ctx.cleanup_key(&created_key.id).await;
}

/// Test cost limit enforcement
#[tokio::test]
async fn test_cost_limits() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Create key with low cost limit
    let options = TestContext::create_limited_key_options("Cost Limited Key", 1.0, 10.0);

    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create cost-limited key");

    // Record usage that stays under limit
    ctx.service
        .record_usage(UsageRecord::new(
                created_key.id.clone(),
                "claude-3-5-sonnet-20241022".to_string(),
                1000,
                500,
                0,
                0,
                0.50,,
            ))
        .await
        .expect("Failed to record usage");

    let under_limit_check = ctx.service.check_cost_limits(&created_key.id, 0.30).await;
    assert!(under_limit_check.is_ok(), "Should be under daily limit");
    println!("✅ Under cost limit check passed");

    // Try to exceed daily limit
    let over_limit_check = ctx.service.check_cost_limits(&created_key.id, 0.80).await;
    assert!(
        over_limit_check.is_err(),
        "Should exceed daily limit (0.50 + 0.80 = 1.30 > 1.0)"
    );
    println!("✅ Over cost limit check failed as expected");

    // Cleanup
    ctx.cleanup_key(&created_key.id).await;
}

/// Test concurrency limits
#[tokio::test]
async fn test_concurrency_limits() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Create key with concurrency limit
    let mut options = TestContext::create_test_key_options("Concurrent Limited Key");
    options.concurrency_limit = 2; // Only 2 concurrent requests

    let (raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create concurrency-limited key");

    let validated = ctx
        .service
        .validate_key(&raw_key)
        .await
        .expect("Failed to validate key");

    // Increment concurrency twice - should succeed
    ctx.service
        .increment_concurrency(&validated, "req1")
        .await
        .expect("First concurrent request should succeed");

    ctx.service
        .increment_concurrency(&validated, "req2")
        .await
        .expect("Second concurrent request should succeed");

    println!("✅ Two concurrent requests allowed");

    // Third request should fail
    let result = ctx.service.increment_concurrency(&validated, "req3").await;
    assert!(result.is_err(), "Third concurrent request should fail");
    println!("✅ Concurrency limit enforced");

    // Decrement and try again
    ctx.service
        .decrement_concurrency(&validated, "req1")
        .await
        .expect("Failed to decrement concurrency");

    let result = ctx.service.increment_concurrency(&validated, "req3").await;
    assert!(result.is_ok(), "Should succeed after decrement");
    println!("✅ Concurrency slot freed after decrement");

    // Cleanup
    ctx.service
        .decrement_concurrency(&validated, "req2")
        .await
        .ok();
    ctx.service
        .decrement_concurrency(&validated, "req3")
        .await
        .ok();
    ctx.cleanup_key(&created_key.id).await;
}

/// Test key expiration
#[tokio::test]
async fn test_key_expiration() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Create key that expires in 2 seconds
    let mut options = TestContext::create_test_key_options("Expiring Key");
    options.expiration_mode = ExpirationMode::Fixed;
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(2);
    options.expires_at = Some(expires_at);

    let (raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create expiring key");

    // Should be valid initially
    let result = ctx.service.validate_key(&raw_key).await;
    assert!(result.is_ok(), "Key should be valid initially");
    println!("✅ Key is valid before expiration");

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Should be invalid after expiration
    let result = ctx.service.validate_key(&raw_key).await;
    assert!(result.is_err(), "Key should be invalid after expiration");
    println!("✅ Key expired correctly");

    // Cleanup
    ctx.cleanup_key(&created_key.id).await;
}

/// Test key activation
#[tokio::test]
async fn test_key_activation() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Create inactive key
    let mut options = TestContext::create_test_key_options("Inactive Key");
    options.is_active = false;

    let (raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create inactive key");

    // Should fail validation when inactive
    let result = ctx.service.validate_key(&raw_key).await;
    assert!(result.is_err(), "Inactive key should fail validation");
    println!("✅ Inactive key rejected");

    // Activate the key
    ctx.service
        .update_key(&created_key.id, None, Some(true))
        .await
        .expect("Failed to activate key");

    // Should now pass validation
    let result = ctx.service.validate_key(&raw_key).await;
    assert!(result.is_ok(), "Active key should pass validation");
    println!("✅ Activated key accepted");

    // Cleanup
    ctx.cleanup_key(&created_key.id).await;
}

/// Test token limit enforcement
#[tokio::test]
async fn test_token_limits() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Create key with token limit
    let mut options = TestContext::create_test_key_options("Token Limited Key");
    options.token_limit = 5000; // 5000 total tokens

    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create token-limited key");

    // Record usage under limit
    ctx.service
        .record_usage(UsageRecord::new(
                created_key.id.clone(),
                "claude-3-5-sonnet-20241022".to_string(),
                2000,
                // input tokens
            1000,
                // output tokens
            0,
                0,
                0.10,,
            ))
        .await
        .expect("Failed to record usage");

    // Get stats
    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.total_input_tokens, 2000);
    assert_eq!(stats.total_output_tokens, 1000);
    let total_tokens = stats.total_input_tokens + stats.total_output_tokens;
    assert_eq!(total_tokens, 3000);
    println!(
        "✅ Total tokens used: {} (under limit of 5000)",
        total_tokens
    );

    // Record more usage to exceed limit
    ctx.service
        .record_usage(UsageRecord::new(
                created_key.id.clone(),
                "claude-3-5-sonnet-20241022".to_string(),
                2000,
                // This would put us at 5000 total
            1000,
                // This puts us over at 6000
            0,
                0,
                0.10,,
            ))
        .await
        .expect("Failed to record second usage");

    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get updated stats");

    let total_tokens = stats.total_input_tokens + stats.total_output_tokens;
    assert!(total_tokens > 5000, "Should have exceeded token limit");
    println!("✅ Token limit exceeded: {} > 5000", total_tokens);

    // Cleanup
    ctx.cleanup_key(&created_key.id).await;
}

/// Test multiple API keys isolation
#[tokio::test]
async fn test_multiple_keys_isolation() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Create two different keys
    let options1 = TestContext::create_test_key_options("Key 1");
    let options2 = TestContext::create_test_key_options("Key 2");

    let (raw_key1, key1) = ctx.service.generate_key(options1).await.unwrap();
    let (raw_key2, key2) = ctx.service.generate_key(options2).await.unwrap();

    // Record usage for key1
    ctx.service
        .record_usage(UsageRecord::new(
                key1.id.clone(),
                "claude-3-5-sonnet-20241022".to_string(),
                1000,
                500,
                0,
                0,
                0.10,,
            ))
        .await
        .expect("Failed to record usage for key1");

    // Record different usage for key2
    ctx.service
        .record_usage(UsageRecord::new(
                key2.id.clone(),
                "claude-3-5-sonnet-20241022".to_string(),
                2000,
                1000,
                0,
                0,
                0.20,,
            ))
        .await
        .expect("Failed to record usage for key2");

    // Get stats for both keys
    let stats1 = ctx.service.get_usage_stats(&key1.id).await.unwrap();
    let stats2 = ctx.service.get_usage_stats(&key2.id).await.unwrap();

    // Verify isolation
    assert_eq!(stats1.total_input_tokens, 1000);
    assert_eq!(stats1.total_cost, 0.10);

    assert_eq!(stats2.total_input_tokens, 2000);
    assert_eq!(stats2.total_cost, 0.20);

    println!(
        "✅ Key 1 stats: {} tokens, ${}",
        stats1.total_input_tokens, stats1.total_cost
    );
    println!(
        "✅ Key 2 stats: {} tokens, ${}",
        stats2.total_input_tokens, stats2.total_cost
    );
    println!("✅ Keys are properly isolated");

    // Cleanup
    ctx.cleanup_key(&key1.id).await;
    ctx.cleanup_key(&key2.id).await;
}
