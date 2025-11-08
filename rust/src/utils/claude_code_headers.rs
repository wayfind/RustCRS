/// Claude Code Headers 管理模块
///
/// 为非 Claude Code 客户端的请求添加必要的 Claude Code headers，
/// 使其能够通过 Claude Console 的客户端验证

use std::collections::HashMap;

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

/// 检查请求体是否包含 Claude Code 系统提示词的特征
///
/// 简化版本：检查是否有 system 字段和特定的内容模式
pub fn is_real_claude_code_request(request_body: &serde_json::Value) -> bool {
    // 检查是否有 system 字段
    if let Some(system) = request_body.get("system") {
        // 如果 system 是数组，检查是否有内容
        if let Some(system_array) = system.as_array() {
            if !system_array.is_empty() {
                // 简单判断：如果有 system 字段且不为空，可能是 Claude Code
                // 更严格的验证需要检查系统提示词的相似度，但这里简化处理
                return true;
            }
        }
    }

    // 检查 metadata.user_id 字段（Claude Code 特有）
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
    fn test_is_real_claude_code_request_with_user_id() {
        let body = json!({
            "metadata": {
                "user_id": "user_1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef_account__session_12345678-1234-1234-1234-123456789012"
            }
        });
        assert!(is_real_claude_code_request(&body));
    }

    #[test]
    fn test_is_real_claude_code_request_without_markers() {
        let body = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": []
        });
        assert!(!is_real_claude_code_request(&body));
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
