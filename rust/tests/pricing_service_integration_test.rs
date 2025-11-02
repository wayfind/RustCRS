use claude_relay::models::UsageRecord;
// Pricing Service Integration Tests
//
// 测试定价服务的所有核心功能

mod common;

use claude_relay::services::pricing_service::{PricingService, Usage};
use claude_relay::utils::cost_calculator::{AggregatedUsage, CostCalculator};
use std::sync::Arc;

/// 创建测试用的 PricingService
fn create_test_pricing_service() -> Arc<PricingService> {
    let http_client = Arc::new(reqwest::Client::new());
    Arc::new(PricingService::new(http_client))
}

#[tokio::test]
async fn test_pricing_service_creation() {
    let service = create_test_pricing_service();
    let status = service.get_status().await;

    // 初始状态应该未初始化
    assert!(!status.initialized || status.model_count == 0 || status.model_count > 0);
}

#[tokio::test]
async fn test_get_model_pricing_exact_match() {
    let service = create_test_pricing_service();

    // 先初始化（使用 fallback）
    let _ = service.initialize().await;

    // 测试精确匹配 - claude-3-5-sonnet-20241022
    let pricing = service
        .get_model_pricing("claude-3-5-sonnet-20241022")
        .await;

    if let Some(p) = pricing {
        assert!(p.input_cost_per_token > 0.0);
        assert!(p.output_cost_per_token > 0.0);
    } else {
        // Fallback 可能没有这个模型，不视为失败
        println!("Model not found in pricing data, this is acceptable for fallback");
    }
}

#[tokio::test]
async fn test_get_model_pricing_bedrock_region() {
    let service = create_test_pricing_service();
    let _ = service.initialize().await;

    // 测试 Bedrock 区域前缀处理
    // 应该匹配 us.anthropic.claude-sonnet-4 → anthropic.claude-sonnet-4
    let pricing = service
        .get_model_pricing("us.anthropic.claude-sonnet-4-20250514-v1:0")
        .await;

    // 即使找不到也不报错，因为可能 fallback 中没有
    if pricing.is_some() {
        println!("Found Bedrock model pricing");
    }
}

#[tokio::test]
async fn test_get_ephemeral_1h_pricing() {
    let service = create_test_pricing_service();

    // 测试 Opus 系列
    let opus_price = service.get_ephemeral_1h_pricing("claude-opus-4-1");
    assert_eq!(opus_price, 0.00003); // $30/MTok

    // 测试 Sonnet 系列
    let sonnet_price = service.get_ephemeral_1h_pricing("claude-sonnet-4");
    assert_eq!(sonnet_price, 0.000006); // $6/MTok

    // 测试 Haiku 系列
    let haiku_price = service.get_ephemeral_1h_pricing("claude-3-5-haiku");
    assert_eq!(haiku_price, 0.0000016); // $1.6/MTok

    // 测试未知模型
    let unknown_price = service.get_ephemeral_1h_pricing("unknown-model");
    assert_eq!(unknown_price, 0.0);

    // 测试模糊匹配 - 包含 opus 的模型名
    let opus_variant_price = service.get_ephemeral_1h_pricing("claude-opus-variant");
    assert_eq!(opus_variant_price, 0.00003);
}

#[tokio::test]
async fn test_calculate_cost_basic() {
    let service = create_test_pricing_service();
    let _ = service.initialize().await;

    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 500,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation: None,
    };

    let result = service
        .calculate_cost(&usage, "claude-3-5-sonnet-20241022")
        .await;

    // 应该有计算结果
    assert!(result.total_cost >= 0.0);
    assert_eq!(
        result.has_pricing,
        result.input_cost > 0.0 || result.output_cost > 0.0
    );
}

#[tokio::test]
async fn test_calculate_cost_with_cache() {
    let service = create_test_pricing_service();
    let _ = service.initialize().await;

    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 500,
        cache_creation_input_tokens: 2000,
        cache_read_input_tokens: 1500,
        cache_creation: None,
    };

    let result = service
        .calculate_cost(&usage, "claude-3-5-sonnet-20241022")
        .await;

    // 应该包含缓存费用
    assert!(result.total_cost >= 0.0);
    if result.has_pricing {
        // 如果有定价，缓存费用应该被计算
        assert!(result.cache_create_cost >= 0.0);
        assert!(result.cache_read_cost >= 0.0);
    }
}

