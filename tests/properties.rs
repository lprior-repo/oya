//! Property-based tests using proptest
//!
//! These tests automatically generate hundreds of test cases
//! to catch edge cases we might miss.

use oya::types::StageName;
use oya::{build_zjj_workspace_name, parse_opencode_sse_events};
use proptest::prelude::*;

// Property: Workspace names are always valid
//
// For any valid inputs, workspace name should:
// - Not be empty
// - Contain only allowed characters
// - Be within length limit
proptest! {
    #[test]
    fn prop_workspace_name_is_valid(
        // Limit run_id to ensure total workspace name stays under 64 chars
        // Format: oya-{run_id}-{stage}-a{attempt} = 4 + len(run_id) + 1 + len(stage) + 2 + len(attempt)
        // Max stage = "gpt_review" (10 chars), max attempt = "10" (2 chars)
        // So run_id max = 64 - 4 - 1 - 10 - 2 - 2 = 45 chars (use 40 for safety)
        run_id in "[a-zA-Z0-9_-]{1,40}",
        stage in "(plan|contract|tdd15|qa|red_queen|gpt_review|ship_gate)",
        attempt in 1u32..10,
    ) {
        // Skip inputs that would normalize to empty (e.g., just "-")
        // These are handled by a separate edge case test
        let has_content = run_id.chars().any(|c| c.is_ascii_alphanumeric());
        prop_assume!(has_content);

        let result = build_zjj_workspace_name(&run_id, &stage, attempt);

        // Property: Should succeed with valid inputs that have content
        prop_assert!(result.is_ok());

        let name = result.unwrap();

        // Property: Not empty
        prop_assert!(!name.is_empty());

        // Property: Contains only valid chars
        prop_assert!(
            name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "Workspace name contains invalid chars: {}",
            name
        );

        // Property: Within length limit
        prop_assert!(
            name.len() <= 64,
            "Workspace name too long: {} chars",
            name.len()
        );
    }
}

// Property: Empty/malformed inputs should fail gracefully
proptest! {
    #[test]
    fn prop_invalid_workspace_inputs_fail(
        run_id in "(|[\x00-\x1f]{1,10})",  // Empty or control chars
        stage in "",                       // Empty
        attempt in 0u32..1,               // Zero (invalid)
    ) {
        let result = build_zjj_workspace_name(&run_id, &stage, attempt);
        // Property: Should fail for invalid inputs
        prop_assert!(result.is_err());
    }
}

// Property: SSE parsing doesn't panic on any input
proptest! {
    #[test]
    fn prop_sse_parsing_no_panic(
        input in "(.*\n){0,100}",
        max_events in 1usize..1000,
    ) {
        // Property: Should never panic
        let _ = parse_opencode_sse_events(&input, max_events);
    }
}

// Property: Stage ordering is consistent
proptest! {
    #[test]
    fn prop_stage_ordering(
        stage in prop::sample::select(vec![
            StageName::Plan,
            StageName::Contract,
            StageName::Tdd15,
            StageName::Qa,
            StageName::RedQueen,
            StageName::GptReview,
            StageName::ShipGate,
        ]),
    ) {
        // Property: Each stage has correct next stage
        match stage {
            StageName::Plan => assert_eq!(stage.next(), Some(StageName::Contract)),
            StageName::Contract => assert_eq!(stage.next(), Some(StageName::Tdd15)),
            StageName::Tdd15 => assert_eq!(stage.next(), Some(StageName::Qa)),
            StageName::Qa => assert_eq!(stage.next(), Some(StageName::RedQueen)),
            StageName::RedQueen => assert_eq!(stage.next(), Some(StageName::GptReview)),
            StageName::GptReview => assert_eq!(stage.next(), Some(StageName::ShipGate)),
            StageName::ShipGate => assert_eq!(stage.next(), None),
        }

        // Property: All stages have valid string representation
        let s = stage.as_str();
        prop_assert!(!s.is_empty());
        prop_assert!(!s.contains(' '));
    }
}

// Property: Max attempts is always 2
proptest! {
    #[test]
    fn prop_max_attempts_is_two(
        stage in prop::sample::select(vec![
            StageName::Plan,
            StageName::Contract,
            StageName::Tdd15,
            StageName::Qa,
            StageName::RedQueen,
            StageName::GptReview,
            StageName::ShipGate,
        ]),
    ) {
        // Property: Every stage allows exactly 2 attempts
        prop_assert_eq!(stage.max_attempts(), 2);
    }
}

// Property: Gates exist for each stage
proptest! {
    #[test]
    fn prop_stages_have_gates(
        stage in prop::sample::select(vec![
            StageName::Plan,
            StageName::Contract,
            StageName::Tdd15,
            StageName::Qa,
            StageName::RedQueen,
            StageName::GptReview,
            StageName::ShipGate,
        ]),
    ) {
        let gates = stage.gates();

        // Property: Every stage has at least one gate
        prop_assert!(!gates.is_empty(), "Stage {:?} has no gates", stage);

        // Property: ShipGate has exactly 2 gates (moon CI, zjj merge)
        if stage == StageName::ShipGate {
            prop_assert_eq!(gates.len(), 2);
        }
    }
}

/// Property: Circuit breaker state transitions are valid
#[test]
fn test_circuit_breaker_properties() {
    use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};

    let config = CircuitConfig::new(5, 3, 60_000);
    let breaker = CircuitBreaker::new("test", config);

    // Property: New breaker starts Closed
    assert_eq!(breaker.state, CircuitState::Closed);
    assert!(breaker.state.allows_operations());

    // Property: After threshold failures, should open
    let breaker = (0..5).fold(breaker, |b, _| b.record_failure());
    assert_eq!(breaker.state, CircuitState::Open);
    assert!(!breaker.state.allows_operations());
}

/// Property: Health metrics calculation
#[test]
fn test_health_metrics_properties() {
    use oya::types::HealthMetrics;

    // Property: Empty metrics have 100% success rate
    let empty = HealthMetrics::new(0, 0, 0, 0);
    assert_eq!(empty.success_rate(), 100);

    // Property: All success = 100%
    let all_success = HealthMetrics::new(10, 10, 0, 0);
    assert_eq!(all_success.success_rate(), 100);

    // Property: All failure = 0%
    let all_fail = HealthMetrics::new(10, 0, 10, 0);
    assert_eq!(all_fail.success_rate(), 0);

    // Property: Mixed is calculated correctly
    let mixed = HealthMetrics::new(10, 5, 5, 0);
    assert_eq!(mixed.success_rate(), 50);
}
