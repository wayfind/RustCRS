use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 账户类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AccountType {
    /// 共享账户（多用户共享）
    #[default]
    Shared,
    /// 专用账户（单用户独占）
    Dedicated,
}

/// 账户平台
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Platform {
    /// Claude 官方
    #[default]
    Claude,
    /// Claude Console
    ClaudeConsole,
    /// Gemini
    Gemini,
    /// OpenAI
    OpenAI,
    /// AWS Bedrock
    Bedrock,
    /// Azure OpenAI
    Azure,
    /// Droid (Factory.ai)
    Droid,
    /// CCR
    CCR,
}

/// 账户状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountStatus {
    /// 正常激活状态
    Active,
    /// 已禁用
    Inactive,
    /// 错误状态（需要修复）
    Error,
    /// 过载状态（临时不可用）
    Overloaded,
    /// 过期
    Expired,
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 代理类型 (socks5, http, https)
    #[serde(rename = "type")]
    pub proxy_type: String,
    /// 代理主机
    pub host: String,
    /// 代理端口
    pub port: u16,
    /// 代理用户名（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 代理密码（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Claude OAuth 数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeOAuthData {
    /// 访问令牌
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// 刷新令牌
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    /// 过期时间（Unix 时间戳毫秒）
    #[serde(rename = "expiresAt")]
    pub expires_at: i64,
    /// 权限范围
    pub scopes: Vec<String>,
}

/// 订阅信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    /// 订阅类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription: Option<String>,
    /// 套餐计划
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    /// 层级
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    /// 账户类型
    #[serde(skip_serializing_if = "Option::is_none", rename = "accountType")]
    pub account_type: Option<String>,
    /// 功能列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    /// 限制信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<serde_json::Value>,
}

/// Claude 账户模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeAccount {
    /// 账户唯一标识符
    pub id: Uuid,
    /// 账户名称
    pub name: String,
    /// 账户描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 邮箱（加密存储）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 密码（加密存储）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Claude OAuth 数据（加密存储的 JSON）
    #[serde(skip_serializing_if = "Option::is_none", rename = "claudeAiOauth")]
    pub claude_ai_oauth: Option<String>,
    /// 访问令牌（加密存储）
    #[serde(skip_serializing_if = "Option::is_none", rename = "accessToken")]
    pub access_token: Option<String>,
    /// 刷新令牌（加密存储）
    #[serde(skip_serializing_if = "Option::is_none", rename = "refreshToken")]
    pub refresh_token: Option<String>,
    /// 会话令牌（Claude Console 使用，加密存储）
    #[serde(skip_serializing_if = "Option::is_none", rename = "session_token")]
    pub session_token: Option<String>,
    /// 自定义 API 端点（Claude Console 使用）
    #[serde(skip_serializing_if = "Option::is_none", rename = "custom_api_endpoint")]
    pub custom_api_endpoint: Option<String>,
    /// 令牌过期时间（Unix 时间戳毫秒字符串）
    #[serde(skip_serializing_if = "Option::is_none", rename = "expiresAt")]
    pub expires_at: Option<String>,
    /// OAuth 权限范围（空格分隔）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<String>,
    /// 代理配置（JSON 字符串）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    /// 是否激活
    #[serde(rename = "isActive")]
    pub is_active: bool,
    /// 账户类型
    #[serde(rename = "accountType")]
    pub account_type: AccountType,
    /// 平台标识
    pub platform: Platform,
    /// 调度优先级 (1-100，数字越小优先级越高)
    pub priority: u8,
    /// 是否可被调度
    pub schedulable: bool,
    /// 订阅信息（JSON 字符串）
    #[serde(skip_serializing_if = "Option::is_none", rename = "subscriptionInfo")]
    pub subscription_info: Option<String>,
    /// 5小时使用量警告时自动停止调度
    #[serde(rename = "autoStopOnWarning")]
    pub auto_stop_on_warning: bool,
    /// 使用统一 User-Agent
    #[serde(rename = "useUnifiedUserAgent")]
    pub use_unified_user_agent: bool,
    /// 使用统一客户端标识
    #[serde(rename = "useUnifiedClientId")]
    pub use_unified_client_id: bool,
    /// 统一客户端标识
    #[serde(skip_serializing_if = "Option::is_none", rename = "unifiedClientId")]
    pub unified_client_id: Option<String>,
    /// 账户订阅到期时间
    #[serde(skip_serializing_if = "Option::is_none", rename = "accountExpiresAt")]
    pub account_expires_at: Option<String>,
    /// 扩展信息（JSON 字符串）
    #[serde(skip_serializing_if = "Option::is_none", rename = "extInfo")]
    pub ext_info: Option<String>,
    /// 账户状态
    pub status: AccountStatus,
    /// 错误消息
    #[serde(skip_serializing_if = "Option::is_none", rename = "errorMessage")]
    pub error_message: Option<String>,
    /// 上次刷新时间
    #[serde(skip_serializing_if = "Option::is_none", rename = "lastRefreshAt")]
    pub last_refresh_at: Option<DateTime<Utc>>,
    /// 并发限制
    #[serde(rename = "concurrencyLimit")]
    pub concurrency_limit: u32,
    /// 当前并发数
    #[serde(rename = "currentConcurrency")]
    pub current_concurrency: u32,
    /// 创建时间
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    /// 更新时间
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

