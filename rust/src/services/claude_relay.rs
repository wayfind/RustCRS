use crate::models::{AccountType, ClaudeAccount, Platform};
use crate::redis::RedisPool;
use crate::services::account::ClaudeAccountService;
use crate::services::account_scheduler::{AccountScheduler, SelectedAccount};
use crate::utils::error::{AppError, Result};
use anyhow::Context;
use bytes::Bytes;
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Claude API请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Claude API响应体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// 转发响应结果
#[derive(Debug)]
pub struct RelayResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub account_id: String,
    pub account_type: AccountType,
    pub usage: Option<Usage>,
}

/// SSE事件（流式响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStart },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: Delta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDelta, usage: Usage },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "error")]
    Error { error: ErrorInfo },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStart {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<JsonValue>,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

/// Claude中继服务配置
#[derive(Debug, Clone)]
pub struct ClaudeRelayConfig {
    pub api_url: String,
    pub api_version: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl Default for ClaudeRelayConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.anthropic.com".to_string(),
            api_version: "2023-06-01".to_string(),
            timeout_seconds: 600, // 10 minutes for long-running requests
            max_retries: 1,
        }
    }
}

/// Claude API中继服务
pub struct ClaudeRelayService {
    config: ClaudeRelayConfig,
    http_client: Arc<Client>,
    redis: Arc<RedisPool>,
    account_service: Arc<ClaudeAccountService>,
    account_scheduler: Arc<AccountScheduler>,
}

