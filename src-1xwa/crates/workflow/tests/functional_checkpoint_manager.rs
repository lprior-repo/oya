//! Functional checkpoint manager state machine tests.

use oya_workflow::checkpoint::{CheckpointManager, CheckpointStrategy};
use oya_workflow::PhaseOutput;

/// Helper to create a successful phase output.
fn success_output() -> PhaseOutput {
    PhaseOutput::success(vec![1, 2, 3])
}

/// Helper to create a failed phase output.
fn failure_output() -> PhaseOutput {
    PhaseOutput {
        success: false,
        data: std::sync::Arc::new(vec![]),
        message: Some("Failed".to_string()),
        artifacts: vec![],
        duration_ms: 100,
    }
}

#[test]
fn test_checkpoint_strategy_always() {
    let mut manager = CheckpointManager::new(CheckpointStrategy::Always);

    // Should always checkpoint
    let decision1 = manager.update(&success_output());
    assert!(decision1.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 0);

    let decision2 = manager.update(&failure_output());
    assert!(decision2.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 0);
}

#[test]
fn test_checkpoint_strategy_on_success() {
    let mut manager = CheckpointManager::new(CheckpointStrategy::OnSuccess);

    // Successful phase -> checkpoint
    let decision1 = manager.update(&success_output());
    assert!(decision1.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 0);

    // Failed phase -> skip
    let decision2 = manager.update(&failure_output());
    assert!(!decision2.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 1);

    // Another failed phase -> skip
    let decision3 = manager.update(&failure_output());
    assert!(!decision3.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 2);

    // Successful phase -> checkpoint
    let decision4 = manager.update(&success_output());
    assert!(decision4.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 0);
}

#[test]
fn test_checkpoint_strategy_interval() {
    let mut manager = CheckpointManager::new(CheckpointStrategy::Interval(3));

    // Phase 1: phases_since_last=0, 0 >= 3 is false, skip
    let decision1 = manager.update(&success_output());
    assert!(!decision1.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 1);

    // Phase 2: phases_since_last=1, 1 >= 3 is false, skip
    let decision2 = manager.update(&success_output());
    assert!(!decision2.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 2);

    // Phase 3: phases_since_last=2, 2 >= 3 is false, skip
    let decision3 = manager.update(&success_output());
    assert!(!decision3.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 3);

    // Phase 4: phases_since_last=3, 3 >= 3 is true, checkpoint!
    let decision4 = manager.update(&success_output());
    assert!(decision4.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 0);

    // Phase 5: counter reset, skip again
    let decision5 = manager.update(&success_output());
    assert!(!decision5.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 1);
}

#[test]
fn test_checkpoint_state_transitions() {
    // Test state transitions through the public update API
    let mut manager = CheckpointManager::new(CheckpointStrategy::Interval(2));

    // Phase 1: phases_since_last=0, 0 >= 2 is false, skip
    let decision1 = manager.update(&success_output());
    assert!(!decision1.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 1);

    // Phase 2: phases_since_last=1, 1 >= 2 is false, skip
    let decision2 = manager.update(&success_output());
    assert!(!decision2.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 2);

    // Phase 3: phases_since_last=2, 2 >= 2 is true, checkpoint!
    let decision3 = manager.update(&success_output());
    assert!(decision3.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 0);
    assert!(manager.last_checkpoint().is_some());
}

#[test]
fn test_checkpoint_manager_accessors() {
    let manager = CheckpointManager::new(CheckpointStrategy::OnSuccess);
    assert_eq!(manager.strategy(), CheckpointStrategy::OnSuccess);
    assert_eq!(manager.phases_since_last(), 0);
    assert!(manager.last_checkpoint().is_none());

    // After update, last_checkpoint should still be None (no checkpoint yet)
    let mut manager = manager;
    manager.update(&failure_output());
    assert!(manager.last_checkpoint().is_none());

    // After successful phase, should have checkpoint time
    manager.update(&success_output());
    assert!(manager.last_checkpoint().is_some());
}

#[test]
fn test_checkpoint_with_zero_interval() {
    let mut manager = CheckpointManager::new(CheckpointStrategy::Interval(0));

    // Zero interval means checkpoint every phase (like Always)
    let decision1 = manager.update(&success_output());
    assert!(decision1.should_checkpoint());

    let decision2 = manager.update(&failure_output());
    assert!(decision2.should_checkpoint());
}

#[test]
fn test_checkpoint_pure_function_pattern() {
    // Verify that the manager maintains state correctly through updates
    let mut manager = CheckpointManager::new(CheckpointStrategy::Always);

    // Initial state
    assert_eq!(manager.phases_since_last(), 0);
    assert!(manager.last_checkpoint().is_none());

    // After first update
    let decision1 = manager.update(&success_output());
    assert!(decision1.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 0);
    assert!(manager.last_checkpoint().is_some());

    // After second update
    let decision2 = manager.update(&failure_output());
    assert!(decision2.should_checkpoint());
    assert_eq!(manager.phases_since_last(), 0);
}