impl ClaudeAccount {
    /// 检查 token 是否即将过期
    ///
    /// # Arguments
    ///
    /// * `threshold_seconds` - 提前刷新的秒数阈值（默认 10 秒）
    ///
    /// # Returns
    ///
    /// 如果 token 在阈值时间内过期则返回 true
    pub fn is_token_expiring(&self, threshold_seconds: i64) -> bool {
        if let Some(expires_at_str) = &self.expires_at {
            if let Ok(expires_at) = expires_at_str.parse::<i64>() {
                let now = Utc::now().timestamp_millis();
                let expiry_time = expires_at;
                let threshold_ms = threshold_seconds * 1000;

                return now + threshold_ms >= expiry_time;
            }
        }
        true // 如果无法解析过期时间，假设已过期
    }

    /// 检查账户是否可用于调度
    pub fn is_available_for_scheduling(&self) -> bool {
        self.is_active
            && self.schedulable
            && self.status == AccountStatus::Active
            && self.current_concurrency < self.concurrency_limit
    }

    /// 检查是否有 OAuth 权限范围
    pub fn has_scope(&self, scope: &str) -> bool {
        if let Some(ref scopes_str) = self.scopes {
            scopes_str.split_whitespace().any(|s| s == scope)
        } else {
            false
        }
    }
}

/// 创建 Claude 账户的选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClaudeAccountOptions {
    /// 账户名称
    pub name: String,
    /// 账户描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 邮箱
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 密码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 刷新令牌
    #[serde(skip_serializing_if = "Option::is_none", rename = "refreshToken")]
    pub refresh_token: Option<String>,
    /// Claude OAuth 数据
    #[serde(skip_serializing_if = "Option::is_none", rename = "claudeAiOauth")]
    pub claude_ai_oauth: Option<ClaudeOAuthData>,
    /// 代理配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxyConfig>,
    /// 是否激活
    #[serde(default = "default_true", rename = "isActive")]
    pub is_active: bool,
    /// 账户类型
    #[serde(default, rename = "accountType")]
    pub account_type: AccountType,
    /// 平台标识
    #[serde(default)]
    pub platform: Platform,
    /// 调度优先级
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// 是否可被调度
    #[serde(default = "default_true")]
    pub schedulable: bool,
    /// 订阅信息
    #[serde(skip_serializing_if = "Option::is_none", rename = "subscriptionInfo")]
    pub subscription_info: Option<SubscriptionInfo>,
    /// 自动停止警告
    #[serde(default, rename = "autoStopOnWarning")]
    pub auto_stop_on_warning: bool,
    /// 使用统一 User-Agent
    #[serde(default, rename = "useUnifiedUserAgent")]
    pub use_unified_user_agent: bool,
    /// 使用统一客户端标识
    #[serde(default, rename = "useUnifiedClientId")]
    pub use_unified_client_id: bool,
    /// 统一客户端标识
    #[serde(skip_serializing_if = "Option::is_none", rename = "unifiedClientId")]
    pub unified_client_id: Option<String>,
    /// 账户过期时间
    #[serde(skip_serializing_if = "Option::is_none", rename = "expiresAt")]
    pub expires_at: Option<String>,
    /// 扩展信息
    #[serde(skip_serializing_if = "Option::is_none", rename = "extInfo")]
    pub ext_info: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