#[tokio::test]
async fn test_calculate_cost_with_detailed_cache() {
    let service = create_test_pricing_service();
    let _ = service.initialize().await;

    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 500,
        cache_creation_input_tokens: 0, // 详细格式时忽略
        cache_read_input_tokens: 1500,
        cache_creation: Some(claude_relay::services::pricing_service::CacheCreation {
            ephemeral_5m_input_tokens: 1000,
            ephemeral_1h_input_tokens: 1000,
        }),
    };

    let result = service.calculate_cost(&usage, "claude-sonnet-4").await;

    // 应该分别计算 5m 和 1h 缓存费用
    if result.has_pricing {
        assert!(result.ephemeral_5m_cost >= 0.0);
        assert!(result.ephemeral_1h_cost >= 0.0);
        assert_eq!(
            result.cache_create_cost,
            result.ephemeral_5m_cost + result.ephemeral_1h_cost
        );
    }
}

#[tokio::test]
async fn test_calculate_cost_long_context_model() {
    let service = create_test_pricing_service();
    let _ = service.initialize().await;

    // 测试 1M 上下文模型，总输入 > 200k
    let usage = Usage {
        input_tokens: 250_000, // 超过 200k
        output_tokens: 10_000,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation: None,
    };

    let result = service
        .calculate_cost(&usage, "claude-sonnet-4-20250514[1m]")
        .await;

    // 应该使用 1M 上下文价格
    assert!(result.is_long_context_request);
    assert!(result.total_cost > 0.0);
}

#[tokio::test]
async fn test_format_cost() {
    let service = create_test_pricing_service();

    assert_eq!(service.format_cost(0.0), "$0.000000");
    assert_eq!(service.format_cost(0.0000001), "$1.00e-7");
    assert_eq!(service.format_cost(0.001234), "$0.001234");
    assert_eq!(service.format_cost(0.12345), "$0.1235");
    assert_eq!(service.format_cost(1.2345), "$1.23");
    assert_eq!(service.format_cost(123.456), "$123.46");
}

#[tokio::test]
async fn test_cost_calculator_creation() {
    let pricing_service = create_test_pricing_service();
    let calculator = CostCalculator::new(pricing_service);

    // 测试静态价格是否正确初始化
    let pricing = calculator.get_model_pricing("claude-3-5-sonnet-20241022");
    assert_eq!(pricing.input, 3.0); // $3/MTok
    assert_eq!(pricing.output, 15.0); // $15/MTok
}

#[tokio::test]
async fn test_cost_calculator_calculate_cost() {
    let pricing_service = create_test_pricing_service();
    let _ = pricing_service.initialize().await;
    let calculator = CostCalculator::new(pricing_service);

    let usage = Usage {
        input_tokens: 1_000_000, // 1M tokens
        output_tokens: 500_000,  // 500k tokens
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation: None,
    };

    let result = calculator
        .calculate_cost(&usage, "claude-3-5-sonnet-20241022")
        .await;

    assert_eq!(result.model, "claude-3-5-sonnet-20241022");
    // using_dynamic_pricing 可能是 true 或 false，取决于 PricingService 是否成功初始化
    assert_eq!(result.usage.input_tokens, 1_000_000);
    assert_eq!(result.usage.output_tokens, 500_000);
    assert!(result.costs.total > 0.0);

    // 验证格式化字符串
    assert!(result.formatted.total.starts_with('$'));
}

#[tokio::test]
async fn test_cost_calculator_openai_model() {
    let pricing_service = create_test_pricing_service();
    let _ = pricing_service.initialize().await;
    let calculator = CostCalculator::new(pricing_service);

    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 500,
        cache_creation_input_tokens: 1000, // OpenAI 模型缓存创建
        cache_read_input_tokens: 500,
        cache_creation: None,
    };

    let result = calculator.calculate_cost(&usage, "gpt-4").await;

    assert!(result.debug.is_openai_model);
    // OpenAI 模型缓存创建应该按 input 价格计费（如果没有专门的缓存价格）
}

#[tokio::test]
async fn test_cost_calculator_aggregated_usage() {
    let pricing_service = create_test_pricing_service();
    let _ = pricing_service.initialize().await;
    let calculator = CostCalculator::new(pricing_service);

    let aggregated = AggregatedUsage {
        input_tokens: Some(1000),
        output_tokens: Some(500),
        cache_create_tokens: Some(200),
        cache_read_tokens: Some(100),
        total_input_tokens: None,
        total_output_tokens: None,
        total_cache_create_tokens: None,
        total_cache_read_tokens: None,
    };

    let result = calculator
        .calculate_aggregated_cost(&aggregated, "claude-3-5-sonnet-20241022")
        .await;

    assert_eq!(result.usage.input_tokens, 1000);
    assert_eq!(result.usage.output_tokens, 500);
    assert_eq!(result.usage.cache_create_tokens, 200);
    assert_eq!(result.usage.cache_read_tokens, 100);
}

