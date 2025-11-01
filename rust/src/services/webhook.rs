// Webhook Service - 简化的 webhook 通知系统
//
// 核心功能：
// 1. Webhook 配置管理（存储在 Redis）
// 2. 多 URL 并发通知
// 3. 重试机制（指数退避）
// 4. HMAC 签名验证
// 5. 超时控制

use crate::RedisPool;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Webhook 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub urls: Vec<String>,
    pub events: Vec<String>,
    pub secret: Option<String>,
    pub enabled: bool,
    pub retry_count: u32,
    pub timeout_ms: u64,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            urls: Vec::new(),
            events: vec![
                "account.failed".to_string(),
                "token.refresh.failed".to_string(),
                "rate_limit.exceeded".to_string(),
            ],
            secret: None,
            enabled: false,
            retry_count: 3,
            timeout_ms: 5000,
        }
    }
}

/// Webhook 通知 Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event_type: String,
    pub timestamp: i64,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Webhook Service
#[derive(Clone)]
pub struct WebhookService {
    redis: Arc<RedisPool>,
    http_client: reqwest::Client,
}

impl WebhookService {
    /// 创建 Webhook Service
    pub fn new(redis: Arc<RedisPool>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { redis, http_client }
    }

    /// 获取配置的 Redis key
    fn config_key(id: &str) -> String {
        format!("webhook_config:{}", id)
    }

    /// 创建 webhook 配置
    pub async fn create_config(&self, mut config: WebhookConfig) -> Result<WebhookConfig, String> {
        // 验证配置
        if config.urls.is_empty() {
            return Err("URLs cannot be empty".to_string());
        }
        if config.events.is_empty() {
            return Err("Events cannot be empty".to_string());
        }

        // 生成新 ID（如果没有）
        if config.id.is_empty() {
            config.id = uuid::Uuid::new_v4().to_string();
        }

        // 序列化并存储到 Redis
        let config_json = serde_json::to_string(&config).map_err(|e| e.to_string())?;

        let mut conn = self
            .redis
            .get_connection()
            .await
            .map_err(|e| format!("Redis connection failed: {}", e))?;

        conn.set::<_, _, ()>(&Self::config_key(&config.id), &config_json)
            .await
            .map_err(|e| format!("Failed to save config: {}", e))?;

        Ok(config)
    }

    /// 获取 webhook 配置
    pub async fn get_config(&self, id: &str) -> Result<WebhookConfig, String> {
        let mut conn = self
            .redis
            .get_connection()
            .await
            .map_err(|e| format!("Redis connection failed: {}", e))?;

        let config_json: String = conn
            .get(Self::config_key(id))
            .await
            .map_err(|e| format!("Config not found: {}", e))?;

        serde_json::from_str(&config_json).map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// 更新 webhook 配置
    pub async fn update_config(&self, config: &WebhookConfig) -> Result<(), String> {
        let config_json = serde_json::to_string(config).map_err(|e| e.to_string())?;

        let mut conn = self
            .redis
            .get_connection()
            .await
            .map_err(|e| format!("Redis connection failed: {}", e))?;

        conn.set::<_, _, ()>(&Self::config_key(&config.id), &config_json)
            .await
            .map_err(|e| format!("Failed to update config: {}", e))?;

        Ok(())
    }

    /// 删除 webhook 配置
    pub async fn delete_config(&self, id: &str) -> Result<(), String> {
        let mut conn = self
            .redis
            .get_connection()
            .await
            .map_err(|e| format!("Redis connection failed: {}", e))?;

        conn.del::<_, ()>(&Self::config_key(id))
            .await
            .map_err(|e| format!("Failed to delete config: {}", e))?;

        Ok(())
    }

    /// 生成 HMAC 签名
    fn generate_signature(secret: &str, payload: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());

        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// 创建 webhook payload
    fn create_payload(
        event_type: &str,
        data: serde_json::Value,
        secret: Option<&str>,
    ) -> Result<WebhookPayload, String> {
        let timestamp = chrono::Utc::now().timestamp();

        let mut payload = WebhookPayload {
            event_type: event_type.to_string(),
            timestamp,
            data,
            signature: None,
        };

        // 生成签名
        if let Some(secret) = secret {
            let payload_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
            payload.signature = Some(Self::generate_signature(secret, &payload_str));
        }

        Ok(payload)
    }

    /// 发送单个 webhook 通知（带重试）
    async fn send_webhook_with_retry(
        &self,
        url: &str,
        payload: &WebhookPayload,
        retry_count: u32,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let mut last_error = String::new();

        for attempt in 0..=retry_count {
            match self.send_webhook_once(url, payload, timeout_ms).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = e.clone();
                    if attempt < retry_count {
                        // 指数退避：100ms * 2^attempt
                        let delay_ms = 100 * (2_u64.pow(attempt));
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(format!(
            "Failed after {} retries: {}",
            retry_count, last_error
        ))
    }

    /// 发送单次 webhook 通知
    async fn send_webhook_once(
        &self,
        url: &str,
        payload: &WebhookPayload,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let response = self
            .http_client
            .post(url)
            .json(payload)
            .timeout(Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP {} {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            ));
        }

        Ok(())
    }

    /// 触发 webhook 通知
    pub async fn trigger(
        &self,
        config_id: &str,
        event_type: &str,
        data: serde_json::Value,
    ) -> Result<(), String> {
        // 获取配置
        let config = self.get_config(config_id).await?;

        // 检查是否启用
        if !config.enabled {
            return Err("Webhook is disabled".to_string());
        }

        // 检查事件类型
        if !config.events.contains(&event_type.to_string()) {
            return Err(format!("Event type '{}' not configured", event_type));
        }

        // 创建 payload
        let payload = Self::create_payload(event_type, data, config.secret.as_deref())?;

        // 并发发送到所有 URLs
        let mut handles = Vec::new();
        for url in config.urls.iter() {
            let url = url.clone();
            let payload = payload.clone();
            let service = self.clone();
            let retry_count = config.retry_count;
            let timeout_ms = config.timeout_ms;

            let handle = tokio::spawn(async move {
                service
                    .send_webhook_with_retry(&url, &payload, retry_count, timeout_ms)
                    .await
            });

            handles.push(handle);
        }

        // 等待所有请求完成
        let mut errors = Vec::new();
        for (idx, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok(_)) => {
                    // 成功
                }
                Ok(Err(e)) => {
                    errors.push(format!("URL {}: {}", idx, e));
                }
                Err(e) => {
                    errors.push(format!("URL {} task failed: {}", idx, e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(format!("Some webhooks failed: {}", errors.join(", ")));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_generation() {
        let secret = "test_secret";
        let payload = r#"{"event":"test","timestamp":1234567890}"#;
        let sig = WebhookService::generate_signature(secret, payload);

        // 验证签名不为空且为 hex 格式
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_default_config() {
        let config = WebhookConfig::default();
        assert!(!config.id.is_empty());
        assert_eq!(config.events.len(), 3);
        assert!(!config.enabled);
        assert_eq!(config.retry_count, 3);
        assert_eq!(config.timeout_ms, 5000);
    }
}
