use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

/// Application error type
#[derive(Debug)]
pub enum AppError {
    // Configuration errors
    ConfigError(String),
    ValidationError(String),

    // Database errors
    RedisError(String),
    DatabaseError(String),

    // Authentication errors
    Unauthorized(String),
    Forbidden(String),
    InvalidApiKey(String),

    // Request errors
    BadRequest(String),
    NotFound(String),
    RateLimitExceeded(String),
    ConcurrencyLimitExceeded(String),
    NoAvailableAccounts(String),

    // External service errors
    UpstreamError(String),
    ProxyError(String),

    // Internal errors
    InternalError(String),
    TokenRefreshFailed(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::RedisError(msg) => write!(f, "Redis error: {}", msg),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            Self::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            Self::InvalidApiKey(msg) => write!(f, "Invalid API key: {}", msg),
            Self::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            Self::ConcurrencyLimitExceeded(msg) => write!(f, "Concurrency limit exceeded: {}", msg),
            Self::NoAvailableAccounts(msg) => write!(f, "No available accounts: {}", msg),
            Self::UpstreamError(msg) => write!(f, "Upstream error: {}", msg),
            Self::ProxyError(msg) => write!(f, "Proxy error: {}", msg),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
            Self::TokenRefreshFailed(msg) => write!(f, "Token refresh failed: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, error_type) = match &self {
            Self::ConfigError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "config_error",
            ),
            Self::ValidationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "validation_error",
            ),
            Self::RedisError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "redis_error",
            ),
            Self::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "database_error",
            ),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone(), "unauthorized"),
            Self::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone(), "forbidden"),
            Self::InvalidApiKey(msg) => (StatusCode::UNAUTHORIZED, msg.clone(), "invalid_api_key"),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone(), "bad_request"),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone(), "not_found"),
            Self::RateLimitExceeded(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                msg.clone(),
                "rate_limit_exceeded",
            ),
            Self::ConcurrencyLimitExceeded(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                msg.clone(),
                "concurrency_limit_exceeded",
            ),
            Self::NoAvailableAccounts(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                msg.clone(),
                "no_available_accounts",
            ),
            Self::UpstreamError(msg) => (StatusCode::BAD_GATEWAY, msg.clone(), "upstream_error"),
            Self::ProxyError(msg) => (StatusCode::BAD_GATEWAY, msg.clone(), "proxy_error"),
            Self::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "internal_error",
            ),
            Self::TokenRefreshFailed(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "token_refresh_failed",
            ),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": error_type,
                "status": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}

// Conversion implementations for common error types
impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        Self::ConfigError(err.to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        Self::RedisError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::UpstreamError(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        // Check if the underlying error is a Redis error
        if let Some(redis_err) = err.downcast_ref::<redis::RedisError>() {
            return Self::RedisError(redis_err.to_string());
        }
        Self::InternalError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::InternalError(format!("JSON serialization error: {}", err))
    }
}

/// Result type alias for application errors
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = AppError::Unauthorized("Invalid credentials".to_string());
        assert_eq!(error.to_string(), "Unauthorized: Invalid credentials");
    }
}
