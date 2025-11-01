// Unified Claude Scheduler - 统一 Claude 账户调度器
//
// 功能：
// 1. 多账户类型支持（claude-official, claude-console, bedrock, ccr）
// 2. 粘性会话管理（session hash → account binding）
// 3. 模型兼容性检查（不同规则 per 账户类型）
// 4. 优先级排序和智能选择
// 5. 并发请求跟踪
// 6. Rate limit 处理
// 7. 错误处理和账户标记

use crate::models::{ClaudeAccount, Platform};
use crate::services::account::ClaudeAccountService;
use crate::services::account_scheduler::AccountScheduler;
use crate::utils::model_helper::{
    is_claude_official_model, is_opus_model, parse_vendor_prefixed_model, ParsedModel,
};
use crate::utils::AppError;
use crate::RedisPool;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// 统一调度器账户变体类型（区分 Claude 官方/Console/Bedrock/CCR）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchedulerAccountVariant {
    ClaudeOfficial,
    ClaudeConsole,
    Bedrock,
    Ccr,
}

impl SchedulerAccountVariant {
    pub fn as_str(&self) -> &str {
        match self {
            SchedulerAccountVariant::ClaudeOfficial => "claude-official",
            SchedulerAccountVariant::ClaudeConsole => "claude-console",
            SchedulerAccountVariant::Bedrock => "bedrock",
            SchedulerAccountVariant::Ccr => "ccr",
        }
    }

    pub fn from_platform(platform: Platform) -> Self {
        match platform {
            Platform::Claude => SchedulerAccountVariant::ClaudeOfficial,
            Platform::ClaudeConsole => SchedulerAccountVariant::ClaudeConsole,
            Platform::Bedrock => SchedulerAccountVariant::Bedrock,
            Platform::CCR => SchedulerAccountVariant::Ccr,
            _ => SchedulerAccountVariant::ClaudeOfficial, // 默认
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "claude-official" => Some(SchedulerAccountVariant::ClaudeOfficial),
            "claude-console" => Some(SchedulerAccountVariant::ClaudeConsole),
            "bedrock" => Some(SchedulerAccountVariant::Bedrock),
            "ccr" => Some(SchedulerAccountVariant::Ccr),
            _ => None,
        }
    }
}

/// 账户选择结果
#[derive(Debug, Clone)]
pub struct SelectedAccount {
    pub account_id: String,
    pub account_variant: SchedulerAccountVariant,
    pub account: ClaudeAccount,
}

/// 会话映射数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMapping {
    account_id: String,
    account_variant: String,
    created_at: i64,
    expires_at: i64,
}

/// 统一 Claude 调度器
pub struct UnifiedClaudeScheduler {
    account_service: Arc<ClaudeAccountService>,
    account_scheduler: Arc<AccountScheduler>,
    redis: Arc<RedisPool>,
    session_mapping_prefix: String,
    sticky_session_ttl_seconds: i64,
    rate_limit_prefix: String,
    rate_limit_ttl_seconds: i64,
}

impl UnifiedClaudeScheduler {
    /// 创建新的统一调度器实例
    pub fn new(
        account_service: Arc<ClaudeAccountService>,
        account_scheduler: Arc<AccountScheduler>,
        redis: Arc<RedisPool>,
    ) -> Self {
        Self {
            account_service,
            account_scheduler,
            redis,
            session_mapping_prefix: "sticky_session:".to_string(),
            sticky_session_ttl_seconds: 3600, // 1 hour default
            rate_limit_prefix: "rate_limit:scheduler:".to_string(),
            rate_limit_ttl_seconds: 300, // 5 minutes default
        }
    }

