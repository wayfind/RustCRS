use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    pub redis: RedisSettings,
    pub security: SecuritySettings,
    pub logging: LoggingSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub request_timeout: u64, // milliseconds
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisSettings {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub db: u8,
    pub pool_size: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecuritySettings {
    pub jwt_secret: String,
    pub encryption_key: String,
    pub api_key_prefix: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingSettings {
    pub level: String,
    pub format: String, // "json" or "pretty"
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let mut builder = Config::builder()
            // Start with default values
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.request_timeout", 600000)? // 10 minutes
            .set_default("redis.host", "localhost")?
            .set_default("redis.port", 6379)?
            .set_default("redis.db", 0)?
            .set_default("redis.pool_size", 10)?
            .set_default("security.jwt_secret", "")?
            .set_default("security.encryption_key", "")?
            .set_default("security.api_key_prefix", "cr_")?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?
            // Load config file if exists
            .add_source(File::with_name("config/config").required(false))
            .add_source(File::with_name(&format!("config/config.{}", run_mode)).required(false));

        // Manually override with environment variables (workaround for case sensitivity issues)
        // Security settings
        if let Ok(val) = env::var("CRS_SECURITY__JWT_SECRET") {
            builder = builder.set_override("security.jwt_secret", val)?;
        }
        if let Ok(val) = env::var("CRS_SECURITY__ENCRYPTION_KEY") {
            builder = builder.set_override("security.encryption_key", val)?;
        }
        if let Ok(val) = env::var("CRS_SECURITY__API_KEY_PREFIX") {
            builder = builder.set_override("security.api_key_prefix", val)?;
        }

        // Server settings
        if let Ok(val) = env::var("CRS_SERVER__HOST") {
            builder = builder.set_override("server.host", val)?;
        }
        if let Ok(val) = env::var("CRS_SERVER__PORT") {
            builder = builder.set_override("server.port", val)?;
        }
        if let Ok(val) = env::var("CRS_SERVER__REQUEST_TIMEOUT") {
            builder = builder.set_override("server.request_timeout", val)?;
        }

        // Redis settings
        if let Ok(val) = env::var("CRS_REDIS__HOST") {
            builder = builder.set_override("redis.host", val)?;
        }
        if let Ok(val) = env::var("CRS_REDIS__PORT") {
            builder = builder.set_override("redis.port", val)?;
        }
        if let Ok(val) = env::var("CRS_REDIS__PASSWORD") {
            builder = builder.set_override("redis.password", val)?;
        }
        if let Ok(val) = env::var("CRS_REDIS__DB") {
            builder = builder.set_override("redis.db", val)?;
        }
        if let Ok(val) = env::var("CRS_REDIS__POOL_SIZE") {
            builder = builder.set_override("redis.pool_size", val)?;
        }

        // Logging settings
        if let Ok(val) = env::var("CRS_LOGGING__LEVEL") {
            builder = builder.set_override("logging.level", val)?;
        }
        if let Ok(val) = env::var("CRS_LOGGING__FORMAT") {
            builder = builder.set_override("logging.format", val)?;
        }

        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate JWT secret length
        if self.security.jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters".to_string());
        }

        // Validate encryption key length (must be exactly 32 bytes)
        if self.security.encryption_key.len() != 32 {
            return Err("ENCRYPTION_KEY must be exactly 32 characters".to_string());
        }

        // Validate Redis pool size
        if self.redis.pool_size == 0 {
            return Err("Redis pool size must be greater than 0".to_string());
        }

        // Validate logging level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(format!(
                "Invalid logging level '{}'. Must be one of: {}",
                self.logging.level,
                valid_levels.join(", ")
            ));
        }

        Ok(())
    }

    /// Get Redis connection string
    pub fn redis_url(&self) -> String {
        match &self.redis.password {
            Some(password) => format!(
                "redis://:{}@{}:{}/{}",
                password, self.redis.host, self.redis.port, self.redis.db
            ),
            None => format!(
                "redis://{}:{}/{}",
                self.redis.host, self.redis.port, self.redis.db
            ),
        }
    }

    /// Get server bind address
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_settings_defaults() {
        // Set minimal required env vars for test (must be set in each test)
        env::set_var(
            "CRS_SECURITY__JWT_SECRET",
            "test_secret_key_minimum_32_chars_long",
        );
        env::set_var(
            "CRS_SECURITY__ENCRYPTION_KEY",
            "12345678901234567890123456789012",
        );

        let settings = Settings::new().expect("Failed to load settings");

        assert_eq!(settings.server.port, 8080);
        assert_eq!(settings.redis.host, "localhost");
        assert_eq!(settings.redis.port, 6379);

        // Clean up env vars
        env::remove_var("CRS_SECURITY__JWT_SECRET");
        env::remove_var("CRS_SECURITY__ENCRYPTION_KEY");
    }

    #[test]
    #[serial]
    fn test_redis_url_without_password() {
        env::set_var(
            "CRS_SECURITY__JWT_SECRET",
            "test_secret_key_minimum_32_chars_long",
        );
        env::set_var(
            "CRS_SECURITY__ENCRYPTION_KEY",
            "12345678901234567890123456789012",
        );

        let settings = Settings::new().expect("Failed to load settings");
        let url = settings.redis_url();

        assert!(url.starts_with("redis://"));
        assert!(!url.contains("@"));

        env::remove_var("CRS_SECURITY__JWT_SECRET");
        env::remove_var("CRS_SECURITY__ENCRYPTION_KEY");
    }

    #[test]
    fn test_validation_jwt_secret_too_short() {
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
                jwt_secret: "short".to_string(),
                encryption_key: "12345678901234567890123456789012".to_string(),
                api_key_prefix: "cr_".to_string(),
            },
            logging: LoggingSettings {
                level: "info".to_string(),
                format: "pretty".to_string(),
            },
        };

        assert!(settings.validate().is_err());
    }
}
