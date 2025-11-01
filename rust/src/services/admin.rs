use anyhow::Result;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::utils::error::AppError;
use crate::RedisPool;

/// 管理员凭据 (存储在 data/init.json 和 Redis)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminCredentials {
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// 初始化数据结构 (data/init.json)
#[derive(Debug, Serialize, Deserialize)]
pub struct InitData {
    #[serde(rename = "initializedAt")]
    pub initialized_at: DateTime<Utc>,
    #[serde(rename = "adminUsername")]
    pub admin_username: String,
    #[serde(rename = "adminPassword")]
    pub admin_password: String, // 明文密码 (仅在 init.json 中)
    pub version: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // username
    pub role: String,     // "admin" or "user"
    pub exp: usize,       // expiration time (Unix timestamp)
    pub iat: usize,       // issued at (Unix timestamp)
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: String,
    pub user: UserInfo,
}

/// 用户信息
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub username: String,
    pub role: String,
}

/// 管理员认证服务
pub struct AdminService {
    redis: Arc<RedisPool>,
    jwt_secret: String,
}

impl AdminService {
    /// 创建新的管理员服务实例
    pub fn new(redis: Arc<RedisPool>, jwt_secret: String) -> Self {
        Self { redis, jwt_secret }
    }

    /// 从 data/init.json 加载管理员凭据
    ///
    /// 这是唯一的真实数据源，每次启动都会从文件读取并同步到 Redis
    pub async fn initialize_admin_from_file(&self) -> Result<(), AppError> {
        let init_file_path = Path::new("data/init.json");

        if !init_file_path.exists() {
            warn!("⚠️  No admin credentials found at data/init.json");
            warn!("   Please run setup to create initial admin credentials");
            return Ok(());
        }

        // 读取 init.json
        let file_content = fs::read_to_string(init_file_path)
            .map_err(|e| AppError::InternalError(format!("Failed to read init.json: {}", e)))?;

        let init_data: InitData = serde_json::from_str(&file_content).map_err(|e| {
            AppError::InternalError(format!("Failed to parse init.json: {}", e))
        })?;

        // 使用 Argon2 哈希密码 (替代 bcrypt)
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2
            .hash_password(init_data.admin_password.as_bytes(), &salt)
            .map_err(|e| AppError::InternalError(format!("Failed to hash password: {}", e)))?
            .to_string();

        // 创建管理员凭据
        let admin_credentials = AdminCredentials {
            username: init_data.admin_username.clone(),
            password_hash,
            created_at: init_data.initialized_at,
            last_login: None,
            updated_at: init_data.updated_at,
        };

        // 存储到 Redis (覆盖模式，确保与 init.json 同步)
        let mut conn = self.redis.get_connection().await?;

        let credentials_json = serde_json::to_string(&admin_credentials).map_err(|e| {
            AppError::InternalError(format!("Failed to serialize credentials: {}", e))
        })?;

        redis::cmd("SET")
            .arg("admin_credentials")
            .arg(credentials_json)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to store credentials: {}", e)))?;

        info!("✅ Admin credentials loaded from init.json (single source of truth)");
        info!("📋 Admin username: {}", admin_credentials.username);

        Ok(())
    }

    /// 验证管理员登录
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<LoginResponse, AppError> {
        // 从 Redis 获取管理员凭据
        let credentials = self.get_admin_credentials().await?;

        // 验证用户名
        if username != credentials.username {
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }

        // 验证密码
        let parsed_hash = PasswordHash::new(&credentials.password_hash).map_err(|e| {
            AppError::InternalError(format!("Failed to parse password hash: {}", e))
        })?;

        let argon2 = Argon2::default();
        argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized("Invalid credentials".to_string()))?;

        // 生成 JWT token
        let token = self.generate_token(username, "admin")?;

        // 更新最后登录时间
        self.update_last_login(username).await?;

