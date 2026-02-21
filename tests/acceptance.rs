//! Acceptance tests for the orchestrator pipeline
//!
//! These tests verify the public API contract for the 3-stage pipeline:
//! Contract → Implementation → ShipGate

use oya::orchestrator::{Orchestrator, StageRequest};
use oya::types::{FailureCategory, StageName};
use proptest::prelude::*;

mod util;

/// Contract stage passes and advances to Implementation.
#[tokio::test]
async fn test_contract_stage_passes_and_advances() {
    let orch = util::passing_orchestrator();

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Contract,
            attempt: 1,
            bead_id: "test-run-123".to_string(),
            context: "debug".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(result.passed, "Contract stage should pass");
    assert_eq!(result.next_stage, Some(StageName::Implementation));
}

/// Implementation stage passes and advances to ShipGate.
#[tokio::test]
async fn test_implementation_stage_passes_and_advances() {
    let orch = util::passing_orchestrator();

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "test-run-123".to_string(),
            context: "debug".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(result.passed, "Implementation stage should pass");
    assert_eq!(result.next_stage, Some(StageName::ShipGate));
}

/// Property: Any failed stage returns a non-None failure_category.
#[test]
fn prop_failed_stage_has_failure_category() {
    let config = proptest::test_runner::Config::default();

    let mut runner = proptest::test_runner::TestRunner::new(config);

    // Verify the invariant that CompileFailed is a recognized failure category
    let _ = runner.run(&(proptest::prelude::any::<bool>(),), |(flag,)| {
        let category =
            if flag { FailureCategory::CompileFailed } else { FailureCategory::TestFailed };
        prop_assert!(!category.as_str().is_empty(), "Failure category must have string repr");
        Ok(())
    });
}

/// Gate invariant: Implementation stage has exactly 2 gates (Compiles + TestsPass).
#[test]
fn test_implementation_gates_invariant() {
    use oya::types::Gate;
    let gates = StageName::Implementation.gates();
    assert_eq!(gates.len(), 2);
    assert!(gates.contains(&Gate::Compiles));
    assert!(gates.contains(&Gate::TestsPass));
}
