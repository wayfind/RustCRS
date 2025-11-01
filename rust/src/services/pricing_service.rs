// 定价服务
//
// 核心功能：
// - 从远程 GitHub 仓库下载模型定价数据
// - 本地缓存和 fallback 机制
// - SHA-256 哈希校验确保数据一致性
// - 定时更新（24小时）和哈希轮询（10分钟）
// - 文件监听和自动重载
// - 成本计算（支持 1M 上下文和 1h 缓存）

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// 模型定价数据
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelPricing {
    pub input_cost_per_token: f64,
    pub output_cost_per_token: f64,
    #[serde(default)]
    pub cache_creation_input_token_cost: Option<f64>,
    #[serde(default)]
    pub cache_read_input_token_cost: Option<f64>,
    #[serde(default)]
    pub litellm_provider: Option<String>,
}

/// 1M 上下文定价
#[derive(Debug, Clone)]
pub struct LongContextPricing {
    pub input: f64,
    pub output: f64,
}

/// 成本计算结果
#[derive(Debug, Clone)]
pub struct CostResult {
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_create_cost: f64,
    pub cache_read_cost: f64,
    pub ephemeral_5m_cost: f64,
    pub ephemeral_1h_cost: f64,
    pub total_cost: f64,
    pub has_pricing: bool,
    pub is_long_context_request: bool,
    pub pricing: PricingDetails,
}

/// 定价详情
#[derive(Debug, Clone)]
pub struct PricingDetails {
    pub input: f64,
    pub output: f64,
    pub cache_create: f64,
    pub cache_read: f64,
    pub ephemeral_1h: f64,
}

/// 使用量数据
#[derive(Debug, Clone)]
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation: Option<CacheCreation>,
}

/// 详细缓存创建数据
#[derive(Debug, Clone, Deserialize)]
pub struct CacheCreation {
    pub ephemeral_5m_input_tokens: i64,
    pub ephemeral_1h_input_tokens: i64,
}

/// 定价服务状态
#[derive(Debug, Clone, Serialize)]
pub struct PricingStatus {
    pub initialized: bool,
    pub last_updated: Option<DateTime<Utc>>,
    pub model_count: usize,
    pub next_update: Option<DateTime<Utc>>,
}

/// 更新结果
#[derive(Debug, Clone, Serialize)]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
}

/// 定价服务
pub struct PricingService {
    // 配置
    data_dir: PathBuf,
    pricing_file: PathBuf,
    pricing_url: String,
    hash_url: String,
    fallback_file: PathBuf,
    local_hash_file: PathBuf,

    // 数据
    pricing_data: Arc<RwLock<Option<HashMap<String, ModelPricing>>>>,
    last_updated: Arc<RwLock<Option<DateTime<Utc>>>>,

    // 间隔
    update_interval: Duration,     // 24 小时
    hash_check_interval: Duration, // 10 分钟

    // 硬编码价格
    ephemeral_1h_pricing: HashMap<String, f64>,
    long_context_pricing: HashMap<String, LongContextPricing>,

    // HTTP 客户端
    http_client: Arc<reqwest::Client>,

    // 哈希同步状态
    hash_sync_in_progress: Arc<RwLock<bool>>,
}

