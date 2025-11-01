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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// AWS Bedrock 配置
#[derive(Debug, Clone)]
pub struct BedrockRelayConfig {
    pub default_region: String,
    pub default_model: String,
    pub small_fast_model: String,
    pub small_fast_model_region: String,
    pub max_output_tokens: u32,
    pub max_thinking_tokens: u32,
    pub enable_prompt_caching: bool,
    pub timeout_seconds: u64,
}

impl Default for BedrockRelayConfig {
    fn default() -> Self {
        Self {
            default_region: "us-east-1".to_string(),
            default_model: "us.anthropic.claude-sonnet-4-20250514-v1:0".to_string(),
            small_fast_model: "us.anthropic.claude-3-5-haiku-20241022-v1:0".to_string(),
            small_fast_model_region: "us-east-1".to_string(),
            max_output_tokens: 4096,
            max_thinking_tokens: 1024,
            enable_prompt_caching: true,
            timeout_seconds: 600,
        }
    }
}

/// Claude 消息格式（与 Bedrock 兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: JsonValue,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

/// Bedrock 请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockRequest {
    pub anthropic_version: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<JsonValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<JsonValue>,
}

/// Bedrock 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<BedrockUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// Bedrock API 转发服务
pub struct BedrockRelayService {
    config: BedrockRelayConfig,
    #[allow(dead_code)]
    http_client: Arc<Client>,
    #[allow(dead_code)]
    redis: Arc<RedisPool>,
    account_service: Arc<ClaudeAccountService>,
    #[allow(dead_code)]
    account_scheduler: Arc<AccountScheduler>,
    #[allow(dead_code)]
    model_mapping: HashMap<String, String>,
}

impl BedrockRelayService {
    /// 创建新的 Bedrock 转发服务
    pub fn new(
        config: BedrockRelayConfig,
        http_client: Arc<Client>,
        redis: Arc<RedisPool>,
        account_service: Arc<ClaudeAccountService>,
        account_scheduler: Arc<AccountScheduler>,
    ) -> Self {
        let mut model_mapping = HashMap::new();

        // Claude Sonnet 4
        model_mapping.insert(
            "claude-sonnet-4".to_string(),
            "us.anthropic.claude-sonnet-4-20250514-v1:0".to_string(),
        );
        model_mapping.insert(
            "claude-sonnet-4-20250514".to_string(),
            "us.anthropic.claude-sonnet-4-20250514-v1:0".to_string(),
        );

        // Claude Opus 4.1
        model_mapping.insert(
            "claude-opus-4".to_string(),
            "us.anthropic.claude-opus-4-1-20250805-v1:0".to_string(),
        );
        model_mapping.insert(
            "claude-opus-4-1".to_string(),
            "us.anthropic.claude-opus-4-1-20250805-v1:0".to_string(),
        );
        model_mapping.insert(
            "claude-opus-4-1-20250805".to_string(),
            "us.anthropic.claude-opus-4-1-20250805-v1:0".to_string(),
        );

        // Claude 3.7 Sonnet
        model_mapping.insert(
            "claude-3-7-sonnet".to_string(),
            "us.anthropic.claude-3-7-sonnet-20250219-v1:0".to_string(),
        );
        model_mapping.insert(
            "claude-3-7-sonnet-20250219".to_string(),
            "us.anthropic.claude-3-7-sonnet-20250219-v1:0".to_string(),
        );

        // Claude 3.5 Sonnet v2
        model_mapping.insert(
            "claude-3-5-sonnet".to_string(),
            "us.anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
        );
        model_mapping.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            "us.anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
        );

        // Claude 3.5 Haiku
        model_mapping.insert(
            "claude-3-5-haiku".to_string(),
            "us.anthropic.claude-3-5-haiku-20241022-v1:0".to_string(),
        );
        model_mapping.insert(
            "claude-3-5-haiku-20241022".to_string(),
            "us.anthropic.claude-3-5-haiku-20241022-v1:0".to_string(),
        );

        // Claude 3 Sonnet
        model_mapping.insert(
            "claude-3-sonnet".to_string(),
            "us.anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
        );
        model_mapping.insert(
            "claude-3-sonnet-20240229".to_string(),
            "us.anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
        );

        // Claude 3 Haiku
        model_mapping.insert(
            "claude-3-haiku".to_string(),
            "us.anthropic.claude-3-haiku-20240307-v1:0".to_string(),
        );
        model_mapping.insert(
            "claude-3-haiku-20240307".to_string(),
            "us.anthropic.claude-3-haiku-20240307-v1:0".to_string(),
        );

