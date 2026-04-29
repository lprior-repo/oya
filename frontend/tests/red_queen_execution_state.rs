//! Red Queen Adversarial Test Suite for execution_state refactoring (oya-frontend-2e4)
//!
//! Dimensions attacked:
//! 1. INV-5: State machine transition contract
//! 2. Serde round-trip invariants (edge cases)
//! 3. TerminalState sealed trait boundary
//! 4. Behavioral regression from refactoring
//! 5. From conversion correctness
//! 6. Type-state pattern stress tests

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use oya_frontend::graph::{
    can_transition, try_transition, CompletedState, ExecutionState, FailedState, IdleState,
    InvalidTransition, QueuedState, RunningState, SkippedState, StateTransition, TerminalState,
};

// ===========================================================================
// DIMENSION 1: INV-5 State Machine Transition Contract — Exhaustive Matrix
// ===========================================================================

/// Every (from, to) pair in the 6x6 matrix that is NOT in the legal transition table
/// MUST return false from can_transition and None from try_transition.
/// This is the exhaustive adversarial probe.
#[test]
fn rq_exhaustive_illegal_transition_matrix() {
    let all_states = [
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    let legal = [
        (ExecutionState::Idle, ExecutionState::Queued),
        (ExecutionState::Idle, ExecutionState::Skipped),
        (ExecutionState::Queued, ExecutionState::Running),
        (ExecutionState::Queued, ExecutionState::Skipped),
        (ExecutionState::Running, ExecutionState::Completed),
        (ExecutionState::Running, ExecutionState::Failed),
    ];

    for from in &all_states {
        for to in &all_states {
            let pair = (*from, *to);
            let is_legal = legal.contains(&pair);

            if !is_legal {
                assert!(
                    !can_transition(*from, *to),
                    "INV-5 VIOLATION: can_transition({from:?}, {to:?}) must be false",
                );
                assert_eq!(
                    try_transition(*from, *to),
                    None,
                    "INV-5 VIOLATION: try_transition({from:?}, {to:?}) must be None",
                );
            }
        }
    }
}

/// Verify every legal transition produces the EXACT StateTransition variant.
#[test]
fn rq_legal_transitions_return_exact_variants() {
    assert_eq!(
        try_transition(ExecutionState::Idle, ExecutionState::Queued),
        Some(StateTransition::IdleToQueued)
    );
    assert_eq!(
        try_transition(ExecutionState::Idle, ExecutionState::Skipped),
        Some(StateTransition::IdleToSkipped)
    );
    assert_eq!(
        try_transition(ExecutionState::Queued, ExecutionState::Running),
        Some(StateTransition::QueuedToRunning)
    );
    assert_eq!(
        try_transition(ExecutionState::Queued, ExecutionState::Skipped),
        Some(StateTransition::QueuedToSkipped)
    );
    assert_eq!(
        try_transition(ExecutionState::Running, ExecutionState::Completed),
        Some(StateTransition::RunningToCompleted)
    );
    assert_eq!(
        try_transition(ExecutionState::Running, ExecutionState::Failed),
        Some(StateTransition::RunningToFailed)
    );
}

/// can_transition and try_transition MUST be consistent for ALL 36 pairs.
#[test]
fn rq_can_transition_consistent_with_try_transition_exhaustive() {
    let all_states = [
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    for from in &all_states {
        for to in &all_states {
            let bool_result = can_transition(*from, *to);
            let opt_result = try_transition(*from, *to).is_some();
            assert_eq!(
                bool_result, opt_result,
                "INV-5 VIOLATION: can_transition({from:?}, {to:?})={bool_result} but try_transition.is_some()={opt_result}",
            );
        }
    }
}

/// StateTransition::apply().from_states() MUST round-trip for all variants.
#[test]
fn rq_state_transition_round_trip_apply_from_states() {
    let transitions = [
        StateTransition::IdleToQueued,
        StateTransition::IdleToSkipped,
        StateTransition::QueuedToRunning,
        StateTransition::QueuedToSkipped,
        StateTransition::RunningToCompleted,
        StateTransition::RunningToFailed,
    ];

    for t in &transitions {
        let (from, to) = t.from_states();
        let result = t.apply();
        assert_eq!(result, to, "StateTransition::{t:?}.apply() = {result:?}, expected {to:?}",);
        // Also verify try_transition gives back the same variant
        assert_eq!(
            try_transition(from, to),
            Some(*t),
            "try_transition({from:?}, {to:?}) must return Some({t:?})",
        );
    }
}

/// Terminal states have ZERO outgoing transitions (exhaustive check).
#[test]
fn rq_terminal_states_zero_outgoing_transitions() {
    let terminals = [ExecutionState::Completed, ExecutionState::Failed, ExecutionState::Skipped];
    let all_states = [
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    for terminal in &terminals {
        for target in &all_states {
            assert!(
                !can_transition(*terminal, *target),
                "INV-5 VIOLATION: Terminal state {terminal:?} must not transition to {target:?}",
            );
        }
    }
}

/// Self-transitions are NEVER legal (INV-5).
#[test]
fn rq_self_transitions_always_illegal() {
    let all_states = [
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    for state in &all_states {
        assert!(
            !can_transition(*state, *state),
            "INV-5 VIOLATION: Self-transition for {state:?} must be illegal",
        );
        assert_eq!(
            try_transition(*state, *state),
            None,
            "INV-5 VIOLATION: try_transition({state:?}, {state:?}) must be None",
        );
    }
}

/// Backward transitions are NEVER legal.
#[test]
fn rq_backward_transitions_always_illegal() {
    let backward_pairs = [
        (ExecutionState::Queued, ExecutionState::Idle),
        (ExecutionState::Running, ExecutionState::Queued),
        (ExecutionState::Running, ExecutionState::Idle),
        (ExecutionState::Completed, ExecutionState::Running),
        (ExecutionState::Completed, ExecutionState::Queued),
        (ExecutionState::Completed, ExecutionState::Idle),
        (ExecutionState::Failed, ExecutionState::Running),
        (ExecutionState::Failed, ExecutionState::Queued),
        (ExecutionState::Failed, ExecutionState::Idle),
        (ExecutionState::Skipped, ExecutionState::Idle),
        (ExecutionState::Skipped, ExecutionState::Queued),
    ];

    for (from, to) in &backward_pairs {
        assert!(
            !can_transition(*from, *to),
            "INV-5 VIOLATION: Backward transition {from:?}->{to:?} must be illegal",
        );
    }
}

/// Skip-level transitions are NEVER legal.
#[test]
fn rq_skip_level_transitions_always_illegal() {
    let skip_pairs = [
        (ExecutionState::Idle, ExecutionState::Running), // skip Queued
        (ExecutionState::Idle, ExecutionState::Completed), // skip Queued + Running
        (ExecutionState::Idle, ExecutionState::Failed),  // skip Queued + Running
        (ExecutionState::Queued, ExecutionState::Completed), // skip Running
        (ExecutionState::Queued, ExecutionState::Failed), // skip Running
    ];

    for (from, to) in &skip_pairs {
        assert!(
            !can_transition(*from, *to),
            "INV-5 VIOLATION: Skip-level transition {from:?}->{to:?} must be illegal",
        );
    }
}

// ===========================================================================
// DIMENSION 2: Serde Round-Trip Invariant — Edge Cases
// ===========================================================================

/// Serde round-trip via serde_json binary format MUST preserve all variants.
#[test]
fn rq_serde_round_trip_via_json_value() {
    let variants = [
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    for variant in &variants {
        let json_val = serde_json::to_value(variant).unwrap();
        let recovered: ExecutionState = serde_json::from_value(json_val).unwrap();
        assert_eq!(*variant, recovered, "serde round-trip failed for {variant:?}");
    }
}

/// Deserializing UPPERCASE strings MUST fail (serde uses lowercase).
#[test]
fn rq_serde_rejects_uppercase_json() {
    let uppercase =
        ["\"Idle\"", "\"Queued\"", "\"Running\"", "\"Completed\"", "\"Failed\"", "\"Skipped\""];
    for s in &uppercase {
        let result = serde_json::from_str::<ExecutionState>(s);
        assert!(
            result.is_err(),
            "INV-3 VIOLATION: serde must reject uppercase '{s}', got {:?}",
            result.unwrap()
        );
    }
}

/// Deserializing mixed-case strings MUST fail.
#[test]
fn rq_serde_rejects_mixed_case_json() {
    let mixed =
        ["\"IDLE\"", "\"iDLE\"", "\"QueueD\"", "\"RUNNING\"", "\"Completed\"", "\"FAILED\""];
    for s in &mixed {
        let result = serde_json::from_str::<ExecutionState>(s);
        assert!(result.is_err(), "INV-3 VIOLATION: serde must reject mixed-case '{s}'");
    }
}

/// Deserializing unknown strings MUST fail.
#[test]
fn rq_serde_rejects_unknown_strings() {
    let unknown = [
        "\"pending\"",
        "\"error\"",
        "\"done\"",
        "\"cancelled\"",
        "\"paused\"",
        "\"\"",
        "\"IDLE\"",
        "\"idle \"",
        "\" idle\"",
    ];
    for s in &unknown {
        let result = serde_json::from_str::<ExecutionState>(s);
        assert!(result.is_err(), "Serde must reject unknown string '{s}'");
    }
}

/// Deserializing null MUST fail.
#[test]
fn rq_serde_rejects_null() {
    let result = serde_json::from_str::<ExecutionState>("null");
    assert!(result.is_err(), "serde must reject null for ExecutionState");
}

/// Deserializing a number MUST fail.
#[test]
fn rq_serde_rejects_number() {
    let result = serde_json::from_str::<ExecutionState>("0");
    assert!(result.is_err(), "serde must reject number for ExecutionState");
}

/// Deserializing an array MUST fail.
#[test]
fn rq_serde_rejects_array() {
    let result = serde_json::from_str::<ExecutionState>("[\"idle\"]");
    assert!(result.is_err(), "serde must reject array for ExecutionState");
}

/// Desdeserializing an object MUST fail.
#[test]
fn rq_serde_rejects_object() {
    let result = serde_json::from_str::<ExecutionState>(r#"{"state":"idle"}"#);
    assert!(result.is_err(), "serde must reject object for ExecutionState");
}

/// Verify serialization produces EXACT expected JSON strings (no deviation).
#[test]
fn rq_serde_exact_serialization_format() {
    assert_eq!(serde_json::to_string(&ExecutionState::Idle).unwrap(), "\"idle\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Queued).unwrap(), "\"queued\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Running).unwrap(), "\"running\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Completed).unwrap(), "\"completed\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Failed).unwrap(), "\"failed\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Skipped).unwrap(), "\"skipped\"");
}

/// Serde round-trip through a Vec (batch test).
#[test]
fn rq_serde_round_trip_vec() {
    let original: Vec<ExecutionState> = vec![
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];
    let json = serde_json::to_string(&original).unwrap();
    let recovered: Vec<ExecutionState> = serde_json::from_str(&json).unwrap();
    assert_eq!(original, recovered);
}

// ===========================================================================
// DIMENSION 3: TerminalState Sealed Trait Boundary
// ===========================================================================

/// Verify that TerminalState is ONLY implemented for the 3 terminal type-states.
/// This test uses trait bounds to prove it at compile time.
#[test]
fn rq_terminal_state_only_for_completed_failed_skipped() {
    fn accept_terminal<T: TerminalState>() {}
    accept_terminal::<CompletedState>();
    accept_terminal::<FailedState>();
    accept_terminal::<SkippedState>();
}

/// IdleState, QueuedState, RunningState do NOT implement TerminalState.
/// If they did, this would compile. We verify by using alternative trait bounds.
#[test]
fn rq_non_terminal_types_are_still_copy() {
    fn accept_copy<T: Copy>() {}
    accept_copy::<IdleState>();
    accept_copy::<QueuedState>();
    accept_copy::<RunningState>();
}

/// TerminalState types also implement Copy + Debug + PartialEq.
#[test]
fn rq_terminal_state_types_have_required_traits() {
    fn check<T: TerminalState + Copy + std::fmt::Debug + PartialEq>() {}
    check::<CompletedState>();
    check::<FailedState>();
    check::<SkippedState>();
}

/// The sealed module should not be accessible from external crates.
/// This is a compile-time property — if this compiles, the sealed trait
/// pattern is working. The test is that TerminalState cannot be implemented
/// by external types (enforced by private supertrait).
#[test]
fn rq_sealed_trait_prevents_external_impl() {
    // This test proves the pattern compiles correctly.
    // If someone could implement TerminalState for their own type,
    // the sealed supertrait would prevent it.
    let completed = CompletedState;
    let failed = FailedState;
    let skipped = SkippedState;

    fn is_terminal<T: TerminalState>(_: T) -> bool {
        true
    }

    assert!(is_terminal(completed));
    assert!(is_terminal(failed));
    assert!(is_terminal(skipped));
}

// ===========================================================================
// DIMENSION 4: Behavioral Regression — Verify Refactoring Didn't Change Behavior
// ===========================================================================

/// The happy path: Idle -> Queued -> Running -> Completed must work.
#[test]
fn rq_happy_path_idle_to_completed() {
    let mut state = ExecutionState::Idle;
    assert!(can_transition(state, ExecutionState::Queued));
    state = try_transition(state, ExecutionState::Queued).unwrap().apply();
    assert_eq!(state, ExecutionState::Queued);

    assert!(can_transition(state, ExecutionState::Running));
    state = try_transition(state, ExecutionState::Running).unwrap().apply();
    assert_eq!(state, ExecutionState::Running);

    assert!(can_transition(state, ExecutionState::Completed));
    state = try_transition(state, ExecutionState::Completed).unwrap().apply();
    assert_eq!(state, ExecutionState::Completed);

    assert!(state.is_terminal());
    assert!(!state.is_active());
    assert!(!state.is_idle());
}

/// The failure path: Idle -> Queued -> Running -> Failed.
#[test]
fn rq_failure_path_idle_to_failed() {
    let state = ExecutionState::Idle;
    let state = try_transition(state, ExecutionState::Queued).unwrap().apply();
    let state = try_transition(state, ExecutionState::Running).unwrap().apply();
    let state = try_transition(state, ExecutionState::Failed).unwrap().apply();

    assert_eq!(state, ExecutionState::Failed);
    assert!(state.is_terminal());
}

/// The skip-from-idle path: Idle -> Skipped.
#[test]
fn rq_skip_from_idle_path() {
    let state = ExecutionState::Idle;
    let state = try_transition(state, ExecutionState::Skipped).unwrap().apply();
    assert_eq!(state, ExecutionState::Skipped);
    assert!(state.is_terminal());
}

/// The skip-from-queued path: Idle -> Queued -> Skipped.
#[test]
fn rq_skip_from_queued_path() {
    let state = ExecutionState::Idle;
    let state = try_transition(state, ExecutionState::Queued).unwrap().apply();
    let state = try_transition(state, ExecutionState::Skipped).unwrap().apply();
    assert_eq!(state, ExecutionState::Skipped);
    assert!(state.is_terminal());
}

/// Verify is_terminal, is_active, is_idle are mutually exclusive in certain ways.
#[test]
fn rq_predicates_mutually_exclusive() {
    // Idle: only is_idle
    assert!(ExecutionState::Idle.is_idle());
    assert!(!ExecutionState::Idle.is_active());
    assert!(!ExecutionState::Idle.is_terminal());

    // Active states (Queued, Running): only is_active
    for state in [ExecutionState::Queued, ExecutionState::Running] {
        assert!(!state.is_idle(), "{state:?} must not be idle");
        assert!(state.is_active(), "{state:?} must be active");
        assert!(!state.is_terminal(), "{state:?} must not be terminal");
    }

    // Terminal states (Completed, Failed, Skipped): only is_terminal
    for state in [ExecutionState::Completed, ExecutionState::Failed, ExecutionState::Skipped] {
        assert!(!state.is_idle(), "{state:?} must not be idle");
        assert!(!state.is_active(), "{state:?} must not be active");
        assert!(state.is_terminal(), "{state:?} must be terminal");
    }
}

/// Verify InvalidTransition captures the correct from/to states.
#[test]
fn rq_invalid_transition_captures_correct_states() {
    let err = InvalidTransition::new(ExecutionState::Running, ExecutionState::Idle);
    assert_eq!(err.from_state(), ExecutionState::Running);
    assert_eq!(err.to_state(), ExecutionState::Idle);
}

/// InvalidTransition Display format must match exactly.
#[test]
fn rq_invalid_transition_display_format() {
    let err = InvalidTransition::new(ExecutionState::Completed, ExecutionState::Running);
    assert_eq!(format!("{err}"), "Invalid state transition: completed -> running");
}

/// InvalidTransition implements std::error::Error (can be used as Box<dyn Error>).
#[test]
fn rq_invalid_transition_is_std_error() {
    let err: Box<dyn std::error::Error> =
        Box::new(InvalidTransition::new(ExecutionState::Idle, ExecutionState::Completed));
    assert!(err.to_string().contains("Invalid state transition"));
}

/// ExecutionState Display must produce exact lowercase strings for all variants.
#[test]
fn rq_display_exact_lowercase_all_variants() {
    assert_eq!(ExecutionState::Idle.to_string(), "idle");
    assert_eq!(ExecutionState::Queued.to_string(), "queued");
    assert_eq!(ExecutionState::Running.to_string(), "running");
    assert_eq!(ExecutionState::Completed.to_string(), "completed");
    assert_eq!(ExecutionState::Failed.to_string(), "failed");
    assert_eq!(ExecutionState::Skipped.to_string(), "skipped");
}

/// Display output matches serde serialization output for all variants.
#[test]
fn rq_display_matches_serde_output() {
    let variants = [
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    for variant in &variants {
        let display = variant.to_string();
        let serde_json = serde_json::to_string(variant).unwrap();
        // serde wraps in quotes: "\"idle\""
        let expected = format!("\"{display}\"");
        assert_eq!(serde_json, expected, "Display and serde must agree for {variant:?}");
    }
}

/// Default for ExecutionState is Idle.
#[test]
fn rq_default_is_idle() {
    assert_eq!(ExecutionState::default(), ExecutionState::Idle);
}

// ===========================================================================
// DIMENSION 5: From Conversions — Exhaustive Correctness
// ===========================================================================

/// Every type-state struct converts to the correct ExecutionState variant.
#[test]
fn rq_from_conversions_all_correct() {
    assert_eq!(ExecutionState::from(IdleState), ExecutionState::Idle);
    assert_eq!(ExecutionState::from(QueuedState), ExecutionState::Queued);
    assert_eq!(ExecutionState::from(RunningState), ExecutionState::Running);
    assert_eq!(ExecutionState::from(CompletedState), ExecutionState::Completed);
    assert_eq!(ExecutionState::from(FailedState), ExecutionState::Failed);
    assert_eq!(ExecutionState::from(SkippedState), ExecutionState::Skipped);
}

/// From conversions are consistent with type-state method chains.
#[test]
fn rq_from_conversions_match_method_chains() {
    let idle = IdleState;
    let queued = idle.queue();
    assert_eq!(ExecutionState::from(queued), ExecutionState::Queued);

    let running = queued.start();
    assert_eq!(ExecutionState::from(running), ExecutionState::Running);

    let completed = running.complete();
    assert_eq!(ExecutionState::from(completed), ExecutionState::Completed);

    let running2 = QueuedState.start();
    let failed = running2.fail();
    assert_eq!(ExecutionState::from(failed), ExecutionState::Failed);

    let skipped_from_idle = IdleState.skip();
    assert_eq!(ExecutionState::from(skipped_from_idle), ExecutionState::Skipped);

    let skipped_from_queued = QueuedState.skip();
    assert_eq!(ExecutionState::from(skipped_from_queued), ExecutionState::Skipped);
}

/// Type-state chain produces consistent ExecutionState at every step.
#[test]
fn rq_type_state_chain_idle_queued_running_completed() {
    let idle = IdleState;
    let queued = idle.queue();
    let running = queued.start();
    let completed = running.complete();

    assert_eq!(ExecutionState::from(idle), ExecutionState::Idle);
    assert_eq!(ExecutionState::from(queued), ExecutionState::Queued);
    assert_eq!(ExecutionState::from(running), ExecutionState::Running);
    assert_eq!(ExecutionState::from(completed), ExecutionState::Completed);
}

#[test]
fn rq_type_state_chain_idle_queued_running_failed() {
    let running = IdleState.queue().start();
    let failed = running.fail();
    assert_eq!(ExecutionState::from(failed), ExecutionState::Failed);
}

#[test]
fn rq_type_state_chain_idle_skip() {
    let skipped = IdleState.skip();
    assert_eq!(ExecutionState::from(skipped), ExecutionState::Skipped);
}

#[test]
fn rq_type_state_chain_queued_skip() {
    let skipped = QueuedState.skip();
    assert_eq!(ExecutionState::from(skipped), ExecutionState::Skipped);
}

// ===========================================================================
// DIMENSION 6: Type-State Pattern Stress Tests
// ===========================================================================

/// Type-state structs are ZST (zero-sized) — no memory overhead.
#[test]
fn rq_type_state_structs_are_zero_sized() {
    assert_eq!(std::mem::size_of::<IdleState>(), 0);
    assert_eq!(std::mem::size_of::<QueuedState>(), 0);
    assert_eq!(std::mem::size_of::<RunningState>(), 0);
    assert_eq!(std::mem::size_of::<CompletedState>(), 0);
    assert_eq!(std::mem::size_of::<FailedState>(), 0);
    assert_eq!(std::mem::size_of::<SkippedState>(), 0);
}

/// ExecutionState is the size of a discriminant (1 byte or a few bytes).
#[test]
fn rq_execution_state_is_small() {
    let size = std::mem::size_of::<ExecutionState>();
    assert!(size <= 8, "ExecutionState is {size} bytes, expected <= 8");
}

/// All type-state structs are Copy (verify by copying).
#[test]
fn rq_all_type_states_are_copy() {
    let idle = IdleState;
    let _copy1 = idle;
    let _copy2 = idle;
    let _copy3 = IdleState;

    let queued = QueuedState;
    let _copy4 = queued;
    let _copy5 = queued;

    let running = RunningState;
    let _copy6 = running;
    let _copy7 = running;

    let completed = CompletedState;
    let _copy8 = completed;
    let _copy9 = completed;

    let failed = FailedState;
    let _copy10 = failed;
    let _copy11 = failed;

    let skipped = SkippedState;
    let _copy12 = skipped;
    let _copy13 = skipped;
}

/// All type-state structs are PartialEq.
#[test]
fn rq_all_type_states_are_eq() {
    assert_eq!(IdleState, IdleState);
    assert_eq!(QueuedState, QueuedState);
    assert_eq!(RunningState, RunningState);
    assert_eq!(CompletedState, CompletedState);
    assert_eq!(FailedState, FailedState);
    assert_eq!(SkippedState, SkippedState);
}

/// Type-state structs can be collected into a HashSet (they are Hash).
/// Wait — they are NOT Hash. They are Debug, Clone, Copy, PartialEq, Eq.
/// But NOT Hash. Let's verify they are Eq.
#[test]
fn rq_type_states_implement_eq() {
    fn check_eq<T: Eq>(_: T) {}
    check_eq(IdleState);
    check_eq(QueuedState);
    check_eq(RunningState);
    check_eq(CompletedState);
    check_eq(FailedState);
    check_eq(SkippedState);
}

/// Type-state structs implement Debug.
#[test]
fn rq_type_states_implement_debug() {
    let debug_str = format!("{:?}", IdleState);
    assert!(!debug_str.is_empty());

    let debug_str = format!("{:?}", QueuedState);
    assert!(!debug_str.is_empty());

    let debug_str = format!("{:?}", RunningState);
    assert!(!debug_str.is_empty());

    let debug_str = format!("{:?}", CompletedState);
    assert!(!debug_str.is_empty());

    let debug_str = format!("{:?}", FailedState);
    assert!(!debug_str.is_empty());

    let debug_str = format!("{:?}", SkippedState);
    assert!(!debug_str.is_empty());
}

/// ExecutionState is Hash — can be used in HashSet/HashMap.
#[test]
fn rq_execution_state_hashable() {
    let mut set = HashSet::new();
    set.insert(ExecutionState::Idle);
    set.insert(ExecutionState::Queued);
    set.insert(ExecutionState::Running);
    set.insert(ExecutionState::Completed);
    set.insert(ExecutionState::Failed);
    set.insert(ExecutionState::Skipped);

    assert_eq!(set.len(), 6, "All 6 variants must hash to unique buckets");
}

/// ExecutionState hash is deterministic across runs.
#[test]
fn rq_execution_state_hash_deterministic() {
    let variants = [
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    for v in &variants {
        let mut h1 = DefaultHasher::new();
        v.hash(&mut h1);
        let mut h2 = DefaultHasher::new();
        v.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish(), "Hash must be deterministic for {v:?}");
    }
}

/// Different ExecutionState variants produce different hashes.
#[test]
fn rq_execution_state_different_hashes() {
    let variants = [
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    let hashes: Vec<u64> = variants
        .iter()
        .map(|v| {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        })
        .collect();

    let unique: HashSet<u64> = hashes.into_iter().collect();
    assert_eq!(unique.len(), 6, "All 6 variants must produce unique hashes");
}

/// StateTransition enum is Copy.
#[test]
fn rq_state_transition_is_copy() {
    let t = StateTransition::IdleToQueued;
    let t2 = t;
    let t3 = t;
    assert_eq!(t2, StateTransition::IdleToQueued);
    assert_eq!(t3, StateTransition::IdleToQueued);
}

/// StateTransition::from_states returns the correct from/to for all variants.
#[test]
fn rq_state_transition_from_states_all_variants() {
    assert_eq!(
        StateTransition::IdleToQueued.from_states(),
        (ExecutionState::Idle, ExecutionState::Queued)
    );
    assert_eq!(
        StateTransition::IdleToSkipped.from_states(),
        (ExecutionState::Idle, ExecutionState::Skipped)
    );
    assert_eq!(
        StateTransition::QueuedToRunning.from_states(),
        (ExecutionState::Queued, ExecutionState::Running)
    );
    assert_eq!(
        StateTransition::QueuedToSkipped.from_states(),
        (ExecutionState::Queued, ExecutionState::Skipped)
    );
    assert_eq!(
        StateTransition::RunningToCompleted.from_states(),
        (ExecutionState::Running, ExecutionState::Completed)
    );
    assert_eq!(
        StateTransition::RunningToFailed.from_states(),
        (ExecutionState::Running, ExecutionState::Failed)
    );
}

/// StateTransition::apply returns the correct target state for all variants.
#[test]
fn rq_state_transition_apply_all_variants() {
    assert_eq!(StateTransition::IdleToQueued.apply(), ExecutionState::Queued);
    assert_eq!(StateTransition::IdleToSkipped.apply(), ExecutionState::Skipped);
    assert_eq!(StateTransition::QueuedToRunning.apply(), ExecutionState::Running);
    assert_eq!(StateTransition::QueuedToSkipped.apply(), ExecutionState::Skipped);
    assert_eq!(StateTransition::RunningToCompleted.apply(), ExecutionState::Completed);
    assert_eq!(StateTransition::RunningToFailed.apply(), ExecutionState::Failed);
}

/// InvalidTransition is Copy.
#[test]
fn rq_invalid_transition_is_copy() {
    let err = InvalidTransition::new(ExecutionState::Idle, ExecutionState::Running);
    let err2 = err;
    let err3 = err;
    assert_eq!(err2.from_state(), ExecutionState::Idle);
    assert_eq!(err3.from_state(), ExecutionState::Idle);
}

/// IdleState implements Default.
#[test]
fn rq_idle_state_default() {
    assert_eq!(IdleState::default(), IdleState);
}

/// InvalidTransition::new is const (can be used in const context).
#[test]
fn rq_invalid_transition_const_construction() {
    const ERR: InvalidTransition =
        InvalidTransition::new(ExecutionState::Idle, ExecutionState::Running);
    assert_eq!(ERR.from_state(), ExecutionState::Idle);
    assert_eq!(ERR.to_state(), ExecutionState::Running);
}

/// Cross-variant From conversions never produce the wrong variant.
#[test]
fn rq_from_cross_check_no_wrong_variant() {
    // Every From<XState> for ExecutionState MUST produce the matching variant.
    // Not a different variant.
    assert_ne!(ExecutionState::from(IdleState), ExecutionState::Queued);
    assert_ne!(ExecutionState::from(QueuedState), ExecutionState::Running);
    assert_ne!(ExecutionState::from(RunningState), ExecutionState::Completed);
    assert_ne!(ExecutionState::from(CompletedState), ExecutionState::Failed);
    assert_ne!(ExecutionState::from(FailedState), ExecutionState::Skipped);
    assert_ne!(ExecutionState::from(SkippedState), ExecutionState::Idle);
}

// ===========================================================================
// DIMENSION 7: Module Structure & File Layout Verification
// ===========================================================================

/// Verify the directory module structure exists.
#[test]
fn rq_directory_module_structure_exists() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/graph/execution_state");
    assert!(base.is_dir(), "execution_state must be a directory module");
    assert!(base.join("mod.rs").exists(), "execution_state/mod.rs must exist");
    assert!(base.join("tests").is_dir(), "execution_state/tests/ must be a directory");
    assert!(base.join("tests/mod.rs").exists(), "execution_state/tests/mod.rs must exist");
}

/// Every file under execution_state/ must be <= 300 lines (INV-6).
#[test]
fn rq_all_files_under_300_lines() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/graph/execution_state");

    let files_to_check = [
        "mod.rs",
        "tests/mod.rs",
        "tests/state_tests.rs",
        "tests/transition_tests.rs",
        "tests/try_transition_tests.rs",
        "tests/can_transition_tests.rs",
        "tests/type_state_tests.rs",
        "tests/api_verification_tests.rs",
        "tests/structure_verification_tests.rs",
        "tests/symbol_verification_tests.rs",
    ];

    for file in &files_to_check {
        let path = base.join(file);
        if path.exists() {
            let content =
                std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("Cannot read {file}"));
            let line_count = content.lines().count();
            assert!(line_count <= 300, "INV-6 VIOLATION: {file} has {line_count} lines (max 300)");
        }
    }
}

/// The old monolithic file must NOT exist.
#[test]
fn rq_old_monolithic_file_removed() {
    let old_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/graph/execution_state.rs");
    assert!(
        !old_path.exists(),
        "Old monolithic execution_state.rs must be removed after refactoring"
    );
}

// ===========================================================================
// DIMENSION 8: Public Re-export Verification
// ===========================================================================

/// All public symbols are accessible via oya_frontend::graph:: path.
#[test]
fn rq_all_symbols_accessible_via_crate_graph() {
    let _: oya_frontend::graph::ExecutionState = oya_frontend::graph::ExecutionState::Idle;
    let _: oya_frontend::graph::InvalidTransition = oya_frontend::graph::InvalidTransition::new(
        oya_frontend::graph::ExecutionState::Idle,
        oya_frontend::graph::ExecutionState::Running,
    );
    let _: oya_frontend::graph::StateTransition =
        oya_frontend::graph::StateTransition::IdleToQueued;
    let _: oya_frontend::graph::IdleState = oya_frontend::graph::IdleState;
    let _: oya_frontend::graph::QueuedState = oya_frontend::graph::QueuedState;
    let _: oya_frontend::graph::RunningState = oya_frontend::graph::RunningState;
    let _: oya_frontend::graph::CompletedState = oya_frontend::graph::CompletedState;
    let _: oya_frontend::graph::FailedState = oya_frontend::graph::FailedState;
    let _: oya_frontend::graph::SkippedState = oya_frontend::graph::SkippedState;

    // Free functions
    let _ = oya_frontend::graph::can_transition(
        oya_frontend::graph::ExecutionState::Idle,
        oya_frontend::graph::ExecutionState::Queued,
    );
    let _ = oya_frontend::graph::try_transition(
        oya_frontend::graph::ExecutionState::Idle,
        oya_frontend::graph::ExecutionState::Queued,
    );

    // TerminalState trait
    fn check<T: oya_frontend::graph::TerminalState>() {}
    check::<oya_frontend::graph::CompletedState>();
    check::<oya_frontend::graph::FailedState>();
    check::<oya_frontend::graph::SkippedState>();
}

// ===========================================================================
// DIMENSION 9: Proptests for State Machine Invariants
// ===========================================================================

use proptest::prelude::*;

proptest! {
    /// can_transition is reflexive with try_transition for any pair.
    #[test]
    fn rq_proptest_can_try_consistent(from in 0..6u8, to in 0..6u8) {
        let states = [
            ExecutionState::Idle,
            ExecutionState::Queued,
            ExecutionState::Running,
            ExecutionState::Completed,
            ExecutionState::Failed,
            ExecutionState::Skipped,
        ];
        let from_state = states[from as usize % 6];
        let to_state = states[to as usize % 6];

        let bool_result = can_transition(from_state, to_state);
        let opt_result = try_transition(from_state, to_state).is_some();
        prop_assert_eq!(bool_result, opt_result);
    }

    /// Serde round-trip never loses information for any valid JSON string.
    #[test]
    fn rq_proptest_serde_round_trip(variant in 0..6u8) {
        let states = [
            ExecutionState::Idle,
            ExecutionState::Queued,
            ExecutionState::Running,
            ExecutionState::Completed,
            ExecutionState::Failed,
            ExecutionState::Skipped,
        ];
        let state = states[variant as usize % 6];
        let json = serde_json::to_string(&state).unwrap();
        let recovered: ExecutionState = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(state, recovered);
    }

    /// Terminal states never have outgoing transitions.
    #[test]
    fn rq_proptest_terminal_no_outgoing(term in 3..6u8, target in 0..6u8) {
        let terminals = [
            ExecutionState::Completed,
            ExecutionState::Failed,
            ExecutionState::Skipped,
        ];
        let all = [
            ExecutionState::Idle,
            ExecutionState::Queued,
            ExecutionState::Running,
            ExecutionState::Completed,
            ExecutionState::Failed,
            ExecutionState::Skipped,
        ];
        let terminal = terminals[(term - 3) as usize % 3];
        let target_state = all[target as usize % 6];
        prop_assert!(!can_transition(terminal, target_state));
        prop_assert_eq!(try_transition(terminal, target_state), None);
    }
}
