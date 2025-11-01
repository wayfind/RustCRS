use claude_relay::models::api_key::{
    ActivationUnit, ApiKeyCreateOptions, ApiKeyPermissions, ExpirationMode,
};
use claude_relay::services::ApiKeyService;
use claude_relay::{RedisPool, Settings};
use once_cell::sync::Lazy;
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::redis::Redis;

/// Global Docker client (reused across all tests)
static DOCKER: Lazy<Cli> = Lazy::new(Cli::default);

/// Test context with automatic Redis container management
///
/// This struct provides a complete testing environment with:
/// - Automatic Docker container lifecycle (start on new(), cleanup on drop)
/// - Fresh Redis instance per test
/// - Pre-configured ApiKeyService
/// - No manual setup required
pub struct TestContext {
    pub service: ApiKeyService,
    pub settings: Settings,
    _container: Option<Container<'static, Redis>>,
}

impl TestContext {
    /// Create a new test context with a fresh Redis container
    ///
    /// # Returns
    ///
    /// `Result<TestContext, Box<dyn std::error::Error>>` - Test context or error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Docker daemon is not running
    /// - Redis container fails to start
    /// - Redis connection fails
    /// - Settings initialization fails
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Check if REDIS_URL is provided (takes precedence)
        let (container, host, port) = if let Ok(redis_url) = std::env::var("REDIS_URL") {
            // Parse Redis URL to extract host and port
            // Format: redis://host:port
            let url_parts: Vec<&str> = redis_url
                .strip_prefix("redis://")
                .unwrap_or(&redis_url)
                .split(':')
                .collect();
            let host = url_parts.first().unwrap_or(&"127.0.0.1").to_string();
            let port = url_parts
                .get(1)
                .and_then(|p| p.parse().ok())
                .unwrap_or(6379);
            (None, host, port)
        } else if std::env::var("USE_LOCAL_REDIS").is_ok() {
            // Use local Redis on default port
            (None, "127.0.0.1".to_string(), 6379)
        } else {
            // Try to start Docker container
            let container = DOCKER.run(Redis);
            let port = container.get_host_port_ipv4(6379);
            (Some(container), "127.0.0.1".to_string(), port)
        };

        // Configure settings to use test Redis instance
        let mut settings = Settings::new()?;
        settings.redis.host = host;
        settings.redis.port = port;
        settings.redis.password = None; // Test container has no password

        // Set encryption key for tests (exactly 32 characters)
        settings.security.encryption_key = "test-encryption-key-32chars!!".to_string();

        // Create Redis pool and verify connection
        let redis = RedisPool::new(&settings)?;
        redis.ping().await?;

        // Clone settings for the service (service takes ownership)
        let settings_clone = settings.clone();

        // Create API Key service
        let service = ApiKeyService::new(redis, settings_clone);

        Ok(TestContext {
            service,
            settings,              // Keep original settings for tests
            _container: container, // Container will be cleaned up when dropped (if Some)
        })
    }

    /// Create test API Key options with sensible defaults
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the API key
    ///
    /// # Returns
    ///
    /// `ApiKeyCreateOptions` with default test configuration
    pub fn create_test_key_options(name: &str) -> ApiKeyCreateOptions {
        ApiKeyCreateOptions {
            name: name.to_string(),
            description: Some(format!("Test key: {}", name)),
            icon: None,
            permissions: ApiKeyPermissions::All,
            is_active: true,
            token_limit: 1000,
            concurrency_limit: 5,
            rate_limit_window: Some(60),
            rate_limit_requests: Some(1000),
            rate_limit_cost: None,
            daily_cost_limit: 100.0,
            total_cost_limit: 1000.0,
            weekly_opus_cost_limit: 50.0,
            expires_at: None,
            enable_model_restriction: false,
            restricted_models: vec![],
            enable_client_restriction: false,
            allowed_clients: vec![],
            tags: vec!["test".to_string()],
            expiration_mode: ExpirationMode::Fixed,
            activation_days: 0,
            activation_unit: ActivationUnit::Days,
            claude_account_id: None,
            claude_console_account_id: None,
            gemini_account_id: None,
            openai_account_id: None,
            azure_openai_account_id: None,
            bedrock_account_id: None,
            droid_account_id: None,
            user_id: None,
            created_by: Some("test_suite".to_string()),
            created_by_type: Some("system".to_string()),
        }
    }

    /// Create test API Key options with custom cost limits
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the API key
    /// * `daily_limit` - Daily cost limit
    /// * `total_limit` - Total cost limit
    pub fn create_limited_key_options(
        name: &str,
        daily_limit: f64,
        total_limit: f64,
    ) -> ApiKeyCreateOptions {
        let mut options = Self::create_test_key_options(name);
        options.daily_cost_limit = daily_limit;
        options.total_cost_limit = total_limit;
        options
    }

    /// Cleanup helper - permanently delete a test key
    ///
    /// This is a convenience method that ignores errors,
    /// useful for cleanup in test teardown
    ///
    /// # Arguments
    ///
    /// * `key_id` - ID of the key to delete
    pub async fn cleanup_key(&self, key_id: &str) {
        let _ = self.service.permanent_delete(key_id).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_creation() {
        let ctx = TestContext::new().await;
        assert!(ctx.is_ok(), "Failed to create test context");
    }

    #[tokio::test]
    async fn test_redis_connection() {
        let ctx = TestContext::new().await.unwrap();
        // Verify Redis is accessible by creating and retrieving a key
        let options = TestContext::create_test_key_options("test");
        let result = ctx.service.generate_key(options).await;
        assert!(result.is_ok(), "Failed to generate key with test Redis");
    }
}
