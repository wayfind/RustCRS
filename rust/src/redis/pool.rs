use deadpool_redis::{Config, Connection, Pool, Runtime};
use redis::AsyncCommands;

use crate::config::Settings;
use crate::utils::{AppError, Result};

/// Redis connection pool wrapper
#[derive(Clone)]
pub struct RedisPool {
    pool: Pool,
}

impl RedisPool {
    /// Create a new Redis connection pool
    pub fn new(settings: &Settings) -> Result<Self> {
        let redis_url = settings.redis_url();

        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| AppError::RedisError(format!("Failed to create Redis pool: {}", e)))?;

        Ok(Self { pool })
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<Connection> {
        self.pool
            .get()
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get Redis connection: {}", e)))
    }

    /// Ping Redis to check connectivity
    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Redis ping failed: {}", e)))?;
        Ok(())
    }

    /// Get a value from Redis
    pub async fn get<T: redis::FromRedisValue>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.get_connection().await?;
        conn.get(key)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get key '{}': {}", key, e)))
    }

    /// Set a value in Redis
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.set(key, value)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to set key '{}': {}", key, e)))
    }

    /// Set a value with expiration
    pub async fn setex(&self, key: &str, value: &str, seconds: u64) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.set_ex(key, value, seconds)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to setex key '{}': {}", key, e)))
    }

    /// Delete a key
    pub async fn del(&self, key: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.del(key)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to delete key '{}': {}", key, e)))
    }

    /// Check if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        conn.exists(key)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to check key '{}': {}", key, e)))
    }

    /// Set expiration on a key
    pub async fn expire(&self, key: &str, seconds: i64) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        conn.expire(key, seconds)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to expire key '{}': {}", key, e)))
    }

    /// Get time to live for a key
    pub async fn ttl(&self, key: &str) -> Result<i64> {
        let mut conn = self.get_connection().await?;
        conn.ttl(key).await.map_err(|e| {
            AppError::RedisError(format!("Failed to get TTL for key '{}': {}", key, e))
        })
    }

    /// Increment a counter
    pub async fn incr(&self, key: &str) -> Result<i64> {
        let mut conn = self.get_connection().await?;
        conn.incr(key, 1)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to increment key '{}': {}", key, e)))
    }

    /// Increment a counter by a specific amount
    pub async fn incr_by(&self, key: &str, amount: i64) -> Result<i64> {
        let mut conn = self.get_connection().await?;
        conn.incr(key, amount).await.map_err(|e| {
            AppError::RedisError(format!(
                "Failed to increment key '{}' by {}: {}",
                key, amount, e
            ))
        })
    }

    /// Add member to a sorted set
    pub async fn zadd(&self, key: &str, score: f64, member: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.zadd(key, member, score)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to zadd to '{}': {}", key, e)))
    }

    /// Remove member from a sorted set
    pub async fn zrem(&self, key: &str, member: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.zrem(key, member)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to zrem from '{}': {}", key, e)))
    }

    /// Get sorted set cardinality (count)
    pub async fn zcard(&self, key: &str) -> Result<i64> {
        let mut conn = self.get_connection().await?;
        conn.zcard(key)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to zcard for '{}': {}", key, e)))
    }

    /// Remove members from sorted set by score range
    pub async fn zremrangebyscore(&self, key: &str, min: f64, max: f64) -> Result<i64> {
        let mut conn = self.get_connection().await?;
        conn.zrembyscore(key, min, max).await.map_err(|e| {
            AppError::RedisError(format!("Failed to zremrangebyscore for '{}': {}", key, e))
        })
    }

    /// Get hash field value
    pub async fn hget<T: redis::FromRedisValue>(
        &self,
        key: &str,
        field: &str,
    ) -> Result<Option<T>> {
        let mut conn = self.get_connection().await?;
        conn.hget(key, field).await.map_err(|e| {
            AppError::RedisError(format!(
                "Failed to hget field '{}' from '{}': {}",
                field, key, e
            ))
        })
    }

    /// Set hash field value
    pub async fn hset(&self, key: &str, field: &str, value: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.hset(key, field, value).await.map_err(|e| {
            AppError::RedisError(format!(
                "Failed to hset field '{}' in '{}': {}",
                field, key, e
            ))
        })
    }

    /// Get all hash fields and values
    pub async fn hgetall(&self, key: &str) -> Result<Vec<(String, String)>> {
        let mut conn = self.get_connection().await?;
        conn.hgetall(key)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to hgetall for '{}': {}", key, e)))
    }

    /// Get keys matching pattern using KEYS command
    /// Note: In production with large datasets, consider using SCAN instead
    pub async fn keys(&self, pattern: &str) -> Result<Vec<String>> {
        let mut conn = self.get_connection().await?;
        redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                AppError::RedisError(format!(
                    "Failed to get keys with pattern '{}': {}",
                    pattern, e
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running Redis instance
    // You can skip them with: cargo test -- --skip redis

    #[tokio::test]
    #[ignore] // Run with: cargo test -- --ignored
    async fn test_redis_ping() {
        let settings = Settings::new().expect("Failed to load settings");
        let pool = RedisPool::new(&settings).expect("Failed to create Redis pool");

        let result = pool.ping().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_set_get() {
        let settings = Settings::new().expect("Failed to load settings");
        let pool = RedisPool::new(&settings).expect("Failed to create Redis pool");

        let test_key = "test:rust:key";
        let test_value = "test_value";

        // Set value
        pool.set(test_key, test_value).await.expect("Failed to set");

        // Get value
        let result: Option<String> = pool.get(test_key).await.expect("Failed to get");
        assert_eq!(result, Some(test_value.to_string()));

        // Cleanup
        pool.del(test_key).await.expect("Failed to delete");
    }
}
