use super::*;
use std::process::Command;

#[derive(Debug, Clone)]
struct WorkflowConfig {
    bead_id: String,
    run_id: String,
    restate_ingress: String,
    restate_admin: String,
    context: String,
    model: String,
    timeout_secs: u64,
    poll_interval_secs: u64,
    repo_root: PathBuf,
    stages: &'static [&'static str],
}

impl WorkflowConfig {
    fn from_args(args: RunArgs, repo_root: PathBuf, oya_config: &config::OyaConfig) -> Self {
        let restate_ingress = args.restate_url.trim_end_matches('/').to_string();
        let restate_admin = restate_ingress.replace(":8080", ":9070");
        let model = args.model.unwrap_or_else(|| oya_config.model.clone());
        Self {
            run_id: args.bead_id.clone(),
            bead_id: args.bead_id,
            restate_ingress,
            restate_admin,
            context: args.context,
            model,
            timeout_secs: args.timeout,
            poll_interval_secs: args.poll_interval.unwrap_or(5),
            repo_root,
            stages: &["plan", "contract", "tdd15", "qa", "red_queen", "gpt_review", "ship_gate"],
        }
    }
}

#[derive(Debug, Clone)]
struct WorkflowStatus {
    status: String,
    stage: String,
    attempt: u32,
    orchestration_status: String,
    last_failure: String,
}

#[derive(Debug, Clone)]
struct WorkflowResult {
    bead_id: String,
    run_id: String,
    status: String,
    final_stage: String,
    error: Option<String>,
    repo_root: PathBuf,
}

impl WorkflowStatus {
    fn from_query_response(body: &str) -> Result<Self, String> {
        let response: serde_json::Value =
            serde_json::from_str(body).map_err(|e| format!("Invalid JSON response: {}", e))?;
        let rows = response
            .get("rows")
            .ok_or("Missing 'rows' field in response")?
            .as_array()
            .ok_or("'rows' field is not an array")?;
        let row = rows.first().ok_or("No rows in response")?;
        let status = row
            .get("status")
            .and_then(|s| s.as_str())
            .ok_or("Missing or invalid 'status' field")?
            .to_string();

        let state_json_str = row.get("state_json").and_then(|s| s.as_str()).unwrap_or("{}");
        let state_outer: serde_json::Value = serde_json::from_str(state_json_str)
            .map_err(|e| format!("Invalid state_json: {}", e))?;
        let state_str = state_outer.as_str().unwrap_or("{}");
        let state: serde_json::Value =
            serde_json::from_str(state_str).map_err(|e| format!("Invalid state string: {}", e))?;

        Ok(Self {
            status,
            stage: state.get("stage").and_then(|s| s.as_str()).unwrap_or("unknown").to_string(),
            attempt: state.get("attempt").and_then(|a| a.as_u64()).unwrap_or(0) as u32,
            orchestration_status: state
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string(),
            last_failure: state
                .get("last_failure")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    fn is_complete(&self) -> bool {
        self.status == "completed"
    }

    fn is_failed(&self) -> bool {
        self.status == "failed"
    }
}

fn find_repo_root() -> Result<PathBuf, String> {
    let current =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    std::iter::successors(Some(current.as_path()), |p| p.parent())
        .find(|p| p.join(".beads").exists())
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!("No .beads/ directory found in {} or any parent directory", current.display())
        })
}

fn validate_bead_exists(bead_id: &str, repo_root: &PathBuf) -> Result<bool, String> {
    let br_available = Command::new("which")
        .arg("br")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !br_available {
        return Err(
            "The 'br' command is not found. Please install it first: https://github.com/your-org/br"
                .to_string(),
        );
    }

    let output = Command::new("br")
        .args(["show", bead_id, "--json"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "The 'br' command is not found. Please install it first.".to_string()
            } else {
                format!("Failed to run 'br show {}': {}", bead_id, e)
            }
        })?;

    Ok(output.status.success())
}

pub(super) async fn run_workflow(args: RunArgs) -> Result<(), DynError> {
    let repo_root = find_repo_root()
        .map_err(|e| format!("Failed to find repo root (no .beads/ directory found): {}", e))?;

    if !validate_bead_exists(&args.bead_id, &repo_root)? {
        return Err(format!(
            "Bead '{}' not found. Run 'br list' to see available beads.",
            args.bead_id
        )
        .into());
    }

    let oya_config = config::load_config(&repo_root)
        .map_err(|err| format!("Failed to load oya config: {}", err))?;
    let workflow_config = WorkflowConfig::from_args(args, repo_root, &oya_config);
    execute_workflow(workflow_config).await
}

async fn execute_workflow(config: WorkflowConfig) -> Result<(), DynError> {
    let client = workflow_http_client()?;
    emit_workflow_starting(&config);
    start_workflow(&client, &config).await?;
    emit_workflow_submitted(&config);
    let final_status = poll_until_complete(&client, &config).await?;
    let result = workflow_result_from_status(&config, &final_status);
    output_result(&result, &config)?;
    if result.status == "shipped" {
        Ok(())
    } else {
        Err(format!("Workflow ended with status: {}", result.status).into())
    }
}

fn workflow_http_client() -> Result<reqwest::Client, DynError> {
    build_http_client(workflow_http_client_settings())
        .map_err(|error| format!("Failed to build HTTP client: {}", error).into())
}

fn emit_workflow_starting(config: &WorkflowConfig) {
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

fn emit_workflow_submitted(config: &WorkflowConfig) {
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

fn workflow_result_from_status(config: &WorkflowConfig, status: &WorkflowStatus) -> WorkflowResult {
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

async fn start_workflow(client: &reqwest::Client, config: &WorkflowConfig) -> Result<(), DynError> {
    let start_url =
        format!("{}/OyaOrchestrator/{}/start/send", config.restate_ingress, config.run_id);
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
struct PollState {
    start_time: std::time::Instant,
    timeout_duration: std::time::Duration,
    poll_interval: std::time::Duration,
    last_status: Option<WorkflowStatus>,
}

async fn poll_until_complete(
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
            "last_failure": if status.last_failure.is_empty() { serde_json::Value::Null } else { serde_json::json!(status.last_failure) },
            "pipeline_stages": config.stages,
            "repo_root": config.repo_root.display().to_string()
        })
    );
}

async fn fetch_workflow_status(
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
             and i.target_handler_name = 'start' \
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
    WorkflowStatus::from_query_response(&body).map_err(|error| error.into())
}

fn output_result(result: &WorkflowResult, config: &WorkflowConfig) -> Result<(), DynError> {
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
