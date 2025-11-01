use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::redis::RedisPool;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub redis: RedisPool,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub components: HealthComponents,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthComponents {
    pub redis: ComponentStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub status: String,
    pub message: Option<String>,
}

/// Health check handler
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<HealthResponse>) {
    // Check Redis connection
    let redis_status = match state.redis.ping().await {
        Ok(_) => ComponentStatus {
            status: "healthy".to_string(),
            message: None,
        },
        Err(e) => ComponentStatus {
            status: "unhealthy".to_string(),
            message: Some(format!("Redis ping failed: {}", e)),
        },
    };

    let overall_status = if redis_status.status == "healthy" {
        "healthy"
    } else {
        "degraded"
    };

    let status_code = if overall_status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: overall_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        components: HealthComponents {
            redis: redis_status,
        },
    };

    (status_code, Json(response))
}

/// Simple ping handler
pub async fn ping() -> &'static str {
    "pong"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            components: HealthComponents {
                redis: ComponentStatus {
                    status: "healthy".to_string(),
                    message: None,
                },
            },
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize");
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));
    }
}
