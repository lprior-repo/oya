//! Symbol verification tests for `execution_state` module split (oya-frontend-2e4).
//!
//! These tests verify public symbol accessibility (INV-7) and import resolution
//! through crate::graph:: re-exports after the refactoring.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use super::super::*;

// ---------------------------------------------------------------------------
// TerminalState helper — defined before use (fixes items_after_statements)
// ---------------------------------------------------------------------------

fn assert_terminal<T: TerminalState>() {}

// ---------------------------------------------------------------------------
// INV-7: Public symbols accessible — split by category, no loops
// ---------------------------------------------------------------------------

#[test]
fn execution_state_variants_accessible() {
    let _idle: ExecutionState = ExecutionState::Idle;
    let _queued: ExecutionState = ExecutionState::Queued;
    let _running: ExecutionState = ExecutionState::Running;
    let _completed: ExecutionState = ExecutionState::Completed;
    let _failed: ExecutionState = ExecutionState::Failed;
    let _skipped: ExecutionState = ExecutionState::Skipped;
}

#[test]
fn execution_state_predicates_correct() {
    assert!(!ExecutionState::Idle.is_terminal());
    assert!(ExecutionState::Completed.is_terminal());
    assert!(ExecutionState::Running.is_active());
    assert!(!ExecutionState::Idle.is_active());
    assert!(ExecutionState::Idle.is_idle());
    assert!(!ExecutionState::Running.is_idle());
}

#[test]
fn invalid_transition_constructs_correctly() {
    let err = InvalidTransition::new(ExecutionState::Idle, ExecutionState::Running);
    assert_eq!(err.from_state(), ExecutionState::Idle);
    assert_eq!(err.to_state(), ExecutionState::Running);
}

#[test]
fn type_state_constructors_accessible() {
    let _idle: IdleState = IdleState;
    let _queued: QueuedState = IdleState.queue();
    let _skipped_from_idle: SkippedState = IdleState.skip();
    let _running: RunningState = QueuedState.start();
    let _skipped_from_queued: SkippedState = QueuedState.skip();
    let _completed: CompletedState = RunningState.complete();
    let _failed: FailedState = RunningState.fail();
}

#[test]
fn state_transition_enum_accessible() {
    let _transition: StateTransition = StateTransition::IdleToQueued;
    assert_eq!(StateTransition::IdleToQueued.apply(), ExecutionState::Queued);
    assert_eq!(
        StateTransition::IdleToQueued.from_states(),
        (ExecutionState::Idle, ExecutionState::Queued)
    );
}

#[test]
fn free_functions_return_correct_values() {
    assert_eq!(
        try_transition(ExecutionState::Idle, ExecutionState::Queued),
        Some(StateTransition::IdleToQueued)
    );
    assert!(can_transition(ExecutionState::Idle, ExecutionState::Queued));
    assert_eq!(try_transition(ExecutionState::Completed, ExecutionState::Idle), None);
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Idle));
}

#[test]
fn terminal_state_trait_bounds_correct() {
    assert_terminal::<CompletedState>();
    assert_terminal::<FailedState>();
    assert_terminal::<SkippedState>();
}

// ---------------------------------------------------------------------------
// Public imports resolve through crate::graph:: re-exports
// ---------------------------------------------------------------------------

#[test]
fn crate_graph_re_exports_all_enums() {
    let _es: crate::graph::ExecutionState = crate::graph::ExecutionState::Idle;
    let _err: crate::graph::InvalidTransition = crate::graph::InvalidTransition::new(
        crate::graph::ExecutionState::Idle,
        crate::graph::ExecutionState::Running,
    );
    let _t: crate::graph::StateTransition = crate::graph::StateTransition::IdleToQueued;
}

#[test]
fn crate_graph_re_exports_all_type_states() {
    let _idle: crate::graph::IdleState = crate::graph::IdleState;
    let _queued: crate::graph::QueuedState = crate::graph::QueuedState;
    let _running: crate::graph::RunningState = crate::graph::RunningState;
    let _completed: crate::graph::CompletedState = crate::graph::CompletedState;
    let _failed: crate::graph::FailedState = crate::graph::FailedState;
    let _skipped: crate::graph::SkippedState = crate::graph::SkippedState;
}

#[test]
fn crate_graph_functions_return_correct_values() {
    // L1 fix: assert try_transition return value (was silent `let _ =`)
    assert_eq!(
        crate::graph::try_transition(
            crate::graph::ExecutionState::Idle,
            crate::graph::ExecutionState::Queued,
        ),
        Some(crate::graph::StateTransition::IdleToQueued)
    );
    // L2 fix: assert can_transition return value (was silent `let _ =`)
    assert!(crate::graph::can_transition(
        crate::graph::ExecutionState::Idle,
        crate::graph::ExecutionState::Queued,
    ));
}

#[test]
fn crate_graph_re_exports_terminal_trait() {
    fn check_terminal<T: crate::graph::TerminalState>() {}
    check_terminal::<crate::graph::CompletedState>();
    check_terminal::<crate::graph::FailedState>();
    check_terminal::<crate::graph::SkippedState>();
}