impl ClaudeRelayService {
    /// 创建新的Claude中继服务实例
    pub fn new(
        config: ClaudeRelayConfig,
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

    /// 转发请求到Claude API
    pub async fn relay_request(
        &self,
        request_body: ClaudeRequest,
        session_hash: Option<String>,
        account_id: Option<String>,  // NEW: 接受已选择的账户 ID
    ) -> Result<RelayResponse> {
        // 1. 使用调度器选择账户（如果未提供账户 ID）
        let selected_account_id = if let Some(id) = account_id {
            id
        } else {
            let selected_account = self
                .account_scheduler
                .select_account(
                    session_hash.as_deref(),
                    Platform::Claude, // Claude官方API
                )
                .await
                .context("Failed to select account")?;
            selected_account.account_id
        };

        info!(
            "📤 Processing request for account: {}, model: {}",
            selected_account_id, request_body.model
        );

        // 2. 获取账户详细信息
        let account = self
            .account_service
            .get_account(&selected_account_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        // 3. 检查token有效性
        if !self.is_token_valid(&account) {
            warn!("Token expired for account {}, needs refresh", account.id);
            return Err(AppError::Unauthorized("Account token expired".to_string()));
        }

        // 4. 获取访问token
        let access_token = self.get_access_token(&account)?;

        // 5. 增加并发计数
        let request_id = uuid::Uuid::new_v4().to_string();
        self.account_scheduler
            .increment_concurrency(&selected_account_id, &request_id, None)
            .await?;

        // 6. 执行HTTP请求
        let result = self
            .make_claude_request(&request_body, &access_token, &account)
            .await;

        // 7. 减少并发计数
        self.account_scheduler
            .decrement_concurrency(&selected_account_id, &request_id)
            .await?;

        // 8. 处理结果
        match result {
            Ok(mut response) => {
                response.account_id = selected_account_id.clone();
                response.account_type = account.account_type.clone();

                // 处理错误状态码
                if response.status_code != 200 && response.status_code != 201 {
                    // handle_error_response 需要 SelectedAccount，这里直接记录错误
                    warn!("Non-OK status code {} from account {}", response.status_code, selected_account_id);
                }

                Ok(response)
            }
            Err(e) => {
                error!(
                    "Failed to make Claude request for account {}: {}",
                    selected_account_id, e
                );
                Err(e)
            }
        }
    }

    /// 执行Claude API HTTP请求
    async fn make_claude_request(
        &self,
        request_body: &ClaudeRequest,
        access_token: &str,
        account: &ClaudeAccount,
    ) -> Result<RelayResponse> {
        // Claude Console 使用 custom_api_endpoint，否则使用默认 API URL
        let base_url = account
            .custom_api_endpoint
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(&self.config.api_url);
        let url = format!("{}/v1/messages", base_url);

        let mut request_builder = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("anthropic-version", &self.config.api_version)
            .header("x-api-key", access_token);

        // 设置 User-Agent (Claude Console 需要特定的值)
        let user_agent = if account.platform == Platform::ClaudeConsole {
            debug!("Setting User-Agent to 'claude_code' for Claude Console");
            "claude_code"  // Claude Console requires this exact User-Agent
        } else {
            debug!("Setting User-Agent to 'claude-relay-service/1.0' for platform: {:?}", account.platform);
            "claude-relay-service/1.0"  // Default for other platforms
        };
        request_builder = request_builder.header("User-Agent", user_agent);

        let request_builder = request_builder.json(request_body);

        // 代理配置已在HTTP Client构建时设置，这里只需记录
        if account.proxy.is_some() {
            debug!("Using proxy for account {}", account.id);
        }

        // 执行请求（带超时）
        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            request_builder.send(),
        )
        .await
        .context("Request timeout")?
        .map_err(|e| {
            error!("HTTP request failed: {:?}", e);
            AppError::InternalError(format!("Failed to send request: {}", e))
        })?;

        let status_code = response.status().as_u16();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // 读取响应体
        let body_bytes = response
            .bytes()
            .await
            .context("Failed to read response body")?
            .to_vec();

        // 尝试解析usage数据
        let usage = if status_code == 200 || status_code == 201 {
            match serde_json::from_slice::<ClaudeResponse>(&body_bytes) {
                Ok(claude_response) => Some(claude_response.usage),
                Err(e) => {
                    warn!("Failed to parse Claude response for usage: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(RelayResponse {
            status_code,
            headers,
            body: body_bytes,
            account_id: account.id.to_string(),
            account_type: account.account_type.clone(),
            usage,
        })
    }

    /// 处理错误响应
    async fn handle_error_response(
        &self,
        response: &RelayResponse,
        selected_account: &SelectedAccount,
    ) -> Result<()> {
        match response.status_code {
            401 => {
                // 未授权错误
                warn!(
                    "🔐 Unauthorized error (401) for account {}",
                    selected_account.account_id
                );
                // 记录401错误次数
                self.record_unauthorized_error(&selected_account.account_id)
                    .await?;
            }
            403 => {
                // 禁止访问错误
                error!(
                    "🚫 Forbidden error (403) for account {}, marking as blocked",
                    selected_account.account_id
                );
                // 标记账户为blocked状态
                self.mark_account_blocked(&selected_account.account_id)
                    .await?;
            }
            429 => {
                // 限流错误
                warn!(
                    "⏱️ Rate limit error (429) for account {}",
                    selected_account.account_id
                );
                // 从响应头中提取重置时间
                let reset_time = self.extract_rate_limit_reset_time(&response.headers);
                self.mark_account_rate_limited(&selected_account.account_id, reset_time)
                    .await?;
            }
            529 => {
                // 服务过载错误
                warn!(
                    "🚫 Overload error (529) for account {}",
                    selected_account.account_id
                );
                self.account_scheduler
                    .mark_account_overloaded(&selected_account.account_id)
                    .await?;
            }
            _ => {
                debug!(
                    "Non-success status code {} for account {}",
                    response.status_code, selected_account.account_id
                );
            }
        }

        Ok(())
    }

    /// 检查token是否有效
    fn is_token_valid(&self, account: &ClaudeAccount) -> bool {
        if let Some(ref expires_at_str) = account.expires_at {
            if let Ok(expires_at) = expires_at_str.parse::<i64>() {
                let now = chrono::Utc::now().timestamp_millis();
                let buffer_ms = 10_000; // 10秒缓冲
                return expires_at > now + buffer_ms;
            }
        }
        // 如果没有expires_at，假设永久有效
        true
    }

    /// 获取访问token（已解密）
    ///
    /// 优先使用 session_token (Claude Console)，其次使用 access_token (官方 OAuth)
    fn get_access_token(&self, account: &ClaudeAccount) -> Result<String> {
        // Claude Console 使用 session_token
        if let Some(ref session_token) = account.session_token {
            return Ok(session_token.clone());
        }

        // 官方 OAuth 使用 access_token
        if let Some(ref access_token) = account.access_token {
            return Ok(access_token.clone());
        }

        Err(AppError::Unauthorized(
            "No access token or session token available".to_string(),
        ))
    }

    /// 记录401错误
    async fn record_unauthorized_error(&self, account_id: &str) -> Result<()> {
        let key = format!("401_errors:{}", account_id);
        let mut conn = self.redis.get_connection().await?;

        // 使用INCR增加计数
        let _: i32 = redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to increment 401 error count")?;

        // 设置5分钟过期
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(300)
            .query_async(&mut conn)
            .await
            .context("Failed to set expiry on 401 error counter")?;

        Ok(())
    }

    /// 标记账户为blocked状态
    async fn mark_account_blocked(&self, account_id: &str) -> Result<()> {
        // 这里应该更新账户状态为blocked
        // 暂时使用Redis标记
        let key = format!("account_blocked:{}", account_id);
        let mut conn = self.redis.get_connection().await?;

        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(3600) // 1小时
            .arg("1")
            .query_async(&mut conn)
            .await
            .context("Failed to mark account as blocked")?;

        warn!("🚫 Account {} marked as blocked", account_id);
        Ok(())
    }

    /// 标记账户为限流状态
    async fn mark_account_rate_limited(
        &self,
        account_id: &str,
        reset_time: Option<i64>,
    ) -> Result<()> {
        let key = format!("rate_limit_state:{}", account_id);
        let mut conn = self.redis.get_connection().await?;

        let ttl = if let Some(reset_ts) = reset_time {
            let now = chrono::Utc::now().timestamp();
            (reset_ts - now).max(60) as u64 // 至少60秒
        } else {
            600 // 默认10分钟
        };

        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl)
            .arg("1")
            .query_async(&mut conn)
            .await
            .context("Failed to mark account as rate limited")?;

        warn!(
            "⏱️ Account {} marked as rate limited for {} seconds",
            account_id, ttl
        );
        Ok(())
    }

    /// 从响应头中提取限流重置时间
    fn extract_rate_limit_reset_time(&self, headers: &[(String, String)]) -> Option<i64> {
        for (name, value) in headers {
            if name.eq_ignore_ascii_case("x-ratelimit-reset")
                || name.eq_ignore_ascii_case("retry-after")
            {
                if let Ok(timestamp) = value.parse::<i64>() {
                    return Some(timestamp);
                }
            }
        }
        None
    }

    /// 流式转发请求到Claude API（SSE）
    pub async fn relay_request_stream(
        &self,
        request_body: ClaudeRequest,
        session_hash: Option<String>,
        account_id: Option<String>,  // NEW: 接受已选择的账户 ID
    ) -> Result<mpsc::Receiver<Result<StreamChunk>>> {
        // 1. 使用调度器选择账户（如果未提供账户 ID）
        let selected_account_id = if let Some(id) = account_id {
            id
        } else {
            let selected_account = self
                .account_scheduler
                .select_account(session_hash.as_deref(), Platform::Claude)
                .await
                .context("Failed to select account")?;
            selected_account.account_id
        };

        info!(
            "📡 Processing stream request for account: {}, model: {}",
            selected_account_id, request_body.model
        );

        // 2. 获取账户详细信息
        let account = self
            .account_service
            .get_account(&selected_account_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        // 3. 检查token有效性
        if !self.is_token_valid(&account) {
            warn!("Token expired for account {}, needs refresh", account.id);
            return Err(AppError::Unauthorized("Account token expired".to_string()));
        }

        // 4. 获取访问token
        let access_token = self.get_access_token(&account)?;

        // 5. 增加并发计数
        let request_id = uuid::Uuid::new_v4().to_string();
        self.account_scheduler
            .increment_concurrency(&selected_account_id, &request_id, None)
            .await?;

        // 6. 创建channel用于流式传输
        let (tx, rx) = mpsc::channel::<Result<StreamChunk>>(100);

        // 7. 克隆所需的数据供异步任务使用
        let account_id = selected_account_id.clone();
        // account_type 不再需要，因为我们已经有完整的 account 对象
        let account_scheduler = Arc::clone(&self.account_scheduler);
        let config = self.config.clone();
        let http_client = Arc::clone(&self.http_client);

        // 8. 启动异步任务处理流式响应
        tokio::spawn(async move {
            let result = Self::process_stream_response(
                http_client,
                config,
                request_body,
                access_token,
                account,
                tx.clone(),
            )
            .await;

            // 9. 减少并发计数（无论成功还是失败）
            if let Err(e) = account_scheduler
                .decrement_concurrency(&account_id, &request_id)
                .await
            {
                error!(
                    "Failed to decrement concurrency for account {}: {}",
                    account_id, e
                );
            }

            // 10. 处理错误
            if let Err(e) = result {
                error!("Stream processing failed for account {}: {}", account_id, e);
                // 发送错误到channel
                let _ = tx.send(Err(AppError::UpstreamError(e.to_string()))).await;
            }
        });

        Ok(rx)
    }

    /// 处理流式响应（内部方法）
    async fn process_stream_response(
        http_client: Arc<Client>,
        config: ClaudeRelayConfig,
        request_body: ClaudeRequest,
        access_token: String,
        account: ClaudeAccount,
        tx: mpsc::Sender<Result<StreamChunk>>,
    ) -> Result<()> {
        // Claude Console 使用 custom_api_endpoint，否则使用默认 API URL
        let base_url = account
            .custom_api_endpoint
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(&config.api_url);
        let url = format!("{}/v1/messages", base_url);

        // 确保请求体包含 stream: true
        let mut stream_body = request_body.clone();
        stream_body.stream = Some(true);

        let mut request_builder = http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("anthropic-version", &config.api_version)
            .header("x-api-key", access_token);

        // Claude Console 需要特定的 User-Agent
        if account.platform == Platform::ClaudeConsole {
            request_builder = request_builder.header("User-Agent", "claude_code");
        }

        let response = timeout(
            Duration::from_secs(config.timeout_seconds),
            request_builder.json(&stream_body).send(),
        )
        .await
        .context("Request timeout")?
        .context("Failed to send request")?;

        let status_code = response.status().as_u16();

        // 检查错误状态码
        if status_code != 200 {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::UpstreamError(format!(
                "Status {}: {}",
                status_code, error_body
            )));
        }

        // 处理SSE流
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut accumulated_usage = Usage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    // 转发原始数据块
                    let chunk_bytes = chunk.to_vec();
                    if let Err(e) = tx
                        .send(Ok(StreamChunk::Data(Bytes::from(chunk_bytes.clone()))))
                        .await
                    {
                        warn!("Failed to send chunk to client: {}", e);
                        break;
                    }

                    // 解析SSE事件提取usage数据
                    let chunk_str = String::from_utf8_lossy(&chunk_bytes);
                    buffer.push_str(&chunk_str);

                    // 处理完整的SSE行
                    let ends_with_newline = buffer.ends_with('\n');
                    let lines: Vec<String> = buffer.lines().map(|s| s.to_string()).collect();

                    // 解析SSE事件（排除最后一行如果它不完整）
                    let lines_to_parse = if ends_with_newline {
                        buffer.clear();
                        &lines[..]
                    } else {
                        // 保留最后的不完整行
                        if let Some(last_line) = lines.last() {
                            buffer = last_line.clone();
                        }
                        &lines[..lines.len().saturating_sub(1)]
                    };

                    for line in lines_to_parse {
                        if let Some(event_data) = Self::parse_sse_line(line) {
                            Self::extract_usage_from_event(&event_data, &mut accumulated_usage);
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading stream chunk: {}", e);
                    let _ = tx.send(Err(AppError::UpstreamError(e.to_string()))).await;
                    break;
                }
            }
        }

        // 发送最终的usage数据
        if accumulated_usage.input_tokens > 0 || accumulated_usage.output_tokens > 0 {
            info!(
                "📊 Stream usage - Input: {}, Output: {}, Cache Create: {:?}, Cache Read: {:?}",
                accumulated_usage.input_tokens,
                accumulated_usage.output_tokens,
                accumulated_usage.cache_creation_input_tokens,
                accumulated_usage.cache_read_input_tokens
            );

            if let Err(e) = tx.send(Ok(StreamChunk::Usage(accumulated_usage))).await {
                warn!("Failed to send usage data: {}", e);
            }
        }

        Ok(())
    }

    /// 解析SSE行
    fn parse_sse_line(line: &str) -> Option<StreamEvent> {
        if line.starts_with("data: ") {
            let json_str = line.trim_start_matches("data: ").trim();
            if json_str.is_empty() || json_str == "[DONE]" {
                return None;
            }
            match serde_json::from_str::<StreamEvent>(json_str) {
                Ok(event) => Some(event),
                Err(e) => {
                    debug!("Failed to parse SSE event: {} - {}", e, json_str);
                    None
                }
            }
        } else {
            None
        }
    }

    /// 从SSE事件中提取usage数据
    fn extract_usage_from_event(event: &StreamEvent, accumulated: &mut Usage) {
        match event {
            StreamEvent::MessageStart { message } => {
                // message_start 包含 input tokens 和 cache tokens
                accumulated.input_tokens = message.usage.input_tokens;
                accumulated.cache_creation_input_tokens = message.usage.cache_creation_input_tokens;
                accumulated.cache_read_input_tokens = message.usage.cache_read_input_tokens;

                debug!(
                    "📊 Collected from message_start - Input: {}, Cache Create: {:?}, Cache Read: {:?}",
                    accumulated.input_tokens,
                    accumulated.cache_creation_input_tokens,
                    accumulated.cache_read_input_tokens
                );
            }
            StreamEvent::MessageDelta { delta: _, usage } => {
                // message_delta 包含 output tokens
                accumulated.output_tokens = usage.output_tokens;

                debug!(
                    "📊 Collected from message_delta - Output: {}",
                    accumulated.output_tokens
                );
            }
            _ => {
                // 其他事件类型不包含usage数据
            }
        }
    }
}

/// 流式数据块
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// 原始SSE数据
    Data(Bytes),
    /// 累积的usage数据（流结束时发送）
    Usage(Usage),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClaudeRelayConfig::default();
        assert_eq!(config.api_url, "https://api.anthropic.com");
        assert_eq!(config.api_version, "2023-06-01");
        assert_eq!(config.timeout_seconds, 600);
    }

    #[test]
    fn test_claude_request_serialization() {
        let request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            system: Some("You are a helpful assistant".to_string()),
            max_tokens: Some(1024),
            temperature: Some(1.0),
            stream: Some(false),
            metadata: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-3-5-sonnet-20241022"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_message_structure() {
        let message = Message {
            role: "user".to_string(),
            content: "Test message".to_string(),
        };

        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Test message");
    }

    #[test]
    fn test_claude_request_with_multiple_messages() {
        let request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "First message".to_string(),
                },
                Message {
                    role: "assistant".to_string(),
                    content: "First response".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: "Second message".to_string(),
                },
            ],
            system: None,
            max_tokens: Some(2048),
            temperature: Some(0.7),
            stream: Some(true),
            metadata: None,
        };

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, "user");
        assert_eq!(request.messages[1].role, "assistant");
        assert_eq!(request.messages[2].role, "user");
    }

    #[test]
    fn test_claude_request_optional_fields() {
        // 测试所有可选字段为 None
        let minimal_request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            system: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            metadata: None,
        };

        let json = serde_json::to_value(&minimal_request).unwrap();
        assert_eq!(json["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(json["messages"][0]["role"], "user");
    }

    #[test]
    fn test_claude_request_deserialization() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Test"
                }
            ],
            "max_tokens": 1024
        }"#;

        let request: ClaudeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.model, "claude-3-5-sonnet-20241022");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.max_tokens, Some(1024));
    }

    #[test]
    fn test_config_custom_values() {
        let config = ClaudeRelayConfig {
            api_url: "https://custom.api.com".to_string(),
            api_version: "2024-01-01".to_string(),
            timeout_seconds: 300,
            max_retries: 5,
        };

        assert_eq!(config.api_url, "https://custom.api.com");
        assert_eq!(config.api_version, "2024-01-01");
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_claude_request_temperature_range() {
        // 测试不同温度值
        let temps = vec![0.0, 0.5, 1.0, 1.5, 2.0];

        for temp in temps {
            let request = ClaudeRequest {
                model: "claude-3-5-sonnet-20241022".to_string(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: "Test".to_string(),
                }],
                system: None,
                max_tokens: Some(1024),
                temperature: Some(temp),
                stream: None,
                metadata: None,
            };

            assert_eq!(request.temperature, Some(temp));
        }
    }

    #[test]
    fn test_claude_request_stream_flag() {
        let streaming_request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Test".to_string(),
            }],
            system: None,
            max_tokens: Some(1024),
            temperature: None,
            stream: Some(true),
            metadata: None,
        };

        let non_streaming_request = ClaudeRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Test".to_string(),
            }],
            system: None,
            max_tokens: Some(1024),
            temperature: None,
            stream: Some(false),
            metadata: None,
        };

        assert_eq!(streaming_request.stream, Some(true));
        assert_eq!(non_streaming_request.stream, Some(false));
    }

    #[test]
    fn test_message_role_variants() {
        let user_msg = Message {
            role: "user".to_string(),
            content: "User message".to_string(),
        };

        let assistant_msg = Message {
            role: "assistant".to_string(),
            content: "Assistant response".to_string(),
        };

        let system_msg = Message {
            role: "system".to_string(),
            content: "System instruction".to_string(),
        };

        assert_eq!(user_msg.role, "user");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(system_msg.role, "system");
    }
}
