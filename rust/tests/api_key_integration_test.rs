use claude_relay::models::UsageRecord;
mod common;

use common::TestContext;

#[tokio::test]
async fn test_complete_key_lifecycle() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context. Make sure Docker is running.");

    // 1. 创建 API Key
    let mut options = TestContext::create_test_key_options("Integration Test Key");
    options.description = Some("Test key for integration testing".to_string());
    options.tags = vec!["test".to_string(), "integration".to_string()];

    let (raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to generate key");

    println!("✅ Created API Key: {}", created_key.id);
    println!("   Raw Key: {}", raw_key);
    println!("   Name: {}", created_key.name);

    // 2. 验证 API Key
    let validated_key = ctx
        .service
        .validate_key(&raw_key)
        .await
        .expect("Failed to validate key");

    assert_eq!(validated_key.id, created_key.id);
    assert_eq!(validated_key.name, created_key.name);
    println!("✅ Key validation successful");

    // 3. 检查权限
    let has_claude_permission = ctx
        .service
        .check_permissions(&validated_key, "claude")
        .expect("Permission check failed");
    assert!(has_claude_permission);
    println!("✅ Permission check successful");

    // 4. 获取 Key
    let retrieved_key = ctx
        .service
        .get_key(&created_key.id)
        .await
        .expect("Failed to get key");
    assert_eq!(retrieved_key.id, created_key.id);
    println!("✅ Key retrieval successful");

    // 5. 更新 Key
    let updated_key = ctx
        .service
        .update_key(&created_key.id, Some("Updated Test Key".to_string()), None)
        .await
        .expect("Failed to update key");
    assert_eq!(updated_key.name, "Updated Test Key");
    println!("✅ Key update successful");

    // 6. 记录使用统计
    ctx.service
        .record_usage(UsageRecord::new(
                created_key.id.clone(),
                "claude-3-5-sonnet-20241022".to_string(),
                1000,
                500,
                100,
                50,
                0.05,,
            ))
        .await
        .expect("Failed to record usage");
    println!("✅ Usage recording successful");

    // 7. 获取使用统计
    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get usage stats");
    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.total_input_tokens, 1000);
    assert_eq!(stats.total_output_tokens, 500);
    assert_eq!(stats.total_cost, 0.05);
    println!("✅ Usage stats retrieval successful");
    println!("   Total Requests: {}", stats.total_requests);
    println!("   Total Cost: ${:.4}", stats.total_cost);

    // 8. 检查成本限制
    let cost_check = ctx.service.check_cost_limits(&created_key.id, 0.01).await;
    assert!(cost_check.is_ok());
    println!("✅ Cost limit check successful");

    // 9. 软删除
    ctx.service
        .delete_key(&created_key.id, "test_suite")
        .await
        .expect("Failed to delete key");
    println!("✅ Key soft deletion successful");

    // 验证删除后无法验证
    let validation_result = ctx.service.validate_key(&raw_key).await;
    assert!(validation_result.is_err());
    println!("✅ Deleted key validation correctly fails");

    // 10. 恢复 Key
    let restored_key = ctx
        .service
        .restore_key(&created_key.id, "test_suite")
        .await
        .expect("Failed to restore key");
    assert!(!restored_key.is_deleted);
    println!("✅ Key restoration successful");

    // 验证恢复后可以验证
    let validation_result = ctx.service.validate_key(&raw_key).await;
    assert!(validation_result.is_ok());
    println!("✅ Restored key validation successful");

    // 11. 永久删除 (清理)
    ctx.service
        .permanent_delete(&created_key.id)
        .await
        .expect("Failed to permanently delete key");
    println!("✅ Permanent deletion successful");

    // 验证永久删除后无法获取
    let get_result = ctx.service.get_key(&created_key.id).await;
    assert!(get_result.is_err());
    println!("✅ Permanently deleted key correctly not found");

    println!("\n🎉 Complete lifecycle test passed!");
}

