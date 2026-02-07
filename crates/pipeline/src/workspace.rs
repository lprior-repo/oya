#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Workspace isolation for bead execution.
//!
//! Creates a zjj workspace per bead, returns the workspace path for execution,
//! and guarantees cleanup through RAII.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use itertools::Itertools;
use serde::Deserialize;
use tracing::{info, warn};

use crate::error::{Error, Result};
use crate::process::{CommandResult, run_command};
use crate::repo;

/// Runs workspace-related commands (zjj) so logic can be tested without
/// spawning real workspaces.
pub trait WorkspaceCommandRunner: Send + Sync {
    fn run(&self, cmd: &str, args: &[&str], cwd: &Path) -> Result<CommandResult>;
}

/// System runner that shells out to the real CLI.
#[derive(Clone, Default)]
pub struct SystemWorkspaceCommandRunner;

impl WorkspaceCommandRunner for SystemWorkspaceCommandRunner {
    fn run(&self, cmd: &str, args: &[&str], cwd: &Path) -> Result<CommandResult> {
        run_command(cmd, args, cwd)
    }
}

/// Manages workspace lifecycle for bead execution.
#[derive(Clone)]
pub struct WorkspaceManager {
    repo_root: PathBuf,
    runner: Arc<dyn WorkspaceCommandRunner>,
}

impl WorkspaceManager {
    /// Create a manager rooted at the current repository (auto-detected).
    pub fn for_current_repo() -> Result<Self> {
        repo::detect_repo_root().map(Self::new)
    }

