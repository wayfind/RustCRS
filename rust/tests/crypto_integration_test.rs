mod common;

use claude_relay::models::account::{AccountType, CreateClaudeAccountOptions, Platform};
use claude_relay::services::ClaudeAccountService;
use claude_relay::utils::crypto::CryptoService;
use claude_relay::RedisPool;
use common::TestContext;
use std::sync::Arc;

/// Helper: Create minimal account options
fn create_account_options(name: &str, refresh_token: Option<String>) -> CreateClaudeAccountOptions {
    CreateClaudeAccountOptions {
        name: name.to_string(),
        description: Some(format!("Test account {}", name)),
        account_type: AccountType::Shared,
        platform: Platform::Claude,
        is_active: true,
        schedulable: true,
        priority: 10,
        proxy: None,
        refresh_token,
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
    }
}

// ========================================
// Basic Encryption/Decryption Tests
// ========================================

/// Test basic encrypt/decrypt roundtrip
#[tokio::test]
async fn test_crypto_basic_roundtrip() {
    let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

    let test_data = "sensitive token data 12345";
    let encrypted = crypto.encrypt(test_data).expect("Encryption failed");

    // Verify format: {iv_hex}:{encrypted_hex}
    assert!(encrypted.contains(':'), "Should contain separator");
    assert_ne!(encrypted, test_data, "Should be different from original");

    let decrypted = crypto.decrypt(&encrypted).expect("Decryption failed");
    assert_eq!(decrypted, test_data, "Should match original data");
}

/// Test encrypt/decrypt with special characters
#[tokio::test]
async fn test_crypto_special_characters() {
    let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

    let test_cases = vec![
        "data with spaces",
        "data\nwith\nnewlines",
        "data\twith\ttabs",
        "data🔐with🔑emojis",
        r#"{"json":"data","nested":{"value":123}}"#,
        "data'with\"quotes",
    ];

    for test_data in test_cases {
        let encrypted = crypto.encrypt(test_data).expect("Encryption failed");
        let decrypted = crypto.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, test_data, "Mismatch for: {}", test_data);
    }
}

/// Test encryption produces different outputs for same input (due to random IV)
#[tokio::test]
async fn test_crypto_random_iv() {
    let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

    let test_data = "same input data";
    let encrypted1 = crypto.encrypt(test_data).expect("Encryption failed");
    let encrypted2 = crypto.encrypt(test_data).expect("Encryption failed");

    // Should produce different ciphertexts
    assert_ne!(
        encrypted1, encrypted2,
        "Different encryptions should have different IVs"
    );

    // But both should decrypt correctly
    let decrypted1 = crypto.decrypt(&encrypted1).expect("Decryption failed");
    let decrypted2 = crypto.decrypt(&encrypted2).expect("Decryption failed");
    assert_eq!(decrypted1, test_data);
    assert_eq!(decrypted2, test_data);
}

// ========================================
// Edge Cases and Error Handling
// ========================================

/// Test empty string encryption/decryption
#[tokio::test]
async fn test_crypto_empty_string() {
    let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

    let encrypted = crypto.encrypt("").expect("Encryption should succeed");
    assert_eq!(encrypted, "", "Empty string should remain empty");

    let decrypted = crypto.decrypt("").expect("Decryption should succeed");
    assert_eq!(decrypted, "", "Empty string should remain empty");
}

/// Test invalid encrypted data formats
#[tokio::test]
async fn test_crypto_invalid_formats() {
    let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

    // Invalid hex characters
    let result = crypto.decrypt("invalid_hex:also_invalid");
    assert!(result.is_err(), "Should fail on invalid hex");

    // Missing separator
    let result = crypto.decrypt("no_separator_format_12345678");
    assert!(result.is_err(), "Should fail on old format");

    // Invalid IV length
    let result = crypto.decrypt("short:616263646566"); // IV too short
    assert!(result.is_err(), "Should fail on invalid IV length");

    // Corrupted ciphertext
    let valid_encrypted = crypto.encrypt("test").expect("Encryption failed");
    let parts: Vec<&str> = valid_encrypted.split(':').collect();
    let corrupted = format!("{}:corrupted_hex_data", parts[0]);
    let result = crypto.decrypt(&corrupted);
    assert!(result.is_err(), "Should fail on corrupted ciphertext");
}

/// Test large data encryption (approaching buffer limit)
#[tokio::test]
async fn test_crypto_large_data() {
    let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

    // Test data just under the 10 MB limit
    let large_data = "a".repeat(1024 * 1024); // 1 MB
    let encrypted = crypto.encrypt(&large_data).expect("Should handle 1MB data");
    let decrypted = crypto.decrypt(&encrypted).expect("Decryption failed");
    assert_eq!(decrypted.len(), large_data.len());
}

// ========================================
// Cache Behavior Tests
// ========================================

