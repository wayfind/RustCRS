/// Template matching and validation logic
///
/// This module provides the main API for validating system prompts
/// against Claude Code templates using similarity matching.
///
/// # Workflow
///
/// 1. Normalize input prompt (remove extra whitespace, placeholders)
/// 2. Normalize all template prompts (cached for performance)
/// 3. Calculate Dice Coefficient similarity scores
/// 4. Return highest matching template if above threshold
///
/// Reference: nodejs-archive/src/validators/clients/claudeCodeValidator.js

use super::algorithm::dice_coefficient;
use super::normalizer::normalize_text;
use super::templates::{get_all_templates, PromptTemplate};
use super::DEFAULT_THRESHOLD;

/// Result of template matching operation
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Whether the input matched any template above threshold
    pub matched: bool,
    /// The best matching template (if any)
    pub best_match: Option<BestMatch>,
    /// All similarity scores (for debugging)
    pub all_scores: Vec<TemplateScore>,
}

impl MatchResult {
    /// Create a successful match result
    pub fn matched(best_match: BestMatch, all_scores: Vec<TemplateScore>) -> Self {
        Self {
            matched: true,
            best_match: Some(best_match),
            all_scores,
        }
    }

    /// Create a failed match result (no template matched)
    pub fn no_match(all_scores: Vec<TemplateScore>) -> Self {
        Self {
            matched: false,
            best_match: None,
            all_scores,
        }
    }
}

/// Best matching template information
#[derive(Debug, Clone)]
pub struct BestMatch {
    /// The template that matched
    pub template_id: String,
    /// The template title
    pub template_title: String,
    /// Similarity score (0.0 - 1.0)
    pub score: f64,
    /// Whether score exceeded threshold
    pub passed: bool,
}

/// Individual template similarity score
#[derive(Debug, Clone)]
pub struct TemplateScore {
    pub template_id: String,
    pub score: f64,
}

impl TemplateScore {
    pub fn new(template_id: impl Into<String>, score: f64) -> Self {
        Self {
            template_id: template_id.into(),
            score,
        }
    }
}

/// Check if a system prompt matches any Claude Code template
///
/// # Arguments
///
/// * `system_prompt` - The system prompt to validate
/// * `threshold` - Minimum similarity score to consider a match (0.0 - 1.0)
///
/// # Returns
///
/// `MatchResult` containing match status and detailed scores
///
/// # Examples
///
/// ```
/// use crate::utils::prompt_similarity::matcher::check_prompt_similarity;
///
/// let result = check_prompt_similarity(
///     "You are Claude Code, Anthropic's official CLI for Claude.",
///     0.5
/// );
/// assert!(result.matched);
/// ```
pub fn check_prompt_similarity(system_prompt: &str, threshold: f64) -> MatchResult {
    // Normalize input prompt
    let normalized_input = normalize_text(system_prompt);

    // Get all templates and calculate scores
    let templates = get_all_templates();
    let mut scores = Vec::new();
    let mut best_score = 0.0;
    let mut best_template: Option<&PromptTemplate> = None;

    for template in templates {
        // Normalize template text
        let normalized_template = normalize_text(template.text);

        // Calculate similarity
        let score = dice_coefficient(&normalized_input, &normalized_template);

        // Track scores
        scores.push(TemplateScore::new(template.id, score));

        // Track best match
        if score > best_score {
            best_score = score;
            best_template = Some(template);
        }
    }

    // Check if best match exceeds threshold
    if best_score >= threshold {
        if let Some(template) = best_template {
            let best_match = BestMatch {
                template_id: template.id.to_string(),
                template_title: template.title.to_string(),
                score: best_score,
                passed: true,
            };
            return MatchResult::matched(best_match, scores);
        }
    }

    // No match found
    MatchResult::no_match(scores)
}

/// Check if a system prompt matches any Claude Code template using default threshold
///
/// This is a convenience function that uses the default threshold (0.5).
///
/// # Examples
///
/// ```
/// use crate::utils::prompt_similarity::matcher::is_claude_code_prompt;
///
/// let is_valid = is_claude_code_prompt(
///     "You are Claude Code, Anthropic's official CLI for Claude."
/// );
/// assert!(is_valid);
/// ```
pub fn is_claude_code_prompt(system_prompt: &str) -> bool {
    check_prompt_similarity(system_prompt, DEFAULT_THRESHOLD).matched
}

