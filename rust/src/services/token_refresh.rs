use crate::redis::RedisPool;
use crate::services::ClaudeAccountService;
use crate::utils::{AppError, HttpClient, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Token 刷新锁服务
///
/// 提供分布式锁机制，避免并发刷新问题
pub struct TokenRefreshService {
    redis: Arc<RedisPool>,
    account_service: Arc<ClaudeAccountService>,
    http_client: Arc<HttpClient>,
    lock_ttl: u64,                                    // 锁的TTL（秒）
    lock_values: Arc<Mutex<HashMap<String, String>>>, // 存储每个锁的唯一值
}

/// Token 刷新配置
#[derive(Debug, Clone)]
pub struct TokenRefreshConfig {
    /// 锁的TTL（秒），默认60秒
    pub lock_ttl: u64,
    /// Token 提前刷新时间（秒），默认10秒
    pub refresh_threshold: i64,
    /// 刷新超时时间（毫秒），默认30秒
    pub refresh_timeout: u64,
    /// 失败告警阈值（连续失败次数），默认3次
    pub alert_threshold: usize,
}

impl Default for TokenRefreshConfig {
    fn default() -> Self {
        Self {
            lock_ttl: 60,
            refresh_threshold: 10,
            refresh_timeout: 30000,
            alert_threshold: 3,
        }
    }
}

/// Token 刷新响应
#[derive(Debug, Serialize, Deserialize)]
struct TokenRefreshResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64, // 秒
    #[serde(skip_serializing_if = "Option::is_none")]
    subscription: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    features: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limits: Option<serde_json::Value>,
}

/// Token 刷新结果
#[derive(Debug)]
pub struct RefreshResult {
    pub success: bool,
    pub access_token: Option<String>,
    pub expires_at: Option<i64>,
    pub error_message: Option<String>,
}

