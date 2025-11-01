// Unified Gemini Scheduler
//
// 智能 Gemini 多账户调度器，支持：
// - 单一账户类型（Gemini）
// - 粘性会话管理
// - 优先级排序
// - 模型支持检查
// - 速率限制集成
// - 并发控制

use crate::models::{ApiKey, ClaudeAccount, Platform};
use crate::redis::RedisPool;
use crate::services::account::ClaudeAccountService;
use crate::services::account_scheduler::AccountScheduler;
use crate::utils::error::{AppError, Result};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Gemini 会话映射数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMapping {
    pub account_id: String,
    pub account_type: String,
}

/// 选中的 Gemini 账户
#[derive(Debug, Clone)]
pub struct SelectedAccount {
    pub account_id: String,
    pub account: ClaudeAccount,
}

/// Unified Gemini Scheduler
pub struct UnifiedGeminiScheduler {
    account_service: Arc<ClaudeAccountService>,
    account_scheduler: Arc<AccountScheduler>,
    redis: Arc<RedisPool>,
    session_mapping_prefix: String,
    sticky_session_ttl_seconds: i64,
    rate_limit_ttl_seconds: i64,
}

impl UnifiedGeminiScheduler {
    /// 创建新的 Gemini 调度器实例
    pub fn new(
        account_service: Arc<ClaudeAccountService>,
        account_scheduler: Arc<AccountScheduler>,
        redis: Arc<RedisPool>,
        sticky_session_ttl_hours: Option<i64>,
    ) -> Self {
        let ttl_hours = sticky_session_ttl_hours.unwrap_or(1);
        Self {
            account_service,
            account_scheduler,
            redis,
            session_mapping_prefix: "unified_gemini_session_mapping:".to_string(),
            sticky_session_ttl_seconds: ttl_hours * 3600,
            rate_limit_ttl_seconds: 300, // 5 minutes default
        }
    }

    /// 为 API Key 选择 Gemini 账户
    ///
    /// # Arguments
    /// * `api_key` - API Key 数据（用于专属账户绑定）
    /// * `session_hash` - 可选的会话哈希（用于粘性会话）
    /// * `requested_model` - 可选的请求模型（用于模型支持检查）
    ///
    /// # Returns
    /// 返回选中的账户信息
    pub async fn select_account(
        &self,
        api_key: &ApiKey,
        session_hash: Option<&str>,
        requested_model: Option<&str>,
    ) -> Result<SelectedAccount> {
        // 1. 检查 API Key 是否绑定了专属 Gemini 账户
        if let Some(ref gemini_account_id) = api_key.gemini_account_id {
            if let Some(account) = self.get_bound_account(gemini_account_id).await? {
                info!(
                    "🎯 Using bound dedicated Gemini account: {} ({}) for API key {}",
                    account.name, gemini_account_id, api_key.name
                );
                return Ok(SelectedAccount {
                    account_id: gemini_account_id.clone(),
                    account,
                });
            } else {
                warn!(
                    "⚠️ Bound Gemini account {} is not available, falling back to pool",
                    gemini_account_id
                );
            }
        }

        // 2. 检查粘性会话
        if let Some(hash) = session_hash {
            if let Some(mapping) = self.get_session_mapping(hash).await? {
                if let Some(account) = self.get_account_if_available(&mapping.account_id).await? {
                    // 续期会话
                    self.extend_session_mapping_ttl(hash).await?;
                    info!(
                        "🎯 Using sticky session account: {} for session {}",
                        mapping.account_id, hash
                    );
                    return Ok(SelectedAccount {
                        account_id: mapping.account_id.clone(),
                        account,
                    });
                } else {
                    warn!(
                        "⚠️ Mapped account {} is no longer available, selecting new account",
                        mapping.account_id
                    );
                    self.delete_session_mapping(hash).await?;
                }
            }
        }

        // 3. 选择新账户
        let selected = self.select_new_account(requested_model).await?;

        // 4. 创建粘性会话映射
        if let Some(hash) = session_hash {
            self.set_session_mapping(hash, &selected.account_id, "gemini")
                .await?;
            info!(
                "🎯 Created new sticky session mapping: {} for session {}",
                selected.account_id, hash
            );
        }

        Ok(selected)
    }

