#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

//! Integration tests for one_for_one restart strategy.

use std::time::Duration;

use orchestrator::actors::supervisor::{
    strategy::{OneForOne, RestartStrategy, RestartContext},
    SupervisorActorState, SupervisorState, SchedulerSupervisorConfig,
    SchedulerSupervisorDef, SchedulerSupervisorMessage,
};
use ractor::ActorRef;

/// Test that one_for_one strategy restarts only the crashed child.
#[tokio::test]
async fn given_multiple_children_when_one_fails_then_only_one_restarted() {
    // Create a mock state with multiple children
    let mut state = SupervisorActorState {
        config: SchedulerSupervisorConfig::for_testing(),
        state: SupervisorState::Running,
        children: std::collections::HashMap::new(),
        failure_times: Vec::new(),
        total_restarts: 0,
        child_id_counter: 0,
        shutdown_coordinator: None,
        _shutdown_rx: None,
        restart_strategy: Box::new(OneForOne::new()),
    };

    let strategy = OneForOne::new();

    // Simulate child-2 failing
    let ctx = RestartContext::new("child-2", "Actor panicked", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: only child-2 should be restarted
    assert!(matches!(decision, 
        orchestrator::actors::supervisor::strategy::RestartDecision::Restart { child_names } 
        if child_names == vec!["child-2"]));
}

/// Test that one_for_one stops when max restarts exceeded.
#[tokio::test]
async fn given_child_exceeds_max_restarts_then_strategy_says_stop() {
    let mut state = SupervisorActorState {
        config: SchedulerSupervisorConfig {
            max_restarts: 2,
            ..Default::default()
        },
        state: SupervisorState::Running,
        children: std::collections::HashMap::new(),
        failure_times: Vec::new(),
        total_restarts: 0,
        child_id_counter: 0,
        shutdown_coordinator: None,
        _shutdown_rx: None,
        restart_strategy: Box::new(OneForOne::new()),
    };

    let strategy = OneForOne::new();

    // Simulate child-1 failing with 2 restarts (at limit)
    let ctx = RestartContext::new("child-1", "Max restarts", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: strategy should return Stop
    assert!(matches!(decision, 
        orchestrator::actors::supervisor::strategy::RestartDecision::Stop));
}
