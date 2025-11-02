// Model Helper - 模型名称解析工具
//
// 功能：
// 1. 解析 vendor 前缀（如 ccr:claude-3-5-sonnet）
// 2. 提取基础模型名称
// 3. 模型名称规范化

use serde::{Deserialize, Serialize};

/// 解析结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedModel {
    /// 供应商前缀（如 "ccr", "bedrock"）
    pub vendor: Option<String>,
    /// 基础模型名称（去除前缀后的名称）
    pub base_model: String,
    /// 原始模型名称
    pub original: String,
}

/// 解析供应商前缀的模型名称
///
/// # 格式
/// - `ccr:claude-3-5-sonnet` → vendor=Some("ccr"), base_model="claude-3-5-sonnet"
/// - `bedrock:us.anthropic.claude-sonnet-4` → vendor=Some("bedrock"), base_model="us.anthropic.claude-sonnet-4"
/// - `claude-3-5-sonnet` → vendor=None, base_model="claude-3-5-sonnet"
///
/// # Examples
/// ```
/// use claude_relay::utils::model_helper::parse_vendor_prefixed_model;
///
/// let parsed = parse_vendor_prefixed_model("ccr:claude-3-5-sonnet");
/// assert_eq!(parsed.vendor, Some("ccr".to_string()));
/// assert_eq!(parsed.base_model, "claude-3-5-sonnet");
///
/// let parsed2 = parse_vendor_prefixed_model("claude-3-5-sonnet");
/// assert_eq!(parsed2.vendor, None);
/// assert_eq!(parsed2.base_model, "claude-3-5-sonnet");
/// ```
pub fn parse_vendor_prefixed_model(model_name: &str) -> ParsedModel {
    if let Some(colon_pos) = model_name.find(':') {
        // 有冒号，可能是 vendor:model 格式
        let vendor = &model_name[..colon_pos];
        let base_model = &model_name[colon_pos + 1..];

        // 验证 vendor 是否为已知的供应商前缀
        let known_vendors = ["ccr", "bedrock", "azure", "custom"];
        if known_vendors.contains(&vendor) {
            return ParsedModel {
                vendor: Some(vendor.to_string()),
                base_model: base_model.to_string(),
                original: model_name.to_string(),
            };
        }
    }

    // 没有有效的 vendor 前缀，返回原始名称
    ParsedModel {
        vendor: None,
        base_model: model_name.to_string(),
        original: model_name.to_string(),
    }
}

/// 检查模型名称是否包含指定的关键词
///
/// # Examples
/// ```
/// use claude_relay::utils::model_helper::model_contains;
///
/// assert!(model_contains("claude-3-5-sonnet", "sonnet"));
/// assert!(model_contains("claude-opus-4-1", "opus"));
/// assert!(!model_contains("claude-3-5-haiku", "sonnet"));
/// ```
pub fn model_contains(model_name: &str, keyword: &str) -> bool {
    model_name.to_lowercase().contains(&keyword.to_lowercase())
}

/// 检查是否为 Claude 官方模型
///
/// Claude 官方模型包含：claude-, sonnet, opus, haiku 等关键词
pub fn is_claude_official_model(model_name: &str) -> bool {
    let lower = model_name.to_lowercase();
    lower.starts_with("claude-")
        || lower.contains("claude")
        || lower.contains("sonnet")
        || lower.contains("opus")
        || lower.contains("haiku")
}

/// 检查是否为 Opus 模型
pub fn is_opus_model(model_name: &str) -> bool {
    model_contains(model_name, "opus")
}

/// 检查是否为 Sonnet 模型
pub fn is_sonnet_model(model_name: &str) -> bool {
    model_contains(model_name, "sonnet")
}

/// 检查是否为 Haiku 模型
pub fn is_haiku_model(model_name: &str) -> bool {
    model_contains(model_name, "haiku")
}

