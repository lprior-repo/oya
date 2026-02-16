use oya::domain::*;

#[test]
fn given_closed_circuit_when_failure_threshold_reached_then_circuit_opens() {
    let config = CircuitConfig::new(3, 2, 60_000);
    let cb = CircuitBreaker::new("test-scope", config);

    let cb = cb.record_failure().record_failure().record_failure();

    assert_eq!(cb.state, CircuitState::Open);
    assert!(cb.opened_at.is_some());
}

#[test]
fn given_open_circuit_when_reset_timeout_elapsed_then_circuit_half_opens() {
    let config = CircuitConfig::new(2, 2, 1);
    let cb = CircuitBreaker::new("test-scope", config).record_failure().record_failure();

    assert_eq!(cb.state, CircuitState::Open);

    std::thread::sleep(std::time::Duration::from_millis(2));

    let cb = cb.try_half_open();
    assert_eq!(cb.state, CircuitState::HalfOpen);
}

#[test]
fn given_half_open_circuit_when_success_threshold_reached_then_circuit_closes() {
    let config = CircuitConfig::new(2, 2, 1);
    let mut cb = CircuitBreaker::new("test-scope", config).record_failure().record_failure();

    cb.state = CircuitState::HalfOpen;

    let cb = cb.record_success().record_success();
    assert_eq!(cb.state, CircuitState::Closed);
}

#[test]
fn circuit_state_allows_operations_when_closed_or_half_open() {
    assert!(CircuitState::Closed.allows_operations());
    assert!(CircuitState::HalfOpen.allows_operations());
    assert!(!CircuitState::Open.allows_operations());
}

#[test]
fn circuit_state_roundtrip_preserves_values() {
    let cases = [
        (CircuitState::Closed, "closed"),
        (CircuitState::Open, "open"),
        (CircuitState::HalfOpen, "half_open"),
    ];

    for (state, expected) in cases {
        assert_eq!(state.as_str(), expected);
        assert_eq!(CircuitState::try_from(expected), Ok(state));
    }
}

#[test]
fn given_operations_when_calculating_success_rate_then_returns_percentage() {
    let metrics = HealthMetrics::new(100, 80, 20, 0);
    assert_eq!(metrics.success_rate(), 80);
}

#[test]
fn given_no_operations_when_calculating_success_rate_then_returns_100() {
    let metrics = HealthMetrics::default();
    assert_eq!(metrics.success_rate(), 100);
}

#[test]
fn given_low_success_rate_and_sufficient_operations_when_checking_critical_then_returns_true() {
    let metrics = HealthMetrics::new(100, 30, 70, 0);
    assert!(metrics.is_critical(50));
}

#[test]
fn given_few_operations_when_checking_critical_then_returns_false() {
    let metrics = HealthMetrics::new(5, 1, 4, 0);
    assert!(!metrics.is_critical(50));
}

#[test]
fn given_healthmetrics_when_recording_operations_then_returns_immutable_new_state() {
    let metrics = HealthMetrics::new(10, 8, 2, 1);
    let after_success = metrics.record_success();

    assert_eq!(metrics.total_operations, 10);
    assert_eq!(after_success.total_operations, 11);
}

#[test]
fn given_fingerprint_with_high_idle_time_when_checking_stuck_then_returns_true() {
    let fp =
        BehavioralFingerprint::new("agent-1", Some("bead-123".to_string()), "implement", 0, 600, 0);
    assert!(fp.is_stuck(300, 5));
}

#[test]
fn given_fingerprint_with_high_consecutive_failures_when_checking_stuck_then_returns_true() {
    let fp =
        BehavioralFingerprint::new("agent-1", Some("bead-123".to_string()), "implement", 10, 60, 0);
    assert!(fp.is_stuck(300, 5));
}

#[test]
fn given_fingerprint_with_high_retry_count_when_checking_retry_loop_then_returns_true() {
    let fp =
        BehavioralFingerprint::new("agent-1", Some("bead-123".to_string()), "implement", 0, 60, 15);
    assert!(fp.is_retry_loop(10));
}

#[test]
fn given_healthy_fingerprint_when_checking_health_status_then_returns_healthy() {
    let fp =
        BehavioralFingerprint::new("agent-1", Some("bead-123".to_string()), "contract", 0, 60, 0);
    assert_eq!(fp.health_status(), AgentHealthStatus::Healthy);
}

#[test]
fn given_fingerprint_with_failures_when_checking_health_status_then_returns_degraded() {
    let fp =
        BehavioralFingerprint::new("agent-1", Some("bead-123".to_string()), "implement", 3, 60, 0);
    assert_eq!(fp.health_status(), AgentHealthStatus::Degraded);
}