impl PricingService {
    /// 创建新的定价服务
    pub fn new(http_client: Arc<reqwest::Client>) -> Self {
        // 从环境变量获取配置
        let repository = std::env::var("PRICE_MIRROR_REPO")
            .or_else(|_| std::env::var("GITHUB_REPOSITORY"))
            .unwrap_or_else(|_| {
                warn!("未设置 PRICE_MIRROR_REPO 或 GITHUB_REPOSITORY 环境变量");
                String::new()
            });

        let branch =
            std::env::var("PRICE_MIRROR_BRANCH").unwrap_or_else(|_| "price-mirror".to_string());
        let pricing_filename = std::env::var("PRICE_MIRROR_FILENAME")
            .unwrap_or_else(|_| "model_prices_and_context_window.json".to_string());
        let hash_filename = std::env::var("PRICE_MIRROR_HASH_FILENAME")
            .unwrap_or_else(|_| "model_prices_and_context_window.sha256".to_string());

        let base_url = std::env::var("PRICE_MIRROR_BASE_URL").unwrap_or_else(|_| {
            format!(
                "https://raw.githubusercontent.com/{}/{}",
                repository, branch
            )
        });

        let pricing_url = std::env::var("PRICE_MIRROR_JSON_URL")
            .unwrap_or_else(|_| format!("{}/{}", base_url, pricing_filename));
        let hash_url = std::env::var("PRICE_MIRROR_HASH_URL")
            .unwrap_or_else(|_| format!("{}/{}", base_url, hash_filename));

        let data_dir = PathBuf::from("data");
        let pricing_file = data_dir.join("model_pricing.json");
        let local_hash_file = data_dir.join("model_pricing.sha256");
        let fallback_file =
            PathBuf::from("resources/model-pricing/model_prices_and_context_window.json");

        // 硬编码的 1 小时缓存价格（美元/百万 token）
        let mut ephemeral_1h_pricing = HashMap::new();
        // Opus 系列: $30/MTok
        ephemeral_1h_pricing.insert("claude-opus-4-1".to_string(), 0.00003);
        ephemeral_1h_pricing.insert("claude-opus-4-1-20250805".to_string(), 0.00003);
        ephemeral_1h_pricing.insert("claude-opus-4".to_string(), 0.00003);
        ephemeral_1h_pricing.insert("claude-opus-4-20250514".to_string(), 0.00003);
        ephemeral_1h_pricing.insert("claude-3-opus".to_string(), 0.00003);
        ephemeral_1h_pricing.insert("claude-3-opus-latest".to_string(), 0.00003);
        ephemeral_1h_pricing.insert("claude-3-opus-20240229".to_string(), 0.00003);

        // Sonnet 系列: $6/MTok
        ephemeral_1h_pricing.insert("claude-3-5-sonnet".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-3-5-sonnet-latest".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-3-5-sonnet-20241022".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-3-5-sonnet-20240620".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-3-sonnet".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-3-sonnet-20240307".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-sonnet-3".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-sonnet-3-5".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-sonnet-3-7".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-sonnet-4".to_string(), 0.000006);
        ephemeral_1h_pricing.insert("claude-sonnet-4-20250514".to_string(), 0.000006);

        // Haiku 系列: $1.6/MTok
        ephemeral_1h_pricing.insert("claude-3-5-haiku".to_string(), 0.0000016);
        ephemeral_1h_pricing.insert("claude-3-5-haiku-latest".to_string(), 0.0000016);
        ephemeral_1h_pricing.insert("claude-3-5-haiku-20241022".to_string(), 0.0000016);
        ephemeral_1h_pricing.insert("claude-3-haiku".to_string(), 0.0000016);
        ephemeral_1h_pricing.insert("claude-3-haiku-20240307".to_string(), 0.0000016);
        ephemeral_1h_pricing.insert("claude-haiku-3".to_string(), 0.0000016);
        ephemeral_1h_pricing.insert("claude-haiku-3-5".to_string(), 0.0000016);

        // 硬编码的 1M 上下文模型价格
        let mut long_context_pricing = HashMap::new();
        long_context_pricing.insert(
            "claude-sonnet-4-20250514[1m]".to_string(),
            LongContextPricing {
                input: 0.000006,   // $6/MTok
                output: 0.0000225, // $22.50/MTok
            },
        );

        Self {
            data_dir,
            pricing_file,
            pricing_url,
            hash_url,
            fallback_file,
            local_hash_file,
            pricing_data: Arc::new(RwLock::new(None)),
            last_updated: Arc::new(RwLock::new(None)),
            update_interval: Duration::from_secs(24 * 3600),
            hash_check_interval: Duration::from_secs(10 * 60),
            ephemeral_1h_pricing,
            long_context_pricing,
            http_client,
            hash_sync_in_progress: Arc::new(RwLock::new(false)),
        }
    }

    /// 初始化价格服务
    pub async fn initialize(&self) -> Result<()> {
        // 确保 data 目录存在
        tokio::fs::create_dir_all(&self.data_dir).await?;
        info!("📁 Data directory ensured");

        // 检查并更新价格数据
        self.check_and_update_pricing().await?;

        // 初次启动时执行哈希校验
        self.sync_with_remote_hash().await?;

        // 启动定时更新任务
        self.start_update_timer();

        // 启动哈希轮询任务
        self.start_hash_check_timer();

        info!("💰 Pricing service initialized successfully");
        Ok(())
    }

    /// 检查并更新价格数据
    async fn check_and_update_pricing(&self) -> Result<()> {
        let needs_update = self.needs_update().await?;

        if needs_update {
            info!("🔄 Updating model pricing data...");
            self.download_pricing_data().await?;
        } else {
            self.load_pricing_data().await?;
        }

        Ok(())
    }

    /// 检查是否需要更新
    async fn needs_update(&self) -> Result<bool> {
        if !self.pricing_file.exists() {
            info!("📋 Pricing file not found, will download");
            return Ok(true);
        }

        let metadata = tokio::fs::metadata(&self.pricing_file).await?;
        let modified = metadata.modified()?;
        let file_age = modified.elapsed()?;

        if file_age > self.update_interval {
            let hours = file_age.as_secs() / 3600;
            info!("📋 Pricing file is {} hours old, will update", hours);
            return Ok(true);
        }

        Ok(false)
    }

    /// 下载价格数据
    async fn download_pricing_data(&self) -> Result<()> {
        match self.download_from_remote().await {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("⚠️  Failed to download pricing data: {}", e);
                info!("📋 Using local fallback pricing data...");
                self.use_fallback_pricing().await
            }
        }
    }

