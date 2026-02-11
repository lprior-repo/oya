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
    pub fn new_unchecked(slug: impl Into<String>) -> Self {
        Self(slug.into())
    }

    /// Get the underlying string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the underlying string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Slug {
    type Error = crate::OyaError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Slug {
    type Error = crate::OyaError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Slug> for String {
    fn from(slug: Slug) -> Self {
        slug.0
    }
}

impl AsRef<str> for Slug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {

    #![allow(clippy::uninlined_format_args)]
    #![allow(clippy::manual_let_else)]
    #![allow(clippy::missing_panics_doc)]
    #![allow(clippy::struct_field_names)]
    #![allow(clippy::should_implement_trait)]
    #![allow(clippy::if_then_some_else_none)]
    #![allow(clippy::redundant_clone)]
    #![allow(clippy::map_or_none)]
    #![allow(clippy::missing_docs_in_private_items)]

    use super::*;

    #[test]
    fn test_slug_valid() {
        assert!(Slug::new("valid-slug").is_ok());
        assert!(Slug::new("my-task-123").is_ok());
        assert!(Slug::new("abc").is_ok());
    }

    #[test]
    fn test_slug_empty() {
        let result = Slug::new("");
        assert!(result.is_err());
        let err = match result {
            Err(e) => e,
            Ok(_) => {
                panic!("Expected error for empty slug");
            }
        };
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_slug_too_long() {
        let long_slug = "a".repeat(Slug::MAX_LENGTH + 1);
        let result = Slug::new(long_slug);
        assert!(result.is_err());
        let err = match result {
            Err(e) => e,
            Ok(_) => {
                panic!("Expected error for too long slug");
            }
        };
        assert!(err.to_string().contains("exceeds maximum length"));
    }

    #[test]
    fn test_slug_non_ascii() {
        let result = Slug::new("invalid-日本語");
        assert!(result.is_err());
        let err = match result {
            Err(e) => e,
            Ok(_) => {
                panic!("Expected error for non-ASCII slug");
            }
        };
        assert!(err.to_string().contains("ASCII"));
    }

    #[test]
    fn test_slug_uppercase_rejected() {
        let result = Slug::new("Invalid-Slug");
        assert!(result.is_err());
        let err = match result {
            Err(e) => e,
            Ok(_) => {
                panic!("Expected error for uppercase slug");
            }
        };
        assert!(err.to_string().contains("lowercase"));
    }

    #[test]
    fn test_slug_invalid_characters() {
        let result = Slug::new("invalid_slug");
        assert!(result.is_err());
        let err = match result {
            Err(e) => e,
            Ok(_) => {
                panic!("Expected error for invalid characters");
            }
        };
        assert!(err.to_string().contains("alphanumeric"));
    }

    #[test]
    fn test_slug_consecutive_hyphens() {
        let result = Slug::new("invalid--slug");
        assert!(result.is_err());
        let err = match result {
            Err(e) => e,
            Ok(_) => {
                panic!("Expected error for consecutive hyphens");
            }
        };
        assert!(err.to_string().contains("consecutive"));
    }

    #[test]
    fn test_slug_starts_with_hyphen() {
        let result = Slug::new("-invalid");
        assert!(result.is_err());
        let err = match result {
            Err(e) => e,
            Ok(_) => {
                panic!("Expected error for slug starting with hyphen");
            }
        };
        assert!(err.to_string().contains("start"));
    }

    #[test]
    fn test_slug_ends_with_hyphen() {
        let result = Slug::new("invalid-");
        assert!(result.is_err());
        let err = match result {
            Err(e) => e,
            Ok(_) => {
                panic!("Expected error for slug ending with hyphen");
            }
        };
        assert!(err.to_string().contains("end"));
    }

    #[test]
    fn test_slug_as_str() {
        let slug = match Slug::new("test-slug") {
            Ok(s) => s,
            Err(e) => {
                panic!("Failed to create slug: {e}");
            }
        };
        assert_eq!(slug.as_str(), "test-slug");
    }

    #[test]
    fn test_slug_into_inner() {
        let slug = match Slug::new("test-slug") {
            Ok(s) => s,
            Err(e) => {
                panic!("Failed to create slug: {e}");
            }
        };
        assert_eq!(slug.into_inner(), "test-slug");
    }

    #[test]
    fn test_slug_display() {
        let slug = match Slug::new("test-slug") {
            Ok(s) => s,
            Err(e) => {
                panic!("Failed to create slug: {e}");
            }
        };
        assert_eq!(format!("{}", slug), "test-slug");
    }

    #[test]
    fn test_slug_try_from_string() {
        let slug = Slug::try_from("test-slug".to_string());
        assert!(slug.is_ok());
    }

    #[test]
    fn test_slug_try_from_str() {
        let slug = Slug::try_from("test-slug");
        assert!(slug.is_ok());
    }

    #[test]
    fn test_slug_from_into_string() {
        let slug = match Slug::new("test-slug") {
            Ok(s) => s,
            Err(e) => {
                panic!("Failed to create slug: {e}");
            }
        };
        let s: String = slug.into();
        assert_eq!(s, "test-slug");
    }
}
