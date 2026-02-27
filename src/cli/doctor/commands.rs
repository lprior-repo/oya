#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::path::Path;

pub struct CommandOutcome {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

struct CommandSpec<'a> {
    program: &'a str,
    args: &'a [&'a str],
    workdir: Option<&'a Path>,
}

fn decode_utf8(bytes: Vec<u8>, label: &str) -> anyhow::Result<String> {
    String::from_utf8(bytes)
        .map_err(|error| anyhow::anyhow!("{label} output was not UTF-8: {error}"))
}

async fn execute(spec: CommandSpec<'_>) -> anyhow::Result<std::process::Output> {
    match spec.workdir {
        Some(path) => tokio::process::Command::new(spec.program)
            .args(spec.args)
            .current_dir(path)
            .output()
            .await
            .map_err(|error| anyhow::anyhow!("failed to run {}: {error}", spec.program)),
        None => tokio::process::Command::new(spec.program)
            .args(spec.args)
            .output()
            .await
            .map_err(|error| anyhow::anyhow!("failed to run {}: {error}", spec.program)),
    }
}

/// # Errors
///
/// Returns an error when command execution fails or captured output is not valid UTF-8.
pub async fn run_command_outcome(
    command: &str,
    args: &[&str],
    workdir: Option<&Path>,
) -> anyhow::Result<CommandOutcome> {
    let output = execute(CommandSpec { program: command, args, workdir }).await?;
    let stdout = decode_utf8(output.stdout, command)?;
    let stderr = decode_utf8(output.stderr, command)?;
    Ok(CommandOutcome { success: output.status.success(), stdout, stderr })
}

/// # Errors
///
/// Returns an error when command execution fails or captured output is not valid UTF-8.
pub async fn run_command_capture(
    command: &str,
    args: &[&str],
    workdir: Option<&Path>,
) -> anyhow::Result<String> {
    let output = run_command_outcome(command, args, workdir).await?;
    if output.success {
        Ok(output.stdout)
    } else {
        Err(anyhow::anyhow!("{command} failed: {}", output.stderr.trim()))
    }
}
