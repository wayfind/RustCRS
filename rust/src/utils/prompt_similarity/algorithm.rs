/// String similarity algorithm implementation using Dice Coefficient
///
/// The Dice Coefficient (also known as Sørensen-Dice coefficient) measures
/// the similarity between two strings based on their bigram (2-character) overlaps.
///
/// Formula: Dice = 2 * |X ∩ Y| / (|X| + |Y|)
/// where X and Y are the sets of bigrams from each string.
///
/// Reference: nodejs-archive/src/utils/contents.js (uses string-similarity npm package)

use std::collections::HashSet;

/// Calculate Dice Coefficient between two strings
///
/// Returns a value between 0.0 (completely different) and 1.0 (identical)
///
/// # Examples
///
/// ```
/// use crate::utils::prompt_similarity::algorithm::dice_coefficient;
///
/// let score = dice_coefficient("hello world", "hello world");
/// assert_eq!(score, 1.0);
///
/// let score = dice_coefficient("hello", "world");
/// assert!(score < 0.5);
/// ```
pub fn dice_coefficient(s1: &str, s2: &str) -> f64 {
    // Handle empty strings
    if s1.is_empty() || s2.is_empty() {
        return if s1.is_empty() && s2.is_empty() {
            1.0 // Both empty = identical
        } else {
            0.0 // One empty = completely different
        };
    }

    // Early exit for identical strings
    if s1 == s2 {
        return 1.0;
    }

    // Extract bigrams from both strings
    let bigrams1 = extract_bigrams(s1);
    let bigrams2 = extract_bigrams(s2);

    // Handle edge case: strings too short for bigrams
    if bigrams1.is_empty() || bigrams2.is_empty() {
        return if bigrams1.is_empty() && bigrams2.is_empty() {
            1.0 // Both have no bigrams (single char?) = identical
        } else {
            0.0
        };
    }

    // Calculate intersection and total bigrams
    let intersection = bigrams1.intersection(&bigrams2).count();
    let total_bigrams = bigrams1.len() + bigrams2.len();

    // Dice Coefficient formula
    2.0 * intersection as f64 / total_bigrams as f64
}

/// Extract bigrams (2-character sequences) from a string
///
/// # Examples
///
/// ```
/// let bigrams = extract_bigrams("hello");
/// // Returns: {"he", "el", "ll", "lo"}
/// ```
fn extract_bigrams(s: &str) -> HashSet<String> {
    let chars: Vec<char> = s.chars().collect();

    if chars.len() < 2 {
        return HashSet::new();
    }

    chars
        .windows(2)
        .map(|window| format!("{}{}", window[0], window[1]))
        .collect()
}

/// Calculate similarity with a simple comparison
///
/// Returns a result struct with score, threshold, and pass/fail status
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    pub score: f64,
    pub threshold: f64,
    pub passed: bool,
}

impl SimilarityResult {
    pub fn new(score: f64, threshold: f64) -> Self {
        Self {
            score,
            threshold,
            passed: score >= threshold,
        }
    }
}

