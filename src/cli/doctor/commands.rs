#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::path::Path;

pub struct CommandOutcome {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub async fn run_command_outcome(
    command: &str,
    args: &[&str],
    workdir: Option<&Path>,
) -> anyhow::Result<CommandOutcome> {
    let mut process = tokio::process::Command::new(command);
    process.args(args);
    if let Some(path) = workdir {
        process.current_dir(path);
    }
    let output = process
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run {command}: {error}"))?;
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("{command} output was not UTF-8: {error}"))?;
    let stderr = String::from_utf8(output.stderr)
        .map_err(|error| anyhow::anyhow!("{command} output was not UTF-8: {error}"))?;
    Ok(CommandOutcome { success: output.status.success(), stdout, stderr })
}

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
