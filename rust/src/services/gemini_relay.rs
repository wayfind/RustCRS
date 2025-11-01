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
use tracing::{info, warn};

/// Gemini API 配置
#[derive(Debug, Clone)]
pub struct GeminiRelayConfig {
    pub api_base_url: String,
    pub default_model: String,
    pub timeout_seconds: u64,
}

impl Default for GeminiRelayConfig {
    fn default() -> Self {
        Self {
            api_base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            default_model: "gemini-2.0-flash-exp".to_string(),
            timeout_seconds: 600,
        }
    }
}

/// Gemini 消息格式
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiMessage {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text {
        text: String,
    },
    #[allow(non_snake_case)]
    FunctionCall {
        functionCall: FunctionCall,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    args: JsonValue,
}

/// Gemini 请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "systemInstruction")]
    system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "generationConfig")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: Option<u32>,
}

/// Gemini 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Candidate {
    content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Content {
    parts: Vec<GeminiPart>,
    role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

/// Gemini API 转发服务
pub struct GeminiRelayService {
    config: GeminiRelayConfig,
    http_client: Arc<Client>,
    #[allow(dead_code)]
    redis: Arc<RedisPool>,
    account_service: Arc<ClaudeAccountService>,
    account_scheduler: Arc<AccountScheduler>,
}

impl GeminiRelayService {
    /// 创建新的 Gemini 转发服务
    pub fn new(
        config: GeminiRelayConfig,
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

    /// 转换 OpenAI 格式的消息到 Gemini 格式
    fn convert_messages_to_gemini(
        messages: &[JsonValue],
    ) -> Result<(Vec<GeminiMessage>, Option<String>)> {
        let mut contents = Vec::new();
        let mut system_instruction = String::new();

        for message in messages {
            let role = message["role"]
                .as_str()
                .ok_or_else(|| AppError::BadRequest("Message missing role".to_string()))?;
            let content = message["content"]
                .as_str()
                .ok_or_else(|| AppError::BadRequest("Message missing content".to_string()))?;

            match role {
                "system" => {
                    if !system_instruction.is_empty() {
                        system_instruction.push_str("\n\n");
                    }
                    system_instruction.push_str(content);
                }
                "user" => {
                    contents.push(GeminiMessage {
                        role: "user".to_string(),
                        parts: vec![GeminiPart::Text {
                            text: content.to_string(),
                        }],
                    });
                }
                "assistant" => {
                    contents.push(GeminiMessage {
                        role: "model".to_string(),
                        parts: vec![GeminiPart::Text {
                            text: content.to_string(),
                        }],
                    });
                }
                _ => {
                    warn!("Unknown message role: {}", role);
                }
            }
        }

        let system_inst = if system_instruction.is_empty() {
            None
        } else {
            Some(system_instruction)
        };

        Ok((contents, system_inst))
    }

    /// 转换 Gemini 响应到 OpenAI 格式
    #[allow(dead_code)]
    fn convert_gemini_to_openai(gemini_response: &GeminiResponse, model: &str) -> JsonValue {
        let candidate = gemini_response
            .candidates
            .first()
            .cloned()
            .unwrap_or_else(|| Candidate {
                content: Content {
                    parts: vec![],
                    role: "model".to_string(),
                },
                finish_reason: Some("error".to_string()),
            });

        let mut text_content = String::new();
        for part in &candidate.content.parts {
            if let GeminiPart::Text { text } = part {
                text_content.push_str(text);
            }
        }

        json!({
            "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            "object": "chat.completion",
            "created": chrono::Utc::now().timestamp(),
            "model": model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": text_content
                },
                "finish_reason": candidate.finish_reason.unwrap_or_else(|| "stop".to_string())
            }],
            "usage": {
                "prompt_tokens": gemini_response.usage_metadata.as_ref().map(|u| u.prompt_token_count).unwrap_or(0),
                "completion_tokens": gemini_response.usage_metadata.as_ref().map(|u| u.candidates_token_count).unwrap_or(0),
                "total_tokens": gemini_response.usage_metadata.as_ref().map(|u| u.total_token_count).unwrap_or(0)
            }
        })
    }

    /// 处理 Gemini 流式响应
    async fn process_gemini_stream_response(
        http_client: Arc<Client>,
        config: GeminiRelayConfig,
        request: RelayRequest,
        api_key: String,
        tx: mpsc::Sender<Result<GenericStreamChunk>>,
    ) -> Result<()> {
        use bytes::Buf;
        use futures::StreamExt;

        // 1. 构建请求URL
        let model_name = if request.model.starts_with("models/") {
            request.model.clone()
        } else {
            format!("models/{}", request.model)
        };
        let url = format!("{}:streamGenerateContent?key={}", model_name, api_key);
        let full_url = format!("{}/{}", config.api_base_url, url);

        // 2. 转换请求格式
        let gemini_body = {
            let messages = request.body["messages"].as_array().ok_or_else(|| {
                AppError::BadRequest("Request missing messages array".to_string())
            })?;

            let (contents, system_instruction) = Self::convert_messages_to_gemini(messages)?;

            let mut gemini_req = GeminiRequest {
                contents,
                system_instruction: system_instruction.map(|text| SystemInstruction {
                    parts: vec![GeminiPart::Text { text }],
                }),
                generation_config: None,
            };

            if let Some(temp) = request.body["temperature"].as_f64() {
                let mut config = GenerationConfig {
                    temperature: Some(temp as f32),
                    max_output_tokens: None,
                };

                if let Some(max_tokens) = request.body["max_tokens"].as_u64() {
                    config.max_output_tokens = Some(max_tokens as u32);
                }

                gemini_req.generation_config = Some(config);
            }

            serde_json::to_value(gemini_req).context("Failed to serialize Gemini request")?
        };

        // 3. 发送流式请求
        let response = timeout(
            Duration::from_secs(config.timeout_seconds),
            http_client
                .post(&full_url)
                .header("Content-Type", "application/json")
                .json(&gemini_body)
                .send(),
        )
        .await
        .context("Request timeout")?
        .context("Failed to send request")?;

        let status_code = response.status();

        // 4. 检查响应状态
        if !status_code.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::UpstreamError(format!(
                "Gemini API error ({}): {}",
                status_code, error_body
            )));
        }

        // 5. 获取字节流
        let mut bytes_stream = response.bytes_stream();

        // 6. 累积 usage 数据
        let mut accumulated_usage = UsageStats {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            total_tokens: 0,
        };

        // 7. 处理流式数据
        while let Some(chunk_result) = bytes_stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    // 尝试解析 SSE 事件
                    if let Ok(text) = std::str::from_utf8(chunk.chunk()) {
                        // Gemini 流式响应格式: data: {...}\n\n
                        for line in text.lines() {
                            if let Some(json_str) = line.strip_prefix("data: ") {
                                // 跳过 "data: "
                                if let Ok(json_value) = serde_json::from_str::<JsonValue>(json_str)
                                {
                                    // 提取 usageMetadata
                                    if let Some(usage_meta) = json_value.get("usageMetadata") {
                                        if let Some(prompt_tokens) = usage_meta
                                            .get("promptTokenCount")
                                            .and_then(|v| v.as_u64())
                                        {
                                            accumulated_usage.input_tokens = prompt_tokens as u32;
                                        }
                                        if let Some(candidates_tokens) = usage_meta
                                            .get("candidatesTokenCount")
                                            .and_then(|v| v.as_u64())
                                        {
                                            accumulated_usage.output_tokens =
                                                candidates_tokens as u32;
                                        }
                                        if let Some(total_tokens) = usage_meta
                                            .get("totalTokenCount")
                                            .and_then(|v| v.as_u64())
                                        {
                                            accumulated_usage.total_tokens = total_tokens as u32;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 转发原始数据
                    if tx.send(Ok(GenericStreamChunk::Data(chunk))).await.is_err() {
                        // 客户端断开连接
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(Err(AppError::UpstreamError(format!("Stream error: {}", e))))
                        .await;
                    return Err(AppError::UpstreamError(format!("Stream error: {}", e)));
                }
            }
        }

        // 8. 发送最终 usage 数据
        if accumulated_usage.total_tokens > 0 {
            let _ = tx
                .send(Ok(GenericStreamChunk::Usage(accumulated_usage)))
                .await;
        }

        Ok(())
    }
}

#[async_trait]
impl RelayService for GeminiRelayService {
    fn platform(&self) -> Platform {
        Platform::Gemini
    }

    fn api_base_url(&self) -> &str {
        &self.config.api_base_url
    }

    async fn relay_request(&self, request: RelayRequest) -> Result<GenericRelayResponse> {
        // 1. 选择账户
        let selected_account = self
            .account_scheduler
            .select_account(request.session_hash.as_deref(), Platform::Gemini)
            .await
            .context("Failed to select Gemini account")?;

        info!(
            "📤 Processing Gemini request for account: {} ({:?}), model: {}",
            selected_account.account_id, selected_account.account_type, request.model
        );

        // 2. 获取账户详细信息
        let account = self
            .account_service
            .get_account(&selected_account.account_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Gemini account not found".to_string()))?;

        // 3. 获取 API Key（Gemini 使用 access_token 字段）
        let api_key = account
            .access_token
            .as_ref()
            .ok_or_else(|| AppError::Unauthorized("No Gemini API key available".to_string()))?;

        // 4. 转换请求格式
        let gemini_body = self.transform_request(&request)?;

        // 5. 构建请求URL
        let model_name = if request.model.starts_with("models/") {
            request.model.clone()
        } else {
            format!("models/{}", request.model)
        };
        let url = format!(
            "{}/{}:generateContent?key={}",
            self.config.api_base_url, model_name, api_key
        );

        // 6. 发送请求
        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.http_client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&gemini_body)
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
        request: RelayRequest,
    ) -> Result<mpsc::Receiver<Result<GenericStreamChunk>>> {
        // 1. 选择账户
        let selected_account = self
            .account_scheduler
            .select_account(request.session_hash.as_deref(), Platform::Gemini)
            .await
            .context("Failed to select Gemini account")?;

        info!(
            "📡 Processing stream request for Gemini account: {} ({:?}), model: {}",
            selected_account.account_id, selected_account.account_type, request.model
        );

        // 2. 获取账户详细信息
        let account = self
            .account_service
            .get_account(&selected_account.account_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Gemini account not found".to_string()))?;

        // 3. 获取 API Key
        let api_key = account
            .access_token
            .as_ref()
            .ok_or_else(|| AppError::Unauthorized("No Gemini API key available".to_string()))?
            .clone();

        // 4. 增加并发计数
        let request_id = uuid::Uuid::new_v4().to_string();
        self.account_scheduler
            .increment_concurrency(&selected_account.account_id, &request_id, None)
            .await?;

        // 5. 创建channel用于流式传输
        let (tx, rx) = mpsc::channel::<Result<GenericStreamChunk>>(100);

        // 6. 克隆所需的数据供异步任务使用
        let account_id = selected_account.account_id.clone();
        let account_scheduler = Arc::clone(&self.account_scheduler);
        let config = self.config.clone();
        let http_client = Arc::clone(&self.http_client);

        // 7. 启动异步任务处理流式响应
        tokio::spawn(async move {
            let result = Self::process_gemini_stream_response(
                http_client,
                config,
                request,
                api_key,
                tx.clone(),
            )
            .await;

            // 8. 减少并发计数（无论成功还是失败）
            if let Err(e) = account_scheduler
                .decrement_concurrency(&account_id, &request_id)
                .await
            {
                tracing::error!(
                    "Failed to decrement concurrency for account {}: {}",
                    account_id,
                    e
                );
            }

            // 9. 处理错误
            if let Err(e) = result {
                tracing::error!(
                    "Gemini stream processing failed for account {}: {}",
                    account_id,
                    e
                );
                // 发送错误到channel
                let _ = tx.send(Err(AppError::UpstreamError(e.to_string()))).await;
            }
        });

        Ok(rx)
    }

    fn transform_request(&self, request: &RelayRequest) -> Result<JsonValue> {
        // 提取 messages
        let messages = request.body["messages"]
            .as_array()
            .ok_or_else(|| AppError::BadRequest("Request missing messages array".to_string()))?;

        // 转换消息格式
        let (contents, system_instruction) = Self::convert_messages_to_gemini(messages)?;

        // 构建 Gemini 请求
        let mut gemini_req = GeminiRequest {
            contents,
            system_instruction: system_instruction.map(|text| SystemInstruction {
                parts: vec![GeminiPart::Text { text }],
            }),
            generation_config: None,
        };

        // 添加 generation config
        if let Some(temp) = request.body["temperature"].as_f64() {
            let mut config = GenerationConfig {
                temperature: Some(temp as f32),
                max_output_tokens: None,
            };

            if let Some(max_tokens) = request.body["max_tokens"].as_u64() {
                config.max_output_tokens = Some(max_tokens as u32);
            }

            gemini_req.generation_config = Some(config);
        }

        Ok(serde_json::to_value(gemini_req).context("Failed to serialize Gemini request")?)
    }

    fn transform_response(&self, response_body: &[u8]) -> Result<UsageStats> {
        let gemini_response: GeminiResponse =
            serde_json::from_slice(response_body).context("Failed to parse Gemini response")?;

        if let Some(usage_meta) = gemini_response.usage_metadata {
            Ok(UsageStats {
                input_tokens: usage_meta.prompt_token_count,
                output_tokens: usage_meta.candidates_token_count,
                cache_creation_tokens: None,
                cache_read_tokens: None,
                total_tokens: usage_meta.total_token_count,
            })
        } else {
            Err(AppError::InternalError(
                "No usage metadata in Gemini response".to_string(),
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
        let config = GeminiRelayConfig::default();
        assert!(config
            .api_base_url
            .contains("generativelanguage.googleapis.com"));
        assert_eq!(config.timeout_seconds, 600);
    }

    #[test]
    fn test_message_conversion() {
        let messages = vec![
            json!({"role": "system", "content": "You are helpful"}),
            json!({"role": "user", "content": "Hello"}),
        ];

        let (contents, system_inst) =
            GeminiRelayService::convert_messages_to_gemini(&messages).unwrap();

        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, "user");
        assert_eq!(system_inst, Some("You are helpful".to_string()));
    }
}