        Self {
            config,
            http_client,
            redis,
            account_service,
            account_scheduler,
            model_mapping,
        }
    }

    /// 映射标准 Claude 模型名到 Bedrock 格式
    #[allow(dead_code)]
    fn map_to_bedrock_model(&self, model_name: &str) -> String {
        // 如果已经是 Bedrock 格式，直接返回
        if model_name.contains(".anthropic.") || model_name.starts_with("anthropic.") {
            return model_name.to_string();
        }

        // 查找映射
        if let Some(mapped_model) = self.model_mapping.get(model_name) {
            info!("🔄 Model mapping: {} → {}", model_name, mapped_model);
            return mapped_model.clone();
        }

        // 如果没有找到映射，返回原始模型名
        warn!(
            "⚠️ No model mapping found for: {}, using original name",
            model_name
        );
        model_name.to_string()
    }

    /// 选择区域
    #[allow(dead_code)]
    fn select_region(&self, model_id: &str) -> &str {
        // 对于小模型，使用专门的区域配置
        if model_id.contains("haiku") {
            &self.config.small_fast_model_region
        } else {
            &self.config.default_region
        }
    }

    /// 转换 Claude 格式请求到 Bedrock 格式
    fn convert_to_bedrock_format(&self, request: &RelayRequest) -> Result<BedrockRequest> {
        let messages: Vec<ClaudeMessage> = serde_json::from_value(request.body["messages"].clone())
            .context("Failed to parse messages")?;

        let max_tokens = request.body["max_tokens"]
            .as_u64()
            .unwrap_or(self.config.max_output_tokens as u64)
            .min(self.config.max_output_tokens as u64) as u32;

        let mut bedrock_req = BedrockRequest {
            anthropic_version: "bedrock-2023-05-31".to_string(),
            max_tokens,
            messages,
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            metadata: None,
            thinking: None,
        };

        // 添加可选参数
        if let Some(system) = request.body["system"].as_str() {
            bedrock_req.system = Some(system.to_string());
        }

        if let Some(temp) = request.body["temperature"].as_f64() {
            bedrock_req.temperature = Some(temp as f32);
        }

        if let Some(top_p) = request.body["top_p"].as_f64() {
            bedrock_req.top_p = Some(top_p as f32);
        }

        if let Some(top_k) = request.body["top_k"].as_u64() {
            bedrock_req.top_k = Some(top_k as u32);
        }

        if let Some(stop_sequences) = request.body["stop_sequences"].as_array() {
            bedrock_req.stop_sequences = Some(
                stop_sequences
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
            );
        }

        if let Some(tools) = request.body["tools"].as_array() {
            bedrock_req.tools = Some(tools.clone());
        }

        if let Some(tool_choice) = request.body.get("tool_choice") {
            bedrock_req.tool_choice = Some(tool_choice.clone());
        }

        if let Some(metadata) = request.body.get("metadata") {
            bedrock_req.metadata = Some(metadata.clone());
        }

        if let Some(thinking) = request.body.get("thinking") {
            bedrock_req.thinking = Some(thinking.clone());
            info!("🧠 Extended Thinking enabled for Bedrock");
        }

        Ok(bedrock_req)
    }

    /// 转换 Bedrock 响应到 Claude 格式
    #[allow(dead_code)]
    fn convert_from_bedrock_format(&self, bedrock_response: &BedrockResponse) -> JsonValue {
        json!({
            "id": bedrock_response.id,
            "type": bedrock_response.response_type,
            "role": bedrock_response.role,
            "content": bedrock_response.content,
            "model": bedrock_response.model.as_ref().unwrap_or(&self.config.default_model),
            "stop_reason": bedrock_response.stop_reason,
            "stop_sequence": bedrock_response.stop_sequence,
            "usage": bedrock_response.usage.as_ref().map(|u| json!({
                "input_tokens": u.input_tokens,
                "output_tokens": u.output_tokens,
                "cache_creation_input_tokens": u.cache_creation_input_tokens,
                "cache_read_input_tokens": u.cache_read_input_tokens,
            }))
        })
    }
}

#[async_trait]
impl RelayService for BedrockRelayService {
    fn platform(&self) -> Platform {
        Platform::Bedrock
    }

    fn api_base_url(&self) -> &str {
        // Bedrock 使用 AWS SDK，没有固定的 base URL
        "bedrock-runtime"
    }

    async fn relay_request(&self, _request: RelayRequest) -> Result<GenericRelayResponse> {
        // TODO: 实现 AWS Bedrock API 调用
        // 需要集成 AWS SDK for Rust (aws-sdk-bedrockruntime)
        // 这需要额外的依赖和 AWS 凭证管理

        Err(AppError::BadRequest(
            "Bedrock relay not yet fully implemented - requires AWS SDK integration".to_string(),
        ))
    }

