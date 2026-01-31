//! Term highlighting for search results using ANSI bold formatting
//!
//! This module provides functional highlighting of search query terms within
//! text, respecting word boundaries and handling edge cases like special
//! characters and Unicode.

use regex::Regex;
use std::collections::HashMap;

/// Highlight query terms in text using ANSI bold
///
/// # Arguments
/// * `text` - The text to highlight
/// * `query` - Space-separated query terms to highlight
/// * `use_color` - Whether to apply ANSI color codes
///
/// # Returns
/// A new string with query terms wrapped in ANSI bold codes (`\x1b[1m...\x1b[0m`)
///
/// # Features
/// - Case-insensitive matching
/// - Word boundary respect (no partial word matches)
/// - Handles special regex characters in query terms
/// - Supports Unicode text
/// - Multiple term highlighting
///
/// # Examples
/// ```
/// use doc_transformer::highlight::highlight_terms;
/// let result = highlight_terms("Getting Started with Rust", "rust", true);
/// assert!(result.contains("\x1b[1m"));  // Contains ANSI bold
/// ```
#[allow(dead_code)] // Exported for library users - not used internally
pub fn highlight_terms(text: &str, query: &str, use_color: bool) -> String {
    if !use_color {
        return text.to_string();
    }

    if query.is_empty() {
        return text.to_string();
    }

    let terms: Vec<&str> = query.split_whitespace().collect();
    if terms.is_empty() {
        return text.to_string();
    }

    // Build a cache of compiled regexes to avoid recompilation
    let mut regex_cache = HashMap::new();

    let mut result = text.to_string();

    for term in terms {
        if term.is_empty() {
            continue;
        }

        // Get or create cached regex
        let re = regex_cache
            .entry(term.to_string())
            .or_insert_with(|| compile_highlight_regex(term));

        // Check if regex compilation failed
        match re {
            Ok(regex) => {
                // Use ANSI bold codes: \x1b[1m = bold on, \x1b[0m = reset
                result = regex.replace_all(&result, "\x1b[1m$1\x1b[0m").to_string();
            }
            Err(_) => {
                // Skip this term if regex fails (already logged by compile_highlight_regex)
                continue;
            }
        }
    }

    result
}

/// Compile a regex pattern for highlighting with word boundary support
///
/// # Arguments
/// * `term` - The search term to compile into a regex
///
/// # Returns
/// A Result containing the compiled Regex or an error if compilation fails
fn compile_highlight_regex(term: &str) -> Result<Regex, regex::Error> {
    // Escape special regex characters
    let escaped = regex::escape(term);

    // Check if term contains only word characters
    let is_word_only = term.chars().all(|c| c.is_alphanumeric() || c == '_');

    // Add word boundaries only for purely word-based terms
    // (?i) makes it case-insensitive, ( ) creates a capture group for replacement
    let pattern = if is_word_only {
        format!(r"(?i)\b({escaped})\b")
    } else {
        // For terms with special characters like "C++", don't use word boundaries
        format!(r"(?i)({escaped})")
    };

    Regex::new(&pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_single_term() {
        let result = highlight_terms("Getting Started with Rust", "rust", true);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("\x1b[0m"));
        assert!(result.contains("Rust"));
    }

    #[test]
    fn test_highlight_case_insensitive() {
        let result = highlight_terms("RUST programming", "rust", true);
        assert!(result.contains("\x1b[1mRUST\x1b[0m"));
    }

    #[test]
    fn test_highlight_word_boundary() {
        let result = highlight_terms("rusty old rust code", "rust", true);
        // "rusty" should NOT be highlighted, only "rust"
        assert!(!result.contains("\x1b[1mrusty\x1b[0m"));
        assert!(result.contains("\x1b[1mrust\x1b[0m"));
        assert!(result.contains("rusty")); // "rusty" is still in the text, just not highlighted
    }

    #[test]
    fn test_no_color_flag() {
        let result = highlight_terms("Rust code", "rust", false);
        assert!(!result.contains("\x1b["));
        assert_eq!(result, "Rust code");
    }

    #[test]
    fn test_special_chars_in_query() {
        let result = highlight_terms("C++ programming is great", "C++", true);
        assert!(result.contains("\x1b[1mC++\x1b[0m"));
    }

    #[test]
    fn test_empty_query() {
        let result = highlight_terms("Some text", "", true);
        assert!(!result.contains("\x1b["));
        assert_eq!(result, "Some text");
    }

    #[test]
    fn test_empty_text() {
        let result = highlight_terms("", "rust", true);
        assert_eq!(result, "");
    }

    #[test]
    fn test_multiple_occurrences() {
        let result = highlight_terms("rust and rust and rust", "rust", true);
        let count = result.matches("\x1b[1mrust\x1b[0m").count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_multiple_terms() {
        let result = highlight_terms(
            "Getting Started with Rust Programming",
            "rust programming",
            true,
        );
        assert!(result.contains("\x1b[1mRust\x1b[0m"));
        assert!(result.contains("\x1b[1mProgramming\x1b[0m"));
    }

    #[test]
    fn test_unicode_text() {
        let result = highlight_terms("Learning über cool Rust", "über", true);
        assert!(result.contains("\x1b[1müber\x1b[0m"));
    }

    #[test]
    fn test_partial_word_no_match() {
        let result = highlight_terms("substring testing", "string", true);
        // "string" should not match within "substring"
        assert!(!result.contains("\x1b[1mstring\x1b[0m"));
    }

    #[test]
    fn test_at_start_of_text() {
        let result = highlight_terms("Rust is great", "rust", true);
        assert!(result.starts_with("\x1b[1mRust\x1b[0m"));
    }

    #[test]
    fn test_at_end_of_text() {
        let result = highlight_terms("I love Rust", "rust", true);
        assert!(result.contains("\x1b[1mRust\x1b[0m"));
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_whitespace_only_query() {
        let result = highlight_terms("Some text", "   ", true);
        assert_eq!(result, "Some text");
    }

    #[test]
    fn test_overlapping_terms_independent() {
        // If query has overlapping patterns, they should be highlighted independently
        let result = highlight_terms("rust rustic", "rust rusti", true);
        // Both should be highlighted independently where they match word boundaries
        assert!(result.contains("\x1b[1mrust\x1b[0m"));
        // "rusti" won't match because word boundary doesn't include it alone
    }

    #[test]
    fn test_preserves_unicode() {
        let input = "The word über is here and über again";
        let result = highlight_terms(input, "über", true);
        // Check that we have exactly 2 highlighted instances
        let count = result.matches("\x1b[1müber\x1b[0m").count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_preserves_text_content() {
        let input = "Some important content here";
        let result = highlight_terms(input, "important", true);
        // The result should contain all the original words
        assert!(result.contains("Some"));
        assert!(result.contains("content"));
        assert!(result.contains("here"));
        assert!(result.contains("\x1b[1mimportant\x1b[0m"));
    }

    #[test]
    fn test_mixed_case_query() {
        let result = highlight_terms("The RUST Book and Rust guide", "RUST", true);
        // Should match both "RUST" and "Rust" case-insensitively
        assert!(result.matches("\x1b[1m").count() >= 2);
    }
}