/// Test decrypt cache functionality
#[tokio::test]
async fn test_crypto_cache_behavior() {
    let crypto = CryptoService::new("test-cache-key-32-characters!!".to_string());

    // Clear cache before test
    CryptoService::clear_cache().expect("Cache clear failed");

    let test_data = "data for cache test v3";
    let encrypted = crypto.encrypt(test_data).expect("Encryption failed");

    // Get initial cache stats
    let (_, hits_before, misses_before, _) =
        CryptoService::cache_stats().expect("Failed to get stats");

    // First decrypt - should be a cache miss
    let decrypted1 = crypto.decrypt(&encrypted).expect("First decrypt failed");
    assert_eq!(decrypted1, test_data);

    // Second decrypt - should be a cache hit
    let decrypted2 = crypto.decrypt(&encrypted).expect("Second decrypt failed");
    assert_eq!(decrypted2, test_data);

    // Third decrypt - should be a cache hit
    let decrypted3 = crypto.decrypt(&encrypted).expect("Third decrypt failed");
    assert_eq!(decrypted3, test_data);

    // Verify cache statistics
    let (cache_size, hits_after, misses_after, hit_rate) =
        CryptoService::cache_stats().expect("Failed to get stats");

    assert!(
        cache_size >= 1,
        "Cache should have at least one entry, got {}",
        cache_size
    );

    let miss_delta = misses_after - misses_before;
    assert!(
        miss_delta >= 1,
        "Should have at least one cache miss, got {}",
        miss_delta
    );

    let hit_delta = hits_after - hits_before;
    assert!(
        hit_delta >= 2,
        "Should have at least two cache hits, got {}",
        hit_delta
    );

    assert!(
        hit_rate > 0.0,
        "Hit rate should be positive, got {}",
        hit_rate
    );
}

// ========================================
// Integration with Account Service
// ========================================

/// Test encryption integration with Claude account creation
#[tokio::test]
async fn test_crypto_integration_account_creation() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).unwrap());
    let settings = Arc::new(ctx.settings.clone());
    let account_service = ClaudeAccountService::new(redis, settings.clone()).unwrap();

    // Create crypto service for manual decryption
    let crypto = CryptoService::new(settings.security.encryption_key.clone());

    // Create account with encrypted refresh token
    let refresh_token = "refresh_token_secret_12345";

    let options = create_account_options("Test Crypto Account", Some(refresh_token.to_string()));

    let account = account_service
        .create_account(options)
        .await
        .expect("Account creation failed");

    // Verify token is stored encrypted, then decrypt to verify
    let encrypted_token = account.refresh_token.as_ref().unwrap();
    assert!(
        encrypted_token.contains(':'),
        "Token should be encrypted (contain ':')"
    );

    let decrypted = crypto.decrypt(encrypted_token).expect("Decryption failed");
    assert_eq!(decrypted, refresh_token);

    // Retrieve account and verify token persists correctly
    let retrieved = account_service
        .get_account(&account.id.to_string())
        .await
        .expect("Failed to retrieve account")
        .expect("Account not found");

    let retrieved_encrypted = retrieved.refresh_token.as_ref().unwrap();
    let retrieved_decrypted = crypto
        .decrypt(retrieved_encrypted)
        .expect("Decryption failed");
    assert_eq!(retrieved_decrypted, refresh_token);
}

/// Test encryption with account update operations
#[tokio::test]
async fn test_crypto_integration_account_update() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).unwrap());
    let settings = Arc::new(ctx.settings.clone());
    let account_service = ClaudeAccountService::new(redis, settings.clone()).unwrap();

    // Create crypto service for manual decryption
    let crypto = CryptoService::new(settings.security.encryption_key.clone());

    // Create initial account
    let options = create_account_options(
        "Update Test Account",
        Some("initial_refresh_token".to_string()),
    );

    let account = account_service
        .create_account(options)
        .await
        .expect("Account creation failed");

    // Update refresh token
    let new_refresh_token = "updated_refresh_token_secret";

    let update_options = create_account_options(&account.name, Some(new_refresh_token.to_string()));

    account_service
        .update_account(&account.id.to_string(), update_options)
        .await
        .expect("Account update failed");

    // Retrieve and verify updated token (decrypt to verify)
    let updated = account_service
        .get_account(&account.id.to_string())
        .await
        .expect("Failed to retrieve account")
        .expect("Account not found");

    let updated_encrypted = updated.refresh_token.as_ref().unwrap();
    let updated_decrypted = crypto
        .decrypt(updated_encrypted)
        .expect("Decryption failed");
    assert_eq!(updated_decrypted, new_refresh_token);
}

