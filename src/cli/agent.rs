#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::process::{ExitStatus, Output};
use tokio::process::Command;

use crate::lifecycle::effects::run::{opencode_output_error_kind, OpencodeOutputErrorKind};
use crate::lifecycle::state::StateDb;
use crate::lifecycle::types::{
    EvidenceEnvelope, EvidenceEnvelopeParts, EvidenceKind, EvidenceMetadata, EvidenceRecordId,
};
use crate::restate_oya::OpencodeServerConfig;

pub(crate) const AGENT_OUTPUT_LIMIT_BYTES: usize = 4096;

const REDACTED_OUTPUT_LINE: &str = "[redacted]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentRunStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentFailureCategory {
    InvalidModel,
    ServerAuth,
    ServerConfig,
    ServerUnavailable,
    SubprocessUnavailable,
    AgentCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentRunMode {
    Subprocess,
    Server,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentRunResult {
    pub(crate) mode: AgentRunMode,
    pub(crate) status: AgentRunStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) failure_category: Option<AgentFailureCategory>,
    pub(crate) stdout: AgentOutputCapture,
    pub(crate) stderr: AgentOutputCapture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentOutputCapture {
    pub(crate) preview: String,
    pub(crate) original_bytes: usize,
    pub(crate) stored_bytes: usize,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentOutputSummary {
    pub original_bytes: usize,
    pub stored_bytes: usize,
    pub truncated: bool,
}

pub(crate) async fn run_opencode_subprocess(prompt: &str, model: &str) -> AgentRunResult {
    match Command::new("opencode")
        .arg("run")
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg(model)
        .arg(prompt)
        .output()
        .await
    {
        Ok(output) => AgentRunResult::from_output(output),
        Err(_) => AgentRunResult::subprocess_unavailable(),
    }
}

pub(crate) async fn run_opencode_server_if_configured(
    prompt: &str,
    model: &str,
) -> Option<AgentRunResult> {
    match OpencodeServerConfig::from_env() {
        Ok(Some(config)) => Some(run_opencode_server(&config, prompt, model).await),
        Ok(None) => None,
        Err(error) => Some(AgentRunResult::server_config_error(&error.to_string())),
    }
}

pub(crate) fn persist_agent_run(
    db: &StateDb,
    model: &str,
    agent_request: &EvidenceEnvelope,
    result: &AgentRunResult,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceEnvelope> {
    let envelope = agent_run_envelope(
        model,
        result,
        agent_run_timestamp(agent_request.timestamp, timestamp)?,
        agent_request,
    )?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

impl AgentRunStatus {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

impl AgentFailureCategory {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InvalidModel => "invalid_model",
            Self::ServerAuth => "server_auth",
            Self::ServerConfig => "server_config",
            Self::ServerUnavailable => "server_unavailable",
            Self::SubprocessUnavailable => "subprocess_unavailable",
            Self::AgentCommand => "agent_command",
        }
    }
}

impl AgentRunMode {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Subprocess => "subprocess",
            Self::Server => "server",
        }
    }
}

impl AgentRunResult {
    #[must_use]
    pub(crate) fn from_process(status: ExitStatus, stdout: &[u8], stderr: &[u8]) -> Self {
        let category = classify_agent_failure(status.success(), stdout, stderr);
        Self::from_parts(AgentRunMode::Subprocess, status.code(), category, stdout, stderr)
    }

    #[must_use]
    pub(crate) fn from_server_failure(status: u16, body: &[u8]) -> Self {
        Self::from_parts(
            AgentRunMode::Server,
            Some(i32::from(status)),
            Some(classify_server_failure(status, body)),
            b"",
            body,
        )
    }

    #[must_use]
    pub(crate) fn succeeded(&self) -> bool {
        self.status == AgentRunStatus::Succeeded
    }

    #[must_use]
    pub(crate) fn failure_category_name(&self) -> Option<&'static str> {
        self.failure_category.map(AgentFailureCategory::as_str)
    }

    #[must_use]
    pub(crate) fn failure_category_metadata(&self) -> &'static str {
        self.failure_category_name().map_or("none", |category| category)
    }

    #[must_use]
    pub(crate) fn exit_code_metadata(&self) -> String {
        self.exit_code.map_or_else(|| "none".to_owned(), |code| code.to_string())
    }

    #[must_use]
    pub(crate) fn sanitized_error(&self) -> Option<String> {
        self.failure_category.map(|category| match category {
            AgentFailureCategory::InvalidModel => "opencode invalid model".to_owned(),
            AgentFailureCategory::ServerAuth => "opencode server authentication failed".to_owned(),
            AgentFailureCategory::ServerConfig => "opencode server config invalid".to_owned(),
            AgentFailureCategory::ServerUnavailable => "opencode server unavailable".to_owned(),
            AgentFailureCategory::SubprocessUnavailable => {
                "opencode subprocess unavailable".to_owned()
            }
            AgentFailureCategory::AgentCommand => "opencode command failed".to_owned(),
        })
    }

    fn from_output(output: Output) -> Self {
        Self::from_process(output.status, &output.stdout, &output.stderr)
    }

    fn subprocess_unavailable() -> Self {
        Self::from_parts(
            AgentRunMode::Subprocess,
            None,
            Some(AgentFailureCategory::SubprocessUnavailable),
            b"",
            b"opencode subprocess unavailable",
        )
    }

    fn server_unavailable() -> Self {
        Self::from_parts(
            AgentRunMode::Server,
            None,
            Some(AgentFailureCategory::ServerUnavailable),
            b"",
            b"opencode server unavailable",
        )
    }

    fn server_config_error(message: &str) -> Self {
        Self::from_parts(
            AgentRunMode::Server,
            None,
            Some(AgentFailureCategory::ServerConfig),
            b"",
            message.as_bytes(),
        )
    }

    fn from_parts(
        mode: AgentRunMode,
        exit_code: Option<i32>,
        failure_category: Option<AgentFailureCategory>,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Self {
        Self {
            mode,
            status: agent_status(failure_category),
            exit_code,
            failure_category,
            stdout: AgentOutputCapture::from_bytes(stdout),
            stderr: AgentOutputCapture::from_bytes(stderr),
        }
    }
}

