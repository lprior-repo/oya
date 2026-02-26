#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::{BeadData, FailureCategory, LifecycleError, WorkspaceName};
use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time;

pub const MOON_CI_TIMEOUT_SECS: u64 = 900;
pub const OPENCODE_TIMEOUT_SECS: u64 = 1_200;
const DEFAULT_CLI_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    Jj { args: Vec<String> },
    Br { args: Vec<String> },
    Gh { args: Vec<String> },
    MoonCi,
    Opencode { prompt: String, model: String },
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
    ) -> Result<CommandResult, CommandFailure> {
        let join = Command::new(program).args(args).stdin(Stdio::null()).output();
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
        Effect::MoonCi => MOON_CI_TIMEOUT_SECS,
        Effect::Opencode { .. } => OPENCODE_TIMEOUT_SECS,
        Effect::Jj { .. } | Effect::Br { .. } | Effect::Gh { .. } => DEFAULT_CLI_TIMEOUT_SECS,
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
    let (program, args) = effect_command(&effect);
    let timeout_secs = effect_timeout_secs(&effect);
    let timeout = Duration::from_secs(timeout_secs);
    let output = executor.run(program, &args, timeout).await;

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
        Effect::Jj { .. } => LifecycleError::terminal(FailureCategory::Workspace, message),
        Effect::Br { .. } | Effect::MoonCi => {
            LifecycleError::terminal(FailureCategory::Command, message)
        }
        Effect::Gh { .. } => LifecycleError::terminal(FailureCategory::PullRequest, message),
        Effect::Opencode { .. } => LifecycleError::transient(FailureCategory::Command, message),
    }
}

fn status_ok(status_code: Option<i32>) -> bool {
    status_code == Some(0)
}

fn effect_command(effect: &Effect) -> (&'static str, Vec<String>) {
    match effect {
        Effect::Jj { args } => ("jj", args.clone()),
        Effect::Br { args } => ("br", args.clone()),
        Effect::Gh { args } => ("gh", args.clone()),
        Effect::MoonCi => ("moon", vec!["run".to_owned(), ":ci".to_owned()]),
        Effect::Opencode { prompt, model } => (
            "opencode",
            vec![
                "run".to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
                "--model".to_owned(),
                model.clone(),
                prompt.clone(),
            ],
        ),
    }
}
