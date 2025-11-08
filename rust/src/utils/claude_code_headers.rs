/// Claude Code Headers 管理模块
///
/// 为非 Claude Code 客户端的请求添加必要的 Claude Code headers，
/// 使其能够通过 Claude Console 的客户端验证

use std::collections::HashMap;

use super::prompt_similarity::is_claude_code_prompt;

/// 默认的 Claude Code headers
///
/// 这些 headers 是 Claude Code CLI 客户端的标准 headers
pub fn get_default_claude_code_headers() -> HashMap<String, String> {
    let mut headers = HashMap::new();

    // Stainless SDK headers
    headers.insert("x-stainless-retry-count".to_string(), "0".to_string());
    headers.insert("x-stainless-timeout".to_string(), "60".to_string());
    headers.insert("x-stainless-lang".to_string(), "js".to_string());
    headers.insert("x-stainless-package-version".to_string(), "0.55.1".to_string());
    headers.insert("x-stainless-os".to_string(), "Linux".to_string());
    headers.insert("x-stainless-arch".to_string(), "x64".to_string());
    headers.insert("x-stainless-runtime".to_string(), "node".to_string());
    headers.insert("x-stainless-runtime-version".to_string(), "v20.19.2".to_string());

    // Claude Code specific headers
    headers.insert("anthropic-dangerous-direct-browser-access".to_string(), "true".to_string());
    headers.insert("x-app".to_string(), "cli".to_string());
    headers.insert("accept-language".to_string(), "*".to_string());
    headers.insert("sec-fetch-mode".to_string(), "cors".to_string());

    // anthropic-beta for extended features
    headers.insert("anthropic-beta".to_string(), "prompt-caching-2024-07-31,max-tokens-3-5-sonnet-2024-07-15".to_string());

    headers
}

/// Claude Code headers 的关键字列表
///
/// 用于从客户端请求中提取这些 headers
pub fn get_claude_code_header_keys() -> Vec<&'static str> {
    vec![
        "x-stainless-retry-count",
        "x-stainless-timeout",
        "x-stainless-lang",
        "x-stainless-package-version",
        "x-stainless-os",
        "x-stainless-arch",
        "x-stainless-runtime",
        "x-stainless-runtime-version",
        "anthropic-dangerous-direct-browser-access",
        "x-app",
        "accept-language",
        "sec-fetch-mode",
        "accept-encoding",
        "anthropic-beta",
    ]
}

/// 从请求体中提取系统提示词文本
///
/// 支持多种格式：
/// - system: "string"
/// - system: [{"type": "text", "text": "..."}, ...]
fn extract_system_prompt(request_body: &serde_json::Value) -> Option<String> {
    let system = request_body.get("system")?;

    // 情况1: system 是字符串
    if let Some(text) = system.as_str() {
        return Some(text.to_string());
    }

    // 情况2: system 是数组 [{"type": "text", "text": "..."}, ...]
    if let Some(system_array) = system.as_array() {
        let mut full_text = String::new();

        for item in system_array {
            if let Some(obj) = item.as_object() {
                // 检查 type 是否为 "text"
                if let Some(item_type) = obj.get("type") {
                    if item_type.as_str() == Some("text") {
                        if let Some(text) = obj.get("text") {
                            if let Some(text_str) = text.as_str() {
                                if !full_text.is_empty() {
                                    full_text.push(' ');
                                }
                                full_text.push_str(text_str);
                            }
        }
                    }
                }
            } else if let Some(text) = item.as_str() {
                // 直接是字符串
                if !full_text.is_empty() {
                    full_text.push(' ');
                }
                full_text.push_str(text);
            }
        }

        if !full_text.is_empty() {
            return Some(full_text);
        }
    }

    None
}

/// 检查请求体是否来自真实的 Claude Code 客户端
///
/// 使用多层验证：
/// 1. 系统提示词相似度匹配（主要方法）
/// 2. metadata.user_id 格式检查（辅助方法）
///
/// # Arguments
///
/// * `request_body` - JSON 格式的请求体
///
/// # Returns
///
/// `true` 如果检测到这是真实的 Claude Code 请求
///
/// # Examples
///
/// ```
/// use serde_json::json;
///
/// let body = json!({
///     "system": "You are Claude Code, Anthropic's official CLI for Claude.",
///     "messages": []
/// });
/// assert!(is_real_claude_code_request(&body));
/// ```
pub fn is_real_claude_code_request(request_body: &serde_json::Value) -> bool {
    // 方法1: 检查系统提示词相似度（主要方法，准确度高）
    if let Some(system_prompt) = extract_system_prompt(request_body) {
        if is_claude_code_prompt(&system_prompt) {
            return true;
        }
    }

    // 方法2: 检查 metadata.user_id 字段（辅助方法，作为备用）
    if let Some(metadata) = request_body.get("metadata") {
        if let Some(user_id) = metadata.get("user_id") {
            if let Some(user_id_str) = user_id.as_str() {
                // Claude Code 的 user_id 格式: user_{64位hex}_account__session_{uuid}
                if user_id_str.starts_with("user_") && user_id_str.contains("_account__session_") {
                    return true;
                }
            }
        }
    }

    false
}

