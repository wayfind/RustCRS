use reqwest::{Client, Proxy};
use std::time::Duration;

use crate::config::Settings;
use crate::utils::{AppError, Result};

/// HTTP client wrapper with proxy support
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    /// Create a new HTTP client with proxy support
    pub fn new(settings: &Settings) -> Result<Self> {
        let timeout = Duration::from_millis(settings.server.request_timeout);

        let builder = Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            // NOTE: Accept invalid certs when using proxy (Claude Code uses self-signed certs for TLS inspection)
            .danger_accept_invalid_certs(true);
        // NOTE: Don't set default user_agent here - let each service set it per request
        // This allows Claude Console to use "claude_code" as required

        // NOTE: We don't call .no_proxy() here - let reqwest use system proxy from env vars
        // This is required in Claude Code environment to access external networks

        let client = builder
            .build()
            .map_err(|e| AppError::InternalError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client })
    }

    /// Create a new HTTP client with custom proxy
    pub fn with_proxy(settings: &Settings, proxy_url: &str) -> Result<Self> {
        let timeout = Duration::from_millis(settings.server.request_timeout);

        let proxy = Proxy::all(proxy_url)
            .map_err(|e| AppError::ProxyError(format!("Failed to configure proxy: {}", e)))?;

        let client = Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            // NOTE: Don't set default user_agent - let each service set it per request
            .proxy(proxy)
            .build()
            .map_err(|e| AppError::InternalError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client })
    }

    /// Get the underlying reqwest client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Make a GET request
    pub async fn get(&self, url: &str) -> Result<reqwest::Response> {
        self.client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::UpstreamError(format!("GET request failed: {}", e)))
    }

    /// Make a POST request with JSON body
    pub async fn post_json<T: serde::Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<reqwest::Response> {
        self.client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::UpstreamError(format!("POST request failed: {}", e)))
    }

    /// Make a POST request with custom headers and JSON body
    pub async fn post_json_with_headers<T: serde::Serialize>(
        &self,
        url: &str,
        headers: reqwest::header::HeaderMap,
        body: &T,
    ) -> Result<reqwest::Response> {
        self.client
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::UpstreamError(format!("POST request failed: {}", e)))
    }

    /// Make a request with custom method, headers, and body
    pub async fn request(
        &self,
        method: reqwest::Method,
        url: &str,
        headers: Option<reqwest::header::HeaderMap>,
        body: Option<String>,
    ) -> Result<reqwest::Response> {
        let mut request = self.client.request(method, url);

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        if let Some(body) = body {
            request = request.body(body);
        }

        request
            .send()
            .await
            .map_err(|e| AppError::UpstreamError(format!("Request failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LoggingSettings, RedisSettings, SecuritySettings, ServerSettings};

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
                jwt_secret: "test_secret_key_minimum_32_chars_long".to_string(),
                encryption_key: "12345678901234567890123456789012".to_string(),
                api_key_prefix: "cr_".to_string(),
            },
            logging: LoggingSettings {
                level: "info".to_string(),
                format: "pretty".to_string(),
            },
        }
    }

    #[test]
    fn test_http_client_creation() {
        let settings = create_test_settings();
        let client = HttpClient::new(&settings);
        assert!(client.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_http_get_request() {
        let settings = create_test_settings();
        let client = HttpClient::new(&settings).expect("Failed to create HTTP client");

        let result = client.get("https://httpbin.org/get").await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.status().is_success());
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_http_post_json_request() {
        let settings = create_test_settings();
        let client = HttpClient::new(&settings).expect("Failed to create HTTP client");

        let body = serde_json::json!({
            "test": "data"
        });

        let result = client.post_json("https://httpbin.org/post", &body).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.status().is_success());
    }
}
