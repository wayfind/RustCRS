// 成本计算工具
//
// 提供费用计算功能，支持：
// - 静态备用定价
// - 动态定价服务集成
// - OpenAI 模型特殊处理
// - 缓存节省计算

use crate::services::pricing_service::{PricingService, Usage};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

/// 静态模型定价（备用定价，USD per 1M tokens）
#[derive(Debug, Clone, Serialize)]
pub struct StaticModelPricing {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
}

/// 使用详情
#[derive(Debug, Clone, Serialize)]
pub struct UsageDetails {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_create_tokens: i64,
    pub cache_read_tokens: i64,
    pub total_tokens: i64,
}

/// 成本详情
#[derive(Debug, Clone, Serialize)]
pub struct CostDetails {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
    pub total: f64,
}

/// 格式化的成本
#[derive(Debug, Clone, Serialize)]
pub struct FormattedCosts {
    pub input: String,
    pub output: String,
    pub cache_write: String,
    pub cache_read: String,
    pub total: String,
}

/// 调试信息
#[derive(Debug, Clone, Serialize)]
pub struct DebugInfo {
    pub is_openai_model: bool,
    pub has_cache_create_price: bool,
    pub cache_create_tokens: i64,
    pub cache_write_price_used: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_long_context_model: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_long_context_request: Option<bool>,
}

/// 成本计算结果
#[derive(Debug, Clone, Serialize)]
pub struct CostCalculationResult {
    pub model: String,
    pub pricing: StaticModelPricing,
    pub using_dynamic_pricing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_long_context_request: Option<bool>,
    pub usage: UsageDetails,
    pub costs: CostDetails,
    pub formatted: FormattedCosts,
    pub debug: DebugInfo,
}

/// 聚合使用量数据
#[derive(Debug, Clone)]
pub struct AggregatedUsage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_create_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub total_cache_create_tokens: Option<i64>,
    pub total_cache_read_tokens: Option<i64>,
}

/// 缓存节省信息
#[derive(Debug, Clone, Serialize)]
pub struct CacheSavings {
    pub normal_cost: f64,
    pub cache_cost: f64,
    pub savings: f64,
    pub savings_percentage: f64,
    pub formatted: FormattedSavings,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormattedSavings {
    pub normal_cost: String,
    pub cache_cost: String,
    pub savings: String,
    pub savings_percentage: String,
}

/// 成本计算器
pub struct CostCalculator {
    pricing_service: Arc<PricingService>,
    static_pricing: HashMap<String, StaticModelPricing>,
}

impl CostCalculator {
    /// 创建新的成本计算器
    pub fn new(pricing_service: Arc<PricingService>) -> Self {
        let mut static_pricing = HashMap::new();

        // Claude 3.5 Sonnet
        static_pricing.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            StaticModelPricing {
                input: 3.0,
                output: 15.0,
                cache_write: 3.75,
                cache_read: 0.3,
            },
        );
        static_pricing.insert(
            "claude-sonnet-4-20250514".to_string(),
            StaticModelPricing {
                input: 3.0,
                output: 15.0,
                cache_write: 3.75,
                cache_read: 0.3,
            },
        );
        static_pricing.insert(
            "claude-sonnet-4-5-20250929".to_string(),
            StaticModelPricing {
                input: 3.0,
                output: 15.0,
                cache_write: 3.75,
                cache_read: 0.3,
            },
        );

        // Claude 3.5 Haiku
        static_pricing.insert(
            "claude-3-5-haiku-20241022".to_string(),
            StaticModelPricing {
                input: 0.25,
                output: 1.25,
                cache_write: 0.3,
                cache_read: 0.03,
            },
        );

        // Claude 3 Opus
        static_pricing.insert(
            "claude-3-opus-20240229".to_string(),
            StaticModelPricing {
                input: 15.0,
                output: 75.0,
                cache_write: 18.75,
                cache_read: 1.5,
            },
        );

        // Claude Opus 4.1
        static_pricing.insert(
            "claude-opus-4-1-20250805".to_string(),
            StaticModelPricing {
                input: 15.0,
                output: 75.0,
                cache_write: 18.75,
                cache_read: 1.5,
            },
        );

        // Claude 3 Sonnet
        static_pricing.insert(
            "claude-3-sonnet-20240229".to_string(),
            StaticModelPricing {
                input: 3.0,
                output: 15.0,
                cache_write: 3.75,
                cache_read: 0.3,
            },
        );

        // Claude 3 Haiku
        static_pricing.insert(
            "claude-3-haiku-20240307".to_string(),
            StaticModelPricing {
                input: 0.25,
                output: 1.25,
                cache_write: 0.3,
                cache_read: 0.03,
            },
        );

