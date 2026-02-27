#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::workflow::{LifecycleProgressUpdate, LifecycleStepStatus};
use crate::restate_oya::types::{LifecycleStatusSnapshot, LifecycleStepSnapshot};
use restate_sdk::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use super::status::lifecycle_status_label;

static RUNTIME_LIFECYCLE_STATUS: LazyLock<RwLock<HashMap<String, LifecycleStatusSnapshot>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn initialize_lifecycle_status(
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
    store_compensation_diagnostics(ctx, &[]);
}

pub fn default_step_snapshots() -> Vec<LifecycleStepSnapshot> {
    [
        "mark_in_progress",
        "workspace_prepare",
        "workspace_add",
        "opencode",
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

pub fn apply_progress_update(
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

pub struct StepUpdate {
    pub step: String,
    pub status: LifecycleStepStatus,
    pub message: Option<String>,
    pub details: Option<Value>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
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

fn store_lifecycle_steps(ctx: &WorkflowContext<'_>, steps: &[LifecycleStepSnapshot]) {
    if let Ok(value) = serde_json::to_value(steps) {
        ctx.set("lifecycle_steps", Json::from(value));
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

pub fn upsert_step(
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

pub fn store_lifecycle_state(
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

pub fn get_runtime_status(key: &str) -> Option<LifecycleStatusSnapshot> {
    RUNTIME_LIFECYCLE_STATUS.read().ok().and_then(|map| {
        runtime_lookup_keys(key).into_iter().find_map(|candidate| map.get(&candidate).cloned())
    })
}

pub fn seed_runtime_status(
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
                compensation_diagnostics: Vec::new(),
            },
        );
    }
}

pub fn update_runtime_progress(
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
    runtime_store_keys(workflow_key, snapshot.bead_id.as_deref()).into_iter().for_each(
        |candidate| {
            map.insert(candidate, snapshot.clone());
        },
    );
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
            compensation_diagnostics: current.compensation_diagnostics,
        },
        LifecycleProgressUpdate::Step { message, .. } => LifecycleStatusSnapshot {
            bead_id: current.bead_id,
            steps: live_steps.to_vec(),
            state: current.state,
            pr_url: current.pr_url,
            done: false,
            success: None,
            message,
            compensation_diagnostics: current.compensation_diagnostics,
        },
        LifecycleProgressUpdate::Finished {
            success,
            pr_url,
            message,
            compensation_diagnostics,
        } => LifecycleStatusSnapshot {
            bead_id: current.bead_id,
            steps: live_steps.to_vec(),
            state: current.state,
            pr_url,
            done: true,
            success: Some(success),
            message,
            compensation_diagnostics,
        },
    }
}

pub async fn forget_workspace_for_key(key: String) -> Result<String, HandlerError> {
    let workspace = format!("oya-{key}");
    let output = tokio::process::Command::new("jj")
        .arg("workspace")
        .arg("forget")
        .arg(&workspace)
        .output()
        .await
        .map_err(|error| {
            HandlerError::from(format!("failed to run jj workspace forget: {error}"))
        })?;
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

pub fn cleanup_targets_for_key(key: &str) -> Vec<String> {
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

pub async fn forget_workspace_for_targets(targets: Vec<String>) -> Result<String, HandlerError> {
    let mut messages = Vec::new();
    for target in targets {
        messages.push(forget_workspace_for_key(target).await?);
    }
    Ok(messages.join("; "))
}
