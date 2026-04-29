use super::super::*;

// ===========================================================================
// InvalidTransition Display Tests
// ===========================================================================

#[test]
fn invalid_transition_display_shows_from_and_to() {
    let err = InvalidTransition::new(ExecutionState::Idle, ExecutionState::Running);
    assert_eq!(format!("{}", err), "Invalid state transition: idle -> running");
}

#[test]
fn invalid_transition_display_shows_queued_to_failed() {
    let err = InvalidTransition::new(ExecutionState::Queued, ExecutionState::Failed);
    assert_eq!(format!("{}", err), "Invalid state transition: queued -> failed");
}

#[test]
fn invalid_transition_display_shows_running_to_idle() {
    let err = InvalidTransition::new(ExecutionState::Running, ExecutionState::Idle);
    assert_eq!(format!("{}", err), "Invalid state transition: running -> idle");
}

// ===========================================================================
// InvalidTransition Error Trait Tests
// ===========================================================================

#[test]
fn invalid_transition_implements_error_trait() {
    let err: Box<dyn std::error::Error> =
        Box::new(InvalidTransition::new(ExecutionState::Idle, ExecutionState::Completed));
    assert!(err.to_string().contains("Invalid state transition"));
}

// ===========================================================================
// StateTransition apply Tests
// ===========================================================================

#[test]
fn state_transition_apply_idle_to_queued_returns_queued() {
    let transition = StateTransition::IdleToQueued;
    assert_eq!(transition.apply(), ExecutionState::Queued);
}

#[test]
fn state_transition_apply_idle_to_skipped_returns_skipped() {
    let transition = StateTransition::IdleToSkipped;
    assert_eq!(transition.apply(), ExecutionState::Skipped);
}

#[test]
fn state_transition_apply_queued_to_running_returns_running() {
    let transition = StateTransition::QueuedToRunning;
    assert_eq!(transition.apply(), ExecutionState::Running);
}

#[test]
fn state_transition_apply_queued_to_skipped_returns_skipped() {
    let transition = StateTransition::QueuedToSkipped;
    assert_eq!(transition.apply(), ExecutionState::Skipped);
}

#[test]
fn state_transition_apply_running_to_completed_returns_completed() {
    let transition = StateTransition::RunningToCompleted;
    assert_eq!(transition.apply(), ExecutionState::Completed);
}

#[test]
fn state_transition_apply_running_to_failed_returns_failed() {
    let transition = StateTransition::RunningToFailed;
    assert_eq!(transition.apply(), ExecutionState::Failed);
}

// ===========================================================================
// StateTransition from_states Tests
// ===========================================================================

#[test]
fn state_transition_from_states_idle_to_queued_returns_tuple() {
    let transition = StateTransition::IdleToQueued;
    assert_eq!(transition.from_states(), (ExecutionState::Idle, ExecutionState::Queued));
}

#[test]
fn state_transition_from_states_idle_to_skipped_returns_tuple() {
    let transition = StateTransition::IdleToSkipped;
    assert_eq!(transition.from_states(), (ExecutionState::Idle, ExecutionState::Skipped));
}

#[test]
fn state_transition_from_states_queued_to_running_returns_tuple() {
    let transition = StateTransition::QueuedToRunning;
    assert_eq!(transition.from_states(), (ExecutionState::Queued, ExecutionState::Running));
}

#[test]
fn state_transition_from_states_queued_to_skipped_returns_tuple() {
    let transition = StateTransition::QueuedToSkipped;
    assert_eq!(transition.from_states(), (ExecutionState::Queued, ExecutionState::Skipped));
}

#[test]
fn state_transition_from_states_running_to_completed_returns_tuple() {
    let transition = StateTransition::RunningToCompleted;
    assert_eq!(transition.from_states(), (ExecutionState::Running, ExecutionState::Completed));
}

#[test]
fn state_transition_from_states_running_to_failed_returns_tuple() {
    let transition = StateTransition::RunningToFailed;
    assert_eq!(transition.from_states(), (ExecutionState::Running, ExecutionState::Failed));
}

// ===========================================================================
// StateTransition Clone and Copy Tests
// ===========================================================================

#[test]
fn state_transition_is_copy() {
    let transition = StateTransition::IdleToQueued;
    let transition2 = transition;
    let transition3 = transition;
    assert_eq!(transition2, StateTransition::IdleToQueued);
    assert_eq!(transition3, StateTransition::IdleToQueued);
}

#[test]
fn state_transition_is_clone() {
    let transition = StateTransition::QueuedToRunning;
    let cloned = transition;
    assert_eq!(cloned, StateTransition::QueuedToRunning);
}

// ===========================================================================
// InvalidTransition Clone and Copy Tests
// ===========================================================================

#[test]
fn invalid_transition_is_copy() {
    let err = InvalidTransition::new(ExecutionState::Idle, ExecutionState::Running);
    let err2 = err;
    let err3 = err;
    assert_eq!(err2.from_state(), ExecutionState::Idle);
    assert_eq!(err3.from_state(), ExecutionState::Idle);
}

#[test]
fn invalid_transition_is_clone() {
    let err = InvalidTransition::new(ExecutionState::Queued, ExecutionState::Failed);
    let cloned = err;
    assert_eq!(cloned.from_state(), ExecutionState::Queued);
    assert_eq!(cloned.to_state(), ExecutionState::Failed);
}
