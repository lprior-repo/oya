//! Isolation schema definitions for workspace and scheduled execution.
//!
//! This module provides type-safe Rust mappings for the `workspace` and `schedule` tables,
//! which enable workspace isolation (zjj integration) and deferred execution (cron-like scheduling).
//!
//! # Tables
//!
//! - `workspace`: Tracks zjj sessions with workspace paths, branches, and status
//! - `schedule`: Manages scheduled tasks with cron expressions and execution tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during isolation operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IsolationError {
    /// Invalid workspace path (empty or invalid format).
    #[error("invalid workspace path: {0}")]
    InvalidWorkspacePath(String),

    /// Invalid branch name (empty or invalid format).
    #[error("invalid branch name: {0}")]
    InvalidBranchName(String),

    /// Invalid cron expression.
    #[error("invalid cron expression: {0}")]
    InvalidCronExpression(String),

    /// Workspace path already exists (unique constraint violation).
    #[error("workspace path already exists: {0}")]
    WorkspaceAlreadyExists(String),

    /// Invalid schedule state (`next_run` must be in the future when scheduled).
    #[error("invalid schedule state: next_run must be in the future when scheduled")]
    InvalidScheduleState,

    /// Invalid status transition for workspace.
    #[error("invalid status transition from {from:?} to {to:?}")]
    InvalidStatusTransition {
        from: WorkspaceStatus,
        to: WorkspaceStatus,
    },

    /// Cannot transition completed workspace back to active.
    #[error("cannot activate completed workspace")]
    CannotActivateCompleted,

    /// Cannot delete active workspace (must complete or pause first).
    #[error("cannot delete active workspace: {0}")]
    CannotDeleteActive(String),
}

pub type Result<T> = std::result::Result<T, IsolationError>;

// ============================================================================
// Workspace Status (Type State Pattern)
// ============================================================================

/// Status of a workspace session.
///
/// This enum uses a state machine pattern to enforce valid transitions:
/// - `Creating` → `Active` (initial creation)
/// - `Active` ↔ `Paused` (work interruption)
/// - `Active` → `Completed` (work finished)
/// - `Active` → `Failed` (error during creation)
/// - `Paused` → `Completed` (finish paused work)
/// - `Failed` → `Creating` (retry failed creation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    /// Workspace is being created (transient state).
    Creating,

    /// Workspace is active and ready for work.
    Active,

    /// Workspace is paused (work interrupted, can be resumed).
    Paused,

    /// Workspace is completed (work finished, merged to main).
    Completed,

    /// Workspace failed (error during creation or operation).
    Failed,
}

impl fmt::Display for WorkspaceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl WorkspaceStatus {
    /// Check if this status allows transitioning to another status.
    #[must_use]
    pub const fn can_transition_to(&self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Creating, Self::Active | Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed | Self::Failed)
                | (Self::Paused, Self::Active | Self::Completed)
                | (Self::Failed, Self::Creating)
        )
    }

    /// Check if workspace is in a terminal state (cannot be modified).
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    /// Check if workspace can be activated (resumed from paused or creating).
    #[must_use]
    pub const fn can_activate(&self) -> bool {
        matches!(self, Self::Creating | Self::Paused)
    }
}

// ============================================================================
// Workspace Types
// ============================================================================

/// Configuration for workspace creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceConfig {
    name: String,
    branch: String,
}

impl WorkspaceConfig {
    /// Create a new workspace configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if name or branch is empty.
    pub fn new(name: String, branch: String) -> Result<Self> {
        if name.trim().is_empty() {
            return Err(IsolationError::InvalidBranchName(name));
        }
        if branch.trim().is_empty() {
            return Err(IsolationError::InvalidBranchName(branch));
        }

        Ok(Self { name, branch })
    }

    /// Get the workspace name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the branch name.
    #[must_use]
    pub fn branch(&self) -> &str {
        &self.branch
    }
}

/// Workspace path newtype for type safety and validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspacePath(String);

impl WorkspacePath {
    /// Create a new workspace path.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is empty or contains invalid characters.
    pub fn new(path: String) -> Result<Self> {
        if path.trim().is_empty() {
            return Err(IsolationError::InvalidWorkspacePath(path));
        }

        // Basic path validation (no null bytes, no control characters)
        if path.contains('\0') || path.chars().any(char::is_control) {
            return Err(IsolationError::InvalidWorkspacePath(
                "contains invalid characters".to_string(),
            ));
        }

        Ok(Self(path))
    }

    /// Get the inner path string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for WorkspacePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Workspace table row representing a zjj session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub workspace_path: String,
    pub branch: String,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_synced: Option<DateTime<Utc>>,
}

