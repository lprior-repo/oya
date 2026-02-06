//! Audit module - Track all task state changes and decisions.
//!
//! Provides full audit trail for compliance and debugging.
//! Uses JSONL format for git-friendly diffs.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    process::{append_text_file, create_dir_all, read_text_file},
};

/// Type of audit event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    TaskCreated,
    TaskUpdated,
    StageStarted,
    StagePassed,
    StageFailed,
    StageRetried,
    TaskApproved,
    TaskRejected,
    DeploymentStarted,
    DeploymentCompleted,
    DeploymentRolledBack,
}

impl AuditEventType {
    /// Convert to string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TaskCreated => "task_created",
            Self::TaskUpdated => "task_updated",
            Self::StageStarted => "stage_started",
            Self::StagePassed => "stage_passed",
            Self::StageFailed => "stage_failed",
            Self::StageRetried => "stage_retried",
            Self::TaskApproved => "task_approved",
            Self::TaskRejected => "task_rejected",
            Self::DeploymentStarted => "deployment_started",
            Self::DeploymentCompleted => "deployment_completed",
            Self::DeploymentRolledBack => "deployment_rolled_back",
        }
    }

    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "task_created" => Some(Self::TaskCreated),
            "task_updated" => Some(Self::TaskUpdated),
            "stage_started" => Some(Self::StageStarted),
            "stage_passed" => Some(Self::StagePassed),
            "stage_failed" => Some(Self::StageFailed),
            "stage_retried" => Some(Self::StageRetried),
            "task_approved" => Some(Self::TaskApproved),
            "task_rejected" => Some(Self::TaskRejected),
            "deployment_started" => Some(Self::DeploymentStarted),
            "deployment_completed" => Some(Self::DeploymentCompleted),
            "deployment_rolled_back" => Some(Self::DeploymentRolledBack),
            _ => None,
        }
    }
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metadata key-value pair for audit entries.
pub type Metadata = Vec<(String, String)>;

/// Single audit entry with full context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub event_type: AuditEventType,
    pub task_slug: String,
    pub actor: String,
    pub details: String,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Audit log containing all entries for a task.
#[derive(Debug, Clone)]
pub struct AuditLog {
    pub task_slug: String,
    pub entries: Vec<AuditEntry>,
}

/// Get current timestamp in ISO format.
fn get_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Get current actor from environment or default.
fn get_actor() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("OYA_ACTOR"))
        .unwrap_or_else(|_| "oya".to_string())
}

/// Create a new audit entry.
#[must_use]
pub fn create_entry(
    event_type: AuditEventType,
    task_slug: &str,
    details: &str,
    metadata: &[(impl AsRef<str>, impl AsRef<str>)],
) -> AuditEntry {
    let metadata_map: std::collections::HashMap<String, String> = metadata
        .iter()
        .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
        .collect();

    AuditEntry {
        timestamp: get_timestamp(),
        event_type,
        task_slug: task_slug.to_string(),
        actor: get_actor(),
        details: details.to_string(),
        metadata: metadata_map,
    }
}

/// Get audit log file path.
#[must_use]
pub fn audit_file_path(repo_root: &Path, task_slug: &str) -> PathBuf {
    repo_root
        .join(".oya")
        .join("audit")
        .join(format!("{task_slug}.jsonl"))
}

/// Append audit entry to log file (JSONL format).
pub fn log_event(
    repo_root: &Path,
    event_type: AuditEventType,
    task_slug: &str,
    details: &str,
    metadata: &[(&str, &str)],
) -> Result<()> {
    let entry = create_entry(event_type, task_slug, details, metadata);
    let file_path = audit_file_path(repo_root, task_slug);
    let audit_dir = repo_root.join(".oya").join("audit");

    create_dir_all(&audit_dir)?;

    let json_line = serde_json::to_string(&entry).map_err(|e| Error::AuditWriteFailed {
        reason: e.to_string(),
    })?;

    append_text_file(&file_path, &format!("{json_line}\n")).map_err(|_| Error::AuditWriteFailed {
        reason: "could not write audit entry".to_string(),
    })
}

/// Read all audit entries for a task.
pub fn read_audit_log(repo_root: &Path, task_slug: &str) -> Result<AuditLog> {
    let file_path = audit_file_path(repo_root, task_slug);

    if !file_path.exists() {
        return Ok(AuditLog {
            task_slug: task_slug.to_string(),
            entries: Vec::new(),
        });
    }

    let content = read_text_file(&file_path)?;

    let entries: Vec<AuditEntry> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    Ok(AuditLog {
        task_slug: task_slug.to_string(),
        entries,
    })
}