impl TokenRefreshService {
    /// 创建新的 Token 刷新服务
    pub fn new(
        redis: Arc<RedisPool>,
        account_service: Arc<ClaudeAccountService>,
        http_client: Arc<HttpClient>,
    ) -> Self {
        Self {
            redis,
            account_service,
            http_client,
            lock_ttl: 60,
            lock_values: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 创建带配置的 Token 刷新服务
    pub fn with_config(
        redis: Arc<RedisPool>,
        account_service: Arc<ClaudeAccountService>,
        http_client: Arc<HttpClient>,
        config: TokenRefreshConfig,
    ) -> Self {
        Self {
            redis,
            account_service,
            http_client,
            lock_ttl: config.lock_ttl,
            lock_values: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 辅助函数：异步释放锁（用于 scopeguard）
    ///
    /// 直接使用 Redis 删除锁，不需要完整的服务实例
    async fn release_lock_directly(redis: Arc<RedisPool>, lock_key: String) -> Result<()> {
        let mut conn = redis.get_connection().await?;
        let _: () = redis::cmd("DEL")
            .arg(&lock_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(())
    }

    // ========================================
    // 分布式锁功能
    // ========================================

    /// 获取分布式锁
    ///
    /// 使用唯一标识符作为值，避免误释放其他进程的锁
    ///
    /// # Arguments
    /// * `lock_key` - 锁的键名
    ///
    /// # Returns
    /// * `Result<bool>` - 是否成功获取锁
    async fn acquire_lock(&self, lock_key: &str) -> Result<bool> {
        let lock_id = Uuid::new_v4().to_string();
        let mut conn = self.redis.get_connection().await?;

        // SET key value NX EX ttl
        let result: Option<String> = redis::cmd("SET")
            .arg(lock_key)
            .arg(&lock_id)
            .arg("NX")
            .arg("EX")
            .arg(self.lock_ttl)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to acquire lock: {}", e)))?;

        if result.as_deref() == Some("OK") {
            let mut lock_values = self.lock_values.lock().await;
            lock_values.insert(lock_key.to_string(), lock_id.clone());
            tracing::debug!(
                "🔒 Acquired lock {} with ID {}, TTL: {}s",
                lock_key,
                lock_id,
                self.lock_ttl
            );
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 释放分布式锁
    ///
    /// 使用 Lua 脚本确保只释放自己持有的锁
    ///
    /// # Arguments
    /// * `lock_key` - 锁的键名
    async fn release_lock(&self, lock_key: &str) -> Result<()> {
        let lock_id = {
            let lock_values = self.lock_values.lock().await;
            lock_values.get(lock_key).cloned()
        };

        if lock_id.is_none() {
            tracing::warn!("⚠️ No lock ID found for {}, skipping release", lock_key);
            return Ok(());
        }

        let lock_id = lock_id.unwrap();

        // Lua 脚本：只有当值匹配时才删除
        let lua_script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end
        "#;

        let mut conn = self.redis.get_connection().await?;
        let result: i32 = redis::Script::new(lua_script)
            .key(lock_key)
            .arg(&lock_id)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to release lock: {}", e)))?;

        if result == 1 {
            let mut lock_values = self.lock_values.lock().await;
            lock_values.remove(lock_key);
            tracing::debug!("🔓 Released lock {} with ID {}", lock_key, lock_id);
        } else {
            tracing::warn!(
                "⚠️ Lock {} was not released - value mismatch or already expired",
                lock_key
            );
        }

        Ok(())
    }

    /// 获取刷新锁
    ///
    /// # Arguments
    /// * `account_id` - 账户ID
    /// * `platform` - 平台类型 (claude/gemini)
    ///
    /// # Returns
    /// * `Result<bool>` - 是否成功获取锁
    pub async fn acquire_refresh_lock(&self, account_id: &str, platform: &str) -> Result<bool> {
        let lock_key = format!("token_refresh_lock:{}:{}", platform, account_id);
        self.acquire_lock(&lock_key).await
    }

    /// 释放刷新锁
    ///
    /// # Arguments
    /// * `account_id` - 账户ID
    /// * `platform` - 平台类型 (claude/gemini)
    pub async fn release_refresh_lock(&self, account_id: &str, platform: &str) -> Result<()> {
        let lock_key = format!("token_refresh_lock:{}:{}", platform, account_id);
        self.release_lock(&lock_key).await
    }

    /// 检查刷新锁状态
    ///
    /// # Arguments
    /// * `account_id` - 账户ID
    /// * `platform` - 平台类型 (claude/gemini)
    ///
    /// # Returns
    /// * `Result<bool>` - 锁是否存在
    pub async fn is_refresh_locked(&self, account_id: &str, platform: &str) -> Result<bool> {
        let lock_key = format!("token_refresh_lock:{}:{}", platform, account_id);
        let exists: bool = self.redis.exists(&lock_key).await?;
        Ok(exists)
    }

    /// 获取锁的剩余TTL
    ///
    /// # Arguments
    /// * `account_id` - 账户ID
    /// * `platform` - 平台类型 (claude/gemini)
    ///
    /// # Returns
    /// * `Result<i64>` - 剩余秒数，-1表示锁不存在
    pub async fn get_lock_ttl(&self, account_id: &str, platform: &str) -> Result<i64> {
        let lock_key = format!("token_refresh_lock:{}:{}", platform, account_id);
        let mut conn = self.redis.get_connection().await?;

        let ttl: i64 = redis::cmd("TTL")
            .arg(&lock_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get lock TTL: {}", e)))?;

        Ok(ttl)
    }

    /// 清理本地锁记录
    ///
    /// 在进程退出时调用，避免内存泄漏
    pub async fn cleanup(&self) {
        let mut lock_values = self.lock_values.lock().await;
        lock_values.clear();
        tracing::info!("🧹 Cleaned up local lock records");
    }

    // ========================================
    // Token 过期检测
    // ========================================

    /// 检查 Token 是否即将过期
    ///
    /// # Arguments
    /// * `expires_at` - Token 过期时间（毫秒时间戳）
    /// * `threshold_seconds` - 提前刷新阈值（秒），默认10秒
    ///
    /// # Returns
    /// * `bool` - 是否需要刷新
    pub fn is_token_expiring(expires_at: i64, threshold_seconds: Option<i64>) -> bool {
        let threshold = threshold_seconds.unwrap_or(10);
        let now = Utc::now().timestamp_millis();
        let threshold_ms = threshold * 1000;

        // Token 已过期或将在 threshold 秒内过期
        expires_at - now <= threshold_ms
    }

    /// 检查账户 Token 是否需要刷新
    ///
    /// # Arguments
    /// * `account_id` - 账户ID
    ///
    /// # Returns
    /// * `Result<bool>` - 是否需要刷新
    pub async fn should_refresh_token(&self, account_id: &str) -> Result<bool> {
        let account = self
            .account_service
            .get_account(account_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Account {} not found", account_id)))?;

        // 检查是否有过期时间
        if let Some(expires_at_str) = account.expires_at {
            // 尝试解析为 i64
            if let Ok(expires_at) = expires_at_str.parse::<i64>() {
                return Ok(Self::is_token_expiring(expires_at, Some(10)));
            }
        }

        // 如果没有过期时间或解析失败，返回 false
        Ok(false)
    }

    // ========================================
    // Token 刷新功能
    // ========================================

    /// 刷新账户 OAuth Token
    ///
    /// # Arguments
    /// * `account_id` - 账户ID
    ///
    /// # Returns
    /// * `Result<RefreshResult>` - 刷新结果
    pub async fn refresh_account_token(&self, account_id: &str) -> Result<RefreshResult> {
        // 1. 获取账户数据
        let account = match self
            .account_service
            .get_account_decrypted(account_id)
            .await?
        {
            Some(acc) => acc,
            None => {
                return Ok(RefreshResult {
                    success: false,
                    access_token: None,
                    expires_at: None,
                    error_message: Some("Account not found".to_string()),
                });
            }
        };

        // 2. 检查 refresh_token
        let refresh_token = match account.refresh_token {
            Some(ref token) if !token.is_empty() => token.clone(),
            _ => {
                return Ok(RefreshResult {
                    success: false,
                    access_token: None,
                    expires_at: None,
                    error_message: Some(
                        "No refresh token available - manual token update required".to_string(),
                    ),
                });
            }
        };

        // 3. 尝试获取分布式锁
        let lock_acquired = self
            .acquire_refresh_lock(account_id, "claude")
            .await
            .unwrap_or(false);

        if !lock_acquired {
            // 如果无法获取锁，说明另一个进程正在刷新
            tracing::info!(
                "🔒 Token refresh already in progress for account: {} ({})",
                account.name,
                account_id
            );

            // 等待一段时间后返回，期望其他进程已完成刷新
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // 重新获取账户数据（可能已被其他进程刷新）
            if let Some(updated_account) = self
                .account_service
                .get_account_decrypted(account_id)
                .await?
            {
                if let Some(access_token) = updated_account.access_token {
                    if !access_token.is_empty() {
                        let expires_at = updated_account
                            .expires_at
                            .and_then(|s| s.parse::<i64>().ok());
                        return Ok(RefreshResult {
                            success: true,
                            access_token: Some(access_token),
                            expires_at,
                            error_message: None,
                        });
                    }
                }
            }

            return Ok(RefreshResult {
                success: false,
                access_token: None,
                expires_at: None,
                error_message: Some("Token refresh in progress by another process".to_string()),
            });
        }

        // 使用 scopeguard 确保锁一定会被释放
        let _guard = scopeguard::guard((), |_| {
            let redis = self.redis.clone();
            let lock_key = format!("token_refresh:claude:{}", account_id);
            tokio::spawn(async move {
                let _ = Self::release_lock_directly(redis, lock_key).await;
            });
        });

        // 4. 记录开始刷新
        tracing::info!(
            "🔄 Starting token refresh for account: {} ({})",
            account.name,
            account_id
        );

        // 5. 发送刷新请求到 Claude OAuth API
        let refresh_result = self
            .send_refresh_request(&refresh_token, account.proxy.as_deref())
            .await;

        match refresh_result {
            Ok(response) => {
                // 6. 更新账户数据
                let expires_at =
                    (Utc::now().timestamp_millis() + response.expires_in * 1000).to_string();

                // 构造更新选项
                let update_options = crate::models::account::CreateClaudeAccountOptions {
                    name: account.name.clone(),
                    description: account.description.clone(),
                    email: account.email.clone(),
                    password: None,
                    refresh_token: Some(response.refresh_token.clone()),
                    claude_ai_oauth: Some(crate::models::account::ClaudeOAuthData {
                        access_token: response.access_token.clone(),
                        refresh_token: response.refresh_token.clone(),
                        expires_at: Utc::now().timestamp_millis() + response.expires_in * 1000,
                        scopes: account
                            .scopes
                            .as_ref()
                            .map(|s| s.split_whitespace().map(String::from).collect())
                            .unwrap_or_default(),
                    }),
                    proxy: account
                        .proxy
                        .as_ref()
                        .and_then(|p| serde_json::from_str(p).ok()),
                    is_active: true,
                    account_type: account.account_type,
                    platform: account.platform,
                    priority: account.priority,
                    schedulable: account.schedulable,
                    subscription_info: response.subscription.map(|s| {
                        crate::models::account::SubscriptionInfo {
                            subscription: Some(s.to_string()),
                            plan: response.plan.clone(),
                            tier: response.tier.clone(),
                            account_type: response.account_type.clone(),
                            features: response.features.as_ref().and_then(|f| {
                                if let serde_json::Value::Array(arr) = f {
                                    Some(
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect(),
                                    )
                                } else {
                                    None
                                }
                            }),
                            limits: response.limits.clone(),
                        }
                    }),
                    auto_stop_on_warning: account.auto_stop_on_warning,
                    use_unified_user_agent: account.use_unified_user_agent,
                    use_unified_client_id: account.use_unified_client_id,
                    unified_client_id: account.unified_client_id.clone(),
                    expires_at: Some(expires_at.clone()),
                    ext_info: account
                        .ext_info
                        .as_ref()
                        .and_then(|e| serde_json::from_str(e).ok()),
                };

                // 更新账户
                match self
                    .account_service
                    .update_account(account_id, update_options)
                    .await
                {
                    Ok(_) => {
                        tracing::info!(
                            "✅ Successfully refreshed token for account: {} ({})",
                            account.name,
                            account_id
                        );

                        Ok(RefreshResult {
                            success: true,
                            access_token: Some(response.access_token),
                            expires_at: Some(expires_at.parse().unwrap_or(0)),
                            error_message: None,
                        })
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to update account after refresh: {}", e);
                        Ok(RefreshResult {
                            success: false,
                            access_token: None,
                            expires_at: None,
                            error_message: Some(format!("Failed to update account: {}", e)),
                        })
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "❌ Failed to refresh token for account {} ({}): {}",
                    account.name,
                    account_id,
                    e
                );

                Ok(RefreshResult {
                    success: false,
                    access_token: None,
                    expires_at: None,
                    error_message: Some(format!("Token refresh failed: {}", e)),
                })
            }
        }
    }

    /// 发送 Token 刷新请求到 Claude OAuth API
    ///
    /// # Arguments
    /// * `refresh_token` - 刷新令牌
    /// * `proxy` - 代理配置（JSON字符串）
    ///
    /// # Returns
    /// * `Result<TokenRefreshResponse>` - 刷新响应
    async fn send_refresh_request(
        &self,
        refresh_token: &str,
        proxy: Option<&str>,
    ) -> Result<TokenRefreshResponse> {
        const CLAUDE_OAUTH_URL: &str = "https://console.anthropic.com/v1/oauth/token";
        const CLAUDE_OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

        // 构造请求体
        let request_body = serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": CLAUDE_OAUTH_CLIENT_ID,
        });

        // 设置请求头
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/json, text/plain, */*".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            "claude-cli/1.0.56 (external, cli)".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            "en-US,en;q=0.9".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::REFERER,
            "https://claude.ai/".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ORIGIN,
            "https://claude.ai".parse().unwrap(),
        );

        // 构建 HTTP 客户端（带代理支持）
        let client = if let Some(proxy_str) = proxy {
            // TODO: 解析代理配置并设置
            tracing::debug!("Using proxy configuration: {}", proxy_str);
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(30000))
                .default_headers(headers)
                .build()
                .map_err(|e| AppError::InternalError(format!("Failed to build client: {}", e)))?
        } else {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(30000))
                .default_headers(headers)
                .build()
                .map_err(|e| AppError::InternalError(format!("Failed to build client: {}", e)))?
        };

        // 发送请求
        let response = client
            .post(CLAUDE_OAUTH_URL)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("OAuth request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::InternalError(format!(
                "OAuth token refresh failed with status {}: {}",
                status, error_text
            )));
        }

        // 解析响应
        let refresh_response: TokenRefreshResponse = response
            .json()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to parse response: {}", e)))?;

        Ok(refresh_response)
    }

    // ========================================
    // 自动刷新任务
    // ========================================

    /// 启动自动刷新任务（后台定时器）
    ///
    /// 定期检查所有账户的 Token 是否即将过期，自动刷新
    ///
    /// # Arguments
    /// * `interval_minutes` - 检查间隔（分钟），默认 5 分钟
    /// * `batch_size` - 每批处理的账户数量，默认 10
    ///
    /// # Returns
    /// * `tokio::task::JoinHandle` - 后台任务句柄，可用于取消任务
    pub fn start_auto_refresh_task(
        self: Arc<Self>,
        interval_minutes: Option<u64>,
        batch_size: Option<usize>,
    ) -> tokio::task::JoinHandle<()> {
        let interval = interval_minutes.unwrap_or(5);
        let batch_size = batch_size.unwrap_or(10);

        tracing::info!(
            "🔄 Starting auto token refresh task (interval: {} minutes, batch size: {})",
            interval,
            batch_size
        );

        tokio::spawn(async move {
            let mut interval_timer =
                tokio::time::interval(tokio::time::Duration::from_secs(interval * 60));
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval_timer.tick().await;

                tracing::debug!("🔍 Auto refresh task: checking accounts...");

                // 执行批量刷新
                match self.refresh_expiring_accounts(batch_size).await {
                    Ok((total, success, failed)) => {
                        if total > 0 {
                            tracing::info!(
                                "✅ Auto refresh completed: {} accounts checked, {} refreshed, {} failed",
                                total,
                                success,
                                failed
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Auto refresh task error: {}", e);
                    }
                }
            }
        })
    }

    /// 刷新所有即将过期的账户
    ///
    /// 批量检查账户列表，找出即将过期的账户并刷新
    ///
    /// # Arguments
    /// * `batch_size` - 每批处理的账户数量
    ///
    /// # Returns
    /// * `Result<(usize, usize, usize)>` - (总数, 成功数, 失败数)
    pub async fn refresh_expiring_accounts(
        &self,
        batch_size: usize,
    ) -> Result<(usize, usize, usize)> {
        // 1. 获取所有账户列表
        let accounts = self.account_service.list_accounts(0, 1000).await?;

        if accounts.is_empty() {
            tracing::debug!("No accounts found for auto refresh");
            return Ok((0, 0, 0));
        }

        tracing::debug!("🔍 Checking {} accounts for token expiry", accounts.len());

        // 2. 筛选需要刷新的账户
        let mut accounts_to_refresh = Vec::new();
        for account in accounts {
            // 只处理 OAuth 账户（有 expires_at 的账户）
            if let Some(ref expires_at_str) = account.expires_at {
                // 解析过期时间
                if let Ok(expires_at) = expires_at_str.parse::<i64>() {
                    // 检查是否即将过期（10秒阈值）
                    if Self::is_token_expiring(expires_at, Some(10)) {
                        accounts_to_refresh.push((account.id, expires_at));
                    }
                }
            }
        }

        let total = accounts_to_refresh.len();
        if total == 0 {
            tracing::debug!("No accounts need token refresh");
            return Ok((0, 0, 0));
        }

        tracing::info!(
            "🔄 Found {} accounts with expiring tokens, starting batch refresh...",
            total
        );

        // 3. 分批刷新
        let mut success_count = 0;
        let mut failed_count = 0;

        for chunk in accounts_to_refresh.chunks(batch_size) {
            // 并发刷新当前批次
            let refresh_tasks: Vec<_> = chunk
                .iter()
                .map(|(account_id, expires_at)| {
                    let account_id_str = account_id.to_string();
                    let expires_at = *expires_at;
                    // 创建一个 self 的引用副本（通过 Redis, account_service, http_client 的 Arc）
                    let redis = Arc::clone(&self.redis);
                    let account_service = Arc::clone(&self.account_service);
                    let http_client = Arc::clone(&self.http_client);
                    let lock_ttl = self.lock_ttl;
                    let lock_values = Arc::clone(&self.lock_values);

                    tokio::spawn(async move {
                        // 重建 service 实例
                        let service = TokenRefreshService {
                            redis,
                            account_service,
                            http_client,
                            lock_ttl,
                            lock_values,
                        };

                        tracing::debug!(
                            "🔄 Refreshing token for account: {} (expires_at: {})",
                            account_id_str,
                            expires_at
                        );

                        match service.refresh_account_token(&account_id_str).await {
                            Ok(result) => {
                                if result.success {
                                    tracing::info!(
                                        "✅ Successfully refreshed token for account: {}",
                                        account_id_str
                                    );
                                    Ok(())
                                } else {
                                    let error_msg = result
                                        .error_message
                                        .unwrap_or_else(|| "Unknown error".to_string());
                                    tracing::warn!(
                                        "⚠️ Failed to refresh token for account {}: {}",
                                        account_id_str,
                                        error_msg
                                    );
                                    Err(error_msg)
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "❌ Error refreshing token for account {}: {}",
                                    account_id_str,
                                    e
                                );
                                Err(e.to_string())
                            }
                        }
                    })
                })
                .collect();

            // 等待当前批次完成
            let results = futures::future::join_all(refresh_tasks).await;

            // 统计结果
            for result in results {
                match result {
                    Ok(Ok(())) => success_count += 1,
                    Ok(Err(_)) | Err(_) => failed_count += 1,
                }
            }

            // 批次间稍微延迟，避免过载
            if chunk.len() >= batch_size {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        tracing::info!(
            "📊 Batch refresh summary: total={}, success={}, failed={}",
            total,
            success_count,
            failed_count
        );

        // 失败告警检查
        if failed_count > 0 {
            let failure_rate = (failed_count as f64 / total as f64) * 100.0;

            if failure_rate >= 50.0 {
                // 严重告警：失败率 >= 50%
                tracing::error!(
                    "🚨 CRITICAL: Token refresh failure rate is {:.1}% ({}/{})",
                    failure_rate,
                    failed_count,
                    total
                );
            } else if failure_rate >= 30.0 {
                // 警告：失败率 >= 30%
                tracing::warn!(
                    "⚠️ WARNING: Token refresh failure rate is {:.1}% ({}/{})",
                    failure_rate,
                    failed_count,
                    total
                );
            } else if failed_count >= 3 {
                // 一般告警：失败次数 >= 3
                tracing::warn!(
                    "⚠️ Token refresh failures detected: {} out of {} accounts failed",
                    failed_count,
                    total
                );
            }
        }

        Ok((total, success_count, failed_count))
    }

    /// 手动触发一次刷新检查（用于测试或手动触发）
    ///
    /// # Arguments
    /// * `batch_size` - 批处理大小，默认 10
    ///
    /// # Returns
    /// * `Result<(usize, usize, usize)>` - (总数, 成功数, 失败数)
    pub async fn trigger_refresh_check(
        &self,
        batch_size: Option<usize>,
    ) -> Result<(usize, usize, usize)> {
        let batch_size = batch_size.unwrap_or(10);
        tracing::info!(
            "🔄 Manual refresh check triggered (batch size: {})",
            batch_size
        );
        self.refresh_expiring_accounts(batch_size).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_token_expiring_already_expired() {
        let now = Utc::now().timestamp_millis();
        let past = now - 10000; // 10秒前

        assert!(TokenRefreshService::is_token_expiring(past, Some(10)));
    }

    #[test]
    fn test_is_token_expiring_within_threshold() {
        let now = Utc::now().timestamp_millis();
        let soon = now + 5000; // 5秒后

        assert!(TokenRefreshService::is_token_expiring(soon, Some(10)));
    }

    #[test]
    fn test_is_token_expiring_not_yet() {
        let now = Utc::now().timestamp_millis();
        let future = now + 60000; // 60秒后

        assert!(!TokenRefreshService::is_token_expiring(future, Some(10)));
    }

    #[test]
    fn test_is_token_expiring_default_threshold() {
        let now = Utc::now().timestamp_millis();
        let soon = now + 5000; // 5秒后

        assert!(TokenRefreshService::is_token_expiring(soon, None));
    }

    #[test]
    fn test_refresh_result_success() {
        let result = RefreshResult {
            success: true,
            access_token: Some("new_access_token".to_string()),
            expires_at: Some(1234567890),
            error_message: None,
        };

        assert!(result.success);
        assert!(result.access_token.is_some());
        assert_eq!(result.access_token.unwrap(), "new_access_token");
        assert_eq!(result.expires_at, Some(1234567890));
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_refresh_result_failure() {
        let result = RefreshResult {
            success: false,
            access_token: None,
            expires_at: None,
            error_message: Some("Token refresh failed".to_string()),
        };

        assert!(!result.success);
        assert!(result.access_token.is_none());
        assert!(result.expires_at.is_none());
        assert!(result.error_message.is_some());
        assert_eq!(result.error_message.unwrap(), "Token refresh failed");
    }

    #[test]
    fn test_token_refresh_response_parsing() {
        let json = r#"{
            "access_token": "new_token_123",
            "refresh_token": "refresh_token_456",
            "expires_in": 3600
        }"#;

        let response: TokenRefreshResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "new_token_123");
        assert_eq!(response.refresh_token, "refresh_token_456");
        assert_eq!(response.expires_in, 3600);
    }

    #[test]
    fn test_is_token_expiring_edge_cases() {
        let now = Utc::now().timestamp_millis();

        // 正好在阈值边界（10秒）
        let exactly_threshold = now + 10000;
        assert!(TokenRefreshService::is_token_expiring(
            exactly_threshold,
            Some(10)
        ));

        // 略微超过阈值
        let just_over_threshold = now + 10001;
        assert!(!TokenRefreshService::is_token_expiring(
            just_over_threshold,
            Some(10)
        ));

        // 零阈值情况
        let future = now + 1000;
        assert!(!TokenRefreshService::is_token_expiring(future, Some(0)));
    }

    #[test]
    fn test_token_refresh_response_json_compatibility() {
        // 测试与 Node.js 版本的 JSON 格式兼容性
        let response = TokenRefreshResponse {
            access_token: "test_token".to_string(),
            refresh_token: "test_refresh".to_string(),
            expires_in: 3600,
            subscription: None,
            plan: None,
            tier: None,
            account_type: None,
            features: None,
            limits: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("access_token"));
        assert!(json.contains("expires_in"));
        assert!(json.contains("test_token"));
        assert!(json.contains("3600"));
    }

    #[test]
    fn test_refresh_result_default_values() {
        // 测试默认值场景
        let result = RefreshResult {
            success: false,
            access_token: None,
            expires_at: None,
            error_message: None,
        };

        assert!(!result.success);
        assert!(result.access_token.is_none());
        assert!(result.expires_at.is_none());
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_is_token_expiring_negative_values() {
        // 测试负数时间戳（历史时间）
        let past_timestamp = -1000;
        assert!(TokenRefreshService::is_token_expiring(
            past_timestamp,
            Some(10)
        ));
    }

    #[test]
    fn test_is_token_expiring_large_threshold() {
        let now = Utc::now().timestamp_millis();
        let future = now + 30000; // 30秒后

        // 使用较大的阈值（60秒）
        assert!(TokenRefreshService::is_token_expiring(future, Some(60)));

        // 使用较小的阈值（10秒）
        assert!(!TokenRefreshService::is_token_expiring(future, Some(10)));
    }
}