    /// 选择新的 Gemini 账户
    async fn select_new_account(&self, requested_model: Option<&str>) -> Result<SelectedAccount> {
        let all_accounts = self.get_all_available_accounts().await?;
        let mut candidates: Vec<ClaudeAccount> = all_accounts
            .into_iter()
            .filter(|account| self.is_model_supported(account, requested_model))
            .collect();

        if candidates.is_empty() {
            return Err(AppError::NoAvailableAccounts(
                if let Some(model) = requested_model {
                    format!("No Gemini accounts support model: {}", model)
                } else {
                    "No available Gemini accounts".to_string()
                },
            ));
        }

        // 按优先级和最后刷新时间排序
        candidates.sort_by(|a, b| {
            // 优先级排序（数字越小优先级越高）
            match a.priority.cmp(&b.priority) {
                std::cmp::Ordering::Equal => {
                    // 优先级相同时，按最后刷新时间排序（最久未使用的优先）
                    // None < Some, 所以没有刷新过的账户优先级更高
                    a.last_refresh_at.cmp(&b.last_refresh_at)
                }
                other => other,
            }
        });

        // Async 检查第一个可用的账户
        for account in candidates {
            if self.is_account_available_for_scheduling(&account).await? {
                let account_id = account.id.to_string();
                info!(
                    "🎯 Selected Gemini account: {} ({}) with priority {}",
                    account.name, account_id, account.priority
                );
                return Ok(SelectedAccount {
                    account_id,
                    account,
                });
            }
        }

        Err(AppError::NoAvailableAccounts(
            "All Gemini accounts are currently unavailable".to_string(),
        ))
    }

    /// 获取所有可用的 Gemini 账户
    async fn get_all_available_accounts(&self) -> Result<Vec<ClaudeAccount>> {
        let all_accounts = self.account_service.list_accounts(0, 1000).await?;

        // 过滤出 Gemini 平台的账户
        let available: Vec<ClaudeAccount> = all_accounts
            .into_iter()
            .filter(|account| {
                // 必须是 Gemini 平台 && 基本状态检查
                account.platform == Platform::Gemini
                    && account.is_active
                    && matches!(account.status, crate::models::AccountStatus::Active)
                    && account.schedulable
            })
            .collect();

        info!("📊 Total available Gemini accounts: {}", available.len());
        Ok(available)
    }

    /// 获取绑定的专属账户（如果可用）
    async fn get_bound_account(&self, account_id: &str) -> Result<Option<ClaudeAccount>> {
        if let Some(account) = self.account_service.get_account(account_id).await? {
            if account.platform == Platform::Gemini
                && account.is_active
                && matches!(account.status, crate::models::AccountStatus::Active)
                && !self.is_account_rate_limited(account_id).await?
            {
                return Ok(Some(account));
            }
        }
        Ok(None)
    }

    /// 获取账户（如果可用）
    async fn get_account_if_available(&self, account_id: &str) -> Result<Option<ClaudeAccount>> {
        if let Some(account) = self.account_service.get_account(account_id).await? {
            if account.platform == Platform::Gemini
                && self.is_account_available_for_scheduling(&account).await?
            {
                return Ok(Some(account));
            }
        }
        Ok(None)
    }

