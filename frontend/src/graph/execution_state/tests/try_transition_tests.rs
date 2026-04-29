use super::super::*;

// ===========================================================================
// try_transition Valid Transitions Tests
// ===========================================================================

#[test]
fn try_transition_returns_some_for_idle_to_queued() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Queued);
    assert_eq!(result, Some(StateTransition::IdleToQueued));
}

#[test]
fn try_transition_returns_some_for_idle_to_skipped() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Skipped);
    assert_eq!(result, Some(StateTransition::IdleToSkipped));
}

#[test]
fn try_transition_returns_some_for_queued_to_running() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Running);
    assert_eq!(result, Some(StateTransition::QueuedToRunning));
}

#[test]
fn try_transition_returns_some_for_queued_to_skipped() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Skipped);
    assert_eq!(result, Some(StateTransition::QueuedToSkipped));
}

#[test]
fn try_transition_returns_some_for_running_to_completed() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Completed);
    assert_eq!(result, Some(StateTransition::RunningToCompleted));
}

#[test]
fn try_transition_returns_some_for_running_to_failed() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Failed);
    assert_eq!(result, Some(StateTransition::RunningToFailed));
}

// ===========================================================================
// try_transition Invalid Transitions Tests
// ===========================================================================

#[test]
fn try_transition_returns_none_for_idle_to_running() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Running);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_idle_to_completed() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Completed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_idle_to_failed() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Failed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_queued_to_idle() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Idle);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_queued_to_completed() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Completed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_queued_to_failed() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Failed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_running_to_idle() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Idle);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_running_to_queued() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Queued);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_running_to_skipped() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Skipped);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_completed_to_idle() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Idle);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_completed_to_queued() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Queued);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_completed_to_running() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Running);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_completed_to_failed() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Failed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_completed_to_skipped() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Skipped);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_failed_to_idle() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Idle);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_failed_to_queued() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Queued);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_failed_to_running() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Running);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_failed_to_completed() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Completed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_failed_to_skipped() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Skipped);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_skipped_to_idle() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Idle);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_skipped_to_queued() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Queued);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_skipped_to_running() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Running);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_skipped_to_completed() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Completed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_skipped_to_failed() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Failed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_completed_to_completed() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Completed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_failed_to_failed() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Failed);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_skipped_to_skipped() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Skipped);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_idle_to_idle() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Idle);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_queued_to_queued() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Queued);
    assert_eq!(result, None);
}

#[test]
fn try_transition_returns_none_for_running_to_running() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Running);
    assert_eq!(result, None);
}
