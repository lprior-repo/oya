use super::super::*;
use super::http::{
    build_blocking_http_client, opencode_config, opencode_http_client_settings, OpenCodeConfig,
};
use std::path::PathBuf;
use std::process::Command;

const OPENCODE_TIMEOUT_SECONDS: u64 = 300;

pub(crate) fn run_command_with_timeout(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String), OyaError> {
    let (passed, stdout, stderr, _exit_code) =
        run_command_with_timeout_with_exit(command_name, args, timeout_seconds, repo_root)?;
    Ok((passed, combine_command_output(stdout, stderr)))
}

pub(crate) fn combine_command_output(stdout: String, stderr: String) -> String {
    format!("{}\n{}", stdout, stderr)
}

#[tracing::instrument(
    name = "cli_command",
    skip(repo_root),
    fields(
        command = %command_name,
        args = ?args,
        timeout_seconds = timeout_seconds,
        repo_root = %repo_root.display()
    )
)]
pub(crate) fn run_command_with_timeout_with_exit(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String, String, i32), OyaError> {
    let start = std::time::Instant::now();
    let result = if has_timeout_command() {
        run_with_timeout_command(command_name, args, timeout_seconds, repo_root)?
    } else {
        run_with_spawn_fallback(command_name, args, timeout_seconds, repo_root)?
    };
    log_cli_command(CommandLog {
        command_name,
        args,
        timeout_seconds,
        duration_ms: start.elapsed().as_millis(),
        result: &result,
    });
    Ok(result)
}

fn has_timeout_command() -> bool {
    Command::new("which").arg("timeout").output().is_ok_and(|output| output.status.success())
}

fn run_with_timeout_command(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String, String, i32), OyaError> {
    let output = Command::new("timeout")
        .arg(timeout_seconds.to_string())
        .arg(command_name)
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|error| {
            OyaError(format!("Failed to run {} with timeout: {}", command_name, error))
        })?;
    Ok(command_output_result(output))
}

fn run_with_spawn_fallback(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String, String, i32), OyaError> {
    let child = Command::new(command_name)
        .args(args)
        .current_dir(repo_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| spawn_error(command_name, error))?;
    wait_for_child(child, timeout_seconds)
}

fn spawn_error(command_name: &str, error: std::io::Error) -> OyaError {
    if error.kind() == std::io::ErrorKind::NotFound {
        OyaError(format!(
            "Command '{}' not found. Please ensure it is installed and in PATH.",
            command_name
        ))
    } else {
        OyaError(format!("Failed to spawn {}: {}", command_name, error))
    }
}