impl AgentOutputCapture {
    #[must_use]
    pub(crate) fn summary(&self) -> AgentOutputSummary {
        AgentOutputSummary {
            original_bytes: self.original_bytes,
            stored_bytes: self.stored_bytes,
            truncated: self.truncated,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let preview = bounded_redacted_preview(bytes);
        Self {
            preview: preview.clone(),
            original_bytes: bytes.len(),
            stored_bytes: preview.len(),
            truncated: bytes.len() > AGENT_OUTPUT_LIMIT_BYTES,
        }
    }
}

pub(crate) fn agent_output_metadata(
    prefix: &str,
    output: &AgentOutputCapture,
) -> [(String, String); 5] {
    [
        (format!("{prefix}_original_bytes"), output.original_bytes.to_string()),
        (format!("{prefix}_stored_bytes"), output.stored_bytes.to_string()),
        (format!("{prefix}_truncated"), output.truncated.to_string()),
        (format!("{prefix}_limit_bytes"), AGENT_OUTPUT_LIMIT_BYTES.to_string()),
        (format!("{prefix}_preview"), output.preview.clone()),
    ]
}

fn agent_run_envelope(
    model: &str,
    result: &AgentRunResult,
    timestamp: DateTime<Utc>,
    agent_request: &EvidenceEnvelope,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: agent_run_record_id(&agent_request.bead_id, timestamp)?,
        run_id: agent_request.run_id.clone(),
        bead_id: agent_request.bead_id.clone(),
        timestamp,
        kind: EvidenceKind::AgentRun,
        metadata: agent_run_metadata(model, result, agent_request, timestamp),
        previous_checksum: Some(agent_request.checksum.clone()),
    })
    .map_err(Into::into)
}

fn agent_run_timestamp(
    previous: DateTime<Utc>,
    requested: DateTime<Utc>,
) -> anyhow::Result<DateTime<Utc>> {
    if requested <= previous {
        previous
            .checked_add_signed(Duration::milliseconds(1))
            .ok_or_else(|| anyhow::anyhow!("agent run timestamp overflow"))
    } else {
        Ok(requested)
    }
}