#[tokio::test]
async fn test_get_all_keys() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // 创建多个测试 keys
    let mut created_keys = vec![];

    for i in 0..3 {
        let options = TestContext::create_test_key_options(&format!("Test Key {}", i));
        let (_, key) = ctx
            .service
            .generate_key(options)
            .await
            .expect("Failed to create key");
        created_keys.push(key.id.clone());
    }

    println!("✅ Created {} test keys", created_keys.len());

    // 获取所有 keys (不包括已删除)
    let all_keys = ctx
        .service
        .get_all_keys(false)
        .await
        .expect("Failed to get all keys");

    println!("✅ Retrieved {} keys total", all_keys.len());
    assert!(all_keys.len() >= 3, "Should have at least 3 keys");

    // 删除一个 key
    ctx.service
        .delete_key(&created_keys[0], "test")
        .await
        .expect("Failed to delete key");

    // 获取所有 keys (不包括已删除)
    let active_keys = ctx
        .service
        .get_all_keys(false)
        .await
        .expect("Failed to get active keys");

    // 获取所有 keys (包括已删除)
    let all_keys_with_deleted = ctx
        .service
        .get_all_keys(true)
        .await
        .expect("Failed to get all keys including deleted");

    println!("✅ Active keys: {}", active_keys.len());
    println!(
        "✅ All keys (including deleted): {}",
        all_keys_with_deleted.len()
    );

    assert!(
        all_keys_with_deleted.len() > active_keys.len(),
        "Should have more keys when including deleted"
    );

    // 清理
    for key_id in &created_keys {
        ctx.cleanup_key(key_id).await;
    }

    println!("\n🎉 Get all keys test passed!");
}

#[tokio::test]
async fn test_cost_limit_enforcement() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // 创建有成本限制的 key
    let options = TestContext::create_limited_key_options("Cost Limited Key", 1.0, 10.0);

    let (_, key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");
    println!("✅ Created cost-limited key: {}", key.id);

    // 记录一些使用 (总计 $0.5)
    ctx.service
        .record_usage(UsageRecord::new(
                key.id.clone(),
                "claude-3-5-sonnet-20241022".to_string(),
                1000,
                500,
                0,
                0,
                0.5,
            ))
        .await
        .expect("Failed to record usage");

    let stats_after_first = ctx
        .service
        .get_usage_stats(&key.id)
        .await
        .expect("Failed to get stats");
    println!(
        "📊 After first usage - Daily: ${:.2}, Total: ${:.2}",
        stats_after_first.daily_cost, stats_after_first.total_cost
    );

    // 检查 $0.3 应该通过 (总计会是 $0.8)
    let check_result = ctx.service.check_cost_limits(&key.id, 0.3).await;
    assert!(check_result.is_ok(), "Should allow $0.3 when total is $0.5");
    println!("✅ Cost limit check passed for $0.3");

    // 检查 $1.0 应该失败 (总计会是 $1.5,超过每日限制 $1.0)
    let check_result = ctx.service.check_cost_limits(&key.id, 1.0).await;
    println!("📊 Check result for $1.0: {:?}", check_result);
    assert!(
        check_result.is_err(),
        "Should reject $1.0 when it exceeds daily limit"
    );
    println!("✅ Cost limit correctly enforced for $1.0");

    // 清理
    ctx.cleanup_key(&key.id).await;

    println!("\n🎉 Cost limit enforcement test passed!");
}

#[tokio::test]
async fn test_stats_reset() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // 创建测试 key
    let options = TestContext::create_test_key_options("Stats Reset Test Key");

    let (_, key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录使用
    ctx.service
        .record_usage(UsageRecord::new(
                key.id.clone(),
                "test-model".to_string(),
                100,
                50,
                0,
                0,
                0.01,
            ))
        .await
        .expect("Failed to record usage");

    let stats = ctx
        .service
        .get_usage_stats(&key.id)
        .await
        .expect("Failed to get stats");
    assert_eq!(stats.daily_cost, 0.01);
    println!("✅ Initial daily cost: ${:.4}", stats.daily_cost);

    // 重置每日统计
    ctx.service
        .reset_daily_stats(&key.id)
        .await
        .expect("Failed to reset daily stats");

    let stats = ctx
        .service
        .get_usage_stats(&key.id)
        .await
        .expect("Failed to get stats");
    assert_eq!(stats.daily_cost, 0.0);
    assert_eq!(stats.total_cost, 0.01); // 总成本不应该重置
    println!("✅ Daily cost after reset: ${:.4}", stats.daily_cost);
    println!("✅ Total cost unchanged: ${:.4}", stats.total_cost);

    // 清理
    ctx.cleanup_key(&key.id).await;

    println!("\n🎉 Stats reset test passed!");
}
