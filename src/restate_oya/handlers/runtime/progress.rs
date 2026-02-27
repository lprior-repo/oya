#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::workflow::{LifecycleProgressUpdate, LifecycleStepStatus};
use crate::restate_oya::types::LifecycleStepSnapshot;
use restate_sdk::prelude::*;
use serde_json::Value;

use super::super::status::lifecycle_status_label;

const KEY_BEAD_ID: &str = "lifecycle_bead_id";
const KEY_STEPS: &str = "lifecycle_steps";
const KEY_STATE: &str = "lifecycle_state";
const KEY_PR_URL: &str = "lifecycle_pr_url";
const KEY_DONE: &str = "lifecycle_done";
const KEY_SUCCESS: &str = "lifecycle_success";
const KEY_MESSAGE: &str = "lifecycle_message";
const KEY_COMPENSATION_DIAGNOSTICS: &str = "lifecycle_compensation_diagnostics";

pub struct StepUpdate {
    pub step: String,
    pub status: LifecycleStepStatus,
    pub message: Option<String>,
    pub details: Option<Value>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
}

fn pending_status_label() -> String {
    lifecycle_status_label(&LifecycleStepStatus::Pending).to_owned()
}

fn updated_snapshot(
    existing: LifecycleStepSnapshot,
    update: &StepUpdate,
    status_label: &str,
) -> LifecycleStepSnapshot {
    LifecycleStepSnapshot {
        step: existing.step,
        status: status_label.to_owned(),
        message: update.message.clone(),
        details: update.details.clone(),
        started_at: update.started_at.clone().or(existing.started_at),
        finished_at: update.finished_at.clone().or(existing.finished_at),
        duration_ms: update.duration_ms.or(existing.duration_ms),
    }
}

fn update_to_snapshot(update: StepUpdate, status_label: &str) -> LifecycleStepSnapshot {
    LifecycleStepSnapshot {
        step: update.step,
        status: status_label.to_owned(),
        message: update.message,
        details: update.details,
        started_at: update.started_at,
        finished_at: update.finished_at,
        duration_ms: update.duration_ms,
    }
}

pub fn initialize_lifecycle_status(
    ctx: &WorkflowContext<'_>,
    bead_id: Option<String>,
    steps: &[LifecycleStepSnapshot],
) {
    ctx.set(KEY_BEAD_ID, bead_id);
    store_lifecycle_steps(ctx, steps);
    ctx.clear(KEY_STATE);
    ctx.clear(KEY_PR_URL);
    ctx.set(KEY_DONE, false);
    ctx.clear(KEY_SUCCESS);
    ctx.clear(KEY_MESSAGE);
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
    .map(make_pending_snapshot)
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
            apply_finished_update(ctx, success, pr_url, message, &compensation_diagnostics);
        }
    }
}

pub fn upsert_step(
    steps: Vec<LifecycleStepSnapshot>,
    update: StepUpdate,
) -> Vec<LifecycleStepSnapshot> {
    let status_label = lifecycle_status_label(&update.status).to_owned();
    let found = steps.iter().any(|item| item.step == update.step);
    let mapped = steps
        .into_iter()
        .map(|item| {
            if item.step == update.step {
                updated_snapshot(item, &update, &status_label)
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
            .chain(std::iter::once(update_to_snapshot(update, &status_label)))
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
    ctx.set(KEY_STATE, Json::from(value));
    ctx.set(KEY_PR_URL, extract_pr_url_from_state(state));
    Ok(())
}

fn apply_initialized_update(
    ctx: &WorkflowContext<'_>,
    live_steps: &mut Vec<LifecycleStepSnapshot>,
    bead_id: String,
    steps: Vec<String>,
) {
    *live_steps = steps.into_iter().map(make_pending_snapshot).collect::<Vec<_>>();
    ctx.set(KEY_BEAD_ID, Some(bead_id));
    store_lifecycle_steps(ctx, live_steps);
    ctx.set(KEY_MESSAGE, Option::<String>::None);
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
    compensation_diagnostics: &[crate::lifecycle::types::CompensationDiagnostic],
) {
    ctx.set(KEY_DONE, true);
    ctx.set(KEY_SUCCESS, Some(success));
    ctx.set(KEY_PR_URL, pr_url);
    ctx.set(KEY_MESSAGE, message);
    store_compensation_diagnostics(ctx, compensation_diagnostics);
}

fn make_pending_snapshot(step: impl Into<String>) -> LifecycleStepSnapshot {
    LifecycleStepSnapshot {
        step: step.into(),
        status: pending_status_label(),
        message: None,
        details: None,
        started_at: None,
        finished_at: None,
        duration_ms: None,
    }
}

fn store_lifecycle_steps(ctx: &WorkflowContext<'_>, steps: &[LifecycleStepSnapshot]) {
    if let Ok(value) = serde_json::to_value(steps) {
        ctx.set(KEY_STEPS, Json::from(value));
    }
}

fn store_compensation_diagnostics(
    ctx: &WorkflowContext<'_>,
    diagnostics: &[crate::lifecycle::types::CompensationDiagnostic],
) {
    if let Ok(value) = serde_json::to_value(diagnostics) {
        ctx.set(KEY_COMPENSATION_DIAGNOSTICS, Json::from(value));
    }
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
