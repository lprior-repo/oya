#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use chrono::{SecondsFormat, Utc};
use serde_json::Value;

use super::types::{LifecycleProgressUpdate, LifecycleStepStatus};

pub fn make_step_progress_running(step: &str, started_at: &str) -> LifecycleProgressUpdate {
    LifecycleProgressUpdate::Step {
        step: step.to_owned(),
        status: LifecycleStepStatus::Running,
        message: None,
        details: None,
        started_at: Some(started_at.to_owned()),
        finished_at: None,
        duration_ms: None,
    }
}

pub fn make_step_progress_success(
    step: String,
    details: Option<Value>,
    started_at: &str,
    finished_at: &str,
    duration_ms: u64,
) -> LifecycleProgressUpdate {
    LifecycleProgressUpdate::Step {
        step,
        status: LifecycleStepStatus::Succeeded,
        message: None,
        details,
        started_at: Some(started_at.to_owned()),
        finished_at: Some(finished_at.to_owned()),
        duration_ms: Some(duration_ms),
    }
}

pub fn make_step_progress_failure(
    step: String,
    message: String,
    started_at: &str,
    finished_at: &str,
    duration_ms: u64,
) -> LifecycleProgressUpdate {
    LifecycleProgressUpdate::Step {
        step,
        status: LifecycleStepStatus::Failed,
        message: Some(message),
        details: None,
        started_at: Some(started_at.to_owned()),
        finished_at: Some(finished_at.to_owned()),
        duration_ms: Some(duration_ms),
    }
}

pub fn compute_duration_ms(start: &std::time::Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub fn timestamp_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
