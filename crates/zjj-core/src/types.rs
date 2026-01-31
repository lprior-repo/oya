//! Core domain types for zjj with contracts and validation
//!
//! All types implement the `HasContract` trait, providing:
//! - Type constraints and validation
//! - Contextual hints for AI agents
//! - JSON Schema generation
//! - Self-documenting APIs

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    contracts::{Constraint, ContextualHint, FieldContract, HasContract, HintType, TypeContract},
    Error, Result,
};

// ═══════════════════════════════════════════════════════════════════════════
// SESSION TYPES
// ═════════════════════════════════════════════════════════════════════════

/// Session lifecycle states
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is being created (transient)
    Creating,
    /// Session is ready for use
    Active,
    /// Session exists but not currently in use
    Paused,
    /// Work completed, ready for removal
    Completed,
    /// Creation or hook failed
    Failed,
}

impl SessionStatus {
    /// Valid state transitions
    ///
    /// # Returns
    /// `true` if transition from current state to `next` is valid
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Creating | Self::Paused, Self::Active)
                | (Self::Creating, Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed)
                | (Self::Paused, Self::Completed)
        )
    }

    /// Allowed operations in this state
    pub const fn allowed_operations(self) -> &'static [Operation] {
        match self {
            Self::Creating => &[],
            Self::Active => &[
                Operation::Status,
                Operation::Diff,
                Operation::Focus,
                Operation::Remove,
            ],
            Self::Paused => &[Operation::Status, Operation::Focus, Operation::Remove],
            Self::Completed | Self::Failed => &[Operation::Remove],
        }
    }

    /// Check if an operation is allowed in this state
    pub fn allows_operation(self, op: Operation) -> bool {
        self.allowed_operations().contains(&op)
    }
}

/// Operations that can be performed on sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// View session status
    Status,
    /// View diff
    Diff,
    /// Focus session
    Focus,
    /// Remove session
    Remove,
}

/// A session represents a parallel workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: String,

    /// Human-readable session name
    ///
    /// # Contract
    /// - MUST match regex: `^[a-zA-Z0-9_-]+$`
    /// - MUST be unique across all sessions
    /// - MUST NOT exceed 64 characters
    pub name: String,

    /// Current session status
    pub status: SessionStatus,

    /// Absolute path to workspace directory
    ///
    /// # Contract
    /// - MUST be absolute path
    /// - MUST exist if status != Creating
    pub workspace_path: PathBuf,

    /// Optional branch name
    ///
    /// # Contract
    /// - `Some` if session has explicit branch
    /// - `None` if using anonymous branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Creation timestamp (UTC)
    pub created_at: DateTime<Utc>,

    /// Last update timestamp (UTC)
    pub updated_at: DateTime<Utc>,

    /// Last sync timestamp (UTC, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<DateTime<Utc>>,

    /// Arbitrary metadata (extensibility)
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Session {
    /// Validate session invariants
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Name doesn't match regex
    /// - Workspace path is not absolute
    /// - Workspace doesn't exist (if status != Creating)
    /// - Timestamps are in wrong order
    pub fn validate(&self) -> Result<()> {
        // Name validation
        let name_regex = regex::Regex::new(r"^[a-zA-Z0-9_-]+$")
            .map_err(|e| Error::ValidationError(format!("Invalid regex: {e}")))?;

        if !name_regex.is_match(&self.name) {
            return Err(Error::ValidationError(format!(
                "Session name '{}' must contain only alphanumeric characters, hyphens, and underscores",
                self.name
            )));
        }

        if self.name.len() > 64 {
            return Err(Error::ValidationError(format!(
                "Session name '{}' exceeds maximum length of 64 characters",
                self.name
            )));
        }

        // Path validation
        if !self.workspace_path.is_absolute() {
            return Err(Error::ValidationError(format!(
                "Workspace path '{}' must be absolute",
                self.workspace_path.display()
            )));
        }

        // Existence check (except during creation)
        if self.status != SessionStatus::Creating && !self.workspace_path.exists() {
            return Err(Error::ValidationError(format!(
                "Workspace '{}' does not exist",
                self.workspace_path.display()
            )));
        }

        // Timestamp order
        if self.updated_at < self.created_at {
            return Err(Error::ValidationError(
                "Updated timestamp cannot be before created timestamp".to_string(),
            ));
        }

        Ok(())
    }
}

