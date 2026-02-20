use crate::runtime_tools::{build_http_client, workflow_http_client_settings};

use super::types::{WorkflowConfig, WorkflowResult, WorkflowStatus};
use super::DynError;

pub(super) fn workflow_http_client() -> Result<reqwest::Client, DynError> {
    build_http_client(workflow_http_client_settings())
        .map_err(|error| format!("Failed to build HTTP client: {}", error).into())
}

pub(super) fn emit_workflow_starting(config: &WorkflowConfig) {
    eprintln!(
        "{}",
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
        })
    );
}

pub(super) fn emit_workflow_submitted(config: &WorkflowConfig) {
    eprintln!(
        "{}",
        serde_json::json!({
            "type": "workflow_submitted",
            "bead_id": config.bead_id,
            "run_id": config.run_id,
            "timeout_seconds": config.timeout_secs,
            "poll_interval_seconds": config.poll_interval_secs,
            "message": "Workflow submitted to Restate, polling for completion"
        })
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
        .map_err(|error| format!("Failed to start workflow: {}", error))?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body =
        response.text().await.map_or_else(|_| "<no body>".to_string(), std::convert::identity);
    Err(format!("Failed to start workflow (HTTP {}): {}", status, body).into())
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
        return Err(format!("Workflow timed out after {} seconds", config.timeout_secs).into());
    }
    let status = fetch_workflow_status(client, config).await?;
    print_stage_progress(config, &status, state.start_time, state.last_status.as_ref());
    if status.is_complete() {
        return Ok(Some(status));
    }
    if status.is_failed() {
        return Err(format!("Workflow failed: {}", status.last_failure).into());
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
    eprintln!(
        "{}",
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
        })
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
        .map_err(|error| format!("Query request failed: {}", error))?;
    let body = response.text().await.map_or_else(|_| "{}".to_string(), std::convert::identity);
    WorkflowStatus::from_query_response(&body).map_err(|error| -> DynError { error.into() })
}

pub(super) fn output_result(
    result: &WorkflowResult,
    config: &WorkflowConfig,
) -> Result<(), DynError> {
    println!(
        "{}",
        serde_json::json!({
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
                    {"action": "run_ci", "command": "moon run :ci", "description": "Run CI quality gates"},
                    {"action": "merge_workspace", "command": "zjj done", "description": "Merge zjj workspace to main"},
                    {"action": "close_bead", "command": format!("br close {}", result.bead_id), "description": "Close the bead issue"}
                ])
            } else {
                serde_json::json!([
                    {"action": "review_error", "description": "Review the error output above to understand the failure"},
                    {"action": "fix_issue", "path": format!("{}/src/", result.repo_root.display()), "description": "Fix the underlying issue in the source code"},
                    {"action": "rerun", "command": format!("oya run {}", result.bead_id), "description": "Re-run the workflow after fixing"}
                ])
            }
        })
    );
    Ok(())
}