/// Get the best matching template for a system prompt
///
/// Returns the template ID and score of the best match, or None if no match found.
///
/// # Examples
///
/// ```
/// use crate::utils::prompt_similarity::matcher::get_best_match;
///
/// let result = get_best_match("You are Claude Code...", 0.5);
/// if let Some((template_id, score)) = result {
///     println!("Best match: {} with score {}", template_id, score);
/// }
/// ```
pub fn get_best_match(system_prompt: &str, threshold: f64) -> Option<(String, f64)> {
    let result = check_prompt_similarity(system_prompt, threshold);
    result
        .best_match
        .map(|m| (m.template_id, m.score))
}

/// Calculate similarity scores against all templates
///
/// This is useful for debugging and understanding which templates
/// are close to matching.
///
/// # Examples
///
/// ```
/// use crate::utils::prompt_similarity::matcher::get_all_scores;
///
/// let scores = get_all_scores("You are a helpful assistant.");
/// for score in scores {
///     println!("{}: {:.2}%", score.template_id, score.score * 100.0);
/// }
/// ```
pub fn get_all_scores(system_prompt: &str) -> Vec<TemplateScore> {
    let result = check_prompt_similarity(system_prompt, 0.0); // Use 0.0 to get all scores
    result.all_scores
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_primary_template() {
        let prompt = "You are Claude Code, Anthropic's official CLI for Claude.";
        let result = check_prompt_similarity(prompt, 0.5);

        assert!(result.matched, "应该匹配 Claude Code 提示词");
        assert!(result.best_match.is_some());

        let best = result.best_match.unwrap();
        assert_eq!(best.template_id, "claude_code_primary");
        assert_eq!(best.score, 1.0, "精确匹配应该得分1.0");
        assert!(best.passed);
    }

    #[test]
    fn test_match_with_extra_whitespace() {
        let prompt = "You are Claude Code,  Anthropic's   official CLI for Claude.";
        let result = check_prompt_similarity(prompt, 0.5);

        assert!(result.matched, "规范化后应该匹配");
        let best = result.best_match.unwrap();
        assert_eq!(best.score, 1.0, "规范化后应该完全匹配");
    }

    #[test]
    fn test_no_match_custom_prompt() {
        let prompt = "You are a helpful AI assistant that answers questions.";
        let result = check_prompt_similarity(prompt, 0.5);

        assert!(!result.matched, "自定义提示词不应匹配");
        assert!(result.best_match.is_none());
    }

    #[test]
    fn test_partial_match_below_threshold() {
        let prompt = "You are Claude, an AI assistant.";
        let result = check_prompt_similarity(prompt, 0.5);

        // 这个提示词与 Claude Code 有部分相似，但不应超过阈值
        assert!(!result.matched, "部分相似不应超过阈值");

        // 验证确实有相似度分数，只是不够高
        let primary_score = result
            .all_scores
            .iter()
            .find(|s| s.template_id == "claude_code_primary")
            .unwrap();
        assert!(primary_score.score > 0.0, "应该有一些相似度");
        assert!(primary_score.score < 0.5, "但不应超过阈值");
    }

    #[test]
    fn test_is_claude_code_prompt_convenience() {
        assert!(is_claude_code_prompt(
            "You are Claude Code, Anthropic's official CLI for Claude."
        ));
        assert!(!is_claude_code_prompt("You are a helpful assistant."));
    }

    #[test]
    fn test_get_best_match_function() {
        let prompt = "You are Claude Code, Anthropic's official CLI for Claude.";
        let result = get_best_match(prompt, 0.5);

        assert!(result.is_some());
        let (template_id, score) = result.unwrap();
        assert_eq!(template_id, "claude_code_primary");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_get_best_match_no_match() {
        let prompt = "You are a helpful assistant.";
        let result = get_best_match(prompt, 0.5);

        assert!(result.is_none(), "不应找到匹配");
    }

    #[test]
    fn test_get_all_scores() {
        let prompt = "You are Claude Code, Anthropic's official CLI for Claude.";
        let scores = get_all_scores(prompt);

        assert_eq!(scores.len(), 5, "应该有5个模板的分数");

        // 验证主模板得分最高
        let primary_score = scores
            .iter()
            .find(|s| s.template_id == "claude_code_primary")
            .unwrap();
        assert_eq!(primary_score.score, 1.0);
    }

    #[test]
    fn test_all_scores_vector() {
        let prompt = "You are a helpful assistant.";
        let scores = get_all_scores(prompt);

        // 所有分数都应该存在
        assert_eq!(scores.len(), 5);

        // 所有分数都应该低于阈值
        for score in scores {
            assert!(score.score < 0.5, "自定义提示词与所有模板相似度都应低");
        }
    }

    #[test]
    fn test_secondary_template_match() {
        let prompt = "You are an interactive CLI tool that helps users with software engineering tasks. Use the instructions below and the tools available to you to assist the user.";
        let result = check_prompt_similarity(prompt, 0.5);

        assert!(result.matched, "应该匹配 secondary 模板");
        let best = result.best_match.unwrap();
        assert_eq!(best.template_id, "claude_code_secondary");
        assert!(best.score >= 0.5, "分数应该超过阈值");
    }

    #[test]
    fn test_agent_sdk_template_match() {
        let prompt = "You are a Claude agent, built on Anthropic's Claude Agent SDK.";
        let result = check_prompt_similarity(prompt, 0.5);

        assert!(result.matched, "应该匹配 agent_sdk 模板");
        let best = result.best_match.unwrap();
        assert_eq!(best.template_id, "claude_agent_sdk");
        assert_eq!(best.score, 1.0);
    }

    #[test]
    fn test_code_agent_sdk_template_match() {
        let prompt = "You are Claude Code, Anthropic's official CLI for Claude, running within the Claude Agent SDK.";
        let result = check_prompt_similarity(prompt, 0.5);

        assert!(result.matched, "应该匹配 code_agent_sdk 模板");
        let best = result.best_match.unwrap();
        assert_eq!(best.template_id, "claude_code_agent_sdk");
        assert_eq!(best.score, 1.0);
    }

    #[test]
    fn test_compact_template_match() {
        let prompt = "You are a helpful AI assistant tasked with summarizing conversations.";
        let result = check_prompt_similarity(prompt, 0.5);

        assert!(result.matched, "应该匹配 compact 模板");
        let best = result.best_match.unwrap();
        assert_eq!(best.template_id, "claude_code_compact");
        assert_eq!(best.score, 1.0);
    }

    #[test]
    fn test_threshold_boundary() {
        let prompt = "You are Claude Code, Anthropic's official CLI for Claude.";

        // 测试不同的阈值
        let result_low = check_prompt_similarity(prompt, 0.3);
        assert!(result_low.matched);

        let result_medium = check_prompt_similarity(prompt, 0.5);
        assert!(result_medium.matched);

        let result_high = check_prompt_similarity(prompt, 0.9);
        assert!(result_high.matched);

        let result_perfect = check_prompt_similarity(prompt, 1.0);
        assert!(result_perfect.matched);

        let result_impossible = check_prompt_similarity(prompt, 1.1);
        assert!(!result_impossible.matched, "超过1.0的阈值不应匹配");
    }

    #[test]
    fn test_empty_prompt() {
        let result = check_prompt_similarity("", 0.5);
        assert!(!result.matched, "空提示词不应匹配");
    }

    #[test]
    fn test_placeholder_handling_in_matching() {
        // 模板中有占位符，输入中有实际内容
        let prompt = "You are an interactive CLI tool that helps users some dynamic content Use the instructions below and the tools available to you to assist the user.";
        let result = check_prompt_similarity(prompt, 0.5);

        // 规范化后占位符会被移除，所以应该有较高的匹配度
        assert!(result.matched, "占位符处理后应该匹配");
    }
}
