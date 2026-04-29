#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::lifecycle::effects::run::opencode_output_is_error;
use crate::lifecycle::types::Model;
use restate_sdk::prelude::{HandlerError, TerminalError};
use serde_json::Value;
use std::fmt;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use super::trace::normalize_opencode_event;
use super::types::OpenCodeTraceEvent;

const OPENCODE_SERVER_PASSWORD_ENV: &str = "OPENCODE_SERVER_PASSWORD";
const OPENCODE_SERVER_URL_ENV: &str = "OPENCODE_SERVER_URL";
const OPENCODE_SERVER_USER_ENV: &str = "OPENCODE_SERVER_USER";
const DEFAULT_OPENCODE_SERVER_URL: &str = "http://localhost:4099";
const DEFAULT_OPENCODE_SERVER_USER: &str = "opencode";
const REDACTED_CONFIG_VALUE: &str = "[redacted]";

#[derive(Debug, Clone)]
pub struct Prompt(String);

impl Prompt {
    pub fn parse(raw: String) -> Result<Self, TerminalError> {
        let normalized = raw.trim().to_owned();
        if normalized.is_empty() {
            return Err(TerminalError::new("prompt cannot be empty"));
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

pub fn pipeline_prompt(bead_id: &str, bead_state: Value) -> Result<Prompt, TerminalError> {
    let state_json = serde_json::to_string_pretty(&bead_state)
        .map_err(|error| TerminalError::new(format!("invalid bead_state json: {error}")))?;
    Prompt::parse(format!(
        "Implement bead {bead_id} using this state from Restate.\n\nBead State:\n{state_json}\n\nSteps: 1) implement requested changes in repo, 2) use Moon for build/test/lint, 3) use OpenCode for agent execution, 4) use Git/GitHub for version-control and PR flow, 5) do not require any non-Git version-control tool, 6) summarize files changed and test result."
    ))
}

#[must_use]
pub fn model_or_default(value: Option<String>) -> Model {
    match value.and_then(|model| Model::parse(&model).ok()) {
        Some(model) => model,
        None => Model::default_model(),
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct OpencodeServerConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpencodeServerConfigError {
    EmptyUsername,
    EmptyPassword,
    InvalidEnvironment { name: &'static str },
    InvalidUrl { redacted_url: String },
}

impl OpencodeServerConfig {
    pub fn parse(
        url: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, OpencodeServerConfigError> {
        Ok(Self {
            url: parse_server_url(url)?,
            username: parse_server_username(username)?,
            password: parse_server_password(password)?,
        })
    }

    pub fn from_env() -> Result<Option<Self>, OpencodeServerConfigError> {
        let password = match read_env(OPENCODE_SERVER_PASSWORD_ENV)? {
            Some(value) => value,
            None => return Ok(None),
        };
        let url = match read_env(OPENCODE_SERVER_URL_ENV)? {
            Some(value) => value,
            None => DEFAULT_OPENCODE_SERVER_URL.to_owned(),
        };
        let username = match read_env(OPENCODE_SERVER_USER_ENV)? {
            Some(value) => value,
            None => DEFAULT_OPENCODE_SERVER_USER.to_owned(),
        };
        Self::parse(&url, &username, &password).map(Some)
    }
}

impl fmt::Debug for OpencodeServerConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpencodeServerConfig")
            .field("url", &redact_url_for_display(&self.url))
            .field("username", &self.username)
            .field("password", &REDACTED_CONFIG_VALUE)
            .finish()
    }
}

impl fmt::Display for OpencodeServerConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyUsername => formatter.write_str("opencode server username cannot be empty"),
            Self::EmptyPassword => formatter.write_str("opencode server password cannot be empty"),
            Self::InvalidEnvironment { name } => {
                write!(
                    formatter,
                    "opencode server environment variable {name} is not valid unicode"
                )
            }
            Self::InvalidUrl { redacted_url } => {
                write!(formatter, "opencode server url is invalid: {redacted_url}")
            }
        }
    }
}

impl std::error::Error for OpencodeServerConfigError {}

fn read_env(name: &'static str) -> Result<Option<String>, OpencodeServerConfigError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(OpencodeServerConfigError::InvalidEnvironment { name })
        }
    }
}

fn parse_server_url(value: &str) -> Result<String, OpencodeServerConfigError> {
    let trimmed = value.trim();
    let parsed = url::Url::parse(trimmed).map_err(|_| OpencodeServerConfigError::InvalidUrl {
        redacted_url: redact_url_for_display(trimmed),
    })?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        Err(OpencodeServerConfigError::InvalidUrl { redacted_url: redact_url_for_display(trimmed) })
    } else {
        Ok(trimmed.trim_end_matches('/').to_owned())
    }
}

