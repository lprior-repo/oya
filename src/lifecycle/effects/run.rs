#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use crate::lifecycle::types::{FailureCategory, LifecycleError, WorkspaceName};

use super::{
    classify_command_failure, classify_non_zero, effect_timeout_secs, CommandExecutor,
    CommandResult, Compensation, Effect, EffectJournalEntry, DEFAULT_CLI_TIMEOUT_SECS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoonTask {
    Ci,
    CiLegacy,
    Quick,
    QuickLegacy,
}

impl MoonTask {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Ci => ":ci",
            Self::CiLegacy => "ci",
            Self::Quick => ":quick",
            Self::QuickLegacy => "quick",
        }
    }

    fn command_args(self) -> Vec<String> {
        vec!["run".to_owned(), self.as_arg().to_owned()]
    }
}

/// Executes a lifecycle effect using the command executor.
///
/// # Errors
/// Returns a classified `LifecycleError` when command execution fails or exits non-zero.
pub async fn run_effect(
    executor: &dyn CommandExecutor,
    effect: Effect,
) -> Result<EffectJournalEntry, LifecycleError> {
    if let Effect::WorkspacePrepare { workspace, path } = effect.clone() {
        return prepare_workspace(executor, workspace, path).await;
    }
    if let Effect::MoonCi { cwd } = effect.clone() {
        return run_moon_ci_effect(executor, effect, cwd).await;
    }

    let timeout_secs = effect_timeout_secs(&effect);
    let timeout = Duration::from_secs(timeout_secs);
    let output = run_effect_command(executor, &effect, timeout).await;

    match output {
        Ok(result) if status_ok(result.status_code) => Ok(EffectJournalEntry {
            effect,
            timeout_secs,
            success: true,
            stdout: result.stdout,
            stderr: result.stderr,
        }),
        Ok(result) => {
            if let Some(existing_url) = existing_pr_url_from_non_zero(&effect, &result.stderr) {
                return Ok(EffectJournalEntry {
                    effect,
                    timeout_secs,
                    success: true,
                    stdout: existing_url,
                    stderr: result.stderr,
                });
            }
            Err(classify_non_zero(&effect, result.status_code, &result.stderr))
        }
        Err(failure) => Err(classify_command_failure(&effect, failure)),
    }
}

async fn run_effect_command(
    executor: &dyn CommandExecutor,
    effect: &Effect,
    timeout: Duration,
) -> Result<CommandResult, super::CommandFailure> {
    let (program, args, cwd) = effect_command(effect);
    let first = executor.run(program, &args, timeout, cwd.as_deref()).await;
    match first {
        Ok(result) if is_stale_workspace_add(effect, &result.stderr) => {
            let refresh_args = vec!["workspace".to_owned(), "update-stale".to_owned()];
            let _ = executor.run("jj", &refresh_args, timeout, None).await;
            executor.run(program, &args, timeout, cwd.as_deref()).await
        }
        other => other,
    }
}

fn is_stale_workspace_add(effect: &Effect, stderr: &str) -> bool {
    matches!(effect, Effect::Jj { args, .. } if is_workspace_add_args(args))
        && stderr.to_ascii_lowercase().contains("working copy is stale")
}

fn is_workspace_add_args(args: &[String]) -> bool {
    args.first().is_some_and(|arg| arg == "workspace")
        && args.get(1).is_some_and(|arg| arg == "add")
}

/// Executes a compensation command.
///
/// # Errors
/// Returns a classified `LifecycleError` when the compensation command fails.
pub async fn run_compensation(
    executor: &dyn CommandExecutor,
    compensation: Compensation,
) -> Result<EffectJournalEntry, LifecycleError> {
    match compensation {
        Compensation::ForgetWorkspace { workspace } => {
            run_forget_workspace_compensation(executor, workspace).await
        }
        Compensation::MarkBeadBlocked { bead, reason } => {
            let effect = Effect::Br {
                args: vec![
                    "update".to_owned(),
                    bead.bead_id.as_str().to_owned(),
                    "--status".to_owned(),
                    "blocked".to_owned(),
                    "--notes".to_owned(),
                    reason,
                ],
                cwd: None,
            };
            run_effect(executor, effect).await
        }
    }
}

