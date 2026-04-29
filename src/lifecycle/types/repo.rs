#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepoSlug(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RepoSlugError {
    #[error("repo slug must not be empty")]
    Empty,
    #[error("repo slug must be in OWNER/REPO format")]
    InvalidFormat,
    #[error("repo slug contains invalid chars; expected [A-Za-z0-9._-]")]
    InvalidChars,
}

impl RepoSlug {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn owner(&self) -> &str {
        self.0.split_once('/').map_or("", |(owner, _)| owner)
    }

    #[must_use]
    pub fn repo(&self) -> &str {
        self.0.split_once('/').map_or("", |(_, repo)| repo)
    }

    /// Parses a repo slug from text.
    ///
    /// # Errors
    /// Returns `RepoSlugError::Empty` for blank input,
    /// `RepoSlugError::InvalidFormat` for missing/extra separators or empty parts,
    /// and `RepoSlugError::InvalidChars` when characters are outside `[A-Za-z0-9._-]`.
    pub fn parse(input: &str) -> Result<Self, RepoSlugError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(RepoSlugError::Empty);
        }
        let maybe_parts = trimmed.split_once('/').filter(|(_, right)| !right.contains('/'));
        let Some((owner, name)) = maybe_parts else {
            return Err(RepoSlugError::InvalidFormat);
        };
        if owner.is_empty() || name.is_empty() {
            return Err(RepoSlugError::InvalidFormat);
        }
        if !is_valid_repo_part(owner) || !is_valid_repo_part(name) {
            return Err(RepoSlugError::InvalidChars);
        }
        Ok(Self(trimmed.to_owned()))
    }
}

fn is_valid_repo_part(value: &str) -> bool {
    value.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_valid_owner_repo() {
        let slug = RepoSlug::parse("lprior-repo/oya")
            .unwrap_or_else(|_| crate::lifecycle::types::repo::RepoSlug("".to_string()));
        assert_eq!(slug.as_str(), "lprior-repo/oya");
        assert_eq!(slug.owner(), "lprior-repo");
        assert_eq!(slug.repo(), "oya");
    }

    #[test]
    fn parse_accepts_dots_and_underscores() {
        let slug = RepoSlug::parse("user_name/repo.name")
            .unwrap_or_else(|_| crate::lifecycle::types::repo::RepoSlug("".to_string()));
        assert_eq!(slug.as_str(), "user_name/repo.name");
    }

    #[test]
    fn parse_trims_whitespace() {
        let slug = RepoSlug::parse("  owner/repo  ")
            .unwrap_or_else(|_| crate::lifecycle::types::repo::RepoSlug("".to_string()));
        assert_eq!(slug.as_str(), "owner/repo");
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(matches!(RepoSlug::parse(""), Err(RepoSlugError::Empty)));
        assert!(matches!(RepoSlug::parse("   "), Err(RepoSlugError::Empty)));
    }

    #[test]
    fn parse_rejects_missing_separator() {
        assert!(matches!(RepoSlug::parse("ownerrepo"), Err(RepoSlugError::InvalidFormat)));
    }

    #[test]
    fn parse_rejects_extra_path_segments() {
        assert!(matches!(RepoSlug::parse("owner/repo/extra"), Err(RepoSlugError::InvalidFormat)));
    }

    #[test]
    fn parse_rejects_empty_parts() {
        assert!(matches!(RepoSlug::parse("/repo"), Err(RepoSlugError::InvalidFormat)));
        assert!(matches!(RepoSlug::parse("owner/"), Err(RepoSlugError::InvalidFormat)));
    }

    #[test]
    fn parse_rejects_invalid_chars() {
        assert!(matches!(RepoSlug::parse("owner/repo!"), Err(RepoSlugError::InvalidChars)));
        assert!(matches!(RepoSlug::parse("own@er/repo"), Err(RepoSlugError::InvalidChars)));
    }
}
