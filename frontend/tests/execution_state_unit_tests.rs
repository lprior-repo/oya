//! Unit tests for ExecutionState enum operations
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{can_transition, try_transition, ExecutionState, StateTransition};

// ===========================================================================
// Default State Tests
// ===========================================================================

#[test]
fn execution_state_default_is_idle() {
    let state = ExecutionState::default();
    assert_eq!(state, ExecutionState::Idle);
}

// ===========================================================================
// is_terminal Tests
// ===========================================================================

#[test]
fn is_terminal_returns_false_for_idle() {
    assert!(!ExecutionState::Idle.is_terminal());
}

#[test]
fn is_terminal_returns_false_for_queued() {
    assert!(!ExecutionState::Queued.is_terminal());
}

#[test]
fn is_terminal_returns_false_for_running() {
    assert!(!ExecutionState::Running.is_terminal());
}

#[test]
fn is_terminal_returns_true_for_completed() {
    assert!(ExecutionState::Completed.is_terminal());
}

#[test]
fn is_terminal_returns_true_for_failed() {
    assert!(ExecutionState::Failed.is_terminal());
}

#[test]
fn is_terminal_returns_true_for_skipped() {
    assert!(ExecutionState::Skipped.is_terminal());
}

// ===========================================================================
// is_active Tests
// ===========================================================================

#[test]
fn is_active_returns_true_for_queued() {
    assert!(ExecutionState::Queued.is_active());
}

#[test]
fn is_active_returns_true_for_running() {
    assert!(ExecutionState::Running.is_active());
}

#[test]
fn is_active_returns_false_for_idle() {
    assert!(!ExecutionState::Idle.is_active());
}

#[test]
fn is_active_returns_false_for_completed() {
    assert!(!ExecutionState::Completed.is_active());
}

#[test]
fn is_active_returns_false_for_failed() {
    assert!(!ExecutionState::Failed.is_active());
}

#[test]
fn is_active_returns_false_for_skipped() {
    assert!(!ExecutionState::Skipped.is_active());
}

// ===========================================================================
// try_transition Valid Transitions Tests
// ===========================================================================

#[test]
fn try_transition_returns_some_for_idle_to_queued() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Queued);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), StateTransition::IdleToQueued);
}

#[test]
fn try_transition_returns_some_for_idle_to_skipped() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Skipped);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), StateTransition::IdleToSkipped);
}

#[test]
fn try_transition_returns_some_for_queued_to_running() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Running);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), StateTransition::QueuedToRunning);
}

#[test]
fn try_transition_returns_some_for_queued_to_skipped() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Skipped);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), StateTransition::QueuedToSkipped);
}

#[test]
fn try_transition_returns_some_for_running_to_completed() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Completed);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), StateTransition::RunningToCompleted);
}

#[test]
fn try_transition_returns_some_for_running_to_failed() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Failed);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), StateTransition::RunningToFailed);
}

// ===========================================================================
// try_transition Invalid Transitions Tests (36 total pairs)
// ===========================================================================

#[test]
fn try_transition_returns_none_for_completed_to_running() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Running);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_completed_to_failed() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Failed);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_completed_to_skipped() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Skipped);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_completed_to_idle() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Idle);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_completed_to_queued() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Queued);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_running_to_idle() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Idle);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_running_to_queued() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Queued);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_running_to_skipped() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Skipped);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_queued_to_idle() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Idle);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_queued_to_completed() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Completed);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_queued_to_failed() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Failed);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_idle_to_running() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Running);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_idle_to_completed() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Completed);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_idle_to_failed() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Failed);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_failed_to_running() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Running);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_failed_to_completed() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Completed);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_failed_to_skipped() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Skipped);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_failed_to_idle() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Idle);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_failed_to_queued() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Queued);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_skipped_to_running() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Running);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_skipped_to_completed() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Completed);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_skipped_to_failed() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Failed);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_skipped_to_idle() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Idle);
    assert!(result.is_none());
}

#[test]
fn try_transition_returns_none_for_skipped_to_queued() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Queued);
    assert!(result.is_none());
}

// ===========================================================================
// Reflexive Transition Tests (all 6 states)
// ===========================================================================

#[test]
fn try_transition_rejects_self_transition_idle() {
    let result = try_transition(ExecutionState::Idle, ExecutionState::Idle);
    assert!(result.is_none());
}

#[test]
fn try_transition_rejects_self_transition_queued() {
    let result = try_transition(ExecutionState::Queued, ExecutionState::Queued);
    assert!(result.is_none());
}

#[test]
fn try_transition_rejects_self_transition_running() {
    let result = try_transition(ExecutionState::Running, ExecutionState::Running);
    assert!(result.is_none());
}

#[test]
fn try_transition_rejects_self_transition_completed() {
    let result = try_transition(ExecutionState::Completed, ExecutionState::Completed);
    assert!(result.is_none());
}

#[test]
fn try_transition_rejects_self_transition_failed() {
    let result = try_transition(ExecutionState::Failed, ExecutionState::Failed);
    assert!(result.is_none());
}

#[test]
fn try_transition_rejects_self_transition_skipped() {
    let result = try_transition(ExecutionState::Skipped, ExecutionState::Skipped);
    assert!(result.is_none());
}

// ===========================================================================
// can_transition Tests
// ===========================================================================

#[test]
fn can_transition_returns_true_for_valid_transition() {
    assert!(can_transition(ExecutionState::Idle, ExecutionState::Queued));
}

#[test]
fn can_transition_returns_false_for_invalid_transition() {
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Running));
}

#[test]
fn can_transition_returns_false_for_backwards_transition() {
    assert!(!can_transition(ExecutionState::Running, ExecutionState::Idle));
}

#[test]
fn can_transition_returns_false_for_terminal_state_transition() {
    assert!(!can_transition(ExecutionState::Failed, ExecutionState::Queued));
}
