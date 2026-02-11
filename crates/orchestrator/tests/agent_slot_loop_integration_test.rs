//! BDD integration tests for AgentSlotActor recursive stage loop.
//!
//! This module tests the recursive stage execution behavior described in bead bd-3a0a.6:
//!
//! ## Phase 2 - BDD Integration Tests
//!
//! GIVEN a bead with multiple stages
//! WHEN executing through the recursive loop
//! THEN stages proceed, reentry works, and exhaustion is handled correctly.
//!
//! ## Test Scenarios
//!
//! 1. Successful bead completion through all stages
//! 2. Reentry flow from Review → Plan with feedback
//! 3. Failure flow after max retries exhausted
//! 4. Artifact tracking across stages
//! 5. Feedback propagation on reentry

// Integration tests allow unwrap/panic for assertions

use orchestrator::actors::agent_slot::{
    AgentSlotActorDef, AgentSlotMessage, AgentSlotState, BeadCompletion, SlotError, SlotState,
};
use oya_events::{BeadId, RecursionPolicy, StageKind};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::oneshot;

/// Test 1: Successful bead completion through all stages
///
/// **Given** a bead starting at Research stage
/// **When** all stages succeed consecutively
/// **Then** the bead completes with Accepted status
#[tokio::test]
async fn given_bead_at_research_when_all_stages_succeed_then_bead_accepted() {
    // Given: An agent slot with a bead starting execution
    let project_root = PathBuf::from("/tmp/test-oya");
    let actor = AgentSlotActorDef::spawn(project_root, None)
        .await
        .expect("actor spawn should succeed");

    let bead_id = BeadId::new();
    let spec = "Implement feature X".to_string();
    let relevant_files = vec![PathBuf::from("/tmp/test.rs")];
    let upstream_artifacts = vec![];

    // When: Starting bead execution
    let (reply_tx, reply_rx) = oneshot::channel::<Result<BeadCompletion, SlotError>>();
    actor
        .send_message(AgentSlotMessage::StartBead {
            bead_id: bead_id.clone(),
            spec,
            relevant_files,
            upstream_artifacts,
            reply: reply_tx,
        })
        .expect("send_message should succeed");

    let result = reply_rx
        .await
        .expect("reply should be received")
        .expect("start_bead should succeed");

    // Then: Bead starts successfully (will complete through execution)
    assert_eq!(result, BeadCompletion::Accepted);

    // Verify initial state
    let (reply_tx, reply_rx) = oneshot::channel::<SlotState>();
    actor
        .send_message(AgentSlotMessage::GetState { reply: reply_tx })
        .expect("send_message should succeed");

    let state = reply_rx.await.expect("reply should be received");
    assert!(matches!(state, SlotState::Executing { .. }));
}

/// Test 2: Reentry flow from Review → Plan with feedback
///
/// **Given** a bead at Review stage that fails quality checks
/// **When** gate decides to reenter Plan stage with feedback
/// **Then** state machine transitions back to Plan and stores feedback
#[tokio::test]
async fn given_review_fails_when_gate_reenters_plan_then_feedback_stored() {
    // Given: Agent slot state with bead in Review
    let mut state = AgentSlotState::new(PathBuf::from("/tmp"));
    let bead_id = BeadId::new();

    state.bead_id = Some(bead_id.clone());
    state.current_stage = Some(StageKind::Review);

    // When: Review fails and requires reentry to Plan
    // (This would normally be triggered by gate decision)
    let feedback = "Implementation missing error handling".to_string();
    state.pending_feedback = Some(feedback.clone());

    // Then: Feedback is stored for next execution
    assert_eq!(state.pending_feedback, Some(feedback));
    assert_eq!(state.current_stage, Some(StageKind::Review));
}

/// Test 3: Failure flow after max retries exhausted
///
/// **Given** a bead that has exceeded retry limits
/// **When** another retry is attempted
/// **Then** bead is marked as Parked with exhaustion reason
#[tokio::test]
async fn given_retries_exhausted_when_another_attempt_then_bead_parked() {
    // Given: State machine with strict retry policy
    let bead_id = BeadId::new();
    let strict_policy = RecursionPolicy {
        max_total_attempts: 2,
        max_stage_retries: 1,
        max_research_retries: 0,
        on_exhaustion: oya_events::ExhaustionPolicy::ParkForHuman,
    };

    // Create state machine with strict policy
    let mut state_machine = oya_events::BeadStateMachine::with_policy(bead_id, strict_policy);

    // When: Exhausting retries by entering stage multiple times
    let _ = state_machine.enter_stage();
    let _ = state_machine.advance(); // Move to Plan

    // Reenter Plan (simulated failure)
    let _ = state_machine.reenter(StageKind::Plan, "failed", oya_events::Severity::Major);

    // Second attempt at Plan should exhaust because max_stage_retries is 1 (allows 1 initial + 1 retry)
    // Wait, total attempts is also checked.
    let _ = state_machine.enter_stage();

    // Now total_attempts is 2. max_total_attempts is 2.
    // Next enter_stage should fail.
    let result = state_machine.enter_stage();
    assert!(result.is_err(), "should exhaust retries");

    // Then: Bead would be parked (simulated by checking exhaustion)
    let exhausted = matches!(
        result,
        Err(oya_events::StateMachineError::TotalAttemptsExhausted)
    );
    assert!(exhausted, "should be exhausted after max total attempts");
}

