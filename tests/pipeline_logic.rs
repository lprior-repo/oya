//! Pipeline state management behavior tests
//!
//! These test the core orchestration logic extracted from main.rs

use oya::types::FailureCategory;

/// The decision engine: Given a failure category, should we retry?
#[test]
fn given_testfailed_error_when_checking_retryable_then_should_retry() {
    use oya::is_retryable_failure;

    assert!(is_retryable_failure(&FailureCategory::TestFailed));
}

#[test]
fn given_compilefailed_error_when_checking_retryable_then_should_retry() {
    use oya::is_retryable_failure;

    // CompileFailed IS retryable - AI can fix compilation errors
    assert!(is_retryable_failure(&FailureCategory::CompileFailed));
}

#[test]
fn given_lintfailed_error_when_checking_retryable_then_should_retry() {
    use oya::is_retryable_failure;

    assert!(is_retryable_failure(&FailureCategory::LintFailed));
}

#[test]
fn given_outputparsefailure_error_when_checking_retryable_then_should_retry() {
    use oya::is_retryable_failure;

    assert!(is_retryable_failure(&FailureCategory::OutputParseFailure));
}

#[test]
fn given_rate_limited_when_checking_retryable_then_should_not_retry() {
    use oya::is_retryable_failure;

    // RateLimited is NOT retryable - it should trigger cooldown instead
    assert!(!is_retryable_failure(&FailureCategory::RateLimited));
}

#[test]
fn given_merge_conflict_when_checking_retryable_then_should_not_retry() {
    use oya::is_retryable_failure;

    assert!(!is_retryable_failure(&FailureCategory::MergeConflict));
}

#[test]
fn given_auth_failed_when_checking_retryable_then_should_not_retry() {
    use oya::is_retryable_failure;

    assert!(!is_retryable_failure(&FailureCategory::AuthFailed));
}

// =============================================================================
// STAGE TRANSITION LOGIC

#[test]
fn given_pipeline_start_when_checking_first_stage_then_is_contract() {
    use oya::types::StageName;

    let next = StageName::Contract.next();
    assert_eq!(next, Some(StageName::AcceptanceTest));
}

#[test]
fn given_contract_stage_when_checking_model_tier_then_is_fast() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::Contract.model_for_stage(), ModelTier::Fast);
}

#[test]
fn given_implementation_stage_when_checking_model_tier_then_is_balanced() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::Implementation.model_for_stage(), ModelTier::Balanced);
}

#[test]
fn given_review_stage_when_checking_model_tier_then_is_capable() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::Review.model_for_stage(), ModelTier::Capable);
}

#[test]
fn given_shipgate_stage_when_checking_model_tier_then_is_a() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::ShipGate.model_for_stage(), ModelTier::Best);
}

#[test]
fn given_shipgate_when_checking_next_stage_then_is_none() {
    use oya::types::StageName;

    let next = StageName::ShipGate.next();
    assert_eq!(next, None);
}

#[test]
fn given_stage_order_when_verifying_then_follows_five_stage_pipeline_flow() {
    use oya::types::StageName;

    // Canonical five-stage ATDD pipeline flow:
    // Contract -> AcceptanceTest -> Implementation -> Review -> ShipGate
    let stages = vec![
        StageName::Contract,
        StageName::AcceptanceTest,
        StageName::Implementation,
        StageName::Review,
        StageName::ShipGate,
    ];

    for i in 0..stages.len() - 1 {
        assert_eq!(stages[i].next(), Some(stages[i + 1].clone()));
    }

    // Verify terminal stage
    assert_eq!(StageName::ShipGate.next(), None);
}

#[test]
fn given_five_stages_when_counting_then_exactly_five() {
    use oya::types::StageName;

    // Count all non-legacy stage variants
    let canonical_stages = [
        StageName::Contract,
        StageName::AcceptanceTest,
        StageName::Implementation,
        StageName::Review,
        StageName::ShipGate,
    ];

    assert_eq!(canonical_stages.len(), 5, "Pipeline must have exactly 5 canonical stages");
}

#[test]
fn given_acceptance_test_stage_when_checking_gates_then_compiles_and_red_required() {
    use oya::types::{Gate, StageName};

    let gates = StageName::AcceptanceTest.gates();
    assert_eq!(gates.len(), 2);
    assert!(gates.contains(&Gate::Compiles));
    assert!(gates.contains(&Gate::AcceptanceTestsAreRed));
}

// =============================================================================
// GATE DEFINITIONS

#[test]
fn given_any_canonical_stage_when_checking_max_attempts_then_is_always_two() {
    use oya::types::StageName;

    for stage in [
        StageName::Contract,
        StageName::AcceptanceTest,
        StageName::Implementation,
        StageName::Review,
        StageName::ShipGate,
    ] {
        assert_eq!(stage.max_attempts(), 2);
    }
}

#[test]
fn given_contract_stage_when_checking_gates_then_only_compiles_required() {
    use oya::types::StageName;

    let gates = StageName::Contract.gates();
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0], oya::types::Gate::Compiles);
}

#[test]
fn given_implementation_stage_when_checking_gates_then_compiles_and_tests_required() {
    use oya::types::StageName;

    let gates = StageName::Implementation.gates();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0], oya::types::Gate::Compiles);
    assert_eq!(gates[1], oya::types::Gate::TestsPass);
}

#[test]
fn given_review_stage_when_checking_gates_then_quality_gates_required() {
    use oya::types::{Gate, StageName};

    let gates = StageName::Review.gates();
    // Review stage consolidates Qa + RedQueen + GptReview gates
    assert!(gates.contains(&Gate::TestsPass));
    assert!(gates.contains(&Gate::EdgeCases));
    assert!(gates.contains(&Gate::NoVulnerabilities));
    assert!(gates.contains(&Gate::ClippyClean));
    assert!(gates.contains(&Gate::Security));
}

#[test]
fn given_shipgate_stage_when_checking_gates_then_moon_ci_and_merge_queue_required() {
    use oya::types::StageName;

    let gates = StageName::ShipGate.gates();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0], oya::types::Gate::MoonCi);
    assert_eq!(gates[1], oya::types::Gate::ZjjMergeQueue);
}
