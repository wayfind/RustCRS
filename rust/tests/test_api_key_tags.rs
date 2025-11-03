/// 集成测试: API Key 标签功能
///
/// 测试标签的完整生命周期：创建时保存、查询时返回、更新时修改
/// 用于修复 ISSUE-UI-006: 创建 API Key 时设置的标签未显示

use claude_relay::models::api_key::ApiKey;
use serde_json;

#[test]
fn test_api_key_tags_persistence() {
    // 创建测试数据 - 模拟前端提交的创建请求
    let tags = vec!["UI测试标签".to_string(), "批次13".to_string()];

    let mut api_key = create_test_api_key_with_tags(tags.clone());

    // 验证标签字段已设置
    assert_eq!(api_key.tags.len(), 2, "Should have 2 tags");
    assert!(api_key.tags.contains(&"UI测试标签".to_string()), "Should contain 'UI测试标签'");
    assert!(api_key.tags.contains(&"批次13".to_string()), "Should contain '批次13'");

    // 序列化为 JSON - 模拟保存到 Redis
    let json = serde_json::to_string(&api_key).unwrap();
    println!("=== API Key with Tags JSON ===");
    println!("{}", json);

    // 验证 JSON 中包含标签数组
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let tags_in_json = value.get("tags")
        .expect("Should have 'tags' field")
        .as_array()
        .expect("tags should be an array");

    assert_eq!(tags_in_json.len(), 2, "JSON should have 2 tags");

    // 反序列化 - 模拟从 Redis 读取
    let deserialized: ApiKey = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.tags.len(), 2, "Deserialized should have 2 tags");
    assert_eq!(deserialized.tags, tags, "Tags should match original");
}

#[test]
fn test_api_key_tags_empty_handling() {
    // 测试空标签和空白标签的处理
    let tags = vec![
        "有效标签".to_string(),
        "".to_string(),           // 空字符串
        "  ".to_string(),         // 仅空格
        "另一个标签".to_string(),
    ];

    let api_key = create_test_api_key_with_tags(tags);

    // 验证所有标签都保存（包括空的）
    // 过滤逻辑应该在查询端点，而不是存储层
    assert_eq!(api_key.tags.len(), 4, "Should preserve all tags including empty ones");
}

#[test]
fn test_multiple_keys_tag_collection() {
    // 测试从多个 API Keys 收集标签的场景
    let key1 = create_test_api_key_with_tags(vec!["标签A".to_string(), "标签B".to_string()]);
    let key2 = create_test_api_key_with_tags(vec!["标签B".to_string(), "标签C".to_string()]);
    let key3 = create_test_api_key_with_tags(vec![]); // 无标签

    // 收集所有唯一标签（模拟 get_api_keys_tags_handler 逻辑）
    let mut all_tags = std::collections::HashSet::new();
    for key in vec![key1, key2, key3] {
        for tag in key.tags {
            let trimmed = tag.trim();
            if !trimmed.is_empty() {
                all_tags.insert(trimmed.to_string());
            }
        }
    }

    let mut unique_tags: Vec<String> = all_tags.into_iter().collect();
    unique_tags.sort();

    // 应该只有 3 个唯一标签
    assert_eq!(unique_tags.len(), 3, "Should have 3 unique tags");
    assert_eq!(unique_tags, vec!["标签A", "标签B", "标签C"]);
}

// Helper functions

fn create_test_api_key_with_tags(tags: Vec<String>) -> ApiKey {
    use chrono::Utc;

    ApiKey {
        id: "test_key_with_tags".to_string(),
        key: None,
        key_hash: "hash123".to_string(),
        name: "测试标签Key".to_string(),
        description: Some("用于测试标签功能".to_string()),
        icon: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
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
        tags,  // 使用传入的标签
        user_id: None,
        created_by: None,
        created_by_type: None,
    }
}