impl HasContract for Session {
    fn contract() -> TypeContract {
        TypeContract::builder("Session")
            .description("A parallel workspace for isolating work")
            .field(
                "name",
                FieldContract::builder("name", "String")
                    .required()
                    .description("Human-readable session name")
                    .constraint(Constraint::Regex {
                        pattern: r"^[a-zA-Z0-9_-]+$".to_string(),
                        description: "alphanumeric with hyphens and underscores".to_string(),
                    })
                    .constraint(Constraint::Length {
                        min: Some(1),
                        max: Some(64),
                    })
                    .constraint(Constraint::Unique)
                    .example("feature-auth")
                    .example("bugfix-123")
                    .example("experiment_idea")
                    .build(),
            )
            .field(
                "status",
                FieldContract::builder("status", "SessionStatus")
                    .required()
                    .description("Current lifecycle state of the session")
                    .constraint(Constraint::Enum {
                        values: vec![
                            "creating".to_string(),
                            "active".to_string(),
                            "paused".to_string(),
                            "completed".to_string(),
                            "failed".to_string(),
                        ],
                    })
                    .build(),
            )
            .field(
                "workspace_path",
                FieldContract::builder("workspace_path", "PathBuf")
                    .required()
                    .description("Absolute path to the workspace directory")
                    .constraint(Constraint::PathAbsolute)
                    .constraint(Constraint::Custom {
                        rule: "must exist if status != creating".to_string(),
                        description: "Workspace directory must exist for non-creating sessions"
                            .to_string(),
                    })
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::BestPractice,
                message: "Use descriptive session names that indicate the purpose of the work"
                    .to_string(),
                condition: None,
                related_to: Some("name".to_string()),
            })
            .hint(ContextualHint {
                hint_type: HintType::Warning,
                message: "Session name cannot be changed after creation".to_string(),
                condition: None,
                related_to: Some("name".to_string()),
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        self.validate()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CHANGE TRACKING TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// File modification status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FileStatus {
    /// File modified
    #[serde(rename = "M")]
    Modified,
    /// File added
    #[serde(rename = "A")]
    Added,
    /// File deleted
    #[serde(rename = "D")]
    Deleted,
    /// File renamed
    #[serde(rename = "R")]
    Renamed,
    /// File untracked
    #[serde(rename = "?")]
    Untracked,
}

/// A single file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// File path relative to workspace root
    pub path: PathBuf,

    /// Modification status
    pub status: FileStatus,

    /// Original path (only for `Renamed`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<PathBuf>,
}

impl HasContract for FileChange {
    fn contract() -> TypeContract {
        TypeContract::builder("FileChange")
            .description("Represents a change to a file in the workspace")
            .field(
                "path",
                FieldContract::builder("path", "PathBuf")
                    .required()
                    .description("File path relative to workspace root")
                    .build(),
            )
            .field(
                "status",
                FieldContract::builder("status", "FileStatus")
                    .required()
                    .description("Type of modification")
                    .constraint(Constraint::Enum {
                        values: vec![
                            "M".to_string(),
                            "A".to_string(),
                            "D".to_string(),
                            "R".to_string(),
                            "?".to_string(),
                        ],
                    })
                    .build(),
            )
            .field(
                "old_path",
                FieldContract::builder("old_path", "Option<PathBuf>")
                    .description("Original path for renamed files")
                    .constraint(Constraint::Custom {
                        rule: "required when status is Renamed".to_string(),
                        description: "Must be set when file is renamed".to_string(),
                    })
                    .build(),
            )
            .build()
    }

    fn validate(&self) -> Result<()> {
        if self.status == FileStatus::Renamed && self.old_path.is_none() {
            return Err(Error::ValidationError(
                "Renamed files must have old_path set".to_string(),
            ));
        }
        Ok(())
    }
}

/// Summary of changes in a workspace
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangesSummary {
    /// Number of modified files
    pub modified: usize,

    /// Number of added files
    pub added: usize,

    /// Number of deleted files
    pub deleted: usize,

    /// Number of renamed files
    pub renamed: usize,

    /// Number of untracked files
    pub untracked: usize,
}

impl ChangesSummary {
    /// Total number of changed files
    #[must_use]
    pub const fn total(&self) -> usize {
        self.modified + self.added + self.deleted + self.renamed
    }

