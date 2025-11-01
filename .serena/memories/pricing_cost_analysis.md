# Phase 7: 定价和成本服务分析

## Node.js 实现概览

### 核心服务组件

#### 1. PricingService (`pricingService.js` - 806 行)

**核心功能**:
- 模型定价数据管理和自动更新
- 从远程 GitHub 仓库下载最新定价数据
- 本地缓存和 fallback 机制
- 文件监听和自动重载
- 哈希校验确保数据一致性
- 成本计算（支持 1M 上下文和 1h 缓存）

**关键特性**:

1. **定价数据源** (`pricingSource.js`):
   - 从 GitHub raw URL 下载: `https://raw.githubusercontent.com/{repo}/{branch}/{filename}`
   - 默认文件: `model_prices_and_context_window.json`
   - 哈希文件: `model_prices_and_context_window.sha256`
   - 环境变量配置: `PRICE_MIRROR_REPO`, `PRICE_MIRROR_BRANCH`, `PRICE_MIRROR_BASE_URL`

2. **自动更新机制**:
   - 24 小时定时更新（`updateInterval`）
   - 10 分钟哈希轮询（`hashCheckInterval`）
   - 文件监听器（60 秒轮询）
   - 防抖重载（500ms）
   
3. **哈希校验**:
   - 下载前后计算 SHA-256 哈希
   - 与远程哈希文件对比
   - 哈希不匹配时自动重新下载
   - 本地哈希缓存（`data/model_pricing.sha256`）

4. **Fallback 机制**:
   - 主数据文件: `data/model_pricing.json`
   - Fallback 文件: `resources/model-pricing/model_prices_and_context_window.json`
   - 下载失败时自动使用 fallback
   - Fallback 数据复制到 data 目录

5. **硬编码价格**:
   - **1 小时缓存价格** (`ephemeral1hPricing`):
     - Opus 系列: $30/MTok
     - Sonnet 系列: $6/MTok
     - Haiku 系列: $1.6/MTok
   - **1M 上下文价格** (`longContextPricing`):
     - `claude-sonnet-4-20250514[1m]`: input=$6/MTok, output=$22.50/MTok
     - 当总输入 tokens > 200k 时自动使用

6. **模型名称匹配**:
   - 精确匹配
   - `gpt-5-codex` → `gpt-5` fallback
   - Bedrock 区域前缀处理（`us.anthropic.` → `anthropic.`）
   - 模糊匹配（去除 `-` 和 `_`）
   - Bedrock 核心模型匹配

7. **成本计算** (`calculateCost`):
   - **输入参数**: 
     ```javascript
     {
       input_tokens: number,
       output_tokens: number,
       cache_creation_input_tokens: number,
       cache_read_input_tokens: number,
       cache_creation: {  // 新格式，支持详细缓存类型
         ephemeral_5m_input_tokens: number,
         ephemeral_1h_input_tokens: number
       }
     }
     ```
   - **返回值**:
     ```javascript
     {
       inputCost: number,
       outputCost: number,
       cacheCreateCost: number,
       cacheReadCost: number,
       ephemeral5mCost: number,
       ephemeral1hCost: number,
       totalCost: number,
       hasPricing: boolean,
       isLongContextRequest: boolean,
       pricing: {
         input: number,
         output: number,
         cacheCreate: number,
         cacheRead: number,
         ephemeral1h: number
       }
     }
     ```

#### 2. CostInitService (`costInitService.js` - 197 行)

**核心功能**:
- 初始化历史使用记录的费用数据
- 扫描所有 API Key 的使用统计
- 计算并存储费用数据到 Redis

**关键流程**:

1. **初始化所有费用** (`initializeAllCosts`):
   - 获取所有 API Keys
   - 逐个处理每个 Key
   - 每 10 个 Key 打印进度
   - 返回处理和错误计数

