#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::workflow::LifecycleStepStatus;
use crate::restate_oya::types::{LifecycleStatusSnapshot, LifecycleStepSnapshot, StartResponse};
use restate_sdk::prelude::*;
use serde_json::Value;

use super::runtime::seed_runtime_status;
use super::OyaClient;

pub fn lifecycle_status_label(status: &LifecycleStepStatus) -> &'static str {
    match status {
        LifecycleStepStatus::Pending => "pending",
        LifecycleStepStatus::Running => "running",
        LifecycleStepStatus::Succeeded => "succeeded",
        LifecycleStepStatus::Failed => "failed",
    }
}

pub async fn read_lifecycle_status(
    ctx: &SharedWorkflowContext<'_>,
) -> Result<LifecycleStatusSnapshot, HandlerError> {
    let steps = ctx
        .get::<Json<Value>>("lifecycle_steps")
        .await
        .ok()
        .flatten()
        .map(Json::into_inner)
        .and_then(|value| serde_json::from_value::<Vec<LifecycleStepSnapshot>>(value).ok())
        .unwrap_or_default();
    let state = ctx
        .get::<Json<Value>>("lifecycle_state")
        .await
        .ok()
        .flatten()
        .map(Json::into_inner)
        .and_then(|value| if value.is_null() { None } else { Some(value) });
    let compensation_diagnostics = ctx
        .get::<Json<Value>>("lifecycle_compensation_diagnostics")
        .await
        .ok()
        .flatten()
        .map(Json::into_inner)
        .and_then(|value| {
            serde_json::from_value::<Vec<crate::lifecycle::types::CompensationDiagnostic>>(value)
                .ok()
        })
        .unwrap_or_default();
    Ok(LifecycleStatusSnapshot {
        bead_id: get_optional_string(ctx, "lifecycle_bead_id").await?,
        steps,
        state,
        pr_url: get_optional_string(ctx, "lifecycle_pr_url").await?,
        done: ctx.get::<bool>("lifecycle_done").await.ok().flatten().unwrap_or(false),
        success: ctx.get::<bool>("lifecycle_success").await.ok().flatten(),
        message: get_optional_string(ctx, "lifecycle_message").await?,
        compensation_diagnostics,
    })
}

async fn get_optional_string(
    ctx: &SharedWorkflowContext<'_>,
    key: &str,
) -> Result<Option<String>, HandlerError> {
    match ctx.get::<String>(key).await {
        Ok(value) => Ok(value),
        Err(_) => Ok(None),
    }
}

pub fn serialize_workflow_outcome(
    outcome: &crate::lifecycle::workflow::LifecycleRunOutcome,
) -> Result<StartResponse, HandlerError> {
    let output = serde_json::to_string(outcome).map_err(|error| {
        HandlerError::from(format!("failed to serialize lifecycle outcome: {error}"))
    })?;
    Ok(StartResponse { output })
}

pub fn workflow_key_for_service_key(key: &str) -> String {
    key.strip_prefix("Oya/")
        .and_then(|value| value.strip_suffix("/run"))
        .map_or_else(|| key.to_owned(), std::borrow::ToOwned::to_owned)
}

pub async fn read_workflow_status(
    ctx: &Context<'_>,
    workflow_key: &str,
) -> Option<LifecycleStatusSnapshot> {
    ctx.workflow_client::<OyaClient>(workflow_key).status().call().await.ok().map(|snapshot| {
        let status = snapshot.into_inner();
        seed_runtime_status(workflow_key, status.bead_id.clone(), status.steps.as_slice());
        status
    })
}

pub async fn fetch_lifecycle_status_raw(key: String) -> Result<String, HandlerError> {
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
    tokio::process::Command::new("restate")
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
    let output = tokio::process::Command::new("restate")
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

pub fn parse_lifecycle_status_snapshot(raw: &str, key: &str) -> LifecycleStatusSnapshot {
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

#[cfg(test)]
mod tests {
    use super::super::runtime::{upsert_step, StepUpdate};
    use super::*;
    use crate::lifecycle::workflow::LifecycleStepStatus;

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
}