/// Calculate similarity and return detailed result
pub fn calculate_similarity(actual: &str, expected: &str, threshold: f64) -> SimilarityResult {
    let score = dice_coefficient(actual, expected);
    SimilarityResult::new(score, threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dice_coefficient_identical_strings() {
        let score = dice_coefficient("hello world", "hello world");
        assert_eq!(score, 1.0, "Identical strings should have score 1.0");
    }

    #[test]
    fn test_dice_coefficient_completely_different() {
        let score = dice_coefficient("abc", "xyz");
        assert_eq!(score, 0.0, "Completely different strings should have score 0.0");
    }

    #[test]
    fn test_dice_coefficient_partial_match() {
        let score = dice_coefficient("hello world", "hello rust");
        assert!(
            score > 0.0 && score < 1.0,
            "Partially similar strings should have 0 < score < 1, got {}",
            score
        );
        // "hello world" vs "hello rust"
        // Shared bigrams: "he", "el", "ll", "lo", "o "
        // Total bigrams: 10 (hello world) + 9 (hello rust) = 19
        // Intersection: 5
        // Dice = 2*5/19 ≈ 0.526
        assert!(
            (score - 0.526).abs() < 0.01,
            "Expected score around 0.526, got {}",
            score
        );
    }

    #[test]
    fn test_dice_coefficient_empty_strings() {
        assert_eq!(
            dice_coefficient("", ""),
            1.0,
            "Two empty strings should be identical"
        );
        assert_eq!(
            dice_coefficient("hello", ""),
            0.0,
            "Empty vs non-empty should be 0.0"
        );
        assert_eq!(
            dice_coefficient("", "world"),
            0.0,
            "Empty vs non-empty should be 0.0"
        );
    }

    #[test]
    fn test_dice_coefficient_single_character() {
        assert_eq!(
            dice_coefficient("a", "a"),
            1.0,
            "Single identical characters should be 1.0"
        );
        assert_eq!(
            dice_coefficient("a", "b"),
            1.0,
            "Single different characters (no bigrams) should be 1.0"
        );
    }

    #[test]
    fn test_dice_coefficient_case_sensitive() {
        let score = dice_coefficient("Hello", "hello");
        assert!(
            score < 1.0,
            "Case differences should affect score, got {}",
            score
        );
    }

    #[test]
    fn test_extract_bigrams_normal() {
        let bigrams = extract_bigrams("hello");
        assert_eq!(bigrams.len(), 4);
        assert!(bigrams.contains("he"));
        assert!(bigrams.contains("el"));
        assert!(bigrams.contains("ll"));
        assert!(bigrams.contains("lo"));
    }

    #[test]
    fn test_extract_bigrams_short_string() {
        let bigrams = extract_bigrams("a");
        assert_eq!(bigrams.len(), 0, "Single character has no bigrams");
    }

    #[test]
    fn test_extract_bigrams_two_chars() {
        let bigrams = extract_bigrams("ab");
        assert_eq!(bigrams.len(), 1);
        assert!(bigrams.contains("ab"));
    }

    #[test]
    fn test_similarity_result_passed() {
        let result = SimilarityResult::new(0.6, 0.5);
        assert!(result.passed, "Score 0.6 should pass threshold 0.5");
        assert_eq!(result.score, 0.6);
        assert_eq!(result.threshold, 0.5);
    }

    #[test]
    fn test_similarity_result_failed() {
        let result = SimilarityResult::new(0.4, 0.5);
        assert!(!result.passed, "Score 0.4 should not pass threshold 0.5");
    }

    #[test]
    fn test_calculate_similarity() {
        let result = calculate_similarity("hello world", "hello world", 0.5);
        assert!(result.passed);
        assert_eq!(result.score, 1.0);

        let result = calculate_similarity("hello", "world", 0.5);
        assert!(!result.passed);
        assert!(result.score < 0.5);
    }

    #[test]
    fn test_claude_code_prompt_similarity() {
        // Test with actual Claude Code prompt fragments
        let template = "You are Claude Code, Anthropic's official CLI for Claude.";
        let similar = "You are Claude Code, Anthropic's official CLI for Claude.";
        let different = "You are a helpful assistant.";

        let result1 = calculate_similarity(similar, template, 0.5);
        assert!(
            result1.passed,
            "Identical Claude Code prompts should pass"
        );

        let result2 = calculate_similarity(different, template, 0.5);
        assert!(
            !result2.passed,
            "Different prompts should not pass, score: {}",
            result2.score
        );
    }

    #[test]
    fn test_real_world_scenario() {
        // Based on Node.js code testing scenario
        let claude_code_prompt = "You are an interactive CLI tool that helps users with software engineering tasks.";
        let custom_prompt = "You are a helpful AI assistant.";

        let score = dice_coefficient(claude_code_prompt, custom_prompt);
        assert!(
            score < 0.5,
            "Custom prompt should have low similarity to Claude Code prompt, got {}",
            score
        );
    }
}