    /// 从远程下载
    async fn download_from_remote(&self) -> Result<()> {
        let response = self
            .http_client
            .get(&self.pricing_url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP {}: {}", response.status(), response.status()));
        }

        let content = response.bytes().await?;
        let json_data: HashMap<String, ModelPricing> = serde_json::from_slice(&content)?;

        // 保存文件
        tokio::fs::write(&self.pricing_file, &content).await?;

        // 更新哈希
        self.persist_local_hash(&content).await?;

        // 更新内存数据
        *self.pricing_data.write().await = Some(json_data.clone());
        *self.last_updated.write().await = Some(Utc::now());

        info!("💰 Downloaded pricing data for {} models", json_data.len());

        Ok(())
    }

    /// 使用 fallback 定价数据
    async fn use_fallback_pricing(&self) -> Result<()> {
        if !self.fallback_file.exists() {
            error!(
                "❌ Fallback pricing file not found at: {:?}",
                self.fallback_file
            );
            return Err(anyhow!("Fallback pricing file not found"));
        }

        info!("📋 Copying fallback pricing data to data directory...");

        let fallback_data = tokio::fs::read(&self.fallback_file).await?;
        let json_data: HashMap<String, ModelPricing> = serde_json::from_slice(&fallback_data)?;

        // 格式化 JSON
        let formatted_json = serde_json::to_vec_pretty(&json_data)?;

        // 保存到 data 目录
        tokio::fs::write(&self.pricing_file, &formatted_json).await?;
        self.persist_local_hash(&formatted_json).await?;

        // 更新内存数据
        *self.pricing_data.write().await = Some(json_data.clone());
        *self.last_updated.write().await = Some(Utc::now());

        warn!(
            "⚠️  Using fallback pricing data for {} models",
            json_data.len()
        );
        info!("💡 Note: This fallback data may be outdated. The system will try to update from the remote source on next check.");

        Ok(())
    }

    /// 加载本地定价数据
    async fn load_pricing_data(&self) -> Result<()> {
        if !self.pricing_file.exists() {
            warn!("💰 No pricing data file found, will use fallback");
            return self.use_fallback_pricing().await;
        }

        let data = tokio::fs::read(&self.pricing_file).await?;
        let json_data: HashMap<String, ModelPricing> = serde_json::from_slice(&data)?;

        *self.pricing_data.write().await = Some(json_data.clone());

        let metadata = tokio::fs::metadata(&self.pricing_file).await?;
        *self.last_updated.write().await = Some(DateTime::from(metadata.modified()?));

        info!(
            "💰 Loaded pricing data for {} models from cache",
            json_data.len()
        );

        Ok(())
    }

    /// 与远程哈希对比
    async fn sync_with_remote_hash(&self) -> Result<()> {
        let mut in_progress = self.hash_sync_in_progress.write().await;
        if *in_progress {
            return Ok(());
        }
        *in_progress = true;
        drop(in_progress);

        let result = async {
            let remote_hash = match self.fetch_remote_hash().await {
                Ok(h) => h,
                Err(_) => return Ok(()),
            };

            let local_hash = self.compute_local_hash().await;

            if local_hash.is_none() {
                info!("📄 本地价格文件缺失，尝试下载最新版本");
                self.download_pricing_data().await?;
                return Ok(());
            }

            if remote_hash != local_hash.unwrap() {
                info!("🔁 检测到远端价格文件更新，开始下载最新数据");
                self.download_pricing_data().await?;
            }

            Ok::<(), anyhow::Error>(())
        }
        .await;

        *self.hash_sync_in_progress.write().await = false;

        if let Err(e) = result {
            warn!("⚠️  哈希校验失败：{}", e);
        }

        Ok(())
    }

