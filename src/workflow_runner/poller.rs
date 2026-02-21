use crate::runtime_tools::{build_http_client, workflow_http_client_settings};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;

use super::types::{WorkflowConfig, WorkflowResult, WorkflowStatus};
use super::DynError;

#[derive(Debug, Error)]
pub(super) enum WorkflowRunnerError {
    #[error("failed to build HTTP client: {0}")]
    HttpClientBuild(String),
    #[error("failed to start workflow request: {0}")]
    WorkflowStartRequest(String),
    #[error("failed to start workflow (HTTP {status}): {body}")]
    WorkflowStartRejected { status: u16, body: String },
    #[error("workflow timed out after {timeout_secs} seconds")]
    WorkflowTimeout { timeout_secs: u64 },
    #[error("workflow failed at stage {stage} attempt {attempt}: {message}")]
    WorkflowFailed { stage: String, attempt: u32, message: String },
    #[error("workflow query request failed: {0}")]
    WorkflowQueryRequest(String),
    #[error("workflow query response invalid: {0}")]
    WorkflowQueryResponse(String),
    #[error("bridge directory creation failed: {0}")]
    BridgeDirectoryCreate(String),
    #[error("bridge events file open failed: {0}")]
    BridgeOpen(String),
    #[error("bridge events serialization failed: {0}")]
    BridgeSerialize(String),
    #[error("bridge events newline write failed: {0}")]
    BridgeWrite(String),
}

impl WorkflowRunnerError {
    fn code(&self) -> &'static str {
        match self {
            Self::HttpClientBuild(_) => "http_client_build",
            Self::WorkflowStartRequest(_) => "workflow_start_request",
            Self::WorkflowStartRejected { .. } => "workflow_start_rejected",
            Self::WorkflowTimeout { .. } => "workflow_timeout",
            Self::WorkflowFailed { .. } => "workflow_failed",
            Self::WorkflowQueryRequest(_) => "workflow_query_request",
            Self::WorkflowQueryResponse(_) => "workflow_query_response",
            Self::BridgeDirectoryCreate(_) => "bridge_directory_create",
            Self::BridgeOpen(_) => "bridge_open",
            Self::BridgeSerialize(_) => "bridge_serialize",
            Self::BridgeWrite(_) => "bridge_write",
        }
    }
}

pub(super) fn workflow_http_client() -> Result<reqwest::Client, DynError> {
    build_http_client(workflow_http_client_settings())
        .map_err(|error| WorkflowRunnerError::HttpClientBuild(error.to_string()).into())
}

pub(super) fn emit_workflow_starting(config: &WorkflowConfig) {
    emit_event(
        config,
        serde_json::json!({
            "type": "workflow_starting",
            "bead_id": config.bead_id,
            "run_id": config.run_id,
            "context": config.context,
            "model": config.model,
            "repo_root": config.repo_root.display().to_string(),
            "restate_ingress": config.restate_ingress,
            "restate_admin": config.restate_admin,
            "timeout_seconds": config.timeout_secs,
            "poll_interval_seconds": config.poll_interval_secs,
            "pipeline_stages": config.stages,
            "tool": "oya",
            "action": "run"
        }),
    );
}

pub(super) fn emit_workflow_submitted(config: &WorkflowConfig) {
    emit_event(
        config,
        serde_json::json!({
            "type": "workflow_submitted",
            "bead_id": config.bead_id,
            "run_id": config.run_id,
            "timeout_seconds": config.timeout_secs,
            "poll_interval_seconds": config.poll_interval_secs,
            "message": "Workflow submitted to Restate, polling for completion"
        }),
    );
}

pub(super) fn workflow_result_from_status(
    config: &WorkflowConfig,
    status: &WorkflowStatus,
) -> WorkflowResult {
    WorkflowResult {
        bead_id: config.bead_id.clone(),
        run_id: config.run_id.clone(),
        status: status.orchestration_status.clone(),
        final_stage: status.stage.clone(),
        error: if status.last_failure.is_empty() {
            None
        } else {
            Some(status.last_failure.clone())
        },
        repo_root: config.repo_root.clone(),
    }
}

