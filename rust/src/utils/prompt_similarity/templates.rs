/// Claude Code system prompt templates
///
/// This module defines the standard system prompts used by Claude Code CLI.
/// These templates are used for similarity matching to detect real Claude Code requests.
///
/// Templates are extracted from actual Claude Code system prompts with dynamic
/// content replaced by __PLACEHOLDER__ markers.
///
/// Reference: nodejs-archive/src/utils/contents.js (PROMPT_DEFINITIONS)

use std::collections::HashMap;

/// Prompt template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptCategory {
    System,
    OutputStyle,
    Tools,
    Web,
    Agents,
    Summaries,
    Notes,
    Quality,
}

/// Prompt template definition
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub id: &'static str,
    pub title: &'static str,
    pub category: PromptCategory,
    pub text: &'static str,
}

impl PromptTemplate {
    pub const fn new(
        id: &'static str,
        title: &'static str,
        category: PromptCategory,
        text: &'static str,
    ) -> Self {
        Self {
            id,
            title,
            category,
            text,
        }
    }
}

/// Claude Code primary system prompt
const CLAUDE_CODE_PRIMARY: PromptTemplate = PromptTemplate::new(
    "claude_code_primary",
    "Claude Code System Prompt (Primary)",
    PromptCategory::System,
    "You are Claude Code, Anthropic's official CLI for Claude.",
);

/// Claude Code secondary (full) system prompt
///
/// This is a condensed version. The full prompt is very long (>10KB).
/// We include key distinctive phrases that can identify Claude Code.
const CLAUDE_CODE_SECONDARY: PromptTemplate = PromptTemplate::new(
    "claude_code_secondary",
    "Claude Code System Prompt (Secondary)",
    PromptCategory::System,
    "You are an interactive CLI tool that helps users __PLACEHOLDER__ Use the instructions below and the tools available to you to assist the user.",
);

/// Claude Agent SDK system prompt
const CLAUDE_AGENT_SDK: PromptTemplate = PromptTemplate::new(
    "claude_agent_sdk",
    "Claude Agent SDK System Prompt",
    PromptCategory::System,
    "You are a Claude agent, built on Anthropic's Claude Agent SDK.",
);

/// Claude Code running within Agent SDK
const CLAUDE_CODE_AGENT_SDK: PromptTemplate = PromptTemplate::new(
    "claude_code_agent_sdk",
    "Claude Code Compact System Prompt Agent SDK",
    PromptCategory::System,
    "You are Claude Code, Anthropic's official CLI for Claude, running within the Claude Agent SDK.",
);

/// Claude Code compact system prompt
///
/// Note: This template is more specific to avoid false positives with generic assistant prompts
const CLAUDE_CODE_COMPACT: PromptTemplate = PromptTemplate::new(
    "claude_code_compact",
    "Claude Code Compact System Prompt",
    PromptCategory::System,
    "You are Claude, tasked with summarizing conversations from Claude Code sessions.",
);

/// All Claude Code system prompt templates
///
/// This is the definitive list of templates used for similarity matching.
pub const CLAUDE_CODE_TEMPLATES: &[PromptTemplate] = &[
    CLAUDE_CODE_PRIMARY,
    CLAUDE_CODE_SECONDARY,
    CLAUDE_AGENT_SDK,
    CLAUDE_CODE_AGENT_SDK,
    CLAUDE_CODE_COMPACT,
];

/// Get all templates as a slice
pub fn get_all_templates() -> &'static [PromptTemplate] {
    CLAUDE_CODE_TEMPLATES
}

/// Get templates by category
pub fn get_templates_by_category(category: PromptCategory) -> Vec<&'static PromptTemplate> {
    CLAUDE_CODE_TEMPLATES
        .iter()
        .filter(|t| t.category == category)
        .collect()
}

/// Get template by ID
pub fn get_template_by_id(id: &str) -> Option<&'static PromptTemplate> {
    CLAUDE_CODE_TEMPLATES.iter().find(|t| t.id == id)
}

/// Get template text by ID
pub fn get_template_text(id: &str) -> Option<&'static str> {
    get_template_by_id(id).map(|t| t.text)
}

