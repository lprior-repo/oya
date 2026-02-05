#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Workspace isolation for bead execution.
//!
//! Creates a zjj workspace per bead, returns the workspace path for execution,
//! and guarantees cleanup through RAII.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use tracing::{info, warn};

use crate::error::{Error, Result};
use crate::process::{run_command, CommandResult};
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
            runner: Arc::new(SystemWorkspaceCommandRunner::default()),
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

        let remove_args = ["remove", self.name.as_str(), "-f"]; // force removal, no prompts
        let remove_result = self.runner.run("zjj", &remove_args, &self.repo_root)?;
        remove_result.check_success()?;

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
    let parsed: ZjjStatusEnvelope = serde_json::from_str(status_json)
        .map_err(|err| Error::json_parse_failed(format!("zjj status parse error: {err}")))?;

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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Clone, Default)]
    struct StubRunner {
        calls: Arc<Mutex<Vec<(String, Vec<String>, PathBuf)>>>,
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
                let arg_vec = args.iter().map(|a| a.to_string()).collect();
                calls.push((cmd.to_string(), arg_vec, cwd.to_path_buf()));
            }

            let mut responses = self.responses.lock().map_err(|err| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("mutex poisoned: {err}"),
                ))
            })?;

            responses.pop().ok_or_else(|| Error::InvalidRecord {
                reason: "no stubbed response".to_string(),
            })
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
    fn creates_workspace_and_cleans_up() {
        let runner = StubRunner::default();
        let status_json =
            r#"{"sessions":[{"name":"bead-123","workspace_path":"/tmp/workspace/bead-123"}]}"#;

        runner.push_response(success("")); // cleanup remove
        runner.push_response(success(status_json)); // status
        runner.push_response(success("")); // add

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        let mut guard = manager
            .create_for_bead("bead-123")
            .expect("workspace created");
        assert_eq!(guard.path(), Path::new("/tmp/workspace/bead-123"));
        assert_eq!(guard.name(), "bead-123");

        let cleanup = guard.cleanup();
        assert!(cleanup.is_ok());

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "zjj");
        assert_eq!(calls[1].0, "zjj");
        assert_eq!(calls[2].0, "zjj");
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
        assert_eq!(sanitized, "bead-id-with-spaces-");

        let empty = sanitize_workspace_name("***");
        assert_eq!(empty, "workspace");
    }

    #[test]
    fn execute_runs_and_cleans_after_error() {
        let runner = StubRunner::default();
        let status_json =
            r#"{"sessions":[{"name":"bead-err","workspace_path":"/tmp/workspace/bead-err"}]}"#;

        runner.push_response(success("")); // cleanup
        runner.push_response(success(status_json));
        runner.push_response(success(""));

        let manager =
            WorkspaceManager::with_runner(PathBuf::from("/repo"), Arc::new(runner.clone()));

        let result = manager.execute_with_workspace("bead-err", |_| {
            Err(Error::InvalidRecord {
                reason: "boom".to_string(),
            })
        });

        assert!(matches!(result, Err(Error::InvalidRecord { .. })));

        let calls = runner.recorded_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[2].1.get(0).cloned().unwrap_or_default(), "remove");
    }
}