pub(super) async fn start_workflow(
    client: &reqwest::Client,
    config: &WorkflowConfig,
) -> Result<(), DynError> {
    let start_url =
        format!("{}/OyaOrchestrator/{}/run/send", config.restate_ingress, config.run_id);
    let payload = serde_json::json!({
        "bead_id": config.bead_id,
        "context": config.context,
        "model": config.model
    });

    let response = client
        .post(&start_url)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|error| WorkflowRunnerError::WorkflowStartRequest(error.to_string()))?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body =
        response.text().await.map_or_else(|_| "<no body>".to_string(), std::convert::identity);
    let error = WorkflowRunnerError::WorkflowStartRejected { status: status.as_u16(), body };
    emit_workflow_error(config, &error);
    Err(error.into())
}

#[derive(Clone)]
pub(super) struct PollState {
    pub(super) start_time: std::time::Instant,
    pub(super) timeout_duration: std::time::Duration,
    pub(super) poll_interval: std::time::Duration,
    pub(super) last_status: Option<WorkflowStatus>,
}

pub(super) async fn poll_until_complete(
    client: &reqwest::Client,
    config: &WorkflowConfig,
) -> Result<WorkflowStatus, DynError> {
    let mut state = PollState {
        start_time: std::time::Instant::now(),
        timeout_duration: std::time::Duration::from_secs(config.timeout_secs),
        poll_interval: std::time::Duration::from_secs(config.poll_interval_secs),
        last_status: None,
    };
    loop {
        if let Some(done) = poll_iteration(client, config, &mut state).await? {
            return Ok(done);
        }
        tokio::time::sleep(state.poll_interval).await;
    }
}

async fn poll_iteration(
    client: &reqwest::Client,
    config: &WorkflowConfig,
    state: &mut PollState,
) -> Result<Option<WorkflowStatus>, DynError> {
    if state.start_time.elapsed() > state.timeout_duration {
        let error = WorkflowRunnerError::WorkflowTimeout { timeout_secs: config.timeout_secs };
        emit_workflow_error(config, &error);
        return Err(error.into());
    }
    let status = fetch_workflow_status(client, config).await?;
    print_stage_progress(config, &status, state.start_time, state.last_status.as_ref());
    if status.is_complete() {
        return Ok(Some(status));
    }
    if status.is_failed() {
        let error = WorkflowRunnerError::WorkflowFailed {
            stage: status.stage.clone(),
            attempt: status.attempt,
            message: status.last_failure.clone(),
        };
        emit_workflow_error(config, &error);
        return Err(error.into());
    }
    state.last_status = Some(status);
    Ok(None)
}

fn print_stage_progress(
    config: &WorkflowConfig,
    status: &WorkflowStatus,
    start_time: std::time::Instant,
    last_status: Option<&WorkflowStatus>,
) {
    let status_changed = last_status.is_none_or(|last| {
        last.status != status.status || last.stage != status.stage || last.attempt != status.attempt
    });
    if !status_changed {
        return;
    }
    let elapsed_secs = start_time.elapsed().as_secs();
    emit_event(
        config,
        serde_json::json!({
            "type": "stage_progress",
            "bead_id": config.bead_id,
            "run_id": config.run_id,
            "invocation_status": status.status,
            "orchestration_status": status.orchestration_status,
            "current_stage": status.stage,
            "attempt": status.attempt,
            "elapsed_seconds": elapsed_secs,
            "remaining_seconds": config.timeout_secs.saturating_sub(elapsed_secs),
            "last_failure": if status.last_failure.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!(status.last_failure)
            },
            "pipeline_stages": config.stages,
            "repo_root": config.repo_root.display().to_string()
        }),
    );
}

