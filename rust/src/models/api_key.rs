use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API Key 权限类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyPermissions {
    All,
    Claude,
    Gemini,
    OpenAI,
    Droid,
}

impl ApiKeyPermissions {
    pub fn can_access_claude(&self) -> bool {
        matches!(self, Self::All | Self::Claude)
    }

    pub fn can_access_gemini(&self) -> bool {
        matches!(self, Self::All | Self::Gemini)
    }

    pub fn can_access_openai(&self) -> bool {
        matches!(self, Self::All | Self::OpenAI)
    }

    pub fn can_access_droid(&self) -> bool {
        matches!(self, Self::All | Self::Droid)
    }
}

impl Default for ApiKeyPermissions {
    fn default() -> Self {
        Self::All
    }
}

/// 过期模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExpirationMode {
    /// 固定时间过期
    Fixed,
    /// 首次使用后激活计时
    Activation,
}

impl Default for ExpirationMode {
    fn default() -> Self {
        Self::Fixed
    }
}

/// 激活时间单位
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActivationUnit {
    Hours,
    Days,
}

impl Default for ActivationUnit {
    fn default() -> Self {
        Self::Days
    }
}

/// API Key 完整数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// API Key ID (UUID)
    pub id: String,

    /// API Key 值 (哈希存储)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// API Key 哈希值 (用于验证)
    pub key_hash: String,

    /// 密钥名称
    pub name: String,

    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 图标 (base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// 创建时间
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    /// 更新时间
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    /// 过期时间
    #[serde(skip_serializing_if = "Option::is_none", rename = "expiresAt")]
    pub expires_at: Option<DateTime<Utc>>,

    /// 激活时间 (首次使用时间)
    #[serde(skip_serializing_if = "Option::is_none", rename = "activatedAt")]
    pub activated_at: Option<DateTime<Utc>>,

    /// 最后使用时间
    #[serde(skip_serializing_if = "Option::is_none", rename = "lastUsedAt")]
    pub last_used_at: Option<DateTime<Utc>>,

    /// 是否激活
    #[serde(rename = "isActive")]
    pub is_active: bool,

    /// 是否已删除
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,

    /// 删除时间
    #[serde(skip_serializing_if = "Option::is_none", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,

    /// 删除者
    #[serde(skip_serializing_if = "Option::is_none", rename = "deletedBy")]
    pub deleted_by: Option<String>,

    /// 删除者类型
    #[serde(skip_serializing_if = "Option::is_none", rename = "deletedByType")]
    pub deleted_by_type: Option<String>,

    /// 权限
    pub permissions: ApiKeyPermissions,

    /// Token 限制 (已废弃,保留兼容性)
    pub token_limit: i64,

    /// 并发限制
    pub concurrency_limit: i64,

    /// 速率限制窗口 (秒)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_window: Option<i64>,

    /// 速率限制请求数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_requests: Option<i64>,

    /// 速率限制成本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_cost: Option<f64>,

    /// 每日成本限制
    pub daily_cost_limit: f64,

    /// 总成本限制
    pub total_cost_limit: f64,

    /// 每周 Opus 成本限制
    pub weekly_opus_cost_limit: f64,

    /// 启用模型限制
    pub enable_model_restriction: bool,

    /// 受限模型列表
    pub restricted_models: Vec<String>,

    /// 启用客户端限制
    pub enable_client_restriction: bool,

    /// 允许的客户端列表
    pub allowed_clients: Vec<String>,

    /// 标签
    pub tags: Vec<String>,

    /// 过期模式
    pub expiration_mode: ExpirationMode,

    /// 激活后有效天数
    pub activation_days: i64,

    /// 激活时间单位
    pub activation_unit: ActivationUnit,

    /// 绑定的账户 ID
    #[serde(rename = "claudeAccountId")]
    pub claude_account_id: Option<String>,
    #[serde(rename = "claudeConsoleAccountId")]
    pub claude_console_account_id: Option<String>,
    #[serde(rename = "geminiAccountId")]
    pub gemini_account_id: Option<String>,
    #[serde(rename = "openaiAccountId")]
    pub openai_account_id: Option<String>,
    #[serde(rename = "azureOpenaiAccountId")]
    pub azure_openai_account_id: Option<String>,
    #[serde(rename = "bedrockAccountId")]
    pub bedrock_account_id: Option<String>,
    #[serde(rename = "droidAccountId")]
    pub droid_account_id: Option<String>,

    /// 用户 ID (如果启用用户管理)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// 创建者
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// 创建者类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_type: Option<String>,
}

