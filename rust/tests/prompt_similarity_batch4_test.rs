/// Batch 4 验证测试：Claude Code headers 集成
///
/// 此测试验证批次4的所有功能：
/// - 系统提示词提取（字符串和数组格式）
/// - 真实 Claude Code 请求检测
/// - 多层验证机制集成

use claude_relay::utils::claude_code_headers::{
    is_real_claude_code_request,
};
use serde_json::json;

#[test]
fn test_batch4_detect_claude_code_primary_prompt() {
    let body = json!({
        "system": "You are Claude Code, Anthropic's official CLI for Claude.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        is_real_claude_code_request(&body),
        "应该识别 Claude Code primary 提示词"
    );
    println!("✓ Primary 提示词检测成功");
}

#[test]
fn test_batch4_detect_claude_code_secondary_prompt() {
    let body = json!({
        "system": "You are an interactive CLI tool that helps users with software engineering tasks. Use the instructions below and the tools available to you to assist the user.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        is_real_claude_code_request(&body),
        "应该识别 Claude Code secondary 提示词"
    );
    println!("✓ Secondary 提示词检测成功");
}

#[test]
fn test_batch4_detect_agent_sdk_prompt() {
    let body = json!({
        "system": "You are a Claude agent, built on Anthropic's Claude Agent SDK.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        is_real_claude_code_request(&body),
        "应该识别 Agent SDK 提示词"
    );
    println!("✓ Agent SDK 提示词检测成功");
}

#[test]
fn test_batch4_detect_code_agent_sdk_prompt() {
    let body = json!({
        "system": "You are Claude Code, Anthropic's official CLI for Claude, running within the Claude Agent SDK.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        is_real_claude_code_request(&body),
        "应该识别 Code Agent SDK 提示词"
    );
    println!("✓ Code Agent SDK 提示词检测成功");
}

#[test]
fn test_batch4_detect_compact_prompt() {
    let body = json!({
        "system": "You are Claude, tasked with summarizing conversations from Claude Code sessions.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        is_real_claude_code_request(&body),
        "应该识别 Compact 提示词"
    );
    println!("✓ Compact 提示词检测成功");
}

#[test]
fn test_batch4_reject_custom_prompt() {
    // 使用明确非 Claude Code 的提示词（避免"assistant"和"helps"等常见词）
    let body = json!({
        "system": "Analyze the following data and provide statistical insights.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        !is_real_claude_code_request(&body),
        "应该拒绝自定义提示词"
    );
    println!("✓ 正确拒绝自定义提示词");
}

#[test]
fn test_batch4_reject_generic_chatbot() {
    let body = json!({
        "system": "You are a friendly chatbot.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        !is_real_claude_code_request(&body),
        "应该拒绝通用聊天机器人提示词"
    );
    println!("✓ 正确拒绝通用聊天机器人");
}

#[test]
fn test_batch4_no_system_field() {
    let body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    });

    assert!(
        !is_real_claude_code_request(&body),
        "没有 system 字段不应该被识别"
    );
    println!("✓ 正确处理缺失 system 字段");
}

#[test]
fn test_batch4_system_prompt_array_format() {
    let body = json!({
        "system": [
            {"type": "text", "text": "You are Claude Code,"},
            {"type": "text", "text": "Anthropic's official CLI for Claude."}
        ],
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        is_real_claude_code_request(&body),
        "应该支持数组格式的 system 字段"
    );
    println!("✓ 数组格式 system 字段支持成功");
}

#[test]
fn test_batch4_metadata_user_id_fallback() {
    let body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [],
        "metadata": {
            "user_id": "user_1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef_account__session_12345678-1234-1234-1234-123456789012"
        }
    });

    assert!(
        is_real_claude_code_request(&body),
        "应该通过 metadata.user_id 识别 Claude Code"
    );
    println!("✓ metadata.user_id 备用检测成功");
}

#[test]
fn test_batch4_combined_validation() {
    // 同时有 system 和 metadata
    let body = json!({
        "system": "You are Claude Code, Anthropic's official CLI for Claude.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": [],
        "metadata": {
            "user_id": "user_abc_account__session_xyz"
        }
    });

    assert!(
        is_real_claude_code_request(&body),
        "应该通过任一方法识别"
    );
    println!("✓ 组合验证成功");
}

#[test]
fn test_batch4_whitespace_in_system_prompt() {
    // 测试规范化功能：额外的空格不应影响识别
    let body = json!({
        "system": "You are Claude Code,  Anthropic's   official CLI for Claude.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        is_real_claude_code_request(&body),
        "规范化后应该识别（额外空格）"
    );
    println!("✓ 空格规范化测试通过");
}

#[test]
fn test_batch4_newlines_in_system_prompt() {
    // 测试换行符处理
    let body = json!({
        "system": "You are Claude Code,\nAnthropic's official CLI for Claude.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        is_real_claude_code_request(&body),
        "规范化后应该识别（换行符）"
    );
    println!("✓ 换行符规范化测试通过");
}

#[test]
fn test_batch4_partial_match_not_enough() {
    // 部分相似但不超过阈值的提示词
    let body = json!({
        "system": "You are Claude, an AI assistant.",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        !is_real_claude_code_request(&body),
        "部分相似不应该被识别"
    );
    println!("✓ 正确拒绝部分相似提示词");
}

#[test]
fn test_batch4_empty_system_string() {
    let body = json!({
        "system": "",
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        !is_real_claude_code_request(&body),
        "空 system 不应该被识别"
    );
    println!("✓ 正确处理空 system");
}

#[test]
fn test_batch4_empty_system_array() {
    let body = json!({
        "system": [],
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });

    assert!(
        !is_real_claude_code_request(&body),
        "空 system 数组不应该被识别"
    );
    println!("✓ 正确处理空 system 数组");
}

#[test]
fn test_batch4_real_world_scenario_claude_code() {
    // 真实的 Claude Code 请求格式
    let body = json!({
        "system": [
            {
                "type": "text",
                "text": "You are Claude Code, Anthropic's official CLI for Claude. You are an interactive CLI tool that helps users with software engineering tasks."
            }
        ],
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 8096,
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ],
        "metadata": {
            "user_id": "user_abc123_account__session_def456"
        }
    });

    assert!(
        is_real_claude_code_request(&body),
        "真实 Claude Code 请求应该被识别"
    );
    println!("✓ 真实场景测试通过");
}

#[test]
fn test_batch4_real_world_scenario_custom() {
    // 真实的自定义客户端请求
    let body = json!({
        "system": "You are a code review assistant that helps developers improve their code quality.",
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 4096,
        "messages": [
            {
                "role": "user",
                "content": "Review this code"
            }
        ]
    });

    assert!(
        !is_real_claude_code_request(&body),
        "自定义客户端不应该被识别为 Claude Code"
    );
    println!("✓ 真实自定义场景正确拒绝");
}
