use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::BeadId;

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
        format!("/home/lewis/src/{}", self.0)
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
