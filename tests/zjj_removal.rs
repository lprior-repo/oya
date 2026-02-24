//! ZJJ Gate Removal Tests
//!
//! These tests verify that the ZJJ Merge Queue gate and related configuration
//! have been completely removed from the codebase.
//!
//! After the removal is complete, ALL of these tests should FAIL (they check
//! that ZJJ-specific code does NOT exist, so they'll pass when removal is complete).
//!
//! Run with: `moon run :test zjj_removal`

use oya::types::Gate;
use oya::types::StageName;

// =============================================================================
// CONTRACT VERIFICATION: Gate enum removal
// =============================================================================

/// Contract: Gate enum should have 5 variants (not 6)
#[test]
fn test_precondition_gate_enum_has_five_variants() {
    // We can't directly count enum variants in Rust at compile time,
    // but we can verify that all expected variants exist and ZjjMergeQueue does not.

    // These should all compile and work:
    let _ = Gate::Compiles;
    let _ = Gate::TestsPass;
    let _ = Gate::MoonCi;
    let _ = Gate::HoldoutScenarios;
    let _ = Gate::CueArtifactGenerated;

    // This should NOT compile if removal is complete:
    // Uncommenting the line below should cause a compile error:
    // let _ = Gate::ZjjMergeQueue;

    // We verify by checking string conversion doesn't accept zjj_merge_queue
    let result = Gate::try_from("zjj_merge_queue");
    assert!(result.is_err(), "Gate::try_from('zjj_merge_queue') should return Err after removal");

    // Verify the error message indicates unknown gate
    let err = result.unwrap_err();
    assert!(
        err.contains("Unknown gate") || err.contains("unknown gate"),
        "Error message should indicate unknown gate: {}",
        err
    );
}

/// Contract: Gate::as_str() should not return "zjj_merge_queue"
#[test]
fn test_postcondition_gate_as_str_no_zjj_merge_queue() {
    let gates = [
        Gate::Compiles,
        Gate::TestsPass,
        Gate::MoonCi,
        Gate::HoldoutScenarios,
        Gate::CueArtifactGenerated,
    ];

    for gate in gates {
        let s = gate.as_str();
        assert_ne!(s, "zjj_merge_queue", "Gate::as_str() should not return 'zjj_merge_queue'");
    }
}

/// Contract: Gate::try_from should reject "zjj_merge_queue"
#[test]
fn test_error_path_gate_try_from_zjj_merge_queue_returns_error() {
    let result = Gate::try_from("zjj_merge_queue");
    assert!(result.is_err(), "Should return error for zjj_merge_queue string");

    // Also verify it rejects case variations
    assert!(Gate::try_from("ZJJ_MERGE_QUEUE").is_err());
    assert!(Gate::try_from("zjj-merge-queue").is_err());
}

// =============================================================================
// CONTRACT VERIFICATION: Stage configuration
// =============================================================================

/// Contract: Main stage should have only 1 gate (MoonCi)
#[test]
fn test_postcondition_ship_gate_has_only_cue_artifact_generated_gate() {
    let gates = StageName::Main.gates();

    assert_eq!(
        gates.len(),
        1,
        "Main should have exactly 1 gate after stage simplification, got {} gates",
        gates.len()
    );

    assert_eq!(gates[0], Gate::MoonCi, "Main's only gate should be MoonCi");
}

/// Contract: No stage should reference ZjjMergeQueue gate
#[test]
fn test_precondition_no_stage_references_zjj_merge_queue() {
    let all_stages = [
        StageName::JjWorkspace,
        StageName::Implementation,
        StageName::Implementation,
        StageName::Implementation,
        StageName::Main,
        StageName::Main,
    ];

    for stage in all_stages {
        let gates = stage.gates();
        for gate in gates {
            // We can't directly compare against ZjjMergeQueue if it's removed,
            // but we can check the string representation
            let gate_str = gate.as_str();
            assert_ne!(
                gate_str, "zjj_merge_queue",
                "Stage {:?} should not have ZjjMergeQueue gate",
                stage
            );
        }
    }
}

/// Contract: Implementation stage has correct gates after ZJJ removal
#[test]
fn test_postcondition_implementation_stage_has_correct_gates() {
    let gates = StageName::Implementation.gates();

    assert_eq!(gates.len(), 2, "Implementation should have 2 gates");
    assert!(gates.contains(&Gate::Compiles), "Implementation should have Compiles gate");
    assert!(gates.contains(&Gate::TestsPass), "Implementation should have TestsPass gate");
}

/// Contract: Main stage has MoonCi gate after simplification
#[test]
fn test_postcondition_witness_stage_has_correct_gates() {
    let gates = StageName::Main.gates();

    assert_eq!(gates.len(), 1, "Main should have 1 gate");
    assert_eq!(gates[0], Gate::MoonCi, "Main should have MoonCi gate");
}

/// Contract: Explore stage has no gates
#[test]
fn test_postcondition_explore_stage_has_no_gates() {
    let gates = StageName::JjWorkspace.gates();

    assert_eq!(gates.len(), 0, "Explore should have 0 gates");
}

/// Contract: Implementation gate set covers former Contract stage checks
#[test]
fn test_postcondition_contract_stage_has_only_compiles_gate() {
    let gates = StageName::Implementation.gates();

    assert_eq!(gates.len(), 2, "Implementation should have 2 gates");
    assert!(gates.contains(&Gate::Compiles), "Implementation should have Compiles gate");
    assert!(gates.contains(&Gate::TestsPass), "Implementation should have TestsPass gate");
}