    /// Create a manager with the default system runner.
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            runner: Arc::new(SystemWorkspaceCommandRunner),
        }
    }

    /// Create a manager with a custom runner (useful for testing).
    pub fn with_runner(repo_root: PathBuf, runner: Arc<dyn WorkspaceCommandRunner>) -> Self {
        Self { repo_root, runner }
    }

    /// Create a workspace for the given bead and return a guard that cleans up.
    pub fn create_for_bead(&self, bead_id: &str) -> Result<WorkspaceGuard> {
        let workspace_name = sanitize_workspace_name(bead_id);

        let add_args = ["add", workspace_name.as_str(), "--no-zellij"]; // non-interactive
        let add_result = self.runner.run("zjj", &add_args, &self.repo_root)?;
        add_result.check_success()?;

        let status_args = ["status", workspace_name.as_str(), "--json"];
        let status_result = self.runner.run("zjj", &status_args, &self.repo_root)?;
        status_result.check_success()?;

        let workspace_path = parse_workspace_path(&status_result.stdout, &workspace_name)?;

        info!(
            bead_id = bead_id,
            workspace = workspace_name,
            path = %workspace_path.display(),
            "Workspace created for bead",
        );

        Ok(WorkspaceGuard::new(
            workspace_name,
            workspace_path,
            self.repo_root.clone(),
            Arc::clone(&self.runner),
        ))
    }

    /// Create a workspace using jj workspace add directly (not via zjj wrapper).
    ///
    /// This creates an isolated jj workspace with a UUID-based name for uniqueness.
    /// The returned WorkspaceGuard ensures cleanup via RAII (jj workspace forget on drop).
    ///
    /// # Arguments
    ///
    /// * `uuid` - Unique identifier for the workspace (typically a ULID)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - jj workspace add command fails
    /// - workspace directory cannot be determined
    /// - repo_root is not a valid jj repository
    pub fn create_workspace(&self, uuid: &str) -> Result<WorkspaceGuard> {
        // Validate UUID is not empty
        let workspace_id = if uuid.trim().is_empty() {
            return Err(Error::InvalidRecord {
                reason: "workspace UUID cannot be empty".to_string(),
            });
        } else {
            uuid.to_string()
        };

        // Run: jj workspace add <uuid>
        let add_args = ["workspace", "add", &workspace_id];
        let add_result = self.runner.run("jj", &add_args, &self.repo_root)?;
        add_result.check_success()?;

        // Determine workspace path: <repo_root>/.jj/workspaces/<uuid>
        let workspace_path = self
            .repo_root
            .join(".jj")
            .join("workspaces")
            .join(&workspace_id);

        // Verify workspace directory exists
        if !workspace_path.exists() {
            return Err(Error::InvalidRecord {
                reason: format!(
                    "workspace directory not found after creation: {}",
                    workspace_path.display()
                ),
            });
        }

        info!(
            workspace_uuid = workspace_id,
            path = %workspace_path.display(),
            "Workspace created with jj workspace add",
        );

        Ok(WorkspaceGuard::new(
            workspace_id,
            workspace_path,
            self.repo_root.clone(),
            Arc::clone(&self.runner),
        ))
    }

    /// Execute a bead inside an isolated workspace. The executor closure receives
    /// the workspace path. Cleanup is attempted even if execution fails.
    pub fn execute_with_workspace<T, F>(&self, bead_id: &str, executor: F) -> Result<T>
    where
        F: FnOnce(&Path) -> Result<T>,
    {
        let mut guard = self.create_for_bead(bead_id)?;

        let exec_result = executor(guard.path());
        let cleanup_result = guard.cleanup();

        match (exec_result, cleanup_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(cleanup_err)) => Err(cleanup_err),
            (Err(exec_err), Ok(())) => Err(exec_err),
            (Err(exec_err), Err(cleanup_err)) => {
                warn!(
                    bead_id = bead_id,
                    workspace = guard.name(),
                    error = %cleanup_err,
                    "Workspace cleanup failed after execution error",
                );
                Err(exec_err)
            }
        }
    }

    /// List all jj workspaces in the repository.
    ///
    /// Uses `jj workspace list` to get all workspace names and their metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - jj workspace list command fails
    /// - output cannot be parsed
    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceInfo>> {
        let list_args = ["workspace", "list"];
        let list_result = self.runner.run("jj", &list_args, &self.repo_root)?;
        list_result.check_success()?;

        parse_workspace_list(&list_result.stdout)
    }

    /// Cleanup orphaned workspaces.
    ///
    /// A workspace is considered orphaned if:
    /// - It is older than the given age threshold
    /// - It is not in the set of active workspace names
    ///
    /// # Arguments
    ///
    /// * `age_threshold_hours` - Minimum age in hours for a workspace to be considered orphaned
    /// * `active_workspace_names` - Set of workspace names that are currently active (associated with running beads)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - listing workspaces fails
    /// - forgetting a workspace fails
    ///
    /// # Returns
    ///
    /// Returns the number of workspaces cleaned up.
    pub fn cleanup_orphaned_workspaces(
        &self,
        age_threshold_hours: u64,
        active_workspace_names: &HashSet<String>,
    ) -> Result<usize> {
        let workspaces = self.list_workspaces()?;
        let threshold_secs = age_threshold_hours * 3600;

        let mut cleaned_count = 0usize;

        for workspace in workspaces {
            // Skip if workspace is in active set
            if active_workspace_names.contains(&workspace.name) {
                continue;
            }

            // Skip if workspace is too young
            if workspace.age_seconds < threshold_secs {
                continue;
            }

            // Cleanup this orphaned workspace
            match self.forget_workspace(&workspace.name) {
                Ok(()) => {
                    info!(
                        workspace = %workspace.name,
                        age_hours = workspace.age_seconds / 3600,
                        "Cleaned up orphaned workspace",
                    );
                    cleaned_count += 1;
                }
                Err(e) => {
                    warn!(
                        workspace = %workspace.name,
                        error = %e,
                        "Failed to cleanup orphaned workspace",
                    );
                    // Continue with other workspaces even if one fails
                }
            }
        }

        Ok(cleaned_count)
    }

    /// Forget (delete) a workspace by name.
    ///
    /// Uses `jj workspace forget` to remove the workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the jj command fails.
    fn forget_workspace(&self, workspace_name: &str) -> Result<()> {
        let forget_args = ["workspace", "forget", workspace_name];
        let forget_result = self.runner.run("jj", &forget_args, &self.repo_root)?;
        forget_result.check_success()
    }
}

/// RAII guard that cleans up a workspace on drop.
pub struct WorkspaceGuard {
    name: String,
    path: PathBuf,
    repo_root: PathBuf,
    runner: Arc<dyn WorkspaceCommandRunner>,
    cleaned: bool,
}

impl WorkspaceGuard {
    fn new(
        name: String,
        path: PathBuf,
        repo_root: PathBuf,
        runner: Arc<dyn WorkspaceCommandRunner>,
    ) -> Self {
        Self {
            name,
            path,
            repo_root,
            runner,
            cleaned: false,
        }
    }

    /// Workspace path to execute within.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Workspace name used by zjj.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Explicitly clean up the workspace (idempotent).
    pub fn cleanup(&mut self) -> Result<()> {
        if self.cleaned {
            return Ok(());
        }

        let forget_args = ["workspace", "forget", self.name.as_str()];
        let forget_result = self.runner.run("jj", &forget_args, &self.repo_root)?;
        forget_result.check_success()?;

        self.cleaned = true;
        Ok(())
    }
}

