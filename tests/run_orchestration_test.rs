#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use chrono::Utc;
use oya::domain::{AgentId, AgentState, AgentStatus, RunState};
use oya::domain::{BeadId, Run, StageName, StageResult};

#[test]
fn test_starts_new_run_in_pending_state() {
    let bead_id = BeadId::new("test-bead-1");
    let run = Run::new(bead_id.clone());

    assert_eq!(run.state, RunState::Pending);
    assert_eq!(run.bead_id, bead_id);
    assert!(run.created_at <= Utc::now());
}

#[test]
fn test_transitions_run_to_next_stage_on_success() {
    let bead_id = BeadId::new("test-bead-2");
    let mut run = Run::new(bead_id);
    run = run.start().expect("Should start");

    if let RunState::Running { current_stage } = &run.state {
        assert_eq!(*current_stage, StageName::Contract);
    } else {
        panic!("Run should be running");
    }

    let stage_result = StageResult {
        run_id: run.id.as_str().to_string(),
        stage: StageName::Contract,
        attempt: 1,
        passed: true,
        output: serde_json::json!({}),
        failure_category: None,
        next_stage: Some(StageName::Tdd15),
    };

    run = run.complete_stage(StageName::Contract, stage_result).expect("Should complete stage");

    if let RunState::Running { current_stage } = &run.state {
        assert_eq!(*current_stage, StageName::Tdd15);
    } else {
        panic!("Run should be running in Tdd15");
    }
}

#[test]
fn test_completes_run_when_all_stages_pass() {
    let bead_id = BeadId::new("test-bead-3");
    let mut run = Run::new(bead_id);
    run = run.start().expect("Should start");

    // Fast forward to last stage
    if let RunState::Running { ref mut current_stage } = run.state {
        *current_stage = StageName::ShipGate;
    }

    let stage_result = StageResult {
        run_id: run.id.as_str().to_string(),
        stage: StageName::ShipGate,
        attempt: 1,
        passed: true,
        output: serde_json::json!({}),
        failure_category: None,
        next_stage: None,
    };

    run = run.complete_stage(StageName::ShipGate, stage_result).expect("Should complete stage");

    match run.state {
        RunState::Shipped { .. } => {}
        _ => panic!("Run should be shipped"),
    }
}

#[test]
fn test_fails_run_when_critical_stage_fails() {
    let bead_id = BeadId::new("test-bead-fail");
    let mut run = Run::new(bead_id);
    run = run.start().expect("Should start");

    let fail_reason = "Critical failure".to_string();
    run = run.fail(fail_reason.clone());

    match run.state {
        RunState::Failed { reason, .. } => assert_eq!(reason, fail_reason),
        _ => panic!("Run should be failed"),
    }
}

#[test]
fn test_agent_state_invariants() {
    let agent_id = AgentId::new();

    // Invalid: Working without bead
    let state =
        AgentState::new(agent_id.clone(), None, Some(StageName::Contract), AgentStatus::Working, 1);
    assert!(state.validate_invariants().is_err());

    // Valid: Working with bead and stage
    let state = AgentState::new(
        agent_id.clone(),
        Some(BeadId::new("b1")),
        Some(StageName::Contract),
        AgentStatus::Working,
        1,
    );
    assert!(state.validate_invariants().is_ok());

    // Invalid: Idle with bead
    let state =
        AgentState::new(agent_id.clone(), Some(BeadId::new("b1")), None, AgentStatus::Idle, 0);
    assert!(state.validate_invariants().is_err());

    // Valid: Idle without bead
    let state = AgentState::new(agent_id.clone(), None, None, AgentStatus::Idle, 0);
    assert!(state.validate_invariants().is_ok());
}

#[test]
fn test_bdd_scenario_successful_tdd_cycle() {
    // Given: A Run is in Tdd15 stage
    let bead_id = BeadId::new("bdd-scenario-1");
    let mut run = Run::new(bead_id);
    run = run.start().expect("Start");

    // Manually set stage to Tdd15 to simulate "Given"
    if let RunState::Running { ref mut current_stage } = run.state {
        *current_stage = StageName::Tdd15;
    }

    // When: The stage completes with passed=true
    let result = StageResult {
        run_id: run.id.as_str().to_string(),
        stage: StageName::Tdd15,
        attempt: 1,
        passed: true,
        output: serde_json::json!({"test": "passed"}),
        failure_category: None,
        next_stage: Some(StageName::Qa),
    };

    run = run.complete_stage(StageName::Tdd15, result).expect("Complete");

    // Then: The Run status updates to Running (implied, still running)
    // Then: The current stage advances to Qa
    match run.state {
        RunState::Running { current_stage } => assert_eq!(current_stage, StageName::Qa),
        _ => panic!("Should be running in Qa"),
    }
}
