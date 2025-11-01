mod common;

use chrono::Utc;
use claude_relay::models::account::{AccountType, CreateClaudeAccountOptions, Platform};
use claude_relay::services::ClaudeAccountService;
use claude_relay::RedisPool;
use common::TestContext;
use std::sync::Arc;

/// Test account creation and retrieval
#[tokio::test]
async fn test_account_create_and_get() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).unwrap());
    let settings = Arc::new(ctx.settings.clone());
    let account_service = ClaudeAccountService::new(redis, settings).unwrap();

    // Create account options
    let options = CreateClaudeAccountOptions {
        name: "Test Account".to_string(),
        description: Some("Integration test account".to_string()),
        account_type: AccountType::Shared,
        platform: Platform::Claude,
        is_active: true,
        schedulable: true,
        priority: 10,
        proxy: None,
        refresh_token: Some("test_refresh_token".to_string()),
        claude_ai_oauth: None,
        subscription_info: None,
        email: None,
        password: None,
        auto_stop_on_warning: false,
        use_unified_user_agent: false,
        use_unified_client_id: false,
        unified_client_id: None,
        expires_at: None,
        ext_info: None,
    };

    // Create account
    let account = account_service
        .create_account(options)
        .await
        .expect("Failed to create account");

    println!("✅ Created account: {}", account.id);
    assert_eq!(account.name, "Test Account");
    assert_eq!(account.account_type, AccountType::Shared);
    assert_eq!(account.platform, Platform::Claude);
    assert_eq!(account.priority, 10);

    // Retrieve account
    let retrieved = account_service
        .get_account(&account.id.to_string())
        .await
        .expect("Failed to get account")
        .expect("Account not found");

    assert_eq!(retrieved.id, account.id);
    assert_eq!(retrieved.name, account.name);
    println!("✅ Account retrieval successful");

    // Cleanup
    account_service
        .delete_account(&account.id.to_string())
        .await
        .expect("Failed to delete account");
}

/// Test account list operations
#[tokio::test]
async fn test_account_list() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).unwrap());
    let settings = Arc::new(ctx.settings.clone());
    let account_service = ClaudeAccountService::new(redis, settings).unwrap();

    // Create multiple accounts
    for i in 1..=3 {
        let options = CreateClaudeAccountOptions {
            name: format!("Test Account {}", i),
            description: Some(format!("Test account number {}", i)),
            account_type: AccountType::Shared,
            platform: Platform::Claude,
            is_active: true,
            schedulable: true,
            priority: i,
            proxy: None,
            refresh_token: Some(format!("refresh_token_{}", i)),
            claude_ai_oauth: None,
            subscription_info: None,
            email: None,
            password: None,
            auto_stop_on_warning: false,
            use_unified_user_agent: false,
            use_unified_client_id: false,
            unified_client_id: None,
            expires_at: None,
            ext_info: None,
        };

        account_service
            .create_account(options)
            .await
            .expect("Failed to create account");
    }

    // List all accounts (offset: 0, limit: 100)
    let accounts = account_service
        .list_accounts(0, 100)
        .await
        .expect("Failed to list accounts");

    assert!(accounts.len() >= 3, "Should have at least 3 accounts");
    println!("✅ Listed {} accounts", accounts.len());

    // Cleanup
    for account in accounts {
        let _ = account_service
            .delete_account(&account.id.to_string())
            .await;
    }
}