        // Unknown 默认定价
        static_pricing.insert(
            "unknown".to_string(),
            StaticModelPricing {
                input: 3.0,
                output: 15.0,
                cache_write: 3.75,
                cache_read: 0.3,
            },
        );

        Self {
            pricing_service,
            static_pricing,
        }
    }

    /// 计算单次请求费用
    pub async fn calculate_cost(&self, usage: &Usage, model: &str) -> CostCalculationResult {
        // 如果 usage 包含详细的 cache_creation 对象或是 1M 模型，使用 pricingService
        if usage.cache_creation.is_some() || model.contains("[1m]") {
            return self.calculate_cost_with_pricing_service(usage, model).await;
        }

        // 否则使用旧逻辑（向后兼容）
        self.calculate_cost_legacy(usage, model).await
    }

    /// 使用 pricingService 计算费用
    async fn calculate_cost_with_pricing_service(
        &self,
        usage: &Usage,
        model: &str,
    ) -> CostCalculationResult {
        let result = self.pricing_service.calculate_cost(usage, model).await;

        // 转换格式
        CostCalculationResult {
            model: model.to_string(),
            pricing: StaticModelPricing {
                input: result.pricing.input * 1_000_000.0, // 转换为 per 1M tokens
                output: result.pricing.output * 1_000_000.0,
                cache_write: result.pricing.cache_create * 1_000_000.0,
                cache_read: result.pricing.cache_read * 1_000_000.0,
            },
            using_dynamic_pricing: true,
            is_long_context_request: Some(result.is_long_context_request),
            usage: UsageDetails {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_create_tokens: usage.cache_creation_input_tokens,
                cache_read_tokens: usage.cache_read_input_tokens,
                total_tokens: usage.input_tokens
                    + usage.output_tokens
                    + usage.cache_creation_input_tokens
                    + usage.cache_read_input_tokens,
            },
            costs: CostDetails {
                input: result.input_cost,
                output: result.output_cost,
                cache_write: result.cache_create_cost,
                cache_read: result.cache_read_cost,
                total: result.total_cost,
            },
            formatted: FormattedCosts {
                input: self.format_cost(result.input_cost, 6),
                output: self.format_cost(result.output_cost, 6),
                cache_write: self.format_cost(result.cache_create_cost, 6),
                cache_read: self.format_cost(result.cache_read_cost, 6),
                total: self.format_cost(result.total_cost, 6),
            },
            debug: DebugInfo {
                is_openai_model: model.contains("gpt") || model.contains("o1"),
                has_cache_create_price: result.pricing.cache_create > 0.0,
                cache_create_tokens: usage.cache_creation_input_tokens,
                cache_write_price_used: result.pricing.cache_create * 1_000_000.0,
                is_long_context_model: Some(model.contains("[1m]")),
                is_long_context_request: Some(result.is_long_context_request),
            },
        }
    }

    /// 旧版计算逻辑（向后兼容）
    async fn calculate_cost_legacy(&self, usage: &Usage, model: &str) -> CostCalculationResult {
        // 优先使用动态价格服务
        let (pricing, using_dynamic_pricing) = if let Some(pricing_data) =
            self.pricing_service.get_model_pricing(model).await
        {
            let input_price = pricing_data.input_cost_per_token * 1_000_000.0; // 转换为 per 1M tokens
            let output_price = pricing_data.output_cost_per_token * 1_000_000.0;
            let cache_read_price =
                pricing_data.cache_read_input_token_cost.unwrap_or(0.0) * 1_000_000.0;

            // OpenAI 模型特殊处理
            let is_openai_model = model.contains("gpt")
                || model.contains("o1")
                || pricing_data.litellm_provider.as_deref() == Some("openai");

            let cache_write_price =
                if let Some(cache_create_price) = pricing_data.cache_creation_input_token_cost {
                    cache_create_price * 1_000_000.0
                } else if is_openai_model && usage.cache_creation_input_tokens > 0 {
                    // OpenAI 模型：缓存创建按普通 input 价格计费
                    input_price
                } else {
                    0.0
                };

            let pricing = StaticModelPricing {
                input: input_price,
                output: output_price,
                cache_write: cache_write_price,
                cache_read: cache_read_price,
            };

            (pricing, true)
        } else {
            // 回退到静态价格
            let pricing = self
                .static_pricing
                .get(model)
                .or_else(|| self.static_pricing.get("unknown"))
                .cloned()
                .unwrap();

            (pricing, false)
        };

        // 计算费用 (USD)
        let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * pricing.input;
        let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * pricing.output;
        let cache_write_cost =
            (usage.cache_creation_input_tokens as f64 / 1_000_000.0) * pricing.cache_write;
        let cache_read_cost =
            (usage.cache_read_input_tokens as f64 / 1_000_000.0) * pricing.cache_read;

        let total_cost = input_cost + output_cost + cache_write_cost + cache_read_cost;

        CostCalculationResult {
            model: model.to_string(),
            pricing: pricing.clone(),
            using_dynamic_pricing,
            is_long_context_request: None,
            usage: UsageDetails {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_create_tokens: usage.cache_creation_input_tokens,
                cache_read_tokens: usage.cache_read_input_tokens,
                total_tokens: usage.input_tokens
                    + usage.output_tokens
                    + usage.cache_creation_input_tokens
                    + usage.cache_read_input_tokens,
            },
            costs: CostDetails {
                input: input_cost,
                output: output_cost,
                cache_write: cache_write_cost,
                cache_read: cache_read_cost,
                total: total_cost,
            },
            formatted: FormattedCosts {
                input: self.format_cost(input_cost, 6),
                output: self.format_cost(output_cost, 6),
                cache_write: self.format_cost(cache_write_cost, 6),
                cache_read: self.format_cost(cache_read_cost, 6),
                total: self.format_cost(total_cost, 6),
            },
            debug: DebugInfo {
                is_openai_model: model.contains("gpt") || model.contains("o1"),
                has_cache_create_price: pricing.cache_write > 0.0,
                cache_create_tokens: usage.cache_creation_input_tokens,
                cache_write_price_used: pricing.cache_write,
                is_long_context_model: None,
                is_long_context_request: None,
            },
        }
    }

