use crate::config::Settings;
use crate::models::api_key::{ApiKey, ApiKeyCreateOptions, ApiKeyUsageStats, ModelUsage};
use crate::redis::RedisPool;
use crate::utils::error::{AppError, Result};
use chrono::Utc;
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// API Key 服务
#[derive(Clone)]
pub struct ApiKeyService {
    redis: RedisPool,
    config: Settings,
}

impl ApiKeyService {
    /// 创建新的 API Key 服务实例
    pub fn new(redis: RedisPool, config: Settings) -> Self {
        Self { redis, config }
    }

    /// 生成随机 API Key
    ///
    /// 格式: {prefix}_{32字节随机字符串}
    /// 默认前缀: cr_
    ///
    /// # 示例
    ///
    /// ```
    /// let key = service.generate_random_key();
    /// // 返回类似: "cr_1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p"
    /// ```
    fn generate_random_key(&self) -> String {
        let prefix = &self.config.security.api_key_prefix;

        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let random_str = hex::encode(random_bytes);

        format!("{}{}", prefix, random_str)
    }

    /// 计算 API Key 的 SHA-256 哈希值
    ///
    /// # 参数
    ///
    /// * `key` - 原始 API Key 字符串
    ///
    /// # 返回
    ///
    /// SHA-256 哈希的十六进制字符串
    fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// 生成新的 API Key
    ///
    /// # 参数
    ///
    /// * `options` - API Key 创建选项
    ///
    /// # 返回
    ///
    /// 返回元组 (原始key, ApiKey对象)
    /// 注意: 原始key只会在创建时返回一次,之后只存储哈希值
    ///
    /// # 示例
    ///
    /// ```
    /// let options = ApiKeyCreateOptions {
    ///     name: "My App".to_string(),
    ///     permissions: ApiKeyPermissions::All,
    ///     ..Default::default()
    /// };
    ///
    /// let (raw_key, api_key) = service.generate_key(options).await?;
    /// println!("保存此Key: {}", raw_key);  // cr_1a2b3c...
    /// println!("Key ID: {}", api_key.id);   // UUID
    /// ```
    pub async fn generate_key(&self, options: ApiKeyCreateOptions) -> Result<(String, ApiKey)> {
        // 生成随机key
        let raw_key = self.generate_random_key();

        // 计算哈希
        let key_hash = self.hash_key(&raw_key);

        // 生成UUID
        let id = Uuid::new_v4().to_string();

        // 创建时间
        let now = Utc::now();

        // 构建ApiKey对象
        let api_key = ApiKey {
            id: id.clone(),
            key: Some(raw_key.clone()), // 仅在创建时包含原始key
            key_hash: key_hash.clone(),
            name: options.name,
            description: options.description,
            icon: options.icon,
            created_at: now,
            updated_at: now,
            expires_at: options.expires_at,
            activated_at: None,
            last_used_at: None,
            is_active: options.is_active,
            is_deleted: false,
            deleted_at: None,
            deleted_by: None,
            deleted_by_type: None,
            permissions: options.permissions,
            token_limit: options.token_limit,
            concurrency_limit: options.concurrency_limit,
            rate_limit_window: options.rate_limit_window,
            rate_limit_requests: options.rate_limit_requests,
            rate_limit_cost: options.rate_limit_cost,
            daily_cost_limit: options.daily_cost_limit,
            total_cost_limit: options.total_cost_limit,
            weekly_opus_cost_limit: options.weekly_opus_cost_limit,
            enable_model_restriction: options.enable_model_restriction,
            restricted_models: options.restricted_models,
            enable_client_restriction: options.enable_client_restriction,
            allowed_clients: options.allowed_clients,
            tags: options.tags,
            expiration_mode: options.expiration_mode,
            activation_days: options.activation_days,
            activation_unit: options.activation_unit,
            claude_account_id: options.claude_account_id,
            claude_console_account_id: options.claude_console_account_id,
            gemini_account_id: options.gemini_account_id,
            openai_account_id: options.openai_account_id,
            azure_openai_account_id: options.azure_openai_account_id,
            bedrock_account_id: options.bedrock_account_id,
            droid_account_id: options.droid_account_id,
            user_id: options.user_id,
            created_by: options.created_by,
            created_by_type: options.created_by_type,
        };

        // 存储到Redis
        self.store_api_key(&api_key, &key_hash).await?;

        Ok((raw_key, api_key))
    }

