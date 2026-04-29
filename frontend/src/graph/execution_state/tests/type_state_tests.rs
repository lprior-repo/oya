use super::super::*;

// ===========================================================================
// IdleState Tests
// ===========================================================================

#[test]
fn idle_state_queue_returns_queued_state() {
    let idle = IdleState;
    let queued = idle.queue();
    assert_eq!(queued, QueuedState);
}

#[test]
fn idle_state_skip_returns_skipped_state() {
    let idle = IdleState;
    let skipped = idle.skip();
    assert_eq!(skipped, SkippedState);
}

// ===========================================================================
// QueuedState Tests
// ===========================================================================

#[test]
fn queued_state_start_returns_running_state() {
    let queued = QueuedState;
    let running = queued.start();
    assert_eq!(running, RunningState);
}

#[test]
fn queued_state_skip_returns_skipped_state() {
    let queued = QueuedState;
    let skipped = queued.skip();
    assert_eq!(skipped, SkippedState);
}

// ===========================================================================
// RunningState Tests
// ===========================================================================

#[test]
fn running_state_complete_returns_completed_state() {
    let running = RunningState;
    let completed = running.complete();
    assert_eq!(completed, CompletedState);
}

#[test]
fn running_state_fail_returns_failed_state() {
    let running = RunningState;
    let failed = running.fail();
    assert_eq!(failed, FailedState);
}

// ===========================================================================
// From Trait Conversion Tests
// ===========================================================================

#[test]
fn idle_state_converts_to_execution_state_idle() {
    let state: ExecutionState = IdleState.into();
    assert_eq!(state, ExecutionState::Idle);
}

#[test]
fn queued_state_converts_to_execution_state_queued() {
    let state: ExecutionState = QueuedState.into();
    assert_eq!(state, ExecutionState::Queued);
}

#[test]
fn running_state_converts_to_execution_state_running() {
    let state: ExecutionState = RunningState.into();
    assert_eq!(state, ExecutionState::Running);
}

#[test]
fn completed_state_converts_to_execution_state_completed() {
    let state: ExecutionState = CompletedState.into();
    assert_eq!(state, ExecutionState::Completed);
}

#[test]
fn failed_state_converts_to_execution_state_failed() {
    let state: ExecutionState = FailedState.into();
    assert_eq!(state, ExecutionState::Failed);
}

#[test]
fn skipped_state_converts_to_execution_state_skipped() {
    let state: ExecutionState = SkippedState.into();
    assert_eq!(state, ExecutionState::Skipped);
}

// ===========================================================================
// Static Trait Tests
// ===========================================================================

#[test]
fn completed_state_implements_terminal_state() {
    fn assert_terminal<T: TerminalState>() {}
    assert_terminal::<CompletedState>();
}

#[test]
fn failed_state_implements_terminal_state() {
    fn assert_terminal<T: TerminalState>() {}
    assert_terminal::<FailedState>();
}

#[test]
fn skipped_state_implements_terminal_state() {
    fn assert_terminal<T: TerminalState>() {}
    assert_terminal::<SkippedState>();
}

#[test]
fn idle_state_does_not_implement_terminal_state() {
    fn assert_non_terminal<T>()
    where
        T: Copy,
    {
    }
    assert_non_terminal::<IdleState>();
}

#[test]
fn queued_state_does_not_implement_terminal_state() {
    fn assert_non_terminal<T>()
    where
        T: Copy,
    {
    }
    assert_non_terminal::<QueuedState>();
}

#[test]
fn running_state_does_not_implement_terminal_state() {
    fn assert_non_terminal<T>()
    where
        T: Copy,
    {
    }
    assert_non_terminal::<RunningState>();
}

// ===========================================================================
// Type State Clone and Copy Tests
// ===========================================================================

#[test]
fn idle_state_is_copy() {
    let state = IdleState;
    let state2 = state;
    let state3 = state;
    assert_eq!(state2, IdleState);
    assert_eq!(state3, IdleState);
}

#[test]
fn queued_state_is_copy() {
    let state = QueuedState;
    let state2 = state;
    let state3 = state;
    assert_eq!(state2, QueuedState);
    assert_eq!(state3, QueuedState);
}

#[test]
fn running_state_is_copy() {
    let state = RunningState;
    let state2 = state;
    let state3 = state;
    assert_eq!(state2, RunningState);
    assert_eq!(state3, RunningState);
}

#[test]
fn completed_state_is_copy() {
    let state = CompletedState;
    let state2 = state;
    let state3 = state;
    assert_eq!(state2, CompletedState);
    assert_eq!(state3, CompletedState);
}

#[test]
fn failed_state_is_copy() {
    let state = FailedState;
    let state2 = state;
    let state3 = state;
    assert_eq!(state2, FailedState);
    assert_eq!(state3, FailedState);
}

#[test]
fn skipped_state_is_copy() {
    let state = SkippedState;
    let state2 = state;
    let state3 = state;
    assert_eq!(state2, SkippedState);
    assert_eq!(state3, SkippedState);
}

// ===========================================================================
// Type State Default Tests
// ===========================================================================

#[test]
fn idle_state_default_is_idle() {
    let state = IdleState::default();
    assert_eq!(state, IdleState);
}