/// Test account update
#[tokio::test]
async fn test_account_update() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).unwrap());
    let settings = Arc::new(ctx.settings.clone());
    let account_service = ClaudeAccountService::new(redis, settings).unwrap();

    // Create account
    let options = CreateClaudeAccountOptions {
        name: "Original Name".to_string(),
        description: Some("Original description".to_string()),
        account_type: AccountType::Shared,
        platform: Platform::Claude,
        is_active: true,
        schedulable: true,
        priority: 5,
        proxy: None,
        refresh_token: Some("test_refresh_token".to_string()),
        claude_ai_oauth: None,
        subscription_info: None,
        email: None,
        password: None,
        auto_stop_on_warning: false,
        use_unified_user_agent: false,
        use_unified_client_id: false,
        unified_client_id: None,
        expires_at: None,
        ext_info: None,
    };

    let account = account_service
        .create_account(options)
        .await
        .expect("Failed to create account");

    // Update account
    let update_options = CreateClaudeAccountOptions {
        name: "Updated Name".to_string(),
        description: Some("Updated description".to_string()),
        account_type: AccountType::Dedicated,
        platform: Platform::Claude,
        is_active: true,
        schedulable: false,
        priority: 20,
        proxy: None,
        refresh_token: Some("new_refresh_token".to_string()),
        claude_ai_oauth: None,
        subscription_info: None,
        email: None,
        password: None,
        auto_stop_on_warning: false,
        use_unified_user_agent: false,
        use_unified_client_id: false,
        unified_client_id: None,
        expires_at: Some(format!("{}", Utc::now().timestamp_millis() + 3600000)),
        ext_info: None,
    };

    let updated = account_service
        .update_account(&account.id.to_string(), update_options)
        .await
        .expect("Failed to update account");

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.account_type, AccountType::Dedicated);
    assert_eq!(updated.priority, 20);
    assert!(!updated.schedulable);
    println!("✅ Account update successful");

    // Cleanup
    account_service
        .delete_account(&account.id.to_string())
        .await
        .expect("Failed to delete account");
}

/// Test account status changes
#[tokio::test]
async fn test_account_status_changes() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).unwrap());
    let settings = Arc::new(ctx.settings.clone());
    let account_service = ClaudeAccountService::new(redis, settings).unwrap();

    // Create account
    let options = CreateClaudeAccountOptions {
        name: "Status Test Account".to_string(),
        description: Some("Test account for status changes".to_string()),
        account_type: AccountType::Shared,
        platform: Platform::Claude,
        is_active: true,
        schedulable: true,
        priority: 10,
        proxy: None,
        refresh_token: Some("test_refresh_token".to_string()),
        claude_ai_oauth: None,
        subscription_info: None,
        email: None,
        password: None,
        auto_stop_on_warning: false,
        use_unified_user_agent: false,
        use_unified_client_id: false,
        unified_client_id: None,
        expires_at: None,
        ext_info: None,
    };

    let account = account_service
        .create_account(options)
        .await
        .expect("Failed to create account");

    assert!(account.is_active);
    println!("✅ Account created as active");

    // Deactivate account (using update_account with is_active: false)
    let deactivate_options = CreateClaudeAccountOptions {
        name: account.name.clone(),
        description: account.description.clone(),
        account_type: account.account_type.clone(),
        platform: account.platform,
        is_active: false, // Deactivate
        schedulable: account.schedulable,
        priority: account.priority,
        proxy: None, // Proxy is stored as String in account, but CreateOptions needs ProxyConfig
        refresh_token: Some("test_refresh_token".to_string()),
        claude_ai_oauth: None,
        subscription_info: None,
        email: None,
        password: None,
        auto_stop_on_warning: account.auto_stop_on_warning,
        use_unified_user_agent: account.use_unified_user_agent,
        use_unified_client_id: account.use_unified_client_id,
        unified_client_id: account.unified_client_id.clone(),
        expires_at: None,
        ext_info: None,
    };

    let deactivated = account_service
        .update_account(&account.id.to_string(), deactivate_options)
        .await
        .expect("Failed to deactivate account");

    assert!(!deactivated.is_active);
    println!("✅ Account deactivated successfully");

    // Reactivate account (using update_account with is_active: true)
    let reactivate_options = CreateClaudeAccountOptions {
        name: deactivated.name.clone(),
        description: deactivated.description.clone(),
        account_type: deactivated.account_type.clone(),
        platform: deactivated.platform,
        is_active: true, // Reactivate
        schedulable: deactivated.schedulable,
        priority: deactivated.priority,
        proxy: None, // Proxy is stored as String in account, but CreateOptions needs ProxyConfig
        refresh_token: Some("test_refresh_token".to_string()),
        claude_ai_oauth: None,
        subscription_info: None,
        email: None,
        password: None,
        auto_stop_on_warning: deactivated.auto_stop_on_warning,
        use_unified_user_agent: deactivated.use_unified_user_agent,
        use_unified_client_id: deactivated.use_unified_client_id,
        unified_client_id: deactivated.unified_client_id.clone(),
        expires_at: None,
        ext_info: None,
    };

    let reactivated = account_service
        .update_account(&account.id.to_string(), reactivate_options)
        .await
        .expect("Failed to reactivate account");

    assert!(reactivated.is_active);
    println!("✅ Account reactivated successfully");

    // Cleanup
    account_service
        .delete_account(&account.id.to_string())
        .await
        .expect("Failed to delete account");
}

