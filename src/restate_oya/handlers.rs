use crate::lifecycle::effects::TokioCommandExecutor;
use crate::lifecycle::types::{BeadId, BeadStatus, CancelState};
use crate::lifecycle::workflow::{
    run_lifecycle_with_progress, LifecycleProgressUpdate, LifecycleRunRequest, LifecycleStepStatus,
};
use restate_sdk::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{LazyLock, RwLock};
use tokio::process::Command;

use super::opencode::{
    cancel_invocation, cancel_invocation_query, model_or_default, pipeline_prompt, run_opencode,
};
use super::trace::{build_clean_trace, fallback_summary, parse_jsonl_events, summarize_events};
use super::types::{
    BeadSnapshot, BeadSyncRequest, CancelResponse, KeyRequest, LifecycleRequest,
    LifecycleStatusSnapshot, LifecycleStepSnapshot, MemorySnapshot, PipelineRequest, StartRequest,
    StartResponse,
};

#[restate_sdk::object]
trait OyaMemory {
    async fn start(req: Json<StartRequest>) -> Result<Json<StartResponse>, HandlerError>;
    async fn sync_bead(req: Json<BeadSyncRequest>) -> Result<Json<StartResponse>, HandlerError>;
    async fn run_pipeline(req: Json<PipelineRequest>) -> Result<Json<StartResponse>, HandlerError>;
    #[shared]
    async fn get_state() -> Result<Json<MemorySnapshot>, HandlerError>;
    #[shared]
    async fn get_bead() -> Result<Json<BeadSnapshot>, HandlerError>;
    async fn request_cancel() -> Result<Json<CancelResponse>, HandlerError>;
}

pub struct OyaMemoryBridge;

#[restate_sdk::service]
trait OyaService {
    async fn get_state(req: Json<KeyRequest>) -> Result<Json<MemorySnapshot>, HandlerError>;
    async fn get_bead(req: Json<KeyRequest>) -> Result<Json<BeadSnapshot>, HandlerError>;
    async fn get_lifecycle(
        req: Json<KeyRequest>,
    ) -> Result<Json<LifecycleStatusSnapshot>, HandlerError>;
    async fn cancel(req: Json<KeyRequest>) -> Result<Json<CancelResponse>, HandlerError>;
}

pub struct OyaServiceBridge;

#[restate_sdk::workflow]
trait Oya {
    async fn run(req: Json<LifecycleRequest>) -> Result<Json<StartResponse>, HandlerError>;
    #[shared]
    async fn status() -> Result<Json<LifecycleStatusSnapshot>, HandlerError>;
}

pub struct OyaBridge;

static RUNTIME_LIFECYCLE_STATUS: LazyLock<RwLock<HashMap<String, LifecycleStatusSnapshot>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

impl Oya for OyaBridge {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: Json<LifecycleRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let body = req.into_inner();
        let workflow_key = ctx.key().to_owned();
        let initial_steps = default_step_snapshots();
        let requested_bead_id = body.bead_id.clone();
        initialize_lifecycle_status(&ctx, requested_bead_id.clone(), &initial_steps);
        seed_runtime_status(&workflow_key, requested_bead_id, &initial_steps);
        let mut live_steps: Vec<LifecycleStepSnapshot> = Vec::new();
        let result = run_lifecycle_with_progress(
            &TokioCommandExecutor,
            LifecycleRunRequest { bead_id: body.bead_id, model: body.model },
            |update| {
                let update_clone = update.clone();
                apply_progress_update(&ctx, &mut live_steps, update);
                update_runtime_progress(&workflow_key, &live_steps, update_clone);
            },
        )
        .await;
        match result {
            Ok(outcome) => {
                store_lifecycle_state(&ctx, &outcome.state)?;
                serialize_workflow_outcome(&outcome).map(Into::into)
            }
            Err(failure) => {
                if let Some(state) = &failure.state {
                    store_lifecycle_state(&ctx, state)?;
                }
                let message = serde_json::to_string(&failure).map_err(|error| {
                    HandlerError::from(format!("failed to serialize lifecycle failure: {error}"))
                })?;
                Err(TerminalError::new(message).into())
            }
        }
    }

    async fn status(
        &self,
        ctx: SharedWorkflowContext<'_>,
    ) -> Result<Json<LifecycleStatusSnapshot>, HandlerError> {
        read_lifecycle_status(&ctx).await.map(Into::into)
    }
}

