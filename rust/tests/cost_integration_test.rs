// Cost 计算和跟踪集成测试
//
// 测试 API Key 的成本计算、统计和限制功能

mod common;

use common::TestContext;

// ========================================
// 基础成本记录测试
// ========================================

/// 测试基础使用记录和成本累计
#[tokio::test]
async fn test_basic_cost_recording() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // 创建测试 API Key
    let options = TestContext::create_test_key_options("Cost Test Key");
    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录第一次使用
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-5-sonnet-20241022",
            1000, // input tokens
            500,  // output tokens
            0,    // cache creation
            0,    // cache read
            0.15, // cost
        )
        .await
        .expect("Failed to record first usage");

    // 验证统计数据
    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.total_input_tokens, 1000);
    assert_eq!(stats.total_output_tokens, 500);
    assert_eq!(stats.total_cost, 0.15);
    assert_eq!(stats.daily_cost, 0.15);
    println!("✅ First usage recorded: $0.15");

    // 记录第二次使用
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-5-sonnet-20241022",
            2000,
            1000,
            0,
            0,
            0.30,
        )
        .await
        .expect("Failed to record second usage");

    // 验证累计数据
    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get updated stats");

    assert_eq!(stats.total_requests, 2);
    assert_eq!(stats.total_input_tokens, 3000);
    assert_eq!(stats.total_output_tokens, 1500);
    assert_eq!(stats.total_cost, 0.45);
    assert_eq!(stats.daily_cost, 0.45);
    println!("✅ Second usage recorded: total cost $0.45");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

/// 测试多模型成本跟踪
#[tokio::test]
async fn test_multi_model_cost_tracking() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let options = TestContext::create_test_key_options("Multi Model Key");
    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 使用不同模型
    let models = vec![
        ("claude-3-5-sonnet-20241022", 1000, 500, 0.15),
        ("claude-3-opus-20240229", 2000, 1000, 0.50),
        ("claude-3-haiku-20240307", 5000, 2000, 0.10),
    ];

    for (model, input, output, cost) in &models {
        ctx.service
            .record_usage(&created_key.id, model, *input, *output, 0, 0, *cost)
            .await
            .unwrap_or_else(|_| panic!("Failed to record usage for {}", model));
    }

    // 验证总计
    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.total_requests, 3);
    assert_eq!(stats.total_input_tokens, 8000);
    assert_eq!(stats.total_output_tokens, 3500);
    assert_eq!(stats.total_cost, 0.75);
    println!("✅ Total cost across 3 models: $0.75");

    // 验证按模型的统计
    assert_eq!(stats.usage_by_model.len(), 3);
    for (model, input, output, cost) in &models {
        let model_stats = stats
            .usage_by_model
            .get(*model)
            .unwrap_or_else(|| panic!("Model {} stats not found", model));
        assert_eq!(model_stats.input_tokens, *input);
        assert_eq!(model_stats.output_tokens, *output);
        assert_eq!(model_stats.cost, *cost);
        println!("✅ Model {} tracked: ${}", model, cost);
    }

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

// ========================================
// 缓存 Token 成本测试
// ========================================

/// 测试缓存 token 的记录
#[tokio::test]
async fn test_cache_tokens_tracking() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let options = TestContext::create_test_key_options("Cache Test Key");
    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录带缓存的使用
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-5-sonnet-20241022",
            1000, // input tokens
            500,  // output tokens
            2000, // cache creation tokens
            1500, // cache read tokens
            0.25, // cost
        )
        .await
        .expect("Failed to record cached usage");

    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.total_input_tokens, 1000);
    assert_eq!(stats.total_output_tokens, 500);
    assert_eq!(stats.total_cache_creation_tokens, 2000);
    assert_eq!(stats.total_cache_read_tokens, 1500);
    assert_eq!(stats.total_cost, 0.25);
    println!("✅ Cache tokens tracked: creation=2000, read=1500");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

// ========================================
// 成本限制测试
// ========================================

/// 测试每日成本限制
#[tokio::test]
async fn test_daily_cost_limit() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // 创建有每日成本限制的 Key
    let options = TestContext::create_limited_key_options("Daily Limit Key", 1.0, 10.0);

    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录使用，接近但未超过限制
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-5-sonnet-20241022",
            1000,
            500,
            0,
            0,
            0.80,
        )
        .await
        .expect("Failed to record usage");

    // 尝试再次使用，应该被允许（估算 0.15，总共 0.95 < 1.0）
    let check_result = ctx.service.check_cost_limits(&created_key.id, 0.15).await;
    assert!(check_result.is_ok(), "Should be under daily limit");
    println!("✅ Under daily limit: $0.80 + $0.15 = $0.95 < $1.00");

    // 尝试使用超过限制的金额
    let check_result = ctx.service.check_cost_limits(&created_key.id, 0.30).await;
    assert!(
        check_result.is_err(),
        "Should exceed daily limit: $0.80 + $0.30 = $1.10 > $1.00"
    );
    println!("✅ Daily limit enforced: $0.80 + $0.30 = $1.10 > $1.00");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