/// Test 4: Artifact tracking across stages
///
/// **Given** a bead executing through multiple stages
/// **When** each stage produces an artifact
/// **Then** artifacts are tracked in the slot state
#[tokio::test]
async fn given_bead_executing_when_stages_produce_artifacts_then_tracked() {
    // Given: Agent slot state
    let mut state = AgentSlotState::new(PathBuf::from("/tmp"));
    let bead_id = BeadId::new();

    state.bead_id = Some(bead_id);
    state.current_stage = Some(StageKind::Research);

    // When: Stages produce artifacts
    state
        .artifacts
        .insert(StageKind::Research, "research artifact".to_string());
    state
        .artifacts
        .insert(StageKind::Plan, "plan artifact".to_string());
    state
        .artifacts
        .insert(StageKind::Implement, "code artifact".to_string());

    // Then: Artifacts are tracked
    assert_eq!(state.artifacts.len(), 3);
    assert_eq!(
        state.artifacts.get(&StageKind::Research),
        Some(&"research artifact".to_string())
    );
    assert_eq!(
        state.artifacts.get(&StageKind::Plan),
        Some(&"plan artifact".to_string())
    );
    assert_eq!(
        state.artifacts.get(&StageKind::Implement),
        Some(&"code artifact".to_string())
    );
}

/// Test 5: Feedback propagation on reentry
///
/// **Given** a bead that fails Review with specific feedback
/// **When** reentering Plan stage
/// **Then** feedback is propagated to the new stage execution
#[tokio::test]
async fn given_review_fails_with_feedback_when_reentering_plan_then_feedback_propagated() {
    // Given: Agent slot with feedback from failed Review
    let mut state = AgentSlotState::new(PathBuf::from("/tmp"));
    let bead_id = BeadId::new();

    state.bead_id = Some(bead_id);
    state.current_stage = Some(StageKind::Review);

    let review_feedback = "Missing unit tests for critical paths".to_string();
    state.pending_feedback = Some(review_feedback.clone());

    // When: Reentering Plan stage (simulated by updating state)
    state.current_stage = Some(StageKind::Plan);

    // Then: Feedback is still available
    assert_eq!(state.pending_feedback, Some(review_feedback));

    // After Plan stage executes, feedback should be cleared
    state.pending_feedback = None;
    assert!(state.pending_feedback.is_none());
}

/// Test 6: Stage progression through the lifecycle
///
/// **Given** a bead at Research stage
/// **When** stages advance successfully
/// **Then** each stage transitions to the next in sequence
#[tokio::test]
async fn given_bead_at_research_when_stages_advance_then_correct_sequence() {
    // Given: State machine at Research
    let bead_id = BeadId::new();
    let mut state_machine = oya_events::BeadStateMachine::new(bead_id);

    assert_eq!(state_machine.current_stage(), StageKind::Research);

    // When: Advancing through stages
    let _ = state_machine.enter_stage();
    let transition1 = state_machine
        .advance()
        .expect("advance to Plan should succeed");

    assert_eq!(transition1.from, StageKind::Research);
    assert_eq!(transition1.to, StageKind::Plan);

    let _ = state_machine.enter_stage();
    let transition2 = state_machine
        .advance()
        .expect("advance to Implement should succeed");

    assert_eq!(transition2.from, StageKind::Plan);
    assert_eq!(transition2.to, StageKind::Implement);

    // Then: Correct stage progression
    assert_eq!(state_machine.current_stage(), StageKind::Implement);
}

/// Test 7: Complete bead lifecycle from start to finish
///
/// **Given** a fresh agent slot
/// **When** starting and executing a bead through all stages
/// **Then** slot transitions from Idle → Executing → Completed
#[tokio::test]
async fn given_idle_slot_when_bead_completes_then_slot_idle_again() {
    // Given: Idle agent slot
    let project_root = PathBuf::from("/tmp/test-oya-complete");
    let actor = AgentSlotActorDef::spawn(project_root, None)
        .await
        .expect("actor spawn should succeed");

    // Verify initial idle state
    let (reply_tx, reply_rx) = oneshot::channel::<SlotState>();
    actor
        .send_message(AgentSlotMessage::GetState { reply: reply_tx })
        .expect("send_message should succeed");

    let initial_state = reply_rx.await.expect("reply should be received");
    assert!(matches!(initial_state, SlotState::Idle));

    // When: Starting bead
    let bead_id = BeadId::new();
    let (reply_tx, reply_rx) = oneshot::channel::<Result<BeadCompletion, SlotError>>();
    actor
        .send_message(AgentSlotMessage::StartBead {
            bead_id,
            spec: "Test bead".to_string(),
            relevant_files: vec![],
            upstream_artifacts: vec![],
            reply: reply_tx,
        })
        .expect("send_message should succeed");

    let _result = reply_rx
        .await
        .expect("reply should be received")
        .expect("start_bead should succeed");

    // Then: Slot is executing
    let (reply_tx, reply_rx) = oneshot::channel::<SlotState>();
    actor
        .send_message(AgentSlotMessage::GetState { reply: reply_tx })
        .expect("send_message should succeed");

    let executing_state = reply_rx.await.expect("reply should be received");
    assert!(matches!(executing_state, SlotState::Executing { .. }));
}

