#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::{BeadData, FailureCategory, LifecycleError, WorkspaceName};
use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time;

pub const MOON_CI_TIMEOUT_SECS: u64 = 900;
pub const OPENCODE_TIMEOUT_SECS: u64 = 1_200;
const DEFAULT_CLI_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    WorkspacePrepare { workspace: WorkspaceName, path: String },
    Jj { args: Vec<String>, cwd: Option<String> },
    Br { args: Vec<String>, cwd: Option<String> },
    Gh { args: Vec<String>, cwd: Option<String> },
    Git { args: Vec<String>, cwd: Option<String> },
    MoonCi { cwd: Option<String> },
    Opencode { prompt: String, model: String, cwd: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Compensation {
    ForgetWorkspace { workspace: WorkspaceName },
    MarkBeadBlocked { bead: BeadData, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectJournalEntry {
    pub effect: Effect,
    pub timeout_secs: u64,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandFailure {
    #[error("command timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },
    #[error("failed to spawn process: {message}")]
    Spawn { message: String },
    #[error("process output was not UTF-8: {message}")]
    Utf8 { message: String },
}

#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
        cwd: Option<&str>,
    ) -> Result<CommandResult, CommandFailure>;
}

#[derive(Debug, Default)]
pub struct TokioCommandExecutor;

#[async_trait]
impl CommandExecutor for TokioCommandExecutor {
    async fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
        cwd: Option<&str>,
    ) -> Result<CommandResult, CommandFailure> {
        let mut command = Command::new(program);
        command.args(args).stdin(Stdio::null());
        if let Some(path) = cwd {
            command.current_dir(path);
        }
        let join = command.output();
        let output = time::timeout(timeout, join)
            .await
            .map_err(|_| CommandFailure::Timeout { timeout_secs: timeout.as_secs() })?;
        let output =
            output.map_err(|error| CommandFailure::Spawn { message: error.to_string() })?;
        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| CommandFailure::Utf8 { message: error.to_string() })?;
        let stderr = String::from_utf8(output.stderr)
            .map_err(|error| CommandFailure::Utf8 { message: error.to_string() })?;
        Ok(CommandResult { status_code: output.status.code(), stdout, stderr })
    }
}

#[must_use]
pub fn effect_timeout_secs(effect: &Effect) -> u64 {
    match effect {
        Effect::WorkspacePrepare { .. } => DEFAULT_CLI_TIMEOUT_SECS,
        Effect::MoonCi { .. } => MOON_CI_TIMEOUT_SECS,
        Effect::Opencode { .. } => OPENCODE_TIMEOUT_SECS,
        Effect::Jj { .. } | Effect::Br { .. } | Effect::Gh { .. } | Effect::Git { .. } => {
            DEFAULT_CLI_TIMEOUT_SECS
        }
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
    let (program, args, cwd) = effect_command(&effect);
    let timeout_secs = effect_timeout_secs(&effect);
    let timeout = Duration::from_secs(timeout_secs);
    let output = executor.run(program, &args, timeout, cwd.as_deref()).await;

    match output {
        Ok(result) if status_ok(result.status_code) => Ok(EffectJournalEntry {
            effect,
            timeout_secs,
            success: true,
            stdout: result.stdout,
            stderr: result.stderr,
        }),
        Ok(result) => {
            let error = classify_non_zero(&effect, result.status_code, &result.stderr);
            Err(error)
        }
        Err(failure) => Err(classify_command_failure(&effect, failure)),
    }
}

/// Executes a compensation command.
///
/// # Errors
/// Returns an `anyhow::Error` when the compensation command fails.
pub async fn run_compensation(
    executor: &dyn CommandExecutor,
    compensation: Compensation,
) -> anyhow::Result<EffectJournalEntry> {
    let effect = match compensation {
        Compensation::ForgetWorkspace { workspace } => Effect::Jj {
            args: vec!["workspace".to_owned(), "forget".to_owned(), workspace.as_str().to_owned()],
            cwd: None,
        },
        Compensation::MarkBeadBlocked { bead, reason } => Effect::Br {
            args: vec![
                "update".to_owned(),
                bead.bead_id.as_str().to_owned(),
                "--status".to_owned(),
                "blocked".to_owned(),
                "--notes".to_owned(),
                reason,
            ],
            cwd: None,
        },
    };
    run_effect(executor, effect)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("compensation command failed")
}

#[must_use]
pub fn classify_command_failure(effect: &Effect, failure: CommandFailure) -> LifecycleError {
    match failure {
        CommandFailure::Timeout { timeout_secs } => LifecycleError::transient(
            FailureCategory::Command,
            format!("timeout after {timeout_secs}s while running {effect:?}"),
        ),
        CommandFailure::Spawn { message } | CommandFailure::Utf8 { message } => {
            LifecycleError::transient(FailureCategory::Command, message)
        }
    }
}

#[must_use]
pub fn classify_non_zero(
    effect: &Effect,
    status_code: Option<i32>,
    stderr: &str,
) -> LifecycleError {
    let message = format!("{effect:?} exited with {:?}: {}", status_code, stderr.trim());
    match effect {
        Effect::WorkspacePrepare { .. } => {
            LifecycleError::terminal(FailureCategory::Workspace, message)
        }
        Effect::Jj { .. } => LifecycleError::terminal(FailureCategory::Workspace, message),
        Effect::Br { .. } | Effect::MoonCi { .. } | Effect::Git { .. } => {
            LifecycleError::terminal(FailureCategory::Command, message)
        }
        Effect::Gh { .. } => LifecycleError::terminal(FailureCategory::PullRequest, message),
        Effect::Opencode { .. } => LifecycleError::transient(FailureCategory::Command, message),
    }
}

fn status_ok(status_code: Option<i32>) -> bool {
    status_code == Some(0)
}

fn effect_command(effect: &Effect) -> (&'static str, Vec<String>, Option<String>) {
    match effect {
        Effect::WorkspacePrepare { .. } => ("true", Vec::new(), None),
        Effect::Jj { args, cwd } => ("jj", args.clone(), cwd.clone()),
        Effect::Br { args, cwd } => ("br", args.clone(), cwd.clone()),
        Effect::Gh { args, cwd } => ("gh", args.clone(), cwd.clone()),
        Effect::Git { args, cwd } => ("git", args.clone(), cwd.clone()),
        Effect::MoonCi { cwd } => ("moon", vec!["run".to_owned(), ":ci".to_owned()], cwd.clone()),
        Effect::Opencode { prompt, model, cwd } => (
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
    let stderr = forget_result.err().map(|error| error.to_string()).unwrap_or_default();
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
        fs::remove_dir_all(target).map_err(|error| {
            LifecycleError::terminal(
                FailureCategory::Workspace,
                format!("failed to clean workspace directory {path}: {error}"),
            )
        })?;
        Ok(format!("workspace path {path} prepared"))
    } else {
        Ok(format!("workspace path {path} already clean"))
    }
}