/// 从客户端 headers 中提取 Claude Code 相关的 headers
pub fn extract_claude_code_headers(client_headers: &axum::http::HeaderMap) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    let keys = get_claude_code_header_keys();

    for key in keys {
        if let Some(value) = client_headers.get(key) {
            if let Ok(value_str) = value.to_str() {
                headers.insert(key.to_string(), value_str.to_string());
            }
        }
    }

    headers
}

/// 合并 headers：客户端提供的 + 默认的（客户端没有提供的才添加）
pub fn merge_claude_code_headers(
    client_headers: HashMap<String, String>,
    is_real_claude_code: bool,
) -> HashMap<String, String> {
    if is_real_claude_code {
        // 如果是真实的 Claude Code 请求，直接使用客户端的 headers
        return client_headers;
    }

    // 不是真实的 Claude Code 请求，添加默认 headers
    let mut merged = get_default_claude_code_headers();

    // 客户端提供的 headers 会覆盖默认值
    for (key, value) in client_headers {
        merged.insert(key, value);
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_default_headers() {
        let headers = get_default_claude_code_headers();
        assert!(headers.contains_key("x-app"));
        assert_eq!(headers.get("x-app").unwrap(), "cli");
        assert!(headers.contains_key("anthropic-beta"));
    }

    #[test]
    fn test_extract_system_prompt_string() {
        let body = json!({
            "system": "You are Claude Code, Anthropic's official CLI for Claude."
        });
        let prompt = extract_system_prompt(&body);
        assert!(prompt.is_some());
        assert_eq!(
            prompt.unwrap(),
            "You are Claude Code, Anthropic's official CLI for Claude."
        );
    }

    #[test]
    fn test_extract_system_prompt_array() {
        let body = json!({
            "system": [
                {"type": "text", "text": "You are Claude Code,"},
                {"type": "text", "text": "Anthropic's official CLI for Claude."}
            ]
        });
        let prompt = extract_system_prompt(&body);
        assert!(prompt.is_some());
        assert_eq!(
            prompt.unwrap(),
            "You are Claude Code, Anthropic's official CLI for Claude."
        );
    }

    #[test]
    fn test_extract_system_prompt_none() {
        let body = json!({
            "model": "claude-3-5-sonnet-20241022"
        });
        let prompt = extract_system_prompt(&body);
        assert!(prompt.is_none());
    }

    #[test]
    fn test_is_real_claude_code_request_with_system_prompt() {
        let body = json!({
            "system": "You are Claude Code, Anthropic's official CLI for Claude.",
            "messages": []
        });
        assert!(
            is_real_claude_code_request(&body),
            "应该通过系统提示词相似度检测到 Claude Code"
        );
    }

    #[test]
    fn test_is_real_claude_code_request_with_array_system() {
        let body = json!({
            "system": [
                {"type": "text", "text": "You are Claude Code, Anthropic's official CLI for Claude."}
            ],
            "messages": []
        });
        assert!(
            is_real_claude_code_request(&body),
            "应该支持数组格式的 system 字段"
        );
    }

    #[test]
    fn test_is_real_claude_code_request_with_user_id() {
        let body = json!({
            "metadata": {
                "user_id": "user_1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef_account__session_12345678-1234-1234-1234-123456789012"
            }
        });
        assert!(
            is_real_claude_code_request(&body),
            "应该通过 metadata.user_id 检测到 Claude Code"
        );
    }

    #[test]
    fn test_is_real_claude_code_request_with_agent_sdk_prompt() {
        let body = json!({
            "system": "You are a Claude agent, built on Anthropic's Claude Agent SDK.",
            "messages": []
        });
        assert!(
            is_real_claude_code_request(&body),
            "应该识别 Agent SDK 提示词"
        );
    }

    #[test]
    fn test_is_real_claude_code_request_custom_prompt() {
        let body = json!({
            "system": "You are a helpful assistant that answers questions.",
            "messages": []
        });
        assert!(
            !is_real_claude_code_request(&body),
            "自定义提示词不应该被识别为 Claude Code"
        );
    }

    #[test]
    fn test_is_real_claude_code_request_without_system() {
        let body = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": []
        });
        assert!(
            !is_real_claude_code_request(&body),
            "没有 system 字段的请求不应该被识别"
        );
    }

    #[test]
    fn test_merge_headers_real_claude_code() {
        let client_headers = HashMap::from([
            ("x-app".to_string(), "custom-cli".to_string()),
        ]);

        let merged = merge_claude_code_headers(client_headers.clone(), true);

        // 真实的 Claude Code 请求应该保持客户端的 headers
        assert_eq!(merged.get("x-app").unwrap(), "custom-cli");
        assert_eq!(merged.len(), 1); // 只有客户端提供的
    }

    #[test]
    fn test_merge_headers_not_real_claude_code() {
        let client_headers = HashMap::new();

        let merged = merge_claude_code_headers(client_headers, false);

        // 不是真实的 Claude Code 请求应该添加默认 headers
        assert!(merged.contains_key("x-app"));
        assert_eq!(merged.get("x-app").unwrap(), "cli");
        assert!(merged.len() > 10); // 应该有很多默认 headers
    }
}