    /// 计算聚合使用量的费用
    pub async fn calculate_aggregated_cost(
        &self,
        aggregated_usage: &AggregatedUsage,
        model: &str,
    ) -> CostCalculationResult {
        let usage = Usage {
            input_tokens: aggregated_usage
                .input_tokens
                .or(aggregated_usage.total_input_tokens)
                .unwrap_or(0),
            output_tokens: aggregated_usage
                .output_tokens
                .or(aggregated_usage.total_output_tokens)
                .unwrap_or(0),
            cache_creation_input_tokens: aggregated_usage
                .cache_create_tokens
                .or(aggregated_usage.total_cache_create_tokens)
                .unwrap_or(0),
            cache_read_input_tokens: aggregated_usage
                .cache_read_tokens
                .or(aggregated_usage.total_cache_read_tokens)
                .unwrap_or(0),
            cache_creation: None,
        };

        self.calculate_cost(&usage, model).await
    }

    /// 获取模型定价
    pub fn get_model_pricing(&self, model: &str) -> StaticModelPricing {
        // 特殊处理：gpt-5-codex fallback
        if model == "gpt-5-codex" && !self.static_pricing.contains_key("gpt-5-codex") {
            if let Some(gpt5_pricing) = self.static_pricing.get("gpt-5") {
                println!("Using gpt-5 pricing as fallback for {}", model);
                return gpt5_pricing.clone();
            }
        }

        self.static_pricing
            .get(model)
            .or_else(|| self.static_pricing.get("unknown"))
            .cloned()
            .unwrap()
    }

    /// 获取所有模型定价
    pub fn get_all_model_pricing(&self) -> HashMap<String, StaticModelPricing> {
        self.static_pricing.clone()
    }

    /// 检查模型是否支持
    pub fn is_model_supported(&self, model: &str) -> bool {
        self.static_pricing.contains_key(model)
    }

    /// 格式化费用
    pub fn format_cost(&self, cost: f64, decimals: usize) -> String {
        if cost >= 1.0 {
            format!("${:.2}", cost)
        } else if cost >= 0.001 {
            format!("${:.4}", cost)
        } else {
            format!("${:.1$}", cost, decimals)
        }
    }

    /// 计算缓存节省
    pub async fn calculate_cache_savings(&self, usage: &Usage, model: &str) -> CacheSavings {
        let pricing = self.get_model_pricing(model);
        let cache_read_tokens = usage.cache_read_input_tokens as f64;

        // 如果不使用缓存，需要按正常 input 价格计费
        let normal_cost = (cache_read_tokens / 1_000_000.0) * pricing.input;
        let cache_cost = (cache_read_tokens / 1_000_000.0) * pricing.cache_read;
        let savings = normal_cost - cache_cost;
        let savings_percentage = if normal_cost > 0.0 {
            (savings / normal_cost) * 100.0
        } else {
            0.0
        };

        CacheSavings {
            normal_cost,
            cache_cost,
            savings,
            savings_percentage,
            formatted: FormattedSavings {
                normal_cost: self.format_cost(normal_cost, 6),
                cache_cost: self.format_cost(cache_cost, 6),
                savings: self.format_cost(savings, 6),
                savings_percentage: format!("{:.1}%", savings_percentage),
            },
        }
    }
}
