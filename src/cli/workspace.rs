#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum WorkspaceOwnershipError {
    #[error("unowned_dirty_workspace: run '{run_id}' cannot start because workspace has {pending_changes} pending change(s)")]
    Dirty { run_id: String, pending_changes: usize },

    #[error("workspace_status_unavailable: failed to run git status: {message}")]
    GitUnavailable { message: String },

    #[error("workspace_status_failed: git status exited unsuccessfully: {message}")]
    GitFailed { message: String },

    #[error("workspace_status_invalid: git status output was not UTF-8")]
    InvalidOutput,
}

pub(crate) async fn ensure_clean_workspace_for_run(
    run_id: &str,
) -> Result<(), WorkspaceOwnershipError> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=normal"])
        .output()
        .await
        .map_err(|error| WorkspaceOwnershipError::GitUnavailable {
            message: sanitize_status_message(&error.to_string()),
        })?;
    if !output.status.success() {
        return Err(WorkspaceOwnershipError::GitFailed {
            message: sanitize_status_message(&String::from_utf8_lossy(&output.stderr)),
        });
    }
    let stdout =
        String::from_utf8(output.stdout).map_err(|_| WorkspaceOwnershipError::InvalidOutput)?;
    ensure_workspace_owned_from_status(run_id, &stdout)
}

pub(crate) fn ensure_workspace_owned_from_status(
    run_id: &str,
    status_stdout: &str,
) -> Result<(), WorkspaceOwnershipError> {
    let pending_changes = count_pending_changes(status_stdout);
    if pending_changes == 0 {
        Ok(())
    } else {
        Err(WorkspaceOwnershipError::Dirty { run_id: run_id.to_owned(), pending_changes })
    }
}

fn count_pending_changes(status_stdout: &str) -> usize {
    status_stdout.lines().filter(|line| !line.trim().is_empty()).count()
}

fn sanitize_status_message(message: &str) -> String {
    message.split_whitespace().take(24).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn workspace_ownership_allows_clean_status() {
        let result = ensure_workspace_owned_from_status("run-demo", "");

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn workspace_ownership_blocks_dirty_status_without_leaking_paths() {
        let result = ensure_workspace_owned_from_status("run-demo", " M src/secret.rs\n?? .env\n");

        assert_eq!(
            result,
            Err(WorkspaceOwnershipError::Dirty {
                run_id: "run-demo".to_owned(),
                pending_changes: 2,
            })
        );
        assert!(!result.err().unwrap().to_string().contains(".env"));
    }

    #[test]
    fn workspace_ownership_ignores_blank_status_lines() {
        let result = ensure_workspace_owned_from_status("run-demo", "\n  \n");

        assert_eq!(result, Ok(()));
    }
}
