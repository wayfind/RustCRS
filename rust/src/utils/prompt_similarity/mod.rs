/// System prompt similarity validation module
///
/// Implements Claude Code system prompt detection using Dice Coefficient
/// string similarity algorithm and template matching.
///
/// # Architecture
///
/// - `algorithm`: Dice Coefficient implementation for string similarity
/// - `normalizer`: Text normalization utilities
///
/// # Usage
///
/// ```rust
/// use crate::utils::prompt_similarity::{dice_coefficient, normalize_text};
///
/// let score = dice_coefficient("hello world", "hello rust");
/// let normalized = normalize_text("hello  world");
/// ```
///
/// Reference: nodejs-archive/src/utils/contents.js

pub mod algorithm;
pub mod normalizer;
pub mod templates;

// Re-export commonly used items
pub use algorithm::{calculate_similarity, dice_coefficient, SimilarityResult};
pub use normalizer::{collapse_whitespace, normalize_text, PLACEHOLDER_TOKEN};
pub use templates::{
    get_all_templates, get_template_by_id, get_templates_by_category, PromptCategory,
    PromptTemplate, CLAUDE_CODE_TEMPLATES,
};

/// Default similarity threshold for Claude Code prompt detection
///
/// Set to 0.5 (50% similarity) to match Node.js implementation
pub const DEFAULT_THRESHOLD: f64 = 0.5;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_workflow() {
        // Simulate the complete workflow: normalize then compare
        let template = "You are Claude Code, Anthropic's official CLI for Claude.";
        let input = "You are Claude Code,  Anthropic's   official CLI for Claude.";

        let normalized_template = normalize_text(template);
        let normalized_input = normalize_text(input);

        let score = dice_coefficient(&normalized_input, &normalized_template);

        assert_eq!(score, 1.0, "Normalized versions should be identical");
    }

    #[test]
    fn test_workflow_with_placeholder() {
        let template = "You are Claude Code __PLACEHOLDER__ Use the tools available.";
        let input = "You are Claude Code some dynamic content Use the tools available.";

        let normalized_template = normalize_text(template);
        let normalized_input = normalize_text(input);

        // After normalization, placeholder is removed, so they won't match exactly
        // But they should still have high similarity
        let score = dice_coefficient(&normalized_input, &normalized_template);

        assert!(score > 0.5, "Should have reasonable similarity, got {}", score);
    }

    #[test]
    fn test_reject_custom_prompt() {
        let template = "You are Claude Code, Anthropic's official CLI for Claude.";
        let custom = "You are a helpful assistant that answers questions.";

        let score = dice_coefficient(custom, template);

        assert!(score < DEFAULT_THRESHOLD, "Custom prompt should be rejected");
    }
}
