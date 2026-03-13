use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::MAX_BEAD_ID_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadStatus {
    Planned,
    InProgress,
    Blocked,
    Completed,
}

impl BeadStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
        }
    }

    /// Parses a bead status from text.
    ///
    /// # Errors
    /// Returns `BeadStatusError::Invalid` when input is not one of
    /// "planned", "`in_progress`", "blocked", or "completed".
    pub fn parse(input: &str) -> Result<Self, BeadStatusError> {
        match input.trim().to_lowercase().as_str() {
            "planned" => Ok(Self::Planned),
            "in_progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "completed" => Ok(Self::Completed),
            _ => Err(BeadStatusError::Invalid(input.to_owned())),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BeadStatusError {
    #[error("invalid bead status: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BeadId(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BeadIdError {
    #[error("bead id must not be empty")]
    Empty,
    #[error("bead id exceeds max length: {len} > {max}")]
    TooLong { len: usize, max: usize },
    #[error("bead id contains invalid chars")]
    InvalidChars,
}

impl BeadId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses a bead id from text.
    ///
    /// # Errors
    /// Returns `BeadIdError::Empty` for blank input,
    /// `BeadIdError::TooLong` for IDs over 64 chars, and
    /// `BeadIdError::InvalidChars` when characters are outside `[a-z0-9-]`.
    pub fn parse(input: &str) -> Result<Self, BeadIdError> {
        let normalized = input.trim();
        if normalized.is_empty() {
            Err(BeadIdError::Empty)
        } else if normalized.len() > MAX_BEAD_ID_LEN {
            Err(BeadIdError::TooLong { len: normalized.len(), max: MAX_BEAD_ID_LEN })
        } else if has_only_bead_chars(normalized) {
            Ok(Self(normalized.to_owned()))
        } else {
            Err(BeadIdError::InvalidChars)
        }
    }
}

fn has_only_bead_chars(input: &str) -> bool {
    input.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}