fn agent_run_metadata(
    model: &str,
    result: &AgentRunResult,
    agent_request: &EvidenceEnvelope,
    timestamp: DateTime<Utc>,
) -> EvidenceMetadata {
    [
        ("agent".to_owned(), "opencode".to_owned()),
        ("duration_ms".to_owned(), agent_run_duration_ms(agent_request.timestamp, timestamp)),
        ("exit_code".to_owned(), result.exit_code_metadata()),
        ("failure_category".to_owned(), result.failure_category_metadata().to_owned()),
        ("mode".to_owned(), result.mode.as_str().to_owned()),
        ("model".to_owned(), model.to_owned()),
        ("redacted".to_owned(), "true".to_owned()),
        ("request_checksum".to_owned(), agent_request.checksum.as_str().to_owned()),
        ("request_record_id".to_owned(), agent_request.record_id.as_str().to_owned()),
        ("sanitized_message".to_owned(), agent_run_sanitized_message(result)),
        ("status".to_owned(), result.status.as_str().to_owned()),
    ]
    .into_iter()
    .chain(agent_output_metadata("stdout", &result.stdout))
    .chain(agent_output_metadata("stderr", &result.stderr))
    .collect()
}

fn agent_run_duration_ms(started_at: DateTime<Utc>, finished_at: DateTime<Utc>) -> String {
    finished_at.signed_duration_since(started_at).num_milliseconds().to_string()
}

fn agent_run_sanitized_message(result: &AgentRunResult) -> String {
    match result.sanitized_error() {
        Some(message) => message,
        None => "opencode completed".to_owned(),
    }
}

async fn run_opencode_server(
    config: &OpencodeServerConfig,
    prompt: &str,
    model: &str,
) -> AgentRunResult {
    let client = reqwest::Client::new();
    match create_server_session(config, prompt, &client).await {
        Ok(session_id) => send_server_message(config, &session_id, prompt, model, &client).await,
        Err(result) => result,
    }
}

async fn create_server_session(
    config: &OpencodeServerConfig,
    prompt: &str,
    client: &reqwest::Client,
) -> Result<String, AgentRunResult> {
    let url = format!("{}/session", config.url.trim_end_matches('/'));
    match client
        .post(url)
        .basic_auth(&config.username, Some(&config.password))
        .json(&serde_json::json!({ "title": format!("oya: {}", prompt.len().min(40)) }))
        .send()
        .await
    {
        Ok(response) => read_session_response(response).await,
        Err(_) => Err(AgentRunResult::server_unavailable()),
    }
}

async fn send_server_message(
    config: &OpencodeServerConfig,
    session_id: &str,
    prompt: &str,
    model: &str,
    client: &reqwest::Client,
) -> AgentRunResult {
    let url = format!("{}/session/{session_id}/message", config.url.trim_end_matches('/'));
    match client
        .post(url)
        .basic_auth(&config.username, Some(&config.password))
        .json(&server_message_body(prompt, model))
        .send()
        .await
    {
        Ok(response) => read_server_agent_response(response).await,
        Err(_) => AgentRunResult::server_unavailable(),
    }
}

async fn read_session_response(response: reqwest::Response) -> Result<String, AgentRunResult> {
    if !response.status().is_success() {
        return Err(server_failure_from_response(response).await);
    }
    match response.json::<serde_json::Value>().await {
        Ok(value) => value
            .get("id")
            .and_then(|id| id.as_str())
            .map(std::borrow::ToOwned::to_owned)
            .ok_or_else(AgentRunResult::server_unavailable),
        Err(_) => Err(AgentRunResult::server_unavailable()),
    }
}

async fn read_server_agent_response(response: reqwest::Response) -> AgentRunResult {
    if !response.status().is_success() {
        return server_failure_from_response(response).await;
    }
    match response.bytes().await {
        Ok(bytes) => AgentRunResult::from_parts(AgentRunMode::Server, Some(0), None, &bytes, b""),
        Err(_) => AgentRunResult::server_unavailable(),
    }
}

async fn server_failure_from_response(response: reqwest::Response) -> AgentRunResult {
    let status = response.status().as_u16();
    match response.bytes().await {
        Ok(bytes) => AgentRunResult::from_server_failure(status, &bytes),
        Err(_) => AgentRunResult::from_server_failure(status, b"opencode server failure"),
    }
}

fn server_message_body(prompt: &str, model: &str) -> serde_json::Value {
    let (provider_id, model_id) = model_provider_parts(model);
    serde_json::json!({
        "parts": [{"type": "text", "text": prompt}],
        "providerID": provider_id,
        "modelID": model_id,
    })
}

fn model_provider_parts(model: &str) -> (&str, &str) {
    match model.split_once('/') {
        Some((provider, id)) => (provider, id),
        None => ("anthropic", model),
    }
}