async fn run_forget_workspace_compensation(
    executor: &dyn CommandExecutor,
    workspace: WorkspaceName,
) -> Result<EffectJournalEntry, LifecycleError> {
    let timeout_secs = DEFAULT_CLI_TIMEOUT_SECS;
    let timeout = Duration::from_secs(timeout_secs);
    let args = vec!["workspace".to_owned(), "forget".to_owned(), workspace.as_str().to_owned()];
    let jj_result = executor.run("jj", &args, timeout, None).await;
    let workspace_path = workspace.workspace_path();
    let dir_result = remove_workspace_dir(&workspace_path);
    match (jj_result, dir_result) {
        (Ok(jj_output), Ok(_)) => Ok(EffectJournalEntry {
            effect: Effect::Jj { args, cwd: None },
            timeout_secs,
            success: true,
            stdout: jj_output.stdout,
            stderr: jj_output.stderr,
        }),
        (Err(jj_err), Err(dir_err)) => {
            let jj_classified =
                classify_command_failure(&Effect::Jj { args: args.clone(), cwd: None }, jj_err);
            let dir_classified = dir_err;
            let message = format!(
                "jj forget failed: {}, directory removal failed: {}",
                jj_classified.message(),
                dir_classified.message()
            );
            Err(LifecycleError::terminal(FailureCategory::Workspace, message))
        }
        (Err(jj_err), Ok(_)) => {
            let jj_classified =
                classify_command_failure(&Effect::Jj { args: args.clone(), cwd: None }, jj_err);
            Err(LifecycleError::terminal(
                FailureCategory::Workspace,
                format!("jj forget failed: {}", jj_classified.message()),
            ))
        }
        (Ok(_), Err(dir_err)) => Err(dir_err),
    }
}

fn effect_command(effect: &Effect) -> (&'static str, Vec<String>, Option<String>) {
    match effect {
        Effect::WorkspacePrepare { .. } => ("true", Vec::new(), None),
        Effect::Jj { args, cwd } => ("jj", args.clone(), cwd.clone()),
        Effect::Br { args, cwd } => ("bd", args.clone(), cwd.clone()),
        Effect::Gh { args, cwd } => ("gh", args.clone(), cwd.clone()),
        Effect::MoonRun { task, cwd } => {
            ("moon", vec!["run".to_owned(), task.clone()], cwd.clone())
        }
        Effect::MoonCi { cwd } => ("moon", vec!["run".to_owned(), ":ci".to_owned()], cwd.clone()),
        Effect::Opencode { prompt, model, cwd } | Effect::OpencodeQa { prompt, model, cwd } => (
            "opencode",
            vec![
                "run".to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
                "--model".to_owned(),
                model.clone(),
                prompt.clone(),
            ],
            cwd.clone(),
        ),
    }
}

fn status_ok(status_code: Option<i32>) -> bool {
    status_code == Some(0)
}

fn existing_pr_url_from_non_zero(effect: &Effect, stderr: &str) -> Option<String> {
    match effect {
        Effect::Gh { args, .. } if is_pr_create_args(args) => extract_pull_request_url(stderr),
        _ => None,
    }
}

fn is_pr_create_args(args: &[String]) -> bool {
    args.first().is_some_and(|value| value == "pr")
        && args.get(1).is_some_and(|value| value == "create")
}

fn extract_pull_request_url(raw: &str) -> Option<String> {
    raw.split_whitespace()
        .map(|token| token.trim_end_matches([')', ']', '.', ',', ';']))
        .find(|token| token.starts_with("https://") && token.contains("/pull/"))
        .map(std::borrow::ToOwned::to_owned)
}

