#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BeadId(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BeadIdError {
    #[error("bead id must not be empty")]
    Empty,
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
    /// Returns `BeadIdError::Empty` for blank input and
    /// `BeadIdError::InvalidChars` when characters are outside `[a-z0-9-]`.
    pub fn parse(input: &str) -> Result<Self, BeadIdError> {
        if input.trim().is_empty() {
            Err(BeadIdError::Empty)
        } else if has_only_bead_chars(input) {
            Ok(Self(input.to_owned()))
        } else {
            Err(BeadIdError::InvalidChars)
        }
    }
}

fn has_only_bead_chars(input: &str) -> bool {
    input.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceName(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkspaceNameError {
    #[error("workspace name must not be empty")]
    Empty,
    #[error("workspace name must start with oya-")]
    MissingPrefix,
    #[error("workspace name does not contain a valid bead id")]
    InvalidBeadId,
}

impl WorkspaceName {
    #[must_use]
    pub fn from_bead_id(bead_id: &BeadId) -> Self {
        Self(format!("oya-{}", bead_id.as_str()))
    }

    /// Parses a workspace name in `oya-{bead_id}` form.
    ///
    /// # Errors
    /// Returns a `WorkspaceNameError` when input is blank, missing `oya-`,
    /// or the suffix is not a valid `BeadId`.
    pub fn parse(input: &str) -> Result<Self, WorkspaceNameError> {
        if input.trim().is_empty() {
            Err(WorkspaceNameError::Empty)
        } else {
            input.strip_prefix("oya-").ok_or(WorkspaceNameError::MissingPrefix).and_then(|suffix| {
                BeadId::parse(suffix)
                    .map(|_| Self(input.to_owned()))
                    .map_err(|_| WorkspaceNameError::InvalidBeadId)
            })
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn workspace_path(&self) -> String {
        format!("./workspaces/{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BookmarkName(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BookmarkNameError {
    #[error("bookmark name must not be empty")]
    Empty,
    #[error("bookmark name does not contain a valid bead id")]
    InvalidBeadId,
    #[error("bookmark name must match bead id")]
    BeadMismatch,
}

impl BookmarkName {
    #[must_use]
    pub fn from_bead_id(bead_id: &BeadId) -> Self {
        Self(bead_id.as_str().to_owned())
    }

    /// Parses a bookmark name and validates it belongs to `bead_id`.
    ///
    /// # Errors
    /// Returns a `BookmarkNameError` when input is blank, not a valid bead id,
    /// or refers to a different bead.
    pub fn parse_for_bead(input: &str, bead_id: &BeadId) -> Result<Self, BookmarkNameError> {
        if input.trim().is_empty() {
            Err(BookmarkNameError::Empty)
        } else {
            BeadId::parse(input).map_err(|_| BookmarkNameError::InvalidBeadId).and_then(|parsed| {
                if parsed == *bead_id {
                    Ok(Self(input.to_owned()))
                } else {
                    Err(BookmarkNameError::BeadMismatch)
                }
            })
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrNumber(u64);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PrNumberError {
    #[error("pr number must be greater than zero")]
    Zero,
}

impl PrNumber {
    /// Creates a pull request number.
    ///
    /// # Errors
    /// Returns `PrNumberError::Zero` when `value` is `0`.
    pub fn new(value: u64) -> Result<Self, PrNumberError> {
        if value == 0 {
            Err(PrNumberError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureCategory {
    Validation,
    Workspace,
    Bookmark,
    PullRequest,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadData {
    pub bead_id: BeadId,
    pub workspace: WorkspaceName,
    pub workspace_path: String,
    pub bookmark: BookmarkName,
}

impl BeadData {
    #[must_use]
    pub fn from_bead_id(bead_id: BeadId) -> Self {
        let workspace = WorkspaceName::from_bead_id(&bead_id);
        let bookmark = BookmarkName::from_bead_id(&bead_id);
        let workspace_path = workspace.workspace_path();
        Self { bead_id, workspace, workspace_path, bookmark }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: PrNumber,
    pub bookmark: BookmarkName,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleResult {
    pub bead: BeadData,
    pub pr: Option<PrInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Planned(BeadData),
    WorkspaceReady(BeadData),
    PrOpen { bead: BeadData, pr: PrInfo },
    Failed { bead: BeadData, category: FailureCategory, message: String },
    Completed(LifecycleResult),
}

impl Phase {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed(_) | Self::Failed { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleState {
    pub phase: Phase,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_id_parse_valid() {
        assert_eq!(BeadId::parse("src-abc123"), Ok(BeadId("src-abc123".into())));
    }

    #[test]
    fn test_bead_id_parse_empty() {
        assert_eq!(BeadId::parse(""), Err(BeadIdError::Empty));
    }

    #[test]
    fn test_bead_id_parse_invalid_chars() {
        assert_eq!(BeadId::parse("src-abc!123"), Err(BeadIdError::InvalidChars));
    }

    #[test]
    fn test_phase_is_terminal_completed() {
        let bead = BeadData::from_bead_id(BeadId("src-abc123".into()));
        let phase = Phase::Completed(LifecycleResult { bead, pr: None });
        assert!(phase.is_terminal());
    }
}