    async fn relay_request_stream(
        &self,
        _request: RelayRequest,
    ) -> Result<mpsc::Receiver<Result<GenericStreamChunk>>> {
        Err(AppError::BadRequest(
            "Bedrock streaming not yet implemented".to_string(),
        ))
    }

    fn transform_request(&self, request: &RelayRequest) -> Result<JsonValue> {
        let bedrock_req = self.convert_to_bedrock_format(request)?;
        Ok(serde_json::to_value(bedrock_req).context("Failed to serialize Bedrock request")?)
    }

    fn transform_response(&self, response_body: &[u8]) -> Result<UsageStats> {
        let bedrock_response: BedrockResponse =
            serde_json::from_slice(response_body).context("Failed to parse Bedrock response")?;

        if let Some(usage) = bedrock_response.usage {
            Ok(UsageStats {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_creation_tokens: usage.cache_creation_input_tokens,
                cache_read_tokens: usage.cache_read_input_tokens,
                total_tokens: usage.input_tokens + usage.output_tokens,
            })
        } else {
            Err(AppError::InternalError(
                "No usage data in Bedrock response".to_string(),
            ))
        }
    }

    async fn validate_account(&self, account_id: &str) -> Result<bool> {
        let account = self.account_service.get_account(account_id).await?;
        // Bedrock 账户应该有 AWS 凭证配置
        Ok(account.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        LoggingSettings, RedisSettings, SecuritySettings, ServerSettings, Settings,
    };

    fn create_test_settings() -> Settings {
        Settings {
            server: ServerSettings {
                host: "0.0.0.0".to_string(),
                port: 8080,
                request_timeout: 600000,
            },
            redis: RedisSettings {
                host: "localhost".to_string(),
                port: 6379,
                password: None,
                db: 0,
                pool_size: 10,
            },
            security: SecuritySettings {
                jwt_secret: "test-jwt-secret-32-characters!!".to_string(),
                encryption_key: "test-encryption-key-32chars!!".to_string(),
                api_key_prefix: "cr_".to_string(),
            },
            logging: LoggingSettings {
                level: "info".to_string(),
                format: "pretty".to_string(),
            },
        }
    }

    #[test]
    fn test_default_config() {
        let config = BedrockRelayConfig::default();
        assert_eq!(config.default_region, "us-east-1");
        assert_eq!(config.max_output_tokens, 4096);
        assert!(config.enable_prompt_caching);
    }

    #[test]
    fn test_model_mapping() {
        let config = BedrockRelayConfig::default();
        let http_client = Arc::new(Client::new());
        let settings = create_test_settings();
        let redis = Arc::new(RedisPool::new(&settings).unwrap());
        let settings_arc = Arc::new(settings);
        let account_service =
            Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
        let account_scheduler = Arc::new(AccountScheduler::new(
            redis.clone(),
            account_service.clone(),
        ));

        let service = BedrockRelayService::new(
            config,
            http_client,
            redis,
            account_service,
            account_scheduler,
        );

        // 测试标准模型名映射
        assert_eq!(
            service.map_to_bedrock_model("claude-sonnet-4"),
            "us.anthropic.claude-sonnet-4-20250514-v1:0"
        );

        assert_eq!(
            service.map_to_bedrock_model("claude-3-5-haiku"),
            "us.anthropic.claude-3-5-haiku-20241022-v1:0"
        );

        // 测试已经是 Bedrock 格式的模型名
        assert_eq!(
            service.map_to_bedrock_model("us.anthropic.claude-sonnet-4-20250514-v1:0"),
            "us.anthropic.claude-sonnet-4-20250514-v1:0"
        );
    }

    #[test]
    fn test_select_region() {
        let config = BedrockRelayConfig::default();
        let http_client = Arc::new(Client::new());
        let settings = create_test_settings();
        let redis = Arc::new(RedisPool::new(&settings).unwrap());
        let settings_arc = Arc::new(settings);
        let account_service =
            Arc::new(ClaudeAccountService::new(redis.clone(), settings_arc).unwrap());
        let account_scheduler = Arc::new(AccountScheduler::new(
            redis.clone(),
            account_service.clone(),
        ));

        let service = BedrockRelayService::new(
            config,
            http_client,
            redis,
            account_service,
            account_scheduler,
        );

        // Haiku 模型使用专门的区域
        assert_eq!(
            service.select_region("us.anthropic.claude-3-5-haiku-20241022-v1:0"),
            "us-east-1"
        );

        // 其他模型使用默认区域
        assert_eq!(
            service.select_region("us.anthropic.claude-sonnet-4-20250514-v1:0"),
            "us-east-1"
        );
    }
}