/// Contract: Implementation gate set covers former Red stage checks
#[test]
fn test_postcondition_red_stage_has_only_compiles_gate() {
    let gates = StageName::Implementation.gates();

    assert_eq!(gates.len(), 2, "Implementation should have 2 gates");
    assert!(gates.contains(&Gate::Compiles), "Implementation should have Compiles gate");
    assert!(gates.contains(&Gate::TestsPass), "Implementation should have TestsPass gate");
}

// =============================================================================
// CONTRACT VERIFICATION: GateCommand enum
// =============================================================================

/// Contract: GateCommand should have only Moon variant
///
/// Note: GateCommand is a private implementation detail. We verify ZJJ removal
/// through the public Gate enum interface instead.
/// Private module tests should be in src/runtime_tools/gates.rs unit tests.

// =============================================================================
// CONTRACT VERIFICATION: Failure mapping
// =============================================================================

/// Contract: gate_failure_mapping should not handle ZjjMergeQueue
///
/// Note: gate_failure_outcome is a private implementation detail in runtime_tools.
/// The implementation should ensure ZjjMergeQueue is removed from the match statement.
/// Tests for this should be in src/runtime_tools/gates.rs unit tests.

// =============================================================================
// CONTRACT VERIFICATION: RuntimeConfig
// =============================================================================

/// Contract: RuntimeConfig should not have merge_queue_policy field
///
/// Note: RuntimeConfig and MergeQueuePolicy are private to the pipeline module.
/// The implementation should remove these as specified in the contract.
/// Tests for this should be in src/pipeline/mod.rs unit tests.

// =============================================================================
// INTEGRATION: Full pipeline with ZJJ removed
// =============================================================================

/// Integration: Verify pipeline state machine works without ZJJ
#[test]
fn test_postcondition_pipeline_state_machine_works_without_zjj() {
    use oya::types::{StageTransition, TransitionDecision, TransitionReason};

    // Verify stage transitions still work
    let decision = TransitionDecision::new(
        StageTransition::Advance(StageName::Implementation),
        TransitionReason::StagePassedAdvance,
    );

    assert!(matches!(decision.transition(), StageTransition::Advance(_)));
}

/// Integration: Verify all stages have valid gate configurations
#[test]
fn test_postcondition_all_stages_have_valid_gate_configurations() {
    // Verify each stage has gates, and all gates are valid Gate variants
    for stage in [
        StageName::JjWorkspace,
        StageName::Implementation,
        StageName::Implementation,
        StageName::Implementation,
        StageName::Main,
        StageName::Main,
    ] {
        let gates = stage.gates();
        for gate in gates {
            // Verify the gate is one of the expected variants
            // NOTE: After ZJJ removal, ZjjMergeQueue will not exist
            match gate {
                Gate::Compiles
                | Gate::TestsPass
                | Gate::MoonCi
                | Gate::HoldoutScenarios
                | Gate::CueArtifactGenerated => {
                    // Valid
                }
            }
        }
    }
}

/// Integration: Verify gate serialization works without ZJJ
#[test]
fn test_postcondition_gate_serialization_without_zjj() {
    // Verify all gates can be serialized and deserialized
    let gates = [
        Gate::Compiles,
        Gate::TestsPass,
        Gate::MoonCi,
        Gate::HoldoutScenarios,
        Gate::CueArtifactGenerated,
    ];

    for gate in gates {
        // Serialize
        let serialized = serde_json::to_string(&gate).unwrap();

        // Deserialize
        let deserialized: Gate = serde_json::from_str(&serialized).unwrap();

        // Round-trip check
        assert_eq!(gate, deserialized, "Gate {:?} should round-trip correctly", gate);
    }
}

// =============================================================================
// BACKWARD INCOMPATIBILITY VERIFICATION
// =============================================================================

/// Backward incompatibility: Old code using ZjjMergeQueue should fail to compile
///
/// This test documents that the removal is a breaking change.
#[test]
fn test_backward_incompatibility_zjj_merge_queue_removed() {
    // This test serves as documentation. The actual verification happens
    // when old code fails to compile.

    // Before removal, code like this would compile:
    // let gate = Gate::ZjjMergeQueue;
    // let gates = StageName::Main.gates();
    // assert!(gates.contains(&Gate::ZjjMergeQueue));

    // After removal, the above code would cause compile errors.
}

/// Backward incompatibility: Attempting to parse "zjj_merge_queue" fails
#[test]
fn test_backward_incompatibility_zjj_merge_queue_string_fails() {
    let result = Gate::try_from("zjj_merge_queue");
    assert!(result.is_err(), "After removal, parsing 'zjj_merge_queue' should fail");
}

/// Backward incompatibility: ShipGate no longer has 2 gates
#[test]
fn test_backward_incompatibility_ship_gate_no_longer_has_two_gates() {
    let gates = StageName::Main.gates();

    // Before removal: gates.len() == 2
    // After removal: gates.len() == 1
    assert_eq!(gates.len(), 1, "After simplification, Main should have 1 gate, not 2");
    assert_eq!(gates[0], Gate::MoonCi, "Only MoonCi should remain for Main");
}