    /// 检查账户是否可调度（rate limit + 基本状态）
    async fn is_account_available_for_scheduling(&self, account: &ClaudeAccount) -> Result<bool> {
        // 1. 基本状态检查
        if !account.is_active
            || !matches!(account.status, crate::models::AccountStatus::Active)
            || !account.schedulable
        {
            return Ok(false);
        }

        // 2. Rate limit 检查
        if self
            .is_account_rate_limited(&account.id.to_string())
            .await?
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// 检查模型是否被账户支持
    fn is_model_supported(&self, _account: &ClaudeAccount, requested_model: Option<&str>) -> bool {
        if let Some(_model) = requested_model {
            // TODO: 从 ext_info 中解析 supportedModels
            // Gemini 账户的 supportedModels 可能存储在 ext_info JSON 中
            // 暂时假设所有 Gemini 账户支持所有 Gemini 模型
        }
        // 如果没有指定模型或账户没有限制，则支持
        true
    }

    // ============================================================================
    // Rate Limiting Methods
    // ============================================================================

    /// 检查账户是否被限流
    ///
    /// Gemini 账户的 rate_limit_status 和 rate_limited_at 存储在 ext_info JSON 中
    pub async fn is_account_rate_limited(&self, account_id: &str) -> Result<bool> {
        if let Some(account) = self.account_service.get_account(account_id).await? {
            if account.platform != Platform::Gemini {
                return Ok(false);
            }

            // TODO: 从 ext_info JSON 中解析 rateLimitStatus 和 rateLimitedAt
            // 目前简化处理：检查 status 是否为 Overloaded
            if matches!(account.status, crate::models::AccountStatus::Overloaded) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 标记账户为限流状态
    pub async fn mark_account_rate_limited(
        &self,
        account_id: &str,
        session_hash: Option<&str>,
    ) -> Result<()> {
        let key = format!("rate_limit:{}", account_id);
        let ttl = self.rate_limit_ttl_seconds;
        let mut conn = self.redis.get_connection().await?;

        info!(
            "Marking Gemini account {} as rate limited for {} seconds",
            account_id, ttl
        );

        conn.set_ex::<_, _, ()>(&key, "1", ttl as u64).await?;

        // 删除会话映射
        if let Some(hash) = session_hash {
            self.delete_session_mapping(hash).await?;
        }

        Ok(())
    }

    /// 移除账户的限流状态
    pub async fn remove_account_rate_limit(&self, account_id: &str) -> Result<()> {
        // TODO: 更新 ext_info JSON 移除 rateLimitStatus
        warn!(
            "Removing rate limit for Gemini account {} (not implemented)",
            account_id
        );
        Ok(())
    }

    /// 处理 rate limit 错误
    pub async fn on_rate_limit_error(
        &self,
        account_id: &str,
        session_hash: Option<&str>,
    ) -> Result<()> {
        warn!(
            "Account {} hit rate limit, marking temporarily unavailable",
            account_id
        );
        self.mark_account_rate_limited(account_id, session_hash)
            .await
    }

    // ============================================================================
    // Sticky Session Management
    // ============================================================================

    /// 获取会话映射
    async fn get_session_mapping(&self, session_hash: &str) -> Result<Option<SessionMapping>> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mut conn = self.redis.get_connection().await?;

        if let Some(json) = conn.get::<_, Option<String>>(&key).await? {
            match serde_json::from_str::<SessionMapping>(&json) {
                Ok(mapping) => Ok(Some(mapping)),
                Err(e) => {
                    warn!("⚠️ Failed to parse session mapping: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// 设置会话映射
    async fn set_session_mapping(
        &self,
        session_hash: &str,
        account_id: &str,
        account_type: &str,
    ) -> Result<()> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mapping = SessionMapping {
            account_id: account_id.to_string(),
            account_type: account_type.to_string(),
        };
        let json = serde_json::to_string(&mapping)?;
        let mut conn = self.redis.get_connection().await?;
        conn.set_ex::<_, _, ()>(&key, json, self.sticky_session_ttl_seconds as u64)
            .await?;
        Ok(())
    }

    /// 删除会话映射
    async fn delete_session_mapping(&self, session_hash: &str) -> Result<()> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mut conn = self.redis.get_connection().await?;
        conn.del::<_, ()>(&key).await?;
        Ok(())
    }

    /// 续期会话映射 TTL
    async fn extend_session_mapping_ttl(&self, session_hash: &str) -> Result<bool> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mut conn = self.redis.get_connection().await?;

        let remaining_ttl: i64 = conn.ttl(&key).await?;

        // -2 表示键不存在，-1 表示没有过期时间
        if remaining_ttl == -2 {
            return Ok(false);
        }
        if remaining_ttl == -1 {
            return Ok(true);
        }

        // TODO: 从配置读取 renewalThresholdMinutes
        let renewal_threshold_seconds = 0; // 0 表示禁用续期

        if renewal_threshold_seconds > 0 && remaining_ttl < renewal_threshold_seconds {
            conn.expire::<_, ()>(&key, self.sticky_session_ttl_seconds)
                .await?;
            info!(
                "🔄 Renewed Gemini session TTL: {} (was {}s, renewed to {}s)",
                session_hash, remaining_ttl, self.sticky_session_ttl_seconds
            );
        }

        Ok(true)
    }

    // ============================================================================
    // Concurrency Control (Delegated to AccountScheduler)
    // ============================================================================

    /// 增加账户并发计数
    pub async fn increment_account_concurrency(
        &self,
        account_id: &str,
        request_id: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<()> {
        self.account_scheduler
            .increment_concurrency(account_id, request_id, ttl_seconds)
            .await
    }

    /// 减少账户并发计数
    pub async fn decrement_account_concurrency(
        &self,
        account_id: &str,
        request_id: &str,
    ) -> Result<()> {
        self.account_scheduler
            .decrement_concurrency(account_id, request_id)
            .await
    }

    /// 获取账户当前并发数
    pub async fn get_account_concurrency(&self, account_id: &str) -> Result<usize> {
        self.account_scheduler
            .get_account_concurrency(account_id)
            .await
    }

    /// 检查账户并发是否超限
    pub async fn is_account_concurrency_exceeded(
        &self,
        account_id: &str,
        max_concurrent: usize,
    ) -> Result<bool> {
        let current = self.get_account_concurrency(account_id).await?;
        Ok(current >= max_concurrent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_mapping_serde() {
        let mapping = SessionMapping {
            account_id: "test-123".to_string(),
            account_type: "gemini".to_string(),
        };

        let json = serde_json::to_string(&mapping).unwrap();
        let deserialized: SessionMapping = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.account_id, "test-123");
        assert_eq!(deserialized.account_type, "gemini");
    }
}