    /// Has any changes?
    #[must_use]
    pub const fn has_changes(&self) -> bool {
        self.total() > 0
    }

    /// Has any tracked changes (excluding untracked)?
    #[must_use]
    pub const fn has_tracked_changes(&self) -> bool {
        self.modified + self.added + self.deleted + self.renamed > 0
    }
}

impl HasContract for ChangesSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("ChangesSummary")
            .description("Summary of file changes in a workspace")
            .field(
                "modified",
                FieldContract::builder("modified", "usize")
                    .required()
                    .description("Number of modified files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .field(
                "added",
                FieldContract::builder("added", "usize")
                    .required()
                    .description("Number of added files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .field(
                "deleted",
                FieldContract::builder("deleted", "usize")
                    .required()
                    .description("Number of deleted files")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .default("0")
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::Example,
                message: "Use total() method to get sum of all changes".to_string(),
                condition: None,
                related_to: None,
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        // All fields are usize, so always valid
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DIFF TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Diff statistics for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffStat {
    /// File path
    pub path: PathBuf,

    /// Lines inserted
    pub insertions: usize,

    /// Lines deleted
    pub deletions: usize,

    /// File status (`A`/`M`/`D`/`R`)
    pub status: FileStatus,
}

/// Summary of diff statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of lines inserted
    pub insertions: usize,

    /// Number of lines deleted
    pub deletions: usize,

    /// Number of files changed
    pub files_changed: usize,

    /// Per-file statistics
    pub files: Vec<FileDiffStat>,
}

impl HasContract for DiffSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("DiffSummary")
            .description("Summary of differences between commits or workspace state")
            .field(
                "insertions",
                FieldContract::builder("insertions", "usize")
                    .required()
                    .description("Total number of lines inserted")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .field(
                "deletions",
                FieldContract::builder("deletions", "usize")
                    .required()
                    .description("Total number of lines deleted")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .field(
                "files_changed",
                FieldContract::builder("files_changed", "usize")
                    .required()
                    .description("Number of files changed")
                    .constraint(Constraint::Range {
                        min: Some(0),
                        max: None,
                        inclusive: true,
                    })
                    .build(),
            )
            .build()
    }

    fn validate(&self) -> Result<()> {
        if self.files.len() != self.files_changed {
            return Err(Error::ValidationError(format!(
                "files_changed ({}) does not match files array length ({})",
                self.files_changed,
                self.files.len()
            )));
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BEADS TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Issue status from beads
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    Open,
    InProgress,
    Blocked,
    Closed,
}

/// A beads issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsIssue {
    /// Issue ID (e.g., "zjj-abc")
    pub id: String,

    /// Issue title
    pub title: String,

    /// Issue status
    pub status: IssueStatus,

    /// Priority (e.g., "P1", "P2")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,

    /// Issue type (e.g., "task", "bug", "feature")
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
}

/// Summary of beads issues
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeadsSummary {
    /// Number of open issues
    pub open: usize,

    /// Number of in-progress issues
    pub in_progress: usize,

    /// Number of blocked issues
    pub blocked: usize,

    /// Number of closed issues
    pub closed: usize,
}

impl BeadsSummary {
    /// Total number of issues
    #[must_use]
    pub const fn total(&self) -> usize {
        self.open + self.in_progress + self.blocked + self.closed
    }

    /// Number of active issues (open + `in_progress`)
    #[must_use]
    pub const fn active(&self) -> usize {
        self.open + self.in_progress
    }

    /// Has blocking issues?
    #[must_use]
    pub const fn has_blockers(&self) -> bool {
        self.blocked > 0
    }
}

impl HasContract for BeadsSummary {
    fn contract() -> TypeContract {
        TypeContract::builder("BeadsSummary")
            .description("Summary of beads issues in a workspace")
            .field(
                "open",
                FieldContract::builder("open", "usize")
                    .required()
                    .description("Number of open issues")
                    .default("0")
                    .build(),
            )
            .field(
                "in_progress",
                FieldContract::builder("in_progress", "usize")
                    .required()
                    .description("Number of in-progress issues")
                    .default("0")
                    .build(),
            )
            .field(
                "blocked",
                FieldContract::builder("blocked", "usize")
                    .required()
                    .description("Number of blocked issues")
                    .default("0")
                    .build(),
            )
            .hint(ContextualHint {
                hint_type: HintType::Warning,
                message: "Blocked issues prevent progress - resolve blockers first".to_string(),
                condition: Some("blocked > 0".to_string()),
                related_to: Some("blocked".to_string()),
            })
            .build()
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_transitions() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));

        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Creating));

        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));
    }

    #[test]
    fn test_session_status_allowed_operations() {
        assert_eq!(SessionStatus::Creating.allowed_operations().len(), 0);
        assert!(SessionStatus::Active.allows_operation(Operation::Status));
        assert!(SessionStatus::Active.allows_operation(Operation::Focus));
        assert!(SessionStatus::Paused.allows_operation(Operation::Remove));
        assert!(!SessionStatus::Creating.allows_operation(Operation::Status));
    }

    #[test]
    fn test_session_validate_name_regex() {
        let session = Session {
            id: "id123".to_string(),
            name: "invalid name".to_string(), // contains space
            status: SessionStatus::Creating,
            workspace_path: PathBuf::from("/tmp/test"),
            branch: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: None,
            metadata: serde_json::Value::Null,
        };

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_validate_path_not_absolute() {
        let session = Session {
            id: "id123".to_string(),
            name: "valid-name".to_string(),
            status: SessionStatus::Creating,
            workspace_path: PathBuf::from("relative/path"), // not absolute
            branch: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: None,
            metadata: serde_json::Value::Null,
        };

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_validate_timestamps() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(60);

        let session = Session {
            id: "id123".to_string(),
            name: "valid-name".to_string(),
            status: SessionStatus::Creating,
            workspace_path: PathBuf::from("/tmp/test"),
            branch: None,
            created_at: now,
            updated_at: earlier, // updated before created!
            last_synced: None,
            metadata: serde_json::Value::Null,
        };

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_changes_summary_total() {
        let summary = ChangesSummary {
            modified: 5,
            added: 3,
            deleted: 2,
            renamed: 1,
            untracked: 4,
        };

        assert_eq!(summary.total(), 11);
        assert!(summary.has_changes());
        assert!(summary.has_tracked_changes());
    }

    #[test]
    fn test_changes_summary_no_changes() {
        let summary = ChangesSummary::default();
        assert_eq!(summary.total(), 0);
        assert!(!summary.has_changes());
    }

    #[test]
    fn test_beads_summary_active() {
        let summary = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 1,
            closed: 5,
        };

        assert_eq!(summary.total(), 11);
        assert_eq!(summary.active(), 5);
        assert!(summary.has_blockers());
    }

    #[test]
    fn test_beads_summary_no_blockers() {
        let summary = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 0,
            closed: 5,
        };

        assert!(!summary.has_blockers());
    }

    #[test]
    fn test_file_change_renamed_validation() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: None, // Missing old_path!
        };

        assert!(change.validate().is_err());
    }

    #[test]
    fn test_file_change_renamed_valid() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: Some(PathBuf::from("old/path.txt")),
        };

        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_validation() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 2,
            files: vec![
                FileDiffStat {
                    path: PathBuf::from("file1.txt"),
                    insertions: 5,
                    deletions: 2,
                    status: FileStatus::Modified,
                },
                FileDiffStat {
                    path: PathBuf::from("file2.txt"),
                    insertions: 5,
                    deletions: 3,
                    status: FileStatus::Added,
                },
            ],
        };

        assert!(diff.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_mismatch() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 5, // Mismatch!
            files: vec![FileDiffStat {
                path: PathBuf::from("file1.txt"),
                insertions: 5,
                deletions: 2,
                status: FileStatus::Modified,
            }],
        };

        assert!(diff.validate().is_err());
    }

    #[test]
    fn test_session_contract() {
        let contract = Session::contract();
        assert_eq!(contract.name, "Session");
        assert!(!contract.fields.is_empty());
        assert!(contract.fields.contains_key("name"));
        assert!(contract.fields.contains_key("status"));
    }

    #[test]
    fn test_session_json_schema() {
        let schema = Session::json_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["title"], "Session");
        assert!(schema["properties"].is_object());
    }
}