2. **单个 Key 初始化** (`initializeApiKeyCosts`):
   - 扫描 Redis keys: `usage:{keyId}:model:{period}:{model}:{date}`
   - 支持的周期: `daily`, `monthly`, `hourly`
   - 使用 CostCalculator 计算费用
   - 按日期/月份/小时分组累加
   - 存储到 Redis:
     - `usage:cost:daily:{keyId}:{date}` (30天过期)
     - `usage:cost:monthly:{keyId}:{month}` (90天过期)
     - `usage:cost:hourly:{keyId}:{hour}` (7天过期)
     - `usage:cost:total:{keyId}` (不过期)

3. **检查是否需要初始化** (`needsInitialization`):
   - 检查是否存在 `usage:cost:*` keys
   - 抽样检查使用数据是否有对应费用数据
   - 返回 boolean

#### 3. CostCalculator (`costCalculator.js` - 327 行)

**核心功能**:
- 费用计算工具类
- 静态备用定价 + 动态定价服务集成
- 模型定价信息管理

**关键特性**:

1. **静态备用定价** (`MODEL_PRICING`):
   - Claude 模型价格（USD per 1M tokens）
   - 包含: input, output, cacheWrite, cacheRead
   - 未知模型使用默认定价

2. **计算单次请求费用** (`calculateCost`):
   - **优先使用 pricingService** (支持详细缓存类型和 1M 模型)
   - Fallback 到静态价格
   - **OpenAI 模型特殊处理**:
     - 如果没有 `cache_creation_input_token_cost`
     - 缓存创建按普通 input 价格计费
   - 返回详细费用分解和调试信息

3. **计算聚合使用费用** (`calculateAggregatedCost`):
   - 支持字段名变体（`inputTokens` / `totalInputTokens`）
   - 复用 `calculateCost` 方法

4. **其他工具方法**:
   - `getModelPricing`: 获取模型定价（支持 gpt-5-codex fallback）
   - `getAllModelPricing`: 获取所有模型定价
   - `isModelSupported`: 检查模型是否支持
   - `formatCost`: 格式化费用显示
   - `calculateCacheSavings`: 计算缓存节省

## Rust 实现需求

### 核心数据结构

#### 1. PricingService

```rust
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
    
    // 定时器
    update_interval: Duration,  // 24 小时
    hash_check_interval: Duration,  // 10 分钟
    
    // 硬编码价格
    ephemeral_1h_pricing: HashMap<String, f64>,
    long_context_pricing: HashMap<String, LongContextPricing>,
    
    // HTTP 客户端
    http_client: Arc<reqwest::Client>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelPricing {
    pub input_cost_per_token: f64,
    pub output_cost_per_token: f64,
    pub cache_creation_input_token_cost: Option<f64>,
    pub cache_read_input_token_cost: Option<f64>,
    pub litellm_provider: Option<String>,
    // 其他字段根据实际 JSON 结构添加
}

#[derive(Debug, Clone)]
pub struct LongContextPricing {
    pub input: f64,
    pub output: f64,
}

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

#[derive(Debug, Clone)]
pub struct PricingDetails {
    pub input: f64,
    pub output: f64,
    pub cache_create: f64,
    pub cache_read: f64,
    pub ephemeral_1h: f64,
}
```

#### 2. CostCalculator

```rust
pub struct CostCalculator {
    pricing_service: Arc<PricingService>,
    static_pricing: HashMap<String, StaticModelPricing>,
}

#[derive(Debug, Clone)]
pub struct StaticModelPricing {
    pub input: f64,   // USD per 1M tokens
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
}

#[derive(Debug, Clone)]
pub struct CostCalculationResult {
    pub model: String,
    pub pricing: StaticModelPricing,
    pub using_dynamic_pricing: bool,
    pub is_long_context_request: bool,
    pub usage: UsageDetails,
    pub costs: CostDetails,
    pub formatted: FormattedCosts,
    pub debug: DebugInfo,
}

#[derive(Debug, Clone)]
pub struct UsageDetails {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_create_tokens: i64,
    pub cache_read_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone)]
pub struct CostDetails {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
    pub total: f64,
}
```