impl OyaMemory for OyaMemoryBridge {
    async fn start(
        &self,
        ctx: ObjectContext<'_>,
        req: Json<StartRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let body = req.into_inner();
        persist_bead_state(&ctx, &body);
        let prompt = super::opencode::Prompt::parse(body.prompt).map_err(HandlerError::from)?;
        let model = model_or_default(body.model);
        let output = ctx.run(move || run_opencode(prompt, model)).name("opencode_run").await?;
        store_output(&ctx, &output);
        Ok(StartResponse { output }.into())
    }

    async fn sync_bead(
        &self,
        ctx: ObjectContext<'_>,
        req: Json<BeadSyncRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let bead = req.into_inner();
        ctx.set("bead_id", bead.bead_id.clone());
        ctx.set("bead_status", bead.bead_status.clone());
        ctx.set("bead_state", Json::from(bead.bead_state));
        let output = format!("synced bead {}", bead.bead_id);
        Ok(StartResponse { output }.into())
    }

    async fn run_pipeline(
        &self,
        ctx: ObjectContext<'_>,
        req: Json<PipelineRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let cancel_state = ctx
            .get::<String>("cancel_state")
            .await?
            .and_then(parse_cancel_state)
            .unwrap_or_default();
        if cancel_state.is_cancel_requested() {
            return Err(TerminalError::new("cancel requested before pipeline run").into());
        }
        let model = model_or_default(req.into_inner().model);
        ctx.set("active_invocation_id", ctx.invocation_id().to_owned());
        ctx.set("cancel_state", "active".to_owned());
        let bead_id = require_state_string(&ctx, "bead_id").await?;
        let bead_state = require_state_json(&ctx, "bead_state").await?;
        let prompt = pipeline_prompt(&bead_id, bead_state)?;
        let output = ctx.run(move || run_opencode(prompt, model)).name("opencode_pipeline").await?;
        store_output(&ctx, &output);
        ctx.clear("active_invocation_id");
        Ok(StartResponse { output }.into())
    }