        Ok(LoginResponse {
            success: true,
            token,
            user: UserInfo {
                username: username.to_string(),
                role: "admin".to_string(),
            },
        })
    }

    /// 生成 JWT token
    pub fn generate_token(&self, username: &str, role: &str) -> Result<String, AppError> {
        let now = Utc::now();
        let expiration = now + chrono::Duration::hours(24); // 24 小时有效期

        let claims = Claims {
            sub: username.to_string(),
            role: role.to_string(),
            exp: expiration.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::InternalError(format!("Failed to generate token: {}", e)))?;

        Ok(token)
    }

    /// 验证 JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims, AppError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| AppError::Unauthorized(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }

    /// 获取管理员凭据
    async fn get_admin_credentials(&self) -> Result<AdminCredentials, AppError> {
        let mut conn = self.redis.get_connection().await?;

        let credentials_json: String = redis::cmd("GET")
            .arg("admin_credentials")
            .query_async(&mut conn)
            .await
            .map_err(|_| {
                AppError::Unauthorized("Administrator credentials not found".to_string())
            })?;

        let credentials: AdminCredentials = serde_json::from_str(&credentials_json).map_err(
            |e| AppError::InternalError(format!("Failed to deserialize credentials: {}", e)),
        )?;

        Ok(credentials)
    }

    /// 更新最后登录时间
    async fn update_last_login(&self, username: &str) -> Result<(), AppError> {
        let mut credentials = self.get_admin_credentials().await?;

        credentials.last_login = Some(Utc::now());
        credentials.updated_at = Some(Utc::now());

        let credentials_json = serde_json::to_string(&credentials).map_err(|e| {
            AppError::InternalError(format!("Failed to serialize credentials: {}", e))
        })?;

        let mut conn = self.redis.get_connection().await?;

        redis::cmd("SET")
            .arg("admin_credentials")
            .arg(credentials_json)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to update last login: {}", e))
            })?;

        Ok(())
    }

    /// 创建初始管理员账户 (用于 CLI)
    pub async fn create_initial_admin(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(), AppError> {
        // 检查密码长度
        if password.len() < 8 {
            return Err(AppError::ValidationError(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        // 检查用户名长度
        if username.len() < 3 {
            return Err(AppError::ValidationError(
                "Username must be at least 3 characters".to_string(),
            ));
        }

        // 使用 Argon2 哈希密码
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::InternalError(format!("Failed to hash password: {}", e)))?
            .to_string();

        // 创建 init.json 数据
        let now = Utc::now();
        let init_data = InitData {
            initialized_at: now,
            admin_username: username.to_string(),
            admin_password: password.to_string(), // 明文密码存储在 init.json
            version: "1.0.0".to_string(),
            updated_at: Some(now),
        };

        // 确保 data 目录存在
        fs::create_dir_all("data").map_err(|e| {
            AppError::InternalError(format!("Failed to create data directory: {}", e))
        })?;

        // 写入 init.json
        let init_json = serde_json::to_string_pretty(&init_data).map_err(|e| {
            AppError::InternalError(format!("Failed to serialize init data: {}", e))
        })?;

        fs::write("data/init.json", init_json).map_err(|e| {
            AppError::InternalError(format!("Failed to write init.json: {}", e))
        })?;

        // 创建管理员凭据
        let admin_credentials = AdminCredentials {
            username: username.to_string(),
            password_hash,
            created_at: now,
            last_login: None,
            updated_at: Some(now),
        };

        // 存储到 Redis
        let credentials_json = serde_json::to_string(&admin_credentials).map_err(|e| {
            AppError::InternalError(format!("Failed to serialize credentials: {}", e))
        })?;

        let mut conn = self.redis.get_connection().await?;

        redis::cmd("SET")
            .arg("admin_credentials")
            .arg(credentials_json)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to store credentials: {}", e)))?;

        info!("✅ Admin account created: {}", username);
        info!("📝 Credentials saved to data/init.json");

        Ok(())
    }

    /// 重置管理员密码
    pub async fn reset_password(
        &self,
        username: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        // 检查密码长度
        if new_password.len() < 8 {
            return Err(AppError::ValidationError(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        // 验证用户名
        let credentials = self.get_admin_credentials().await?;
        if username != credentials.username {
            return Err(AppError::NotFound("Admin not found".to_string()));
        }

        // 哈希新密码
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| AppError::InternalError(format!("Failed to hash password: {}", e)))?
            .to_string();

        // 更新 init.json
        let init_file_path = Path::new("data/init.json");
        if init_file_path.exists() {
            let file_content = fs::read_to_string(init_file_path).map_err(|e| {
                AppError::InternalError(format!("Failed to read init.json: {}", e))
            })?;

            let mut init_data: InitData = serde_json::from_str(&file_content).map_err(|e| {
                AppError::InternalError(format!("Failed to parse init.json: {}", e))
            })?;

            init_data.admin_password = new_password.to_string();
            init_data.updated_at = Some(Utc::now());

            let init_json = serde_json::to_string_pretty(&init_data).map_err(|e| {
                AppError::InternalError(format!("Failed to serialize init data: {}", e))
            })?;

            fs::write(init_file_path, init_json).map_err(|e| {
                AppError::InternalError(format!("Failed to write init.json: {}", e))
            })?;
        }

        // 更新 Redis
        let updated_credentials = AdminCredentials {
            username: credentials.username,
            password_hash,
            created_at: credentials.created_at,
            last_login: credentials.last_login,
            updated_at: Some(Utc::now()),
        };

        let credentials_json = serde_json::to_string(&updated_credentials).map_err(|e| {
            AppError::InternalError(format!("Failed to serialize credentials: {}", e))
        })?;

        let mut conn = self.redis.get_connection().await?;

        redis::cmd("SET")
            .arg("admin_credentials")
            .arg(credentials_json)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to update credentials: {}", e))
            })?;

        info!("✅ Admin password reset for: {}", username);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_token_generation_and_verification() {
        let redis = Arc::new(RedisPool::new("redis://localhost:6379", 10).await.unwrap());
        let service = AdminService::new(redis, "test_secret_key_at_least_32_chars_long".to_string());

        let token = service
            .generate_token("admin", "admin")
            .expect("Failed to generate token");

        let claims = service
            .verify_token(&token)
            .expect("Failed to verify token");

        assert_eq!(claims.sub, "admin");
        assert_eq!(claims.role, "admin");
    }

    #[tokio::test]
    async fn test_invalid_token() {
        let redis = Arc::new(RedisPool::new("redis://localhost:6379", 10).await.unwrap());
        let service = AdminService::new(redis, "test_secret_key_at_least_32_chars_long".to_string());

        let result = service.verify_token("invalid_token");
        assert!(result.is_err());
    }
}
