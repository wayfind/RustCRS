/// Batch 2 验证测试：Claude Code 提示词模板管理
///
/// 此测试验证批次2的所有功能：
/// - 模板定义和结构
/// - 模板查询函数
/// - 模板规范化
/// - 模板完整性

use claude_relay::utils::prompt_similarity::{
    get_all_templates, get_template_by_id, get_templates_by_category,
    PromptCategory, CLAUDE_CODE_TEMPLATES,
};

#[test]
fn test_batch2_all_templates_loaded() {
    let templates = get_all_templates();
    assert_eq!(templates.len(), 5, "应该有5个Claude Code模板");

    // 验证所有模板都有有效的ID和内容
    for template in templates {
        assert!(!template.id.is_empty(), "模板ID不应为空");
        assert!(!template.text.is_empty(), "模板文本不应为空");
        assert!(!template.title.is_empty(), "模板标题不应为空");
        println!("✓ 模板 '{}' 加载成功: {}", template.id, template.title);
    }
}

#[test]
fn test_batch2_template_lookup_by_id() {
    // 测试查找主模板
    let primary = get_template_by_id("claude_code_primary");
    assert!(primary.is_some(), "应该找到主模板");
    let primary = primary.unwrap();
    assert_eq!(primary.id, "claude_code_primary");
    assert!(primary.text.contains("Claude Code"), "主模板应包含'Claude Code'");
    println!("✓ 主模板查找成功: {}", primary.text);

    // 测试查找不存在的模板
    let not_found = get_template_by_id("nonexistent");
    assert!(not_found.is_none(), "不存在的模板应返回None");
    println!("✓ 不存在的模板正确返回None");
}

#[test]
fn test_batch2_template_categories() {
    let system_templates = get_templates_by_category(PromptCategory::System);
    assert_eq!(system_templates.len(), 5, "所有5个模板都应该是System类别");

    for template in system_templates {
        assert_eq!(template.category, PromptCategory::System);
        println!("✓ 模板 '{}' 属于System类别", template.id);
    }
}

#[test]
fn test_batch2_template_ids_unique() {
    use std::collections::HashSet;

    let templates = get_all_templates();
    let mut ids = HashSet::new();

    for template in templates {
        assert!(
            ids.insert(template.id),
            "发现重复的模板ID: {}",
            template.id
        );
    }

    println!("✓ 所有模板ID唯一");
}

#[test]
fn test_batch2_specific_templates() {
    // 验证每个特定模板的存在和内容

    // 1. Primary模板
    let primary = get_template_by_id("claude_code_primary").unwrap();
    assert_eq!(
        primary.text,
        "You are Claude Code, Anthropic's official CLI for Claude."
    );
    println!("✓ Primary模板验证通过");

    // 2. Secondary模板
    let secondary = get_template_by_id("claude_code_secondary").unwrap();
    assert!(secondary.text.contains("interactive CLI tool"));
    assert!(secondary.text.contains("__PLACEHOLDER__"));
    println!("✓ Secondary模板验证通过");

    // 3. Agent SDK模板
    let sdk = get_template_by_id("claude_agent_sdk").unwrap();
    assert!(sdk.text.contains("Agent SDK"));
    println!("✓ Agent SDK模板验证通过");

    // 4. Code Agent SDK模板
    let code_sdk = get_template_by_id("claude_code_agent_sdk").unwrap();
    assert!(code_sdk.text.contains("Claude Code"));
    assert!(code_sdk.text.contains("Agent SDK"));
    println!("✓ Code Agent SDK模板验证通过");

    // 5. Compact模板
    let compact = get_template_by_id("claude_code_compact").unwrap();
    assert!(compact.text.contains("Claude"));
    assert!(compact.text.contains("summarizing conversations"));
    assert!(compact.text.contains("Claude Code sessions"));
    println!("✓ Compact模板验证通过");
}

#[test]
fn test_batch2_template_array_consistency() {
    // 验证CLAUDE_CODE_TEMPLATES常量与get_all_templates()一致
    assert_eq!(
        CLAUDE_CODE_TEMPLATES.len(),
        get_all_templates().len(),
        "模板数组应与get_all_templates()返回相同数量"
    );

    for (i, template) in CLAUDE_CODE_TEMPLATES.iter().enumerate() {
        let from_getter = &get_all_templates()[i];
        assert_eq!(template.id, from_getter.id, "模板ID应一致");
        assert_eq!(template.text, from_getter.text, "模板文本应一致");
    }

    println!("✓ 模板数组与getter函数一致");
}

#[test]
fn test_batch2_template_normalization_map() {
    use claude_relay::utils::prompt_similarity::templates::create_normalized_template_map;

    let map = create_normalized_template_map();

    // 验证所有模板都被规范化
    assert_eq!(map.len(), 5, "应该有5个规范化的模板");

    // 验证规范化后的文本不包含占位符
    for (id, normalized_text) in map.iter() {
        assert!(!normalized_text.contains("__PLACEHOLDER__"),
                "规范化文本不应包含占位符: {}", id);
        assert!(!normalized_text.is_empty(), "规范化文本不应为空: {}", id);
        println!("✓ 模板 '{}' 规范化成功: {} 字符", id, normalized_text.len());
    }

    // 验证primary模板规范化后保持不变（因为它没有额外空格或占位符）
    let primary_normalized = map.get("claude_code_primary").unwrap();
    assert_eq!(
        primary_normalized,
        "You are Claude Code, Anthropic's official CLI for Claude.",
        "Primary模板规范化后应保持原样"
    );
}