/// API Key 创建选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCreateOptions {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(default)]
    pub permissions: ApiKeyPermissions,

    #[serde(default)]
    pub is_active: bool,

    #[serde(default)]
    pub token_limit: i64,

    #[serde(default)]
    pub concurrency_limit: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_window: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_requests: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_cost: Option<f64>,

    #[serde(default)]
    pub daily_cost_limit: f64,

    #[serde(default)]
    pub total_cost_limit: f64,

    #[serde(default)]
    pub weekly_opus_cost_limit: f64,

    #[serde(default)]
    pub enable_model_restriction: bool,

    #[serde(default)]
    pub restricted_models: Vec<String>,

    #[serde(default)]
    pub enable_client_restriction: bool,

    #[serde(default)]
    pub allowed_clients: Vec<String>,

    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(default)]
    pub expiration_mode: ExpirationMode,

    #[serde(default)]
    pub activation_days: i64,

    #[serde(default)]
    pub activation_unit: ActivationUnit,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,

    // 账户绑定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_account_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_console_account_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini_account_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_account_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_openai_account_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock_account_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub droid_account_id: Option<String>,

    // 用户关联
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_type: Option<String>,
}

impl Default for ApiKeyCreateOptions {
    fn default() -> Self {
        Self {
            name: "Unnamed Key".to_string(),
            description: None,
            icon: None,
            permissions: ApiKeyPermissions::All,
            is_active: true,
            token_limit: 0,
            concurrency_limit: 0,
            rate_limit_window: None,
            rate_limit_requests: None,
            rate_limit_cost: None,
            daily_cost_limit: 0.0,
            total_cost_limit: 0.0,
            weekly_opus_cost_limit: 0.0,
            enable_model_restriction: false,
            restricted_models: Vec::new(),
            enable_client_restriction: false,
            allowed_clients: Vec::new(),
            tags: Vec::new(),
            expiration_mode: ExpirationMode::Fixed,
            activation_days: 0,
            activation_unit: ActivationUnit::Days,
            expires_at: None,
            claude_account_id: None,
            claude_console_account_id: None,
            gemini_account_id: None,
            openai_account_id: None,
            azure_openai_account_id: None,
            bedrock_account_id: None,
            droid_account_id: None,
            user_id: None,
            created_by: None,
            created_by_type: None,
        }
    }
}

/// API Key 使用统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeyUsageStats {
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_creation_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cost: f64,
    pub daily_cost: f64,
    pub weekly_opus_cost: f64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub usage_by_model: HashMap<String, ModelUsage>,
}

/// 按模型的使用统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelUsage {
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub cost: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissions_default() {
        let perms = ApiKeyPermissions::default();
        assert_eq!(perms, ApiKeyPermissions::All);
        assert!(perms.can_access_claude());
        assert!(perms.can_access_gemini());
        assert!(perms.can_access_openai());
        assert!(perms.can_access_droid());
    }

    #[test]
    fn test_permissions_claude_only() {
        let perms = ApiKeyPermissions::Claude;
        assert!(perms.can_access_claude());
        assert!(!perms.can_access_gemini());
        assert!(!perms.can_access_openai());
        assert!(!perms.can_access_droid());
    }

    #[test]
    fn test_expiration_mode_default() {
        let mode = ExpirationMode::default();
        assert_eq!(mode, ExpirationMode::Fixed);
    }

    #[test]
    fn test_api_key_create_options_default() {
        let options = ApiKeyCreateOptions::default();
        assert_eq!(options.name, "Unnamed Key");
        assert_eq!(options.permissions, ApiKeyPermissions::All);
        assert!(options.is_active);
        assert_eq!(options.token_limit, 0);
        assert_eq!(options.concurrency_limit, 0);
    }
}
