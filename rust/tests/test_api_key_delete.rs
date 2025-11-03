/// 集成测试: API Key 软删除功能
///
/// 测试 API Key 的软删除功能是否正确工作
/// 用于修复 ISSUE-UI-008: 删除 API Key 操作未生效

use claude_relay::models::api_key::ApiKey;
use serde_json;

#[test]
fn test_api_key_soft_delete() {
    // 创建测试数据
    let mut api_key = create_test_api_key();

    // 初始状态检查
    assert!(!api_key.is_deleted, "Initial state: is_deleted should be false");
    assert!(api_key.deleted_at.is_none(), "Initial state: deleted_at should be None");
    assert!(api_key.deleted_by.is_none(), "Initial state: deleted_by should be None");

    // 模拟软删除操作
    api_key.is_deleted = true;
    api_key.deleted_at = Some(chrono::Utc::now());
    api_key.deleted_by = Some("admin".to_string());

    // 删除后状态检查
    assert!(api_key.is_deleted, "After delete: is_deleted should be true");
    assert!(api_key.deleted_at.is_some(), "After delete: deleted_at should be set");
    assert!(api_key.deleted_by.is_some(), "After delete: deleted_by should be set");

    // 序列化测试 - 确保删除标志正确序列化
    let json = serde_json::to_string(&api_key).unwrap();
    println!("=== Deleted API Key JSON ===");
    println!("{}", json);

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // 验证序列化后的字段 - 应该是 camelCase
    assert_eq!(value.get("isDeleted").and_then(|v| v.as_bool()), Some(true),
               "Serialized JSON should have isDeleted: true (camelCase)");

    assert_eq!(value.get("isActive").and_then(|v| v.as_bool()), Some(true),
               "Serialized JSON should have isActive: true (camelCase)");

    assert!(value.get("deletedAt").is_some(),
            "Serialized JSON should have deletedAt field (camelCase)");

    assert_eq!(value.get("deletedBy").and_then(|v| v.as_str()), Some("admin"),
               "Serialized JSON should have deletedBy: \"admin\" (camelCase)");

    // 确保没有 snake_case 字段
    assert!(value.get("is_deleted").is_none(),
            "Should NOT have snake_case is_deleted field");

    assert!(value.get("is_active").is_none(),
            "Should NOT have snake_case is_active field");

    assert!(value.get("deleted_by").is_none(),
            "Should NOT have snake_case deleted_by field");
}

#[test]
fn test_api_key_list_filters_deleted() {
    // 创建测试数据集
    let active_key1 = create_test_api_key_with_id("active-1");
    let active_key2 = create_test_api_key_with_id("active-2");
    let mut deleted_key = create_test_api_key_with_id("deleted-1");

    // 标记一个为已删除
    deleted_key.is_deleted = true;
    deleted_key.deleted_at = Some(chrono::Utc::now());

    // 模拟过滤逻辑 (实际在 ApiKeyService::get_all_keys 中实现)
    let all_keys = vec![active_key1.clone(), active_key2.clone(), deleted_key.clone()];

    // 不包括已删除的 keys (include_deleted = false)
    let filtered_keys: Vec<&ApiKey> = all_keys
        .iter()
        .filter(|key| !key.is_deleted)
        .collect();

    assert_eq!(filtered_keys.len(), 2,
               "Should have 2 active keys when excluding deleted");

    assert!(filtered_keys.iter().any(|k| k.id == "active-1"),
            "active-1 should be in the list");

    assert!(filtered_keys.iter().any(|k| k.id == "active-2"),
            "active-2 should be in the list");

    assert!(!filtered_keys.iter().any(|k| k.id == "deleted-1"),
            "deleted-1 should NOT be in the list");

    // 包括已删除的 keys (include_deleted = true)
    let all_keys_including_deleted: Vec<&ApiKey> = all_keys
        .iter()
        .collect();

    assert_eq!(all_keys_including_deleted.len(), 3,
               "Should have 3 keys when including deleted");
}

// Helper functions

fn create_test_api_key() -> ApiKey {
    create_test_api_key_with_id("test_key_id")
}

fn create_test_api_key_with_id(id: &str) -> ApiKey {
    use chrono::Utc;

    ApiKey {
        id: id.to_string(),
        key: None,
        key_hash: "hash123".to_string(),
        name: "Test Key".to_string(),
        description: None,
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
        tags: vec![],
        user_id: None,
        created_by: None,
        created_by_type: None,
    }
}
