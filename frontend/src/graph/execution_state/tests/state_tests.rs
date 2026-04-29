use super::super::*;

// ===========================================================================
// ExecutionState Display Tests
// ===========================================================================

#[test]
fn execution_state_display_shows_idle_lowercase() {
    assert_eq!(format!("{}", ExecutionState::Idle), "idle");
}

#[test]
fn execution_state_display_shows_queued_lowercase() {
    assert_eq!(format!("{}", ExecutionState::Queued), "queued");
}

#[test]
fn execution_state_display_shows_running_lowercase() {
    assert_eq!(format!("{}", ExecutionState::Running), "running");
}

#[test]
fn execution_state_display_shows_completed_lowercase() {
    assert_eq!(format!("{}", ExecutionState::Completed), "completed");
}

#[test]
fn execution_state_display_shows_failed_lowercase() {
    assert_eq!(format!("{}", ExecutionState::Failed), "failed");
}

#[test]
fn execution_state_display_shows_skipped_lowercase() {
    assert_eq!(format!("{}", ExecutionState::Skipped), "skipped");
}

// ===========================================================================
// ExecutionState is_terminal Tests
// ===========================================================================

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

// ===========================================================================
// ExecutionState is_active Tests
// ===========================================================================

#[test]
fn is_active_returns_true_for_running() {
    assert!(ExecutionState::Running.is_active());
}

#[test]
fn is_active_returns_true_for_queued() {
    assert!(ExecutionState::Queued.is_active());
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
// ExecutionState is_idle Tests
// ===========================================================================

#[test]
fn is_idle_returns_true_for_idle() {
    assert!(ExecutionState::Idle.is_idle());
}

#[test]
fn is_idle_returns_false_for_queued() {
    assert!(!ExecutionState::Queued.is_idle());
}

#[test]
fn is_idle_returns_false_for_running() {
    assert!(!ExecutionState::Running.is_idle());
}

#[test]
fn is_idle_returns_false_for_completed() {
    assert!(!ExecutionState::Completed.is_idle());
}

#[test]
fn is_idle_returns_false_for_failed() {
    assert!(!ExecutionState::Failed.is_idle());
}

#[test]
fn is_idle_returns_false_for_skipped() {
    assert!(!ExecutionState::Skipped.is_idle());
}

// ===========================================================================
// ExecutionState Clone and Copy Tests
// ===========================================================================

#[test]
fn execution_state_is_copy() {
    let state = ExecutionState::Running;
    let state2 = state;
    let state3 = state;
    assert_eq!(state2, ExecutionState::Running);
    assert_eq!(state3, ExecutionState::Running);
}

#[test]
fn execution_state_is_clone() {
    let state = ExecutionState::Queued;
    let cloned = state;
    assert_eq!(cloned, ExecutionState::Queued);
}

// ===========================================================================
// ExecutionState Hash Tests
// ===========================================================================

#[test]
fn execution_state_hash_is_consistent() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher1 = DefaultHasher::new();
    ExecutionState::Running.hash(&mut hasher1);

    let mut hasher2 = DefaultHasher::new();
    ExecutionState::Running.hash(&mut hasher2);

    assert_eq!(hasher1.finish(), hasher2.finish());
}

#[test]
fn execution_state_hash_differs_for_different_states() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher1 = DefaultHasher::new();
    ExecutionState::Idle.hash(&mut hasher1);

    let mut hasher2 = DefaultHasher::new();
    ExecutionState::Queued.hash(&mut hasher2);

    assert_ne!(hasher1.finish(), hasher2.finish());
}

// ===========================================================================
// ExecutionState Default Tests
// ===========================================================================

#[test]
fn execution_state_default_is_idle() {
    let default_state = ExecutionState::default();
    assert_eq!(default_state, ExecutionState::Idle);
}

// ===========================================================================
// ExecutionState Serialize/Deserialize Tests
// ===========================================================================

#[test]
fn execution_state_serialize_idle() {
    let json = serde_json::to_string(&ExecutionState::Idle).unwrap();
    assert_eq!(json, "\"idle\"");
}

#[test]
fn execution_state_serialize_queued() {
    let json = serde_json::to_string(&ExecutionState::Queued).unwrap();
    assert_eq!(json, "\"queued\"");
}

#[test]
fn execution_state_serialize_running() {
    let json = serde_json::to_string(&ExecutionState::Running).unwrap();
    assert_eq!(json, "\"running\"");
}

#[test]
fn execution_state_serialize_completed() {
    let json = serde_json::to_string(&ExecutionState::Completed).unwrap();
    assert_eq!(json, "\"completed\"");
}

#[test]
fn execution_state_serialize_failed() {
    let json = serde_json::to_string(&ExecutionState::Failed).unwrap();
    assert_eq!(json, "\"failed\"");
}

#[test]
fn execution_state_serialize_skipped() {
    let json = serde_json::to_string(&ExecutionState::Skipped).unwrap();
    assert_eq!(json, "\"skipped\"");
}

#[test]
fn execution_state_deserialize_idle() {
    let state: ExecutionState = serde_json::from_str("\"idle\"").unwrap();
    assert_eq!(state, ExecutionState::Idle);
}

#[test]
fn execution_state_deserialize_queued() {
    let state: ExecutionState = serde_json::from_str("\"queued\"").unwrap();
    assert_eq!(state, ExecutionState::Queued);
}

#[test]
fn execution_state_deserialize_running() {
    let state: ExecutionState = serde_json::from_str("\"running\"").unwrap();
    assert_eq!(state, ExecutionState::Running);
}

#[test]
fn execution_state_deserialize_completed() {
    let state: ExecutionState = serde_json::from_str("\"completed\"").unwrap();
    assert_eq!(state, ExecutionState::Completed);
}

#[test]
fn execution_state_deserialize_failed() {
    let state: ExecutionState = serde_json::from_str("\"failed\"").unwrap();
    assert_eq!(state, ExecutionState::Failed);
}

#[test]
fn execution_state_deserialize_skipped() {
    let state: ExecutionState = serde_json::from_str("\"skipped\"").unwrap();
    assert_eq!(state, ExecutionState::Skipped);
}
