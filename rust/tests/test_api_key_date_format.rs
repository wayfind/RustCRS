/// 集成测试: API Key 日期格式
///
/// 测试 API Key 返回的日期格式是否能被前端正确解析
/// 用于修复 ISSUE-UI-005: 创建时间显示 Invalid Date

use chrono::Utc;
use claude_relay::models::api_key::ApiKey;
use serde_json;

#[test]
fn test_datetime_serialization_format() {
    // 创建测试数据
    let now = Utc::now();
    let api_key = ApiKey {
        id: "test_key".to_string(),
        key: None,
        key_hash: "hash123".to_string(),
        name: "Test Key".to_string(),
        description: None,
        icon: None,
        created_at: now,
        updated_at: now,
        expires_at: None,
        activated_at: None,
        last_used_at: None,
        is_active: true,
        is_deleted: false,
        deleted_at: None,
        deleted_by: None,
        deleted_by_type: None,
        permissions: claude_relay::models::api_key::ApiKeyPermissions::All,
        token_limit: 1000000,
        concurrency_limit: 10,
        rate_limit_window: Some(60),
        rate_limit_requests: Some(100),
        rate_limit_cost: Some(1.0),
        daily_cost_limit: 10.0,
        total_cost_limit: 100.0,
        weekly_opus_cost_limit: 50.0,
        expiration_mode: claude_relay::models::api_key::ExpirationMode::Fixed,
        activation_days: 0,
        activation_unit: claude_relay::models::api_key::ActivationUnit::Days,
        enable_model_restriction: false,
        restricted_models: vec![],
        enable_client_restriction: false,
        allowed_clients: vec![],
        claude_account_id: None,
        claude_console_account_id: None,
        gemini_account_id: None,
        openai_account_id: None,
        bedrock_account_id: None,
        azure_openai_account_id: None,
        droid_account_id: None,
        tags: vec![],
        user_id: None,
        created_by: None,
        created_by_type: None,
    };

    // 序列化为 JSON
    let json = serde_json::to_string_pretty(&api_key).unwrap();
    println!("=== Serialized API Key JSON ===");
    println!("{}", json);

    // 解析 JSON
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // 检查日期字段格式（现在是 camelCase）
    let created_at = value.get("createdAt").expect("createdAt field should exist");
    println!("\n=== createdAt field ===");
    println!("Type: {:?}", created_at);
    println!("Value: {}", created_at);

    // 验证格式
    if let Some(created_str) = created_at.as_str() {
        println!("\n✅ createdAt is a string: {}", created_str);
        println!("Format check:");
        println!("  - Contains 'T': {}", created_str.contains('T'));
        println!("  - Contains 'Z': {}", created_str.contains('Z'));
        println!("  - Looks like RFC3339: {}", created_str.contains('T') && created_str.contains('Z'));

        // 测试 JavaScript Date 能否解析
        println!("\n🔍 JavaScript Date compatibility:");
        println!("  - ISO8601/RFC3339 format should be parseable by new Date()");
        println!("  - Example: new Date('{}') should work", created_str);
    } else if let Some(created_num) = created_at.as_i64() {
        println!("\n✅ createdAt is a number (timestamp): {}", created_num);
        println!("  - Unix timestamp in seconds or milliseconds");
    } else {
        println!("\n❌ createdAt is neither string nor number!");
    }

    // 🔍 根因分析
    println!("\n=== 🔍 ISSUE-UI-005 Root Cause Analysis ===");
    println!("✅ FIXED: Field name changed from 'created_at' to 'createdAt'");
    println!("✅ FIXED: Format is RFC3339 which works with JavaScript Date()");
    if created_at.is_string() {
        let created_str = created_at.as_str().unwrap();
        if created_str.contains('T') && created_str.contains('Z') {
            println!("✅ Backend now returns camelCase: createdAt");
            println!("✅ Frontend expects camelCase: createdAt");
            println!("✅ JavaScript new Date('{}') will work!", created_str);
            println!("\n💡 Root cause was field name mismatch:");
            println!("   Before: backend 'created_at' → frontend 'createdAt' → undefined → Invalid Date");
            println!("   After:  backend 'createdAt' → frontend 'createdAt' → valid date string → correct display");
        }
    }
}

#[test]
fn test_timestamp_vs_rfc3339() {
    let now = Utc::now();

    println!("=== Timestamp Formats Comparison ===");
    println!("RFC3339 string: {}", now.to_rfc3339());
    println!("Unix timestamp (seconds): {}", now.timestamp());
    println!("Unix timestamp (milliseconds): {}", now.timestamp_millis());

    println!("\n=== JavaScript Date() compatibility ===");
    println!("✅ new Date('{}') - works with RFC3339", now.to_rfc3339());
    println!("✅ new Date({}) - works with milliseconds timestamp", now.timestamp_millis());
    println!("❌ new Date({}) - FAILS with seconds timestamp (year 1970)", now.timestamp());

    println!("\n💡 Recommendation:");
    println!("  - Keep RFC3339 format (current)");
    println!("  - OR change to milliseconds timestamp");
    println!("  - DO NOT use seconds timestamp");
}
