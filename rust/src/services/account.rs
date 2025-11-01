use std::sync::Arc;
use uuid::Uuid;

use crate::config::Settings;
use crate::models::{ClaudeAccount, CreateClaudeAccountOptions};
use crate::redis::RedisPool;
use crate::utils::{AppError, CryptoService, Result};

/// Claude account service for managing Claude accounts
///
/// This service provides CRUD operations for Claude accounts with:
/// - Encrypted storage of sensitive data (tokens, credentials)
/// - Redis-based persistence
/// - Account lifecycle management
/// - Caching and performance optimization
pub struct ClaudeAccountService {
    redis: Arc<RedisPool>,
    crypto: Arc<CryptoService>,
    #[allow(dead_code)]
    settings: Arc<Settings>,
}

impl ClaudeAccountService {
    /// Create a new Claude account service
    pub fn new(redis: Arc<RedisPool>, settings: Arc<Settings>) -> Result<Self> {
        let encryption_key = settings.security.encryption_key.clone();

        if encryption_key.is_empty() {
            return Err(AppError::ConfigError(
                "Encryption key not configured".to_string(),
            ));
        }

        let crypto = Arc::new(CryptoService::new(encryption_key));

        Ok(Self {
            redis,
            crypto,
            settings,
        })
    }

    /// Create a new Claude account
    ///
    /// # Arguments
    /// * `options` - Account creation options including name, email, password, OAuth data, etc.
    ///
    /// # Returns
    /// * `Result<ClaudeAccount>` - The created account with encrypted sensitive data
    ///
    /// # Example
    /// ```rust,no_run
    /// use claude_relay::services::ClaudeAccountService;
    /// use claude_relay::models::CreateClaudeAccountOptions;
    ///
    /// # async fn example(service: ClaudeAccountService) -> Result<(), Box<dyn std::error::Error>> {
    /// let options = CreateClaudeAccountOptions {
    ///     name: "My Account".to_string(),
    ///     email: Some("user@example.com".to_string()),
    ///     password: Some("password".to_string()),
    ///     ..Default::default()
    /// };
    ///
    /// let account = service.create_account(options).await?;
    /// println!("Created account: {}", account.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_account(
        &self,
        options: CreateClaudeAccountOptions,
    ) -> Result<ClaudeAccount> {
        use chrono::Utc;

        // 1. Generate unique ID
        let account_id = Uuid::new_v4().to_string();

        // 2. Validate required fields
        if options.name.is_empty() {
            return Err(AppError::ValidationError(
                "Account name is required".to_string(),
            ));
        }

        // 3. Determine account status based on whether we have OAuth data
        let status = if options.claude_ai_oauth.is_some() {
            crate::models::AccountStatus::Active
        } else {
            crate::models::AccountStatus::Inactive
        };

        // 4. Create account with encrypted sensitive data
        let mut account = ClaudeAccount {
            id: Uuid::parse_str(&account_id)
                .map_err(|e| AppError::InternalError(format!("Invalid UUID: {}", e)))?,
            name: options.name.clone(),
            description: options.description.clone(),
            email: None,
            password: None,
            claude_ai_oauth: None,
            access_token: None,
            refresh_token: None,
            expires_at: None,
            scopes: None,
            proxy: options
                .proxy
                .as_ref()
                .map(|p| serde_json::to_string(p).unwrap_or_default()),
            is_active: options.is_active,
            account_type: options.account_type,
            platform: options.platform,
            priority: options.priority,
            schedulable: options.schedulable,
            subscription_info: options
                .subscription_info
                .as_ref()
                .map(|info| serde_json::to_string(info).unwrap_or_default()),
            auto_stop_on_warning: options.auto_stop_on_warning,
            use_unified_user_agent: options.use_unified_user_agent,
            use_unified_client_id: options.use_unified_client_id,
            unified_client_id: options.unified_client_id.clone(),
            account_expires_at: options.expires_at.map(|ts| ts.to_string()),
            ext_info: options.ext_info.as_ref().map(|v| v.to_string()),
            status,
            error_message: None,
            last_refresh_at: None,
            concurrency_limit: 10, // 默认并发限制
            current_concurrency: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 5. Encrypt sensitive data
        if let Some(ref email) = options.email {
            account.email = Some(self.encrypt_field(email)?);
        }

        if let Some(ref password) = options.password {
            account.password = Some(self.encrypt_field(password)?);
        }

        if let Some(ref oauth) = options.claude_ai_oauth {
            // Encrypt OAuth data
            let oauth_json = serde_json::to_string(&oauth).map_err(|e| {
                AppError::InternalError(format!("Failed to serialize OAuth data: {}", e))
            })?;
            account.claude_ai_oauth = Some(self.encrypt_field(&oauth_json)?);

            // Set OAuth fields
            account.access_token = Some(self.encrypt_field(&oauth.access_token)?);
            account.refresh_token = Some(self.encrypt_field(&oauth.refresh_token)?);
            account.expires_at = Some(oauth.expires_at.to_string());

            // Convert Vec<String> to space-separated String
            account.scopes = Some(oauth.scopes.join(" "));
        } else if let Some(ref refresh_token) = options.refresh_token {
            // Legacy format: just refresh token
            account.refresh_token = Some(self.encrypt_field(refresh_token)?);
        }

        // 6. Serialize account to JSON
        let account_json = serde_json::to_string(&account)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize account: {}", e)))?;

        // 7. Store to Redis
        let key = self.account_key(&account_id);
        self.redis.set(&key, &account_json).await?;

        // 8. Add to account list (using Redis SET)
        let list_key = self.account_list_key();
        let mut conn = self.redis.get_connection().await?;
        redis::cmd("SADD")
            .arg(&list_key)
            .arg(&account_id)
            .query_async::<_, i32>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to add account to list: {}", e)))?;

        tracing::info!(
            account_id = %account.id,
            name = %account.name,
            platform = ?account.platform,
            "🏢 Created Claude account"
        );

        Ok(account)
    }

    /// Get an account by ID
    ///
    /// # Arguments
    /// * `account_id` - The account ID to retrieve
    ///
    /// # Returns
    /// * `Result<Option<ClaudeAccount>>` - The account if found, with decrypted sensitive data
    ///
    /// # Example
    /// ```rust,no_run
    /// # async fn example(service: ClaudeAccountService) -> Result<(), Box<dyn std::error::Error>> {
    /// let account = service.get_account("account-id-123").await?;
    /// if let Some(acc) = account {
    ///     println!("Found account: {}", acc.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_account(&self, account_id: &str) -> Result<Option<ClaudeAccount>> {
        // 1. Read from Redis
        let key = self.account_key(account_id);
        let account_json: Option<String> = self.redis.get(&key).await?;

        // 2. Check if account exists
        let account_json = match account_json {
            Some(json) if !json.is_empty() => json,
            _ => return Ok(None),
        };

        // 3. Deserialize JSON to ClaudeAccount
        let account: ClaudeAccount = serde_json::from_str(&account_json).map_err(|e| {
            AppError::InternalError(format!("Failed to deserialize account: {}", e))
        })?;

        // 4. Decrypt sensitive data (if present)
        // Note: We don't decrypt here to avoid performance overhead
        // Decryption will be done on-demand when accessing sensitive fields
        // This matches the Node.js behavior where encrypted data is returned as-is

        tracing::debug!(
            account_id = %account.id,
            name = %account.name,
            "Retrieved account from Redis"
        );

        Ok(Some(account))
    }

    /// Get an account with decrypted sensitive data
    ///
    /// This method retrieves the account and decrypts all sensitive fields.
    /// Use this when you need access to the actual token values.
    ///
    /// # Arguments
    /// * `account_id` - The account ID to retrieve
    ///
    /// # Returns
    /// * `Result<Option<ClaudeAccount>>` - The account with decrypted data
    pub async fn get_account_decrypted(&self, account_id: &str) -> Result<Option<ClaudeAccount>> {
        let account = self.get_account(account_id).await?;

        if let Some(mut acc) = account {
            // Decrypt sensitive fields
            if let Some(ref encrypted_email) = acc.email {
                acc.email = Some(self.decrypt_field(encrypted_email)?);
            }

            if let Some(ref encrypted_password) = acc.password {
                acc.password = Some(self.decrypt_field(encrypted_password)?);
            }

            if let Some(ref encrypted_access_token) = acc.access_token {
                acc.access_token = Some(self.decrypt_field(encrypted_access_token)?);
            }

            if let Some(ref encrypted_refresh_token) = acc.refresh_token {
                acc.refresh_token = Some(self.decrypt_field(encrypted_refresh_token)?);
            }

            if let Some(ref encrypted_oauth) = acc.claude_ai_oauth {
                acc.claude_ai_oauth = Some(self.decrypt_field(encrypted_oauth)?);
            }

            tracing::debug!(
                account_id = %acc.id,
                name = %acc.name,
                "Decrypted account sensitive data"
            );

            Ok(Some(acc))
        } else {
            Ok(None)
        }
    }

    /// Update an existing account
    ///
    /// # Arguments
    /// * `account_id` - The account ID to update
    /// * `updates` - Partial account data to update
    ///
    /// # Returns
    /// * `Result<ClaudeAccount>` - The updated account
    ///
    /// # Example
    /// ```rust,no_run
    /// # async fn example(service: ClaudeAccountService) -> Result<(), Box<dyn std::error::Error>> {
    /// use claude_relay::models::CreateClaudeAccountOptions;
    ///
    /// let updates = CreateClaudeAccountOptions {
    ///     name: "Updated Name".to_string(),
    ///     is_active: false,
    ///     ..Default::default()
    /// };
    ///
    /// let account = service.update_account("account-id", updates).await?;
    /// println!("Updated account: {}", account.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_account(
        &self,
        account_id: &str,
        updates: CreateClaudeAccountOptions,
    ) -> Result<ClaudeAccount> {
        use chrono::Utc;

        // 1. Read existing account
        let mut account = self
            .get_account(account_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Account {} not found", account_id)))?;

        // 2. Apply updates to non-sensitive fields
        if !updates.name.is_empty() {
            account.name = updates.name;
        }

        if let Some(desc) = updates.description {
            account.description = Some(desc);
        }

        // Update boolean and numeric fields
        account.is_active = updates.is_active;
        account.schedulable = updates.schedulable;
        account.auto_stop_on_warning = updates.auto_stop_on_warning;
        account.use_unified_user_agent = updates.use_unified_user_agent;
        account.use_unified_client_id = updates.use_unified_client_id;
        account.priority = updates.priority;
        account.account_type = updates.account_type;
        account.platform = updates.platform;

        // Update optional string fields
        if let Some(client_id) = updates.unified_client_id {
            account.unified_client_id = Some(client_id);
        }

        if let Some(expires_at) = updates.expires_at {
            account.account_expires_at = Some(expires_at.to_string());
        }

        // Update proxy
        if let Some(proxy) = updates.proxy {
            account.proxy = Some(serde_json::to_string(&proxy).unwrap_or_default());
        }

        // Update subscription info
        if let Some(sub_info) = updates.subscription_info {
            account.subscription_info = Some(serde_json::to_string(&sub_info).unwrap_or_default());
        }

        // Update ext_info
        if let Some(ext_info) = updates.ext_info {
            account.ext_info = Some(ext_info.to_string());
        }

        // 3. Update and encrypt sensitive fields
        if let Some(email) = updates.email {
            account.email = Some(self.encrypt_field(&email)?);
        }

        if let Some(password) = updates.password {
            account.password = Some(self.encrypt_field(&password)?);
        }

        if let Some(oauth) = updates.claude_ai_oauth {
            // Full OAuth update
            let oauth_json = serde_json::to_string(&oauth).map_err(|e| {
                AppError::InternalError(format!("Failed to serialize OAuth data: {}", e))
            })?;
            account.claude_ai_oauth = Some(self.encrypt_field(&oauth_json)?);
            account.access_token = Some(self.encrypt_field(&oauth.access_token)?);
            account.refresh_token = Some(self.encrypt_field(&oauth.refresh_token)?);
            account.expires_at = Some(oauth.expires_at.to_string());
            account.scopes = Some(oauth.scopes.join(" "));
            account.status = crate::models::AccountStatus::Active;
            account.error_message = None;
            account.last_refresh_at = Some(Utc::now());
        } else if let Some(refresh_token) = updates.refresh_token {
            // Just update refresh token (legacy format)
            account.refresh_token = Some(self.encrypt_field(&refresh_token)?);
        }

        // 4. Update timestamp
        account.updated_at = Utc::now();

        // 5. Serialize and save to Redis
        let account_json = serde_json::to_string(&account)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize account: {}", e)))?;

        let key = self.account_key(account_id);
        self.redis.set(&key, &account_json).await?;

        tracing::info!(
            account_id = %account.id,
            name = %account.name,
            "🔄 Updated Claude account"
        );

        Ok(account)
    }

    /// Delete an account by ID
    ///
    /// # Arguments
    /// * `account_id` - The account ID to delete
    ///
    /// # Returns
    /// * `Result<bool>` - True if account was deleted, false if not found
    pub async fn delete_account(&self, account_id: &str) -> Result<bool> {
        tracing::info!("🗑️ Deleting account: {}", account_id);

        // 1. Check if account exists
        let key = self.account_key(account_id);
        let exists: bool = self.redis.exists(&key).await?;

        if !exists {
            tracing::warn!("⚠️ Account not found: {}", account_id);
            return Ok(false);
        }

        // 2. Get account info for logging
        let account = self.get_account(account_id).await?;
        if let Some(ref acc) = account {
            tracing::info!(
                "📝 Deleting account '{}' (platform: {:?}, type: {:?})",
                acc.name,
                acc.platform,
                acc.account_type
            );
        }

        // 3. Delete account data from Redis
        self.redis.del(&key).await?;
        tracing::debug!("✅ Deleted account data: {}", key);

        // 4. Remove from account list
        let list_key = self.account_list_key();
        let mut conn = self.redis.get_connection().await?;
        let _: i32 = redis::cmd("SREM")
            .arg(&list_key)
            .arg(account_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                AppError::RedisError(format!("Failed to remove from account list: {}", e))
            })?;
        tracing::debug!("✅ Removed from account list: {}", list_key);

        // 5. Clean up related data (sessions, concurrency, etc.)
        // Note: In production, these keys should be cleaned by separate cleanup tasks
        // or when they're accessed and found to reference a deleted account
        let session_key = format!("session_window:{}", account_id);
        let concurrency_key = format!("concurrency:{}", account_id);
        let overload_key = format!("overload:{}", account_id);

        let _: () = redis::pipe()
            .cmd("DEL")
            .arg(&session_key)
            .ignore()
            .cmd("DEL")
            .arg(&concurrency_key)
            .ignore()
            .cmd("DEL")
            .arg(&overload_key)
            .ignore()
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to clean up related data: {}", e)))?;

        tracing::info!(
            "🎯 Account deleted successfully: {} (cleaned session, concurrency, overload data)",
            account_id
        );

        Ok(true)
    }

    /// List all accounts with optional filtering
    ///
    /// # Arguments
    /// * `offset` - Pagination offset (default: 0)
    /// * `limit` - Pagination limit (default: 100, max: 1000)
    ///
    /// # Returns
    /// * `Result<Vec<ClaudeAccount>>` - List of accounts matching the criteria
    ///
    /// # Example
    /// ```
    /// // Get first 10 accounts
    /// let accounts = service.list_accounts(0, 10).await?;
    ///
    /// // Get next 10 accounts
    /// let accounts = service.list_accounts(10, 10).await?;
    /// ```
    pub async fn list_accounts(&self, offset: usize, limit: usize) -> Result<Vec<ClaudeAccount>> {
        tracing::debug!("📋 Listing accounts (offset: {}, limit: {})", offset, limit);

        // Enforce maximum limit
        let effective_limit = limit.min(1000);

        // 1. Get all account IDs from Redis SET
        let list_key = self.account_list_key();
        let mut conn = self.redis.get_connection().await?;
        let account_ids: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&list_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get account list: {}", e)))?;

        tracing::debug!("📝 Found {} total accounts in Redis", account_ids.len());

        // 2. Apply pagination to account IDs
        let paginated_ids: Vec<String> = account_ids
            .into_iter()
            .skip(offset)
            .take(effective_limit)
            .collect();

        if paginated_ids.is_empty() {
            tracing::info!(
                "✅ No accounts found for offset={}, limit={}",
                offset,
                limit
            );
            return Ok(Vec::new());
        }

        tracing::debug!(
            "🔍 Fetching {} accounts (after pagination)",
            paginated_ids.len()
        );

        // 3. Fetch accounts in batch using pipeline for performance
        let keys: Vec<String> = paginated_ids
            .iter()
            .map(|id| self.account_key(id))
            .collect();

        let mut pipe = redis::pipe();
        for key in &keys {
            pipe.cmd("GET").arg(key);
        }

        let account_jsons: Vec<Option<String>> = pipe
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to fetch accounts: {}", e)))?;

        // 4. Parse accounts and filter out invalid/missing ones
        let mut accounts = Vec::new();
        for (idx, account_json) in account_jsons.into_iter().enumerate() {
            match account_json {
                Some(json) if !json.is_empty() => {
                    match serde_json::from_str::<ClaudeAccount>(&json) {
                        Ok(account) => {
                            accounts.push(account);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "⚠️ Failed to parse account {}: {}",
                                paginated_ids[idx],
                                e
                            );
                        }
                    }
                }
                _ => {
                    tracing::warn!("⚠️ Account {} not found in Redis", paginated_ids[idx]);
                }
            }
        }

        tracing::info!(
            "✅ Successfully listed {} accounts (offset: {}, limit: {})",
            accounts.len(),
            offset,
            limit
        );

        Ok(accounts)
    }

    /// Generate Redis key for account data
    fn account_key(&self, account_id: &str) -> String {
        format!("claude_account:{}", account_id)
    }

    /// Generate Redis key for account list
    fn account_list_key(&self) -> String {
        "claude_accounts".to_string()
    }

    /// Encrypt sensitive field
    fn encrypt_field(&self, data: &str) -> Result<String> {
        if data.is_empty() {
            return Ok(String::new());
        }
        self.crypto.encrypt(data)
    }

    /// Decrypt sensitive field
    fn decrypt_field(&self, encrypted: &str) -> Result<String> {
        if encrypted.is_empty() {
            return Ok(String::new());
        }
        self.crypto.decrypt(encrypted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::account::{AccountType, ClaudeOAuthData, Platform};

    // Helper function to create test account options
    fn create_test_account_options(name: &str) -> CreateClaudeAccountOptions {
        CreateClaudeAccountOptions {
            name: name.to_string(),
            description: Some("Test account".to_string()),
            email: Some("test@example.com".to_string()),
            password: None,
            refresh_token: None,
            claude_ai_oauth: Some(ClaudeOAuthData {
                access_token: "test_access_token_123".to_string(),
                refresh_token: "test_refresh_token_456".to_string(),
                expires_at: 1735660800000, // 2025-01-01 in milliseconds
                scopes: vec!["read".to_string(), "write".to_string()],
            }),
            proxy: None,
            is_active: true,
            account_type: AccountType::Shared,
            platform: Platform::Claude,
            priority: 1,
            schedulable: true,
            subscription_info: None,
            auto_stop_on_warning: false,
            use_unified_user_agent: false,
            use_unified_client_id: false,
            unified_client_id: None,
            expires_at: None,
            ext_info: None,
        }
    }

    // Helper function to create mock service for testing
    // Note: This requires Redis connection and proper encryption key configuration
    fn create_mock_service() -> (ClaudeAccountService, Settings) {
        // Set test encryption key if not already set
        std::env::set_var("ENCRYPTION_KEY", "test-encryption-key-32chars!!");

        let config = Settings::new().expect("Failed to create test config");
        let redis = Arc::new(RedisPool::new(&config).expect("Failed to create Redis pool"));
        let service = ClaudeAccountService::new(redis, Arc::new(config.clone()))
            .expect("Failed to create service");
        (service, config)
    }

    #[test]
    fn test_account_key_generation() {
        let account_id = "test-account-123";
        let key = format!("claude_account:{}", account_id);

        assert_eq!(key, "claude_account:test-account-123");
        assert!(key.starts_with("claude_account:"));
    }

    #[test]
    fn test_account_list_key_generation() {
        let list_key = "claude_accounts".to_string();

        assert_eq!(list_key, "claude_accounts");
    }

    #[test]
    fn test_create_account_options_structure() {
        let options = create_test_account_options("Test Account");

        assert_eq!(options.name, "Test Account");
        assert_eq!(options.platform, Platform::Claude);
        assert_eq!(options.account_type, AccountType::Shared);
        assert!(options.is_active);
        assert_eq!(options.priority, 1);
        assert!(options.schedulable);
        assert!(options.claude_ai_oauth.is_some());

        let oauth = options.claude_ai_oauth.unwrap();
        assert_eq!(oauth.access_token, "test_access_token_123");
        assert_eq!(oauth.refresh_token, "test_refresh_token_456");
        assert_eq!(oauth.scopes.len(), 2);
        assert!(oauth.scopes.contains(&"read".to_string()));
        assert!(oauth.scopes.contains(&"write".to_string()));
    }

    #[tokio::test]
    #[ignore]
    async fn test_encrypt_decrypt_field() {
        let (service, _) = create_mock_service();

        let original_data = "sensitive_password_123";
        let encrypted = service
            .encrypt_field(original_data)
            .expect("Encryption failed");

        assert_ne!(encrypted, original_data);
        assert!(
            encrypted.contains(':'),
            "Encrypted data should contain IV separator"
        );

        let decrypted = service
            .decrypt_field(&encrypted)
            .expect("Decryption failed");
        assert_eq!(decrypted, original_data);
    }

    #[tokio::test]
    #[ignore]
    async fn test_encrypt_empty_string() {
        let (service, _) = create_mock_service();

        let encrypted = service
            .encrypt_field("")
            .expect("Should handle empty string");
        assert_eq!(encrypted, "");

        let decrypted = service
            .decrypt_field("")
            .expect("Should handle empty string");
        assert_eq!(decrypted, "");
    }

    #[tokio::test]
    #[ignore]
    async fn test_encrypt_field_different_results() {
        let (service, _) = create_mock_service();

        let data = "test_data";
        let encrypted1 = service.encrypt_field(data).expect("Encryption failed");
        let encrypted2 = service.encrypt_field(data).expect("Encryption failed");

        // Different IVs should result in different encrypted strings
        assert_ne!(
            encrypted1, encrypted2,
            "Same data should encrypt differently with different IVs"
        );

        // But both should decrypt to the same original data
        let decrypted1 = service
            .decrypt_field(&encrypted1)
            .expect("Decryption failed");
        let decrypted2 = service
            .decrypt_field(&encrypted2)
            .expect("Decryption failed");
        assert_eq!(decrypted1, data);
        assert_eq!(decrypted2, data);
    }

    #[test]
    fn test_list_accounts_max_limit() {
        // Test that limit is enforced (should be capped at 1000)
        // This is a logic test only, actual list_accounts requires Redis
        let max_limit = 1000usize;
        let excessive_limit = 5000usize;
        let effective_limit = excessive_limit.min(max_limit);

        assert_eq!(effective_limit, 1000, "Limit should be capped at 1000");
    }

    // ========================================
    // Integration tests (require Redis)
    // Run with: cargo test --test account_integration_test
    // ========================================

    #[tokio::test]
    #[ignore]
    async fn test_create_account() {
        let (service, _) = create_mock_service();
        let options = create_test_account_options("Integration Test Account");

        let result = service.create_account(options).await;
        assert!(result.is_ok(), "Account creation should succeed");

        let account = result.unwrap();
        assert_eq!(account.name, "Integration Test Account");
        assert_eq!(account.platform, Platform::Claude);
        assert!(account.access_token.is_some());

        // Cleanup
        let _ = service.delete_account(&account.id.to_string()).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_account() {
        let (service, _) = create_mock_service();
        let options = create_test_account_options("Get Test Account");

        // Create account first
        let created = service.create_account(options).await.unwrap();
        let account_id = created.id.to_string();

        // Get account
        let result = service.get_account(&account_id).await;
        assert!(result.is_ok());

        let account = result.unwrap();
        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.name, "Get Test Account");

        // Cleanup
        let _ = service.delete_account(&account_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_update_account() {
        let (service, _) = create_mock_service();
        let options = create_test_account_options("Update Test Account");

        // Create account
        let created = service.create_account(options).await.unwrap();
        let account_id = created.id.to_string();

        // Update account
        let mut update_options = create_test_account_options("Updated Account Name");
        update_options.priority = 10;
        update_options.is_active = false;

        let result = service.update_account(&account_id, update_options).await;
        assert!(result.is_ok());

        let updated = result.unwrap();
        assert_eq!(updated.name, "Updated Account Name");
        assert_eq!(updated.priority, 10);
        assert!(!updated.is_active);

        // Cleanup
        let _ = service.delete_account(&account_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_delete_account() {
        let (service, _) = create_mock_service();
        let options = create_test_account_options("Delete Test Account");

        // Create account
        let created = service.create_account(options).await.unwrap();
        let account_id = created.id.to_string();

        // Delete account
        let result = service.delete_account(&account_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap(), "Delete should return true");

        // Verify account is gone
        let get_result = service.get_account(&account_id).await;
        assert!(get_result.is_ok());
        assert!(get_result.unwrap().is_none(), "Account should be deleted");
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_accounts() {
        let (service, _) = create_mock_service();

        // Create multiple accounts
        for i in 1..=5 {
            let options = create_test_account_options(&format!("List Test Account {}", i));
            let _ = service.create_account(options).await;
        }

        // List accounts
        let result = service.list_accounts(0, 10).await;
        assert!(result.is_ok());

        let accounts = result.unwrap();
        assert!(accounts.len() >= 5, "Should have at least 5 accounts");

        // Cleanup (list all and delete)
        for account in accounts {
            let _ = service.delete_account(&account.id.to_string()).await;
        }
    }
}
