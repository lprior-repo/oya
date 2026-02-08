//! Orphan workspace cleanup with periodic task.
//!
//! This module implements periodic cleanup of orphaned zjj workspaces.
//! It uses tokio's Interval for scheduling and invokes zjj clean with
//! age threshold enforcement.

use std::process::Output;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;
use tokio::time::Interval;
use tracing::{debug, info};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during workspace cleanup.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CleanupError {
    /// zjj CLI not found at expected path.
    #[error("zjj CLI not found at {0}")]
    ZjjCommandNotFound(String),

    /// zjj clean command returned non-zero exit code.
    #[error("zjj clean failed with exit code {code}: {stderr}")]
    ZjjCleanExecutionFailed { code: i32, stderr: String },

    /// Failed to parse zjj clean JSON output.
    #[error("failed to parse zjj clean JSON output: {0}")]
    JsonParseFailed(String),

    /// Workspace younger than threshold was marked for deletion.
    #[error("workspace {workspace_id} is {age_secs}s old, below threshold {threshold_secs}s")]
    AgeThresholdNotEnforced {
        workspace_id: String,
        age_secs: i64,
        threshold_secs: i64,
    },

    /// Cleanup attempted to delete workspace with active bead.
    #[error("cleanup attempted to delete active workspace {workspace_id} with bead {bead_id}")]
    ActiveWorkspaceDeleted {
        workspace_id: String,
        bead_id: String,
    },

    /// Failed to write cleanup log entry.
    #[error("failed to write cleanup log entry: {0}")]
    LogWriteFailed(String),

    /// Timer scheduling failed.
    #[error("failed to schedule timer: {0}")]
    TimerScheduleFailed(String),
}

pub type CleanupResult<T> = Result<T, CleanupError>;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for orphan workspace cleanup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupConfig {
    /// Path to zjj CLI binary.
    pub zjj_path: String,
    /// Age threshold in seconds (default: 7200 = 2 hours).
    pub age_threshold_secs: i64,
    /// Periodic interval in seconds (default: 3600 = 1 hour).
    pub interval_secs: u64,
    /// Enable dry-run mode (list without deletion).
    pub dry_run: bool,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            zjj_path: "/home/lewis/.local/bin/zjj".to_string(),
            age_threshold_secs: 7200, // 2 hours
            interval_secs: 3600,      // 1 hour
            dry_run: false,
        }
    }
}

// ============================================================================
// zjj Clean Output Types
// ============================================================================

/// JSON output from `zjj clean --json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZjjCleanOutput {
    /// Number of sessions removed.
    pub removed_count: usize,
    /// Sessions that were cleaned or would be cleaned (dry-run).
    pub sessions: Vec<CleanedSession>,
}

/// A session that was cleaned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanedSession {
    /// Session name.
    pub name: String,
    /// Reason for cleanup.
    pub reason: String,
    /// Workspace path (if available).
    pub workspace_path: Option<String>,
}

// ============================================================================
// Cleanup Operations
// ============================================================================

/// Verify zjj CLI exists at configured path.
///
/// # Errors
///
/// Returns `CleanupError::ZjjCommandNotFound` if zjj binary doesn't exist.
pub fn verify_zjj_exists(config: &CleanupConfig) -> CleanupResult<()> {
    std::path::Path::new(&config.zjj_path)
        .exists()
        .then_some(())
        .ok_or_else(|| CleanupError::ZjjCommandNotFound(config.zjj_path.clone()))
}

/// Run zjj clean with configured options.
///
/// # Errors
///
/// Returns `CleanupError::ZjjCleanExecutionFailed` if command fails.
/// Returns `CleanupError::JsonParseFailed` if JSON parsing fails.
pub async fn run_zjj_clean(config: &CleanupConfig) -> CleanupResult<ZjjCleanOutput> {
    info!(
        "Running zjj clean with age threshold: {}s",
        config.age_threshold_secs
    );

    let mut cmd = Command::new(&config.zjj_path);
    cmd.arg("clean")
        .arg("--force")
        .arg("--age-threshold")
        .arg(config.age_threshold_secs.to_string())
        .arg("--json");

    if config.dry_run {
        cmd.arg("--dry-run");
    }

    let output_result = cmd.output().await;

    let output = output_result
        .map_err(|e| CleanupError::ZjjCommandNotFound(format!("{}: {}", config.zjj_path, e)))?;

    check_zjj_exit_code(&output)?;

    parse_zjj_json(&output.stdout)
}

/// Check zjj clean exit code and return error if non-zero.
///
/// # Errors
///
/// Returns `CleanupError::ZjjCleanExecutionFailed` if exit code is non-zero.
pub fn check_zjj_exit_code(output: &Output) -> CleanupResult<()> {
    output.status.success().then_some(()).ok_or_else(|| {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        CleanupError::ZjjCleanExecutionFailed {
            code: output.status.code().unwrap_or(-1),
            stderr,
        }
    })
}

/// Parse JSON output from zjj clean.
///
/// # Errors
///
/// Returns `CleanupError::JsonParseFailed` if JSON is invalid or missing required fields.
pub fn parse_zjj_json(stdout: &[u8]) -> CleanupResult<ZjjCleanOutput> {
    serde_json::from_slice::<ZjjCleanOutput>(stdout).map_err(|e| {
        CleanupError::JsonParseFailed(format!(
            "Invalid JSON: {}. Raw output: {}",
            e,
            String::from_utf8_lossy(stdout)
        ))
    })
}