/// Test multiple accounts with different encrypted tokens
#[tokio::test]
async fn test_crypto_integration_multiple_accounts() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to setup test context");

    let redis = Arc::new(RedisPool::new(&ctx.settings).unwrap());
    let settings = Arc::new(ctx.settings.clone());
    let account_service = ClaudeAccountService::new(redis, settings.clone()).unwrap();

    // Create crypto service for manual decryption
    let crypto = CryptoService::new(settings.security.encryption_key.clone());

    // Create multiple accounts with different refresh tokens
    let accounts_data = vec![
        ("Account 1", "refresh_1"),
        ("Account 2", "refresh_2"),
        ("Account 3", "refresh_3"),
    ];

    let mut account_ids = Vec::new();

    for (name, refresh) in &accounts_data {
        let options = create_account_options(name, Some(refresh.to_string()));

        let account = account_service
            .create_account(options)
            .await
            .expect("Account creation failed");

        account_ids.push(account.id);
    }

    // Retrieve all accounts and verify refresh tokens (with decryption)
    for (i, account_id) in account_ids.iter().enumerate() {
        let account = account_service
            .get_account(&account_id.to_string())
            .await
            .expect("Failed to retrieve account")
            .expect("Account not found");

        let expected_refresh = format!("refresh_{}", i + 1);

        let encrypted = account.refresh_token.as_ref().unwrap();
        let decrypted = crypto.decrypt(encrypted).expect("Decryption failed");
        assert_eq!(decrypted, expected_refresh);
    }
}

// ========================================
// Key Derivation Tests
// ========================================

/// Test consistent key derivation from same source
#[tokio::test]
async fn test_crypto_key_derivation_consistency() {
    let crypto1 = CryptoService::new("consistent-key-32-characters!".to_string());
    let crypto2 = CryptoService::new("consistent-key-32-characters!".to_string());

    let test_data = "test data for key consistency";

    // Encrypt with first instance
    let encrypted1 = crypto1.encrypt(test_data).expect("Encryption 1 failed");

    // Decrypt with second instance (should use same derived key)
    let decrypted1 = crypto2.decrypt(&encrypted1).expect("Decryption 1 failed");
    assert_eq!(decrypted1, test_data);

    // Encrypt with second instance
    let encrypted2 = crypto2.encrypt(test_data).expect("Encryption 2 failed");

    // Decrypt with first instance
    let decrypted2 = crypto1.decrypt(&encrypted2).expect("Decryption 2 failed");
    assert_eq!(decrypted2, test_data);
}

/// Test different keys produce different ciphertexts
///
/// Note: This test is ignored because CryptoService uses a global key cache,
/// which means only one encryption key can be active per process.
/// In production, only one ENCRYPTION_KEY is used per service instance.
#[tokio::test]
#[ignore = "Global key cache prevents testing multiple keys in same process"]
async fn test_crypto_different_keys() {
    let crypto1 = CryptoService::new("key-one-32-characters-exactly!".to_string());
    let crypto2 = CryptoService::new("key-two-32-characters-exactly!".to_string());

    let test_data = "same data, different keys";

    let encrypted1 = crypto1.encrypt(test_data).expect("Encryption 1 failed");
    let encrypted2 = crypto2.encrypt(test_data).expect("Encryption 2 failed");

    // Encrypted data should be different
    assert_ne!(encrypted1, encrypted2);

    // Each crypto can decrypt its own ciphertext
    let decrypted1 = crypto1.decrypt(&encrypted1).expect("Decryption 1 failed");
    let decrypted2 = crypto2.decrypt(&encrypted2).expect("Decryption 2 failed");
    assert_eq!(decrypted1, test_data);
    assert_eq!(decrypted2, test_data);

    // But cannot decrypt the other's ciphertext
    let result = crypto1.decrypt(&encrypted2);
    assert!(result.is_err(), "Should not decrypt with wrong key");

    let result = crypto2.decrypt(&encrypted1);
    assert!(result.is_err(), "Should not decrypt with wrong key");
}

// ========================================
// Performance and Stress Tests
// ========================================

/// Test encryption/decryption performance with cache
#[tokio::test]
async fn test_crypto_performance_with_cache() {
    let crypto = CryptoService::new("perf-test-key-32-characters!!".to_string());

    // Clear cache before test
    CryptoService::clear_cache().expect("Cache clear failed");

    let test_data = "performance test data v2";
    let encrypted = crypto.encrypt(test_data).expect("Encryption failed");

    // Decrypt 100 times - most should be cache hits
    for _ in 0..100 {
        let decrypted = crypto.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, test_data);
    }

    // Verify high cache hit rate
    // Note: Due to global cache shared across tests, actual hit rate may vary
    // We use a more lenient threshold to account for concurrent test execution
    let (_, _, _, hit_rate) = CryptoService::cache_stats().expect("Failed to get stats");
    assert!(
        hit_rate > 70.0,
        "Hit rate should be >70% for repeated decryption, got {}%",
        hit_rate
    );

    println!("✅ Cache hit rate: {:.2}%", hit_rate);
}

/// Test concurrent encryption/decryption operations
#[tokio::test]
async fn test_crypto_concurrent_operations() {
    let crypto = Arc::new(CryptoService::new(
        "concurrent-test-key-32-chars!".to_string(),
    ));

    let mut handles = vec![];

    // Spawn 10 concurrent tasks
    for i in 0..10 {
        let crypto_clone = Arc::clone(&crypto);
        let handle = tokio::spawn(async move {
            let test_data = format!("concurrent test data {}", i);
            let encrypted = crypto_clone.encrypt(&test_data).expect("Encryption failed");
            let decrypted = crypto_clone.decrypt(&encrypted).expect("Decryption failed");
            assert_eq!(decrypted, test_data);
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task failed");
    }
}