/// 移除 Bedrock 区域前缀
///
/// 将 "us.anthropic.claude-sonnet-4" → "anthropic.claude-sonnet-4"
///
/// # Examples
/// ```
/// use claude_relay::utils::model_helper::remove_bedrock_region_prefix;
///
/// assert_eq!(remove_bedrock_region_prefix("us.anthropic.claude-sonnet-4"), "anthropic.claude-sonnet-4");
/// assert_eq!(remove_bedrock_region_prefix("eu.anthropic.claude-opus-4"), "anthropic.claude-opus-4");
/// assert_eq!(remove_bedrock_region_prefix("anthropic.claude-sonnet-4"), "anthropic.claude-sonnet-4");
/// ```
pub fn remove_bedrock_region_prefix(model_name: &str) -> String {
    // Bedrock 区域前缀：us., eu., apac.
    let regions = ["us.", "eu.", "apac.", "ap-", "ca-"];

    for region in &regions {
        if let Some(stripped) = model_name.strip_prefix(region) {
            return stripped.to_string();
        }
    }

    model_name.to_string()
}

/// 规范化模型名称（用于模糊匹配）
///
/// 移除 `-` 和 `_`，转换为小写
///
/// # Examples
/// ```
/// use claude_relay::utils::model_helper::normalize_model_name;
///
/// assert_eq!(normalize_model_name("claude-3-5-sonnet"), "claude35sonnet");
/// assert_eq!(normalize_model_name("claude_3_5_sonnet"), "claude35sonnet");
/// ```
pub fn normalize_model_name(model_name: &str) -> String {
    model_name.to_lowercase().replace(['-', '_'], "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vendor_prefixed_model() {
        // 有 vendor 前缀
        let parsed = parse_vendor_prefixed_model("ccr:claude-3-5-sonnet");
        assert_eq!(parsed.vendor, Some("ccr".to_string()));
        assert_eq!(parsed.base_model, "claude-3-5-sonnet");
        assert_eq!(parsed.original, "ccr:claude-3-5-sonnet");

        // 无 vendor 前缀
        let parsed2 = parse_vendor_prefixed_model("claude-3-5-sonnet");
        assert_eq!(parsed2.vendor, None);
        assert_eq!(parsed2.base_model, "claude-3-5-sonnet");

        // 有冒号但不是已知 vendor
        let parsed3 = parse_vendor_prefixed_model("unknown:claude-3-5-sonnet");
        assert_eq!(parsed3.vendor, None);
        assert_eq!(parsed3.base_model, "unknown:claude-3-5-sonnet");
    }

    #[test]
    fn test_is_claude_official_model() {
        assert!(is_claude_official_model("claude-3-5-sonnet"));
        assert!(is_claude_official_model("claude-opus-4-1"));
        assert!(is_claude_official_model("sonnet-3-5"));
        assert!(!is_claude_official_model("gpt-4"));
        assert!(!is_claude_official_model("deepseek-chat"));
    }

    #[test]
    fn test_opus_sonnet_haiku_detection() {
        assert!(is_opus_model("claude-opus-4-1"));
        assert!(is_sonnet_model("claude-3-5-sonnet"));
        assert!(is_haiku_model("claude-3-5-haiku"));

        assert!(!is_opus_model("claude-3-5-sonnet"));
        assert!(!is_sonnet_model("claude-opus-4-1"));
    }

    #[test]
    fn test_remove_bedrock_region_prefix() {
        assert_eq!(
            remove_bedrock_region_prefix("us.anthropic.claude-sonnet-4"),
            "anthropic.claude-sonnet-4"
        );
        assert_eq!(
            remove_bedrock_region_prefix("eu.anthropic.claude-opus-4"),
            "anthropic.claude-opus-4"
        );
        assert_eq!(
            remove_bedrock_region_prefix("anthropic.claude-sonnet-4"),
            "anthropic.claude-sonnet-4"
        );
    }

    #[test]
    fn test_normalize_model_name() {
        assert_eq!(normalize_model_name("claude-3-5-sonnet"), "claude35sonnet");
        assert_eq!(normalize_model_name("claude_3_5_sonnet"), "claude35sonnet");
        assert_eq!(normalize_model_name("Claude-3-5-Sonnet"), "claude35sonnet");
    }

    #[test]
    fn test_model_contains() {
        assert!(model_contains("claude-3-5-sonnet", "sonnet"));
        assert!(model_contains("CLAUDE-OPUS-4", "opus"));
        assert!(!model_contains("claude-3-5-haiku", "sonnet"));
    }
}
