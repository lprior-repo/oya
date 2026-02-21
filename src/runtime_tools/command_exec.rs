use super::super::*;
use super::http::{
    build_blocking_http_client, opencode_config, opencode_http_client_settings, OpenCodeConfig,
};
use std::path::PathBuf;
use std::process::Command;

const OPENCODE_TIMEOUT_SECONDS: u64 = 600;
const OPENCODE_CLI_RATE_LIMIT_RETRIES: u32 = 2;
const FALLBACK_MODEL: &str = "google/gemini-3-flash-preview";

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
    let opencode_program = resolve_opencode_program();
    tracing::info!("Running opencode with prompt ({} chars) model={}", prompt.len(), model);
    let command_result = run_command_with_timeout(
        opencode_program.as_str(),
        &["run", "--format", "json", "--model", model, prompt],
        OPENCODE_TIMEOUT_SECONDS,
        repo_root,
    );

    match command_result {
        Ok((passed, output)) if passed => Ok((passed, output)),
        Ok((false, output)) if is_rate_limited_cli_output(output.as_str()) => {
            retry_rate_limited_opencode(prompt, repo_root, model, output)
        }
        Ok((_passed, output)) if is_opencode_cli_missing_output(output.as_str()) => {
            match fallback_to_opencode_http(prompt, model, output.as_str()) {
                Ok(http_output) => Ok((true, http_output)),
                Err(error) => Ok((false, error.to_string())),
            }
        }
        Ok((false, output)) if is_timeout_failure(output.as_str()) => {
            retry_with_fallback_model(prompt, repo_root, model, output.as_str())
        }
        Ok(res) => Ok(res),
        Err(err) if is_opencode_missing_error(err.to_string().as_str()) => {
            match fallback_to_opencode_http(prompt, model, err.to_string().as_str()) {
                Ok(http_output) => Ok((true, http_output)),
                Err(error) => Ok((false, error.to_string())),
            }
        }
        Err(err) => Err(err),
    }
}

fn retry_with_fallback_model(
    prompt: &str,
    repo_root: &PathBuf,
    model: &str,
    output: &str,
) -> Result<(bool, String), OyaError> {
    if model == FALLBACK_MODEL {
        return Ok((false, output.to_string()));
    }

    tracing::warn!(
        "OpenCode command timed out with model {}; retrying once with fallback {}",
        model,
        FALLBACK_MODEL
    );
    run_command_with_timeout(
        resolve_opencode_program().as_str(),
        &["run", "--format", "json", "--model", FALLBACK_MODEL, prompt],
        OPENCODE_TIMEOUT_SECONDS,
        repo_root,
    )
}

fn retry_rate_limited_opencode(
    prompt: &str,
    repo_root: &PathBuf,
    model: &str,
    first_output: String,
) -> Result<(bool, String), OyaError> {
    let mut attempt = 0;
    let mut last_output = first_output;
    while attempt < OPENCODE_CLI_RATE_LIMIT_RETRIES {
        std::thread::sleep(rate_limit_backoff(attempt));
        let (passed, output) = run_command_with_timeout(
            resolve_opencode_program().as_str(),
            &["run", "--format", "json", "--model", model, prompt],
            OPENCODE_TIMEOUT_SECONDS,
            repo_root,
        )?;
        if passed {
            return Ok((true, output));
        }
        if !is_rate_limited_cli_output(output.as_str()) {
            return Ok((false, output));
        }
        last_output = output;
        attempt += 1;
    }
    Ok((false, last_output))
}

fn is_rate_limited_cli_output(output: &str) -> bool {
    oya::classify_opencode_error(output) == Some(oya::types::FailureCategory::RateLimited)
}

fn rate_limit_backoff(attempt: u32) -> std::time::Duration {
    let millis = 400_u64.saturating_mul(2_u64.saturating_pow(attempt));
    std::time::Duration::from_millis(millis)
}

fn resolve_opencode_program() -> String {
    std::env::var("OPENCODE_PATH")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "opencode".to_string())
}

fn fallback_to_opencode_http(
    prompt: &str,
    model: &str,
    source_message: &str,
) -> Result<String, OyaError> {
    tracing::warn!("opencode CLI unavailable, attempting HTTP fallback: {}", source_message);
    let config = opencode_config().map_err(|cfg_err| {
        OyaError(format!(
            "opencode CLI unavailable and opencode HTTP config invalid: {} / {}",
            source_message, cfg_err
        ))
    })?;
    let (_passed, output) = run_opencode_via_http_blocking(&config, prompt, model)?;
    Ok(output)
}

fn is_opencode_missing_error(message: &str) -> bool {
    message.contains("not found") || message.contains("not found.")
}

fn is_opencode_cli_missing_output(output: &str) -> bool {
    let normalized = output.to_ascii_lowercase();
    normalized.contains("failed to run command")
        && normalized.contains("opencode")
        && normalized.contains("no such file or directory")
}

fn is_timeout_failure(output: &str) -> bool {
    output.to_ascii_lowercase().contains("timed out")
}

fn run_opencode_via_http_blocking(
    config: &OpenCodeConfig,
    prompt: &str,
    model: &str,
) -> Result<(bool, String), OyaError> {
    let settings = opencode_http_client_settings(OPENCODE_TIMEOUT_SECONDS);
    let client = build_blocking_http_client(settings)
        .map_err(|e| OyaError(format!("Failed to build blocking HTTP client: {}", e)))?;

    verify_opencode_http_ready(&client, config)?;

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

fn verify_opencode_http_ready(
    client: &reqwest::blocking::Client,
    config: &OpenCodeConfig,
) -> Result<(), OyaError> {
    let health_url = format!("{}/session/status", config.base_url.trim_end_matches('/'));
    let request = config.password.as_ref().map_or_else(
        || client.get(&health_url),
        |pwd| client.get(&health_url).basic_auth("opencode", Some(pwd)),
    );

    let response = request
        .send()
        .map_err(|error| OyaError(format!("OpenCode provider unavailable (health): {}", error)))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(OyaError(format!(
            "OpenCode provider unavailable (health status {} at /session/status)",
            response.status().as_u16()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_opencode_cli_missing_output_detects_timeout_wrapper_message() {
        let output = "timeout: failed to run command 'opencode': No such file or directory";
        assert!(is_opencode_cli_missing_output(output));
    }

    #[test]
    fn test_is_opencode_cli_missing_output_ignores_unrelated_output() {
        let output = "command failed with exit code 1";
        assert!(!is_opencode_cli_missing_output(output));
    }

    #[test]
    fn test_is_opencode_missing_error_detects_not_found_variants() {
        assert!(is_opencode_missing_error("Command 'opencode' not found"));
        assert!(is_opencode_missing_error("opencode not found."));
        assert!(!is_opencode_missing_error("timeout after 300 seconds"));
    }

    #[test]
    fn test_is_rate_limited_cli_output_detects_common_messages() {
        assert!(is_rate_limited_cli_output("429 Too Many Requests"));
        assert!(is_rate_limited_cli_output("Provider is overloaded"));
        assert!(!is_rate_limited_cli_output("syntax error in prompt"));
    }

    #[test]
    fn test_rate_limit_backoff_is_exponential() {
        assert_eq!(rate_limit_backoff(0), std::time::Duration::from_millis(400));
        assert_eq!(rate_limit_backoff(1), std::time::Duration::from_millis(800));
        assert_eq!(rate_limit_backoff(2), std::time::Duration::from_millis(1600));
    }
}