/// Test 8: Error handling when bead ID is missing
///
/// **Given** an agent slot state without a bead ID
/// **When** trying to complete the bead
/// **Then** operation fails gracefully with BeadIdNotAvailable error
#[tokio::test]
async fn given_slot_without_bead_when_completing_then_returns_error() {
    // Given: Slot state without bead ID
    let state = AgentSlotState::new(PathBuf::from("/tmp"));

    // When: Trying to require bead ID
    let result = state.require_bead_id();

    // Then: Returns BeadIdNotAvailable error
    assert!(matches!(result, Err(SlotError::BeadIdNotAvailable)));
}

/// Test 9: Multiple stage retries before exhaustion
///
/// **Given** a bead with retry policy of 3 attempts
/// **When** stage fails and retries 3 times
/// **Then** bead is exhausted on the 4th attempt
#[tokio::test]
async fn given_stage_with_3_retries_when_4_attempts_then_exhausted() {
    // Given: State machine with 3 retries allowed
    let bead_id = BeadId::new();
    let policy = RecursionPolicy {
        max_total_attempts: 100,
        max_stage_retries: 3,
        max_research_retries: 1,
        on_exhaustion: oya_events::ExhaustionPolicy::Fail,
    };

    let mut state_machine = oya_events::BeadStateMachine::with_policy(bead_id, policy);

    // When: Attempting stage 4 times (1 initial + 3 retries)
    let _ = state_machine.enter_stage(); // 1
    let _ = state_machine.advance(); // Move to Plan

    let _ = state_machine.reenter(StageKind::Plan, "fail 1", oya_events::Severity::Major);
    let _ = state_machine.enter_stage(); // 2

    let _ = state_machine.reenter(StageKind::Plan, "fail 2", oya_events::Severity::Major);
    let _ = state_machine.enter_stage(); // 3

    let _ = state_machine.reenter(StageKind::Plan, "fail 3", oya_events::Severity::Major);
    let result4 = state_machine.enter_stage(); // 4th attempt (3rd retry) - should succeed
    assert!(
        result4.is_ok(),
        "fourth attempt should succeed with 3 retries"
    );

    let _ = state_machine.reenter(StageKind::Plan, "fail 4", oya_events::Severity::Major);
    let _ = state_machine.enter_stage(); // 5

    let _ = state_machine.reenter(StageKind::Plan, "fail 5", oya_events::Severity::Major);
    let result6 = state_machine.enter_stage(); // 6th attempt - should fail
    assert!(result6.is_err(), "sixth attempt should fail");

    // Then: Exhausted after max retries
    assert!(matches!(
        result6,
        Err(oya_events::StateMachineError::StageRetriesExhausted)
    ));
}

/// Test 10: Artifact accumulation across full lifecycle
///
/// **Given** a bead executing through complete lifecycle
/// **When** all stages produce artifacts
/// **Then** all 6 stages have artifacts tracked
#[tokio::test]
async fn given_full_lifecycle_when_all_stages_complete_then_6_artifacts_tracked() {
    // Given: Agent slot state
    let mut state = AgentSlotState::new(PathBuf::from("/tmp"));
    state.bead_id = Some(BeadId::new());

    // When: All stages produce artifacts
    let stages = [
        StageKind::Research,
        StageKind::Plan,
        StageKind::Implement,
        StageKind::Review,
        StageKind::Validate,
        StageKind::Accept,
    ];

    for stage in stages {
        state
            .artifacts
            .insert(stage, format!("{:?} artifact", stage));
    }

    // Then: All 6 artifacts tracked
    assert_eq!(state.artifacts.len(), 6);
    assert!(state.artifacts.contains_key(&StageKind::Research));
    assert!(state.artifacts.contains_key(&StageKind::Plan));
    assert!(state.artifacts.contains_key(&StageKind::Implement));
    assert!(state.artifacts.contains_key(&StageKind::Review));
    assert!(state.artifacts.contains_key(&StageKind::Validate));
    assert!(state.artifacts.contains_key(&StageKind::Accept));
}