pub(super) async fn fetch_workflow_status(
    client: &reqwest::Client,
    config: &WorkflowConfig,
) -> Result<WorkflowStatus, DynError> {
    let query_payload = serde_json::json!({
        "query": format!(
            "select i.status, s.value_utf8 as state_json from sys_invocation i \
             left join state s on s.service_name = i.target_service_name \
             and s.service_key = i.target_service_key and s.key = 'state' \
              where i.target_service_name = 'OyaOrchestrator' \
              and i.target_service_key = '{}' \
              and i.target_handler_name = 'run' \
             order by i.modified_at desc limit 1",
            config.run_id
        )
    });
    let response = client
        .post(format!("{}/query", config.restate_admin))
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .json(&query_payload)
        .send()
        .await
        .map_err(|error| WorkflowRunnerError::WorkflowQueryRequest(error.to_string()))?;
    let body = response.text().await.map_or_else(|_| "{}".to_string(), std::convert::identity);
    WorkflowStatus::from_query_response(&body)
        .map_err(|error| WorkflowRunnerError::WorkflowQueryResponse(error).into())
}

pub(super) fn output_result(
    result: &WorkflowResult,
    config: &WorkflowConfig,
) -> Result<(), DynError> {
    let event = serde_json::json!({
        "type": "workflow_result",
        "bead_id": result.bead_id,
        "run_id": result.run_id,
        "status": result.status,
        "final_stage": result.final_stage,
        "error": result.error,
        "repo_root": result.repo_root.display().to_string(),
        "pipeline_stages": config.stages,
        "is_success": result.status == "shipped",
        "next_steps": if result.status == "shipped" {
            serde_json::json!([
                {"action": "review_code", "path": format!("{}/src/", result.repo_root.display()), "description": "Review generated source code"},
                {"action": "verify_landing", "description": "Confirm landing commands completed in ShipGate logs"},
                {"action": "inspect_timeline", "description": "Review timeline and stage artifacts in Restate admin"}
            ])
        } else {
            serde_json::json!([
                {"action": "review_error", "description": "Review the error output above to understand the failure"},
                {"action": "fix_issue", "path": format!("{}/src/", result.repo_root.display()), "description": "Fix the underlying issue in the source code"},
                {"action": "rerun", "command": format!("oya run {}", result.bead_id), "description": "Re-run the workflow after fixing"}
            ])
        }
    });
    println!("{}", event);
    append_bridge_event(config, &event)?;
    Ok(())
}

fn emit_event(config: &WorkflowConfig, event: serde_json::Value) {
    eprintln!("{}", event);
    if let Err(error) = append_bridge_event(config, &event) {
        eprintln!(
            "{}",
            serde_json::json!({
                "type": "bridge_write_error",
                "run_id": config.run_id,
                "bead_id": config.bead_id,
                "message": error.to_string()
            })
        );
    }
}

fn emit_workflow_error(config: &WorkflowConfig, error: &WorkflowRunnerError) {
    emit_event(
        config,
        serde_json::json!({
            "type": "workflow_error",
            "bead_id": config.bead_id,
            "run_id": config.run_id,
            "error_code": error.code(),
            "message": error.to_string(),
        }),
    );
}

fn append_bridge_event(config: &WorkflowConfig, event: &serde_json::Value) -> Result<(), DynError> {
    let path = bridge_events_path(config);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| WorkflowRunnerError::BridgeDirectoryCreate(error.to_string()))?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| WorkflowRunnerError::BridgeOpen(error.to_string()))?;

    serde_json::to_writer(&mut file, event)
        .map_err(|error| WorkflowRunnerError::BridgeSerialize(error.to_string()))?;
    file.write_all(b"\n").map_err(|error| WorkflowRunnerError::BridgeWrite(error.to_string()))?;
    Ok(())
}

fn bridge_events_path(config: &WorkflowConfig) -> PathBuf {
    config.repo_root.join(".oya").join("bridge").join("events.jsonl")
}