fn default_priority() -> u8 {
    50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_token_expiring() {
        let now = Utc::now().timestamp_millis();
        let mut account = ClaudeAccount {
            id: Uuid::new_v4(),
            name: "Test Account".to_string(),
            description: None,
            email: None,
            password: None,
            claude_ai_oauth: None,
            access_token: None,
            refresh_token: None,
            expires_at: Some((now + 5000).to_string()), // 5 秒后过期
            scopes: None,
            proxy: None,
            is_active: true,
            account_type: AccountType::Shared,
            platform: Platform::Claude,
            priority: 50,
            schedulable: true,
            subscription_info: None,
            auto_stop_on_warning: false,
            use_unified_user_agent: false,
            use_unified_client_id: false,
            unified_client_id: None,
            account_expires_at: None,
            ext_info: None,
            status: AccountStatus::Active,
            error_message: None,
            last_refresh_at: None,
            concurrency_limit: 5,
            current_concurrency: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 应该检测到即将过期（10 秒阈值）
        assert!(account.is_token_expiring(10));

        // 不应该检测到即将过期（1 秒阈值）
        assert!(!account.is_token_expiring(1));

        // 测试已过期
        account.expires_at = Some((now - 1000).to_string());
        assert!(account.is_token_expiring(10));
    }

    #[test]
    fn test_is_available_for_scheduling() {
        let account = ClaudeAccount {
            id: Uuid::new_v4(),
            name: "Test Account".to_string(),
            description: None,
            email: None,
            password: None,
            claude_ai_oauth: None,
            access_token: None,
            refresh_token: None,
            expires_at: None,
            scopes: None,
            proxy: None,
            is_active: true,
            account_type: AccountType::Shared,
            platform: Platform::Claude,
            priority: 50,
            schedulable: true,
            subscription_info: None,
            auto_stop_on_warning: false,
            use_unified_user_agent: false,
            use_unified_client_id: false,
            unified_client_id: None,
            account_expires_at: None,
            ext_info: None,
            status: AccountStatus::Active,
            error_message: None,
            last_refresh_at: None,
            concurrency_limit: 5,
            current_concurrency: 2,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 正常情况应该可以调度
        assert!(account.is_available_for_scheduling());

        // 并发已满
        let mut account_full = account.clone();
        account_full.current_concurrency = 5;
        assert!(!account_full.is_available_for_scheduling());

        // 账户未激活
        let mut account_inactive = account.clone();
        account_inactive.is_active = false;
        assert!(!account_inactive.is_available_for_scheduling());
    }

    #[test]
    fn test_has_scope() {
        let account = ClaudeAccount {
            id: Uuid::new_v4(),
            name: "Test Account".to_string(),
            description: None,
            email: None,
            password: None,
            claude_ai_oauth: None,
            access_token: None,
            refresh_token: None,
            expires_at: None,
            scopes: Some("user:profile claude:conversations".to_string()),
            proxy: None,
            is_active: true,
            account_type: AccountType::Shared,
            platform: Platform::Claude,
            priority: 50,
            schedulable: true,
            subscription_info: None,
            auto_stop_on_warning: false,
            use_unified_user_agent: false,
            use_unified_client_id: false,
            unified_client_id: None,
            account_expires_at: None,
            ext_info: None,
            status: AccountStatus::Active,
            error_message: None,
            last_refresh_at: None,
            concurrency_limit: 5,
            current_concurrency: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(account.has_scope("user:profile"));
        assert!(account.has_scope("claude:conversations"));
        assert!(!account.has_scope("admin:write"));
    }
}