fn agent_run_record_id(
    bead_id: &crate::lifecycle::types::BeadId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-agent-run-{}",
        bead_id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn classify_agent_failure(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Option<AgentFailureCategory> {
    if let Some(kind) = opencode_error_kind_from_bytes(stdout, stderr) {
        Some(agent_failure_category(kind))
    } else if success {
        None
    } else {
        Some(AgentFailureCategory::AgentCommand)
    }
}

fn classify_server_failure(status: u16, body: &[u8]) -> AgentFailureCategory {
    if matches!(status, 401 | 403) {
        AgentFailureCategory::ServerAuth
    } else if let Some(kind) = opencode_error_kind_from_bytes(b"", body) {
        agent_failure_category(kind)
    } else {
        AgentFailureCategory::AgentCommand
    }
}

fn opencode_error_kind_from_bytes(stdout: &[u8], stderr: &[u8]) -> Option<OpencodeOutputErrorKind> {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    opencode_output_error_kind(&stdout, &stderr)
}

fn agent_failure_category(kind: OpencodeOutputErrorKind) -> AgentFailureCategory {
    match kind {
        OpencodeOutputErrorKind::InvalidModel => AgentFailureCategory::InvalidModel,
        OpencodeOutputErrorKind::AuthFailure => AgentFailureCategory::ServerAuth,
        OpencodeOutputErrorKind::ErrorEvent => AgentFailureCategory::AgentCommand,
    }
}

fn agent_status(category: Option<AgentFailureCategory>) -> AgentRunStatus {
    match category {
        Some(_) => AgentRunStatus::Failed,
        None => AgentRunStatus::Succeeded,
    }
}

fn bounded_redacted_preview(bytes: &[u8]) -> String {
    let limit = bytes.len().min(AGENT_OUTPUT_LIMIT_BYTES);
    let lossy = String::from_utf8_lossy(&bytes[..limit]);
    limit_text_to_bytes(&redact_output_preview(&lossy), AGENT_OUTPUT_LIMIT_BYTES)
}

fn redact_output_preview(input: &str) -> String {
    input.lines().map(redact_output_line).collect::<Vec<_>>().join("\n")
}

fn redact_output_line(line: &str) -> String {
    let normalized = line.to_ascii_lowercase();
    if is_sensitive_output_line(&normalized) {
        REDACTED_OUTPUT_LINE.to_owned()
    } else {
        line.to_owned()
    }
}

fn is_sensitive_output_line(normalized: &str) -> bool {
    ["token", "secret", "password", "api_key", "apikey"]
        .into_iter()
        .any(|needle| normalized.contains(needle))
        || is_stack_trace_output_line(normalized)
}

fn is_stack_trace_output_line(normalized: &str) -> bool {
    let trimmed = normalized.trim_start();
    normalized.contains("stack trace")
        || normalized.contains("traceback")
        || trimmed.starts_with("at ")
        || trimmed.starts_with("file ")
}

fn limit_text_to_bytes(input: &str, max_bytes: usize) -> String {
    let boundary = byte_boundary(input, max_bytes);
    match input.get(..boundary) {
        Some(value) => value.to_owned(),
        None => String::new(),
    }
}

fn byte_boundary(input: &str, max_bytes: usize) -> usize {
    if input.len() <= max_bytes {
        input.len()
    } else if input.is_char_boundary(max_bytes) {
        max_bytes
    } else {
        input
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index < max_bytes)
            .last()
            .map_or(0, |index| index)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{BeadId, RunId};

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    #[test]
    #[cfg(unix)]
    fn agent_result_classifies_invalid_model_without_leaking_secrets() {
        let stderr = b"ProviderModelNotFoundError: token=super-secret-token";
        let result = AgentRunResult::from_process(ExitStatus::from_raw(256), b"", stderr);

        assert_eq!(result.status, AgentRunStatus::Failed);
        assert_eq!(result.failure_category, Some(AgentFailureCategory::InvalidModel));
        assert_eq!(result.failure_category_metadata(), "invalid_model");
        assert_eq!(result.stderr.preview, "[redacted]");
        assert!(!result.stderr.preview.contains("super-secret-token"));
    }

    #[test]
    fn agent_result_classifies_server_auth_failure_without_leaking_password() {
        let result =
            AgentRunResult::from_server_failure(401, b"unauthorized: password=server-secret-token");

        assert_eq!(result.mode, AgentRunMode::Server);
        assert_eq!(result.status, AgentRunStatus::Failed);
        assert_eq!(result.failure_category, Some(AgentFailureCategory::ServerAuth));
        assert_eq!(result.failure_category_metadata(), "server_auth");
        assert_eq!(
            result.sanitized_error(),
            Some("opencode server authentication failed".to_owned())
        );
        assert_eq!(result.stderr.preview, "[redacted]");
        assert!(!result.stderr.preview.contains("server-secret-token"));
    }

    #[test]
    #[cfg(unix)]
    fn agent_result_uses_shared_opencode_error_predicate_for_subprocess() {
        let stdout = br#"{"type":"error","message":"provider failed"}"#;
        let result = AgentRunResult::from_process(ExitStatus::from_raw(0), stdout, b"");

        assert_eq!(result.mode, AgentRunMode::Subprocess);
        assert_eq!(result.status, AgentRunStatus::Failed);
        assert_eq!(result.failure_category, Some(AgentFailureCategory::AgentCommand));
    }

    #[test]
    fn agent_result_uses_shared_opencode_error_predicate_for_server_body() {
        let result = AgentRunResult::from_server_failure(500, b"Model not found: bad/model");

        assert_eq!(result.mode, AgentRunMode::Server);
        assert_eq!(result.status, AgentRunStatus::Failed);
        assert_eq!(result.failure_category, Some(AgentFailureCategory::InvalidModel));
        assert_eq!(result.sanitized_error(), Some("opencode invalid model".to_owned()));
    }

    #[test]
    #[cfg(unix)]
    fn opencode_no_secret_leak_redacts_provider_stack_trace_and_password() {
        let stderr = b"ProviderModelNotFoundError: password=server-secret-token\n    at Provider.request (/home/lewis/.cache/opencode/provider.js:42:7)\nTraceback (most recent call last):";
        let result = AgentRunResult::from_process(ExitStatus::from_raw(256), b"", stderr);
        let preview = result.stderr.preview;

        assert_eq!(result.failure_category, Some(AgentFailureCategory::InvalidModel));
        assert!(!preview.contains("server-secret-token"));
        assert!(!preview.contains("Provider.request"));
        assert!(!preview.contains("Traceback"));
        assert_eq!(preview, "[redacted]\n[redacted]\n[redacted]");
    }

    #[test]
    #[cfg(unix)]
    fn output_limit_bounds_agent_evidence_preview_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let request = agent_request_fixture();
        let stdout = oversized_agent_output("stdout-tail-marker");
        let stderr = b"password=server-secret-token";
        let result = AgentRunResult::from_process(ExitStatus::from_raw(0), &stdout, stderr);

        let evidence =
            persist_agent_run(&db, "zai-coding-plan/glm-5", &request, &result, request.timestamp)
                .unwrap();
        let json = evidence.to_canonical_json().unwrap();

        assert_eq!(evidence.metadata.get("stdout_truncated"), Some(&"true".to_owned()));
        assert_eq!(evidence.metadata.get("stdout_original_bytes"), Some(&stdout.len().to_string()));
        assert_eq!(
            evidence.metadata.get("stdout_stored_bytes"),
            Some(&AGENT_OUTPUT_LIMIT_BYTES.to_string())
        );
        assert_eq!(
            evidence.metadata.get("stdout_limit_bytes"),
            Some(&AGENT_OUTPUT_LIMIT_BYTES.to_string())
        );
        assert_eq!(evidence.metadata.get("stderr_preview"), Some(&"[redacted]".to_owned()));
        assert!(json.len() < AGENT_OUTPUT_LIMIT_BYTES * 3);
        assert!(!json.contains("stdout-tail-marker"));
        assert!(!json.contains("server-secret-token"));
    }

    fn agent_request_fixture() -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse("ev-demo-agent-request-001").unwrap(),
            run_id: RunId::parse("run-demo").unwrap(),
            bead_id: BeadId::parse("demo").unwrap(),
            timestamp: Utc::now(),
            kind: EvidenceKind::AgentRequest,
            metadata: EvidenceMetadata::from([("status".to_owned(), "requested".to_owned())]),
            previous_checksum: None,
        })
        .unwrap()
    }

    fn oversized_agent_output(marker: &str) -> Vec<u8> {
        let mut output = vec![b'a'; AGENT_OUTPUT_LIMIT_BYTES + 128];
        output.extend(marker.as_bytes());
        output
    }
}
