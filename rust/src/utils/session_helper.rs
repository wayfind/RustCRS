// Session Helper
//
// 会话哈希生成工具，用于粘性会话保持
// 基于 Anthropic 的 prompt caching 机制，优先使用 metadata 中的 session ID

use sha2::{Digest, Sha256};
use tracing::debug;

/// 生成会话哈希，用于 sticky 会话保持
///
/// 优先级顺序：
/// 1. metadata.user_id 中的 session ID
/// 2. 带 cache_control: {"type": "ephemeral"} 的内容
/// 3. system 内容
/// 4. 第一条消息内容
///
/// # Arguments
/// * `request_body` - Claude 请求体的 JSON 值
///
/// # Returns
/// * `Some(String)` - 32字符的会话哈希
/// * `None` - 无法生成会话哈希
pub fn generate_session_hash(request_body: &serde_json::Value) -> Option<String> {
    // 1. 最高优先级：使用 metadata 中的 session ID
    if let Some(metadata) = request_body.get("metadata") {
        if let Some(user_id) = metadata.get("user_id").and_then(|v| v.as_str()) {
            // 提取 session_xxx 部分
            if let Some(session_id) = extract_session_id(user_id) {
                debug!(
                    "📋 Session ID extracted from metadata.user_id: {}",
                    session_id
                );
                return Some(session_id);
            }
        }
    }

    let system = request_body.get("system");
    let messages = request_body.get("messages").and_then(|v| v.as_array());

    // 2. 提取带有 cache_control: {"type": "ephemeral"} 的内容
    let mut cacheable_content = String::new();

    // 检查 system 中的 cacheable 内容
    if let Some(sys) = system {
        if let Some(array) = sys.as_array() {
            for part in array {
                if has_ephemeral_cache_control(part) {
                    if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                        cacheable_content.push_str(text);
                    }
                }
            }
        }
    }

    // 检查 messages 中的 cacheable 内容
    if let Some(msgs) = messages {
        for msg in msgs {
            if message_has_cache_control(msg) {
                // 提取所有消息文本
                for message in msgs {
                    if let Some(text) = extract_message_text(message) {
                        cacheable_content.push_str(&text);
                        break;
                    }
                }
                break;
            }
        }
    }

    // 3. 如果有 cacheable 内容，直接使用
    if !cacheable_content.is_empty() {
        let hash = compute_hash(&cacheable_content);
        debug!("📋 Session hash generated from cacheable content: {}", hash);
        return Some(hash);
    }

    // 4. Fallback: 使用 system 内容
    if let Some(sys) = system {
        if let Some(system_text) = extract_system_text(sys) {
            if !system_text.is_empty() {
                let hash = compute_hash(&system_text);
                debug!("📋 Session hash generated from system content: {}", hash);
                return Some(hash);
            }
        }
    }

    // 5. 最后 fallback: 使用第一条消息内容
    if let Some(msgs) = messages {
        if let Some(first_msg) = msgs.first() {
            if let Some(first_text) = extract_message_text(first_msg) {
                if !first_text.is_empty() {
                    let hash = compute_hash(&first_text);
                    debug!("📋 Session hash generated from first message: {}", hash);
                    return Some(hash);
                }
            }
        }
    }

    // 无法生成会话哈希
    debug!("📋 Unable to generate session hash - no suitable content found");
    None
}

/// 从 user_id 中提取 session ID
fn extract_session_id(user_id: &str) -> Option<String> {
    // 匹配 session_xxx 格式，xxx 是 UUID (36字符)
    if let Some(start) = user_id.find("session_") {
        let session_part = &user_id[start + 8..]; // "session_" 长度为 8
        if session_part.len() >= 36 {
            let session_id = &session_part[..36];
            // 验证是否为有效的 UUID 格式 (带连字符的36字符)
            if is_valid_uuid(session_id) {
                return Some(session_id.to_string());
            }
        }
    }
    None
}

/// 验证 UUID 格式
fn is_valid_uuid(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// 检查 JSON 值是否有 ephemeral cache_control
fn has_ephemeral_cache_control(value: &serde_json::Value) -> bool {
    value
        .get("cache_control")
        .and_then(|cc| cc.get("type"))
        .and_then(|t| t.as_str())
        == Some("ephemeral")
}

/// 检查消息是否有 cache_control
fn message_has_cache_control(msg: &serde_json::Value) -> bool {
    // 检查消息级别的 cache_control
    if has_ephemeral_cache_control(msg) {
        return true;
    }

    // 检查 content 数组中的 cache_control
    if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
        for part in content {
            if has_ephemeral_cache_control(part) {
                return true;
            }
        }
    }

    false
}

/// 提取 system 文本
fn extract_system_text(system: &serde_json::Value) -> Option<String> {
    if let Some(s) = system.as_str() {
        Some(s.to_string())
    } else if let Some(array) = system.as_array() {
        let text = array
            .iter()
            .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<&str>>()
            .join("");
        if !text.is_empty() {
            Some(text)
        } else {
            None
        }
    } else {
        None
    }
}

/// 提取消息文本
fn extract_message_text(message: &serde_json::Value) -> Option<String> {
    if let Some(content) = message.get("content") {
        if let Some(s) = content.as_str() {
            return Some(s.to_string());
        } else if let Some(array) = content.as_array() {
            let text = array
                .iter()
                .filter(|part| part.get("type").and_then(|v| v.as_str()) == Some("text"))
                .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
                .collect::<Vec<&str>>()
                .join("");
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// 计算 SHA256 哈希（取前32个字符）
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..32].to_string()
}

/// 验证会话哈希格式
pub fn is_valid_session_hash(session_hash: &str) -> bool {
    session_hash.len() == 32 && session_hash.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_session_id() {
        let user_id = "session_550e8400-e29b-41d4-a716-446655440000";
        let result = extract_session_id(user_id);
        assert_eq!(
            result,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_is_valid_uuid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid("550e8400e29b41d4a716446655440000")); // 没有连字符
    }

    #[test]
    fn test_generate_session_hash_from_metadata() {
        let request_body = json!({
            "metadata": {
                "user_id": "session_550e8400-e29b-41d4-a716-446655440000"
            },
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let hash = generate_session_hash(&request_body);
        assert_eq!(
            hash,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_generate_session_hash_from_system() {
        let request_body = json!({
            "system": "You are a helpful assistant",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let hash = generate_session_hash(&request_body);
        assert!(hash.is_some());
        assert_eq!(hash.as_ref().unwrap().len(), 32);
    }

    #[test]
    fn test_generate_session_hash_from_first_message() {
        let request_body = json!({
            "messages": [{"role": "user", "content": "Hello World"}]
        });

        let hash = generate_session_hash(&request_body);
        assert!(hash.is_some());
        assert_eq!(hash.as_ref().unwrap().len(), 32);
    }

    #[test]
    fn test_generate_session_hash_empty() {
        let request_body = json!({
            "messages": []
        });

        let hash = generate_session_hash(&request_body);
        assert_eq!(hash, None);
    }

    #[test]
    fn test_is_valid_session_hash() {
        assert!(is_valid_session_hash("abcdef0123456789abcdef0123456789"));
        assert!(!is_valid_session_hash("not-a-hash"));
        assert!(!is_valid_session_hash("abcdef")); // 太短
    }

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash("test content");
        assert_eq!(hash.len(), 32);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