### 核心功能实现

#### 1. PricingService 方法

```rust
impl PricingService {
    pub async fn new(http_client: Arc<reqwest::Client>) -> Result<Self> { }
    
    pub async fn initialize(&self) -> Result<()> {
        // 1. 确保 data 目录存在
        // 2. 检查并更新定价数据
        // 3. 初次哈希校验
        // 4. 启动定时更新任务
        // 5. 启动哈希轮询任务
        // 6. 启动文件监听器（使用 notify crate）
    }
    
    async fn check_and_update_pricing(&self) -> Result<()> { }
    
    fn needs_update(&self) -> bool {
        // 检查文件是否存在
        // 检查文件年龄是否 > 24 小时
    }
    
    async fn download_pricing_data(&self) -> Result<()> {
        // 从远程下载
        // 失败时使用 fallback
    }
    
    async fn sync_with_remote_hash(&self) -> Result<()> {
        // 获取远程哈希
        // 计算本地哈希
        // 对比并决定是否重新下载
    }
    
    async fn fetch_remote_hash(&self) -> Result<String> {
        // HTTP GET hash_url
        // 30 秒超时
    }
    
    fn compute_local_hash(&self) -> Option<String> {
        // 读取缓存的哈希文件
        // 或计算文件的 SHA-256
    }
    
    fn persist_local_hash(&self, content: &[u8]) -> String {
        // 计算 SHA-256
        // 写入 local_hash_file
    }
    
    async fn _download_from_remote(&self) -> Result<()> {
        // HTTP GET pricing_url
        // 解析 JSON
        // 保存文件并更新哈希
        // 更新内存数据
    }
    
    async fn load_pricing_data(&self) -> Result<()> { }
    
    async fn use_fallback_pricing(&self) -> Result<()> {
        // 复制 fallback 文件到 data 目录
        // 更新内存数据
    }
    
    pub fn get_model_pricing(&self, model_name: &str) -> Option<ModelPricing> {
        // 1. 精确匹配
        // 2. gpt-5-codex fallback
        // 3. Bedrock 区域前缀处理
        // 4. 模糊匹配
        // 5. Bedrock 核心模型匹配
    }
    
    pub fn get_ephemeral_1h_pricing(&self, model_name: &str) -> f64 {
        // 1. 直接匹配
        // 2. 模型名称包含 opus/sonnet/haiku
        // 3. 默认返回 0
    }
    
    pub fn calculate_cost(&self, usage: &Usage, model_name: &str) -> CostResult {
        // 1. 检查是否为 1M 上下文模型
        // 2. 检查总输入 tokens 是否 > 200k
        // 3. 获取模型定价
        // 4. 计算各类费用
        // 5. 处理详细缓存创建数据
        // 6. 返回结果
    }
    
    pub fn format_cost(&self, cost: f64) -> String { }
    
    pub fn get_status(&self) -> PricingStatus { }
    
    pub async fn force_update(&self) -> Result<UpdateResult> { }
}
```

#### 2. CostCalculator 方法

```rust
impl CostCalculator {
    pub fn new(pricing_service: Arc<PricingService>) -> Self { }
    
    pub fn calculate_cost(&self, usage: &Usage, model: &str) -> CostCalculationResult {
        // 1. 检查是否需要使用 pricingService（详细缓存或 1M 模型）
        // 2. 优先使用动态定价
        // 3. Fallback 到静态定价
        // 4. OpenAI 模型特殊处理
        // 5. 计算各类费用
        // 6. 返回详细结果
    }
    
    pub fn calculate_aggregated_cost(&self, aggregated_usage: &AggregatedUsage, model: &str) -> CostCalculationResult { }
    
    pub fn get_model_pricing(&self, model: &str) -> StaticModelPricing { }
    
    pub fn get_all_model_pricing(&self) -> HashMap<String, StaticModelPricing> { }
    
    pub fn is_model_supported(&self, model: &str) -> bool { }
    
    pub fn format_cost(&self, cost: f64, decimals: usize) -> String { }
    
    pub fn calculate_cache_savings(&self, usage: &Usage, model: &str) -> CacheSavings { }
}
```

