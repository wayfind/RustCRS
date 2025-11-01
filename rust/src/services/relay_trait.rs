use crate::models::{AccountType, Platform};
use crate::utils::error::Result;
use async_trait::async_trait;
use bytes::Bytes;
use serde_json::Value as JsonValue;
use tokio::sync::mpsc;

/// 通用 API 转发请求
#[derive(Debug, Clone)]
pub struct RelayRequest {
    /// 模型名称
    pub model: String,
    /// 请求体（JSON格式）
    pub body: JsonValue,
    /// 会话哈希（用于粘性会话）
    pub session_hash: Option<String>,
    /// 是否流式请求
    pub stream: bool,
}

/// 通用 API 转发响应
#[derive(Debug)]
pub struct GenericRelayResponse {
    /// HTTP 状态码
    pub status_code: u16,
    /// 响应头
    pub headers: Vec<(String, String)>,
    /// 响应体
    pub body: Vec<u8>,
    /// 使用的账户ID
    pub account_id: String,
    /// 账户类型
    pub account_type: AccountType,
    /// 使用统计（如果有）
    pub usage: Option<UsageStats>,
}

/// 通用使用统计
#[derive(Debug, Clone)]
pub struct UsageStats {
    /// 输入tokens
    pub input_tokens: u32,
    /// 输出tokens
    pub output_tokens: u32,
    /// 缓存创建tokens（可选）
    pub cache_creation_tokens: Option<u32>,
    /// 缓存读取tokens（可选）
    pub cache_read_tokens: Option<u32>,
    /// 总tokens
    pub total_tokens: u32,
}

/// 流式数据块（通用）
#[derive(Debug, Clone)]
pub enum GenericStreamChunk {
    /// 原始数据
    Data(Bytes),
    /// 使用统计
    Usage(UsageStats),
    /// 错误
    Error(String),
}

/// API 转发服务 Trait
///
/// 定义了所有平台转发服务必须实现的接口
#[async_trait]
pub trait RelayService: Send + Sync {
    /// 获取支持的平台
    fn platform(&self) -> Platform;

    /// 非流式请求转发
    async fn relay_request(&self, request: RelayRequest) -> Result<GenericRelayResponse>;

    /// 流式请求转发
    async fn relay_request_stream(
        &self,
        request: RelayRequest,
    ) -> Result<mpsc::Receiver<Result<GenericStreamChunk>>>;

    /// 转换请求格式（从通用格式到平台特定格式）
    fn transform_request(&self, request: &RelayRequest) -> Result<JsonValue>;

    /// 转换响应格式（从平台特定格式到通用格式）
    fn transform_response(&self, response_body: &[u8]) -> Result<UsageStats>;

    /// 验证账户token是否有效
    async fn validate_account(&self, account_id: &str) -> Result<bool>;

    /// 获取API基础URL
    fn api_base_url(&self) -> &str;
}

/// 转发服务管理器
///
/// 管理多个平台的转发服务，根据平台类型路由请求
pub struct RelayManager {
    services: std::collections::HashMap<Platform, Box<dyn RelayService>>,
}

impl RelayManager {
    /// 创建新的转发管理器
    pub fn new() -> Self {
        Self {
            services: std::collections::HashMap::new(),
        }
    }

    /// 注册平台转发服务
    pub fn register(&mut self, service: Box<dyn RelayService>) {
        let platform = service.platform();
        self.services.insert(platform, service);
    }

    /// 获取指定平台的转发服务
    pub fn get_service(&self, platform: Platform) -> Option<&dyn RelayService> {
        self.services.get(&platform).map(|s| s.as_ref())
    }

    /// 执行非流式转发
    pub async fn relay(
        &self,
        platform: Platform,
        request: RelayRequest,
    ) -> Result<GenericRelayResponse> {
        let service = self.get_service(platform).ok_or_else(|| {
            crate::utils::error::AppError::BadRequest(format!(
                "Platform {:?} not supported",
                platform
            ))
        })?;

        service.relay_request(request).await
    }

    /// 执行流式转发
    pub async fn relay_stream(
        &self,
        platform: Platform,
        request: RelayRequest,
    ) -> Result<mpsc::Receiver<Result<GenericStreamChunk>>> {
        let service = self.get_service(platform).ok_or_else(|| {
            crate::utils::error::AppError::BadRequest(format!(
                "Platform {:?} not supported",
                platform
            ))
        })?;

        service.relay_request_stream(request).await
    }

    /// 获取所有支持的平台
    pub fn supported_platforms(&self) -> Vec<Platform> {
        self.services.keys().copied().collect()
    }
}

impl Default for RelayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_manager_creation() {
        let manager = RelayManager::new();
        assert_eq!(manager.supported_platforms().len(), 0);
    }

    #[test]
    fn test_usage_stats() {
        let stats = UsageStats {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_tokens: Some(20),
            cache_read_tokens: Some(10),
            total_tokens: 150,
        };
        assert_eq!(stats.total_tokens, 150);
    }
}