/// Get last N audit entries for a task.
pub fn get_recent_entries(
    repo_root: &Path,
    task_slug: &str,
    count: usize,
) -> Result<Vec<AuditEntry>> {
    let log = read_audit_log(repo_root, task_slug)?;

    let recent: Vec<_> = log
        .entries
        .into_iter()
        .rev()
        .take(count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(recent)
}

/// Filter audit entries by event type.
#[must_use]
pub fn filter_by_type(log: &AuditLog, event_type: AuditEventType) -> Vec<&AuditEntry> {
    log.entries
        .iter()
        .filter(|entry| entry.event_type == event_type)
        .collect()
}

/// Get all stage events for a task.
#[must_use]
pub fn get_stage_history(log: &AuditLog) -> Vec<&AuditEntry> {
    log.entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.event_type,
                AuditEventType::StageStarted
                    | AuditEventType::StagePassed
                    | AuditEventType::StageFailed
                    | AuditEventType::StageRetried
            )
        })
        .collect()
}

/// Get deployment events for a task.
#[must_use]
pub fn get_deployment_history(log: &AuditLog) -> Vec<&AuditEntry> {
    log.entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.event_type,
                AuditEventType::DeploymentStarted
                    | AuditEventType::DeploymentCompleted
                    | AuditEventType::DeploymentRolledBack
            )
        })
        .collect()
}

// ============================================================================
// CONVENIENCE LOGGING FUNCTIONS
// ============================================================================

/// Log task creation.
pub fn log_task_created(
    repo_root: &Path,
    task_slug: &str,
    language: &str,
    branch: &str,
) -> Result<()> {
    log_event(
        repo_root,
        AuditEventType::TaskCreated,
        task_slug,
        "Task created",
        &[("language", language), ("branch", branch)],
    )
}

/// Log stage start.
pub fn log_stage_started(
    repo_root: &Path,
    task_slug: &str,
    stage_name: &str,
    attempt: i32,
) -> Result<()> {
    log_event(
        repo_root,
        AuditEventType::StageStarted,
        task_slug,
        &format!("Stage started: {stage_name}"),
        &[("stage", stage_name), ("attempt", &attempt.to_string())],
    )
}

/// Log stage pass.
pub fn log_stage_passed(
    repo_root: &Path,
    task_slug: &str,
    stage_name: &str,
    duration_ms: i64,
) -> Result<()> {
    log_event(
        repo_root,
        AuditEventType::StagePassed,
        task_slug,
        &format!("Stage passed: {stage_name}"),
        &[
            ("stage", stage_name),
            ("duration_ms", &duration_ms.to_string()),
        ],
    )
}

/// Log stage failure.
pub fn log_stage_failed(
    repo_root: &Path,
    task_slug: &str,
    stage_name: &str,
    error: &str,
) -> Result<()> {
    log_event(
        repo_root,
        AuditEventType::StageFailed,
        task_slug,
        &format!("Stage failed: {stage_name}"),
        &[("stage", stage_name), ("error", error)],
    )
}

/// Log task approval.
pub fn log_task_approved(repo_root: &Path, task_slug: &str, strategy: &str) -> Result<()> {
    log_event(
        repo_root,
        AuditEventType::TaskApproved,
        task_slug,
        "Task approved for deployment",
        &[("strategy", strategy)],
    )
}

/// Log deployment start.
pub fn log_deployment_started(
    repo_root: &Path,
    task_slug: &str,
    rollout_percentage: i32,
) -> Result<()> {
    log_event(
        repo_root,
        AuditEventType::DeploymentStarted,
        task_slug,
        &format!("Deployment started at {rollout_percentage}%"),
        &[("rollout_percentage", &rollout_percentage.to_string())],
    )
}

/// Log deployment completion.
pub fn log_deployment_completed(repo_root: &Path, task_slug: &str) -> Result<()> {
    log_event(
        repo_root,
        AuditEventType::DeploymentCompleted,
        task_slug,
        "Deployment completed successfully",
        &[],
    )
}

/// Log deployment rollback.
pub fn log_deployment_rolled_back(repo_root: &Path, task_slug: &str, reason: &str) -> Result<()> {
    log_event(
        repo_root,
        AuditEventType::DeploymentRolledBack,
        task_slug,
        &format!("Deployment rolled back: {reason}"),
        &[("reason", reason)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_roundtrip() {
        let event = AuditEventType::TaskCreated;
        let s = event.as_str();
        let parsed = AuditEventType::parse(s);
        assert_eq!(parsed, Some(event));
    }

    #[test]
    fn test_create_entry() {
        let entry = create_entry(
            AuditEventType::TaskCreated,
            "test-task",
            "Created task",
            &[("language", "rust")],
        );

        assert_eq!(entry.task_slug, "test-task");
        assert_eq!(entry.event_type, AuditEventType::TaskCreated);
        assert!(entry.metadata.contains_key("language"));
    }
}
