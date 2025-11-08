/// Text normalization utilities for prompt comparison
///
/// Provides functions to standardize text format for accurate similarity matching.
/// Handles whitespace collapsing, placeholder removal, and trimming.
///
/// Reference: nodejs-archive/src/utils/contents.js

/// Placeholder token used in Claude Code prompt templates
pub const PLACEHOLDER_TOKEN: &str = "__PLACEHOLDER__";

/// Normalize text for comparison
///
/// Performs the following transformations:
/// 1. Collapses multiple whitespace characters into single spaces
/// 2. Replaces placeholder tokens with spaces
/// 3. Trims leading and trailing whitespace
///
/// # Examples
///
/// ```
/// let normalized = normalize_text("hello  world");
/// assert_eq!(normalized, "hello world");
///
/// let normalized = normalize_text("hello __PLACEHOLDER__ world");
/// assert_eq!(normalized, "hello world");
/// ```
pub fn normalize_text(text: &str) -> String {
    // Replace placeholders with single space
    let without_placeholder = text.replace(PLACEHOLDER_TOKEN, " ");

    // Collapse whitespace and trim
    collapse_whitespace(&without_placeholder)
}

/// Collapse consecutive whitespace characters into single spaces
///
/// Handles spaces, tabs, newlines, and other whitespace characters.
///
/// # Examples
///
/// ```
/// let collapsed = collapse_whitespace("hello  \t\n  world");
/// assert_eq!(collapsed, "hello world");
/// ```
pub fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Remove all whitespace characters from text
///
/// Used for structural matching where whitespace is irrelevant.
pub fn remove_all_whitespace(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_text_basic() {
        assert_eq!(normalize_text("hello world"), "hello world");
    }

    #[test]
    fn test_normalize_text_multiple_spaces() {
        assert_eq!(normalize_text("hello  world"), "hello world");
        assert_eq!(normalize_text("hello   world"), "hello world");
    }

    #[test]
    fn test_normalize_text_tabs_and_newlines() {
        assert_eq!(normalize_text("hello\tworld"), "hello world");
        assert_eq!(normalize_text("hello\nworld"), "hello world");
        assert_eq!(normalize_text("hello\r\nworld"), "hello world");
        assert_eq!(normalize_text("hello  \t\n  world"), "hello world");
    }

    #[test]
    fn test_normalize_text_leading_trailing_whitespace() {
        assert_eq!(normalize_text("  hello world  "), "hello world");
        assert_eq!(normalize_text("\t\nhello world\n\t"), "hello world");
    }

    #[test]
    fn test_normalize_text_placeholder() {
        assert_eq!(
            normalize_text("hello __PLACEHOLDER__ world"),
            "hello world"
        );
    }

    #[test]
    fn test_normalize_text_multiple_placeholders() {
        assert_eq!(
            normalize_text("hello __PLACEHOLDER__ __PLACEHOLDER__ world"),
            "hello world"
        );
    }

    #[test]
    fn test_normalize_text_placeholder_and_whitespace() {
        assert_eq!(
            normalize_text("hello  __PLACEHOLDER__  world"),
            "hello world"
        );
    }

    #[test]
    fn test_normalize_text_empty() {
        assert_eq!(normalize_text(""), "");
        assert_eq!(normalize_text("   "), "");
    }

    #[test]
    fn test_collapse_whitespace() {
        assert_eq!(collapse_whitespace("hello  world"), "hello world");
        assert_eq!(collapse_whitespace("hello\tworld"), "hello world");
        assert_eq!(collapse_whitespace("  hello  world  "), "hello world");
    }

    #[test]
    fn test_remove_all_whitespace() {
        assert_eq!(remove_all_whitespace("hello world"), "helloworld");
        assert_eq!(remove_all_whitespace("hello  world"), "helloworld");
        assert_eq!(remove_all_whitespace("hello\t\nworld"), "helloworld");
    }

    #[test]
    fn test_real_world_prompt_normalization() {
        let prompt = "You are an interactive CLI tool that helps users\n\n\
                      with software engineering tasks.";

        let normalized = normalize_text(prompt);
        assert_eq!(
            normalized,
            "You are an interactive CLI tool that helps users with software engineering tasks."
        );
    }

    #[test]
    fn test_claude_code_prompt_with_placeholder() {
        let template = "You are Claude Code __PLACEHOLDER__ Use the instructions below.";
        let normalized = normalize_text(template);
        assert_eq!(normalized, "You are Claude Code Use the instructions below.");
    }
}