    /// 为 API Key 选择账户
    ///
    /// # Arguments
    /// * `session_hash` - 会话哈希（用于粘性会话）
    /// * `requested_model` - 请求的模型名称
    pub async fn select_account(
        &self,
        session_hash: Option<&str>,
        requested_model: Option<&str>,
    ) -> Result<SelectedAccount, AppError> {
        // 1. 解析 vendor 前缀
        let parsed = requested_model
            .map(parse_vendor_prefixed_model)
            .unwrap_or(ParsedModel {
                vendor: None,
                base_model: "claude-3-5-sonnet-20241022".to_string(),
                original: "claude-3-5-sonnet-20241022".to_string(),
            });

        let effective_model = requested_model.or(Some(&parsed.base_model));

        debug!(
            "Selecting account for model: {:?}, session_hash: {:?}",
            effective_model, session_hash
        );

        // 2. 检查粘性会话
        if let Some(hash) = session_hash {
            if let Some(mapping) = self.get_session_mapping(hash).await? {
                debug!(
                    "Found sticky session mapping: account_id={}, variant={}",
                    mapping.account_id, mapping.account_variant
                );

                // 尝试获取映射的账户
                if let Ok(Some(account)) =
                    self.account_service.get_account(&mapping.account_id).await
                {
                    if account.is_active && account.schedulable {
                        debug!("Using sticky session account: {}", account.name);

                        if let Some(variant) =
                            SchedulerAccountVariant::from_str(&mapping.account_variant)
                        {
                            return Ok(SelectedAccount {
                                account_id: account.id.to_string(),
                                account_variant: variant,
                                account,
                            });
                        }
                    }
                } else {
                    // 账户不可用，删除映射
                    warn!(
                        "Sticky session account {} unavailable, removing mapping",
                        mapping.account_id
                    );
                    let _ = self.delete_session_mapping(hash).await;
                }
            }
        }

        // 3. 选择新账户
        let selected = self.select_new_account(effective_model, &parsed).await?;

        // 4. 创建粘性会话映射
        if let Some(hash) = session_hash {
            if let Err(e) = self
                .set_session_mapping(
                    hash,
                    &selected.account_id,
                    selected.account_variant.as_str(),
                )
                .await
            {
                warn!("Failed to set sticky session mapping: {}", e);
            }
        }

        Ok(selected)
    }

    /// 选择新账户（不使用粘性会话）
    async fn select_new_account(
        &self,
        requested_model: Option<&str>,
        parsed: &ParsedModel,
    ) -> Result<SelectedAccount, AppError> {
        // 获取所有可用账户
        let all_accounts = self.get_all_available_accounts().await?;

        if all_accounts.is_empty() {
            return Err(AppError::NoAvailableAccounts(
                "No Claude accounts available".to_string(),
            ));
        }

        // 优先级顺序：Official > Console > Bedrock > CCR
        let priority_order = vec![
            SchedulerAccountVariant::ClaudeOfficial,
            SchedulerAccountVariant::ClaudeConsole,
            SchedulerAccountVariant::Bedrock,
            SchedulerAccountVariant::Ccr,
        ];

        // 如果有 CCR vendor 前缀，优先选择 CCR 账户
        if let Some("ccr") = parsed.vendor.as_deref() {
            let mut ccr_candidates = self.find_accounts_by_variant(
                &all_accounts,
                &SchedulerAccountVariant::Ccr,
                requested_model,
            );
            ccr_candidates.sort_by_key(|account| account.priority);

            for account in ccr_candidates {
                if self
                    .is_account_available_for_scheduling(&account, None)
                    .await?
                {
                    return Ok(SelectedAccount {
                        account_id: account.id.to_string(),
                        account_variant: SchedulerAccountVariant::Ccr,
                        account,
                    });
                }
            }
        }

        // 按优先级顺序查找
        for variant in priority_order {
            // 找到匹配变体和模型的账户
            let mut candidates =
                self.find_accounts_by_variant(&all_accounts, &variant, requested_model);

            // 按 priority 排序（数字越小优先级越高）
            candidates.sort_by_key(|account| account.priority);

            // 异步检查每个候选账户的可用性（rate limit + concurrency）
            for account in candidates {
                // 检查 rate limit 和并发限制
                // TODO: 从配置或账户信息获取 max_concurrent，这里暂时不检查并发
                if self
                    .is_account_available_for_scheduling(&account, None)
                    .await?
                {
                    info!(
                        "Selected account: {} (variant: {:?}, priority: {})",
                        account.name, variant, account.priority
                    );
                    return Ok(SelectedAccount {
                        account_id: account.id.to_string(),
                        account_variant: variant,
                        account,
                    });
                } else {
                    debug!(
                        "Account {} unavailable (rate limited or concurrency exceeded)",
                        account.name
                    );
                }
            }
        }

        Err(AppError::NoAvailableAccounts(format!(
            "No suitable account for model: {:?}",
            requested_model
        )))
    }