impl Workspace {
    /// Create a new workspace with initial status.
    ///
    /// # Errors
    ///
    /// Returns an error if path or branch validation fails.
    pub fn create(workspace_path: &str, branch: &str) -> Result<Self> {
        let path = WorkspacePath::new(workspace_path.to_string())?;
        let now = Utc::now();

        Ok(Self {
            id: ulid::Ulid::new().to_string(),
            workspace_path: path.into_inner(),
            branch: branch.to_string(),
            status: WorkspaceStatus::Creating,
            created_at: now,
            updated_at: now,
            last_synced: None,
        })
    }

    /// Transition workspace to a new status.
    ///
    /// # Errors
    ///
    /// Returns an error if the transition is invalid.
    pub fn transition_status(&mut self, new_status: WorkspaceStatus) -> Result<()> {
        if !self.status.can_transition_to(new_status) {
            return Err(IsolationError::InvalidStatusTransition {
                from: self.status,
                to: new_status,
            });
        }

        // Special check: cannot activate completed workspace
        if self.status == WorkspaceStatus::Completed && new_status == WorkspaceStatus::Active {
            return Err(IsolationError::CannotActivateCompleted);
        }

        self.status = new_status;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if workspace can be deleted (not active).
    #[must_use]
    pub fn can_delete(&self) -> bool {
        self.status != WorkspaceStatus::Active
    }

    /// Update the last synced timestamp.
    pub fn update_synced(&mut self) {
        self.last_synced = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if workspace needs sync (synced more than 5 minutes ago or never).
    #[must_use]
    pub fn needs_sync(&self, threshold_secs: i64) -> bool {
        match self.last_synced {
            Some(last) => {
                let elapsed = (Utc::now() - last).num_seconds();
                elapsed >= threshold_secs
            }
            None => true,
        }
    }
}

// ============================================================================
// Schedule Types
// ============================================================================

/// Configuration for a scheduled task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleConfig {
    cron_expr: String,
}

impl ScheduleConfig {
    /// Create a new schedule configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the cron expression is empty or invalid format.
    pub fn new(cron_expr: String) -> Result<Self> {
        if cron_expr.trim().is_empty() {
            return Err(IsolationError::InvalidCronExpression(cron_expr));
        }

        // Basic cron format validation (5 or 6 fields separated by spaces)
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        if !(5..=6).contains(&parts.len()) {
            return Err(IsolationError::InvalidCronExpression(
                "must be 5 or 6 fields".to_string(),
            ));
        }

        Ok(Self { cron_expr })
    }

    /// Get the cron expression.
    #[must_use]
    pub fn cron_expr(&self) -> &str {
        &self.cron_expr
    }
}

/// Schedule table row for deferred execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Schedule {
    pub id: String,
    pub name: String,
    pub cron_expr: String,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub enabled: bool,
}

impl Schedule {
    /// Create a new schedule.
    ///
    /// # Errors
    ///
    /// Returns an error if cron expression is invalid.
    pub fn create(
        name: String,
        cron_expr: String,
        initial_next_run: DateTime<Utc>,
    ) -> Result<Self> {
        if name.trim().is_empty() {
            return Err(IsolationError::InvalidCronExpression(
                "name cannot be empty".to_string(),
            ));
        }

        let _config = ScheduleConfig::new(cron_expr.clone())?;
        let now = Utc::now();

        Ok(Self {
            id: ulid::Ulid::new().to_string(),
            name,
            cron_expr,
            next_run: initial_next_run,
            last_run: None,
            created_at: now,
            updated_at: now,
            enabled: true,
        })
    }

    /// Update schedule after execution.
    pub fn update_execution(&mut self, next_run: DateTime<Utc>) {
        self.last_run = Some(Utc::now());
        self.next_run = next_run;
        self.updated_at = Utc::now();
    }

    /// Check if schedule is due to run.
    #[must_use]
    pub fn is_due(&self) -> bool {
        self.enabled && self.next_run <= Utc::now()
    }

    /// Enable the schedule.
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
    }

    /// Disable the schedule.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }

    /// Check if schedule is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// ============================================================================
// Query Builders (Pure Functions)
// ============================================================================

/// Build a `SurrealDB` query to create a workspace.
#[must_use]
pub fn build_create_workspace_query(workspace_path: &str, branch: &str) -> String {
    format!(
        "CREATE workspace CONTENT {{
            workspace_path: '{workspace_path}',
            branch: '{branch}',
            status: 'creating',
            created_at: time::now(),
            updated_at: time::now()
        }}"
    )
}

/// Build a `SurrealDB` query to update workspace status.
#[must_use]
pub fn build_update_status_query(id: &str, status: &str) -> String {
    format!("UPDATE workspace:{id} SET status = '{status}', updated_at = time::now() RETURN AFTER")
}