fn wait_for_child(
    mut child: std::process::Child,
    timeout_seconds: u64,
) -> Result<(bool, String, String, i32), OyaError> {
    let timeout = std::time::Duration::from_secs(timeout_seconds);
    let start_wait = std::time::Instant::now();
    loop {
        if start_wait.elapsed() > timeout {
            let _kill = child.kill();
            let _wait = child.wait();
            return Ok((
                false,
                String::new(),
                format!("Command timed out after {} seconds", timeout_seconds),
                124,
            ));
        }
        if let Some(result) = child_wait_result(&mut child)? {
            return Ok(result);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn child_wait_result(
    child: &mut std::process::Child,
) -> Result<Option<(bool, String, String, i32)>, OyaError> {
    match child.try_wait() {
        Ok(Some(status)) => {
            let exit_code = status.code().map_or(128, std::convert::identity);
            let stdout = read_child_stdout(child.stdout.take());
            let stderr = read_child_stderr(child.stderr.take());
            Ok(Some((status.success(), stdout, stderr, exit_code)))
        }
        Ok(None) => Ok(None),
        Err(error) => Err(OyaError(format!("Failed to wait for process: {}", error))),
    }
}

fn read_child_stdout(pipe: Option<std::process::ChildStdout>) -> String {
    pipe.map_or_else(String::new, read_pipe_to_string)
}

fn read_child_stderr(pipe: Option<std::process::ChildStderr>) -> String {
    pipe.map_or_else(String::new, read_pipe_to_string)
}

fn read_pipe_to_string<T: std::io::Read>(mut stream: T) -> String {
    let mut buffer = String::new();
    let _read = std::io::Read::read_to_string(&mut stream, &mut buffer);
    buffer
}

fn command_output_result(output: std::process::Output) -> (bool, String, String, i32) {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().map_or(128, std::convert::identity);
    (output.status.success(), stdout, stderr, exit_code)
}

struct CommandLog<'a> {
    command_name: &'a str,
    args: &'a [&'a str],
    timeout_seconds: u64,
    duration_ms: u128,
    result: &'a (bool, String, String, i32),
}

fn log_cli_command(command: CommandLog<'_>) {
    let (success, stdout, stderr, exit_code) = command.result;
    tracing::debug!(stdout = %stdout, stderr = %stderr, "CLI command detailed output");
    tracing::info!(
        command = %command.command_name,
        args = ?command.args,
        timeout_seconds = command.timeout_seconds,
        exit_code = *exit_code,
        duration_ms = command.duration_ms,
        stdout_len = stdout.len(),
        stderr_len = stderr.len(),
        timed_out = *exit_code == 124,
        success = *success,
        "CLI command execution"
    );
}

pub(crate) fn run_opencode(
    prompt: &str,
    repo_root: &PathBuf,
    model: &str,
) -> Result<(bool, String), OyaError> {
    tracing::info!("Running opencode with prompt ({} chars) model={}", prompt.len(), model);
    match run_command_with_timeout(
        "opencode",
        &["run", "--format", "json", "--model", model, prompt],
        OPENCODE_TIMEOUT_SECONDS,
        repo_root,
    ) {
        Ok(res) => Ok(res),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("not found") || msg.contains("not found.") {
                tracing::warn!("opencode CLI not found on PATH, attempting HTTP fallback: {}", msg);
                match opencode_config() {
                    Ok(config) => run_opencode_via_http_blocking(&config, prompt, model),
                    Err(cfg_err) => Err(OyaError(format!(
                        "opencode CLI missing and opencode HTTP config invalid: {} / {}",
                        msg, cfg_err
                    ))),
                }
            } else {
                Err(err)
            }
        }
    }
}

fn run_opencode_via_http_blocking(
    config: &OpenCodeConfig,
    prompt: &str,
    model: &str,
) -> Result<(bool, String), OyaError> {
    let settings = opencode_http_client_settings(OPENCODE_TIMEOUT_SECONDS);
    let client = build_blocking_http_client(settings)
        .map_err(|e| OyaError(format!("Failed to build blocking HTTP client: {}", e)))?;

    let url = format!("{}/run", config.base_url.trim_end_matches('/'));
    let payload = serde_json::json!({ "model": model, "prompt": prompt, "format": "json" });

    let request = config.password.as_ref().map_or_else(
        || client.post(&url).json(&payload),
        |pwd| client.post(&url).basic_auth("opencode", Some(pwd)).json(&payload),
    );

    let response = request
        .send()
        .map_err(|e| OyaError(format!("OpenCode HTTP request failed for /run: {}", e)))?;
    let status = response.status();
    let text = response
        .text()
        .map_err(|e| OyaError(format!("OpenCode /run response read failed: {}", e)))?;

    if !status.is_success() {
        return Err(OyaError(format!(
            "OpenCode /run failed with status {}: {}",
            status.as_u16(),
            truncate_clean(text.as_str(), 4000)
        )));
    }

    match oya::parse_opencode_output(text.as_str()) {
        Ok(output) => Ok((true, output.stdout)),
        Err(parse_err) => {
            Err(OyaError(format!("OpenCode /run returned invalid output: {}", parse_err)))
        }
    }
}