async fn run_moon_ci_effect(
    executor: &dyn CommandExecutor,
    effect: Effect,
    cwd: Option<String>,
) -> Result<EffectJournalEntry, LifecycleError> {
    let timeout_secs = effect_timeout_secs(&effect);
    let timeout = Duration::from_secs(timeout_secs);
    for task in moon_ci_attempts() {
        let args = task.command_args();
        let output = executor.run("moon", &args, timeout, cwd.as_deref()).await;
        match output {
            Ok(result) if status_ok(result.status_code) => {
                return Ok(EffectJournalEntry {
                    effect,
                    timeout_secs,
                    success: true,
                    stdout: result.stdout,
                    stderr: result.stderr,
                });
            }
            Ok(result) => {
                if is_missing_moon_task(&result.stdout, &result.stderr, task) {
                    continue;
                }
                return Err(classify_non_zero(
                    &Effect::MoonCi { cwd: cwd.clone() },
                    result.status_code,
                    &result.stderr,
                ));
            }
            Err(failure) => {
                return Err(classify_command_failure(
                    &Effect::MoonCi { cwd: cwd.clone() },
                    failure,
                ));
            }
        }
    }

    Err(LifecycleError::terminal(
        FailureCategory::Command,
        "moon CI task missing (tried: run :ci, run ci, run :quick, run quick)".to_owned(),
    ))
}

fn moon_ci_attempts() -> [MoonTask; 4] {
    [MoonTask::Ci, MoonTask::CiLegacy, MoonTask::Quick, MoonTask::QuickLegacy]
}

fn is_missing_moon_task(stdout: &str, stderr: &str, task: MoonTask) -> bool {
    let combined = format!("{stdout}\n{stderr}");
    combined.contains("No tasks found for target(s)") && combined.contains(task.as_arg())
}

async fn prepare_workspace(
    executor: &dyn CommandExecutor,
    workspace: WorkspaceName,
    path: String,
) -> Result<EffectJournalEntry, LifecycleError> {
    let timeout_secs = effect_timeout_secs(&Effect::WorkspacePrepare {
        workspace: workspace.clone(),
        path: path.clone(),
    });
    let timeout = Duration::from_secs(timeout_secs);
    let args = vec!["workspace".to_owned(), "forget".to_owned(), workspace.as_str().to_owned()];
    let forget_result = executor.run("jj", &args, timeout, None).await;
    let path_result = remove_workspace_dir(&path);
    let stderr = match forget_result {
        Ok(_) => String::new(),
        Err(error) => error.to_string(),
    };
    path_result.map(|stdout| EffectJournalEntry {
        effect: Effect::WorkspacePrepare { workspace, path },
        timeout_secs,
        success: true,
        stdout,
        stderr,
    })
}

fn remove_workspace_dir(path: &str) -> Result<String, LifecycleError> {
    let target = Path::new(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            LifecycleError::terminal(
                FailureCategory::Workspace,
                format!("failed to create workspace parent {}: {error}", parent.display()),
            )
        })?;
    }
    if target.exists() {
        remove_workspace_dir_with_retry(target, path)?;
        Ok(format!("workspace path {path} prepared"))
    } else {
        Ok(format!("workspace path {path} already clean"))
    }
}

fn remove_workspace_dir_with_retry(target: &Path, path: &str) -> Result<(), LifecycleError> {
    const ATTEMPTS: usize = 4;
    let mut last_error: Option<std::io::Error> = None;
    for _ in 0..ATTEMPTS {
        match fs::remove_dir_all(target) {
            Ok(()) => return Ok(()),
            Err(error) if is_retryable_workspace_cleanup_error(&error) => {
                last_error = Some(error);
            }
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(workspace_cleanup_error(path, &error)),
        }
    }
    match last_error {
        Some(error) => Err(workspace_cleanup_error(path, &error)),
        None => Err(workspace_cleanup_exhausted_error(path)),
    }
}

fn is_retryable_workspace_cleanup_error(error: &std::io::Error) -> bool {
    error.kind() == ErrorKind::DirectoryNotEmpty
}

