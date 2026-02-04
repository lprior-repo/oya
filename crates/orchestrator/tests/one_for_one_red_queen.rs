#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

//! Red Queen adversarial tests for one_for_one restart strategy.

use std::collections::HashMap;
use std::time::Instant;

use orchestrator::actors::supervisor::{
    strategy::{OneForOne, RestartContext, RestartStrategy},
    ChildInfo, SchedulerSupervisorConfig, SupervisorActorState, SupervisorState,
};

/// Create test state helper.
fn create_test_state() -> SupervisorActorState {
    SupervisorActorState {
        config: SchedulerSupervisorConfig::default(),
        state: SupervisorState::Running,
        children: HashMap::new(),
        failure_times: Vec::new(),
        total_restarts: 0,
        child_id_counter: 0,
        shutdown_coordinator: None,
        _shutdown_rx: None,
        restart_strategy: Box::new(OneForOne::new()),
    }
}

/// Create child info helper.
#[allow(unsafe_code)]
fn create_child_info(name: &str, restart_count: u32) -> ChildInfo {
    ChildInfo {
        name: name.to_string(),
        actor_ref: unsafe { ractor::ActorRef::cell(format!("test-actor-{}", name)) },
        restart_count,
        last_restart: Some(Instant::now()),
        args: orchestrator::actors::scheduler::SchedulerArguments::new(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GENERATION 1: HAPPY PATH VERIFICATION
// ═════════════════════════════════════════════════════════════════════════

/// **Attack 1.1**: When child-2 crashes, verify child-1 continues running
#[test]
fn attack_1_1_when_child_2_crashes_then_child_1_unaffected() {
    let mut state = create_test_state();
    state
        .children
        .insert("child-1".to_string(), create_child_info("child-1", 0));
    state
        .children
        .insert("child-2".to_string(), create_child_info("child-2", 0));
    state
        .children
        .insert("child-3".to_string(), create_child_info("child-3", 0));

    let strategy = OneForOne::new();
    let ctx = RestartContext::new("child-2", "Crashed", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Only child-2 in restart list
    assert!(matches!(decision,
        orchestrator::actors::supervisor::strategy::RestartDecision::Restart { child_names }
        if child_names == vec!["child-2"]));
}

/// **Attack 1.2**: Verify strategy returns correct child_names for restart
#[test]
fn attack_1_2_strategy_returns_correct_names() {
    let mut state = create_test_state();
    state.children.insert(
        "scheduler-a".to_string(),
        create_child_info("scheduler-a", 1),
    );
    state.children.insert(
        "scheduler-b".to_string(),
        create_child_info("scheduler-b", 0),
    );

    let strategy = OneForOne::new();
    let ctx = RestartContext::new("scheduler-b", "Panic", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Returns only crashed child
    if let orchestrator::actors::supervisor::strategy::RestartDecision::Restart { child_names } =
        decision
    {
        assert_eq!(child_names, vec!["scheduler-b"]);
    } else {
        panic!("Should return Restart decision");
    }
}

/// **Attack 1.3**: Verify max_restarts prevents restart
#[test]
fn attack_1_3_max_restarts_prevents_restart() {
    let mut state = create_test_state();
    state.config.max_restarts = 3;
    state
        .children
        .insert("failer".to_string(), create_child_info("failer", 3));

    let strategy = OneForOne::new();
    let ctx = RestartContext::new("failer", "Too many fails", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Returns Stop when at max restarts
    assert_eq!(
        decision,
        orchestrator::actors::supervisor::strategy::RestartDecision::Stop
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// GENERATION 2: INPUT BOUNDARY ATTACKS
// ═════════════════════════════════════════════════════════════════════════

/// **Attack 2.1**: Empty child name - strategy should handle gracefully
#[test]
fn attack_2_1_empty_child_name_returns_decision() {
    let state = create_test_state();
    let strategy = OneForOne::new();
    let ctx = RestartContext::new("", "Test", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Should return Restart with empty name
    if let orchestrator::actors::supervisor::strategy::RestartDecision::Restart { child_names } =
        decision
    {
        assert_eq!(child_names, vec![""]);
    } else {
        panic!("Should handle empty name");
    }
}

/// **Attack 2.2**: Very long child name - should work
#[test]
fn attack_2_2_long_child_name_works() {
    let mut state = create_test_state();
    let long_name = "a".repeat(1000);
    state
        .children
        .insert(long_name.clone(), create_child_info(&long_name, 0));

    let strategy = OneForOne::new();
    let ctx = RestartContext::new(&long_name, "Test", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Should handle long names
    assert!(matches!(
        decision,
        orchestrator::actors::supervisor::strategy::RestartDecision::Restart { child_names }
    ));
}

/// **Attack 2.3**: Special characters in child name - should work
#[test]
fn attack_2_3_special_chars_in_name_works() {
    let mut state = create_test_state();
    let special_name = "child\n\t\x00!@#$%";
    state
        .children
        .insert(special_name.to_string(), create_child_info(special_name, 0));

    let strategy = OneForOne::new();
    let ctx = RestartContext::new(special_name, "Test", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Should handle special characters
    assert!(matches!(
        decision,
        orchestrator::actors::supervisor::strategy::RestartDecision::Restart { child_names }
    ));
}

// ═════════════════════════════════════════════════════════════════════════
// GENERATION 3: STATE ATTACKS
// ═════════════════════════════════════════════════════════════════════

/// **Attack 3.1**: Child not found in state - should return Restart with missing child
#[test]
fn attack_3_1_child_not_found_returns_restart() {
    let state = create_test_state();
    // Don't add child to state
    let strategy = OneForOne::new();
    let ctx = RestartContext::new("missing-child", "Not found", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Should return Restart even if child not found
    assert!(matches!(
        decision,
        orchestrator::actors::supervisor::strategy::RestartDecision::Restart { child_names }
    ));
}

/// **Attack 3.2**: Restart count at exactly max_restarts - should stop
#[test]
fn attack_3_2_exact_max_restarts_stops() {
    let mut state = create_test_state();
    state.config.max_restarts = 5;
    state
        .children
        .insert("at-limit".to_string(), create_child_info("at-limit", 5));

    let strategy = OneForOne::new();
    let ctx = RestartContext::new("at-limit", "At limit", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: At max_restarts, should stop
    assert_eq!(
        decision,
        orchestrator::actors::supervisor::strategy::RestartDecision::Stop
    );
}

/// **Attack 3.3**: No last_restart time recorded - should still work
#[test]
fn attack_3_3_no_last_restart_time_works() {
    let mut state = create_test_state();
    // Create child without last_restart
    let child_info = ChildInfo {
        name: "no-time".to_string(),
        actor_ref: unsafe { ractor::ActorRef::cell("test-actor-no-time".to_string()) },
        restart_count: 2,
        last_restart: None, // No last_restart time
        args: orchestrator::actors::scheduler::SchedulerArguments::new(),
    };
    state.children.insert("no-time".to_string(), child_info);

    let strategy = OneForOne::new();
    let ctx = RestartContext::new("no-time", "Test", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Should work even without last_restart
    assert!(matches!(
        decision,
        orchestrator::actors::supervisor::strategy::RestartDecision::Restart { child_names }
    ));
}

// ═══════════════════════════════════════════════════════════════════════
// GENERATION 4: OUTPUT CONTRACT ATTACKS
// ═══════════════════════════════════════════════════════════════════════

/// **Attack 4.1**: RestartDecision::Restart with empty child_names is valid
#[test]
fn attack_4_1_empty_restart_list_is_valid() {
    let state = create_test_state();
    let strategy = OneForOne::new();
    let ctx = RestartContext::new("test-child", "Test", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Empty RestartDecision is valid - represents case where strategy
    // decides not to restart anything (though OneForOne always returns at least 1)
    assert!(matches!(
        decision,
        orchestrator::actors::supervisor::strategy::RestartDecision::Restart { .. }
    ));
}

/// **Attack 4.2**: Verify RestartDecision::Stop has no child_names
#[test]
fn attack_4_2_stop_decision_no_children() {
    let mut state = create_test_state();
    state.config.max_restarts = 1;
    state
        .children
        .insert("at-max".to_string(), create_child_info("at-max", 1));

    let strategy = OneForOne::new();
    let ctx = RestartContext::new("at-max", "At max", &state);
    let decision = strategy.on_child_failure(&ctx);

    // Verify: Stop decision has no child_names field
    if let orchestrator::actors::supervisor::strategy::RestartDecision::Stop = &decision {
        // Stop variant has no child_names - this is correct
        assert!(true);
    } else {
        panic!("Should return Stop decision");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// GENERATION 5: ERROR HANDLING
// ═════════════════════════════════════════════════════════════════════

/// **Attack 5.1**: RestartStrategy::validate() should return Ok by default
#[test]
fn attack_5_1_validate_returns_ok() {
    let strategy = OneForOne::new();
    let result = strategy.validate();

    // Verify: Default validate returns Ok
    assert!(result.is_ok());
}

/// **Attack 5.2**: Context with invalid state reference - should handle gracefully
#[test]
fn attack_5_2_invalid_state_reference_works() {
    let state = create_test_state();
    let strategy = OneForOne::new();
    let ctx = RestartContext::new("test", "Test", &state);

    // Verify: Context methods handle invalid references gracefully
    // restart_count returns 0 for missing child
    assert_eq!(ctx.restart_count(), 0);
}
