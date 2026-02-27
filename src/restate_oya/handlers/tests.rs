#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::runtime::{upsert_step, StepUpdate};
use super::status::{lifecycle_status_label, parse_lifecycle_status_snapshot};
use crate::lifecycle::workflow::LifecycleStepStatus;
use crate::restate_oya::types::LifecycleStepSnapshot;

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
