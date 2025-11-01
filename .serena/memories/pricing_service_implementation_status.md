# Phase 7 - 定价和成本服务实现状态

## 完成情况

### ✅ 已完成

#### 1. PricingService (`src/services/pricing_service.rs` - 780 行)

**核心功能**:
- ✅ 从远程 GitHub 下载定价数据
- ✅ SHA-256 哈希校验
- ✅ 本地缓存和 fallback 机制
- ✅ 24小时定时更新任务
- ✅ 10分钟哈希轮询任务
- ✅ 硬编码价格（1h 缓存、1M 上下文）
- ✅ 多种模型名称匹配策略
- ✅ 成本计算方法

**数据结构**:
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
    
    // 间隔
    update_interval: Duration,      // 24 小时
    hash_check_interval: Duration,  // 10 分钟
    
    // 硬编码价格
    ephemeral_1h_pricing: HashMap<String, f64>,
    long_context_pricing: HashMap<String, LongContextPricing>,
    
    // HTTP 客户端
    http_client: Arc<reqwest::Client>,
    
    // 哈希同步状态
    hash_sync_in_progress: Arc<RwLock<bool>>,
}
```

**关键方法**:
- `new()` - 创建服务，初始化硬编码价格
- `initialize()` - 初始化服务，启动定时任务
- `check_and_update_pricing()` - 检查并更新定价数据
- `download_from_remote()` - 从 GitHub 下载数据
- `use_fallback_pricing()` - 使用 fallback 数据
- `sync_with_remote_hash()` - 哈希校验
- `get_model_pricing()` - 获取模型定价（多种匹配策略）
- `get_ephemeral_1h_pricing()` - 获取 1h 缓存价格
- `calculate_cost()` - 计算使用费用
- `format_cost()` - 格式化费用
- `get_status()` - 获取服务状态
- `force_update()` - 强制更新

**定时任务**:
- `start_update_timer()` - 24小时定时更新
- `start_hash_check_timer()` - 10分钟哈希轮询

#### 2. CostCalculator (`src/utils/cost_calculator.rs` - 470 行)

**核心功能**:
- ✅ 静态备用定价
- ✅ 动态定价服务集成
- ✅ OpenAI 模型特殊处理
- ✅ 缓存节省计算
- ✅ 聚合使用量计算

**数据结构**:
```rust
pub struct CostCalculator {
    pricing_service: Arc<PricingService>,
    static_pricing: HashMap<String, StaticModelPricing>,
}

pub struct CostCalculationResult {
    pub model: String,
    pub pricing: StaticModelPricing,
    pub using_dynamic_pricing: bool,
    pub is_long_context_request: Option<bool>,
    pub usage: UsageDetails,
    pub costs: CostDetails,
    pub formatted: FormattedCosts,
    pub debug: DebugInfo,
}
```

**关键方法**:
- `new()` - 创建计算器，初始化静态价格
- `calculate_cost()` - 计算单次请求费用
- `calculate_cost_with_pricing_service()` - 使用 pricingService 计算
- `calculate_cost_legacy()` - 旧版计算逻辑（向后兼容）
- `calculate_aggregated_cost()` - 计算聚合使用费用
- `get_model_pricing()` - 获取模型定价
- `get_all_model_pricing()` - 获取所有模型定价
- `is_model_supported()` - 检查模型是否支持
- `format_cost()` - 格式化费用
- `calculate_cache_savings()` - 计算缓存节省

#### 3. 模块导出

**src/services/mod.rs**:
```rust
pub mod pricing_service;

pub use pricing_service::{
    CacheCreation, CostResult, LongContextPricing, ModelPricing, PricingDetails, PricingService,
    PricingStatus, UpdateResult, Usage as PricingUsage,
};
```

**src/utils/mod.rs**:
```rust
pub mod cost_calculator;

pub use cost_calculator::{
    AggregatedUsage, CacheSavings, CostCalculationResult, CostCalculator, CostDetails, DebugInfo,
    FormattedCosts, FormattedSavings, StaticModelPricing, UsageDetails,
};
```

#### 4. 编译状态

- ✅ 编译通过，无警告
- ✅ 所有类型正确导出
- ✅ 依赖关系正确

## 待实现

### 🔲 文件监听器

Node.js 版本使用 `fs.watchFile` 实现文件监听和自动重载。Rust 版本可以使用 `notify` crate 实现类似功能。

**实现方案**:
```rust
use notify::{Watcher, RecursiveMode, watcher, DebouncedEvent};
use std::sync::mpsc::channel;
use std::time::Duration;

async fn start_file_watcher(service: Arc<PricingService>) -> Result<()> {
    let (tx, rx) = channel();
    
    let mut watcher = watcher(tx, Duration::from_secs(60))?;
    watcher.watch(&service.pricing_file, RecursiveMode::NonRecursive)?;
    
    tokio::spawn(async move {
        while let Ok(event) = rx.recv() {
            match event {
                DebouncedEvent::Write(_) | DebouncedEvent::Create(_) => {
                    // 防抖处理
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    if let Err(e) = service.reload_pricing_data().await {
                        error!("Failed to reload pricing: {}", e);
                    }
                }
                _ => {}
            }
        }
    });
    
    Ok(())
}
```

**依赖添加**:
```toml
[dependencies]
notify = "5.0"
```

**原因**: 当前实现已包含定时更新和哈希轮询，文件监听器是可选的增强功能。

### 🔲 集成测试

需要编写完整的集成测试，覆盖：
- 定价数据下载
- 哈希校验
- Fallback 机制
- 成本计算（各种场景）
- 模型名称匹配

**测试文件**: `tests/pricing_service_integration_test.rs`

### 🔲 Main.rs 集成

需要在 `main.rs` 中初始化 PricingService 和 CostCalculator，并提供给各个路由使用。

## 技术亮点

### 1. 异步设计
- 所有 I/O 操作（文件读写、网络请求）都是异步的
- 使用 `RwLock` 保护共享数据，支持并发访问
- 定时任务使用 `tokio::spawn` 并发执行

### 2. 哈希校验
- SHA-256 哈希确保数据完整性
- 本地哈希缓存避免重复计算
- 远程哈希对比自动触发更新

### 3. 多种匹配策略
- 精确匹配
- gpt-5-codex → gpt-5 fallback
- Bedrock 区域前缀处理
- 模糊匹配（去除 `-` 和 `_`）
- Bedrock 核心模型匹配

### 4. 硬编码价格
- 1 小时缓存价格（Opus/Sonnet/Haiku 系列）
- 1M 上下文价格（总输入 > 200k tokens 时使用）

### 5. 成本计算
- 支持详细缓存类型（5m/1h）
- OpenAI 模型特殊处理
- 1M 上下文模型特殊处理
- 向后兼容旧版数据格式

## 下一步

1. **编写集成测试** - 覆盖所有核心功能
2. **Main.rs 集成** - 初始化服务并提供给路由
3. **可选：文件监听器** - 增强自动重载功能
4. **文档更新** - 更新 README 和 API 文档

## 总结

Phase 7 核心功能已完成：
- ✅ PricingService 完整实现（780 行）
- ✅ CostCalculator 完整实现（470 行）
- ✅ 模块正确导出
- ✅ 编译通过，无警告

总代码量：约 1250 行

待完成：
- 🔲 集成测试
- 🔲 Main.rs 集成
- 🔲 可选：文件监听器