fn parse_server_username(value: &str) -> Result<String, OpencodeServerConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(OpencodeServerConfigError::EmptyUsername)
    } else {
        Ok(trimmed.to_owned())
    }
}

fn parse_server_password(value: &str) -> Result<String, OpencodeServerConfigError> {
    if value.trim().is_empty() {
        Err(OpencodeServerConfigError::EmptyPassword)
    } else {
        Ok(value.to_owned())
    }
}

fn redact_url_for_display(value: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(value) else {
        return REDACTED_CONFIG_VALUE.to_owned();
    };
    if !parsed.username().is_empty() && parsed.set_username(REDACTED_CONFIG_VALUE).is_err() {
        return REDACTED_CONFIG_VALUE.to_owned();
    }
    if parsed.password().is_some() {
        let _ = parsed.set_password(Some(REDACTED_CONFIG_VALUE));
    }
    if parsed.query().is_some() {
        parsed.set_query(Some(REDACTED_CONFIG_VALUE));
    }
    if parsed.fragment().is_some() {
        parsed.set_fragment(Some(REDACTED_CONFIG_VALUE));
    }
    parsed.to_string()
}

#[derive(serde::Serialize)]
struct OpencodeMessageBody<'a> {
    parts: Vec<serde_json::Value>,
    #[serde(rename = "providerID")]
    provider_id: &'a str,
    #[serde(rename = "modelID")]
    model_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<&'a str>,
}

pub async fn run_opencode(prompt: Prompt, model: Model) -> Result<String, HandlerError> {
    match OpencodeServerConfig::from_env() {
        Ok(Some(config)) => run_opencode_server(&config, &prompt, &model, None).await,
        Ok(None) => run_opencode_subprocess(prompt, model).await,
        Err(error) => Err(TerminalError::new(error.to_string()).into()),
    }
}

pub async fn run_opencode_streaming<F>(
    prompt: Prompt,
    model: Model,
    on_event: F,
) -> Result<String, HandlerError>
where
    F: Fn(OpenCodeTraceEvent) + Send + Sync + 'static,
{
    match OpencodeServerConfig::from_env() {
        Ok(Some(_)) => run_opencode(prompt, model).await,
        Ok(None) => run_opencode_subprocess_streaming(prompt, model, on_event).await,
        Err(error) => Err(TerminalError::new(error.to_string()).into()),
    }
}

async fn run_opencode_server(
    config: &OpencodeServerConfig,
    prompt: &Prompt,
    model: &Model,
    cwd: Option<&str>,
) -> Result<String, HandlerError> {
    let client = reqwest::Client::new();
    let session_id = server_create_session(config, prompt, &client).await?;
    let message_body = build_message_body(prompt, model, cwd);
    server_send_message(config, &session_id, &message_body, &client).await
}

async fn server_create_session(
    config: &OpencodeServerConfig,
    prompt: &Prompt,
    client: &reqwest::Client,
) -> Result<String, HandlerError> {
    let session_url = format!("{}/session", config.url.trim_end_matches('/'));
    let session_resp = client
        .post(&session_url)
        .basic_auth(&config.username, Some(&config.password))
        .json(&create_session_body(prompt))
        .send()
        .await
        .map_err(|e| HandlerError::from(format!("opencode server session create failed: {e}")))?;
    parse_session_response(session_resp).await
}

fn create_session_body(prompt: &Prompt) -> serde_json::Value {
    serde_json::json!({
        "title": format!("restate-oya: {}", prompt.as_str().len().min(40))
    })
}

async fn parse_session_response(session_resp: reqwest::Response) -> Result<String, HandlerError> {
    if !session_resp.status().is_success() {
        let error_msg = read_error_body(session_resp).await;
        return Err(TerminalError::new(error_msg).into());
    }
    extract_session_id(session_resp).await
}

async fn read_error_body(resp: reqwest::Response) -> String {
    let status = resp.status();
    let body = match resp.text().await {
        Ok(text) => text,
        Err(e) => format!("unable to read response body: {e}"),
    };
    let body = body.chars().take(200).collect::<String>();
    format!("opencode server returned {status}: {body}")
}