    /// 获取远程哈希
    async fn fetch_remote_hash(&self) -> Result<String> {
        let response = self
            .http_client
            .get(&self.hash_url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("哈希文件获取失败：HTTP {}", response.status()));
        }

        let data = response.text().await?;
        let hash = data
            .split_whitespace()
            .next()
            .ok_or_else(|| anyhow!("哈希文件内容为空"))?;

        Ok(hash.to_string())
    }

    /// 计算本地哈希
    async fn compute_local_hash(&self) -> Option<String> {
        // 尝试读取缓存的哈希文件
        if self.local_hash_file.exists() {
            if let Ok(cached) = tokio::fs::read_to_string(&self.local_hash_file).await {
                let trimmed = cached.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }

        // 计算文件哈希
        if !self.pricing_file.exists() {
            return None;
        }

        match tokio::fs::read(&self.pricing_file).await {
            Ok(content) => (self.persist_local_hash(&content).await).ok(),
            Err(_) => None,
        }
    }

    /// 持久化本地哈希
    async fn persist_local_hash(&self, content: &[u8]) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = format!("{:x}", hasher.finalize());

        tokio::fs::write(&self.local_hash_file, format!("{}\n", hash)).await?;

        Ok(hash)
    }

    /// 启动定时更新任务
    fn start_update_timer(&self) {
        let service = Arc::new(self.clone());
        tokio::spawn(async move {
            let mut interval = interval(service.update_interval);
            interval.tick().await; // 跳过第一次

            loop {
                interval.tick().await;
                if let Err(e) = service.check_and_update_pricing().await {
                    error!("Failed to update pricing: {}", e);
                }
            }
        });
    }

    /// 启动哈希轮询任务
    fn start_hash_check_timer(&self) {
        let service = Arc::new(self.clone());
        tokio::spawn(async move {
            let mut interval = interval(service.hash_check_interval);
            interval.tick().await; // 跳过第一次

            loop {
                interval.tick().await;
                if let Err(e) = service.sync_with_remote_hash().await {
                    debug!("Hash check failed: {}", e);
                }
            }
        });

        info!("🕒 已启用价格文件哈希轮询（每10分钟校验一次）");
    }

    /// 获取模型定价
    pub async fn get_model_pricing(&self, model_name: &str) -> Option<ModelPricing> {
        let data = self.pricing_data.read().await;
        let pricing_map = data.as_ref()?;

        // 1. 精确匹配
        if let Some(pricing) = pricing_map.get(model_name) {
            debug!("💰 Found exact pricing match for {}", model_name);
            return Some(pricing.clone());
        }

        // 2. gpt-5-codex fallback
        if model_name == "gpt-5-codex" {
            if let Some(pricing) = pricing_map.get("gpt-5") {
                info!("💰 Using gpt-5 pricing as fallback for {}", model_name);
                return Some(pricing.clone());
            }
        }

        // 3. Bedrock 区域前缀处理
        if model_name.contains(".anthropic.") || model_name.contains(".claude") {
            let without_region = model_name
                .replacen("us.", "", 1)
                .replacen("eu.", "", 1)
                .replacen("apac.", "", 1);
            if let Some(pricing) = pricing_map.get(&without_region) {
                debug!(
                    "💰 Found pricing for {} by removing region prefix: {}",
                    model_name, without_region
                );
                return Some(pricing.clone());
            }
        }

        // 4. 模糊匹配
        let normalized_model = model_name.to_lowercase().replace(['-', '_'], "");
        for (key, value) in pricing_map.iter() {
            let normalized_key = key.to_lowercase().replace(['-', '_'], "");
            if normalized_key.contains(&normalized_model)
                || normalized_model.contains(&normalized_key)
            {
                debug!(
                    "💰 Found pricing for {} using fuzzy match: {}",
                    model_name, key
                );
                return Some(value.clone());
            }
        }

        // 5. Bedrock 核心模型匹配
        if model_name.contains("anthropic.claude") {
            let core_model = model_name
                .replacen("us.", "", 1)
                .replacen("eu.", "", 1)
                .replacen("apac.", "", 1)
                .replace("anthropic.", "");

            for (key, value) in pricing_map.iter() {
                if key.contains(&core_model) || key.replace("anthropic.", "").contains(&core_model)
                {
                    debug!(
                        "💰 Found pricing for {} using Bedrock core model match: {}",
                        model_name, key
                    );
                    return Some(value.clone());
                }
            }
        }

        debug!("💰 No pricing found for model: {}", model_name);
        None
    }

    /// 获取 1 小时缓存定价
    pub fn get_ephemeral_1h_pricing(&self, model_name: &str) -> f64 {
        // 1. 直接匹配
        if let Some(&price) = self.ephemeral_1h_pricing.get(model_name) {
            return price;
        }

        // 2. 检查模型名称
        let model_lower = model_name.to_lowercase();

        if model_lower.contains("opus") {
            return 0.00003; // $30/MTok
        }

        if model_lower.contains("sonnet") {
            return 0.000006; // $6/MTok
        }

        if model_lower.contains("haiku") {
            return 0.0000016; // $1.6/MTok
        }

        debug!("💰 No 1h cache pricing found for model: {}", model_name);
        0.0
    }

    /// 计算使用费用
    pub async fn calculate_cost(&self, usage: &Usage, model_name: &str) -> CostResult {
        // 检查是否为 1M 上下文模型
        let is_long_context_model = model_name.contains("[1m]");
        let mut is_long_context_request = false;
        let mut use_long_context_pricing = false;

        if is_long_context_model {
            let total_input_tokens = usage.input_tokens
                + usage.cache_creation_input_tokens
                + usage.cache_read_input_tokens;

            if total_input_tokens > 200_000 {
                is_long_context_request = true;
                if self.long_context_pricing.contains_key(model_name) {
                    use_long_context_pricing = true;
                } else if let Some(default_model) = self.long_context_pricing.keys().next() {
                    use_long_context_pricing = true;
                    warn!(
                        "⚠️ No specific 1M pricing for {}, using default from {}",
                        model_name, default_model
                    );
                }
            }
        }

        let pricing = self.get_model_pricing(model_name).await;

        if pricing.is_none() && !use_long_context_pricing {
            return CostResult {
                input_cost: 0.0,
                output_cost: 0.0,
                cache_create_cost: 0.0,
                cache_read_cost: 0.0,
                ephemeral_5m_cost: 0.0,
                ephemeral_1h_cost: 0.0,
                total_cost: 0.0,
                has_pricing: false,
                is_long_context_request: false,
                pricing: PricingDetails {
                    input: 0.0,
                    output: 0.0,
                    cache_create: 0.0,
                    cache_read: 0.0,
                    ephemeral_1h: 0.0,
                },
            };
        }

        let (input_cost, output_cost) = if use_long_context_pricing {
            let long_prices = self
                .long_context_pricing
                .get(model_name)
                .or_else(|| self.long_context_pricing.values().next())
                .unwrap();

            let input_cost = usage.input_tokens as f64 * long_prices.input;
            let output_cost = usage.output_tokens as f64 * long_prices.output;

            info!(
                "💰 Using 1M context pricing for {}: input=${}/token, output=${}/token",
                model_name, long_prices.input, long_prices.output
            );

            (input_cost, output_cost)
        } else {
            let pricing = pricing.as_ref().unwrap();
            let input_cost = usage.input_tokens as f64 * pricing.input_cost_per_token;
            let output_cost = usage.output_tokens as f64 * pricing.output_cost_per_token;
            (input_cost, output_cost)
        };

        let pricing_ref = pricing.as_ref().unwrap_or(&ModelPricing {
            input_cost_per_token: 0.0,
            output_cost_per_token: 0.0,
            cache_creation_input_token_cost: None,
            cache_read_input_token_cost: None,
            litellm_provider: None,
        });

        let cache_read_cost = usage.cache_read_input_tokens as f64
            * pricing_ref.cache_read_input_token_cost.unwrap_or(0.0);

        // 处理缓存创建费用
        let (ephemeral_5m_cost, ephemeral_1h_cost, cache_create_cost) =
            if let Some(ref cache_creation) = usage.cache_creation {
                let ephemeral_5m = cache_creation.ephemeral_5m_input_tokens as f64
                    * pricing_ref.cache_creation_input_token_cost.unwrap_or(0.0);

                let ephemeral_1h_price = self.get_ephemeral_1h_pricing(model_name);
                let ephemeral_1h =
                    cache_creation.ephemeral_1h_input_tokens as f64 * ephemeral_1h_price;

                (ephemeral_5m, ephemeral_1h, ephemeral_5m + ephemeral_1h)
            } else {
                let cache_create = usage.cache_creation_input_tokens as f64
                    * pricing_ref.cache_creation_input_token_cost.unwrap_or(0.0);
                (cache_create, 0.0, cache_create)
            };

        let total_cost = input_cost + output_cost + cache_create_cost + cache_read_cost;

        CostResult {
            input_cost,
            output_cost,
            cache_create_cost,
            cache_read_cost,
            ephemeral_5m_cost,
            ephemeral_1h_cost,
            total_cost,
            has_pricing: true,
            is_long_context_request,
            pricing: PricingDetails {
                input: if use_long_context_pricing {
                    self.long_context_pricing
                        .get(model_name)
                        .or_else(|| self.long_context_pricing.values().next())
                        .map(|p| p.input)
                        .unwrap_or(0.0)
                } else {
                    pricing_ref.input_cost_per_token
                },
                output: if use_long_context_pricing {
                    self.long_context_pricing
                        .get(model_name)
                        .or_else(|| self.long_context_pricing.values().next())
                        .map(|p| p.output)
                        .unwrap_or(0.0)
                } else {
                    pricing_ref.output_cost_per_token
                },
                cache_create: pricing_ref.cache_creation_input_token_cost.unwrap_or(0.0),
                cache_read: pricing_ref.cache_read_input_token_cost.unwrap_or(0.0),
                ephemeral_1h: self.get_ephemeral_1h_pricing(model_name),
            },
        }
    }

    /// 格式化费用
    pub fn format_cost(&self, cost: f64) -> String {
        if cost == 0.0 {
            "$0.000000".to_string()
        } else if cost < 0.000001 {
            format!("${:.2e}", cost)
        } else if cost < 0.01 {
            format!("${:.6}", cost)
        } else if cost < 1.0 {
            format!("${:.4}", cost)
        } else {
            format!("${:.2}", cost)
        }
    }

    /// 获取服务状态
    pub async fn get_status(&self) -> PricingStatus {
        let data = self.pricing_data.read().await;
        let last_updated = *self.last_updated.read().await;

        PricingStatus {
            initialized: data.is_some(),
            last_updated,
            model_count: data.as_ref().map(|d| d.len()).unwrap_or(0),
            next_update: last_updated
                .map(|t| t + chrono::Duration::from_std(self.update_interval).unwrap()),
        }
    }

    /// 强制更新
    pub async fn force_update(&self) -> UpdateResult {
        match self.download_from_remote().await {
            Ok(_) => UpdateResult {
                success: true,
                message: "Pricing data updated successfully".to_string(),
            },
            Err(e) => {
                error!("❌ Force update failed: {}", e);
                info!("📋 Force update failed, using fallback pricing data...");
                let _ = self.use_fallback_pricing().await;
                UpdateResult {
                    success: false,
                    message: format!(
                        "Download failed: {}. Using fallback pricing data instead.",
                        e
                    ),
                }
            }
        }
    }
}

// 实现 Clone 用于定时任务
impl Clone for PricingService {
    fn clone(&self) -> Self {
        Self {
            data_dir: self.data_dir.clone(),
            pricing_file: self.pricing_file.clone(),
            pricing_url: self.pricing_url.clone(),
            hash_url: self.hash_url.clone(),
            fallback_file: self.fallback_file.clone(),
            local_hash_file: self.local_hash_file.clone(),
            pricing_data: Arc::clone(&self.pricing_data),
            last_updated: Arc::clone(&self.last_updated),
            update_interval: self.update_interval,
            hash_check_interval: self.hash_check_interval,
            ephemeral_1h_pricing: self.ephemeral_1h_pricing.clone(),
            long_context_pricing: self.long_context_pricing.clone(),
            http_client: Arc::clone(&self.http_client),
            hash_sync_in_progress: Arc::clone(&self.hash_sync_in_progress),
        }
    }
}
