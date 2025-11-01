use crate::models::Platform;
use crate::redis::RedisPool;
use crate::services::account::ClaudeAccountService;
use crate::services::account_scheduler::AccountScheduler;
use crate::services::relay_trait::{
    GenericRelayResponse, GenericStreamChunk, RelayRequest, RelayService, UsageStats,
};
use crate::utils::error::{AppError, Result};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::info;

/// OpenAI API 配置
#[derive(Debug, Clone)]
pub struct OpenAIRelayConfig {
    pub api_base_url: String,
    pub default_model: String,
    pub timeout_seconds: u64,
}

impl Default for OpenAIRelayConfig {
    fn default() -> Self {
        Self {
            api_base_url: "https://api.openai.com/v1".to_string(),
            default_model: "gpt-4".to_string(),
            timeout_seconds: 600,
        }
    }
}

/// OpenAI 消息格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI 请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// OpenAI 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAIUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// OpenAI API 转发服务
pub struct OpenAIRelayService {
    config: OpenAIRelayConfig,
    http_client: Arc<Client>,
    #[allow(dead_code)]
    redis: Arc<RedisPool>,
    account_service: Arc<ClaudeAccountService>,
    account_scheduler: Arc<AccountScheduler>,
}

impl OpenAIRelayService {
    /// 创建新的 OpenAI 转发服务
    pub fn new(
        config: OpenAIRelayConfig,
        http_client: Arc<Client>,
        redis: Arc<RedisPool>,
        account_service: Arc<ClaudeAccountService>,
        account_scheduler: Arc<AccountScheduler>,
    ) -> Self {
        Self {
            config,
            http_client,
            redis,
            account_service,
            account_scheduler,
        }
    }

    /// 提取缓存创建 tokens
    fn extract_cache_creation_tokens(usage: &OpenAIUsage) -> u32 {
        usage
            .prompt_tokens_details
            .as_ref()
            .and_then(|details| details.cache_creation_input_tokens)
            .unwrap_or(0)
    }

    /// 提取缓存读取 tokens
    fn extract_cache_read_tokens(usage: &OpenAIUsage) -> u32 {
        usage
            .prompt_tokens_details
            .as_ref()
            .and_then(|details| details.cache_read_input_tokens)
            .unwrap_or(0)
    }
}

#[async_trait]
impl RelayService for OpenAIRelayService {
    fn platform(&self) -> Platform {
        Platform::OpenAI
    }

    fn api_base_url(&self) -> &str {
        &self.config.api_base_url
    }

    async fn relay_request(&self, request: RelayRequest) -> Result<GenericRelayResponse> {
        // 1. 选择账户
        let selected_account = self
            .account_scheduler
            .select_account(request.session_hash.as_deref(), Platform::OpenAI)
            .await
            .context("Failed to select OpenAI account")?;

        info!(
            "📤 Processing OpenAI request for account: {} ({:?}), model: {}",
            selected_account.account_id, selected_account.account_type, request.model
        );

        // 2. 获取账户详细信息
        let account = self
            .account_service
            .get_account(&selected_account.account_id)
            .await?
            .ok_or_else(|| AppError::NotFound("OpenAI account not found".to_string()))?;

        // 3. 获取 API Key
        let api_key = account
            .access_token
            .as_ref()
            .ok_or_else(|| AppError::Unauthorized("No OpenAI API key available".to_string()))?;

        // 4. 转换请求格式
        let openai_body = self.transform_request(&request)?;

        // 5. 构建请求URL
        let url = format!("{}/chat/completions", self.config.api_base_url);

        // 6. 发送请求
        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.http_client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&openai_body)
                .send(),
        )
        .await
        .context("Request timeout")?
        .context("Failed to send request")?;

        let status_code = response.status().as_u16();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body_bytes = response
            .bytes()
            .await
            .context("Failed to read response body")?
            .to_vec();

        // 7. 解析 usage
        let usage = if status_code == 200 {
            self.transform_response(&body_bytes).ok()
        } else {
            None
        };

        Ok(GenericRelayResponse {
            status_code,
            headers,
            body: body_bytes,
            account_id: selected_account.account_id,
            account_type: selected_account.account_type,
            usage,
        })
    }

    async fn relay_request_stream(
        &self,
        _request: RelayRequest,
    ) -> Result<mpsc::Receiver<Result<GenericStreamChunk>>> {
        // TODO: 实现 OpenAI 流式响应
        Err(AppError::BadRequest(
            "OpenAI streaming not yet implemented".to_string(),
        ))
    }

    fn transform_request(&self, request: &RelayRequest) -> Result<JsonValue> {
        // OpenAI 格式通常与输入兼容，只需要验证必需字段
        let messages = request.body["messages"]
            .as_array()
            .ok_or_else(|| AppError::BadRequest("Request missing messages array".to_string()))?;

        // 构建 OpenAI 请求
        let mut openai_req = json!({
            "model": request.model,
            "messages": messages,
        });

        // 添加可选参数
        if let Some(temp) = request.body["temperature"].as_f64() {
            openai_req["temperature"] = json!(temp);
        }

        if let Some(max_tokens) = request.body["max_tokens"].as_u64() {
            openai_req["max_tokens"] = json!(max_tokens);
        }

        if request.stream {
            openai_req["stream"] = json!(true);
        }

        Ok(openai_req)
    }

    fn transform_response(&self, response_body: &[u8]) -> Result<UsageStats> {
        let openai_response: OpenAIResponse =
            serde_json::from_slice(response_body).context("Failed to parse OpenAI response")?;

        if let Some(usage) = openai_response.usage {
            Ok(UsageStats {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
                cache_creation_tokens: Some(Self::extract_cache_creation_tokens(&usage)),
                cache_read_tokens: Some(Self::extract_cache_read_tokens(&usage)),
                total_tokens: usage.total_tokens,
            })
        } else {
            Err(AppError::InternalError(
                "No usage data in OpenAI response".to_string(),
            ))
        }
    }

    async fn validate_account(&self, account_id: &str) -> Result<bool> {
        let account = self.account_service.get_account(account_id).await?;
        Ok(account.is_some() && account.unwrap().access_token.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OpenAIRelayConfig::default();
        assert!(config.api_base_url.contains("api.openai.com"));
        assert_eq!(config.timeout_seconds, 600);
        assert_eq!(config.default_model, "gpt-4");
    }

    #[test]
    fn test_extract_cache_tokens() {
        let usage = OpenAIUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            prompt_tokens_details: Some(PromptTokensDetails {
                cache_creation_input_tokens: Some(20),
                cache_read_input_tokens: Some(10),
            }),
        };

        assert_eq!(
            OpenAIRelayService::extract_cache_creation_tokens(&usage),
            20
        );
        assert_eq!(OpenAIRelayService::extract_cache_read_tokens(&usage), 10);
    }

    #[test]
    fn test_extract_cache_tokens_none() {
        let usage = OpenAIUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            prompt_tokens_details: None,
        };

        assert_eq!(OpenAIRelayService::extract_cache_creation_tokens(&usage), 0);
        assert_eq!(OpenAIRelayService::extract_cache_read_tokens(&usage), 0);
    }
}