/// Test account filtering by platform
#[tokio::test]
async fn test_account_list_by_platform() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).unwrap());
    let settings = Arc::new(ctx.settings.clone());
    let account_service = ClaudeAccountService::new(redis, settings).unwrap();

    // Create Claude accounts
    for i in 1..=2 {
        let options = CreateClaudeAccountOptions {
            name: format!("Claude Account {}", i),
            description: Some(format!("Claude test account {}", i)),
            account_type: AccountType::Shared,
            platform: Platform::Claude,
            is_active: true,
            schedulable: true,
            priority: i,
            proxy: None,
            refresh_token: Some(format!("refresh_token_{}", i)),
            claude_ai_oauth: None,
            subscription_info: None,
            email: None,
            password: None,
            auto_stop_on_warning: false,
            use_unified_user_agent: false,
            use_unified_client_id: false,
            unified_client_id: None,
            expires_at: None,
            ext_info: None,
        };

        account_service
            .create_account(options)
            .await
            .expect("Failed to create Claude account");
    }

    // Create Gemini accounts
    for i in 1..=2 {
        let options = CreateClaudeAccountOptions {
            name: format!("Gemini Account {}", i),
            description: Some(format!("Gemini test account {}", i)),
            account_type: AccountType::Shared,
            platform: Platform::Gemini,
            is_active: true,
            schedulable: true,
            priority: i,
            proxy: None,
            refresh_token: Some(format!("gemini_refresh_{}", i)),
            claude_ai_oauth: None,
            subscription_info: None,
            email: None,
            password: None,
            auto_stop_on_warning: false,
            use_unified_user_agent: false,
            use_unified_client_id: false,
            unified_client_id: None,
            expires_at: None,
            ext_info: None,
        };

        account_service
            .create_account(options)
            .await
            .expect("Failed to create Gemini account");
    }

    // List all accounts and filter by platform (since list_accounts_by_platform doesn't exist)
    let all_accounts = account_service
        .list_accounts(0, 100)
        .await
        .expect("Failed to list accounts");

    let claude_accounts: Vec<_> = all_accounts
        .iter()
        .filter(|a| a.platform == Platform::Claude)
        .collect();

    assert!(
        claude_accounts.len() >= 2,
        "Should have at least 2 Claude accounts"
    );
    for account in &claude_accounts {
        assert_eq!(account.platform, Platform::Claude);
    }
    println!("✅ Listed {} Claude accounts", claude_accounts.len());

    let gemini_accounts: Vec<_> = all_accounts
        .iter()
        .filter(|a| a.platform == Platform::Gemini)
        .collect();

    assert!(
        gemini_accounts.len() >= 2,
        "Should have at least 2 Gemini accounts"
    );
    for account in &gemini_accounts {
        assert_eq!(account.platform, Platform::Gemini);
    }
    println!("✅ Listed {} Gemini accounts", gemini_accounts.len());

    // Cleanup
    let all_accounts_for_cleanup = account_service.list_accounts(0, 100).await.unwrap();
    for account in all_accounts_for_cleanup {
        let _ = account_service
            .delete_account(&account.id.to_string())
            .await;
    }
}
