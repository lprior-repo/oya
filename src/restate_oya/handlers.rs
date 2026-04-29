#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
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
    cancel_invocation, cancel_invocation_query, model_or_default, pipeline_prompt,
    run_opencode_streaming,
};
use super::trace::{
    apply_trace_event, build_clean_trace, empty_trace_snapshot, fallback_summary, finalize_trace,
    parse_jsonl_events, summarize_events,
};
use super::types::{
    BeadSnapshot, BeadSyncRequest, CancelResponse, KeyRequest, LifecycleGateSnapshot,
    LifecycleRequest, LifecycleStatusSnapshot, LifecycleStepSnapshot, MemorySnapshot,
    OpenCodeTraceEvent, OpenCodeTraceSnapshot, PipelineRequest, StartRequest, StartResponse,
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
    async fn get_opencode_trace(
        req: Json<KeyRequest>,
    ) -> Result<Json<OpenCodeTraceSnapshot>, HandlerError>;
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

static STATE_DB: std::sync::OnceLock<crate::lifecycle::state::StateDb> = std::sync::OnceLock::new();

pub fn init_state_db(db: crate::lifecycle::state::StateDb) {
    let _ = STATE_DB.set(db);
}

static RUNTIME_LIFECYCLE_STATUS: LazyLock<RwLock<HashMap<String, LifecycleStatusSnapshot>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static RUNTIME_OPENCODE_TRACE: LazyLock<RwLock<HashMap<String, OpenCodeTraceSnapshot>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

impl Oya for OyaBridge {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: Json<LifecycleRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let mut body = req.into_inner();
        let workflow_key = ctx.key().to_owned();
        validate_runtime_key(&workflow_key)?;
        body.bead_id = validate_optional_bead_id(body.bead_id)?;
        let initial_steps = default_step_snapshots();
        let requested_bead_id = body.bead_id.clone();
        initialize_lifecycle_status(&ctx, requested_bead_id.clone(), &initial_steps);
        seed_runtime_status(&workflow_key, requested_bead_id, &initial_steps);
        let mut live_steps: Vec<LifecycleStepSnapshot> = Vec::new();
        let result = run_lifecycle_with_progress(
            &TokioCommandExecutor,
            LifecycleRunRequest { bead_id: body.bead_id, model: body.model, repo: body.repo },
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
        let mut body = req.into_inner();
        body.bead_id = validate_optional_bead_id(body.bead_id)?;
        persist_bead_state(&ctx, &body);
        let prompt = super::opencode::Prompt::parse(body.prompt).map_err(HandlerError::from)?;
        let model = model_or_default(body.model);
        let trace_key = ctx.key().to_owned();
        let model_label = model.as_str().to_owned();
        seed_opencode_trace(&trace_key, body.bead_id, ctx.invocation_id(), &model_label);
        let event_key = trace_key.clone();
        let output_result = ctx
            .run(move || {
                run_opencode_streaming(prompt, model, move |event| {
                    append_opencode_trace_event(&event_key, event);
                })
            })
            .name("opencode_run")
            .await;
        let output = finalize_opencode_run(&trace_key, output_result)?;
        store_output(&ctx, &output);
        flush_memory_state(&ctx).await?;
        Ok(StartResponse { output }.into())
    }

    async fn sync_bead(
        &self,
        ctx: ObjectContext<'_>,
        req: Json<BeadSyncRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let mut bead = req.into_inner();
        bead.bead_id = validate_bead_id(bead.bead_id)?;
        ctx.set("bead_id", bead.bead_id.clone());
        ctx.set("bead_status", bead.bead_status.clone());
        ctx.set("bead_state", Json::from(bead.bead_state));
        flush_memory_state(&ctx).await?;
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
        let bead_id = validate_bead_id(require_state_string(&ctx, "bead_id").await?)?;
        let bead_state = require_state_json(&ctx, "bead_state").await?;
        let prompt = pipeline_prompt(&bead_id, bead_state)?;
        let trace_key = ctx.key().to_owned();
        let model_label = model.as_str().to_owned();
        seed_opencode_trace(&trace_key, Some(bead_id.clone()), ctx.invocation_id(), &model_label);
        let event_key = trace_key.clone();
        let output_result = ctx
            .run(move || {
                run_opencode_streaming(prompt, model, move |event| {
                    append_opencode_trace_event(&event_key, event);
                })
            })
            .name("opencode_pipeline")
            .await;
        let output = finalize_opencode_run(&trace_key, output_result)?;
        store_output(&ctx, &output);
        ctx.clear("active_invocation_id");
        flush_memory_state(&ctx).await?;
        Ok(StartResponse { output }.into())
    }

    async fn get_state(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<MemorySnapshot>, HandlerError> {
        if let Some(db) = STATE_DB.get() {
            if let Ok(bead_id) = BeadId::parse(ctx.key()) {
                if let Ok(Some(json)) = db.load_memory(&bead_id) {
                    if let Ok(snapshot) = serde_json::from_str::<MemorySnapshot>(&json) {
                        return Ok(snapshot.into());
                    }
                }
            }
        }

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
        flush_memory_state(&ctx).await?;
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
        validate_runtime_key(&key)?;
        ctx.object_client::<OyaMemoryClient>(&key).get_state().call().await.map_err(Into::into)
    }

    async fn get_bead(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<BeadSnapshot>, HandlerError> {
        let key = req.into_inner().key;
        validate_runtime_key(&key)?;
        ctx.object_client::<OyaMemoryClient>(&key).get_bead().call().await.map_err(Into::into)
    }

    async fn get_lifecycle(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<LifecycleStatusSnapshot>, HandlerError> {
        let key = req.into_inner().key;
        validate_runtime_key(&key)?;
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
        if is_lifecycle_not_found(&raw) {
            return Err(TerminalError::new(format!(
                "not_found: lifecycle '{}' does not exist",
                key
            ))
            .into());
        }
        let snapshot = parse_lifecycle_status_snapshot(&raw, &key);
        Ok(snapshot.into())
    }

    async fn get_opencode_trace(
        &self,
        _ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<OpenCodeTraceSnapshot>, HandlerError> {
        let key = req.into_inner().key;
        validate_runtime_key(&key)?;
        Ok(get_opencode_trace_snapshot(&key).into())
    }

    async fn cancel(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<CancelResponse>, HandlerError> {
        let key = req.into_inner().key;
        validate_runtime_key(&key)?;
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

fn validate_runtime_key(key: &str) -> Result<(), HandlerError> {
    if is_safe_runtime_key(key) {
        Ok(())
    } else {
        Err(TerminalError::new(format!("invalid key: '{}'", key)).into())
    }
}

fn is_safe_runtime_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
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
    if let Some(db) = STATE_DB.get() {
        for candidate in runtime_lookup_keys(key) {
            if let Ok(Some(json)) = db.load_status(&candidate) {
                if let Ok(snapshot) = serde_json::from_str::<LifecycleStatusSnapshot>(&json) {
                    if !is_uninitialized_workflow_snapshot(&snapshot) {
                        return Some(snapshot);
                    }
                }
            }
        }
    }
    RUNTIME_LIFECYCLE_STATUS.read().ok().and_then(|map| {
        runtime_lookup_keys(key).into_iter().find_map(|candidate| {
            map.get(&candidate)
                .cloned()
                .filter(|snapshot| !is_uninitialized_workflow_snapshot(snapshot))
        })
    })
}

fn seed_opencode_trace(key: &str, bead_id: Option<String>, invocation_id: &str, model: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let snapshot = OpenCodeTraceSnapshot {
        bead_id,
        workflow_key: key.to_owned(),
        active_invocation_id: Some(invocation_id.to_owned()),
        model: Some(model.to_owned()),
        started_at: Some(now.clone()),
        updated_at: Some(now),
        finished_at: None,
        status: "running".to_owned(),
        current_event: None,
        events: Vec::new(),
        tool_call_count: 0,
        text_event_count: 0,
        last_error: None,
        summary: None,
    };
    store_opencode_trace(key, snapshot);
}

fn append_opencode_trace_event(key: &str, event: OpenCodeTraceEvent) {
    let current = get_opencode_trace_snapshot(key);
    store_opencode_trace(key, apply_trace_event(current, event));
}

fn finalize_opencode_run(
    key: &str,
    result: Result<String, TerminalError>,
) -> Result<String, TerminalError> {
    match result {
        Ok(output) => {
            let summary = parse_jsonl_events(&output).ok().map(|events| summarize_events(&events));
            finalize_opencode_trace(key, true, None, summary);
            Ok(output)
        }
        Err(error) => {
            finalize_opencode_trace(key, false, Some(format!("{error:?}")), None);
            Err(error)
        }
    }
}

fn finalize_opencode_trace(
    key: &str,
    success: bool,
    last_error: Option<String>,
    summary: Option<Value>,
) {
    let current = get_opencode_trace_snapshot(key);
    let finished_at = chrono::Utc::now().to_rfc3339();
    let next = finalize_trace(current, success, finished_at, last_error, summary);
    store_opencode_trace(key, next);
}

fn get_opencode_trace_snapshot(key: &str) -> OpenCodeTraceSnapshot {
    RUNTIME_OPENCODE_TRACE
        .read()
        .ok()
        .and_then(|map| {
            runtime_lookup_keys(key).into_iter().find_map(|candidate| map.get(&candidate).cloned())
        })
        .unwrap_or_else(|| empty_trace_snapshot(key))
}

fn store_opencode_trace(key: &str, snapshot: OpenCodeTraceSnapshot) {
    if let Ok(mut map) = RUNTIME_OPENCODE_TRACE.write() {
        runtime_store_keys(key, snapshot.bead_id.as_deref()).into_iter().for_each(|candidate| {
            map.insert(candidate, snapshot.clone());
        });
    }
}

async fn read_workflow_status(
    ctx: &Context<'_>,
    workflow_key: &str,
) -> Option<LifecycleStatusSnapshot> {
    ctx.workflow_client::<OyaClient>(workflow_key)
        .status()
        .call()
        .await
        .ok()
        .map(Json::into_inner)
        .and_then(|status| {
            if is_uninitialized_workflow_snapshot(&status) {
                None
            } else {
                seed_runtime_status(workflow_key, status.bead_id.clone(), status.steps.as_slice());
                Some(status)
            }
        })
}

fn is_uninitialized_workflow_snapshot(status: &LifecycleStatusSnapshot) -> bool {
    status.steps.is_empty()
        && status.state.is_none()
        && status.pr_url.is_none()
        && !status.done
        && status.success.is_none()
        && status.message.is_none()
        && status.compensation_diagnostics.is_empty()
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
    if let Some(db) = STATE_DB.get() {
        if let Ok(bead_id) = BeadId::parse(key) {
            if let Ok(json) = serde_json::to_string(&update) {
                let _ = db.append_journal(&bead_id, &json);
                let _ = db.flush();
            }
        }
    }

    if let Ok(mut map) = RUNTIME_LIFECYCLE_STATUS.write() {
        let current = runtime_lookup_keys(key)
            .into_iter()
            .find_map(|candidate| map.get(&candidate).cloned())
            .unwrap_or_else(|| LifecycleStatusSnapshot {
                bead_id: Some(key.to_owned()),
                steps: Vec::new(),
                gates: default_gate_snapshots(),
                discipline_gates: default_discipline_gate_snapshots(),
                state: None,
                pr_url: None,
                done: false,
                success: None,
                message: None,
                compensation_diagnostics: Vec::new(),
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
    let keys = runtime_store_keys(workflow_key, snapshot.bead_id.as_deref());
    if let Some(db) = STATE_DB.get() {
        if let Ok(json) = serde_json::to_string(&snapshot) {
            for key in &keys {
                let _ = db.persist_status(key, &json);
            }
            let _ = db.flush();
        }
    }
    keys.into_iter().for_each(|candidate| {
        map.insert(candidate, snapshot.clone());
    });
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
                gates: gate_snapshots_from_steps(steps),
                discipline_gates: discipline_gate_snapshots_from_steps(steps),
                state: None,
                pr_url: None,
                done: false,
                success: None,
                message: None,
                compensation_diagnostics: Vec::new(),
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
    let base = runtime_status_base(current, live_steps);
    match update {
        LifecycleProgressUpdate::Initialized { bead_id, .. } => {
            LifecycleStatusSnapshot { bead_id: Some(bead_id), message: None, ..base }
        }
        LifecycleProgressUpdate::Step { message, .. } => {
            LifecycleStatusSnapshot { message, ..base }
        }
        LifecycleProgressUpdate::Finished {
            success,
            pr_url,
            message,
            compensation_diagnostics,
        } => LifecycleStatusSnapshot {
            pr_url,
            done: true,
            success: Some(success),
            message,
            compensation_diagnostics,
            ..base
        },
    }
}

fn runtime_status_base(
    current: LifecycleStatusSnapshot,
    live_steps: &[LifecycleStepSnapshot],
) -> LifecycleStatusSnapshot {
    LifecycleStatusSnapshot {
        bead_id: current.bead_id,
        steps: live_steps.to_vec(),
        gates: gate_snapshots_from_steps(live_steps),
        discipline_gates: discipline_gate_snapshots_from_steps(live_steps),
        state: current.state,
        pr_url: current.pr_url,
        done: false,
        success: None,
        message: None,
        compensation_diagnostics: current.compensation_diagnostics,
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
    store_lifecycle_gates(ctx, &default_gate_snapshots());
    store_lifecycle_discipline_gates(ctx, &default_discipline_gate_snapshots());
    ctx.clear("lifecycle_state");
    ctx.clear("lifecycle_pr_url");
    ctx.set("lifecycle_done", false);
    ctx.clear("lifecycle_success");
    ctx.clear("lifecycle_message");
    store_compensation_diagnostics(ctx, &[]);
}

fn default_step_snapshots() -> Vec<LifecycleStepSnapshot> {
    [
        "mark_in_progress",
        "workspace_prepare",
        "workspace_add",
        "opencode",
        "qa_enforcer",
        "ltc_quick",
        "ltc_targeted",
        "ltc_test",
        "moon_ci",
        "jj_sync_main",
        "jj_rebase_main",
        "jj_track",
        "jj_describe",
        "validate_changes",
        "bookmark_create",
        "bookmark_push",
        "pr_create",
    ]
    .into_iter()
    .map(|step| LifecycleStepSnapshot {
        step: step.to_owned(),
        status: lifecycle_status_label(&LifecycleStepStatus::Pending).to_owned(),
        message: None,
        details: None,
        started_at: None,
        finished_at: None,
        duration_ms: None,
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
            apply_initialized_update(ctx, live_steps, bead_id, steps);
        }
        LifecycleProgressUpdate::Step {
            step,
            status,
            message,
            details,
            started_at,
            finished_at,
            duration_ms,
        } => {
            apply_step_update(
                ctx,
                live_steps,
                StepUpdate { step, status, message, details, started_at, finished_at, duration_ms },
            );
        }
        LifecycleProgressUpdate::Finished {
            success,
            pr_url,
            message,
            compensation_diagnostics,
        } => {
            apply_finished_update(ctx, success, pr_url, message, compensation_diagnostics);
        }
    }
}

fn apply_initialized_update(
    ctx: &WorkflowContext<'_>,
    live_steps: &mut Vec<LifecycleStepSnapshot>,
    bead_id: String,
    steps: Vec<String>,
) {
    *live_steps = steps.into_iter().map(make_pending_snapshot).collect::<Vec<_>>();
    ctx.set("lifecycle_bead_id", Some(bead_id));
    store_lifecycle_steps(ctx, live_steps);
    ctx.set("lifecycle_message", Option::<String>::None);
}

fn make_pending_snapshot(step: String) -> LifecycleStepSnapshot {
    LifecycleStepSnapshot {
        step,
        status: lifecycle_status_label(&LifecycleStepStatus::Pending).to_owned(),
        message: None,
        details: None,
        started_at: None,
        finished_at: None,
        duration_ms: None,
    }
}

fn apply_step_update(
    ctx: &WorkflowContext<'_>,
    live_steps: &mut Vec<LifecycleStepSnapshot>,
    update: StepUpdate,
) {
    *live_steps = upsert_step(live_steps.clone(), update);
    store_lifecycle_steps(ctx, live_steps);
    store_lifecycle_gates(ctx, &gate_snapshots_from_steps(live_steps));
    store_lifecycle_discipline_gates(ctx, &discipline_gate_snapshots_from_steps(live_steps));
}

fn gate_snapshots_from_steps(steps: &[LifecycleStepSnapshot]) -> Vec<LifecycleGateSnapshot> {
    apply_gate_updates(default_gate_snapshots(), steps, gate_for_step)
}

fn discipline_gate_snapshots_from_steps(
    steps: &[LifecycleStepSnapshot],
) -> Vec<LifecycleGateSnapshot> {
    apply_gate_updates(default_discipline_gate_snapshots(), steps, discipline_gate_for_step)
}

fn apply_gate_updates(
    gates: Vec<LifecycleGateSnapshot>,
    steps: &[LifecycleStepSnapshot],
    mapper: fn(&str) -> Option<&'static str>,
) -> Vec<LifecycleGateSnapshot> {
    steps.iter().fold(gates, |acc, step| {
        mapper(&step.step).map_or(acc.clone(), |gate_id| {
            acc.into_iter()
                .map(|gate| {
                    if gate.gate_id == gate_id {
                        LifecycleGateSnapshot {
                            gate_id: gate.gate_id,
                            status: step.status.clone(),
                            message: step.message.clone().or(gate.message),
                        }
                    } else {
                        gate
                    }
                })
                .collect()
        })
    })
}

fn gate_for_step(step: &str) -> Option<&'static str> {
    match step {
        "mark_in_progress" => Some("G0"),
        "workspace_prepare" | "workspace_add" => Some("G1"),
        "opencode" => Some("G4"),
        "qa_enforcer" => Some("G6"),
        "ltc_quick" | "ltc_targeted" | "ltc_test" | "moon_ci" => Some("G5"),
        "jj_sync_main" | "jj_rebase_main" | "jj_track" | "jj_describe" | "validate_changes"
        | "bookmark_create" | "bookmark_push" | "pr_create" => Some("G8"),
        _ => None,
    }
}

fn discipline_gate_for_step(step: &str) -> Option<&'static str> {
    match step {
        "opencode" => Some("DG2_impl_quality"),
        "ltc_quick" | "ltc_targeted" | "ltc_test" | "moon_ci" => Some("DG3_validation_quality"),
        "qa_enforcer" => Some("DG4_audit_quality"),
        _ => None,
    }
}

fn default_gate_snapshots() -> Vec<LifecycleGateSnapshot> {
    ["G0", "G1", "G2", "G3", "G4", "G5", "G6", "G7", "G8"]
        .into_iter()
        .map(make_pending_gate)
        .collect()
}

fn default_discipline_gate_snapshots() -> Vec<LifecycleGateSnapshot> {
    [
        "DG0_contract_quality",
        "DG1_test_quality",
        "DG2_impl_quality",
        "DG3_validation_quality",
        "DG4_audit_quality",
    ]
    .into_iter()
    .map(make_pending_gate)
    .collect()
}

fn make_pending_gate(gate_id: &str) -> LifecycleGateSnapshot {
    LifecycleGateSnapshot {
        gate_id: gate_id.to_owned(),
        status: lifecycle_status_label(&LifecycleStepStatus::Pending).to_owned(),
        message: None,
    }
}

fn apply_finished_update(
    ctx: &WorkflowContext<'_>,
    success: bool,
    pr_url: Option<String>,
    message: Option<String>,
    compensation_diagnostics: Vec<crate::lifecycle::types::CompensationDiagnostic>,
) {
    ctx.set("lifecycle_done", true);
    ctx.set("lifecycle_success", Some(success));
    ctx.set("lifecycle_pr_url", pr_url);
    ctx.set("lifecycle_message", message);
    store_compensation_diagnostics(ctx, &compensation_diagnostics);
}

struct StepUpdate {
    step: String,
    status: LifecycleStepStatus,
    message: Option<String>,
    details: Option<Value>,
    started_at: Option<String>,
    finished_at: Option<String>,
    duration_ms: Option<u64>,
}

fn store_lifecycle_steps(ctx: &WorkflowContext<'_>, steps: &[LifecycleStepSnapshot]) {
    if let Ok(value) = serde_json::to_value(steps) {
        ctx.set("lifecycle_steps", Json::from(value));
    }
}

fn store_lifecycle_gates(ctx: &WorkflowContext<'_>, gates: &[LifecycleGateSnapshot]) {
    store_named_gates(ctx, "lifecycle_gates", gates);
}

fn store_lifecycle_discipline_gates(ctx: &WorkflowContext<'_>, gates: &[LifecycleGateSnapshot]) {
    store_named_gates(ctx, "lifecycle_discipline_gates", gates);
}

fn store_named_gates(ctx: &WorkflowContext<'_>, key: &str, gates: &[LifecycleGateSnapshot]) {
    if let Ok(value) = serde_json::to_value(gates) {
        ctx.set(key, Json::from(value));
    }
}

fn store_compensation_diagnostics(
    ctx: &WorkflowContext<'_>,
    diagnostics: &[crate::lifecycle::types::CompensationDiagnostic],
) {
    if let Ok(value) = serde_json::to_value(diagnostics) {
        ctx.set("lifecycle_compensation_diagnostics", Json::from(value));
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
    update: StepUpdate,
) -> Vec<LifecycleStepSnapshot> {
    let StepUpdate { step, status, message, details, started_at, finished_at, duration_ms } =
        update;
    let status_label = lifecycle_status_label(&status).to_owned();
    let mut found = false;
    let mapped = steps
        .into_iter()
        .map(|item| {
            if item.step == step {
                found = true;
                LifecycleStepSnapshot {
                    step: item.step,
                    status: status_label.clone(),
                    message: message.clone(),
                    details: details.clone(),
                    started_at: started_at.clone().or(item.started_at),
                    finished_at: finished_at.clone().or(item.finished_at),
                    duration_ms: duration_ms.or(item.duration_ms),
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
                status: status_label,
                message,
                details,
                started_at,
                finished_at,
                duration_ms,
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
    let json_string = serde_json::to_string(state).map_err(|error| {
        HandlerError::from(format!("failed to serialize lifecycle state to string: {error}"))
    })?;
    let pr_url = extract_pr_url_from_state(state);
    ctx.set("lifecycle_state", Json::from(value));
    ctx.set("lifecycle_pr_url", pr_url);

    if let Some(db) = STATE_DB.get() {
        if let Ok(bead_id) = BeadId::parse(ctx.key()) {
            let _ = db.batch_persist_state(&bead_id, &json_string, &[]);
        }
    }
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

/// Read an optional string state key, treating deserialization failures (e.g. empty-byte
/// values written as `None`) as absent rather than propagating an error.
async fn get_optional_string(
    ctx: &SharedWorkflowContext<'_>,
    key: &str,
) -> Result<Option<String>, HandlerError> {
    match ctx.get::<String>(key).await {
        Ok(value) => Ok(value),
        Err(_) => Ok(None),
    }
}

async fn read_lifecycle_status(
    ctx: &SharedWorkflowContext<'_>,
) -> Result<LifecycleStatusSnapshot, HandlerError> {
    let steps = get_json_vec::<LifecycleStepSnapshot>(ctx, "lifecycle_steps").await;
    let state = get_json_value(ctx, "lifecycle_state").await;
    let compensation_diagnostics = get_json_vec::<crate::lifecycle::types::CompensationDiagnostic>(
        ctx,
        "lifecycle_compensation_diagnostics",
    )
    .await;
    let gates = get_json_vec::<LifecycleGateSnapshot>(ctx, "lifecycle_gates").await;
    let discipline_gates =
        get_json_vec::<LifecycleGateSnapshot>(ctx, "lifecycle_discipline_gates").await;
    Ok(LifecycleStatusSnapshot {
        bead_id: get_optional_string(ctx, "lifecycle_bead_id").await?,
        steps,
        gates: if gates.is_empty() { default_gate_snapshots() } else { gates },
        discipline_gates: if discipline_gates.is_empty() {
            default_discipline_gate_snapshots()
        } else {
            discipline_gates
        },
        state,
        pr_url: get_optional_string(ctx, "lifecycle_pr_url").await?,
        done: ctx.get::<bool>("lifecycle_done").await.ok().flatten().unwrap_or(false),
        success: ctx.get::<bool>("lifecycle_success").await.ok().flatten(),
        message: get_optional_string(ctx, "lifecycle_message").await?,
        compensation_diagnostics,
    })
}

async fn get_json_vec<T>(ctx: &SharedWorkflowContext<'_>, key: &str) -> Vec<T>
where
    T: serde::de::DeserializeOwned,
{
    get_json_raw(ctx, key)
        .await
        .and_then(|value| serde_json::from_value::<Vec<T>>(value).ok())
        .unwrap_or_default()
}

async fn get_json_value(ctx: &SharedWorkflowContext<'_>, key: &str) -> Option<Value> {
    get_json_raw(ctx, key).await.and_then(|value| if value.is_null() { None } else { Some(value) })
}

async fn get_json_raw(ctx: &SharedWorkflowContext<'_>, key: &str) -> Option<Value> {
    ctx.get::<Json<Value>>(key).await.ok().flatten().map(Json::into_inner)
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
                details: None,
                started_at: None,
                finished_at: None,
                duration_ms: None,
            })
        })
        .collect()
}

pub(super) fn is_lifecycle_not_found(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.is_empty() || trimmed.contains("No invocations matched")
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
        gates: default_gate_snapshots(),
        discipline_gates: default_discipline_gate_snapshots(),
        state: None,
        pr_url: extract_pr_url(raw),
        done: !(is_running || is_backing_off),
        success: if is_running || is_backing_off { None } else { Some(!raw.contains("Error:")) },
        message,
        compensation_diagnostics: Vec::new(),
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

fn validate_optional_bead_id(value: Option<String>) -> Result<Option<String>, HandlerError> {
    value.map(validate_bead_id).transpose()
}

fn validate_bead_id(value: String) -> Result<String, HandlerError> {
    BeadId::parse(&value)
        .map(|bead_id| bead_id.as_str().to_owned())
        .map_err(|error| TerminalError::new(format!("invalid bead id: {error}")).into())
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

#[allow(clippy::too_many_arguments)]
async fn build_memory_snapshot(
    bead_id: Option<String>,
    bead_status: Option<String>,
    bead_state: Option<Value>,
    last_output_summary: Option<Value>,
    last_output_trace: Option<Value>,
    active_invocation_id: Option<String>,
    cancel_state: Option<String>,
) -> MemorySnapshot {
    let bead = BeadSnapshot {
        bead_id: bead_id.and_then(|v| BeadId::parse(&v).ok()),
        bead_status: bead_status.and_then(|v| BeadStatus::parse(&v).ok()),
        bead_state,
    };
    MemorySnapshot {
        bead,
        last_output_summary,
        last_output_trace,
        active_invocation_id,
        cancel_state: cancel_state.and_then(parse_cancel_state).unwrap_or_default(),
    }
}

async fn flush_memory_state(ctx: &ObjectContext<'_>) -> Result<(), HandlerError> {
    let bead_id_val = ctx.get::<String>("bead_id").await?;
    let bead_status_val = ctx.get::<String>("bead_status").await?;
    let bead_state_val = ctx.get::<Json<Value>>("bead_state").await?.map(Json::into_inner);
    let summary = ctx.get::<Json<Value>>("last_output_summary").await?.map(Json::into_inner);
    let trace = ctx.get::<Json<Value>>("last_output_trace").await?.map(Json::into_inner);
    let active_inv = ctx.get::<String>("active_invocation_id").await?;
    let cancel = ctx.get::<String>("cancel_state").await?;

    let snapshot = build_memory_snapshot(
        bead_id_val,
        bead_status_val,
        bead_state_val,
        summary,
        trace,
        active_inv,
        cancel,
    )
    .await;

    if let Some(db) = STATE_DB.get() {
        if let Ok(bead_id) = BeadId::parse(ctx.key()) {
            if let Ok(json) = serde_json::to_string(&snapshot) {
                let _ = db.persist_memory(&bead_id, &json);
                let _ = db.flush();
            }
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{
        append_opencode_trace_event, finalize_opencode_run, get_opencode_trace_snapshot,
        is_safe_runtime_key, is_uninitialized_workflow_snapshot, lifecycle_status_label,
        parse_lifecycle_status_snapshot, seed_opencode_trace, upsert_step, validate_bead_id,
        validate_optional_bead_id, StepUpdate,
    };
    use crate::lifecycle::workflow::LifecycleStepStatus;
    use crate::restate_oya::types::{
        LifecycleStatusSnapshot, LifecycleStepSnapshot, OpenCodeTraceEvent,
    };
    use serde_json::json;

    #[test]
    fn upsert_step_preserves_timestamps_across_progress_updates() {
        let started_at = "2026-02-27T02:30:00Z".to_owned();
        let finished_at = "2026-02-27T02:30:01Z".to_owned();
        let initial = vec![LifecycleStepSnapshot {
            step: "moon_ci".to_owned(),
            status: lifecycle_status_label(&LifecycleStepStatus::Running).to_owned(),
            message: Some("started".to_owned()),
            details: None,
            started_at: Some(started_at.clone()),
            finished_at: None,
            duration_ms: None,
        }];

        let updated = upsert_step(
            initial,
            StepUpdate {
                step: "moon_ci".to_owned(),
                status: LifecycleStepStatus::Succeeded,
                message: Some("done".to_owned()),
                details: None,
                started_at: None,
                finished_at: Some(finished_at.clone()),
                duration_ms: Some(1_000),
            },
        );

        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].started_at, Some(started_at));
        assert_eq!(updated[0].finished_at, Some(finished_at));
        assert_eq!(updated[0].duration_ms, Some(1_000));
    }

    #[test]
    fn parse_lifecycle_status_snapshot_running_state_is_incomplete() {
        let snapshot =
            parse_lifecycle_status_snapshot("Status: running\nCommand: moon run :ci\n", "src-1ji");

        assert!(!snapshot.done);
        assert_eq!(snapshot.success, None);
        assert_eq!(snapshot.bead_id, Some("src-1ji".to_owned()));
        assert_eq!(snapshot.steps.len(), 1);
    }

    #[test]
    fn parse_lifecycle_status_snapshot_error_state_is_terminal() {
        let snapshot = parse_lifecycle_status_snapshot(
             "Status: completed\nError: failed to open PR\nhttps://github.com/lprior-repo/oya/pull/42\n",
             "src-1ji",
         );

        assert!(snapshot.done);
        assert_eq!(snapshot.success, Some(false));
        assert_eq!(snapshot.pr_url, Some("https://github.com/lprior-repo/oya/pull/42".to_owned()));
    }

    #[test]
    fn uninitialized_workflow_snapshot_is_detected() {
        let snapshot = LifecycleStatusSnapshot {
            bead_id: None,
            steps: Vec::new(),
            gates: Vec::new(),
            discipline_gates: Vec::new(),
            state: None,
            pr_url: None,
            done: false,
            success: None,
            message: None,
            compensation_diagnostics: Vec::new(),
        };
        assert!(is_uninitialized_workflow_snapshot(&snapshot));

        let snapshot_with_bead = LifecycleStatusSnapshot {
            bead_id: Some("src-test".to_owned()),
            steps: Vec::new(),
            gates: Vec::new(),
            discipline_gates: Vec::new(),
            state: None,
            pr_url: None,
            done: false,
            success: None,
            message: None,
            compensation_diagnostics: Vec::new(),
        };
        assert!(is_uninitialized_workflow_snapshot(&snapshot_with_bead));
    }

    #[test]
    fn runtime_key_validation_accepts_safe_keys() {
        assert!(is_safe_runtime_key("src-1fa"));
        assert!(is_safe_runtime_key("abc_123.def"));
    }

    #[test]
    fn runtime_key_validation_rejects_unsafe_keys() {
        assert!(!is_safe_runtime_key(""));
        assert!(!is_safe_runtime_key("a/b"));
        assert!(!is_safe_runtime_key("a b"));
        assert!(!is_safe_runtime_key("%00"));
    }

    #[test]
    fn bead_id_boundary_validation_accepts_canonical_ids() {
        let Ok(bead_id) = validate_bead_id("oya-8y3".to_owned()) else {
            assert!(false, "canonical bead id should pass");
            return;
        };

        assert_eq!(bead_id, "oya-8y3");
        assert_eq!(
            validate_optional_bead_id(Some("oya-8y3".to_owned())).ok().flatten().as_deref(),
            Some("oya-8y3")
        );
    }

    #[test]
    fn bead_id_boundary_validation_rejects_path_like_ids() {
        let Some(error) = validate_bead_id("bad/../id".to_owned()).err() else {
            assert!(false, "path-like bead id should fail");
            return;
        };
        let message = format!("{error:?}");

        assert!(message.contains("invalid bead id"));
        assert!(message.contains("invalid chars"));
    }

    #[test]
    fn opencode_trace_cache_is_addressable_by_workflow_and_bead_keys() {
        let workflow_key = "trace-workflow-cache-test";
        let bead_key = "trace-bead-cache-test";

        seed_opencode_trace(workflow_key, Some(bead_key.to_owned()), "inv_123", "test/model");
        append_opencode_trace_event(workflow_key, trace_event(1, "tool_use"));

        let by_workflow = get_opencode_trace_snapshot(workflow_key);
        let by_workflow_path = get_opencode_trace_snapshot("Oya/trace-workflow-cache-test/run");
        let by_bead = get_opencode_trace_snapshot(bead_key);

        assert_eq!(by_workflow.workflow_key, workflow_key);
        assert_eq!(by_workflow.status, "running");
        assert_eq!(by_workflow.model.as_deref(), Some("test/model"));
        assert_eq!(by_workflow.active_invocation_id.as_deref(), Some("inv_123"));
        assert_eq!(by_workflow.tool_call_count, 1);
        assert_eq!(by_workflow_path.tool_call_count, 1);
        assert_eq!(by_bead.tool_call_count, 1);
    }

    #[test]
    fn opencode_trace_cache_finalizes_success_summary() {
        let workflow_key = "trace-finalize-cache-test";
        seed_opencode_trace(workflow_key, None, "inv_456", "test/model");

        let output = r#"{"type":"tool_use","part":{"tool":"bash"}}
{"type":"text","part":{"text":"finished"}}"#
            .to_owned();

        let Ok(output) = finalize_opencode_run(workflow_key, Ok(output)) else {
            assert!(false, "trace finalizes");
            return;
        };
        let snapshot = get_opencode_trace_snapshot(workflow_key);

        assert!(output.contains("tool_use"));
        assert_eq!(snapshot.status, "succeeded");
        assert!(snapshot.active_invocation_id.is_none());
        assert_eq!(
            snapshot.summary.as_ref().and_then(|value| value.get("event_count")),
            Some(&json!(2))
        );
        assert_eq!(
            snapshot.summary.as_ref().and_then(|value| value.get("tool_calls")),
            Some(&json!(1))
        );
    }

    fn trace_event(sequence: u64, kind: &str) -> OpenCodeTraceEvent {
        OpenCodeTraceEvent {
            sequence,
            received_at: format!("2026-04-29T00:00:{sequence:02}Z"),
            kind: kind.to_owned(),
            step: Some(sequence),
            tool: Some("bash".to_owned()),
            description: None,
            command: Some("moon run :test".to_owned()),
            query: None,
            text: None,
            error: None,
            raw: json!({ "type": kind }),
        }
    }
}