    /// 存储 API Key 到 Redis
    ///
    /// 存储结构:
    /// - api_key:{id} -> JSON序列化的ApiKey (不含原始key)
    /// - api_key_hash:{hash} -> key_id (快速查找映射)
    async fn store_api_key(&self, api_key: &ApiKey, key_hash: &str) -> Result<()> {
        // 创建不含原始key的副本用于存储
        let mut stored_key = api_key.clone();
        stored_key.key = None; // 不存储原始key

        // 序列化
        let key_json = serde_json::to_string(&stored_key)
            .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

        // 存储到Redis
        let key_id_key = format!("api_key:{}", api_key.id);
        let hash_key = format!("api_key_hash:{}", key_hash);

        // 使用pipeline存储两个键
        self.redis.set(&key_id_key, &key_json).await?;
        self.redis.set(&hash_key, &api_key.id).await?;

        // 如果有过期时间,设置TTL
        if let Some(expires_at) = api_key.expires_at {
            let ttl = (expires_at - Utc::now()).num_seconds();
            if ttl > 0 {
                self.redis.expire(&key_id_key, ttl).await?;
                self.redis.expire(&hash_key, ttl).await?;
            }
        }

        Ok(())
    }

    /// 验证 API Key
    ///
    /// # 参数
    ///
    /// * `key` - 原始 API Key 字符串
    ///
    /// # 返回
    ///
    /// 如果验证成功,返回 ApiKey 对象
    ///
    /// # 错误
    ///
    /// - `ApiKeyNotFound` - Key 不存在
    /// - `ApiKeyInvalid` - Key 已失效
    /// - `ApiKeyExpired` - Key 已过期
    /// - `ApiKeyInactive` - Key 未激活
    pub async fn validate_key(&self, key: &str) -> Result<ApiKey> {
        // 计算哈希
        let key_hash = self.hash_key(key);
        let hash_key = format!("api_key_hash:{}", key_hash);

        // 查找key_id
        let key_id: String = self
            .redis
            .get(&hash_key)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid API Key".to_string()))?;

        // 获取API Key数据
        let key_id_key = format!("api_key:{}", key_id);
        let key_json: String = self
            .redis
            .get(&key_id_key)
            .await?
            .ok_or_else(|| AppError::Unauthorized("API Key not found".to_string()))?;

        // 反序列化
        let api_key: ApiKey = serde_json::from_str(&key_json)
            .map_err(|e| AppError::InternalError(format!("反序列化失败: {}", e)))?;

        // 验证状态
        if api_key.is_deleted {
            return Err(AppError::Unauthorized(
                "API Key has been deleted".to_string(),
            ));
        }

        if !api_key.is_active {
            return Err(AppError::Unauthorized("API Key is inactive".to_string()));
        }

        // 验证过期时间
        if let Some(expires_at) = api_key.expires_at {
            if Utc::now() > expires_at {
                return Err(AppError::Unauthorized("API Key has expired".to_string()));
            }
        }

        Ok(api_key)
    }

    /// 检查权限
    ///
    /// # 参数
    ///
    /// * `api_key` - API Key 对象
    /// * `service` - 服务名称 ("claude", "gemini", "openai", "droid")
    ///
    /// # 返回
    ///
    /// 如果有权限返回 true,否则返回错误
    pub fn check_permissions(&self, api_key: &ApiKey, service: &str) -> Result<bool> {
        let has_permission = match service.to_lowercase().as_str() {
            "claude" => api_key.permissions.can_access_claude(),
            "gemini" => api_key.permissions.can_access_gemini(),
            "openai" => api_key.permissions.can_access_openai(),
            "droid" => api_key.permissions.can_access_droid(),
            _ => false,
        };

        if !has_permission {
            return Err(AppError::Forbidden(format!(
                "API Key does not have permission to access {} service",
                service
            )));
        }

        Ok(true)
    }

