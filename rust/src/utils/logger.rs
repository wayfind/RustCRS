use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::Settings;

/// Initialize the logging system
pub fn init_logger(settings: &Settings) -> anyhow::Result<()> {
    // Parse log level from settings
    let log_level = &settings.logging.level;
    let log_format = &settings.logging.format;

    // Create environment filter with fallback to settings
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Configure format based on settings
    match log_format.as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().json().with_target(false).with_level(true))
                .init();
        }
        "pretty" | _ => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .with_target(false)
                        .with_level(true)
                        .with_ansi(true),
                )
                .init();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LoggingSettings, RedisSettings, SecuritySettings, ServerSettings};

    #[test]
    fn test_logger_initialization() {
        let settings = Settings {
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
        };

        // Note: This test can only be run once per process due to tracing subscriber initialization
        // In a real test suite, you would use a more sophisticated approach
        let result = init_logger(&settings);
        assert!(result.is_ok());
    }
}