/// Log cleanup results.
///
/// # Errors
///
/// Returns `CleanupError::LogWriteFailed` if logging fails (unlikely in practice).
pub fn log_cleanup_results(output: &ZjjCleanOutput) -> CleanupResult<()> {
    info!(
        removed = output.removed_count,
        "Workspace cleanup completed"
    );

    for session in &output.sessions {
        debug!(
            session = %session.name,
            reason = %session.reason,
            workspace = ?session.workspace_path,
            "Cleaned session"
        );
    }

    Ok(())
}

/// Create periodic cleanup timer.
///
/// # Errors
///
/// Returns `CleanupError::TimerScheduleFailed` if interval is invalid.
pub fn create_cleanup_timer(config: &CleanupConfig) -> CleanupResult<Interval> {
    let duration = Duration::from_secs(config.interval_secs);

    // Validate interval is reasonable
    if duration.as_secs() < 60 {
        return Err(CleanupError::TimerScheduleFailed(
            "Interval must be at least 60 seconds".to_string(),
        ));
    }

    Ok(tokio::time::interval(duration))
}

/// Main cleanup task: run zjj clean and log results.
///
/// This function is designed to be called periodically by a timer.
///
/// # Errors
///
/// Returns various `CleanupError` variants if cleanup fails.
pub async fn cleanup_task(config: &CleanupConfig) -> CleanupResult<()> {
    verify_zjj_exists(config)?;

    let output = run_zjj_clean(config).await?;

    log_cleanup_results(&output)?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    // Allow unwrap/panic in test code for test assertions
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_default_config() {
        let config = CleanupConfig::default();
        assert_eq!(config.zjj_path, "/home/lewis/.local/bin/zjj");
        assert_eq!(config.age_threshold_secs, 7200);
        assert_eq!(config.interval_secs, 3600);
        assert!(!config.dry_run);
    }

    #[test]
    fn test_verify_zjj_exists_with_valid_path() {
        let config = CleanupConfig {
            zjj_path: "/usr/bin/true".to_string(),
            ..Default::default()
        };
        assert!(verify_zjj_exists(&config).is_ok());
    }

    #[test]
    fn test_verify_zjj_exists_with_invalid_path() {
        let config = CleanupConfig {
            zjj_path: "/nonexistent/zjj".to_string(),
            ..Default::default()
        };
        let result = verify_zjj_exists(&config);
        assert!(matches!(result, Err(CleanupError::ZjjCommandNotFound(_))));
    }

    #[test]
    fn test_check_zjj_exit_code_success() {
        // Mock successful exit using a real command
        let output = std::process::Command::new("true")
            .output()
            .expect("true command should exist on all Unix systems");

        assert!(check_zjj_exit_code(&output).is_ok());
    }

    #[test]
    fn test_check_zjj_exit_code_failure() {
        // Mock failed exit using a real command
        let output = std::process::Command::new("false")
            .output()
            .expect("false command should exist on all Unix systems");

        let result = check_zjj_exit_code(&output);
        assert!(matches!(
            result,
            Err(CleanupError::ZjjCleanExecutionFailed { .. })
        ));
    }

    #[test]
    fn test_parse_zjj_json_valid() {
        let json = r#"{"removed_count": 2, "sessions": [{"name": "test-session", "reason": "stale", "workspace_path": "/tmp/test"}]}"#;
        let result = parse_zjj_json(json.as_bytes());
        assert!(result.is_ok());
        let output = result.expect("valid JSON should parse successfully");
        assert_eq!(output.removed_count, 2);
        assert_eq!(output.sessions.len(), 1);
        assert_eq!(output.sessions[0].name, "test-session");
    }

    #[test]
    fn test_parse_zjj_json_invalid() {
        let json = b"invalid json";
        let result = parse_zjj_json(json);
        assert!(matches!(result, Err(CleanupError::JsonParseFailed(_))));
    }

    #[test]
    fn test_parse_zjj_json_missing_fields() {
        let json = r#"{"removed_count": 1}"#; // Missing sessions field
        let result = parse_zjj_json(json.as_bytes());
        assert!(matches!(result, Err(CleanupError::JsonParseFailed(_))));
    }

    #[tokio::test]
    async fn test_create_cleanup_timer_valid() {
        let config = CleanupConfig {
            interval_secs: 60,
            ..Default::default()
        };
        assert!(create_cleanup_timer(&config).is_ok());
    }

    #[test]
    fn test_create_cleanup_timer_too_short() {
        let config = CleanupConfig {
            interval_secs: 30,
            ..Default::default()
        };
        let result = create_cleanup_timer(&config);
        assert!(matches!(result, Err(CleanupError::TimerScheduleFailed(_))));
    }

    #[test]
    fn test_age_threshold_not_enforced_error() {
        let error = CleanupError::AgeThresholdNotEnforced {
            workspace_id: "test-workspace".to_string(),
            age_secs: 3600,
            threshold_secs: 7200,
        };
        let error_string = error.to_string();
        assert!(error_string.contains("3600s"));
        assert!(error_string.contains("7200s"));
    }

    #[test]
    fn test_active_workspace_deleted_error() {
        let error = CleanupError::ActiveWorkspaceDeleted {
            workspace_id: "test-workspace".to_string(),
            bead_id: "test-bead".to_string(),
        };
        let error_string = error.to_string();
        assert!(error_string.contains("test-workspace"));
        assert!(error_string.contains("test-bead"));
    }
}
