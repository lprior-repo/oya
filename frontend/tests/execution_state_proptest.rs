//! Proptest scenarios for ExecutionState machine
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{can_transition, try_transition, ExecutionState};
use proptest::prelude::*;

// Strategy to generate all possible ExecutionState values
fn execution_state_strategy() -> impl Strategy<Value = ExecutionState> {
    prop_oneof![
        Just(ExecutionState::Idle),
        Just(ExecutionState::Queued),
        Just(ExecutionState::Running),
        Just(ExecutionState::Completed),
        Just(ExecutionState::Failed),
        Just(ExecutionState::Skipped),
    ]
}

// Strategy to generate all possible state pairs
fn state_pair_strategy() -> impl Strategy<Value = (ExecutionState, ExecutionState)> {
    execution_state_strategy()
        .prop_flat_map(|from| execution_state_strategy().prop_map(move |to| (from, to)))
}

// ===========================================================================
// Proptest: All 36 state pairs tested
// ===========================================================================

proptest! {
    #[test]
    fn prop_try_transition_matrix_exhaustiveness(
        (from, to) in state_pair_strategy()
    ) {
        let result = try_transition(from, to);

        // All 36 state pairs must be tested
        // Valid transitions should return Some, invalid should return None

        let is_valid = matches!(
            (from, to),
            (ExecutionState::Idle, ExecutionState::Queued) |
            (ExecutionState::Idle, ExecutionState::Skipped) |
            (ExecutionState::Queued, ExecutionState::Running) |
            (ExecutionState::Queued, ExecutionState::Skipped) |
            (ExecutionState::Running, ExecutionState::Completed) |
            (ExecutionState::Running, ExecutionState::Failed)
        );

        if is_valid {
            assert!(result.is_some(),
                "try_transition({:?}, {:?}) should return Some", from, to);
        } else {
            assert!(result.is_none(),
                "try_transition({:?}, {:?}) should return None", from, to);
        }
    }
}

// ===========================================================================
// Proptest: State machine compliance
// ===========================================================================

proptest! {
    #[test]
    fn prop_state_machine_compliance(
        from in execution_state_strategy(),
        to in execution_state_strategy()
    ) {
        // No state can be reached without valid transitions between each pair

        let result = try_transition(from, to);

        // The result must be consistent
        if result.is_some() {
            let transition = result.expect("is_some was checked above");
            let (expected_from, expected_to) = transition.from_states();
            assert_eq!(from, expected_from);
            assert_eq!(to, expected_to);
        }
    }
}

// ===========================================================================
// Proptest: Serialization round-trip
// ===========================================================================

use serde_json::{from_str, to_string};

proptest! {
    #[test]
    fn prop_serialization_roundtrip(
        state in execution_state_strategy()
    ) {
        let serialized = to_string(&state).expect("all ExecutionState variants serialize successfully");
        let deserialized: Result<ExecutionState, _> = from_str(&serialized);

        assert!(deserialized.is_ok(),
            "Deserialization of {:?} should succeed", state);

        let deserialized = deserialized.expect("deserialization of valid JSON must succeed");
        assert_eq!(state, deserialized,
            "Round-trip should preserve value for {:?}", state);
    }
}

// ===========================================================================
// Proptest: Config sync consistency
// ===========================================================================

proptest! {
    #[test]
    fn prop_can_transition_symmetry(
        (from, to) in state_pair_strategy()
    ) {
        // can_transition always mirrors try_transition logic

        let try_result = try_transition(from, to);
        let can_result = can_transition(from, to);

        let try_is_valid = try_result.is_some();

        assert_eq!(can_result, try_is_valid,
            "can_transition({:?}, {:?}) = {} should match try_transition result",
            from, to, can_result);
    }
}

// ===========================================================================
// Proptest: Terminal state immutability
// ===========================================================================

proptest! {
    #[test]
    fn prop_terminal_state_immutable(
        terminal_state in prop_oneof![
            Just(ExecutionState::Completed),
            Just(ExecutionState::Failed),
            Just(ExecutionState::Skipped),
        ],
        any_state in execution_state_strategy()
    ) {
        // All transitions from terminal states should be invalid

        let result = try_transition(terminal_state, any_state);

        // Any transition from a terminal state (including to itself) should fail
        assert!(result.is_none(),
            "Transition from terminal state {:?} to {:?} should be invalid",
            terminal_state, any_state);
    }
}

// ===========================================================================
// Proptest: Reflexive transition rejection
// ===========================================================================

proptest! {
    #[test]
    fn prop_reflexive_transitions_rejected(
        state in execution_state_strategy()
    ) {
        // try_transition(same_state) always returns None

        let result = try_transition(state, state);

        assert!(result.is_none(),
            "Self-transition from {:?} to {:?} should be rejected",
            state, state);
    }
}

// ===========================================================================
// Proptest: can_transition symmetry
// ===========================================================================

proptest! {
    #[test]
    fn prop_can_transition_reflexivity(
        state in execution_state_strategy()
    ) {
        // can_transition(state, state) should be false for all states

        let result = can_transition(state, state);

        assert!(!result,
            "can_transition({:?}, {:?}) should be false", state, state);
    }
}

// ===========================================================================
// Proptest: Valid transition count
// ===========================================================================

proptest! {
    #[test]
    fn prop_valid_transition_count(
        (from, to) in state_pair_strategy()
    ) {
        // Count should match known valid transitions

        let result = try_transition(from, to);

        let valid_count = matches!(
            (from, to),
            (ExecutionState::Idle, ExecutionState::Queued) |
            (ExecutionState::Idle, ExecutionState::Skipped) |
            (ExecutionState::Queued, ExecutionState::Running) |
            (ExecutionState::Queued, ExecutionState::Skipped) |
            (ExecutionState::Running, ExecutionState::Completed) |
            (ExecutionState::Running, ExecutionState::Failed)
        ) as u32;

        assert_eq!(result.is_some(), valid_count == 1,
            "Valid transition count should be 1 for valid pairs, 0 for invalid");
    }
}

// ===========================================================================
// Proptest: State completeness
// ===========================================================================

proptest! {
    #[test]
    fn prop_execution_state_completeness(
        state in execution_state_strategy()
    ) {
        // Every state must have either is_terminal or is_active true or both false

        let is_term = state.is_terminal();
        let is_act = state.is_active();

        // A state can be terminal, active, or neither (Idle is neither)
        // But never both terminal and active
        assert!(!(is_term && is_act),
            "State {:?} cannot be both terminal and active", state);
    }
}