### 定时任务和文件监听

#### 1. 定时更新任务

```rust
// 使用 tokio::time::interval
async fn start_update_timer(service: Arc<PricingService>) {
    let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
    
    loop {
        interval.tick().await;
        if let Err(e) = service.check_and_update_pricing().await {
            error!("Failed to update pricing: {}", e);
        }
    }
}
```

#### 2. 哈希轮询任务

```rust
async fn start_hash_check_timer(service: Arc<PricingService>) {
    let mut interval = tokio::time::interval(Duration::from_secs(10 * 60));
    
    loop {
        interval.tick().await;
        if let Err(e) = service.sync_with_remote_hash().await {
            warn!("Hash check failed: {}", e);
        }
    }
}
```

#### 3. 文件监听器

```rust
// 使用 notify crate
use notify::{Watcher, RecursiveMode, watcher};

async fn start_file_watcher(service: Arc<PricingService>) -> Result<()> {
    let (tx, rx) = channel();
    
    let mut watcher = watcher(tx, Duration::from_secs(60))?;
    watcher.watch(&service.pricing_file, RecursiveMode::NonRecursive)?;
    
    tokio::spawn(async move {
        while let Ok(event) = rx.recv() {
            // 防抖处理
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Err(e) = service.reload_pricing_data().await {
                error!("Failed to reload pricing: {}", e);
            }
        }
    });
    
    Ok(())
}
```

### 依赖 Crates

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
sha2 = "0.10"
notify = "5.0"  # 文件监听
chrono = "0.4"
```

## 实现优先级

### Phase 7.1: 核心数据结构和基础功能
- ✅ 定义数据结构（PricingService, CostCalculator, 各种 Result 类型）
- ✅ 实现静态价格初始化
- ✅ 实现基础成本计算逻辑

### Phase 7.2: 文件和网络操作
- ⏳ 实现本地文件读写
- ⏳ 实现远程数据下载
- ⏳ 实现哈希计算和校验
- ⏳ 实现 fallback 机制

### Phase 7.3: 模型匹配和价格查询
- ⏳ 实现精确匹配
- ⏳ 实现 fallback 逻辑（gpt-5-codex, Bedrock 区域）
- ⏳ 实现模糊匹配
- ⏳ 实现硬编码价格（1h 缓存、1M 上下文）

### Phase 7.4: 定时任务和监听
- ⏳ 实现定时更新任务
- ⏳ 实现哈希轮询任务
- ⏳ 实现文件监听器（notify crate）
- ⏳ 实现防抖重载机制

### Phase 7.5: 集成测试
- ⏳ 测试定价数据下载
- ⏳ 测试哈希校验
- ⏳ 测试 fallback 机制
- ⏳ 测试成本计算（各种场景）
- ⏳ 测试定时任务

## 待办事项

- [ ] 创建 `src/services/pricing_service.rs`
- [ ] 创建 `src/utils/cost_calculator.rs`
- [ ] 实现 PricingService 基础结构
- [ ] 实现 CostCalculator 基础结构
- [ ] 实现文件下载和哈希校验
- [ ] 实现定时任务和文件监听
- [ ] 编写完整的集成测试
- [ ] 更新 main.rs 集成定价服务

## 估算工作量

- PricingService 核心实现: 2 天
- CostCalculator 实现: 0.5 天
- 定时任务和文件监听: 0.5 天
- 完整测试: 1 天
- **总计**: 约 4 天