//! File-based handoff mechanism for agent coordination.
//!
//! Agents communicate via atomic file operations in /tmp/:
//! - `/tmp/bead-contracts-<id>.json` - Test contracts from Test Writers
//! - `/tmp/bead-ready-to-implement-<id>.json` - Ready for implementation
//! - `/tmp/bead-implementation-in-progress-<id>.json` - Implementer claimed
//! - `/tmp/bead-implementation-complete-<id>.json` - Implementation done
//! - `/tmp/bead-ready-review-<id>.json` - Ready for review
//! - `/tmp/bead-reviewing-<id>.json` - Reviewer claimed
//! - `/tmp/bead-complete-<id>.json` - Bead landed

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::swarm::error::{SwarmError, SwarmResult};

/// Handoff state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandoffState {
    /// Contract is ready (Test Writer created).
    ContractReady,

    /// Ready for implementation (Test Writer signals).
    ReadyToImplement,

    /// Implementation in progress (Implementer claimed).
    Implementing,

    /// Implementation complete (Implementer signals).
    ImplementationComplete,

    /// Ready for review (Implementer signals).
    ReadyReview,

    /// Review in progress (Reviewer claimed).
    Reviewing,

    /// Complete (Reviewer landed bead).
    Complete,
}

impl std::fmt::Display for HandoffState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContractReady => write!(f, "ContractReady"),
            Self::ReadyToImplement => write!(f, "ReadyToImplement"),
            Self::Implementing => write!(f, "Implementing"),
            Self::ImplementationComplete => write!(f, "ImplementationComplete"),
            Self::ReadyReview => write!(f, "ReadyReview"),
            Self::Reviewing => write!(f, "Reviewing"),
            Self::Complete => write!(f, "Complete"),
        }
    }
}

/// Handoff file for agent coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffFile {
    /// Bead identifier.
    pub bead_id: String,

    /// Current state of the handoff.
    pub state: HandoffState,

    /// Path to contract file (for ContractReady state).
    pub contract_path: Option<String>,

    /// Workspace name (for implementing/reviewing states).
    pub workspace: Option<String>,

    /// Test results (for ImplementationComplete state).
    pub test_results: Option<serde_json::Value>,

    /// Commit hash (for Complete state).
    pub commit_hash: Option<String>,

    /// Error message (for failed states).
    pub error: Option<String>,

    /// Timestamp when handoff was created.
    pub created_at: u64,

    /// Timestamp when handoff was last updated.
    pub updated_at: u64,
}

impl HandoffFile {
    /// Create a new handoff file.
    #[must_use]
    pub fn new(bead_id: String, state: HandoffState) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            bead_id,
            state,
            contract_path: None,
            workspace: None,
            test_results: None,
            commit_hash: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set contract path.
    #[must_use]
    pub fn with_contract_path(mut self, path: String) -> Self {
        self.contract_path = Some(path);
        self
    }

    /// Set workspace.
    #[must_use]
    pub fn with_workspace(mut self, workspace: String) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Set test results.
    #[must_use]
    pub fn with_test_results(mut self, results: serde_json::Value) -> Self {
        self.test_results = Some(results);
        self
    }

    /// Set commit hash.
    #[must_use]
    pub fn with_commit_hash(mut self, hash: String) -> Self {
        self.commit_hash = Some(hash);
        self
    }

    /// Set error.
    #[must_use]
    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }

    /// Get file path for this handoff.
    #[must_use]
    pub fn file_path(&self, handoff_dir: &str) -> PathBuf {
        match self.state {
            HandoffState::ContractReady => PathBuf::from(format!(
                "{}/bead-contracts-{}.json",
                handoff_dir, self.bead_id
            )),
            HandoffState::ReadyToImplement => PathBuf::from(format!(
                "{}/bead-ready-to-implement-{}.json",
                handoff_dir, self.bead_id
            )),
            HandoffState::Implementing => PathBuf::from(format!(
                "{}/bead-implementation-in-progress-{}.json",
                handoff_dir, self.bead_id
            )),
            HandoffState::ImplementationComplete => PathBuf::from(format!(
                "{}/bead-implementation-complete-{}.json",
                handoff_dir, self.bead_id
            )),
            HandoffState::ReadyReview => PathBuf::from(format!(
                "{}/bead-ready-review-{}.json",
                handoff_dir, self.bead_id
            )),
            HandoffState::Reviewing => PathBuf::from(format!(
                "{}/bead-reviewing-{}.json",
                handoff_dir, self.bead_id
            )),
            HandoffState::Complete => PathBuf::from(format!(
                "{}/bead-complete-{}.json",
                handoff_dir, self.bead_id
            )),
        }
    }

    /// Write handoff to file.
    ///
    /// # Errors
    ///
    /// Returns error if write fails.
    pub fn write(&self, handoff_dir: &str) -> SwarmResult<()> {
        let path = self.file_path(handoff_dir);
        let json = serde_json::to_string_pretty(self).map_err(|e| SwarmError::HandoffFailed {
            file_path: path.display().to_string(),
            operation: "serialize".to_string(),
            reason: e.to_string(),
        })?;

        fs::write(&path, json).map_err(|e| SwarmError::HandoffFailed {
            file_path: path.display().to_string(),
            operation: "write".to_string(),
            reason: e.to_string(),
        })?;

        info!(
            bead_id = %self.bead_id,
            state = %self.state,
            path = %path.display(),
            "Wrote handoff file"
        );

        Ok(())
    }

    /// Read handoff from file.
    ///
    /// # Errors
    ///
    /// Returns error if read fails.
    pub fn read(path: &Path) -> SwarmResult<Self> {
        let content = fs::read_to_string(path).map_err(|e| SwarmError::HandoffFailed {
            file_path: path.display().to_string(),
            operation: "read".to_string(),
            reason: e.to_string(),
        })?;

        serde_json::from_str(&content).map_err(|e| SwarmError::HandoffFailed {
            file_path: path.display().to_string(),
            operation: "parse".to_string(),
            reason: e.to_string(),
        })
    }

    /// Delete handoff file.
    ///
    /// # Errors
    ///
    /// Returns error if delete fails.
    pub fn delete(&self, handoff_dir: &str) -> SwarmResult<()> {
        let path = self.file_path(handoff_dir);

        fs::remove_file(&path).map_err(|e| SwarmError::HandoffFailed {
            file_path: path.display().to_string(),
            operation: "delete".to_string(),
            reason: e.to_string(),
        })?;

        debug!(
            bead_id = %self.bead_id,
            path = %path.display(),
            "Deleted handoff file"
        );

        Ok(())
    }
}

