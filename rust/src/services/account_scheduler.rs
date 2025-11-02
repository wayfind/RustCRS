use crate::models::account::{AccountType, ClaudeAccount, Platform};
use crate::redis::RedisPool;
use crate::services::ClaudeAccountService;
use crate::utils::{AppError, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 账户调度器配置
#[derive(Debug, Clone)]
pub struct AccountSchedulerConfig {
    /// 粘性会话 TTL（小时），默认 1 小时
    pub sticky_session_ttl_hours: u64,
    /// 粘性会话续期阈值（分钟），默认 0（不续期）
    pub sticky_session_renewal_threshold_minutes: u64,
    /// 并发限制检查开关，默认 true
    pub concurrent_limit_enabled: bool,
    /// 529 错误处理时间（分钟），0 表示禁用
    pub overload_handling_minutes: u64,
}

impl Default for AccountSchedulerConfig {
    fn default() -> Self {
        Self {
            sticky_session_ttl_hours: 1,
            sticky_session_renewal_threshold_minutes: 0,
            concurrent_limit_enabled: true,
            overload_handling_minutes: 10,
        }
    }
}

/// 会话映射数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMapping {
    /// 账户 ID
    pub account_id: String,
    /// 账户类型
    pub account_type: AccountType,
    /// 平台类型
    pub platform: Platform,
    /// 映射创建时间
    pub created_at: i64,
}

/// 账户选择结果
#[derive(Debug, Clone)]
pub struct SelectedAccount {
    /// 账户 ID
    pub account_id: String,
    /// 账户类型
    pub account_type: AccountType,
    /// 平台类型
    pub platform: Platform,
    /// 账户名称
    pub account_name: String,
    /// 优先级
    pub priority: u8,
}

/// Claude 账户调度器
///
/// 提供智能账户选择、粘性会话、并发控制和故障转移功能
pub struct AccountScheduler {
    redis: Arc<RedisPool>,
    account_service: Arc<ClaudeAccountService>,
    config: AccountSchedulerConfig,
    session_mapping_prefix: String,
}

impl AccountScheduler {
    /// 创建新的账户调度器
    pub fn new(redis: Arc<RedisPool>, account_service: Arc<ClaudeAccountService>) -> Self {
        Self {
            redis,
            account_service,
            config: AccountSchedulerConfig::default(),
            session_mapping_prefix: "unified_claude_session_mapping:".to_string(),
        }
    }

    /// 创建带配置的账户调度器
    pub fn with_config(
        redis: Arc<RedisPool>,
        account_service: Arc<ClaudeAccountService>,
        config: AccountSchedulerConfig,
    ) -> Self {
        Self {
            redis,
            account_service,
            config,
            session_mapping_prefix: "unified_claude_session_mapping:".to_string(),
        }
    }

    // ========================================
    // 粘性会话管理
    // ========================================

    /// 获取会话映射
    ///
    /// # Arguments
    /// * `session_hash` - 会话哈希
    ///
    /// # Returns
    /// * `Result<Option<SessionMapping>>` - 会话映射，如果不存在则返回 None
    pub async fn get_session_mapping(&self, session_hash: &str) -> Result<Option<SessionMapping>> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mut conn = self.redis.get_connection().await?;

        let mapping_data: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get session mapping: {}", e)))?;

