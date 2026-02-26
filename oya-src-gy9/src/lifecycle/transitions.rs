#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::{
    BeadData, LifecycleError, LifecycleResult, LifecycleState, Phase, PrInfo,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    WorkspacePrepared,
    PullRequestOpened(PrInfo),
    Completed,
    Failed(LifecycleError),
}

/// Applies one pure lifecycle event transition.
///
/// # Errors
/// Returns a terminal validation error for invalid phase/event combinations.
pub fn apply_event(
    state: &LifecycleState,
    event: LifecycleEvent,
) -> Result<LifecycleState, LifecycleError> {
    match (&state.phase, event) {
        (Phase::Planned(bead), LifecycleEvent::WorkspacePrepared) => {
            Ok(LifecycleState { phase: Phase::WorkspaceReady(bead.clone()) })
        }
        (Phase::WorkspaceReady(bead), LifecycleEvent::PullRequestOpened(pr)) => {
            Ok(LifecycleState { phase: Phase::PrOpen { bead: bead.clone(), pr } })
        }
        (Phase::PrOpen { bead, pr }, LifecycleEvent::Completed) => Ok(LifecycleState {
            phase: Phase::Completed(LifecycleResult { bead: bead.clone(), pr: Some(pr.clone()) }),
        }),
        (Phase::WorkspaceReady(bead), LifecycleEvent::Completed) => Ok(LifecycleState {
            phase: Phase::Completed(LifecycleResult { bead: bead.clone(), pr: None }),
        }),
        (
            Phase::Planned(bead) | Phase::WorkspaceReady(bead) | Phase::PrOpen { bead, .. },
            LifecycleEvent::Failed(error),
        ) => Ok(LifecycleState { phase: Phase::Failed { bead: bead.clone(), error } }),
        (Phase::Completed(_) | Phase::Failed { .. }, LifecycleEvent::Failed(error)) => Err(error),
        (_, invalid) => Err(LifecycleError::terminal(
            crate::lifecycle::types::FailureCategory::Validation,
            format!("invalid transition from {:?} with {:?}", state.phase, invalid),
        )),
    }
}

#[must_use]
pub fn planned_state(bead: BeadData) -> LifecycleState {
    LifecycleState { phase: Phase::Planned(bead) }
}