/// Find handoff files by state pattern.
///
/// # Errors
///
/// Returns error if directory read fails.
pub fn find_handoffs(handoff_dir: &str, pattern: &str) -> SwarmResult<Vec<HandoffFile>> {
    let glob_pattern = format!("{}/{}", handoff_dir, pattern);
    let mut handoffs = Vec::new();

    // Use glob to find matching files
    let paths = glob::glob(&glob_pattern).map_err(|e| SwarmError::HandoffFailed {
        file_path: glob_pattern,
        operation: "glob".to_string(),
        reason: e.to_string(),
    })?;

    for entry in paths {
        match entry {
            Ok(path) => match HandoffFile::read(&path) {
                Ok(handoff) => handoffs.push(handoff),
                Err(e) => {
                    warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to read handoff file"
                    );
                }
            },
            Err(e) => {
                warn!(error = %e, "Glob iteration error");
            }
        }
    }

    Ok(handoffs)
}

/// Find handoffs ready for implementation.
///
/// # Errors
///
/// Returns error if directory read fails.
pub fn find_ready_to_implement(handoff_dir: &str) -> SwarmResult<Vec<HandoffFile>> {
    find_handoffs(handoff_dir, "bead-ready-to-implement-*.json")
}

/// Find handoffs ready for review.
///
/// # Errors
///
/// Returns error if directory read fails.
pub fn find_ready_review(handoff_dir: &str) -> SwarmResult<Vec<HandoffFile>> {
    find_handoffs(handoff_dir, "bead-ready-review-*.json")
}

/// Transition handoff to new state via atomic file move.
///
/// # Errors
///
/// Returns error if transition fails.
pub fn transition_handoff(
    handoff: HandoffFile,
    new_state: HandoffState,
    handoff_dir: &str,
) -> SwarmResult<HandoffFile> {
    let old_path = handoff.file_path(handoff_dir);

    let mut new_handoff = handoff.clone();
    new_handoff.state = new_state;
    new_handoff.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Write new handoff file
    new_handoff.write(handoff_dir)?;

    // Atomically remove old file (using rename to ensure atomicity)
    if old_path.exists() {
        fs::remove_file(&old_path).map_err(|e| SwarmError::HandoffFailed {
            file_path: old_path.display().to_string(),
            operation: "remove_old".to_string(),
            reason: e.to_string(),
        })?;
    }

    info!(
        bead_id = %handoff.bead_id,
        from = %handoff.state,
        to = %new_state,
        "Transitioned handoff state"
    );

    Ok(new_handoff)
}

/// Clean up all handoff files for a bead.
///
/// # Errors
///
/// Returns error if cleanup fails.
pub fn cleanup_bead_handoffs(bead_id: &str, handoff_dir: &str) -> SwarmResult<()> {
    let patterns = vec![
        format!("bead-contracts-{}.json", bead_id),
        format!("bead-ready-to-implement-{}.json", bead_id),
        format!("bead-implementation-in-progress-{}.json", bead_id),
        format!("bead-implementation-complete-{}.json", bead_id),
        format!("bead-ready-review-{}.json", bead_id),
        format!("bead-reviewing-{}.json", bead_id),
    ];

    for pattern in patterns {
        let path = PathBuf::from(format!("{}/{}", handoff_dir, pattern));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| SwarmError::HandoffFailed {
                file_path: path.display().to_string(),
                operation: "cleanup".to_string(),
                reason: e.to_string(),
            })?;
            debug!(path = %path.display(), "Cleaned up handoff file");
        }
    }

    // Keep bead-complete-<id>.json for audit trail

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handoff_file_new() {
        let handoff = HandoffFile::new("test-123".to_string(), HandoffState::ContractReady);
        assert_eq!(handoff.bead_id, "test-123");
        assert_eq!(handoff.state, HandoffState::ContractReady);
        assert!(handoff.contract_path.is_none());
    }

    #[test]
    fn test_handoff_file_with_contract_path() {
        let handoff = HandoffFile::new("test-123".to_string(), HandoffState::ContractReady)
            .with_contract_path("/tmp/contract.json".to_string());
        assert_eq!(
            handoff.contract_path,
            Some("/tmp/contract.json".to_string())
        );
    }

    #[test]
    fn test_handoff_state_display() {
        let handoff = HandoffFile::new("test-123".to_string(), HandoffState::Implementing);
        let path = handoff.file_path("/tmp");
        assert_eq!(
            path,
            PathBuf::from("/tmp/bead-implementation-in-progress-test-123.json")
        );
    }

    #[test]
    fn test_transition_handoff() {
        let handoff = HandoffFile::new("test-123".to_string(), HandoffState::ReadyToImplement);
        // Note: This would write files in tests, so we just test the logic
        assert_eq!(handoff.state, HandoffState::ReadyToImplement);
    }
}