impl Drop for WorkspaceGuard {
    fn drop(&mut self) {
        if self.cleaned {
            return;
        }

        if let Err(error) = self.cleanup() {
            warn!(
                workspace = %self.name,
                path = %self.path.display(),
                error = %error,
                "Failed to clean up workspace on drop",
            );
        }
    }
}

fn sanitize_workspace_name(bead_id: &str) -> String {
    let sanitized: String = bead_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('-');
    let non_empty = if trimmed.is_empty() {
        "workspace"
    } else {
        trimmed
    };

    non_empty.chars().take(64).collect()
}

fn parse_workspace_path(status_json: &str, workspace_name: &str) -> Result<PathBuf> {
    let parsed: ZjjStatusEnvelope =
        serde_json::from_str(status_json).map_err(|err| Error::json_parse_failed(err))?;

    let maybe_session = parsed
        .sessions
        .into_iter()
        .find(|session| session.name == workspace_name);

    match maybe_session {
        Some(session) => Ok(session.workspace_path),
        None => Err(Error::InvalidRecord {
            reason: format!("workspace '{workspace_name}' not found in zjj status output"),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct ZjjStatusEnvelope {
    #[serde(default)]
    sessions: Vec<ZjjSession>,
}

#[derive(Debug, Deserialize)]
struct ZjjSession {
    name: String,
    workspace_path: PathBuf,
}

/// Information about a jj workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceInfo {
    /// Workspace name.
    pub name: String,
    /// Age of the workspace in seconds (since creation).
    pub age_seconds: u64,
}

/// Parse `jj workspace list` output.
///
/// Expected format:
/// ```text
/// workspace-1  example@example.com 2024-01-01 12:00:00
/// workspace-2  example@example.com 2024-01-02 13:00:00
/// ```
///
/// # Errors
///
/// Returns an error if the output cannot be parsed.
fn parse_workspace_list(output: &str) -> Result<Vec<WorkspaceInfo>> {
    let mut workspaces = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse format: "name email timestamp"
        let parts: Vec<&str> = line.split_whitespace().collect_vec();
        if parts.len() < 3 {
            return Err(Error::InvalidRecord {
                reason: format!("invalid workspace list line: {line}"),
            });
        }

        let name = parts[0].to_string();

        // Parse timestamp (format: YYYY-MM-DD HH:MM:SS)
        // We need to reconstruct the timestamp from parts[2] and parts[3]
        let timestamp_str = if parts.len() >= 4 {
            format!("{} {}", parts[2], parts[3])
        } else {
            parts[2].to_string()
        };

        let age_seconds = parse_age_from_timestamp(&timestamp_str)?;

        workspaces.push(WorkspaceInfo { name, age_seconds });
    }

    Ok(workspaces)
}

/// Parse a timestamp string and calculate age in seconds.
///
/// Expected format: "YYYY-MM-DD HH:MM:SS"
fn parse_age_from_timestamp(timestamp: &str) -> Result<u64> {
    use chrono::{DateTime, Utc};

    let dt: DateTime<Utc> = timestamp.parse().map_err(|err| Error::InvalidRecord {
        reason: format!("invalid timestamp '{timestamp}': {err}"),
    })?;

    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < 0 {
        return Err(Error::InvalidRecord {
            reason: format!("timestamp '{timestamp}' is in the future"),
        });
    }

    Ok(duration.num_seconds() as u64)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    type CallLog = Vec<(String, Vec<String>, PathBuf)>;

    #[derive(Clone, Default)]
    struct StubRunner {
        calls: Arc<Mutex<CallLog>>,
        responses: Arc<Mutex<Vec<Result<CommandResult>>>>,
    }

    impl StubRunner {
        fn push_response(&self, response: Result<CommandResult>) {
            if let Ok(mut guard) = self.responses.lock() {
                guard.push(response);
            }
        }

        fn recorded_calls(&self) -> Vec<(String, Vec<String>, PathBuf)> {
            self.calls
                .lock()
                .map(|calls| calls.clone())
                .unwrap_or_default()
        }
    }

    impl WorkspaceCommandRunner for StubRunner {
        fn run(&self, cmd: &str, args: &[&str], cwd: &Path) -> Result<CommandResult> {
            if let Ok(mut calls) = self.calls.lock() {
                let arg_vec = args.iter().map(|a| a.to_string()).collect_vec();
                calls.push((cmd.to_string(), arg_vec, cwd.to_path_buf()));
            }

            let mut responses = self.responses.lock().map_err(|err| {
                Error::Io(std::io::Error::other(format!("mutex poisoned: {err}")))
            })?;

            responses.pop().ok_or_else(|| Error::InvalidRecord {
                reason: "no stubbed response".to_string(),
            })?
        }
    }

    fn success(stdout: &str) -> Result<CommandResult> {
        Ok(CommandResult {
            stdout: stdout.to_string(),
            stderr: String::new(),
            exit_code: 0,
        })
    }

    fn failure(stderr: &str) -> Result<CommandResult> {
        Ok(CommandResult {
            stdout: String::new(),
            stderr: stderr.to_string(),
            exit_code: 1,
        })
    }

    #[test]
    fn creates_workspace_and_cleans_up() -> Result<()> {
        let runner = StubRunner::default();
        let status_json =
            r#"{"sessions":[{"name":"bead-123","workspace_path":"/tmp/workspace/bead-123"}]}"#;

        runner.push_response(success("")); // cleanup remove
        runner.push_response(success(status_json)); // status
        runner.push_response(success("")); // add

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        let mut guard =
            manager
                .create_for_bead("bead-123")
                .map_err(|err| Error::InvalidRecord {
                    reason: format!("workspace created: {err}"),
                })?;
        assert_eq!(guard.path(), Path::new("/tmp/workspace/bead-123"));
        assert_eq!(guard.name(), "bead-123");

        guard.cleanup()?;

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "zjj");
        assert_eq!(calls[1].0, "zjj");
        assert_eq!(calls[2].0, "zjj");
        Ok(())
    }

    #[test]
    fn execute_with_workspace_stops_on_workspace_failure() {
        let runner = StubRunner::default();
        runner.push_response(failure("add failed"));

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        let mut executed = false;
        let result = manager.execute_with_workspace("bead-err", |_| {
            executed = true;
            Ok(())
        });

        assert!(result.is_err());
        assert!(!executed);
    }

    #[test]
    fn sanitize_workspace_name_handles_invalid_characters() {
        let sanitized = sanitize_workspace_name("bead id with spaces!");
        assert_eq!(sanitized, "bead-id-with-spaces");

        let empty = sanitize_workspace_name("***");
        assert_eq!(empty, "workspace");
    }

    #[test]
    fn execute_runs_and_cleans_after_error() -> Result<()> {
        let runner = StubRunner::default();
        let status_json =
            r#"{"sessions":[{"name":"bead-err","workspace_path":"/tmp/workspace/bead-err"}]}"#;

        runner.push_response(success("")); // cleanup
        runner.push_response(success(status_json));
        runner.push_response(success(""));

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        let result: Result<()> = manager.execute_with_workspace("bead-err", |_| {
            Err(Error::InvalidRecord {
                reason: "boom".to_string(),
            })
        });

        assert!(matches!(result, Err(Error::InvalidRecord { .. })));

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[2].1.first().cloned().unwrap_or_default(), "remove");
        Ok(())
    }

    #[test]
    fn list_workspaces_parses_jj_output() -> Result<()> {
        let runner = StubRunner::default();
        let list_output = r#"workspace-1 user@example.com 2024-01-01 12:00:00
workspace-2 user@example.com 2024-01-02 13:00:00
"#;

        runner.push_response(success(list_output));

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        let workspaces = manager.list_workspaces()?;

        assert_eq!(workspaces.len(), 2);
        assert_eq!(workspaces[0].name, "workspace-1");
        assert_eq!(workspaces[1].name, "workspace-2");
        // Age should be calculated based on current time
        assert!(workspaces[0].age_seconds > 0);
        assert!(workspaces[1].age_seconds > 0);

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "jj");
        assert_eq!(calls[0].1[0], "workspace");
        assert_eq!(calls[0].1[1], "list");
        Ok(())
    }

    #[test]
    fn cleanup_orphaned_workspaces_removes_old_inactive_workspaces() -> Result<()> {
        let runner = StubRunner::default();

        // Create a timestamp that's 3 hours old
        let old_timestamp = chrono::Utc::now() - chrono::Duration::hours(3);
        let old_timestamp_str = old_timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        // Create a timestamp that's 1 hour old
        let new_timestamp = chrono::Utc::now() - chrono::Duration::hours(1);
        let new_timestamp_str = new_timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        let list_output = format!(
            "old-workspace user@example.com {old_timestamp_str}\n\
             new-workspace user@example.com {new_timestamp_str}\n\
             active-workspace user@example.com {old_timestamp_str}\n"
        );

        // Responses: forget for old-workspace only
        runner.push_response(success("")); // forget old-workspace
        runner.push_response(success(&list_output)); // list

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        // Only active-workspace is active, old-workspace is orphaned (3 hours old)
        // new-workspace is too young (1 hour old)
        let mut active_workspaces = HashSet::new();
        active_workspaces.insert("active-workspace".to_string());

        let cleaned = manager.cleanup_orphaned_workspaces(2, &active_workspaces)?;

        assert_eq!(cleaned, 1, "Should clean up one orphaned workspace");

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "jj");
        assert_eq!(calls[0].1[0], "workspace");
        assert_eq!(calls[0].1[1], "list");
        assert_eq!(calls[1].0, "jj");
        assert_eq!(calls[1].1[0], "workspace");
        assert_eq!(calls[1].1[1], "forget");
        assert_eq!(calls[1].1[2], "old-workspace");
        Ok(())
    }

    #[test]
    fn cleanup_orphaned_workspaces_skips_active_workspaces() -> Result<()> {
        let runner = StubRunner::default();

        let old_timestamp = chrono::Utc::now() - chrono::Duration::hours(3);
        let old_timestamp_str = old_timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        let list_output = format!(
            "workspace-1 user@example.com {old_timestamp_str}\n\
             workspace-2 user@example.com {old_timestamp_str}\n"
        );

        // No forget calls expected
        runner.push_response(success(&list_output));

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        // Both workspaces are active
        let mut active_workspaces = HashSet::new();
        active_workspaces.insert("workspace-1".to_string());
        active_workspaces.insert("workspace-2".to_string());

        let cleaned = manager.cleanup_orphaned_workspaces(2, &active_workspaces)?;

        assert_eq!(cleaned, 0, "Should not clean up active workspaces");

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1[0], "workspace");
        assert_eq!(calls[0].1[1], "list");
        Ok(())
    }

    #[test]
    fn cleanup_orphaned_workspaces_skips_young_workspaces() -> Result<()> {
        let runner = StubRunner::default();

        let recent_timestamp = chrono::Utc::now() - chrono::Duration::minutes(30);
        let recent_timestamp_str = recent_timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        let list_output = format!("young-workspace user@example.com {recent_timestamp_str}\n");

        runner.push_response(success(&list_output));

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        let active_workspaces = HashSet::new(); // No active workspaces

        let cleaned = manager.cleanup_orphaned_workspaces(2, &active_workspaces)?;

        assert_eq!(
            cleaned, 0,
            "Should not clean up workspaces younger than threshold"
        );

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1[0], "workspace");
        assert_eq!(calls[0].1[1], "list");
        Ok(())
    }

    #[test]
    fn parse_workspace_list_handles_empty_input() -> Result<()> {
        let output = "";
        let workspaces = parse_workspace_list(output)?;

        assert!(workspaces.is_empty());
        Ok(())
    }

    #[test]
    fn parse_workspace_list_handles_multiple_lines() -> Result<()> {
        let output = r#"ws1 user@example.com 2024-01-01 12:00:00
ws2 user@example.com 2024-01-02 13:00:00
ws3 user@example.com 2024-01-03 14:00:00
"#;
        let workspaces = parse_workspace_list(output)?;

        assert_eq!(workspaces.len(), 3);
        assert_eq!(workspaces[0].name, "ws1");
        assert_eq!(workspaces[1].name, "ws2");
        assert_eq!(workspaces[2].name, "ws3");
        Ok(())
    }

    #[test]
    fn parse_workspace_list_rejects_invalid_format() {
        let output = "invalid-line-format";
        let result = parse_workspace_list(output);

        assert!(result.is_err());
    }

    #[test]
    fn parse_age_from_timestamp_calculates_correct_age() -> Result<()> {
        // Create a timestamp 2 hours ago
        let timestamp = chrono::Utc::now() - chrono::Duration::hours(2);
        let timestamp_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        let age = parse_age_from_timestamp(&timestamp_str)?;

        // Age should be approximately 2 hours (7200 seconds)
        // Allow 10 second tolerance for test execution time
        assert!(
            (7190..=7210).contains(&age),
            "Age should be ~7200 seconds, got {age}"
        );
        Ok(())
    }

    #[test]
    fn parse_age_from_timestamp_rejects_future_timestamp() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let future_str = future.format("%Y-%m-%d %H:%M:%S").to_string();

        let result = parse_age_from_timestamp(&future_str);

        assert!(result.is_err());
    }

    #[test]
    fn forget_workspace_calls_correct_command() -> Result<()> {
        let runner = StubRunner::default();
        runner.push_response(success(""));

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        manager.forget_workspace("test-workspace")?;

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "jj");
        assert_eq!(calls[0].1[0], "workspace");
        assert_eq!(calls[0].1[1], "forget");
        assert_eq!(calls[0].1[2], "test-workspace");
        Ok(())
    }
}