/// 测试总成本限制
#[tokio::test]
async fn test_total_cost_limit() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // 创建有总成本限制的 Key（每日10.0，总计5.0 - 总计更严格）
    let options = TestContext::create_limited_key_options("Total Limit Key", 10.0, 5.0);

    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录使用
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-5-sonnet-20241022",
            1000,
            500,
            0,
            0,
            4.50,
        )
        .await
        .expect("Failed to record usage");

    // 尝试再次使用，应该被总成本限制阻止
    let check_result = ctx.service.check_cost_limits(&created_key.id, 0.80).await;
    assert!(
        check_result.is_err(),
        "Should exceed total limit: $4.50 + $0.80 = $5.30 > $5.00"
    );
    println!("✅ Total cost limit enforced: $4.50 + $0.80 = $5.30 > $5.00");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

/// 测试 Opus 每周成本限制
#[tokio::test]
async fn test_opus_weekly_cost_limit() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    // 创建有 Opus 每周限制的 Key
    let mut options = TestContext::create_test_key_options("Opus Limit Key");
    options.weekly_opus_cost_limit = 2.0; // $2 Opus 每周限制

    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录 Opus 使用
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-opus-20240229",
            2000,
            1000,
            0,
            0,
            1.50,
        )
        .await
        .expect("Failed to record Opus usage");

    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.weekly_opus_cost, 1.50);
    println!("✅ Opus weekly cost tracked: $1.50");

    // 记录非 Opus 模型使用（不应计入 Opus 限制）
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-5-sonnet-20241022",
            1000,
            500,
            0,
            0,
            0.15,
        )
        .await
        .expect("Failed to record Sonnet usage");

    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get updated stats");

    assert_eq!(stats.weekly_opus_cost, 1.50); // 应该还是 1.50
    assert_eq!(stats.total_cost, 1.65); // 总成本增加
    println!("✅ Non-Opus usage doesn't affect Opus limit");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

// ========================================
// 统计重置测试
// ========================================

/// 测试每日统计重置
#[tokio::test]
async fn test_daily_stats_reset() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let options = TestContext::create_test_key_options("Reset Test Key");
    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录一些使用
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-5-sonnet-20241022",
            1000,
            500,
            0,
            0,
            0.50,
        )
        .await
        .expect("Failed to record usage");

    let stats_before = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats_before.total_cost, 0.50);
    assert_eq!(stats_before.daily_cost, 0.50);
    println!("✅ Before reset: total=$0.50, daily=$0.50");

    // 重置每日统计
    ctx.service
        .reset_daily_stats(&created_key.id)
        .await
        .expect("Failed to reset daily stats");

    let stats_after = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats after reset");

    assert_eq!(stats_after.total_cost, 0.50); // 总成本不变
    assert_eq!(stats_after.daily_cost, 0.0); // 每日成本重置
    println!("✅ After reset: total=$0.50, daily=$0.00");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

// ========================================
// 并发使用成本累计测试
// ========================================

/// 测试并发请求的成本累计
///
/// 使用 Redis 原子操作（HINCRBY, HINCRBYFLOAT）实现并发安全
#[tokio::test]
async fn test_concurrent_cost_accumulation() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let options = TestContext::create_test_key_options("Concurrent Cost Key");
    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 真正的并发记录多次使用
    let mut handles = vec![];
    for i in 0..10 {
        let service = ctx.service.clone();
        let key_id = created_key.id.clone();
        let handle = tokio::spawn(async move {
            service
                .record_usage(
                    &key_id,
                    "claude-3-5-sonnet-20241022",
                    100,  // input
                    50,   // output
                    0,    // cache creation
                    0,    // cache read
                    0.01, // cost
                )
                .await
                .unwrap_or_else(|_| panic!("Failed to record usage {}", i));
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        handle.await.expect("Task failed");
    }

    // 验证所有成本都被正确累计（原子操作保证准确性）
    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.total_requests, 10);
    assert_eq!(stats.total_input_tokens, 1000);
    assert_eq!(stats.total_output_tokens, 500);
    assert!((stats.total_cost - 0.10).abs() < 0.001); // 允许浮点误差
    println!("✅ Concurrent cost accumulation (atomic): 10 requests = $0.10");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

// ========================================
// 边缘情况测试
// ========================================

/// 测试零成本记录
#[tokio::test]
async fn test_zero_cost_recording() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let options = TestContext::create_test_key_options("Zero Cost Key");
    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录零成本使用（例如测试请求）
    ctx.service
        .record_usage(&created_key.id, "test-model", 0, 0, 0, 0, 0.0)
        .await
        .expect("Failed to record zero cost");

    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.total_cost, 0.0);
    println!("✅ Zero cost usage recorded");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}

/// 测试大额成本记录
#[tokio::test]
async fn test_large_cost_recording() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let options = TestContext::create_test_key_options("Large Cost Key");
    let (_raw_key, created_key) = ctx
        .service
        .generate_key(options)
        .await
        .expect("Failed to create key");

    // 记录大额使用
    ctx.service
        .record_usage(
            &created_key.id,
            "claude-3-opus-20240229",
            1_000_000, // 1M input tokens
            500_000,   // 500K output tokens
            0,
            0,
            150.0, // $150
        )
        .await
        .expect("Failed to record large cost");

    let stats = ctx
        .service
        .get_usage_stats(&created_key.id)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.total_input_tokens, 1_000_000);
    assert_eq!(stats.total_output_tokens, 500_000);
    assert_eq!(stats.total_cost, 150.0);
    println!("✅ Large cost recorded: $150.00");

    // 清理
    ctx.cleanup_key(&created_key.id).await;
}
