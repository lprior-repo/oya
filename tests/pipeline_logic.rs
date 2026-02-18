//! Pipeline state management behavior tests
//!
//! These test the core orchestration logic extracted from main.rs

use oya::types::{FailureCategory, StageName};

/// The decision engine: Given a failure category, should we retry?
#[test]
fn given_testfailed_error_when_checking_retryable_then_should_retry() {
    use oya::is_retryable_failure;

    assert!(is_retryable_failure(&FailureCategory::TestFailed));
}

#[test]
fn given_compilefailed_error_when_checking_retryable_then_should_not_retry() {
    use oya::is_retryable_failure;

    // CompileFailed is NOT retryable - you need to fix the code
    assert!(!is_retryable_failure(&FailureCategory::CompileFailed));
}

#[test]
fn given_lintfailed_error_when_checking_retryable_then_should_retry() {
    use oya::is_retryable_failure;

    assert!(is_retryable_failure(&FailureCategory::LintFailed));
}

#[test]
fn given_outputparsefailure_when_checking_retryable_then_should_retry() {
    use oya::is_retryable_failure;

    assert!(is_retryable_failure(&FailureCategory::OutputParseFailure));
}

#[test]
fn given_authfailed_error_when_checking_retryable_then_should_not_retry() {
    use oya::is_retryable_failure;

    assert!(!is_retryable_failure(&FailureCategory::AuthFailed));
}

#[test]
fn given_rate_limited_when_checking_retryable_then_should_not_retry() {
    use oya::is_retryable_failure;

    assert!(!is_retryable_failure(&FailureCategory::RateLimited));
}

#[test]
fn given_merge_conflict_when_checking_retryable_then_should_not_retry() {
    use oya::is_retryable_failure;

    assert!(!is_retryable_failure(&FailureCategory::MergeConflict));
}

// =============================================================================
// STAGE TRANSITION LOGIC
// =============================================================================

/// Verify all stages have max 3 attempts
#[test]
fn given_any_stage_when_checking_max_attempts_then_is_always_three() {
    let stages = vec![
        StageName::Research,
        StageName::Plan,
        StageName::Contract,
        StageName::Tdd15,
        StageName::Qa,
        StageName::RedQueen,
        StageName::GptReview,
        StageName::ShipGate,
    ];

    for stage in stages {
        assert_eq!(stage.max_attempts(), 3, "{:?} should have 3 max attempts", stage);
    }
}

/// Verify stage ordering is consistent
#[test]
fn given_stage_order_when_verifying_then_follows_pipeline_flow() {
    assert_eq!(StageName::Research.next(), Some(StageName::Plan));
    assert_eq!(StageName::Plan.next(), Some(StageName::Contract));
    assert_eq!(StageName::Contract.next(), Some(StageName::Tdd15));
    assert_eq!(StageName::Tdd15.next(), Some(StageName::Qa));
    assert_eq!(StageName::Qa.next(), Some(StageName::RedQueen));
    assert_eq!(StageName::RedQueen.next(), Some(StageName::GptReview));
    assert_eq!(StageName::GptReview.next(), Some(StageName::ShipGate));
    assert_eq!(StageName::ShipGate.next(), None);
}

/// Verify Research is the starting stage
#[test]
fn given_pipeline_start_when_checking_first_stage_then_is_research() {
    // Research is typically the first stage in any pipeline
    let research = StageName::Research;
    assert_eq!(research.as_str(), "research");
    assert_eq!(research.next(), Some(StageName::Plan));
}

/// Verify ShipGate is terminal
#[test]
fn given_shipgate_when_checking_next_stage_then_is_none() {
    assert_eq!(StageName::ShipGate.next(), None, "ShipGate should be terminal");
}

// =============================================================================
// MODEL TIER ASSIGNMENTS
// =============================================================================

#[test]
fn given_research_stage_when_checking_model_tier_then_is_fast() {
    assert_eq!(StageName::Research.model_for_stage().as_str(), "fast");
}

#[test]
fn given_contract_stage_when_checking_model_tier_then_is_fast() {
    assert_eq!(StageName::Contract.model_for_stage().as_str(), "fast");
}

#[test]
fn given_tdd15_stage_when_checking_model_tier_then_is_balanced() {
    assert_eq!(StageName::Tdd15.model_for_stage().as_str(), "balanced");
}

#[test]
fn given_qa_stage_when_checking_model_tier_then_is_balanced() {
    assert_eq!(StageName::Qa.model_for_stage().as_str(), "balanced");
}

#[test]
fn given_redqueen_stage_when_checking_model_tier_then_is_capable() {
    assert_eq!(StageName::RedQueen.model_for_stage().as_str(), "capable");
}

#[test]
fn given_gptreview_stage_when_checking_model_tier_then_is_capable() {
    assert_eq!(StageName::GptReview.model_for_stage().as_str(), "capable");
}

#[test]
fn given_shipgate_stage_when_checking_model_tier_then_is_best() {
    assert_eq!(StageName::ShipGate.model_for_stage().as_str(), "best");
}