/// Create a map of template ID to normalized text (for performance)
///
/// This can be used to cache normalized templates to avoid repeated normalization.
pub fn create_normalized_template_map() -> HashMap<&'static str, String> {
    use super::normalizer::normalize_text;

    CLAUDE_CODE_TEMPLATES
        .iter()
        .map(|template| (template.id, normalize_text(template.text)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::prompt_similarity::normalizer::PLACEHOLDER_TOKEN;

    #[test]
    fn test_templates_defined() {
        assert_eq!(
            CLAUDE_CODE_TEMPLATES.len(),
            5,
            "Should have 5 Claude Code templates"
        );
    }

    #[test]
    fn test_get_all_templates() {
        let templates = get_all_templates();
        assert_eq!(templates.len(), 5);
    }

    #[test]
    fn test_get_templates_by_category() {
        let system_templates = get_templates_by_category(PromptCategory::System);
        assert_eq!(
            system_templates.len(),
            5,
            "All templates should be System category"
        );
    }

    #[test]
    fn test_get_template_by_id() {
        let template = get_template_by_id("claude_code_primary");
        assert!(template.is_some());
        assert_eq!(template.unwrap().id, "claude_code_primary");

        let not_found = get_template_by_id("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_template_text() {
        let text = get_template_text("claude_code_primary");
        assert!(text.is_some());
        assert!(text.unwrap().contains("Claude Code"));

        let not_found = get_template_text("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_template_ids_unique() {
        let mut ids = std::collections::HashSet::new();
        for template in CLAUDE_CODE_TEMPLATES {
            assert!(
                ids.insert(template.id),
                "Duplicate template ID: {}",
                template.id
            );
        }
    }

    #[test]
    fn test_template_texts_not_empty() {
        for template in CLAUDE_CODE_TEMPLATES {
            assert!(
                !template.text.is_empty(),
                "Template {} has empty text",
                template.id
            );
        }
    }

    #[test]
    fn test_primary_template_content() {
        let primary = get_template_by_id("claude_code_primary").unwrap();
        assert_eq!(
            primary.text,
            "You are Claude Code, Anthropic's official CLI for Claude."
        );
        assert_eq!(primary.category, PromptCategory::System);
    }

    #[test]
    fn test_secondary_template_has_placeholder() {
        let secondary = get_template_by_id("claude_code_secondary").unwrap();
        assert!(
            secondary.text.contains(PLACEHOLDER_TOKEN),
            "Secondary template should contain placeholder"
        );
    }

    #[test]
    fn test_create_normalized_template_map() {
        let map = create_normalized_template_map();
        assert_eq!(map.len(), 5, "Should have 5 normalized templates");

        // Check that normalization actually happened
        let primary_normalized = map.get("claude_code_primary").unwrap();
        assert_eq!(
            primary_normalized,
            "You are Claude Code, Anthropic's official CLI for Claude."
        );

        // Check placeholder is removed in normalized version
        let secondary_normalized = map.get("claude_code_secondary").unwrap();
        assert!(
            !secondary_normalized.contains(PLACEHOLDER_TOKEN),
            "Normalized text should not contain placeholder"
        );
    }

    #[test]
    fn test_agent_sdk_templates() {
        let sdk = get_template_by_id("claude_agent_sdk").unwrap();
        assert!(sdk.text.contains("Agent SDK"));

        let code_sdk = get_template_by_id("claude_code_agent_sdk").unwrap();
        assert!(code_sdk.text.contains("Claude Code"));
        assert!(code_sdk.text.contains("Agent SDK"));
    }

    #[test]
    fn test_compact_template() {
        let compact = get_template_by_id("claude_code_compact").unwrap();
        assert!(compact.text.contains("Claude"));
        assert!(compact.text.contains("summarizing conversations"));
        assert!(compact.text.contains("Claude Code sessions"));
    }

    #[test]
    fn test_template_titles_descriptive() {
        for template in CLAUDE_CODE_TEMPLATES {
            assert!(
                !template.title.is_empty(),
                "Template {} has empty title",
                template.id
            );
            assert!(
                template.title.len() > 10,
                "Template {} title too short: {}",
                template.id,
                template.title
            );
        }
    }
}