    async fn get_state(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<MemorySnapshot>, HandlerError> {
        let bead = BeadSnapshot {
            bead_id: ctx
                .get::<String>("bead_id")
                .await?
                .and_then(|value| BeadId::parse(&value).ok()),
            bead_status: ctx
                .get::<String>("bead_status")
                .await?
                .and_then(|value| BeadStatus::parse(&value).ok()),
            bead_state: ctx.get::<Json<Value>>("bead_state").await?.map(Json::into_inner),
        };
        let snapshot = MemorySnapshot {
            bead,
            last_output_summary: ctx
                .get::<Json<Value>>("last_output_summary")
                .await?
                .map(Json::into_inner),
            last_output_trace: ctx
                .get::<Json<Value>>("last_output_trace")
                .await?
                .map(Json::into_inner),
            active_invocation_id: ctx.get::<String>("active_invocation_id").await?,
            cancel_state: ctx
                .get::<String>("cancel_state")
                .await?
                .and_then(parse_cancel_state)
                .unwrap_or_default(),
        };
        Ok(snapshot.into())
    }

    async fn get_bead(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BeadSnapshot>, HandlerError> {
        Ok(BeadSnapshot {
            bead_id: ctx
                .get::<String>("bead_id")
                .await?
                .and_then(|value| BeadId::parse(&value).ok()),
            bead_status: ctx
                .get::<String>("bead_status")
                .await?
                .and_then(|value| BeadStatus::parse(&value).ok()),
            bead_state: ctx.get::<Json<Value>>("bead_state").await?.map(Json::into_inner),
        }
        .into())
    }

    async fn request_cancel(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<CancelResponse>, HandlerError> {
        ctx.set("cancel_state", "cancel_requested".to_owned());
        let active_invocation_id = ctx.get::<String>("active_invocation_id").await?;
        match active_invocation_id {
            Some(invocation_id) => {
                let cancel_id = invocation_id.clone();
                let cancel_result =
                    ctx.run(move || cancel_invocation(cancel_id)).name("cancel_invocation").await;
                match cancel_result {
                    Ok(()) => Ok(CancelResponse {
                        cancelled: true,
                        message: format!("cancel requested for invocation {invocation_id}"),
                    }
                    .into()),
                    Err(error) => Ok(CancelResponse {
                        cancelled: false,
                        message: format!("failed to cancel invocation {invocation_id}: {error}"),
                    }
                    .into()),
                }
            }
            None => Ok(CancelResponse {
                cancelled: false,
                message: "no active invocation to cancel".to_owned(),
            }
            .into()),
        }
    }
}

impl OyaService for OyaServiceBridge {
    async fn get_state(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<MemorySnapshot>, HandlerError> {
        let key = req.into_inner().key;
        ctx.object_client::<OyaMemoryClient>(&key).get_state().call().await.map_err(Into::into)
    }

    async fn get_bead(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<BeadSnapshot>, HandlerError> {
        let key = req.into_inner().key;
        ctx.object_client::<OyaMemoryClient>(&key).get_bead().call().await.map_err(Into::into)
    }

    async fn get_lifecycle(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<LifecycleStatusSnapshot>, HandlerError> {
        let key = req.into_inner().key;
        if let Some(snapshot) = get_runtime_status(&key) {
            return Ok(snapshot.into());
        }
        let workflow_key = workflow_key_for_service_key(&key);
        if let Some(snapshot) = read_workflow_status(&ctx, &workflow_key).await {
            return Ok(snapshot.into());
        }
        let run_key = workflow_key.clone();
        let raw =
            ctx.run(move || fetch_lifecycle_status_raw(run_key)).name("get_lifecycle").await?;
        let snapshot = parse_lifecycle_status_snapshot(&raw, &key);
        Ok(snapshot.into())
    }

    async fn cancel(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<CancelResponse>, HandlerError> {
        let key = req.into_inner().key;
        let memory_result =
            ctx.object_client::<OyaMemoryClient>(&key).request_cancel().call().await;
        let workflow_query = format!("Oya/{key}/run");
        let workflow_result =
            ctx.run(move || cancel_invocation_query(workflow_query)).name("cancel_workflow").await;
        let cleanup_targets = cleanup_targets_for_key(&key);
        let cleanup_result = ctx
            .run(move || forget_workspace_for_targets(cleanup_targets))
            .name("cleanup_workspace")
            .await;
        Ok(compose_cancel_response(memory_result, workflow_result, cleanup_result).into())
    }
}

fn compose_cancel_response(
    memory_result: Result<Json<CancelResponse>, TerminalError>,
    workflow_result: Result<String, TerminalError>,
    cleanup_result: Result<String, TerminalError>,
) -> CancelResponse {
    let (memory_cancelled, memory_message) = memory_outcome(memory_result);
    let (workflow_cancelled, workflow_message) = workflow_outcome(workflow_result);
    let cleanup_message = cleanup_outcome(cleanup_result);
    CancelResponse {
        cancelled: memory_cancelled || workflow_cancelled,
        message: format!("{}; {}; {}", memory_message, workflow_message, cleanup_message),
    }
}

fn memory_outcome(result: Result<Json<CancelResponse>, TerminalError>) -> (bool, String) {
    match result {
        Ok(memory) => {
            let memory = memory.into_inner();
            (memory.cancelled, memory.message)
        }
        Err(error) => (false, format!("memory cancel error: {:?}", error)),
    }
}

fn workflow_outcome(result: Result<String, TerminalError>) -> (bool, String) {
    match result {
        Ok(message) => (message.starts_with("cancelled"), message),
        Err(error) => (false, format!("workflow cancel error: {:?}", error)),
    }
}

fn cleanup_outcome(result: Result<String, TerminalError>) -> String {
    match result {
        Ok(message) => message,
        Err(error) => format!("cleanup error: {:?}", error),
    }
}

fn get_runtime_status(key: &str) -> Option<LifecycleStatusSnapshot> {
    RUNTIME_LIFECYCLE_STATUS.read().ok().and_then(|map| {
        runtime_lookup_keys(key).into_iter().find_map(|candidate| map.get(&candidate).cloned())
    })
}

async fn read_workflow_status(
    ctx: &Context<'_>,
    workflow_key: &str,
) -> Option<LifecycleStatusSnapshot> {
    ctx.workflow_client::<OyaClient>(workflow_key).status().call().await.ok().map(|snapshot| {
        let status = snapshot.into_inner();
        seed_runtime_status(workflow_key, status.bead_id.clone(), status.steps.as_slice());
        status
    })
}

fn workflow_key_for_service_key(key: &str) -> String {
    key.strip_prefix("Oya/")
        .and_then(|value| value.strip_suffix("/run"))
        .map_or_else(|| key.to_owned(), std::borrow::ToOwned::to_owned)
}

fn update_runtime_progress(
    key: &str,
    live_steps: &[LifecycleStepSnapshot],
    update: LifecycleProgressUpdate,
) {
    if let Ok(mut map) = RUNTIME_LIFECYCLE_STATUS.write() {
        let current = runtime_lookup_keys(key)
            .into_iter()
            .find_map(|candidate| map.get(&candidate).cloned())
            .unwrap_or_else(|| LifecycleStatusSnapshot {
                bead_id: Some(key.to_owned()),
                steps: Vec::new(),
                state: None,
                pr_url: None,
                done: false,
                success: None,
                message: None,
            });
        let next = runtime_status_next(current, live_steps, update);
        insert_runtime_status(&mut map, key, next);
    }
}

fn insert_runtime_status(
    map: &mut HashMap<String, LifecycleStatusSnapshot>,
    workflow_key: &str,
    snapshot: LifecycleStatusSnapshot,
) {
    runtime_store_keys(workflow_key, snapshot.bead_id.as_deref()).into_iter().for_each(
        |candidate| {
            map.insert(candidate, snapshot.clone());
        },
    );
}

fn seed_runtime_status(
    workflow_key: &str,
    bead_id: Option<String>,
    steps: &[LifecycleStepSnapshot],
) {
    if let Ok(mut map) = RUNTIME_LIFECYCLE_STATUS.write() {
        insert_runtime_status(
            &mut map,
            workflow_key,
            LifecycleStatusSnapshot {
                bead_id,
                steps: steps.to_vec(),
                state: None,
                pr_url: None,
                done: false,
                success: None,
                message: None,
            },
        );
    }
}

fn runtime_store_keys(workflow_key: &str, bead_id: Option<&str>) -> Vec<String> {
    let mut keys = runtime_lookup_keys(workflow_key);
    if let Some(id) = bead_id {
        keys = keys.into_iter().chain(runtime_lookup_keys(id)).collect::<Vec<_>>();
    }
    keys.sort();
    keys.dedup();
    keys
}

fn runtime_lookup_keys(key: &str) -> Vec<String> {
    let normalized = key.strip_prefix("Oya/").and_then(|value| value.strip_suffix("/run"));
    match normalized {
        Some(inner) => vec![key.to_owned(), inner.to_owned()],
        None => vec![key.to_owned(), format!("Oya/{key}/run")],
    }
}

fn runtime_status_next(
    current: LifecycleStatusSnapshot,
    live_steps: &[LifecycleStepSnapshot],
    update: LifecycleProgressUpdate,
) -> LifecycleStatusSnapshot {
    match update {
        LifecycleProgressUpdate::Initialized { bead_id, .. } => LifecycleStatusSnapshot {
            bead_id: Some(bead_id),
            steps: live_steps.to_vec(),
            state: current.state,
            pr_url: current.pr_url,
            done: false,
            success: None,
            message: None,
        },
        LifecycleProgressUpdate::Step { message, .. } => LifecycleStatusSnapshot {
            bead_id: current.bead_id,
            steps: live_steps.to_vec(),
            state: current.state,
            pr_url: current.pr_url,
            done: false,
            success: None,
            message,
        },
        LifecycleProgressUpdate::Finished { success, pr_url, message } => LifecycleStatusSnapshot {
            bead_id: current.bead_id,
            steps: live_steps.to_vec(),
            state: current.state,
            pr_url,
            done: true,
            success: Some(success),
            message,
        },
    }
}

async fn forget_workspace_for_key(key: String) -> Result<String, HandlerError> {
    let workspace = format!("oya-{key}");
    let output =
        Command::new("jj").arg("workspace").arg("forget").arg(&workspace).output().await.map_err(
            |error| HandlerError::from(format!("failed to run jj workspace forget: {error}")),
        )?;
    if output.status.success() {
        Ok(format!("workspace cleanup attempted for {workspace}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr.contains("No such workspace") {
            Ok(format!("workspace {workspace} not present"))
        } else {
            Err(HandlerError::from(format!("workspace cleanup failed: {}", stderr.trim())))
        }
    }
}

fn cleanup_targets_for_key(key: &str) -> Vec<String> {
    let mut targets = vec![key.to_owned()];
    if let Some(status) = get_runtime_status(key) {
        if let Some(bead_id) = status.bead_id {
            if bead_id != key {
                targets.push(bead_id);
            }
        }
    }
    targets
}

async fn forget_workspace_for_targets(targets: Vec<String>) -> Result<String, HandlerError> {
    let mut messages = Vec::new();
    for target in targets {
        messages.push(forget_workspace_for_key(target).await?);
    }
    Ok(messages.join("; "))
}

pub async fn serve(bind: SocketAddr) -> anyhow::Result<()> {
    let endpoint = Endpoint::builder()
        .bind(OyaMemoryBridge.serve())
        .bind(OyaBridge.serve())
        .bind(OyaServiceBridge.serve())
        .build();
    HttpServer::new(endpoint).listen_and_serve(bind).await;
    Ok(())
}

fn serialize_workflow_outcome(
    outcome: &crate::lifecycle::workflow::LifecycleRunOutcome,
) -> Result<StartResponse, HandlerError> {
    let output = serde_json::to_string(outcome).map_err(|error| {
        HandlerError::from(format!("failed to serialize lifecycle outcome: {error}"))
    })?;
    Ok(StartResponse { output })
}

fn initialize_lifecycle_status(
    ctx: &WorkflowContext<'_>,
    bead_id: Option<String>,
    steps: &[LifecycleStepSnapshot],
) {
    ctx.set("lifecycle_bead_id", bead_id);
    store_lifecycle_steps(ctx, steps);
    ctx.clear("lifecycle_state");
    ctx.clear("lifecycle_pr_url");
    ctx.set("lifecycle_done", false);
    ctx.clear("lifecycle_success");
    ctx.clear("lifecycle_message");
}

fn default_step_snapshots() -> Vec<LifecycleStepSnapshot> {
    [
        "mark_in_progress",
        "workspace_add",
        "opencode",
        "moon_ci",
        "bookmark_create",
        "bookmark_push",
        "pr_create",
    ]
    .into_iter()
    .map(|step| LifecycleStepSnapshot {
        step: step.to_owned(),
        status: lifecycle_status_label(&LifecycleStepStatus::Pending).to_owned(),
        message: None,
    })
    .collect()
}

fn apply_progress_update(
    ctx: &WorkflowContext<'_>,
    live_steps: &mut Vec<LifecycleStepSnapshot>,
    update: LifecycleProgressUpdate,
) {
    match update {
        LifecycleProgressUpdate::Initialized { bead_id, steps } => {
            *live_steps = steps
                .into_iter()
                .map(|step| LifecycleStepSnapshot {
                    step,
                    status: lifecycle_status_label(&LifecycleStepStatus::Pending).to_owned(),
                    message: None,
                })
                .collect::<Vec<_>>();
            ctx.set("lifecycle_bead_id", Some(bead_id));
            store_lifecycle_steps(ctx, live_steps);
            ctx.set("lifecycle_message", Option::<String>::None);
        }
        LifecycleProgressUpdate::Step { step, status, message } => {
            *live_steps = upsert_step(live_steps.clone(), step, status, message);
            store_lifecycle_steps(ctx, live_steps);
        }
        LifecycleProgressUpdate::Finished { success, pr_url, message } => {
            ctx.set("lifecycle_done", true);
            ctx.set("lifecycle_success", Some(success));
            ctx.set("lifecycle_pr_url", pr_url);
            ctx.set("lifecycle_message", message);
        }
    }
}

fn store_lifecycle_steps(ctx: &WorkflowContext<'_>, steps: &[LifecycleStepSnapshot]) {
    if let Ok(value) = serde_json::to_value(steps) {
        ctx.set("lifecycle_steps", Json::from(value));
    }
}

fn lifecycle_status_label(status: &LifecycleStepStatus) -> &'static str {
    match status {
        LifecycleStepStatus::Pending => "pending",
        LifecycleStepStatus::Running => "running",
        LifecycleStepStatus::Succeeded => "succeeded",
        LifecycleStepStatus::Failed => "failed",
    }
}

fn upsert_step(
    steps: Vec<LifecycleStepSnapshot>,
    step: String,
    status: LifecycleStepStatus,
    message: Option<String>,
) -> Vec<LifecycleStepSnapshot> {
    let mut found = false;
    let mapped = steps
        .into_iter()
        .map(|item| {
            if item.step == step {
                found = true;
                LifecycleStepSnapshot {
                    step: item.step,
                    status: lifecycle_status_label(&status).to_owned(),
                    message: message.clone(),
                }
            } else {
                item
            }
        })
        .collect::<Vec<_>>();
    if found {
        mapped
    } else {
        mapped
            .into_iter()
            .chain(std::iter::once(LifecycleStepSnapshot {
                step,
                status: lifecycle_status_label(&status).to_owned(),
                message,
            }))
            .collect()
    }
}

fn store_lifecycle_state(
    ctx: &WorkflowContext<'_>,
    state: &crate::lifecycle::types::LifecycleState,
) -> Result<(), HandlerError> {
    let value = serde_json::to_value(state).map_err(|error| {
        HandlerError::from(format!("failed to serialize lifecycle state: {error}"))
    })?;
    let pr_url = extract_pr_url_from_state(state);
    ctx.set("lifecycle_state", Json::from(value));
    ctx.set("lifecycle_pr_url", pr_url);
    Ok(())
}

fn extract_pr_url_from_state(state: &crate::lifecycle::types::LifecycleState) -> Option<String> {
    match &state.phase {
        crate::lifecycle::types::Phase::PrOpen { pr, .. } => Some(pr.url.clone()),
        crate::lifecycle::types::Phase::Completed(result) => {
            result.pr.as_ref().map(|pr| pr.url.clone())
        }
        crate::lifecycle::types::Phase::Planned(_)
        | crate::lifecycle::types::Phase::WorkspaceReady(_)
        | crate::lifecycle::types::Phase::Failed { .. } => None,
    }
}

async fn read_lifecycle_status(
    ctx: &SharedWorkflowContext<'_>,
) -> Result<LifecycleStatusSnapshot, HandlerError> {
    let steps = ctx
        .get::<Json<Value>>("lifecycle_steps")
        .await?
        .map(Json::into_inner)
        .and_then(|value| serde_json::from_value::<Vec<LifecycleStepSnapshot>>(value).ok())
        .unwrap_or_default();
    let state = ctx
        .get::<Json<Value>>("lifecycle_state")
        .await?
        .map(Json::into_inner)
        .and_then(|value| if value.is_null() { None } else { Some(value) });
    Ok(LifecycleStatusSnapshot {
        bead_id: ctx.get::<String>("lifecycle_bead_id").await?,
        steps,
        state,
        pr_url: ctx.get::<String>("lifecycle_pr_url").await?,
        done: ctx.get::<bool>("lifecycle_done").await?.unwrap_or(false),
        success: ctx.get::<bool>("lifecycle_success").await?,
        message: ctx.get::<String>("lifecycle_message").await?,
    })
}

async fn fetch_lifecycle_status_raw(key: String) -> Result<String, HandlerError> {
    let list_output = run_invocation_list(&key).await?;
    if !list_output.status.success() {
        return Ok(String::from_utf8_lossy(&list_output.stderr).into_owned());
    }
    let list_stdout = String::from_utf8_lossy(&list_output.stdout).into_owned();
    match extract_invocation_id(&list_stdout) {
        Some(invocation_id) => run_invocation_describe(invocation_id)
            .await
            .map(|describe_text| format!("{list_stdout}\n---DETAIL---\n{describe_text}")),
        None => Ok(list_stdout),
    }
}

async fn run_invocation_list(key: &str) -> Result<std::process::Output, HandlerError> {
    Command::new("restate")
        .arg("invocations")
        .arg("list")
        .arg("--service")
        .arg("Oya")
        .arg("--key")
        .arg(key)
        .arg("--handler")
        .arg("run")
        .arg("--limit")
        .arg("1")
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to query lifecycle status: {error}")))
}

async fn run_invocation_describe(invocation_id: &str) -> Result<String, HandlerError> {
    let output = Command::new("restate")
        .arg("invocations")
        .arg("describe")
        .arg(invocation_id)
        .output()
        .await
        .map_err(|error| {
            HandlerError::from(format!("failed to describe lifecycle invocation: {error}"))
        })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Ok(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

fn extract_invocation_id(text: &str) -> Option<&str> {
    text.split_whitespace().find(|token| token.starts_with("inv_"))
}

fn extract_step_snapshots(raw: &str) -> Vec<LifecycleStepSnapshot> {
    raw.lines()
        .filter_map(|line| {
            let command = line.split("Command:").nth(1)?.trim();
            let step = command.split_whitespace().next().unwrap_or(command).to_owned();
            Some(LifecycleStepSnapshot {
                step,
                status: "seen".to_owned(),
                message: Some(command.to_owned()),
            })
        })
        .collect()
}

fn parse_lifecycle_status_snapshot(raw: &str, key: &str) -> LifecycleStatusSnapshot {
    let is_running = raw.contains("Status:") && raw.contains("running");
    let is_backing_off = raw.contains("Status:") && raw.contains("backing-off");
    let message = extract_status_line(raw).or_else(|| {
        if raw.trim().is_empty() {
            Some("status unavailable".to_owned())
        } else {
            Some(raw.trim().to_owned())
        }
    });
    LifecycleStatusSnapshot {
        bead_id: Some(key.to_owned()),
        steps: extract_step_snapshots(raw),
        state: None,
        pr_url: extract_pr_url(raw),
        done: !(is_running || is_backing_off),
        success: if is_running || is_backing_off { None } else { Some(!raw.contains("Error:")) },
        message,
    }
}

fn extract_status_line(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("Status:"))
        .map(std::borrow::ToOwned::to_owned)
}

fn extract_pr_url(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .find(|token| token.starts_with("https://") && token.contains("/pull/"))
        .map(std::borrow::ToOwned::to_owned)
}

fn persist_bead_state(ctx: &ObjectContext<'_>, request: &StartRequest) {
    if let Some(bead_id) = &request.bead_id {
        ctx.set("bead_id", bead_id.clone());
    }
    if let Some(bead_status) = &request.bead_status {
        ctx.set("bead_status", bead_status.clone());
    }
    if let Some(bead_state) = &request.bead_state {
        ctx.set("bead_state", Json::from(bead_state.clone()));
    }
}

fn store_output(ctx: &ObjectContext<'_>, output: &str) {
    ctx.clear("last_output");
    ctx.clear("last_output_events");
    if let Ok(events) = parse_jsonl_events(output) {
        ctx.set("last_output_summary", Json::from(summarize_events(&events)));
        ctx.set("last_output_trace", Json::from(build_clean_trace(&events)));
    } else {
        ctx.set("last_output_summary", Json::from(fallback_summary(output)));
        ctx.set("last_output_trace", Json::from(Vec::<Value>::new()));
    }
}

fn parse_cancel_state(value: String) -> Option<CancelState> {
    match value.trim().to_lowercase().as_str() {
        "active" => Some(CancelState::Active),
        "cancel_requested" => Some(CancelState::CancelRequested),
        _ => None,
    }
}

async fn require_state_string(ctx: &ObjectContext<'_>, key: &str) -> Result<String, HandlerError> {
    ctx.get::<String>(key)
        .await?
        .ok_or_else(|| TerminalError::new(format!("missing state key: {key}")).into())
}

async fn require_state_json(ctx: &ObjectContext<'_>, key: &str) -> Result<Value, HandlerError> {
    ctx.get::<Json<Value>>(key)
        .await?
        .map(Json::into_inner)
        .ok_or_else(|| TerminalError::new(format!("missing state key: {key}")).into())
}