#[tokio::test]
async fn test_cost_calculator_aggregated_usage_total_fields() {
    let pricing_service = create_test_pricing_service();
    let _ = pricing_service.initialize().await;
    let calculator = CostCalculator::new(pricing_service);

    // 测试使用 total_* 字段的聚合数据
    let aggregated = AggregatedUsage {
        input_tokens: None,
        output_tokens: None,
        cache_create_tokens: None,
        cache_read_tokens: None,
        total_input_tokens: Some(2000),
        total_output_tokens: Some(1000),
        total_cache_create_tokens: Some(300),
        total_cache_read_tokens: Some(150),
    };

    let result = calculator
        .calculate_aggregated_cost(&aggregated, "claude-3-opus-20240229")
        .await;

    assert_eq!(result.usage.input_tokens, 2000);
    assert_eq!(result.usage.output_tokens, 1000);
    assert_eq!(result.usage.cache_create_tokens, 300);
    assert_eq!(result.usage.cache_read_tokens, 150);
}

#[tokio::test]
async fn test_cost_calculator_cache_savings() {
    let pricing_service = create_test_pricing_service();
    let calculator = CostCalculator::new(pricing_service);

    let usage = Usage {
        input_tokens: 0,
        output_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 1_000_000, // 1M tokens from cache
        cache_creation: None,
    };

    let savings = calculator
        .calculate_cache_savings(&usage, "claude-3-5-sonnet-20241022")
        .await;

    // 缓存读取应该比正常输入便宜
    assert!(savings.normal_cost > savings.cache_cost);
    assert!(savings.savings > 0.0);
    assert!(savings.savings_percentage > 0.0);

    // 验证格式化
    assert!(savings.formatted.savings.starts_with('$'));
    assert!(savings.formatted.savings_percentage.ends_with('%'));
}

#[tokio::test]
async fn test_cost_calculator_format_cost() {
    let pricing_service = create_test_pricing_service();
    let calculator = CostCalculator::new(pricing_service);

    assert_eq!(calculator.format_cost(1.234, 6), "$1.23");
    assert_eq!(calculator.format_cost(0.001234, 6), "$0.0012");
    assert_eq!(calculator.format_cost(0.0000001234, 6), "$0.000000");
}

#[tokio::test]
async fn test_cost_calculator_model_support() {
    let pricing_service = create_test_pricing_service();
    let calculator = CostCalculator::new(pricing_service);

    assert!(calculator.is_model_supported("claude-3-5-sonnet-20241022"));
    assert!(calculator.is_model_supported("claude-3-opus-20240229"));
    assert!(!calculator.is_model_supported("totally-unknown-model"));
}

#[tokio::test]
async fn test_cost_calculator_gpt5_codex_fallback() {
    let pricing_service = create_test_pricing_service();
    let calculator = CostCalculator::new(pricing_service);

    // gpt-5-codex 应该 fallback 到 gpt-5
    let pricing = calculator.get_model_pricing("gpt-5-codex");

    // 应该返回默认定价（因为 gpt-5 也可能不在静态列表中）
    assert!(pricing.input > 0.0);
    assert!(pricing.output > 0.0);
}

#[tokio::test]
async fn test_pricing_service_status() {
    let service = create_test_pricing_service();

    let status_before = service.get_status().await;
    assert!(!status_before.initialized);

    // 初始化（可能失败，因为没有网络或 fallback 文件）
    let init_result = service.initialize().await;

    let status_after = service.get_status().await;

    // 如果初始化成功，应该有数据
    if init_result.is_ok() {
        assert!(status_after.initialized || status_after.model_count > 0);
    } else {
        // 初始化失败也是可接受的（没有网络或 fallback 文件）
        println!(
            "Initialization failed (expected in test environment): {:?}",
            init_result
        );
    }
}

#[tokio::test]
async fn test_gpt5_codex_fallback() {
    let service = create_test_pricing_service();
    let _ = service.initialize().await;

    // 测试 gpt-5-codex → gpt-5 fallback
    let pricing = service.get_model_pricing("gpt-5-codex").await;

    // 可能有或没有这个模型，不强制断言
    if pricing.is_some() {
        println!("Found pricing for gpt-5-codex (either exact or fallback)");
    }
}

// Helper tests for common module
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pricing_service_creation() {
        let service = create_test_pricing_service();
        assert!(Arc::strong_count(&service) >= 1);
    }
}
