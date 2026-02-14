//! Slug type for task identifiers.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A slug is a unique identifier for a task or workflow.
///
/// Slugs are URL-friendly identifiers that follow specific formatting rules:
/// - Must be lowercase
/// - Must contain only ASCII alphanumeric characters and hyphens
/// - Must start and end with alphanumeric characters
/// - Must not contain consecutive hyphens
///
/// # ASCII-Only Requirement
///
/// Slugs are restricted to ASCII characters (a-z, 0-9, hyphen) to ensure:
/// - URL compatibility across all systems
/// - Consistent display in terminal UI
/// - Simple validation and parsing
/// - No encoding issues in file paths or storage
///
/// For internationalization, use the `title` field for display names
/// (which supports Unicode) while keeping slugs ASCII-only for identifiers.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Slug(String);

impl Slug {
    /// Maximum length for a slug.
    pub const MAX_LENGTH: usize = 100;

    /// Create a new slug from a string, validating the format.
    ///
    /// # Errors
    /// Returns an error if the slug format is invalid.
    #[inline]
    pub fn new(slug: impl Into<String>) -> Result<Self, crate::OyaError> {
        let slug = slug.into();

        // Validate slug format
        if slug.is_empty() {
            return Err(crate::OyaError::validation("slug", "cannot be empty"));
        }

        if slug.len() > Self::MAX_LENGTH {
            return Err(crate::OyaError::validation(
                "slug",
                format!("exceeds maximum length of {}", Self::MAX_LENGTH),
            ));
        }

        if !slug.is_ascii() {
            return Err(crate::OyaError::validation(
                "slug",
                "must contain only ASCII characters",
            ));
        }

        if slug != slug.to_ascii_lowercase() {
            return Err(crate::OyaError::validation("slug", "must be lowercase"));
        }

        if !slug.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(crate::OyaError::validation(
                "slug",
                "must contain only alphanumeric characters and hyphens",
            ));
        }

        if slug.contains("--") {
            return Err(crate::OyaError::validation(
                "slug",
                "must not contain consecutive hyphens",
            ));
        }

        if slug.starts_with('-') || slug.ends_with('-') {
            return Err(crate::OyaError::validation(
                "slug",
                "must start and end with alphanumeric characters",
            ));
        }

        if !slug.chars().next().is_some_and(char::is_alphanumeric) {
            return Err(crate::OyaError::validation(
                "slug",
                "must start with an alphanumeric character",
            ));
        }

        Ok(Self(slug))
    }

    /// Create a new slug without validation (for internal use).
    ///
    /// # Safety
    /// Only use this when you're certain the slug is valid.
    #[must_use]
    #[inline]
    pub fn new_unchecked(slug: impl Into<String>) -> Self {
        Self(slug.into())
    }

    /// Get the underlying string value.
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the underlying string.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for Slug {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Slug {
    type Error = crate::OyaError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Slug {
    type Error = crate::OyaError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Slug> for String {
    #[inline]
    fn from(slug: Slug) -> Self {
        slug.0
    }
}

impl AsRef<str> for Slug {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_valid() -> Result<(), Box<dyn std::error::Error>> {
        assert!(Slug::new("valid-slug").is_ok());
        assert!(Slug::new("my-task-123").is_ok());
        assert!(Slug::new("abc").is_ok());
        Ok(())
    }

    #[test]
    fn test_slug_empty() -> Result<(), Box<dyn std::error::Error>> {
        let result = Slug::new("");
        assert!(result.is_err());
        let err = result.err().ok_or("Expected error for empty slug")?;
        assert!(err.to_string().contains("cannot be empty"));
        Ok(())
    }

    #[test]
    fn test_slug_too_long() -> Result<(), Box<dyn std::error::Error>> {
        let long_slug = "a".repeat(Slug::MAX_LENGTH + 1);
        let result = Slug::new(long_slug);
        assert!(result.is_err());
        let err = result.err().ok_or("Expected error for too long slug")?;
        assert!(err.to_string().contains("exceeds maximum length"));
        Ok(())
    }

    #[test]
    fn test_slug_non_ascii() -> Result<(), Box<dyn std::error::Error>> {
        let result = Slug::new("invalid-日本語");
        assert!(result.is_err());
        let err = result.err().ok_or("Expected error for non-ASCII slug")?;
        assert!(err.to_string().contains("ASCII"));
        Ok(())
    }

    #[test]
    fn test_slug_uppercase_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let result = Slug::new("Invalid-Slug");
        assert!(result.is_err());
        let err = result.err().ok_or("Expected error for uppercase slug")?;
        assert!(err.to_string().contains("lowercase"));
        Ok(())
    }

    #[test]
    fn test_slug_invalid_characters() -> Result<(), Box<dyn std::error::Error>> {
        let result = Slug::new("invalid_slug");
        assert!(result.is_err());
        let err = result
            .err()
            .ok_or("Expected error for invalid characters")?;
        assert!(err.to_string().contains("alphanumeric"));
        Ok(())
    }

    #[test]
    fn test_slug_consecutive_hyphens() -> Result<(), Box<dyn std::error::Error>> {
        let result = Slug::new("invalid--slug");
        assert!(result.is_err());
        let err = result
            .err()
            .ok_or("Expected error for consecutive hyphens")?;
        assert!(err.to_string().contains("consecutive"));
        Ok(())
    }

    #[test]
    fn test_slug_starts_with_hyphen() -> Result<(), Box<dyn std::error::Error>> {
        let result = Slug::new("-invalid");
        assert!(result.is_err());
        let err = result
            .err()
            .ok_or("Expected error for slug starting with hyphen")?;
        assert!(err.to_string().contains("start"));
        Ok(())
    }

    #[test]
    fn test_slug_ends_with_hyphen() -> Result<(), Box<dyn std::error::Error>> {
        let result = Slug::new("invalid-");
        assert!(result.is_err());
        let err = result
            .err()
            .ok_or("Expected error for slug ending with hyphen")?;
        assert!(err.to_string().contains("end"));
        Ok(())
    }

    #[test]
    fn test_slug_as_str() -> Result<(), Box<dyn std::error::Error>> {
        let slug = Slug::new("test-slug")?;
        assert_eq!(slug.as_str(), "test-slug");
        Ok(())
    }

    #[test]
    fn test_slug_into_inner() -> Result<(), Box<dyn std::error::Error>> {
        let slug = Slug::new("test-slug")?;
        assert_eq!(slug.into_inner(), "test-slug");
        Ok(())
    }

    #[test]
    fn test_slug_display() -> Result<(), Box<dyn std::error::Error>> {
        let slug = Slug::new("test-slug")?;
        assert_eq!(format!("{slug}"), "test-slug");
        Ok(())
    }

    #[test]
    fn test_slug_try_from_string() -> Result<(), Box<dyn std::error::Error>> {
        let slug = Slug::try_from("test-slug".to_string());
        assert!(slug.is_ok());
        Ok(())
    }

    #[test]
    fn test_slug_try_from_str() -> Result<(), Box<dyn std::error::Error>> {
        let slug = Slug::try_from("test-slug");
        assert!(slug.is_ok());
        Ok(())
    }

    #[test]
    fn test_slug_from_into_string() -> Result<(), Box<dyn std::error::Error>> {
        let slug = Slug::new("test-slug")?;
        let s: String = slug.into();
        assert_eq!(s, "test-slug");
        Ok(())
    }
}