        if let Some(data) = mapping_data {
            let mapping: SessionMapping = serde_json::from_str(&data).map_err(|e| {
                AppError::InternalError(format!("Failed to parse session mapping: {}", e))
            })?;
            Ok(Some(mapping))
        } else {
            Ok(None)
        }
    }

    /// 设置会话映射
    ///
    /// # Arguments
    /// * `session_hash` - 会话哈希
    /// * `mapping` - 会话映射数据
    ///
    /// # Returns
    /// * `Result<()>`
    pub async fn set_session_mapping(
        &self,
        session_hash: &str,
        mapping: SessionMapping,
    ) -> Result<()> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mapping_data = serde_json::to_string(&mapping).map_err(|e| {
            AppError::InternalError(format!("Failed to serialize session mapping: {}", e))
        })?;

        let ttl_seconds = self.config.sticky_session_ttl_hours * 60 * 60;
        let mut conn = self.redis.get_connection().await?;

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl_seconds)
            .arg(&mapping_data)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to set session mapping: {}", e)))?;

        tracing::debug!(
            "🎯 Created session mapping: {} -> {} ({:?})",
            session_hash,
            mapping.account_id,
            mapping.account_type
        );

        Ok(())
    }

    /// 删除会话映射
    ///
    /// # Arguments
    /// * `session_hash` - 会话哈希
    ///
    /// # Returns
    /// * `Result<()>`
    pub async fn delete_session_mapping(&self, session_hash: &str) -> Result<()> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mut conn = self.redis.get_connection().await?;

        redis::cmd("DEL")
            .arg(&key)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| {
                AppError::RedisError(format!("Failed to delete session mapping: {}", e))
            })?;

        tracing::debug!("🗑️ Deleted session mapping: {}", session_hash);

        Ok(())
    }

    /// 续期会话映射 TTL
    ///
    /// # Arguments
    /// * `session_hash` - 会话哈希
    ///
    /// # Returns
    /// * `Result<bool>` - 是否续期成功
    pub async fn extend_session_mapping_ttl(&self, session_hash: &str) -> Result<bool> {
        // 如果续期阈值为 0，不进行续期
        if self.config.sticky_session_renewal_threshold_minutes == 0 {
            return Ok(true);
        }

        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mut conn = self.redis.get_connection().await?;

        // 检查当前 TTL
        let remaining_ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get TTL: {}", e)))?;

        // -2: key 不存在；-1: 无过期时间
        if remaining_ttl == -2 {
            return Ok(false);
        }
        if remaining_ttl == -1 {
            return Ok(true);
        }

        let full_ttl_seconds = self.config.sticky_session_ttl_hours * 60 * 60;
        let renewal_threshold_seconds = self.config.sticky_session_renewal_threshold_minutes * 60;

        // 如果剩余时间小于阈值，续期
        if remaining_ttl < renewal_threshold_seconds as i64 {
            redis::cmd("EXPIRE")
                .arg(&key)
                .arg(full_ttl_seconds)
                .query_async::<_, ()>(&mut conn)
                .await
                .map_err(|e| AppError::RedisError(format!("Failed to extend TTL: {}", e)))?;

            tracing::debug!(
                "🔁 Extended session mapping TTL: {} (remaining: {}s -> {}s)",
                session_hash,
                remaining_ttl,
                full_ttl_seconds
            );
        }

        Ok(true)
    }

    // ========================================
    // 账户选择算法
    // ========================================

    /// 选择最优账户
    ///
    /// 综合考虑：
    /// 1. 粘性会话（如果存在）
    /// 2. 账户状态（active、可调度）
    /// 3. Token 有效性
    /// 4. 并发限制
    /// 5. 优先级排序
    ///
    /// # Arguments
    /// * `session_hash` - 会话哈希（可选）
    /// * `platform` - 平台类型
    ///
    /// # Returns
    /// * `Result<SelectedAccount>` - 选中的账户
    pub async fn select_account(
        &self,
        session_hash: Option<&str>,
        platform: Platform,
    ) -> Result<SelectedAccount> {
        // 1. 检查粘性会话
        if let Some(hash) = session_hash {
            if let Some(mapping) = self.get_session_mapping(hash).await? {
                // 验证映射的账户是否仍然可用
                if let Ok(Some(account)) = self.account_service.get_account(&mapping.account_id).await {
                    if self.is_account_available(&account).await? {
                        // 续期 TTL
                        self.extend_session_mapping_ttl(hash).await?;

                        tracing::info!(
                            "🎯 Using sticky session account: {} ({})",
                            account.name,
                            mapping.account_id
                        );

                        return Ok(SelectedAccount {
                            account_id: mapping.account_id,
                            account_type: mapping.account_type,
                            platform: mapping.platform,
                            account_name: account.name,
                            priority: account.priority,
                        });
                    } else {
                        tracing::warn!(
                            "⚠️ Mapped account {} is no longer available, selecting new account",
                            mapping.account_id
                        );
                        self.delete_session_mapping(hash).await?;
                    }
                }
            }
        }

        // 2. 从账户池选择
        let selected = self.select_from_pool(platform).await?;

        // 3. 创建粘性会话映射
        if let Some(hash) = session_hash {
            let mapping = SessionMapping {
                account_id: selected.account_id.clone(),
                account_type: selected.account_type.clone(),
                platform: selected.platform,
                created_at: Utc::now().timestamp_millis(),
            };
            self.set_session_mapping(hash, mapping).await?;

            tracing::info!(
                "🎯 Created new sticky session mapping: {} -> {}",
                hash,
                selected.account_name
            );
        }

        tracing::info!(
            "🎯 Selected account: {} ({}) with priority {}",
            selected.account_name,
            selected.account_id,
            selected.priority
        );

        Ok(selected)
    }

    /// 从账户池选择最优账户
    ///
    /// # Arguments
    /// * `platform` - 平台类型
    ///
    /// # Returns
    /// * `Result<SelectedAccount>` - 选中的账户
    async fn select_from_pool(&self, platform: Platform) -> Result<SelectedAccount> {
        // 获取所有账户
        let accounts = self.account_service.list_accounts(0, 1000).await?;

        if accounts.is_empty() {
            return Err(AppError::InternalError(
                "No accounts available in pool".to_string(),
            ));
        }

        // 筛选可用账户
        let mut available_accounts = Vec::new();
        for account in accounts {
            // 平台匹配
            if account.platform != platform {
                continue;
            }

            // 检查账户可用性
            if self.is_account_available(&account).await? {
                available_accounts.push(account);
            }
        }

        if available_accounts.is_empty() {
            return Err(AppError::InternalError(format!(
                "No available {:?} accounts in pool",
                platform
            )));
        }

        // 按优先级排序（优先级高的在前，数字越小优先级越高）
        available_accounts.sort_by_key(|a| a.priority);

        // 选择第一个（最高优先级）
        let selected = &available_accounts[0];

        Ok(SelectedAccount {
            account_id: selected.id.to_string(),
            account_type: selected.account_type.clone(),
            platform, // 使用传入的 platform 参数
            account_name: selected.name.clone(),
            priority: selected.priority,
        })
    }

    /// 检查账户是否可用
    ///
    /// 综合检查：
    /// 1. 账户状态 (is_active = true)
    /// 2. 可调度 (schedulable = true)
    /// 3. Token 未过期
    /// 4. 未处于 529 过载状态
    /// 5. 并发限制未满
    ///
    /// # Arguments
    /// * `account` - 账户信息
    ///
    /// # Returns
    /// * `Result<bool>` - 是否可用
    async fn is_account_available(&self, account: &ClaudeAccount) -> Result<bool> {
        // 1. 检查基本状态
        if !account.is_active {
            tracing::debug!("Account {} is not active", account.id);
            return Ok(false);
        }

        if !account.schedulable {
            tracing::debug!("Account {} is not schedulable", account.id);
            return Ok(false);
        }

        // 2. 检查 Token 过期（如果有 expires_at）
        if let Some(ref expires_at_str) = account.expires_at {
            if let Ok(expires_at) = expires_at_str.parse::<i64>() {
                let now = Utc::now().timestamp_millis();
                if expires_at <= now {
                    tracing::debug!("Account {} token has expired", account.id);
                    return Ok(false);
                }
            }
        }

        // 3. 检查 529 过载状态
        if self.config.overload_handling_minutes > 0
            && self.is_account_overloaded(&account.id.to_string()).await?
        {
            tracing::debug!("Account {} is in overload state", account.id);
            return Ok(false);
        }

        // 4. 检查并发限制（暂时禁用，等待模型添加 max_concurrent_requests 字段）
        // TODO: 在 ClaudeAccount 模型中添加 max_concurrent_requests 字段后启用
        /*
        if self.config.concurrent_limit_enabled {
            if let Some(max_concurrent) = account.max_concurrent_requests {
                let current_concurrent = self.get_account_concurrency(&account.id.to_string()).await?;
                if current_concurrent >= max_concurrent as usize {
                    tracing::debug!(
                        "Account {} concurrent limit reached: {}/{}",
                        account.id,
                        current_concurrent,
                        max_concurrent
                    );
                    return Ok(false);
                }
            }
        }
        */

        Ok(true)
    }

    // ========================================
    // 并发控制
    // ========================================

    /// 获取账户当前并发数
    ///
    /// # Arguments
    /// * `account_id` - 账户 ID
    ///
    /// # Returns
    /// * `Result<usize>` - 当前并发数
    pub async fn get_account_concurrency(&self, account_id: &str) -> Result<usize> {
        let key = format!("concurrency:{}", account_id);
        let mut conn = self.redis.get_connection().await?;

        let count: usize = redis::cmd("ZCOUNT")
            .arg(&key)
            .arg(Utc::now().timestamp_millis())
            .arg("+inf")
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get concurrency: {}", e)))?;

        Ok(count)
    }

    /// 增加账户并发计数
    ///
    /// # Arguments
    /// * `account_id` - 账户 ID
    /// * `request_id` - 请求 ID（唯一标识）
    /// * `ttl_seconds` - 过期时间（秒），默认 600（10分钟）
    ///
    /// # Returns
    /// * `Result<()>`
    pub async fn increment_concurrency(
        &self,
        account_id: &str,
        request_id: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<()> {
        let key = format!("concurrency:{}", account_id);
        let ttl = ttl_seconds.unwrap_or(600);
        let expiry_time = Utc::now().timestamp_millis() + (ttl as i64 * 1000);

        let mut conn = self.redis.get_connection().await?;

        // 添加到 Sorted Set（score 为过期时间）
        redis::cmd("ZADD")
            .arg(&key)
            .arg(expiry_time)
            .arg(request_id)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to increment concurrency: {}", e)))?;

        // 设置 key 过期时间（避免 key 永久存在）
        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(ttl + 60) // 额外 60 秒缓冲
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| {
                AppError::RedisError(format!("Failed to set concurrency expiry: {}", e))
            })?;

        Ok(())
    }

    /// 减少账户并发计数
    ///
    /// # Arguments
    /// * `account_id` - 账户 ID
    /// * `request_id` - 请求 ID
    ///
    /// # Returns
    /// * `Result<()>`
    pub async fn decrement_concurrency(&self, account_id: &str, request_id: &str) -> Result<()> {
        let key = format!("concurrency:{}", account_id);
        let mut conn = self.redis.get_connection().await?;

        redis::cmd("ZREM")
            .arg(&key)
            .arg(request_id)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to decrement concurrency: {}", e)))?;

        Ok(())
    }

    /// 清理过期的并发记录
    ///
    /// # Arguments
    /// * `account_id` - 账户 ID
    ///
    /// # Returns
    /// * `Result<usize>` - 清理的记录数
    pub async fn cleanup_expired_concurrency(&self, account_id: &str) -> Result<usize> {
        let key = format!("concurrency:{}", account_id);
        let mut conn = self.redis.get_connection().await?;

        let removed: usize = redis::cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg("-inf")
            .arg(Utc::now().timestamp_millis())
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to cleanup concurrency: {}", e)))?;

        if removed > 0 {
            tracing::debug!(
                "🧹 Cleaned up {} expired concurrency records for account {}",
                removed,
                account_id
            );
        }

        Ok(removed)
    }

    // ========================================
    // 故障转移（529 过载处理）
    // ========================================

    /// 标记账户为过载状态
    ///
    /// # Arguments
    /// * `account_id` - 账户 ID
    ///
    /// # Returns
    /// * `Result<()>`
    pub async fn mark_account_overloaded(&self, account_id: &str) -> Result<()> {
        if self.config.overload_handling_minutes == 0 {
            return Ok(());
        }

        let key = format!("overload:{}", account_id);
        let ttl_seconds = self.config.overload_handling_minutes * 60;
        let mut conn = self.redis.get_connection().await?;

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl_seconds)
            .arg("1")
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| {
                AppError::RedisError(format!("Failed to mark account overloaded: {}", e))
            })?;

        tracing::warn!(
            "🚨 Account {} marked as overloaded for {} minutes",
            account_id,
            self.config.overload_handling_minutes
        );

        Ok(())
    }

    /// 检查账户是否处于过载状态
    ///
    /// # Arguments
    /// * `account_id` - 账户 ID
    ///
    /// # Returns
    /// * `Result<bool>` - 是否过载
    pub async fn is_account_overloaded(&self, account_id: &str) -> Result<bool> {
        let key = format!("overload:{}", account_id);
        let mut conn = self.redis.get_connection().await?;

        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to check overload status: {}", e)))?;

        Ok(exists)
    }

    /// 清除账户过载状态
    ///
    /// # Arguments
    /// * `account_id` - 账户 ID
    ///
    /// # Returns
    /// * `Result<()>`
    pub async fn clear_account_overload(&self, account_id: &str) -> Result<()> {
        let key = format!("overload:{}", account_id);
        let mut conn = self.redis.get_connection().await?;

        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to clear overload status: {}", e)))?;

        tracing::info!("✅ Cleared overload status for account {}", account_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AccountSchedulerConfig::default();
        assert_eq!(config.sticky_session_ttl_hours, 1);
        assert_eq!(config.sticky_session_renewal_threshold_minutes, 0);
        assert!(config.concurrent_limit_enabled);
        assert_eq!(config.overload_handling_minutes, 10);
    }

    #[test]
    fn test_session_mapping_serialization() {
        let mapping = SessionMapping {
            account_id: "test-account-id".to_string(),
            account_type: AccountType::Shared,
            platform: Platform::Claude,
            created_at: 1234567890000,
        };

        let serialized = serde_json::to_string(&mapping).unwrap();
        let deserialized: SessionMapping = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.account_id, "test-account-id");
        assert_eq!(deserialized.created_at, 1234567890000);
    }

    #[test]
    fn test_selected_account_creation() {
        let selected = SelectedAccount {
            account_id: "acc-123".to_string(),
            account_type: AccountType::Dedicated,
            platform: Platform::Claude,
            account_name: "Test Account".to_string(),
            priority: 10,
        };

        assert_eq!(selected.account_id, "acc-123");
        assert_eq!(selected.priority, 10);
        assert_eq!(selected.account_name, "Test Account");
    }

    #[test]
    fn test_config_custom_values() {
        let config = AccountSchedulerConfig {
            sticky_session_ttl_hours: 2,
            sticky_session_renewal_threshold_minutes: 15,
            concurrent_limit_enabled: false,
            overload_handling_minutes: 5,
        };

        assert_eq!(config.sticky_session_ttl_hours, 2);
        assert_eq!(config.sticky_session_renewal_threshold_minutes, 15);
        assert!(!config.concurrent_limit_enabled);
        assert_eq!(config.overload_handling_minutes, 5);
    }

    #[test]
    fn test_session_mapping_json_compatibility() {
        // 测试与 Node.js 版本的 JSON 格式兼容性
        let json = r#"{
            "account_id": "test-id",
            "account_type": "shared",
            "platform": "claude",
            "created_at": 1234567890000
        }"#;

        let mapping: SessionMapping = serde_json::from_str(json).unwrap();
        assert_eq!(mapping.account_id, "test-id");
        assert_eq!(mapping.created_at, 1234567890000);

        // 测试序列化输出格式
        let serialized = serde_json::to_value(&mapping).unwrap();
        assert_eq!(serialized["account_type"], "shared");
        assert_eq!(serialized["platform"], "claude");
    }

    #[test]
    fn test_account_type_variants() {
        let shared_mapping = SessionMapping {
            account_id: "shared-1".to_string(),
            account_type: AccountType::Shared,
            platform: Platform::Claude,
            created_at: 0,
        };

        let dedicated_mapping = SessionMapping {
            account_id: "dedicated-1".to_string(),
            account_type: AccountType::Dedicated,
            platform: Platform::Gemini,
            created_at: 0,
        };

        // 验证序列化后的类型字符串
        let shared_json = serde_json::to_value(&shared_mapping).unwrap();
        let dedicated_json = serde_json::to_value(&dedicated_mapping).unwrap();

        assert_eq!(shared_json["account_type"], "shared");
        assert_eq!(dedicated_json["account_type"], "dedicated");
    }

    #[test]
    fn test_platform_variants() {
        let platforms = vec![
            (Platform::Claude, "claude"),
            (Platform::Gemini, "gemini"),
            (Platform::OpenAI, "openai"),
            (Platform::Bedrock, "bedrock"),
        ];

        for (platform, expected_str) in platforms {
            let mapping = SessionMapping {
                account_id: "test".to_string(),
                account_type: AccountType::Shared,
                platform,
                created_at: 0,
            };

            let json = serde_json::to_value(&mapping).unwrap();
            assert_eq!(json["platform"], expected_str);
        }
    }
}
