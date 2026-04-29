#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::lifecycle::effects::run::opencode_output_is_error;
use crate::lifecycle::types::Model;
use restate_sdk::prelude::{HandlerError, TerminalError};
use serde_json::Value;
use tokio::process::Command;

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
        "Implement bead {bead_id} using this state from Restate.\n\nBead State:\n{state_json}\n\nSteps: 1) implement requested changes in repo, 2) run moon run :check, 3) summarize files changed and test result."
    ))
}

#[must_use]
pub fn model_or_default(value: Option<String>) -> Model {
    value.and_then(|m| Model::parse(&m).ok()).unwrap_or_else(Model::default_model)
}

#[derive(Debug, Clone)]
pub struct OpencodeServerConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

impl OpencodeServerConfig {
    pub fn from_env() -> Option<Self> {
        let password = std::env::var("OPENCODE_SERVER_PASSWORD").ok()?;
        let url = std::env::var("OPENCODE_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:4099".to_owned());
        let username =
            std::env::var("OPENCODE_SERVER_USER").unwrap_or_else(|_| "opencode".to_owned());
        Some(Self { url, username, password })
    }
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
    if let Some(config) = OpencodeServerConfig::from_env() {
        run_opencode_server(&config, &prompt, &model, None).await
    } else {
        run_opencode_subprocess(prompt, model).await
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

fn parse_output(output: std::process::Output) -> Result<String, HandlerError> {
    let stdout = String::from_utf8(output.stdout).unwrap_or_else(|_| String::new());
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() || opencode_output_is_error(&stdout, &stderr) {
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