async fn extract_session_id(session_resp: reqwest::Response) -> Result<String, HandlerError> {
    let session_data: serde_json::Value = session_resp
        .json()
        .await
        .map_err(|e| TerminalError::new(format!("failed to parse session response: {e}")))?;
    session_data
        .get("id")
        .and_then(|v| v.as_str())
        .map(std::borrow::ToOwned::to_owned)
        .ok_or_else(|| TerminalError::new("opencode server response missing session id").into())
}

fn build_message_body<'a>(
    prompt: &'a Prompt,
    model: &'a Model,
    cwd: Option<&'a str>,
) -> OpencodeMessageBody<'a> {
    let parts = model.as_str().split('/').collect::<Vec<_>>();
    let (provider_id, model_id) = match parts.as_slice() {
        [provider, model, ..] => (*provider, *model),
        [single] => ("anthropic", *single),
        [] => ("anthropic", "unknown"),
    };
    OpencodeMessageBody {
        parts: vec![serde_json::json!({"type": "text", "text": prompt.as_str()})],
        provider_id,
        model_id,
        cwd,
    }
}

async fn server_send_message(
    config: &OpencodeServerConfig,
    session_id: &str,
    message_body: &OpencodeMessageBody<'_>,
    client: &reqwest::Client,
) -> Result<String, HandlerError> {
    let msg_url = build_message_url(config, session_id);
    let msg_resp =
        send_opencode_request(client, &msg_url, &config.username, &config.password, message_body)
            .await?;
    read_message_response(msg_resp).await
}

fn build_message_url(config: &OpencodeServerConfig, session_id: &str) -> String {
    format!("{}/session/{}/message", config.url.trim_end_matches('/'), session_id)
}

async fn send_opencode_request(
    client: &reqwest::Client,
    url: &str,
    username: &str,
    password: &str,
    body: &OpencodeMessageBody<'_>,
) -> Result<reqwest::Response, HandlerError> {
    client
        .post(url)
        .basic_auth(username, Some(password))
        .json(body)
        .send()
        .await
        .map_err(|e| HandlerError::from(format!("opencode server message send failed: {e}")))
}

async fn read_message_response(msg_resp: reqwest::Response) -> Result<String, HandlerError> {
    let msg_status = msg_resp.status();
    let msg_body = msg_resp
        .text()
        .await
        .map_err(|e| HandlerError::from(format!("opencode server response read failed: {e}")))?;

    let result_str: String = msg_body.chars().take(4096).collect();

    if msg_status.is_success() && !opencode_output_is_error(&result_str, "") {
        Ok(result_str)
    } else {
        Err(TerminalError::new("opencode model not found or unavailable").into())
    }
}

async fn run_opencode_subprocess(prompt: Prompt, model: Model) -> Result<String, HandlerError> {
    let output = Command::new("opencode")
        .arg("run")
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg(model.as_str())
        .arg(prompt.into_inner())
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to run opencode: {error}")))?;
    parse_output(output)
}

async fn run_opencode_subprocess_streaming<F>(
    prompt: Prompt,
    model: Model,
    on_event: F,
) -> Result<String, HandlerError>
where
    F: Fn(OpenCodeTraceEvent) + Send + Sync + 'static,
{
    let mut child = spawn_opencode_child(prompt, model).await?;
    let stderr = child.stderr.take().map(read_stderr_task);
    let stdout = child.stdout.take().ok_or_else(|| {
        HandlerError::from("opencode subprocess stdout pipe was unavailable".to_owned())
    })?;
    let output = read_streaming_stdout(stdout, on_event).await?;
    let status = child
        .wait()
        .await
        .map_err(|error| HandlerError::from(format!("failed to wait for opencode: {error}")))?;
    let stderr = await_stderr(stderr).await;
    parse_process_result(output, stderr, status.success())
}

async fn spawn_opencode_child(
    prompt: Prompt,
    model: Model,
) -> Result<tokio::process::Child, HandlerError> {
    Command::new("opencode")
        .arg("run")
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg(model.as_str())
        .arg(prompt.into_inner())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| HandlerError::from(format!("failed to spawn opencode: {error}")))
}

fn read_stderr_task(stderr: tokio::process::ChildStderr) -> tokio::task::JoinHandle<String> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut buffer = String::new();
        match reader.read_to_string(&mut buffer).await {
            Ok(_) => buffer,
            Err(error) => format!("failed to read opencode stderr: {error}"),
        }
    })
}

async fn await_stderr(task: Option<tokio::task::JoinHandle<String>>) -> String {
    match task {
        Some(handle) => match handle.await {
            Ok(stderr) => stderr,
            Err(error) => format!("failed to join stderr reader: {error}"),
        },
        None => String::new(),
    }
}