fn workspace_cleanup_error(path: &str, error: &std::io::Error) -> LifecycleError {
    LifecycleError::terminal(
        FailureCategory::Workspace,
        format!("failed to clean workspace directory {path}: {error}"),
    )
}

fn workspace_cleanup_exhausted_error(path: &str) -> LifecycleError {
    LifecycleError::terminal(
        FailureCategory::Workspace,
        format!("failed to clean workspace directory {path}: retry exhausted"),
    )
}

#[cfg(test)]
mod compensation_tests {
    use super::*;
    use crate::lifecycle::effects::{CommandExecutor, CommandFailure, CommandResult};
    use crate::lifecycle::types::BeadData;
    use std::sync::Arc;
    use tokio::time::Duration;

    struct MockExecutor {
        jj_result: CommandResult,
        jj_err: Option<CommandFailure>,
    }

    #[async_trait::async_trait]
    impl CommandExecutor for MockExecutor {
        async fn run(
            &self,
            program: &str,
            _args: &[String],
            _timeout: Duration,
            _cwd: Option<&str>,
        ) -> Result<CommandResult, CommandFailure> {
            if program == "jj" {
                match self.jj_err {
                    Some(ref err) => {
                        let err_msg = err.to_string();
                        Err(CommandFailure::Spawn { message: err_msg })
                    }
                    None => Ok(self.jj_result.clone()),
                }
            } else {
                Ok(CommandResult {
                    status_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
        }
    }

    fn create_test_workspace() -> WorkspaceName {
        WorkspaceName::parse("oya-test-001").expect("valid workspace name")
    }

    #[tokio::test]
    async fn test_compensation_runs_jj_workspace_forget_and_removes_directory() {
        let executor: Arc<dyn CommandExecutor> = Arc::new(MockExecutor {
            jj_result: CommandResult {
                status_code: Some(0),
                stdout: "forgotten".to_owned(),
                stderr: String::new(),
            },
            jj_err: None,
        });
        let workspace = create_test_workspace();
        let result =
            run_compensation(&*executor, Compensation::ForgetWorkspace { workspace }).await;
        assert!(result.is_ok(), "Should succeed when jj forget and dir removal succeed");
    }

    #[tokio::test]
    async fn test_compensation_fails_when_jj_workspace_forget_fails() {
        let executor: Arc<dyn CommandExecutor> = Arc::new(MockExecutor {
            jj_result: CommandResult {
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            jj_err: Some(CommandFailure::Spawn { message: "workspace not found".to_owned() }),
        });
        let workspace = create_test_workspace();
        let result =
            run_compensation(&*executor, Compensation::ForgetWorkspace { workspace }).await;
        assert!(result.is_err(), "Should fail when jj workspace forget fails");
        let err = result.unwrap_err();
        assert!(err.is_terminal(), "Should be terminal error");
    }

    #[tokio::test]
    async fn test_compensation_mark_bead_blocked_runs_effect() {
        let executor: Arc<dyn CommandExecutor> = Arc::new(MockExecutor {
            jj_result: CommandResult {
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            jj_err: None,
        });
        let bead_data = BeadData::from_bead_id(
            crate::lifecycle::types::BeadId::parse("test-001").expect("valid bead id"),
        );
        let result = run_compensation(
            &*executor,
            Compensation::MarkBeadBlocked { bead: bead_data, reason: "test".to_owned() },
        )
        .await;
        assert!(result.is_ok(), "MarkBeadBlocked compensation should succeed");
    }

    #[tokio::test]
    async fn test_forget_workspace_error_classification() {
        let executor: Arc<dyn CommandExecutor> = Arc::new(MockExecutor {
            jj_result: CommandResult {
                status_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            jj_err: Some(CommandFailure::Timeout { timeout_secs: 30 }),
        });
        let workspace = create_test_workspace();
        let result =
            run_compensation(&*executor, Compensation::ForgetWorkspace { workspace }).await;
        assert!(result.is_err(), "Should fail on timeout");
        let err = result.unwrap_err();
        assert!(err.is_terminal(), "Should be terminal error");
    }
}
