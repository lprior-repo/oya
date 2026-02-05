//! Tests for pipeline domain types.
//!
//! Validates Slug, GitHash, and Language types.

use oya_core::Result;
use oya_shared::{GitHash, Language, Slug};

#[test]
fn test_slug_new_valid() {
    let slug = Slug::new("valid-slug-123").expect("Valid slug");
    assert_eq!(slug.as_str(), "valid-slug-123");
}

#[test]
fn test_slug_new_empty() {
    let result = Slug::new("");
    assert!(result.is_err());
}

#[test]
fn test_slug_new_too_long() {
    let long_slug = "a".repeat(51);
    let result = Slug::new(long_slug);
    assert!(result.is_err());
}

#[test]
fn test_slug_new_invalid_characters() {
    let invalid = "slug@bad";
    let result = Slug::new(invalid);
    assert!(result.is_err());
}

#[test]
fn test_slug_has_separators() {
    let slug = Slug::new("valid-slug").expect("Valid slug");
    assert!(!slug.has_separators());
}

#[test]
fn test_slug_without_separators() {
    let slug = Slug::new("noseparator").expect("Valid slug");
    assert!(!slug.has_separators());
}

#[test]
fn test_slug_from_string() {
    let s = String::from("test-slug");
    let slug: Slug = s.into();
    assert_eq!(slug.as_str(), "test-slug");
}

#[test]
fn test_slug_into_string() {
    let slug = Slug::new("test").expect("Valid slug");
    let s: String = slug.into();
    assert_eq!(s, "test");
}

#[test]
fn test_slug_display() {
    let slug = Slug::new("test-slug").expect("Valid slug");
    assert_eq!(format!("{}", slug), "test-slug");
}

#[test]
fn test_git_hash_new_valid() {
    let hash = GitHash::new("abc123def456abc123def456abc123").expect("Valid hash");
    assert_eq!(hash.as_str(), "abc123def456abc123def456abc123");
}

#[test]
fn test_git_hash_new_too_short() {
    let short = "abc123";
    let result = GitHash::new(short);
    assert!(result.is_err());
}

#[test]
fn test_git_hash_new_too_long() {
    let long = "abc123def456abc123def456abc123def";
    let result = GitHash::new(long);
    assert!(result.is_err());
}

#[test]
fn test_git_hash_new_invalid_characters() {
    let invalid = "ghijklmnopqrstuvwxyz";
    let result = GitHash::new(invalid);
    assert!(result.is_err());
}

#[test]
fn test_git_hash_from_string() {
    let s = String::from("ABC123DEF456");
    let hash: GitHash = s.into();
    assert_eq!(hash.as_str(), "abc123def456");
}

#[test]
fn test_git_hash_into_string() {
    let hash = GitHash::new("abc123def456abc123def456abc123").expect("Valid hash");
    let s: String = hash.into();
    assert_eq!(s, "abc123def456abc123def456abc123");
}

#[test]
fn test_git_hash_display() {
    let hash = GitHash::new("abc123def456").expect("Valid hash");
    assert_eq!(format!("{}", hash), "abc123def456");
}

#[test]
fn test_language_equality() {
    assert_eq!(Language::Go, Language::Go);
    assert_eq!(Language::Gleam, Language::Gleam);
    assert_eq!(Language::Rust, Language::Rust);
    assert_eq!(Language::Python, Language::Python);
    assert_eq!(Language::Javascript, Language::Javascript);
}

#[test]
fn test_language_serialization() {
    use serde_json;

    let languages = vec![
        Language::Go,
        Language::Gleam,
        Language::Rust,
        Language::Python,
        Language::Javascript,
    ];

    for lang in languages {
        let json = serde_json::to_string(&lang);
        let deserialized: Language = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized, lang);
    }
}