#[test]
fn agent_health_status_needs_intervention_for_stuck_and_retry_loop() {
    assert!(!AgentHealthStatus::Healthy.needs_intervention());
    assert!(!AgentHealthStatus::Degraded.needs_intervention());
    assert!(AgentHealthStatus::Stuck.needs_intervention());
    assert!(AgentHealthStatus::RetryLoop.needs_intervention());
}

#[test]
fn given_contract_stage_when_stage_passes_then_advances_to_tdd15() {
    let decision = determine_transition(StageName::Contract, true, false);

    assert_eq!(decision.transition(), StageTransition::Advance(StageName::Tdd15));
}

#[test]
fn given_tdd15_stage_when_stage_passes_then_advances_to_qa() {
    let decision = determine_transition(StageName::Tdd15, true, false);

    assert_eq!(decision.transition(), StageTransition::Advance(StageName::Qa));
}

#[test]
fn given_red_queen_stage_when_stage_passes_then_advances_to_gpt_review() {
    let decision = determine_transition(StageName::RedQueen, true, false);

    assert_eq!(decision.transition(), StageTransition::Advance(StageName::GptReview));
}

#[test]
fn given_any_stage_when_stage_fails_and_retries_available_then_retry() {
    let decision = determine_transition(StageName::Contract, false, false);

    assert_eq!(decision.transition(), StageTransition::Retry);
}

#[test]
fn given_any_stage_when_stage_fails_and_retries_exhausted_then_block() {
    let decision = determine_transition(StageName::Contract, false, true);

    assert_eq!(decision.transition(), StageTransition::Block);
}

#[test]
fn given_ship_gate_when_stage_passes_then_completes() {
    let decision = determine_transition(StageName::ShipGate, true, false);

    assert_eq!(decision.transition(), StageTransition::Complete);
}

#[test]
fn given_stage_when_getting_next_stage_then_returns_correct_next() {
    assert_eq!(StageName::Contract.next(), Some(StageName::Tdd15));
    assert_eq!(StageName::Tdd15.next(), Some(StageName::Qa));
    assert_eq!(StageName::Qa.next(), Some(StageName::RedQueen));
    assert_eq!(StageName::RedQueen.next(), Some(StageName::GptReview));
    assert_eq!(StageName::GptReview.next(), Some(StageName::ShipGate));
    assert_eq!(StageName::ShipGate.next(), None);
}

#[test]
fn given_stage_when_getting_model_tier_then_returns_efficient_tier() {
    assert_eq!(StageName::Contract.model_for_stage(), ModelTier::Fast);
    assert_eq!(StageName::Tdd15.model_for_stage(), ModelTier::Balanced);
    assert_eq!(StageName::Qa.model_for_stage(), ModelTier::Balanced);
    assert_eq!(StageName::RedQueen.model_for_stage(), ModelTier::Capable);
    assert_eq!(StageName::GptReview.model_for_stage(), ModelTier::Capable);
    assert_eq!(StageName::ShipGate.model_for_stage(), ModelTier::Best);
}

#[test]
fn given_stage_when_getting_max_attempts_then_returns_three() {
    assert_eq!(StageName::Contract.max_attempts(), 3);
    assert_eq!(StageName::Tdd15.max_attempts(), 3);
    assert_eq!(StageName::Qa.max_attempts(), 3);
}

#[test]
fn given_stage_when_getting_gates_then_returns_appropriate_gates() {
    assert_eq!(StageName::Contract.gates(), vec![Gate::Compiles]);
    assert_eq!(StageName::Tdd15.gates(), vec![Gate::Compiles, Gate::TestsPass]);
    assert_eq!(StageName::ShipGate.gates(), vec![Gate::MoonCi, Gate::ZjjMergeQueue]);
}

#[test]
fn given_stage_name_roundtrip_then_preserves_value() {
    let stages = ["contract", "tdd15", "qa", "red_queen", "gpt_review", "ship_gate"];
    for s in stages {
        assert_eq!(StageName::try_from(s).map(|value| value.as_str()), Ok(s));
    }
}

#[test]
fn given_model_tier_roundtrip_then_preserves_value() {
    let tiers = ["fast", "balanced", "capable", "best"];
    for t in tiers {
        assert_eq!(ModelTier::try_from(t).map(|value| value.as_str()), Ok(t));
    }
}

#[test]
fn given_gate_roundtrip_then_preserves_value() {
    let gates = [
        "compiles",
        "tests_pass",
        "edge_cases",
        "no_vulnerabilities",
        "clippy_clean",
        "security",
        "moon_ci",
        "zjj_merge_queue",
    ];
    for g in gates {
        assert_eq!(Gate::try_from(g).map(|value| value.as_str()), Ok(g));
    }
}
