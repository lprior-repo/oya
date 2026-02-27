#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::CompensationDiagnostic;
use crate::lifecycle::workflow::{LifecycleProgressUpdate, LifecycleStepStatus};
use tracing::{field, info_span, span, Instrument, Level};

pub struct StepSpanGuard {
    #[allow(dead_code)]
    span: tracing::Span,
}

impl StepSpanGuard {
    pub fn new(step: &str, status: &LifecycleStepStatus, started_at: &str) -> Self {
        let status_label = status_label(status);
        let span = info_span!(
            "lifecycle_step",
            step = step,
            status = status_label,
            started_at = started_at,
            finished_at = field::Empty,
            duration_ms = field::Empty,
            message = field::Empty,
        );
        Self { span }
    }

    #[allow(dead_code)]
    pub fn finish(&self, finished_at: &str, duration_ms: u64, message: Option<&str>) {
        self.span.record("finished_at", finished_at);
        self.span.record("duration_ms", duration_ms);
        if let Some(msg) = message {
            self.span.record("message", msg);
        }
    }
}

fn status_label(status: &LifecycleStepStatus) -> &'static str {
    match status {
        LifecycleStepStatus::Pending => "pending",
        LifecycleStepStatus::Running => "running",
        LifecycleStepStatus::Succeeded => "succeeded",
        LifecycleStepStatus::Failed => "failed",
    }
}

pub fn emit_step_telemetry(update: &LifecycleProgressUpdate) {
    match update {
        LifecycleProgressUpdate::Initialized { bead_id, steps } => {
            emit_initialized_telemetry(bead_id, steps);
        }
        LifecycleProgressUpdate::Step {
            step,
            status,
            message,
            started_at,
            finished_at,
            duration_ms,
            ..
        } => {
            emit_step_transition_telemetry(
                step,
                status,
                message.as_ref(),
                started_at.as_ref(),
                finished_at.as_ref(),
                duration_ms.as_ref(),
            );
        }
        LifecycleProgressUpdate::Finished {
            success,
            pr_url,
            message,
            compensation_diagnostics,
        } => {
            emit_finished_telemetry(
                *success,
                pr_url.as_ref(),
                message.as_ref(),
                compensation_diagnostics,
            );
        }
    }
}

fn emit_initialized_telemetry(bead_id: &str, steps: &[String]) {
    let span = info_span!("lifecycle_initialized", bead_id = bead_id, step_count = steps.len(),);
    let _enter = span.enter();
    tracing::info!(bead_id = bead_id, "lifecycle workflow initialized");
}

#[allow(clippy::too_many_arguments)]
fn emit_step_transition_telemetry(
    step: &str,
    status: &LifecycleStepStatus,
    message: Option<&String>,
    started_at: Option<&String>,
    finished_at: Option<&String>,
    duration_ms: Option<&u64>,
) {
    let span = span!(
        Level::INFO,
        "lifecycle_step_transition",
        step = step,
        status = status_label(status),
        started_at = started_at.map_or("", |s| s.as_str()),
        finished_at = finished_at.map_or("", |s| s.as_str()),
        duration_ms = duration_ms.map_or(0, |v| *v),
        message = message.map_or("", |s| s.as_str()),
    );
    let _enter = span.enter();
    tracing::info!(step = step, status = status_label(status), "lifecycle step transition");
}

fn emit_finished_telemetry(
    success: bool,
    pr_url: Option<&String>,
    message: Option<&String>,
    compensation_diagnostics: &[CompensationDiagnostic],
) {
    let span = info_span!(
        "lifecycle_finished",
        success = success,
        pr_url = pr_url.map_or("", |s| s.as_str()),
        message = message.map_or("", |s| s.as_str()),
        compensation_count = compensation_diagnostics.len(),
    );
    let _enter = span.enter();
    emit_compensation_telemetry(compensation_diagnostics);
    tracing::info!(
        success = success,
        pr_url = pr_url.map_or("", |s| s.as_str()),
        "lifecycle workflow finished"
    );
}

pub fn emit_compensation_telemetry(diagnostics: &[CompensationDiagnostic]) {
    for diagnostic in diagnostics {
        let span = info_span!(
            "lifecycle_compensation",
            compensation_type = diagnostic.compensation_type.as_str(),
            target = diagnostic.target.as_str(),
            success = diagnostic.success,
            error = diagnostic.error.as_deref().unwrap_or(""),
        );
        let _enter = span.enter();
        tracing::info!(
            compensation_type = diagnostic.compensation_type.as_str(),
            target = diagnostic.target.as_str(),
            success = diagnostic.success,
            "compensation operation recorded"
        );
    }
}