async fn read_streaming_stdout<F>(
    stdout: tokio::process::ChildStdout,
    on_event: F,
) -> Result<String, HandlerError>
where
    F: Fn(OpenCodeTraceEvent) + Send + Sync + 'static,
{
    let mut lines = BufReader::new(stdout).lines();
    let mut output = Vec::new();
    let mut sequence = 0_u64;
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|error| HandlerError::from(format!("failed to read opencode stdout: {error}")))?
    {
        sequence += 1;
        emit_streaming_event(sequence, &line, &on_event);
        output.push(line);
    }
    Ok(output.join("\n"))
}

fn emit_streaming_event<F>(sequence: u64, line: &str, on_event: &F)
where
    F: Fn(OpenCodeTraceEvent) + Send + Sync + 'static,
{
    if let Ok(raw) = serde_json::from_str::<Value>(line) {
        let received_at = chrono::Utc::now().to_rfc3339();
        on_event(normalize_opencode_event(sequence, received_at, raw));
    }
}

fn parse_output(output: std::process::Output) -> Result<String, HandlerError> {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    parse_process_result(stdout, stderr, output.status.success())
}

fn parse_process_result(
    stdout: String,
    stderr: String,
    success: bool,
) -> Result<String, HandlerError> {
    if !success || opencode_output_is_error(&stdout, &stderr) {
        return Err(TerminalError::new("opencode model not found or unavailable").into());
    }

    if stdout.is_empty() {
        return Err(TerminalError::new("opencode output was empty or invalid UTF-8").into());
    }

    Ok(stdout)
}

pub async fn cancel_invocation(invocation_id: String) -> Result<(), HandlerError> {
    let output = Command::new("restate")
        .arg("invocations")
        .arg("cancel")
        .arg(&invocation_id)
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to invoke restate CLI: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(TerminalError::new(format!(
            "restate cancel failed for {invocation_id}: {}",
            stderr.trim()
        ))
        .into())
    }
}

pub async fn cancel_invocation_query(query: String) -> Result<String, HandlerError> {
    let output = Command::new("restate")
        .arg("invocations")
        .arg("cancel")
        .arg(&query)
        .arg("--kill")
        .arg("-y")
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to invoke restate CLI: {error}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        Ok(format!("cancelled workflow query {query}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr.contains("No invocations found for query")
            || stdout.contains("No invocations found for query")
        {
            Ok(format!("no running workflow invocations for {query}"))
        } else {
            Err(TerminalError::new(format!(
                "restate cancel failed for query {query}: {}",
                stderr.trim()
            ))
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{pipeline_prompt, OpencodeServerConfig};
    use serde_json::json;

    #[test]
    fn pipeline_prompt_guides_agents_to_git_only_version_control() {
        let Ok(prompt) = pipeline_prompt("prompt-bead", json!({"status":"ready"})) else {
            assert!(false, "pipeline prompt should build from JSON state");
            return;
        };
        let text = prompt.as_str();

        assert!(text.contains("use Moon for build/test/lint"));
        assert!(text.contains("use OpenCode for agent execution"));
        assert!(text.contains("use Git/GitHub for version-control and PR flow"));
        assert!(text.contains("do not require any non-Git version-control tool"));
        assert!(!text.contains("jj"));
        assert!(!text.contains("Jujutsu"));
        assert!(!text.contains("Use moon/jj/gh"));
    }

    #[test]
    fn opencode_server_config_redacts_secret_values_in_debug_output() {
        let config = OpencodeServerConfig::parse(
            "https://example.test:4099",
            "lewis",
            "server-secret-token",
        )
        .unwrap();

        let debug = format!("{config:?}");

        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("server-secret-token"));
        assert!(!debug.contains("token="));
    }

    #[test]
    fn opencode_server_config_redacts_secret_values_in_error_output() {
        let error = OpencodeServerConfig::parse(
            "ftp://user:server-secret-token@example.test:4099?token=server-secret-token",
            "lewis",
            "server-secret-token",
        )
        .unwrap_err();

        let message = error.to_string();

        assert!(message.contains("opencode server url is invalid"));
        assert!(message.contains("redacted"));
        assert!(!message.contains("server-secret-token"));
        assert!(!message.contains("token="));
    }

    #[test]
    fn opencode_server_config_rejects_missing_password_without_leaking_value() {
        let error =
            OpencodeServerConfig::parse("http://localhost:4099", "lewis", "  ").unwrap_err();

        assert_eq!(error.to_string(), "opencode server password cannot be empty");
    }
}
