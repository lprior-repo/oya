#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

mod run;

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
pub const DEFAULT_CLI_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    WorkspacePrepare { workspace: WorkspaceName, path: String },
    Jj { args: Vec<String>, cwd: Option<String> },
    Br { args: Vec<String>, cwd: Option<String> },
    Gh { args: Vec<String>, cwd: Option<String> },
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
        let output = time::timeout(timeout, command.output())
            .await
            .map_err(|_| CommandFailure::Timeout { timeout_secs: timeout.as_secs() })?
            .map_err(|error| CommandFailure::Spawn { message: error.to_string() })?;
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
        Effect::MoonCi { .. } => MOON_CI_TIMEOUT_SECS,
        Effect::Opencode { .. } => OPENCODE_TIMEOUT_SECS,
        Effect::WorkspacePrepare { .. }
        | Effect::Jj { .. }
        | Effect::Br { .. }
        | Effect::Gh { .. } => DEFAULT_CLI_TIMEOUT_SECS,
    }
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
        Effect::WorkspacePrepare { .. } | Effect::Jj { .. } => {
            LifecycleError::terminal(FailureCategory::Workspace, message)
        }
        Effect::Br { .. } | Effect::MoonCi { .. } => {
            LifecycleError::terminal(FailureCategory::Command, message)
        }
        Effect::Gh { .. } => LifecycleError::terminal(FailureCategory::PullRequest, message),
        Effect::Opencode { .. } => LifecycleError::transient(FailureCategory::Command, message),
    }
}

pub use run::{run_compensation, run_effect};

/// Executes a compensation command.
///
/// # Errors
/// Returns an `anyhow::Error` when the compensation command fails.
pub(crate) async fn run_compensation_effect(
    executor: &dyn CommandExecutor,
    effect: Effect,
) -> anyhow::Result<EffectJournalEntry> {
    run_effect(executor, effect)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("compensation command failed")
}
