use claude_relay::models::UsageRecord;
mod common;

use claude_relay::RedisPool;
use common::TestContext;
use std::sync::Arc;

/// Test basic Redis operations
#[tokio::test]
async fn test_redis_basic_operations() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // Get Redis pool from settings
    let redis = Arc::new(RedisPool::new(&ctx.settings).expect("Failed to create Redis pool"));

    // Test SET and GET
    redis
        .set("test_key", "test_value")
        .await
        .expect("Failed to set key");

    let value: Option<String> = redis.get("test_key").await.expect("Failed to get key");

    assert_eq!(value, Some("test_value".to_string()));
    println!("✅ Redis SET/GET successful");

    // Test SETEX with expiry
    redis
        .setex("expiring_key", "will_expire", 1)
        .await
        .expect("Failed to set key with expiry");

    let value: Option<String> = redis
        .get("expiring_key")
        .await
        .expect("Failed to get expiring key");

    assert_eq!(value, Some("will_expire".to_string()));
    println!("✅ Redis SET with expiry successful");

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let value: Option<String> = redis
        .get("expiring_key")
        .await
        .expect("Failed to check expired key");

    assert_eq!(value, None);
    println!("✅ Redis key expiration successful");
}

/// Test Redis DEL operation
#[tokio::test]
async fn test_redis_delete() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).expect("Failed to create Redis pool"));

    // Set a key
    redis
        .set("key_to_delete", "value")
        .await
        .expect("Failed to set key");

    // Delete it
    redis
        .del("key_to_delete")
        .await
        .expect("Failed to delete key");

    // Verify it's gone
    let value: Option<String> = redis
        .get("key_to_delete")
        .await
        .expect("Failed to get deleted key");

    assert_eq!(value, None);
    println!("✅ Redis DELETE successful");
}

/// Test Redis EXISTS operation
#[tokio::test]
async fn test_redis_exists() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).expect("Failed to create Redis pool"));

    // Set a key
    redis
        .set("existing_key", "value")
        .await
        .expect("Failed to set key");

    // Check if exists
    let exists = redis
        .exists("existing_key")
        .await
        .expect("Failed to check key existence");

    assert!(exists);

    let not_exists = redis
        .exists("nonexistent_key")
        .await
        .expect("Failed to check nonexistent key");

    assert!(!not_exists);
    println!("✅ Redis EXISTS successful");
}

/// Test Redis HSET/HGET operations
#[tokio::test]
async fn test_redis_hash_operations() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).expect("Failed to create Redis pool"));

    // Set hash field
    redis
        .hset("test_hash", "field1", "value1")
        .await
        .expect("Failed to set hash field");

    // Get hash field
    let value: Option<String> = redis
        .hget("test_hash", "field1")
        .await
        .expect("Failed to get hash field");

    assert_eq!(value, Some("value1".to_string()));
    println!("✅ Redis HSET/HGET successful");
}

/// Test Redis connection pool
#[tokio::test]
async fn test_redis_connection_pool() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).expect("Failed to create Redis pool"));

    // Execute multiple operations concurrently
    let redis_clone1 = redis.clone();
    let redis_clone2 = redis.clone();
    let redis_clone3 = redis.clone();

    let handle1 = tokio::spawn(async move {
        redis_clone1
            .set("concurrent_key_1", "value1")
            .await
            .unwrap();
    });

    let handle2 = tokio::spawn(async move {
        redis_clone2
            .set("concurrent_key_2", "value2")
            .await
            .unwrap();
    });

    let handle3 = tokio::spawn(async move {
        redis_clone3
            .set("concurrent_key_3", "value3")
            .await
            .unwrap();
    });

    // Wait for all operations to complete
    handle1.await.expect("Task 1 failed");
    handle2.await.expect("Task 2 failed");
    handle3.await.expect("Task 3 failed");

    // Verify all keys were set
    let val1: Option<String> = redis.get("concurrent_key_1").await.unwrap();
    let val2: Option<String> = redis.get("concurrent_key_2").await.unwrap();
    let val3: Option<String> = redis.get("concurrent_key_3").await.unwrap();

    assert_eq!(val1, Some("value1".to_string()));
    assert_eq!(val2, Some("value2".to_string()));
    assert_eq!(val3, Some("value3".to_string()));

    println!("✅ Redis connection pool handles concurrent operations");
}

/// Test Redis PING
#[tokio::test]
async fn test_redis_ping() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).expect("Failed to create Redis pool"));

    // Test ping
    redis.ping().await.expect("Failed to ping Redis");

    println!("✅ Redis PING successful");
}