pub fn emit_unwind_signal(diagnostic: &CompensationDiagnostic) {
    if diagnostic.success {
        emit_unwind_success(diagnostic);
    } else {
        emit_unwind_failure(diagnostic);
    }
}

fn emit_unwind_success(diagnostic: &CompensationDiagnostic) {
    let span = info_span!(
        "lifecycle_unwind_signal",
        compensation_type = diagnostic.compensation_type.as_str(),
        target = diagnostic.target.as_str(),
        success = diagnostic.success,
        error = diagnostic.error.as_deref().unwrap_or(""),
    );
    let _enter = span.enter();
    tracing::info!(
        compensation_type = diagnostic.compensation_type.as_str(),
        target = diagnostic.target.as_str(),
        "unwind operation succeeded"
    );
}

fn emit_unwind_failure(diagnostic: &CompensationDiagnostic) {
    let span = tracing::warn_span!(
        "lifecycle_unwind_signal",
        compensation_type = diagnostic.compensation_type.as_str(),
        target = diagnostic.target.as_str(),
        success = diagnostic.success,
        error = diagnostic.error.as_deref().unwrap_or(""),
    );
    let _enter = span.enter();
    tracing::warn!(
        compensation_type = diagnostic.compensation_type.as_str(),
        target = diagnostic.target.as_str(),
        error = diagnostic.error.as_deref().unwrap_or("unknown"),
        "unwind operation failed"
    );
}

pub async fn with_step_telemetry<F, T>(step: &str, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let span = info_span!("lifecycle_step_execution", step = step);
    future.instrument(span).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::types::CompensationDiagnostic;

    #[test]
    fn status_label_returns_correct_strings() {
        assert_eq!(status_label(&LifecycleStepStatus::Pending), "pending");
        assert_eq!(status_label(&LifecycleStepStatus::Running), "running");
        assert_eq!(status_label(&LifecycleStepStatus::Succeeded), "succeeded");
        assert_eq!(status_label(&LifecycleStepStatus::Failed), "failed");
    }

    #[test]
    fn emit_step_telemetry_handles_initialized() {
        let update = LifecycleProgressUpdate::Initialized {
            bead_id: "test-bead".to_owned(),
            steps: vec!["step1".to_owned(), "step2".to_owned()],
        };
        emit_step_telemetry(&update);
    }

    #[test]
    fn emit_step_telemetry_handles_step_running() {
        let update = LifecycleProgressUpdate::Step {
            step: "moon_ci".to_owned(),
            status: LifecycleStepStatus::Running,
            message: Some("starting".to_owned()),
            details: None,
            started_at: Some("2026-02-27T00:00:00Z".to_owned()),
            finished_at: None,
            duration_ms: None,
        };
        emit_step_telemetry(&update);
    }

    #[test]
    fn emit_step_telemetry_handles_step_succeeded() {
        let update = LifecycleProgressUpdate::Step {
            step: "moon_ci".to_owned(),
            status: LifecycleStepStatus::Succeeded,
            message: None,
            details: Some(serde_json::json!({"output": "ok"})),
            started_at: Some("2026-02-27T00:00:00Z".to_owned()),
            finished_at: Some("2026-02-27T00:00:05Z".to_owned()),
            duration_ms: Some(5000),
        };
        emit_step_telemetry(&update);
    }

    #[test]
    fn emit_step_telemetry_handles_finished_success() {
        let update = LifecycleProgressUpdate::Finished {
            success: true,
            pr_url: Some("https://github.com/test/repo/pull/1".to_owned()),
            message: None,
            compensation_diagnostics: vec![],
        };
        emit_step_telemetry(&update);
    }

    #[test]
    fn emit_step_telemetry_handles_finished_with_compensations() {
        let diagnostic = CompensationDiagnostic {
            compensation_type: "forget_workspace".to_owned(),
            target: "oya-test".to_owned(),
            success: true,
            error: None,
        };
        let update = LifecycleProgressUpdate::Finished {
            success: false,
            pr_url: None,
            message: Some("step failed".to_owned()),
            compensation_diagnostics: vec![diagnostic],
        };
        emit_step_telemetry(&update);
    }

    #[test]
    fn emit_unwind_signal_success() {
        let diagnostic = CompensationDiagnostic {
            compensation_type: "forget_workspace".to_owned(),
            target: "oya-test".to_owned(),
            success: true,
            error: None,
        };
        emit_unwind_signal(&diagnostic);
    }

    #[test]
    fn emit_unwind_signal_failure() {
        let diagnostic = CompensationDiagnostic {
            compensation_type: "mark_bead_blocked".to_owned(),
            target: "src-abc".to_owned(),
            success: false,
            error: Some("command failed".to_owned()),
        };
        emit_unwind_signal(&diagnostic);
    }
}
