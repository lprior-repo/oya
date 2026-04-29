use super::super::*;

// ===========================================================================
// can_transition Tests
// ===========================================================================

#[test]
fn can_transition_returns_true_for_idle_to_queued() {
    assert!(can_transition(ExecutionState::Idle, ExecutionState::Queued));
}

#[test]
fn can_transition_returns_true_for_idle_to_skipped() {
    assert!(can_transition(ExecutionState::Idle, ExecutionState::Skipped));
}

#[test]
fn can_transition_returns_true_for_queued_to_running() {
    assert!(can_transition(ExecutionState::Queued, ExecutionState::Running));
}

#[test]
fn can_transition_returns_true_for_queued_to_skipped() {
    assert!(can_transition(ExecutionState::Queued, ExecutionState::Skipped));
}

#[test]
fn can_transition_returns_true_for_running_to_completed() {
    assert!(can_transition(ExecutionState::Running, ExecutionState::Completed));
}

#[test]
fn can_transition_returns_true_for_running_to_failed() {
    assert!(can_transition(ExecutionState::Running, ExecutionState::Failed));
}

#[test]
fn can_transition_returns_false_for_idle_to_running() {
    assert!(!can_transition(ExecutionState::Idle, ExecutionState::Running));
}

#[test]
fn can_transition_returns_false_for_idle_to_completed() {
    assert!(!can_transition(ExecutionState::Idle, ExecutionState::Completed));
}

#[test]
fn can_transition_returns_false_for_idle_to_failed() {
    assert!(!can_transition(ExecutionState::Idle, ExecutionState::Failed));
}

#[test]
fn can_transition_returns_false_for_queued_to_idle() {
    assert!(!can_transition(ExecutionState::Queued, ExecutionState::Idle));
}

#[test]
fn can_transition_returns_false_for_queued_to_completed() {
    assert!(!can_transition(ExecutionState::Queued, ExecutionState::Completed));
}

#[test]
fn can_transition_returns_false_for_queued_to_failed() {
    assert!(!can_transition(ExecutionState::Queued, ExecutionState::Failed));
}

#[test]
fn can_transition_returns_false_for_running_to_idle() {
    assert!(!can_transition(ExecutionState::Running, ExecutionState::Idle));
}

#[test]
fn can_transition_returns_false_for_running_to_queued() {
    assert!(!can_transition(ExecutionState::Running, ExecutionState::Queued));
}

#[test]
fn can_transition_returns_false_for_running_to_skipped() {
    assert!(!can_transition(ExecutionState::Running, ExecutionState::Skipped));
}

// ===========================================================================
// Completed: no outgoing transitions (split from multi-assertion test)
// ===========================================================================

#[test]
fn can_transition_returns_false_for_completed_to_idle() {
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Idle));
}

#[test]
fn can_transition_returns_false_for_completed_to_queued() {
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Queued));
}

#[test]
fn can_transition_returns_false_for_completed_to_running() {
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Running));
}

#[test]
fn can_transition_returns_false_for_completed_to_failed() {
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Failed));
}

#[test]
fn can_transition_returns_false_for_completed_to_skipped() {
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Skipped));
}

// ===========================================================================
// Failed: no outgoing transitions (split from multi-assertion test)
// ===========================================================================

#[test]
fn can_transition_returns_false_for_failed_to_idle() {
    assert!(!can_transition(ExecutionState::Failed, ExecutionState::Idle));
}

#[test]
fn can_transition_returns_false_for_failed_to_queued() {
    assert!(!can_transition(ExecutionState::Failed, ExecutionState::Queued));
}

#[test]
fn can_transition_returns_false_for_failed_to_running() {
    assert!(!can_transition(ExecutionState::Failed, ExecutionState::Running));
}

#[test]
fn can_transition_returns_false_for_failed_to_completed() {
    assert!(!can_transition(ExecutionState::Failed, ExecutionState::Completed));
}

#[test]
fn can_transition_returns_false_for_failed_to_skipped() {
    assert!(!can_transition(ExecutionState::Failed, ExecutionState::Skipped));
}

// ===========================================================================
// Skipped: no outgoing transitions (split from multi-assertion test)
// ===========================================================================

#[test]
fn can_transition_returns_false_for_skipped_to_idle() {
    assert!(!can_transition(ExecutionState::Skipped, ExecutionState::Idle));
}

#[test]
fn can_transition_returns_false_for_skipped_to_queued() {
    assert!(!can_transition(ExecutionState::Skipped, ExecutionState::Queued));
}

#[test]
fn can_transition_returns_false_for_skipped_to_running() {
    assert!(!can_transition(ExecutionState::Skipped, ExecutionState::Running));
}

#[test]
fn can_transition_returns_false_for_skipped_to_completed() {
    assert!(!can_transition(ExecutionState::Skipped, ExecutionState::Completed));
}

#[test]
fn can_transition_returns_false_for_skipped_to_failed() {
    assert!(!can_transition(ExecutionState::Skipped, ExecutionState::Failed));
}
// Self-transitions: identity transitions are never legal

#[test]
fn can_transition_returns_false_for_idle_to_idle() {
    assert!(!can_transition(ExecutionState::Idle, ExecutionState::Idle));
}

#[test]
fn can_transition_returns_false_for_queued_to_queued() {
    assert!(!can_transition(ExecutionState::Queued, ExecutionState::Queued));
}

#[test]
fn can_transition_returns_false_for_running_to_running() {
    assert!(!can_transition(ExecutionState::Running, ExecutionState::Running));
}

#[test]
fn can_transition_returns_false_for_completed_to_completed() {
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Completed));
}

#[test]
fn can_transition_returns_false_for_failed_to_failed() {
    assert!(!can_transition(ExecutionState::Failed, ExecutionState::Failed));
}

#[test]
fn can_transition_returns_false_for_skipped_to_skipped() {
    assert!(!can_transition(ExecutionState::Skipped, ExecutionState::Skipped));
}