    /// 获取单个 API Key
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID (UUID)
    ///
    /// # 返回
    ///
    /// 如果找到,返回 ApiKey 对象
    pub async fn get_key(&self, key_id: &str) -> Result<ApiKey> {
        let key = format!("api_key:{}", key_id);
        let key_json: String = self
            .redis
            .get(&key)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("API Key {} not found", key_id)))?;

        let api_key: ApiKey = serde_json::from_str(&key_json)
            .map_err(|e| AppError::InternalError(format!("反序列化失败: {}", e)))?;

        Ok(api_key)
    }

    /// 获取所有 API Keys
    ///
    /// # 参数
    ///
    /// * `include_deleted` - 是否包含已删除的 Keys
    ///
    /// # 返回
    ///
    /// 返回所有 API Keys 的向量
    pub async fn get_all_keys(&self, include_deleted: bool) -> Result<Vec<ApiKey>> {
        // 使用 Redis SCAN 命令获取所有 api_key:* 键
        // 注意: 这里简化实现,实际生产环境应该使用分页
        let pattern = "api_key:*";
        let keys = self.redis.keys(pattern).await?;

        let mut api_keys = Vec::new();
        for key in keys {
            if let Some(key_json) = self.redis.get::<String>(&key).await? {
                if let Ok(api_key) = serde_json::from_str::<ApiKey>(&key_json) {
                    // 根据 include_deleted 参数过滤
                    if include_deleted || !api_key.is_deleted {
                        api_keys.push(api_key);
                    }
                }
            }
        }

        Ok(api_keys)
    }

    /// 更新 API Key
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    /// * `name` - 新名称 (可选)
    /// * `is_active` - 新激活状态 (可选)
    ///
    /// # 返回
    ///
    /// 返回更新后的 ApiKey 对象
    pub async fn update_key(
        &self,
        key_id: &str,
        name: Option<String>,
        is_active: Option<bool>,
    ) -> Result<ApiKey> {
        // 获取现有 Key
        let mut api_key = self.get_key(key_id).await?;

        // 检查是否已删除
        if api_key.is_deleted {
            return Err(AppError::BadRequest(
                "Cannot update deleted API Key".to_string(),
            ));
        }

        // 应用更新
        if let Some(new_name) = name {
            api_key.name = new_name;
        }

        if let Some(new_is_active) = is_active {
            api_key.is_active = new_is_active;
        }

        // 更新时间戳
        api_key.updated_at = Utc::now();

        // 保存到 Redis
        let key = format!("api_key:{}", key_id);
        let key_json = serde_json::to_string(&api_key)
            .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

        self.redis.set(&key, &key_json).await?;

        Ok(api_key)
    }

    /// 删除 API Key (软删除)
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    /// * `deleted_by` - 删除者标识
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    pub async fn delete_key(&self, key_id: &str, deleted_by: &str) -> Result<()> {
        // 获取现有 Key
        let mut api_key = self.get_key(key_id).await?;

        // 检查是否已删除
        if api_key.is_deleted {
            return Err(AppError::BadRequest("API Key already deleted".to_string()));
        }

        // 标记为已删除
        api_key.is_deleted = true;
        api_key.deleted_at = Some(Utc::now());
        api_key.deleted_by = Some(deleted_by.to_string());
        api_key.updated_at = Utc::now();

        // 保存到 Redis
        let key = format!("api_key:{}", key_id);
        let key_json = serde_json::to_string(&api_key)
            .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

        self.redis.set(&key, &key_json).await?;

        Ok(())
    }

    /// 恢复已删除的 API Key
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    /// * `restored_by` - 恢复者标识
    ///
    /// # 返回
    ///
    /// 返回恢复后的 ApiKey 对象
    pub async fn restore_key(&self, key_id: &str, _restored_by: &str) -> Result<ApiKey> {
        // 获取现有 Key
        let mut api_key = self.get_key(key_id).await?;

        // 检查是否已删除
        if !api_key.is_deleted {
            return Err(AppError::BadRequest("API Key is not deleted".to_string()));
        }

        // 恢复
        api_key.is_deleted = false;
        api_key.deleted_at = None;
        api_key.deleted_by = None;
        api_key.deleted_by_type = None;
        api_key.updated_at = Utc::now();

        // 保存到 Redis
        let key = format!("api_key:{}", key_id);
        let key_json = serde_json::to_string(&api_key)
            .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

        self.redis.set(&key, &key_json).await?;

        Ok(api_key)
    }

    /// 永久删除 API Key
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    ///
    /// # 警告
    ///
    /// 此操作不可逆!将完全删除 Redis 中的所有相关数据
    pub async fn permanent_delete(&self, key_id: &str) -> Result<()> {
        // 获取 Key 以验证存在性和获取哈希
        let api_key = self.get_key(key_id).await?;

        // 删除主键
        let key = format!("api_key:{}", key_id);
        self.redis.del(&key).await?;

        // 删除哈希映射
        let hash_key = format!("api_key_hash:{}", api_key.key_hash);
        self.redis.del(&hash_key).await?;

        // TODO: 删除相关的使用统计数据
        // let usage_key = format!("api_key_usage:{}", key_id);
        // self.redis.del(&usage_key).await?;

        Ok(())
    }

    // ========================================
    // 使用统计相关方法
    // ========================================

    /// 记录 API Key 使用
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    /// * `model` - 使用的模型名称
    /// * `input_tokens` - 输入 token 数
    /// * `output_tokens` - 输出 token 数
    /// * `cache_creation_tokens` - 缓存创建 token 数
    /// * `cache_read_tokens` - 缓存读取 token 数
    /// * `cost` - 本次请求成本
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    ///
    /// 使用 Redis Hash + 原子操作实现并发安全的使用记录
    pub async fn record_usage(
        &self,
        key_id: &str,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_read_tokens: i64,
        cost: f64,
    ) -> Result<()> {
        let usage_key = format!("api_key_usage:{}", key_id);
        let model_key = format!("api_key_usage:model:{}:{}", key_id, model);

        let mut conn = self.redis.get_connection().await?;
        let now_timestamp = Utc::now().timestamp();

        // 使用 Redis Pipeline 执行原子操作
        // 1. 更新主要统计（使用 HINCRBY 和 HINCRBYFLOAT）
        let mut pipe = redis::pipe();
        pipe.atomic()
            .hincr(&usage_key, "total_requests", 1)
            .hincr(&usage_key, "total_input_tokens", input_tokens)
            .hincr(&usage_key, "total_output_tokens", output_tokens)
            .hincr(
                &usage_key,
                "total_cache_creation_tokens",
                cache_creation_tokens,
            )
            .hincr(&usage_key, "total_cache_read_tokens", cache_read_tokens)
            .cmd("HINCRBYFLOAT")
            .arg(&usage_key)
            .arg("total_cost")
            .arg(cost)
            .cmd("HINCRBYFLOAT")
            .arg(&usage_key)
            .arg("daily_cost")
            .arg(cost)
            .hset(&usage_key, "last_used_at", now_timestamp);

        // 如果是 Opus 模型，更新 weekly_opus_cost
        if model.to_lowercase().contains("opus") {
            pipe.cmd("HINCRBYFLOAT")
                .arg(&usage_key)
                .arg("weekly_opus_cost")
                .arg(cost);
        }

        // 2. 更新按模型的统计
        pipe.hincr(&model_key, "requests", 1)
            .hincr(&model_key, "input_tokens", input_tokens)
            .hincr(&model_key, "output_tokens", output_tokens)
            .hincr(&model_key, "cache_creation_tokens", cache_creation_tokens)
            .hincr(&model_key, "cache_read_tokens", cache_read_tokens)
            .cmd("HINCRBYFLOAT")
            .arg(&model_key)
            .arg("cost")
            .arg(cost);

        // 执行所有操作
        pipe.query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to record usage: {}", e)))?;

        // 更新 API Key 的 last_used_at (这个可以容忍最终一致性)
        let mut api_key = self.get_key(key_id).await?;
        api_key.last_used_at = Some(Utc::now());
        api_key.updated_at = Utc::now();

        let key = format!("api_key:{}", key_id);
        let key_json = serde_json::to_string(&api_key)
            .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

        self.redis.set(&key, &key_json).await?;

        Ok(())
    }

    /// 获取 API Key 使用统计
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    ///
    /// # 返回
    ///
    /// 返回使用统计数据,如果不存在则返回默认值
    ///
    /// 从 Redis Hash 读取统计数据
    pub async fn get_usage_stats(&self, key_id: &str) -> Result<ApiKeyUsageStats> {
        let usage_key = format!("api_key_usage:{}", key_id);
        let mut conn = self.redis.get_connection().await?;

        // 从 Redis Hash 获取所有字段
        let hash_data: std::collections::HashMap<String, String> = redis::cmd("HGETALL")
            .arg(&usage_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get usage stats: {}", e)))?;

        // 如果 Hash 为空，返回默认值
        if hash_data.is_empty() {
            return Ok(ApiKeyUsageStats::default());
        }

        // 解析 Hash 数据
        let total_requests = hash_data
            .get("total_requests")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let total_input_tokens = hash_data
            .get("total_input_tokens")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let total_output_tokens = hash_data
            .get("total_output_tokens")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let total_cache_creation_tokens = hash_data
            .get("total_cache_creation_tokens")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let total_cache_read_tokens = hash_data
            .get("total_cache_read_tokens")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let total_cost = hash_data
            .get("total_cost")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);

        let daily_cost = hash_data
            .get("daily_cost")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);

        let weekly_opus_cost = hash_data
            .get("weekly_opus_cost")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);

        let last_used_at = hash_data
            .get("last_used_at")
            .and_then(|v| v.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

        // 获取按模型的统计
        let model_pattern = format!("api_key_usage:model:{}:*", key_id);
        let model_keys: Vec<String> = redis::cmd("KEYS")
            .arg(&model_pattern)
            .query_async(&mut conn)
            .await
            .unwrap_or_default();

        let mut usage_by_model = std::collections::HashMap::new();
        for model_key in model_keys {
            // 从 key 中提取模型名
            if let Some(model_name) = model_key.split(':').next_back() {
                let model_hash: std::collections::HashMap<String, String> = redis::cmd("HGETALL")
                    .arg(&model_key)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or_default();

                if !model_hash.is_empty() {
                    let model_usage = ModelUsage {
                        requests: model_hash
                            .get("requests")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                        input_tokens: model_hash
                            .get("input_tokens")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                        output_tokens: model_hash
                            .get("output_tokens")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                        cache_creation_tokens: model_hash
                            .get("cache_creation_tokens")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                        cache_read_tokens: model_hash
                            .get("cache_read_tokens")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                        cost: model_hash
                            .get("cost")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0.0),
                    };
                    usage_by_model.insert(model_name.to_string(), model_usage);
                }
            }
        }

        Ok(ApiKeyUsageStats {
            total_requests,
            total_input_tokens,
            total_output_tokens,
            total_cache_creation_tokens,
            total_cache_read_tokens,
            total_cost,
            daily_cost,
            weekly_opus_cost,
            last_used_at,
            usage_by_model,
        })
    }

    /// 检查成本限制
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    /// * `estimated_cost` - 预估成本
    ///
    /// # 返回
    ///
    /// 如果超过限制返回 Err,否则返回 Ok(())
    pub async fn check_cost_limits(&self, key_id: &str, estimated_cost: f64) -> Result<()> {
        let api_key = self.get_key(key_id).await?;
        let stats = self.get_usage_stats(key_id).await?;

        // 检查总成本限制
        if api_key.total_cost_limit > 0.0 {
            let new_total = stats.total_cost + estimated_cost;
            if new_total > api_key.total_cost_limit {
                return Err(AppError::RateLimitExceeded(format!(
                    "Total cost limit exceeded: {} > {}",
                    new_total, api_key.total_cost_limit
                )));
            }
        }

        // 检查每日成本限制
        if api_key.daily_cost_limit > 0.0 {
            let new_daily = stats.daily_cost + estimated_cost;
            if new_daily > api_key.daily_cost_limit {
                return Err(AppError::RateLimitExceeded(format!(
                    "Daily cost limit exceeded: {} > {}",
                    new_daily, api_key.daily_cost_limit
                )));
            }
        }

        // 检查每周 Opus 成本限制 (假设模型名包含 "opus")
        if api_key.weekly_opus_cost_limit > 0.0 {
            let new_weekly_opus = stats.weekly_opus_cost + estimated_cost;
            if new_weekly_opus > api_key.weekly_opus_cost_limit {
                return Err(AppError::RateLimitExceeded(format!(
                    "Weekly Opus cost limit exceeded: {} > {}",
                    new_weekly_opus, api_key.weekly_opus_cost_limit
                )));
            }
        }

        Ok(())
    }

    /// 检查速率限制
    ///
    /// # 参数
    ///
    /// * `api_key` - API Key 对象
    ///
    /// # 返回
    ///
    /// 如果超过速率限制返回 Err,否则返回 Ok(())
    pub async fn check_rate_limit(&self, api_key: &ApiKey) -> Result<()> {
        // 如果没有设置速率限制,直接返回
        if api_key.rate_limit_requests.is_none() || api_key.rate_limit_window.is_none() {
            return Ok(());
        }

        let window = api_key.rate_limit_window.unwrap();
        let max_requests = api_key.rate_limit_requests.unwrap();

        let request_count_key = format!("rate_limit:requests:{}", api_key.id);
        let window_start_key = format!("rate_limit:window_start:{}", api_key.id);

        let mut conn = self.redis.get_connection().await?;

        // 获取当前窗口开始时间
        let window_start: Option<i64> = redis::cmd("GET")
            .arg(&window_start_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get window start: {}", e)))?;

        let now = Utc::now().timestamp();
        let mut current_window_start = window_start.unwrap_or(now);

        // 如果窗口过期,重置
        if now - current_window_start >= window {
            current_window_start = now;
            // 重置窗口开始时间和请求计数
            redis::cmd("SET")
                .arg(&window_start_key)
                .arg(current_window_start)
                .query_async::<_, ()>(&mut conn)
                .await
                .map_err(|e| {
                    AppError::RedisError(format!("Failed to reset window start: {}", e))
                })?;

            redis::cmd("SET")
                .arg(&request_count_key)
                .arg(0)
                .query_async::<_, ()>(&mut conn)
                .await
                .map_err(|e| {
                    AppError::RedisError(format!("Failed to reset request count: {}", e))
                })?;
        }

        // 获取当前请求计数
        let current_count: i64 = redis::cmd("GET")
            .arg(&request_count_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(0);

        // 检查是否超过限制
        if current_count >= max_requests {
            return Err(AppError::RateLimitExceeded(format!(
                "Rate limit exceeded: {} requests in {} seconds",
                max_requests, window
            )));
        }

        // 增加请求计数
        redis::cmd("INCR")
            .arg(&request_count_key)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| {
                AppError::RedisError(format!("Failed to increment request count: {}", e))
            })?;

        // 设置过期时间
        redis::cmd("EXPIRE")
            .arg(&request_count_key)
            .arg(window)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to set expiry: {}", e)))?;

        Ok(())
    }

    /// 增加并发计数
    ///
    /// # 参数
    ///
    /// * `api_key` - API Key 对象
    /// * `request_id` - 请求 ID
    ///
    /// # 返回
    ///
    /// 如果超过并发限制返回 Err,否则返回 Ok(())
    pub async fn increment_concurrency(&self, api_key: &ApiKey, request_id: &str) -> Result<()> {
        // 如果没有设置并发限制,直接返回
        if api_key.concurrency_limit == 0 {
            return Ok(());
        }

        let key = format!("concurrency:{}", api_key.id);
        let ttl_seconds = 600; // 10分钟过期
        let expiry_time = Utc::now().timestamp_millis() + (ttl_seconds as i64 * 1000);

        let mut conn = self.redis.get_connection().await?;

        // 清理过期的并发记录
        let now = Utc::now().timestamp_millis();
        redis::cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg("-inf")
            .arg(now)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to cleanup concurrency: {}", e)))?;

        // 检查当前并发数
        let current_count: i64 = redis::cmd("ZCARD")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to get concurrency count: {}", e)))?;

        if current_count >= api_key.concurrency_limit {
            return Err(AppError::RateLimitExceeded(format!(
                "Concurrency limit exceeded: {} concurrent requests",
                api_key.concurrency_limit
            )));
        }

        // 添加到 Sorted Set
        redis::cmd("ZADD")
            .arg(&key)
            .arg(expiry_time)
            .arg(request_id)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to increment concurrency: {}", e)))?;

        // 设置 key 过期时间
        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(ttl_seconds + 60)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| {
                AppError::RedisError(format!("Failed to set concurrency expiry: {}", e))
            })?;

        Ok(())
    }

    /// 减少并发计数
    ///
    /// # 参数
    ///
    /// * `api_key` - API Key 对象
    /// * `request_id` - 请求 ID
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    pub async fn decrement_concurrency(&self, api_key: &ApiKey, request_id: &str) -> Result<()> {
        let key = format!("concurrency:{}", api_key.id);
        let mut conn = self.redis.get_connection().await?;

        redis::cmd("ZREM")
            .arg(&key)
            .arg(request_id)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to decrement concurrency: {}", e)))?;

        Ok(())
    }

    /// 重置每日统计
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    pub async fn reset_daily_stats(&self, key_id: &str) -> Result<()> {
        let usage_key = format!("api_key_usage:{}", key_id);
        let mut conn = self.redis.get_connection().await?;

        // 使用 HSET 将 daily_cost 重置为 0
        redis::cmd("HSET")
            .arg(&usage_key)
            .arg("daily_cost")
            .arg("0")
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to reset daily stats: {}", e)))?;

        Ok(())
    }

    /// 重置每周统计
    ///
    /// # 参数
    ///
    /// * `key_id` - API Key ID
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())
    pub async fn reset_weekly_stats(&self, key_id: &str) -> Result<()> {
        let usage_key = format!("api_key_usage:{}", key_id);
        let mut conn = self.redis.get_connection().await?;

        // 使用 HSET 将 weekly_opus_cost 重置为 0
        redis::cmd("HSET")
            .arg(&usage_key)
            .arg("weekly_opus_cost")
            .arg("0")
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Failed to reset weekly stats: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::api_key::{ApiKeyPermissions, ExpirationMode};

    // Helper function to create a mock service for testing
    fn create_mock_service() -> (ApiKeyService, Settings) {
        // Note: These tests are for logic validation only
        // Integration tests with actual Redis are in separate files
        let config = Settings::new().expect("Failed to create test config");
        // We can't create a real RedisPool without Redis connection
        // So we'll skip Redis-dependent tests here and move them to integration tests
        let redis = RedisPool::new(&config).expect("Failed to create Redis pool for testing");
        let service = ApiKeyService::new(redis, config.clone());
        (service, config)
    }

    #[test]
    fn test_hash_key_consistency() {
        let (service, _config) = create_mock_service();

        let key = "cr_test123456";
        let hash1 = service.hash_key(key);
        let hash2 = service.hash_key(key);

        assert_eq!(hash1, hash2, "相同key应该产生相同哈希");
        assert_eq!(hash1.len(), 64, "SHA-256应该产生64字符十六进制");
    }

    #[test]
    fn test_hash_key_different_inputs() {
        let (service, _config) = create_mock_service();

        let hash1 = service.hash_key("cr_key1");
        let hash2 = service.hash_key("cr_key2");

        assert_ne!(hash1, hash2, "不同key应该产生不同哈希");
    }

    #[test]
    fn test_generate_random_key_format() {
        let (service, _config) = create_mock_service();

        let key = service.generate_random_key();

        assert!(key.starts_with("cr_"), "应该以cr_开头");
        assert_eq!(key.len(), 3 + 64, "应该是前缀+64字符随机串");
    }

    #[test]
    fn test_generate_random_key_uniqueness() {
        let (service, _config) = create_mock_service();

        let key1 = service.generate_random_key();
        let key2 = service.generate_random_key();

        assert_ne!(key1, key2, "连续生成的key应该不同");
    }

    #[test]
    fn test_check_permissions_all() {
        let (service, _config) = create_mock_service();

        let api_key = ApiKey {
            id: "test".to_string(),
            key: None,
            key_hash: "hash".to_string(),
            name: "test".to_string(),
            description: None,
            icon: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            activated_at: None,
            last_used_at: None,
            is_active: true,
            is_deleted: false,
            deleted_at: None,
            deleted_by: None,
            deleted_by_type: None,
            permissions: ApiKeyPermissions::All,
            token_limit: 0,
            concurrency_limit: 0,
            rate_limit_window: None,
            rate_limit_requests: None,
            rate_limit_cost: None,
            daily_cost_limit: 0.0,
            total_cost_limit: 0.0,
            weekly_opus_cost_limit: 0.0,
            enable_model_restriction: false,
            restricted_models: Vec::new(),
            enable_client_restriction: false,
            allowed_clients: Vec::new(),
            tags: Vec::new(),
            expiration_mode: ExpirationMode::Fixed,
            activation_days: 0,
            activation_unit: crate::models::api_key::ActivationUnit::Days,
            claude_account_id: None,
            claude_console_account_id: None,
            gemini_account_id: None,
            openai_account_id: None,
            azure_openai_account_id: None,
            bedrock_account_id: None,
            droid_account_id: None,
            user_id: None,
            created_by: None,
            created_by_type: None,
        };

        assert!(service.check_permissions(&api_key, "claude").is_ok());
        assert!(service.check_permissions(&api_key, "gemini").is_ok());
        assert!(service.check_permissions(&api_key, "openai").is_ok());
        assert!(service.check_permissions(&api_key, "droid").is_ok());
    }

    #[test]
    fn test_check_permissions_claude_only() {
        let (service, _config) = create_mock_service();

        let mut api_key = ApiKey {
            id: "test".to_string(),
            key: None,
            key_hash: "hash".to_string(),
            name: "test".to_string(),
            description: None,
            icon: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            activated_at: None,
            last_used_at: None,
            is_active: true,
            is_deleted: false,
            deleted_at: None,
            deleted_by: None,
            deleted_by_type: None,
            permissions: ApiKeyPermissions::Claude,
            token_limit: 0,
            concurrency_limit: 0,
            rate_limit_window: None,
            rate_limit_requests: None,
            rate_limit_cost: None,
            daily_cost_limit: 0.0,
            total_cost_limit: 0.0,
            weekly_opus_cost_limit: 0.0,
            enable_model_restriction: false,
            restricted_models: Vec::new(),
            enable_client_restriction: false,
            allowed_clients: Vec::new(),
            tags: Vec::new(),
            expiration_mode: ExpirationMode::Fixed,
            activation_days: 0,
            activation_unit: crate::models::api_key::ActivationUnit::Days,
            claude_account_id: None,
            claude_console_account_id: None,
            gemini_account_id: None,
            openai_account_id: None,
            azure_openai_account_id: None,
            bedrock_account_id: None,
            droid_account_id: None,
            user_id: None,
            created_by: None,
            created_by_type: None,
        };

        api_key.permissions = ApiKeyPermissions::Claude;

        assert!(service.check_permissions(&api_key, "claude").is_ok());
        assert!(service.check_permissions(&api_key, "gemini").is_err());
        assert!(service.check_permissions(&api_key, "openai").is_err());
        assert!(service.check_permissions(&api_key, "droid").is_err());
    }

    #[test]
    fn test_check_permissions_gemini() {
        let (service, _config) = create_mock_service();

        let mut api_key = ApiKey {
            id: "test".to_string(),
            key: None,
            key_hash: "hash".to_string(),
            name: "test".to_string(),
            description: None,
            icon: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            activated_at: None,
            last_used_at: None,
            is_active: true,
            is_deleted: false,
            deleted_at: None,
            deleted_by: None,
            deleted_by_type: None,
            permissions: ApiKeyPermissions::Gemini,
            token_limit: 0,
            concurrency_limit: 0,
            rate_limit_window: None,
            rate_limit_requests: None,
            rate_limit_cost: None,
            daily_cost_limit: 0.0,
            total_cost_limit: 0.0,
            weekly_opus_cost_limit: 0.0,
            enable_model_restriction: false,
            restricted_models: Vec::new(),
            enable_client_restriction: false,
            allowed_clients: Vec::new(),
            tags: Vec::new(),
            expiration_mode: ExpirationMode::Fixed,
            activation_days: 0,
            activation_unit: crate::models::api_key::ActivationUnit::Days,
            claude_account_id: None,
            claude_console_account_id: None,
            gemini_account_id: None,
            openai_account_id: None,
            azure_openai_account_id: None,
            bedrock_account_id: None,
            droid_account_id: None,
            user_id: None,
            created_by: None,
            created_by_type: None,
        };

        api_key.permissions = ApiKeyPermissions::Gemini;

        assert!(service.check_permissions(&api_key, "gemini").is_ok());
        assert!(service.check_permissions(&api_key, "claude").is_err());
        assert!(service.check_permissions(&api_key, "openai").is_err());
    }

    #[test]
    fn test_check_permissions_openai() {
        let (service, _config) = create_mock_service();

        let api_key = ApiKey {
            id: "test".to_string(),
            key: None,
            key_hash: "hash".to_string(),
            name: "test".to_string(),
            description: None,
            icon: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            activated_at: None,
            last_used_at: None,
            is_active: true,
            is_deleted: false,
            deleted_at: None,
            deleted_by: None,
            deleted_by_type: None,
            permissions: ApiKeyPermissions::OpenAI,
            token_limit: 0,
            concurrency_limit: 0,
            rate_limit_window: None,
            rate_limit_requests: None,
            rate_limit_cost: None,
            daily_cost_limit: 0.0,
            total_cost_limit: 0.0,
            weekly_opus_cost_limit: 0.0,
            enable_model_restriction: false,
            restricted_models: Vec::new(),
            enable_client_restriction: false,
            allowed_clients: Vec::new(),
            tags: Vec::new(),
            expiration_mode: ExpirationMode::Fixed,
            activation_days: 0,
            activation_unit: crate::models::api_key::ActivationUnit::Days,
            claude_account_id: None,
            claude_console_account_id: None,
            gemini_account_id: None,
            openai_account_id: None,
            azure_openai_account_id: None,
            bedrock_account_id: None,
            droid_account_id: None,
            user_id: None,
            created_by: None,
            created_by_type: None,
        };

        assert!(service.check_permissions(&api_key, "openai").is_ok());
        assert!(service.check_permissions(&api_key, "claude").is_err());
        assert!(service.check_permissions(&api_key, "gemini").is_err());
    }

    #[test]
    fn test_key_hash_format() {
        let (service, _config) = create_mock_service();

        let key = "cr_test_key_12345";
        let hash = service.hash_key(key);

        // SHA-256 produces 64-character hex string
        assert_eq!(hash.len(), 64);

        // Verify it's all hexadecimal characters
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_key_with_prefix() {
        let (service, config) = create_mock_service();

        let key = service.generate_random_key();
        let prefix = &config.security.api_key_prefix;

        assert!(key.starts_with(prefix));
    }

    #[test]
    fn test_permission_variants() {
        let permissions = vec![
            ApiKeyPermissions::All,
            ApiKeyPermissions::Claude,
            ApiKeyPermissions::Gemini,
            ApiKeyPermissions::OpenAI,
            ApiKeyPermissions::Droid,
        ];

        // 确保所有枚举值都是有效的
        for perm in permissions {
            let json = serde_json::to_value(&perm).unwrap();
            assert!(json.is_string());
        }
    }

    #[test]
    fn test_api_key_active_status() {
        let mut api_key = ApiKey {
            id: "test".to_string(),
            key: None,
            key_hash: "hash".to_string(),
            name: "test".to_string(),
            description: None,
            icon: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            activated_at: None,
            last_used_at: None,
            is_active: true,
            is_deleted: false,
            deleted_at: None,
            deleted_by: None,
            deleted_by_type: None,
            permissions: ApiKeyPermissions::All,
            token_limit: 0,
            concurrency_limit: 0,
            rate_limit_window: None,
            rate_limit_requests: None,
            rate_limit_cost: None,
            daily_cost_limit: 0.0,
            total_cost_limit: 0.0,
            weekly_opus_cost_limit: 0.0,
            enable_model_restriction: false,
            restricted_models: Vec::new(),
            enable_client_restriction: false,
            allowed_clients: Vec::new(),
            tags: Vec::new(),
            expiration_mode: ExpirationMode::Fixed,
            activation_days: 0,
            activation_unit: crate::models::api_key::ActivationUnit::Days,
            claude_account_id: None,
            claude_console_account_id: None,
            gemini_account_id: None,
            openai_account_id: None,
            azure_openai_account_id: None,
            bedrock_account_id: None,
            droid_account_id: None,
            user_id: None,
            created_by: None,
            created_by_type: None,
        };

        assert!(api_key.is_active);
        assert!(!api_key.is_deleted);

        api_key.is_active = false;
        assert!(!api_key.is_active);

        api_key.is_deleted = true;
        assert!(api_key.is_deleted);
    }

    #[test]
    fn test_expiration_mode_variants() {
        let fixed_mode = ExpirationMode::Fixed;
        let activation_mode = ExpirationMode::Activation;

        let fixed_json = serde_json::to_value(&fixed_mode).unwrap();
        let activation_json = serde_json::to_value(&activation_mode).unwrap();

        assert_eq!(fixed_json, "fixed");
        assert_eq!(activation_json, "activation");
    }
}
