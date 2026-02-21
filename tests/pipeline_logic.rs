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
fn given_pipeline_start_when_checking_first_stage_then_is_plan() {
    use oya::types::StageName;

    let next = StageName::Contract.next();
    assert_eq!(next, Some(StageName::Contract));
}

#[test]
fn given_plan_stage_when_checking_model_tier_then_is_c() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::Contract.model_for_stage(), ModelTier::Balanced);
}

#[test]
fn given_contract_stage_when_checking_model_tier_then_is_d() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::Contract.model_for_stage(), ModelTier::Fast);
}

#[test]
fn given_qa_stage_when_checking_model_tier_then_is_c() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::Implementation.model_for_stage(), ModelTier::Balanced);
}

#[test]
fn given_redqueen_stage_when_checking_model_tier_then_is_b() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::ShipGate.model_for_stage(), ModelTier::Capable);
}

#[test]
fn given_gpt_review_stage_when_checking_model_tier_then_is_a() {
    use oya::types::{ModelTier, StageName};

    assert_eq!(StageName::ShipGate.model_for_stage(), ModelTier::Best);
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
fn given_stage_order_when_verifying_then_follows_pipeline_flow() {
    use oya::types::StageName;

    // ATDD pipeline flow
    let stages = vec![
        StageName::Contract,
        StageName::Contract,
        StageName::Implementation,
        StageName::Implementation,
        StageName::Implementation,
        StageName::ShipGate,
        StageName::ShipGate,
        StageName::ShipGate,
    ];

    for i in 0..stages.len() - 1 {
        assert_eq!(stages[i].next(), Some(stages[i + 1].clone()));
    }

    // Verify terminal stage
    assert_eq!(StageName::ShipGate.next(), None);

    // Verify legacy Tdd15 still works
    assert_eq!(StageName::Implementation.next(), Some(StageName::Implementation));
}

#[test]
fn given_acceptance_test_stage_when_checking_gates_then_compiles_and_red_required() {
    use oya::types::{Gate, StageName};

    let gates = StageName::Implementation.gates();
    assert_eq!(gates.len(), 2);
    assert!(gates.contains(&Gate::Compiles));
    assert!(gates.contains(&Gate::TestsPass));
}

// =============================================================================
// GATE DEFINITIONS

#[test]
fn given_any_stage_when_checking_max_attempts_then_is_always_two() {
    use oya::types::StageName;

    for stage in [
        StageName::Contract,
        StageName::Contract,
        StageName::Implementation,
        StageName::Implementation,
        StageName::Implementation,
        StageName::Implementation,
        StageName::ShipGate,
        StageName::ShipGate,
        StageName::ShipGate,
    ] {
        assert_eq!(stage.max_attempts(), 2);
    }
}

#[test]
fn given_plan_stage_when_checking_gates_then_only_compiles_required() {
    use oya::types::StageName;

    let gates = StageName::Contract.gates();
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0], oya::types::Gate::Compiles);
}

#[test]
fn given_contract_stage_when_checking_gates_then_only_compiles_required() {
    use oya::types::StageName;

    let gates = StageName::Contract.gates();
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0], oya::types::Gate::Compiles);
}

#[test]
fn given_tdd15_stage_when_checking_gates_then_compiles_and_tests_required() {
    use oya::types::StageName;

    let gates = StageName::Implementation.gates();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0], oya::types::Gate::Compiles);
    assert_eq!(gates[1], oya::types::Gate::TestsPass);
}

#[test]
fn given_qa_stage_when_checking_gates_then_tests_and_edge_cases_required() {
    use oya::types::StageName;

    let gates = StageName::Implementation.gates();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0], oya::types::Gate::TestsPass);
    assert_eq!(gates[1], oya::types::Gate::TestsPass);
}

#[test]
fn given_redqueen_stage_when_checking_gates_then_no_vulnerabilities_required() {
    use oya::types::StageName;

    let gates = StageName::ShipGate.gates();
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0], oya::types::Gate::MoonCi);
}

#[test]
fn given_gpt_review_stage_when_checking_gates_then_clippy_and_security_required() {
    use oya::types::StageName;

    let gates = StageName::ShipGate.gates();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0], oya::types::Gate::Compiles);
    assert_eq!(gates[1], oya::types::Gate::MoonCi);
}

#[test]
fn given_shipgate_stage_when_checking_gates_then_moon_ci_and_merge_queue_required() {
    use oya::types::StageName;

    let gates = StageName::ShipGate.gates();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0], oya::types::Gate::MoonCi);
    assert_eq!(gates[1], oya::types::Gate::ZjjMergeQueue);
}
