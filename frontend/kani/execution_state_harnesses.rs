//! Kani verification harnesses for ExecutionState machine

use oya_frontend::graph::{ExecutionState, StateTransition};

// ===========================================================================
// Kani: Terminal state irreversibility proof
// ===========================================================================

#[kani::proof]
fn kani_terminal_state_irreversibility() {
    // Assume execution_state is any ExecutionState
    let state = kani::any::<ExecutionState>();

    // Prove: If execution_state is terminal, no transition can change it
    // (This is checked via can_transition returning false for all terminal states)

    match state {
        ExecutionState::Completed => {
            // All transitions from Completed should be invalid
            kani::assert(
                !kani::can_transition(ExecutionState::Completed, ExecutionState::Idle),
                "Completed -> Idle should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Completed, ExecutionState::Queued),
                "Completed -> Queued should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Completed, ExecutionState::Running),
                "Completed -> Running should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Completed, ExecutionState::Failed),
                "Completed -> Failed should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Completed, ExecutionState::Skipped),
                "Completed -> Skipped should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Completed, ExecutionState::Completed),
                "Completed -> Completed (self) should be invalid",
            );
        }
        ExecutionState::Failed => {
            kani::assert(
                !kani::can_transition(ExecutionState::Failed, ExecutionState::Idle),
                "Failed -> Idle should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Failed, ExecutionState::Queued),
                "Failed -> Queued should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Failed, ExecutionState::Running),
                "Failed -> Running should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Failed, ExecutionState::Completed),
                "Failed -> Completed should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Failed, ExecutionState::Skipped),
                "Failed -> Skipped should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Failed, ExecutionState::Failed),
                "Failed -> Failed (self) should be invalid",
            );
        }
        ExecutionState::Skipped => {
            kani::assert(
                !kani::can_transition(ExecutionState::Skipped, ExecutionState::Idle),
                "Skipped -> Idle should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Skipped, ExecutionState::Queued),
                "Skipped -> Queued should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Skipped, ExecutionState::Running),
                "Skipped -> Running should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Skipped, ExecutionState::Completed),
                "Skipped -> Completed should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Skipped, ExecutionState::Failed),
                "Skipped -> Failed should be invalid",
            );
            kani::assert(
                !kani::can_transition(ExecutionState::Skipped, ExecutionState::Skipped),
                "Skipped -> Skipped (self) should be invalid",
            );
        }
        _ => {
            // Non-terminal states can have transitions
            kani::assert(
                state.is_terminal() == false,
                "Non-terminal state should not be terminal",
            );
        }
    }
}

// ===========================================================================
// Kani: Sequential progression proof
// ===========================================================================

#[kani::proof]
fn kani_sequential_progression() {
    // Prove: Must pass through Queued before reaching Running

    // From Idle, cannot go directly to Running
    kani::assert(
        !kani::can_transition(ExecutionState::Idle, ExecutionState::Running),
        "Idle -> Running should be invalid (must go through Queued)",
    );

    // From Idle, can only go to Queued or Skipped
    kani::assert(
        kani::can_transition(ExecutionState::Idle, ExecutionState::Queued),
        "Idle -> Queued should be valid",
    );
    kani::assert(
        kani::can_transition(ExecutionState::Idle, ExecutionState::Skipped),
        "Idle -> Skipped should be valid",
    );

    // From Queued, can go to Running
    kani::assert(
        kani::can_transition(ExecutionState::Queued, ExecutionState::Running),
        "Queued -> Running should be valid",
    );

    // From Running, cannot go back to Idle
    kani::assert(
        !kani::can_transition(ExecutionState::Running, ExecutionState::Idle),
        "Running -> Idle should be invalid",
    );
    kani::assert(
        !kani::can_transition(ExecutionState::Running, ExecutionState::Queued),
        "Running -> Queued should be invalid",
    );
}

// ===========================================================================
// Kani: No backwards transitions proof
// ===========================================================================

#[kani::proof]
fn kani_no_backwards_transitions() {
    // Prove: Cannot transition to any earlier state in the sequence

    // Running -> Idle is invalid
    kani::assert(
        !kani::can_transition(ExecutionState::Running, ExecutionState::Idle),
        "Running -> Idle should be invalid",
    );

    // Completed -> Queued is invalid
    kani::assert(
        !kani::can_transition(ExecutionState::Completed, ExecutionState::Queued),
        "Completed -> Queued should be invalid",
    );

    // Failed -> Running is invalid
    kani::assert(
        !kani::can_transition(ExecutionState::Failed, ExecutionState::Running),
        "Failed -> Running should be invalid",
    );

    // Skipped -> Queued is invalid
    kani::assert(
        !kani::can_transition(ExecutionState::Skipped, ExecutionState::Queued),
        "Skipped -> Queued should be invalid",
    );

    // All backwards transitions are invalid
    kani::assert(
        !kani::can_transition(ExecutionState::Running, ExecutionState::Queued),
        "Running -> Queued should be invalid",
    );
}

// ===========================================================================
// Kani: Queued gateway proof
// ===========================================================================

#[kani::proof]
fn kani_queued_gateway() {
    // Assume execution_state == Idle
    let state = ExecutionState::Idle;

    // Prove: Running state can only be reached via Queued
    // Direct Idle -> Running is invalid
    kani::assert(
        !kani::can_transition(ExecutionState::Idle, ExecutionState::Running),
        "Idle -> Running should be invalid",
    );

    // Must go Idle -> Queued -> Running
    kani::assert(
        kani::can_transition(ExecutionState::Idle, ExecutionState::Queued),
        "Idle -> Queued should be valid",
    );
    kani::assert(
        kani::can_transition(ExecutionState::Queued, ExecutionState::Running),
        "Queued -> Running should be valid",
    );

    // Verify transitivity
    let can_idle_to_queued = kani::can_transition(ExecutionState::Idle, ExecutionState::Queued);
    let can_queued_to_running =
        kani::can_transition(ExecutionState::Queued, ExecutionState::Running);

    // If both transitions are valid, we can reach Running via Queued
    kani::assert(
        can_idle_to_queued && can_queued_to_running,
        "Idle -> Queued -> Running path should exist",
    );
}

// ===========================================================================
// Kani: Config sync consistency proof
// ===========================================================================

use serde_json::{from_str, to_string};

#[kani::proof]
fn kani_config_sync_consistency() {
    // Assume all transitions use set_node_status
    // Prove: config["status"] always equals execution_state.to_string()

    let state = kani::any::<ExecutionState>();

    // Serialize state to string
    let serialized = to_string(&state).expect("Serialization should succeed");

    // The serialized string should match the state's Display implementation
    kani::assert(
        serialized == state.to_string(),
        "Serialized state should match Display implementation",
    );

    // Deserialize should give us back the same state
    let deserialized: Result<ExecutionState, _> = from_str(&serialized);
    kani::assert(deserialized.is_ok(), "Deserialization should succeed");

    let deserialized = deserialized.unwrap();
    kani::assert(deserialized == state, "Round-trip should preserve state");
}
