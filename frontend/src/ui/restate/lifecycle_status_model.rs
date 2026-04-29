#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Pure display model for Oya lifecycle status.

use oya_contracts::{LifecycleStatusSnapshot, LifecycleStepSnapshot};

const GOOD_STATUS: &[&str] = &["completed", "done", "passed", "success", "succeeded"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleStatusSummary {
    pub run_label: String,
    pub status_label: &'static str,
    pub badge_class: &'static str,
    pub progress_label: String,
    pub gate_label: String,
    pub compensation_label: String,
    pub message: Option<String>,
}

#[must_use]
pub fn lifecycle_summary(
    snapshot: &LifecycleStatusSnapshot,
    selected_run: Option<&str>,
) -> LifecycleStatusSummary {
    LifecycleStatusSummary {
        run_label: run_label(snapshot, selected_run),
        status_label: status_label(snapshot),
        badge_class: status_badge_class(snapshot),
        progress_label: progress_label(&snapshot.steps),
        gate_label: gate_label(snapshot),
        compensation_label: compensation_label(snapshot),
        message: snapshot.message.clone(),
    }
}

#[must_use]
pub fn run_label(snapshot: &LifecycleStatusSnapshot, selected_run: Option<&str>) -> String {
    if let Some(bead_id) = snapshot.bead_id.as_deref().filter(non_empty) {
        return format!("bead {bead_id}");
    }

    if let Some(run) = selected_run.filter(non_empty) {
        return format!("run {run}");
    }

    "current lifecycle".to_owned()
}

#[must_use]
pub const fn status_label(snapshot: &LifecycleStatusSnapshot) -> &'static str {
    match (snapshot.done, snapshot.success) {
        (_, Some(false)) => "failed",
        (true, Some(true)) => "succeeded",
        (true, None) => "done",
        (false, _) => "running",
    }
}

#[must_use]
pub const fn status_badge_class(snapshot: &LifecycleStatusSnapshot) -> &'static str {
    match (snapshot.done, snapshot.success) {
        (_, Some(false)) => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-red-50 text-red-700 border-red-200"
        }
        (true, Some(true)) => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-emerald-50 text-emerald-700 border-emerald-200"
        }
        (true, None) => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-slate-50 text-slate-600 border-slate-200"
        }
        (false, _) => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-blue-50 text-blue-700 border-blue-200"
        }
    }
}

#[must_use]
pub fn progress_label(steps: &[LifecycleStepSnapshot]) -> String {
    if steps.is_empty() {
        return "No lifecycle steps reported.".to_owned();
    }

    let complete = steps.iter().filter(|step| is_good_status(&step.status)).count();
    format!("{complete} of {} steps complete", steps.len())
}

#[must_use]
pub fn gate_label(snapshot: &LifecycleStatusSnapshot) -> String {
    let gate_count = snapshot.gates.len() + snapshot.discipline_gates.len();
    if gate_count == 0 {
        return "No gates reported.".to_owned();
    }

    let passed = snapshot
        .gates
        .iter()
        .chain(snapshot.discipline_gates.iter())
        .filter(|gate| is_good_status(&gate.status))
        .count();
    format!("{passed} of {gate_count} gates passed")
}

#[must_use]
pub fn compensation_label(snapshot: &LifecycleStatusSnapshot) -> String {
    if snapshot.compensation_diagnostics.is_empty() {
        return "No compensation diagnostics.".to_owned();
    }

    let ok = snapshot.compensation_diagnostics.iter().filter(|diag| diag.success).count();
    format!("{ok} of {} compensations succeeded", snapshot.compensation_diagnostics.len())
}

#[must_use]
pub fn is_good_status(status: &str) -> bool {
    GOOD_STATUS.iter().any(|candidate| status.eq_ignore_ascii_case(candidate))
}

fn non_empty(value: &&str) -> bool {
    !value.trim().is_empty()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use oya_contracts::{CompensationDiagnostic, LifecycleGateSnapshot, LifecycleStatusSnapshot};

    #[test]
    fn lifecycle_summary_prefers_bead_id_for_selected_run() {
        let snapshot = lifecycle_fixture(true, Some(true));

        let summary = lifecycle_summary(&snapshot, Some("run-123"));

        assert_eq!(summary.run_label, "bead oya-demo");
        assert_eq!(summary.status_label, "succeeded");
        assert_eq!(summary.progress_label, "1 of 2 steps complete");
        assert_eq!(summary.gate_label, "1 of 2 gates passed");
        assert_eq!(summary.compensation_label, "1 of 2 compensations succeeded");
    }

    #[test]
    fn lifecycle_summary_uses_selected_run_without_bead_id() {
        let mut snapshot = lifecycle_fixture(false, None);
        snapshot.bead_id = None;

        let summary = lifecycle_summary(&snapshot, Some("run-123"));

        assert_eq!(summary.run_label, "run run-123");
        assert_eq!(summary.status_label, "running");
    }

    #[test]
    fn lifecycle_summary_marks_failed_snapshot() {
        let snapshot = lifecycle_fixture(true, Some(false));

        let summary = lifecycle_summary(&snapshot, None);

        assert_eq!(summary.status_label, "failed");
        assert!(summary.badge_class.contains("red"));
    }

    fn lifecycle_fixture(done: bool, success: Option<bool>) -> LifecycleStatusSnapshot {
        LifecycleStatusSnapshot {
            bead_id: Some("oya-demo".to_owned()),
            steps: vec![step("contract", "succeeded"), step("qa", "running")],
            gates: vec![gate("fmt", "passed")],
            discipline_gates: vec![gate("red-queen", "pending")],
            state: None,
            pr_url: Some("https://github.com/priorlewis43/oya/pull/1".to_owned()),
            done,
            success,
            message: Some("Lifecycle active".to_owned()),
            compensation_diagnostics: vec![diag(true), diag(false)],
        }
    }

    fn step(step: &str, status: &str) -> LifecycleStepSnapshot {
        LifecycleStepSnapshot {
            step: step.to_owned(),
            status: status.to_owned(),
            message: None,
            details: None,
            started_at: None,
            finished_at: None,
            duration_ms: None,
        }
    }

    fn gate(gate_id: &str, status: &str) -> LifecycleGateSnapshot {
        LifecycleGateSnapshot {
            gate_id: gate_id.to_owned(),
            status: status.to_owned(),
            message: None,
        }
    }

    fn diag(success: bool) -> CompensationDiagnostic {
        CompensationDiagnostic {
            compensation_type: "rollback".to_owned(),
            target: "oya-demo".to_owned(),
            success,
            error: None,
        }
    }
}
