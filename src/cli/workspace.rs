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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum DiffValidationError {
    #[error("empty_diff: PR creation blocked because diff has no meaningful source changes ({changed_paths} changed path(s))")]
    EmptyDiff { changed_paths: usize },

    #[error("diff_validation_unavailable: failed to run git diff: {message}")]
    GitUnavailable { message: String },

    #[error("diff_validation_failed: git diff exited unsuccessfully: {message}")]
    GitFailed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PullRequestOutcome {
    pub(crate) branch: String,
    pub(crate) url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum PullRequestError {
    #[error("pull_request_command_failed: {command} failed: {message}")]
    CommandFailed { command: &'static str, message: String },

    #[error(
        "pull_request_url_missing: gh pr create did not return a PR URL for branch '{branch}'"
    )]
    MissingUrl { branch: String },
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

pub(crate) async fn validate_meaningful_git_diff() -> Result<(), DiffValidationError> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "origin/main...HEAD"])
        .output()
        .await
        .map_err(|error| DiffValidationError::GitUnavailable {
            message: sanitize_vcs_message(&error.to_string()),
        })?;
    if output.status.success() {
        validate_meaningful_diff_from_paths(&String::from_utf8_lossy(&output.stdout))
    } else {
        Err(DiffValidationError::GitFailed {
            message: sanitize_vcs_message(&String::from_utf8_lossy(&output.stderr)),
        })
    }
}

pub(crate) async fn create_pull_request_after_green_gates(
    branch: &str,
    title: &str,
    body: &str,
) -> Result<PullRequestOutcome, PullRequestError> {
    push_branch_for_pull_request(branch).await?;
    create_github_pull_request(branch, title, body).await
}

pub(crate) fn validate_meaningful_diff_from_paths(
    changed_paths_stdout: &str,
) -> Result<(), DiffValidationError> {
    let (changed_paths, has_meaningful_diff) = changed_paths_stdout
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .fold((0_usize, false), |(count, meaningful), path| {
            (count + 1, meaningful || is_meaningful_diff_path(path))
        });
    if has_meaningful_diff {
        Ok(())
    } else {
        Err(DiffValidationError::EmptyDiff { changed_paths })
    }
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

impl DiffValidationError {
    #[must_use]
    pub(crate) fn failure_type(&self) -> &'static str {
        match self {
            Self::EmptyDiff { .. } => "EmptyDiff",
            Self::GitUnavailable { .. } => "GitUnavailable",
            Self::GitFailed { .. } => "GitFailed",
        }
    }

    #[must_use]
    pub(crate) fn changed_paths(&self) -> Option<usize> {
        match self {
            Self::EmptyDiff { changed_paths } => Some(*changed_paths),
            Self::GitUnavailable { .. } | Self::GitFailed { .. } => None,
        }
    }

    #[must_use]
    pub(crate) fn sanitized_message(&self) -> &str {
        match self {
            Self::EmptyDiff { .. } => "empty diff blocks PR creation",
            Self::GitUnavailable { message } | Self::GitFailed { message } => message,
        }
    }
}

impl PullRequestError {
    #[must_use]
    pub(crate) fn failure_type(&self) -> &'static str {
        match self {
            Self::CommandFailed { .. } => "PullRequestCommandFailed",
            Self::MissingUrl { .. } => "PullRequestUrlMissing",
        }
    }

    #[must_use]
    pub(crate) fn command(&self) -> Option<&'static str> {
        match self {
            Self::CommandFailed { command, .. } => Some(command),
            Self::MissingUrl { .. } => None,
        }
    }

    #[must_use]
    pub(crate) fn sanitized_message(&self) -> &str {
        match self {
            Self::CommandFailed { message, .. } => message,
            Self::MissingUrl { .. } => "pull request URL missing",
        }
    }
}

async fn push_branch_for_pull_request(branch: &str) -> Result<(), PullRequestError> {
    let refspec = format!("HEAD:{branch}");
    run_pull_request_command("git push origin HEAD:<branch>", "git", &["push", "origin", &refspec])
        .await
        .map(|_| ())
}

async fn create_github_pull_request(
    branch: &str,
    title: &str,
    body: &str,
) -> Result<PullRequestOutcome, PullRequestError> {
    let output = run_pull_request_command(
        "gh pr create",
        "gh",
        &["pr", "create", "--head", branch, "--title", title, "--body", body],
    )
    .await?;
    pull_request_from_output(branch, &output)
}

async fn run_pull_request_command(
    command_name: &'static str,
    program: &str,
    args: &[&str],
) -> Result<String, PullRequestError> {
    let output = Command::new(program).args(args).output().await.map_err(|error| {
        PullRequestError::CommandFailed {
            command: command_name,
            message: sanitize_vcs_message(&error.to_string()),
        }
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        Ok(stdout.into_owned())
    } else {
        Err(pull_request_failure_from_output(command_name, &stdout, &stderr))
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

fn pull_request_from_output(
    branch: &str,
    stdout: &str,
) -> Result<PullRequestOutcome, PullRequestError> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| is_pull_request_url(line))
        .map(|url| PullRequestOutcome { branch: branch.to_owned(), url: url.to_owned() })
        .ok_or_else(|| PullRequestError::MissingUrl { branch: branch.to_owned() })
}

fn pull_request_failure_from_output(
    command: &'static str,
    stdout: &str,
    stderr: &str,
) -> PullRequestError {
    PullRequestError::CommandFailed {
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

fn is_meaningful_diff_path(path: &str) -> bool {
    !matches!(path, ".beads") && !path.starts_with(".beads/")
}

fn is_pull_request_url(line: &str) -> bool {
    (line.starts_with("https://") || line.starts_with("http://")) && line.contains("/pull/")
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
    fn diff_validation_blocks_empty_or_beads_only_diff() {
        let empty = validate_meaningful_diff_from_paths("");
        let beads_only =
            validate_meaningful_diff_from_paths(".beads/state.json\n.beads/dolt/config\n");

        assert_eq!(empty, Err(DiffValidationError::EmptyDiff { changed_paths: 0 }));
        assert_eq!(beads_only, Err(DiffValidationError::EmptyDiff { changed_paths: 2 }));
    }

    #[test]
    fn diff_validation_accepts_meaningful_source_diff() {
        let result = validate_meaningful_diff_from_paths(".beads/state.json\nsrc/cli/run.rs\n");

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn pr_creation_extracts_url_and_branch_from_output() {
        let result = pull_request_from_output(
            "oya/demo-run-demo",
            "https://github.com/priorlewis43/oya/pull/123\n",
        )
        .unwrap();

        assert_eq!(result.branch, "oya/demo-run-demo");
        assert_eq!(result.url, "https://github.com/priorlewis43/oya/pull/123");
    }

    #[test]
    fn pr_creation_failure_is_typed_and_sanitized() {
        let result = pull_request_failure_from_output(
            "gh pr create",
            "",
            "remote: token=server-secret-token\nfatal: authentication failed",
        );

        assert_eq!(
            result,
            PullRequestError::CommandFailed {
                command: "gh pr create",
                message: "[redacted] fatal: authentication failed".to_owned(),
            }
        );
        assert!(result.to_string().contains("pull_request_command_failed"));
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
