#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::workflow::{LifecycleProgressUpdate, LifecycleStepStatus};
use crate::restate_oya::types::LifecycleStepSnapshot;
use restate_sdk::prelude::*;
use serde_json::Value;

use super::super::status::lifecycle_status_label;

pub struct StepUpdate {
    pub step: String,
    pub status: LifecycleStepStatus,
    pub message: Option<String>,
    pub details: Option<Value>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
}

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
            apply_finished_update(ctx, success, pr_url, message, compensation_diagnostics);
        }
    }
}

pub fn upsert_step(
    steps: Vec<LifecycleStepSnapshot>,
    update: StepUpdate,
) -> Vec<LifecycleStepSnapshot> {
    let StepUpdate { step, status, message, details, started_at, finished_at, duration_ms } =
        update;
    let status_label = lifecycle_status_label(&status).to_owned();

    let (mapped, found) = steps.into_iter().fold((Vec::new(), false), |(mut acc, found), item| {
        if item.step == step {
            acc.push(LifecycleStepSnapshot {
                step: item.step,
                status: status_label.clone(),
                message: message.clone(),
                details: details.clone(),
                started_at: started_at.clone().or(item.started_at),
                finished_at: finished_at.clone().or(item.finished_at),
                duration_ms: duration_ms.or(item.duration_ms),
            });
            (acc, true)
        } else {
            acc.push(item);
            (acc, found)
        }
    });

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
    ctx.set("lifecycle_state", Json::from(value));
    ctx.set("lifecycle_pr_url", extract_pr_url_from_state(state));
    Ok(())
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

fn make_pending_snapshot(step: impl Into<String>) -> LifecycleStepSnapshot {
    LifecycleStepSnapshot {
        step: step.into(),
        status: lifecycle_status_label(&LifecycleStepStatus::Pending).to_owned(),
        message: None,
        details: None,
        started_at: None,
        finished_at: None,
        duration_ms: None,
    }
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
