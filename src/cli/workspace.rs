#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use thiserror::Error;
use tokio::process::Command;

const BRANCH_PREFIX: &str = "oya/";
const MAX_BRANCH_NAME_LENGTH: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum WorkspaceOwnershipError {
    #[error("working_tree_invalid: run '{run_id}' cannot start because workspace has {pending_changes} unowned pending change(s)")]
    WorkingTreeInvalid { run_id: String, pending_changes: usize },

    #[error("workspace_status_unavailable: failed to run git status: {message}")]
    GitUnavailable { message: String },

    #[error("workspace_status_failed: git status exited unsuccessfully: {message}")]
    GitFailed { message: String },

    #[error("workspace_status_invalid: git status output was not UTF-8")]
    InvalidOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum VcsSyncError {
    #[error("vcs_sync_failed: {command} failed: {message}")]
    VcsSyncFailed { command: &'static str, message: String },
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
        Err(WorkspaceOwnershipError::WorkingTreeInvalid {
            run_id: run_id.to_owned(),
            pending_changes,
        })
    }
}

pub(crate) fn branch_name_from_ids(bead_id: &str, run_id: &str) -> String {
    let suffix =
        format!("{}-{}", sanitize_branch_component(bead_id), sanitize_branch_component(run_id));
    format!("{BRANCH_PREFIX}{}", bound_branch_suffix(&suffix))
}

pub(crate) async fn sync_workspace_with_main() -> Result<(), VcsSyncError> {
    run_vcs_sync_command("git fetch origin", &["fetch", "origin"]).await?;
    run_vcs_sync_command("git rebase origin/main", &["rebase", "origin/main"]).await
}

impl VcsSyncError {
    #[must_use]
    pub(crate) fn command(&self) -> &'static str {
        match self {
            Self::VcsSyncFailed { command, .. } => command,
        }
    }

    #[must_use]
    pub(crate) fn sanitized_message(&self) -> &str {
        match self {
            Self::VcsSyncFailed { message, .. } => message,
        }
    }
}

async fn run_vcs_sync_command(
    command_name: &'static str,
    args: &[&str],
) -> Result<(), VcsSyncError> {
    let output = Command::new("git").args(args).output().await.map_err(|error| {
        VcsSyncError::VcsSyncFailed {
            command: command_name,
            message: sanitize_vcs_message(&error.to_string()),
        }
    })?;
    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(vcs_sync_failure_from_output(command_name, &stdout, &stderr))
    }
}

fn vcs_sync_failure_from_output(command: &'static str, stdout: &str, stderr: &str) -> VcsSyncError {
    VcsSyncError::VcsSyncFailed {
        command,
        message: sanitize_vcs_message(vcs_failure_message(stdout, stderr)),
    }
}

fn vcs_failure_message<'a>(stdout: &'a str, stderr: &'a str) -> &'a str {
    if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    }
}

fn sanitize_branch_component(value: &str) -> String {
    let sanitized = value.chars().map(safe_branch_char).collect::<String>();
    let collapsed = collapse_dashes(&sanitized);
    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        "unknown".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn safe_branch_char(character: char) -> char {
    if character.is_ascii_alphanumeric() {
        character.to_ascii_lowercase()
    } else if character == '_' {
        character
    } else {
        '-'
    }
}

fn collapse_dashes(value: &str) -> String {
    value.chars().fold(String::new(), |mut output, character| {
        if character != '-' || !output.ends_with('-') {
            output.push(character);
        }
        output
    })
}

fn bound_branch_suffix(suffix: &str) -> String {
    let max_suffix_len = MAX_BRANCH_NAME_LENGTH.saturating_sub(BRANCH_PREFIX.len());
    if suffix.len() <= max_suffix_len {
        suffix.to_owned()
    } else {
        suffix.chars().take(max_suffix_len).collect::<String>().trim_matches('-').to_owned()
    }
}

fn count_pending_changes(status_stdout: &str) -> usize {
    status_stdout.lines().filter(|line| !line.trim().is_empty()).count()
}

fn sanitize_status_message(message: &str) -> String {
    message.split_whitespace().take(24).collect::<Vec<_>>().join(" ")
}

fn sanitize_vcs_message(message: &str) -> String {
    let redacted = message.lines().map(redact_vcs_line).collect::<Vec<_>>().join(" ");
    let summarized = sanitize_status_message(&redacted);
    if summarized.is_empty() {
        "no diagnostic output".to_owned()
    } else {
        summarized
    }
}

fn redact_vcs_line(line: &str) -> String {
    let normalized = line.to_ascii_lowercase();
    if is_sensitive_vcs_line(&normalized) {
        "[redacted]".to_owned()
    } else {
        line.trim().to_owned()
    }
}

fn is_sensitive_vcs_line(normalized: &str) -> bool {
    ["token", "secret", "password", "api_key", "apikey"]
        .into_iter()
        .any(|needle| normalized.contains(needle))
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
            Err(WorkspaceOwnershipError::WorkingTreeInvalid {
                run_id: "run-demo".to_owned(),
                pending_changes: 2,
            })
        );
        assert!(!result.err().unwrap().to_string().contains(".env"));
    }

    #[test]
    fn git_dirty_block_returns_working_tree_invalid() {
        let result = ensure_workspace_owned_from_status("run-demo", "?? generated.txt\n");

        assert_eq!(
            result,
            Err(WorkspaceOwnershipError::WorkingTreeInvalid {
                run_id: "run-demo".to_owned(),
                pending_changes: 1,
            })
        );
    }

    #[test]
    fn vcs_sync_failure_is_typed_and_sanitized() {
        let result = vcs_sync_failure_from_output(
            "git fetch origin",
            "",
            "remote: password=server-secret-token\nfatal: authentication failed",
        );

        assert_eq!(
            result,
            VcsSyncError::VcsSyncFailed {
                command: "git fetch origin",
                message: "[redacted] fatal: authentication failed".to_owned(),
            }
        );
        assert!(result.to_string().contains("vcs_sync_failed"));
        assert!(!result.to_string().contains("server-secret-token"));
    }

    #[test]
    fn branch_name_is_valid_bounded_and_deterministic() {
        let first = branch_name_from_ids("oya-ii7", "run-oya-ii7");
        let second = branch_name_from_ids("oya-ii7", "run-oya-ii7");

        assert_eq!(first, second);
        assert_eq!(first, "oya/oya-ii7-run-oya-ii7");
        assert!(first.len() <= 96);
        assert!(is_valid_branch_name(&first));
    }

    #[test]
    fn branch_name_sanitizes_and_bounds_untrusted_ids() {
        let branch = branch_name_from_ids(
            "../Feature ID With Spaces.lock",
            "run@{bad}///with spaces and VERY VERY VERY VERY VERY VERY LONG suffix",
        );

        assert!(branch.starts_with("oya/"));
        assert!(branch.len() <= 96);
        assert!(is_valid_branch_name(&branch));
        assert!(!branch.contains(".."));
        assert!(!branch.contains("@{"));
        assert!(!branch.ends_with(".lock"));
    }

    fn is_valid_branch_name(branch: &str) -> bool {
        !branch.contains("..")
            && !branch.contains("@{")
            && !branch.ends_with('/')
            && !branch.ends_with('.')
            && !branch.ends_with(".lock")
            && branch.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '-' | '_' | '/' | '.')
            })
    }

    #[test]
    fn workspace_ownership_ignores_blank_status_lines() {
        let result = ensure_workspace_owned_from_status("run-demo", "\n  \n");

        assert_eq!(result, Ok(()));
    }
}
