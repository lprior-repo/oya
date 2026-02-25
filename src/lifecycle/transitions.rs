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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{BeadId, BookmarkName, FailureCategory, PrNumber};

    fn bead() -> Result<BeadData, Box<dyn std::error::Error>> {
        BeadId::parse("src-31nq")
            .map(BeadData::from_bead_id)
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
    }

    #[test]
    fn workspace_transition_from_planned() -> Result<(), Box<dyn std::error::Error>> {
        let initial = planned_state(bead()?);
        let next = apply_event(&initial, LifecycleEvent::WorkspacePrepared);
        assert!(matches!(next, Ok(LifecycleState { phase: Phase::WorkspaceReady(_) })));
        Ok(())
    }

    #[test]
    fn pr_opened_transition_from_workspace_ready() -> Result<(), Box<dyn std::error::Error>> {
        let initial = LifecycleState { phase: Phase::WorkspaceReady(bead()?) };
        let bead_id = BeadId::parse("src-31nq")?;
        let pr_number = PrNumber::new(42)?;
        let pr = PrInfo {
            number: pr_number,
            bookmark: BookmarkName::from_bead_id(&bead_id),
            url: "https://example.test/pr/42".to_owned(),
        };
        let next = apply_event(&initial, LifecycleEvent::PullRequestOpened(pr));
        assert!(matches!(next, Ok(LifecycleState { phase: Phase::PrOpen { .. } })));
        Ok(())
    }

    #[test]
    fn terminal_failure_transitions_to_failed() -> Result<(), Box<dyn std::error::Error>> {
        let initial = planned_state(bead()?);
        let error = LifecycleError::terminal(FailureCategory::Workspace, "broken workspace");
        let next = apply_event(&initial, LifecycleEvent::Failed(error.clone()));
        assert!(matches!(next, Ok(LifecycleState { phase: Phase::Failed { .. } })));
        let failed = next.ok();
        assert!(failed.is_some());
        let phase = failed.map(|state| state.phase);
        assert!(matches!(phase, Some(Phase::Failed { error: ref saved, .. }) if saved == &error));
        Ok(())
    }
}