/// Build a `SurrealDB` query to create a schedule.
#[must_use]
pub fn build_create_schedule_query(name: &str, cron_expr: &str, next_run: &str) -> String {
    format!(
        "CREATE schedule CONTENT {{
            name: '{name}',
            cron_expr: '{cron_expr}',
            next_run: '{next_run}',
            created_at: time::now(),
            updated_at: time::now(),
            enabled: true
        }}"
    )
}

/// Build a `SurrealDB` query to find due schedules.
#[must_use]
pub const fn build_find_due_schedules_query() -> &'static str {
    "SELECT * FROM schedule WHERE enabled = true AND next_run <= time::now()"
}

/// Build a `SurrealDB` query to update schedule after execution.
#[must_use]
pub fn build_update_schedule_execution_query(id: &str, next_run: &str) -> String {
    format!(
        "UPDATE schedule:{id} SET \
         last_run = time::now(), \
         next_run = '{next_run}', \
         updated_at = time::now() \
         RETURN AFTER"
    )
}

/// Build a `SurrealDB` query to check workspace path uniqueness.
#[must_use]
pub fn build_check_workspace_exists_query(path: &str) -> String {
    format!("SELECT id FROM workspace WHERE workspace_path = '{path}' LIMIT 1")
}

/// Build a `SurrealDB` query to find active workspaces.
#[must_use]
pub const fn build_find_active_workspaces_query() -> &'static str {
    "SELECT * FROM workspace WHERE status = 'active'"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_status_valid_transitions() {
        assert!(WorkspaceStatus::Creating.can_transition_to(WorkspaceStatus::Active));
        assert!(WorkspaceStatus::Creating.can_transition_to(WorkspaceStatus::Failed));
        assert!(WorkspaceStatus::Active.can_transition_to(WorkspaceStatus::Paused));
        assert!(WorkspaceStatus::Active.can_transition_to(WorkspaceStatus::Completed));
        assert!(WorkspaceStatus::Paused.can_transition_to(WorkspaceStatus::Active));
    }

    #[test]
    fn test_workspace_status_invalid_transitions() {
        assert!(!WorkspaceStatus::Active.can_transition_to(WorkspaceStatus::Creating));
        assert!(!WorkspaceStatus::Completed.can_transition_to(WorkspaceStatus::Active));
        assert!(!WorkspaceStatus::Failed.can_transition_to(WorkspaceStatus::Paused));
    }

    #[test]
    fn test_workspace_creation() {
        let result = Workspace::create("/path/to/workspace", "feature-branch");
        assert!(matches!(
            result,
            Ok(ref w) if w.status == WorkspaceStatus::Creating
        ));
    }

    #[test]
    fn test_workspace_invalid_path() {
        let workspace = Workspace::create("", "branch");
        assert!(workspace.is_err());
    }

    #[test]
    fn test_workspace_status_transition() {
        let result = Workspace::create("/path", "branch").and_then(|mut workspace| {
            workspace
                .transition_status(WorkspaceStatus::Active)
                .map(|()| workspace)
        });

        assert!(matches!(
            result,
            Ok(ref w) if w.status == WorkspaceStatus::Active
        ));
    }

    #[test]
    fn test_workspace_invalid_transition() {
        let result = Workspace::create("/path", "branch")
            .and_then(|mut workspace| workspace.transition_status(WorkspaceStatus::Completed));

        assert!(result.is_err());
    }

    #[test]
    fn test_schedule_creation() {
        let next_run = Utc::now() + chrono::Duration::hours(1);
        let schedule = Schedule::create(
            "test-schedule".to_string(),
            "0 * * * *".to_string(),
            next_run,
        );
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_schedule_invalid_cron() {
        let next_run = Utc::now() + chrono::Duration::hours(1);
        let schedule = Schedule::create("test".to_string(), "invalid".to_string(), next_run);
        assert!(schedule.is_err());
    }

    #[test]
    fn test_workspace_needs_sync() {
        let result = Workspace::create("/path", "branch").map(|mut workspace| {
            // No last synced - should need sync
            let needs_before = workspace.needs_sync(300);
            workspace.update_synced();
            let needs_after = workspace.needs_sync(300);
            (needs_before, needs_after)
        });

        assert!(matches!(result, Ok((true, false))));
    }

    #[test]
    fn test_schedule_is_due() {
        let past = Utc::now() - chrono::Duration::minutes(5);
        let future = Utc::now() + chrono::Duration::hours(1);

        let result =
            Schedule::create("test".to_string(), "0 * * * *".to_string(), past).map(|mut schedule| {
                let is_due_past = schedule.is_due();
                schedule.next_run = future;
                let is_due_future = schedule.is_due();
                schedule.disable();
                let is_due_disabled = schedule.is_due();
                (is_due_past, is_due_future, is_due_disabled)
            });

        assert!(matches!(result, Ok((true, false, false))));
    }
}