    /// 从账户列表中查找指定变体的所有匹配账户
    fn find_accounts_by_variant(
        &self,
        accounts: &[ClaudeAccount],
        variant: &SchedulerAccountVariant,
        requested_model: Option<&str>,
    ) -> Vec<ClaudeAccount> {
        accounts
            .iter()
            .filter(|account| {
                let account_variant = SchedulerAccountVariant::from_platform(account.platform);
                account_variant == *variant
            })
            .filter(|account| {
                requested_model
                    .map(|model| self.is_model_supported_by_account(account, variant, model))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// 获取所有可用的 Claude 账户
    async fn get_all_available_accounts(&self) -> Result<Vec<ClaudeAccount>, AppError> {
        // 使用 list_accounts 获取所有账户（offset=0, limit=1000 应该足够）
        let all_accounts = self.account_service.list_accounts(0, 1000).await?;

        Ok(all_accounts
            .into_iter()
            .filter(|account| account.is_active && account.schedulable)
            .collect())
    }

    /// 检查模型是否被账户支持
    fn is_model_supported_by_account(
        &self,
        account: &ClaudeAccount,
        variant: &SchedulerAccountVariant,
        requested_model: &str,
    ) -> bool {
        match variant {
            SchedulerAccountVariant::ClaudeOfficial => {
                // 只支持 Claude 官方模型
                if !is_claude_official_model(requested_model) {
                    return false;
                }

                // Opus 需要 Max 订阅
                if is_opus_model(requested_model) {
                    if let Some(ref sub_info) = account.subscription_info {
                        if let Ok(info) = serde_json::from_str::<serde_json::Value>(sub_info) {
                            // Pro 用户没有 Max，不支持 Opus
                            if info.get("hasClaudePro") == Some(&serde_json::Value::Bool(true))
                                && info.get("hasClaudeMax") != Some(&serde_json::Value::Bool(true))
                            {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            SchedulerAccountVariant::ClaudeConsole | SchedulerAccountVariant::Ccr => {
                // Console 和 CCR 账户支持所有 Claude 模型
                // TODO: 后续可以从 ext_info 中解析 supportedModels
                true
            }
            SchedulerAccountVariant::Bedrock => {
                // Bedrock 支持所有 Claude 模型
                true
            }
        }
    }

    /// 获取会话映射
    async fn get_session_mapping(
        &self,
        session_hash: &str,
    ) -> Result<Option<SessionMapping>, AppError> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mut conn = self.redis.get_connection().await?;
        let data: Option<String> = conn.get(&key).await?;

        if let Some(json) = data {
            let mapping: SessionMapping = serde_json::from_str(&json)?;
            let now = chrono::Utc::now().timestamp();

            if mapping.expires_at > now {
                Ok(Some(mapping))
            } else {
                // 过期，删除
                let _: () = conn.del(&key).await?;
                Ok(None)
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
        account_variant: &str,
    ) -> Result<(), AppError> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let now = chrono::Utc::now().timestamp();

        let mapping = SessionMapping {
            account_id: account_id.to_string(),
            account_variant: account_variant.to_string(),
            created_at: now,
            expires_at: now + self.sticky_session_ttl_seconds,
        };

        let json = serde_json::to_string(&mapping)?;
        let mut conn = self.redis.get_connection().await?;

        conn.set_ex::<_, _, ()>(&key, json, self.sticky_session_ttl_seconds as u64)
            .await?;

        Ok(())
    }

    /// 删除会话映射
    async fn delete_session_mapping(&self, session_hash: &str) -> Result<(), AppError> {
        let key = format!("{}{}", self.session_mapping_prefix, session_hash);
        let mut conn = self.redis.get_connection().await?;
        let _: () = conn.del(&key).await?;
        Ok(())
    }

    // ============================================================================
    // Rate Limiting 和并发控制
    // ============================================================================

    /// 检查账户是否被 rate limited
    pub async fn is_account_rate_limited(&self, account_id: &str) -> Result<bool, AppError> {
        let key = format!("{}{}", self.rate_limit_prefix, account_id);
        let mut conn = self.redis.get_connection().await?;
        let exists: bool = conn.exists(&key).await?;
        Ok(exists)
    }

    /// 标记账户为 rate limited
    pub async fn mark_account_rate_limited(
        &self,
        account_id: &str,
        duration_seconds: Option<i64>,
    ) -> Result<(), AppError> {
        let key = format!("{}{}", self.rate_limit_prefix, account_id);
        let ttl = duration_seconds.unwrap_or(self.rate_limit_ttl_seconds);
        let mut conn = self.redis.get_connection().await?;

        info!(
            "Marking account {} as rate limited for {} seconds",
            account_id, ttl
        );

        conn.set_ex::<_, _, ()>(&key, "1", ttl as u64).await?;
        Ok(())
    }

    /// 移除账户的 rate limit 标记
    pub async fn remove_account_rate_limit(&self, account_id: &str) -> Result<(), AppError> {
        let key = format!("{}{}", self.rate_limit_prefix, account_id);
        let mut conn = self.redis.get_connection().await?;
        let _: () = conn.del(&key).await?;

        debug!("Removed rate limit for account {}", account_id);
        Ok(())
    }

    /// 增加账户并发计数
    pub async fn increment_account_concurrency(
        &self,
        account_id: &str,
        request_id: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<(), AppError> {
        self.account_scheduler
            .increment_concurrency(account_id, request_id, ttl_seconds)
            .await
    }

    /// 减少账户并发计数
    pub async fn decrement_account_concurrency(
        &self,
        account_id: &str,
        request_id: &str,
    ) -> Result<(), AppError> {
        self.account_scheduler
            .decrement_concurrency(account_id, request_id)
            .await
    }

    /// 获取账户当前并发数
    pub async fn get_account_concurrency(&self, account_id: &str) -> Result<usize, AppError> {
        self.account_scheduler
            .get_account_concurrency(account_id)
            .await
    }

    /// 检查账户是否超过并发限制
    pub async fn is_account_concurrency_exceeded(
        &self,
        account_id: &str,
        max_concurrent: usize,
    ) -> Result<bool, AppError> {
        let current = self.get_account_concurrency(account_id).await?;
        Ok(current >= max_concurrent)
    }

    /// 检查账户是否可调度（综合检查：active, schedulable, rate limit, concurrency）
    pub async fn is_account_available_for_scheduling(
        &self,
        account: &ClaudeAccount,
        max_concurrent: Option<usize>,
    ) -> Result<bool, AppError> {
        // 1. 基础状态检查
        if !account.is_active || !account.schedulable {
            return Ok(false);
        }

        // 2. Rate limit 检查
        if self
            .is_account_rate_limited(&account.id.to_string())
            .await?
        {
            debug!("Account {} is rate limited", account.name);
            return Ok(false);
        }

        // 3. 并发限制检查（如果提供）
        if let Some(max) = max_concurrent {
            if self
                .is_account_concurrency_exceeded(&account.id.to_string(), max)
                .await?
            {
                debug!(
                    "Account {} exceeded concurrency limit: {}",
                    account.name, max
                );
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// 处理请求开始（增加并发计数）
    pub async fn on_request_start(
        &self,
        account_id: &str,
        request_id: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<(), AppError> {
        self.increment_account_concurrency(account_id, request_id, ttl_seconds)
            .await?;
        let count = self.get_account_concurrency(account_id).await?;
        debug!("Account {} concurrency increased to {}", account_id, count);
        Ok(())
    }

    /// 处理请求结束（减少并发计数）
    pub async fn on_request_end(&self, account_id: &str, request_id: &str) -> Result<(), AppError> {
        self.decrement_account_concurrency(account_id, request_id)
            .await?;
        let count = self.get_account_concurrency(account_id).await?;
        debug!("Account {} concurrency decreased to {}", account_id, count);
        Ok(())
    }

    /// 处理 429 Rate Limit 错误
    pub async fn on_rate_limit_error(
        &self,
        account_id: &str,
        duration_seconds: Option<i64>,
    ) -> Result<(), AppError> {
        warn!(
            "Account {} hit rate limit, marking temporarily unavailable",
            account_id
        );
        self.mark_account_rate_limited(account_id, duration_seconds)
            .await
    }

    /// 处理请求成功（可选：重置 rate limit）
    pub async fn on_request_success(&self, account_id: &str) -> Result<(), AppError> {
        // 如果之前被 rate limited，现在成功了，可以移除标记
        if self.is_account_rate_limited(account_id).await? {
            debug!("Request succeeded, removing rate limit for {}", account_id);
            self.remove_account_rate_limit(account_id).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_account_variant_conversion() {
        assert_eq!(
            SchedulerAccountVariant::ClaudeOfficial.as_str(),
            "claude-official"
        );
        assert_eq!(
            SchedulerAccountVariant::ClaudeConsole.as_str(),
            "claude-console"
        );
        assert_eq!(SchedulerAccountVariant::Bedrock.as_str(), "bedrock");
        assert_eq!(SchedulerAccountVariant::Ccr.as_str(), "ccr");

        assert_eq!(
            SchedulerAccountVariant::from_str("claude-official"),
            Some(SchedulerAccountVariant::ClaudeOfficial)
        );
        assert_eq!(SchedulerAccountVariant::from_str("invalid"), None);
    }

    #[test]
    fn test_from_platform() {
        assert_eq!(
            SchedulerAccountVariant::from_platform(Platform::Claude),
            SchedulerAccountVariant::ClaudeOfficial
        );
        assert_eq!(
            SchedulerAccountVariant::from_platform(Platform::ClaudeConsole),
            SchedulerAccountVariant::ClaudeConsole
        );
        assert_eq!(
            SchedulerAccountVariant::from_platform(Platform::Bedrock),
            SchedulerAccountVariant::Bedrock
        );
        assert_eq!(
            SchedulerAccountVariant::from_platform(Platform::CCR),
            SchedulerAccountVariant::Ccr
        );
    }
}
