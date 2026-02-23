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
            StageName::Explore,
            StageName::Contract,
            StageName::Red,
            StageName::Implementation,
            StageName::Witness,
            StageName::ShipGate,
        ]),
    ) {
        // Property: Each stage has correct next stage
        match stage {
            StageName::Explore => assert_eq!(stage.next(), Some(StageName::Contract)),
            StageName::Contract => assert_eq!(stage.next(), Some(StageName::Red)),
            StageName::Red => assert_eq!(stage.next(), Some(StageName::Implementation)),
            StageName::Implementation => assert_eq!(stage.next(), Some(StageName::Witness)),
            StageName::Witness => assert_eq!(stage.next(), Some(StageName::ShipGate)),
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
            StageName::Explore,
            StageName::Contract,
            StageName::Red,
            StageName::Implementation,
            StageName::Witness,
            StageName::ShipGate,
        ]),
    ) {
        // Property: Every stage allows exactly 2 attempts
        prop_assert_eq!(stage.max_attempts(), 2);
    }
}

// Property: Acceptance tests are RED invariant
proptest! {
    #[test]
    fn prop_acceptance_tests_must_fail_initially(
        _passed in any::<bool>(),
        _output in "\\PC*",
    ) {
        // This is a placeholder for a more complex property test
        // In the real system, the AcceptanceTestsAreRed gate would execute 'moon run :test'
        // and expect it to FAIL (exit code != 0).

        // Invariant: If the stage is AcceptanceTest, the goal is to have passed=false
        // but for the GATE to pass. This logic is handled by the gate runner.
    }
}

// Property: Gates exist for each stage
proptest! {
    #[test]
    fn prop_stages_have_gates(
        stage in prop::sample::select(vec![
            StageName::Explore,
            StageName::Contract,
            StageName::Contract,
            StageName::Red,
            StageName::Implementation,
            StageName::Implementation,
            StageName::Implementation,
            StageName::Implementation,
            StageName::Witness,
            StageName::ShipGate,
            StageName::ShipGate,
            StageName::ShipGate,
        ]),
    ) {
        let gates = stage.gates();

        // Explore intentionally has no gates.
        if stage != StageName::Explore {
            prop_assert!(!gates.is_empty(), "Stage {:?} has no gates", stage);
        }

        // Property: Witness has one stable gate and ShipGate has close gates.
        if stage == StageName::Witness {
            prop_assert_eq!(gates.len(), 1);
        }
        if stage == StageName::ShipGate {
            prop_assert_eq!(gates.len(), 1);
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

// ---------------------------------------------------------------------------
// Circuit Breaker Timeout Properties (src-12y)
// ---------------------------------------------------------------------------

// Property: Circuit stays Open before timeout elapses
// INVARIANT: After opening, circuit MUST remain Open until reset_timeout_ms has passed
proptest! {
    #[test]
    fn prop_circuit_remains_open_before_timeout(
        failure_threshold in 1u32..10u32,
        reset_timeout_ms in 100u64..500u64,
        failures in 1u32..20u32,
        wait_before_check_ms in 0u64..50u64,
    ) {
        use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};
        use std::thread;
        use std::time::Duration;

        // Ensure we actually trip the breaker
        prop_assume!(failures >= failure_threshold);

        let config = CircuitConfig::new(failure_threshold, 3, reset_timeout_ms);
        let breaker = CircuitBreaker::new("test-scope", config);

        // Drive to Open state
        let breaker = (0..failures).fold(breaker, |b, _| b.record_failure());

        // Property: Must be Open after failures exceed threshold
        prop_assert_eq!(breaker.state, CircuitState::Open,
            "Circuit should be Open after {} failures with threshold {}",
            failures, failure_threshold);

        // Wait less than timeout
        if wait_before_check_ms > 0 {
            thread::sleep(Duration::from_millis(wait_before_check_ms));
        }

        let breaker = breaker.try_half_open();

        // INVARIANT: Circuit MUST remain Open when timeout not elapsed
        // We waited wait_before_check_ms which is < reset_timeout_ms (by prop constraints)
        prop_assert_eq!(breaker.state, CircuitState::Open,
            "Circuit should REMAIN Open after {}ms wait (timeout={}ms)",
            wait_before_check_ms, reset_timeout_ms);
    }
}

// Property: Circuit transitions to HalfOpen after timeout elapses
// INVARIANT: After reset_timeout_ms, try_half_open MUST transition Open -> HalfOpen
proptest! {
    #[test]
    fn prop_circuit_half_opens_after_timeout(
        failure_threshold in 1u32..5u32,
        reset_timeout_ms in 50u64..150u64,
        failures in 5u32..15u32,
    ) {
        use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};
        use std::thread;
        use std::time::Duration;

        // Ensure we actually trip the breaker
        prop_assume!(failures >= failure_threshold);

        let config = CircuitConfig::new(failure_threshold, 3, reset_timeout_ms);
        let breaker = CircuitBreaker::new("test-scope", config);

        // Drive to Open state
        let breaker = (0..failures).fold(breaker, |b, _| b.record_failure());
        prop_assert_eq!(breaker.state, CircuitState::Open);

        // Wait for timeout to elapse (with 20ms margin for timing variance)
        let wait_ms = reset_timeout_ms.saturating_add(20);
        thread::sleep(Duration::from_millis(wait_ms));

        let breaker = breaker.try_half_open();

        // INVARIANT: Circuit MUST transition to HalfOpen after timeout
        prop_assert_eq!(breaker.state, CircuitState::HalfOpen,
            "Circuit should transition to HalfOpen after {}ms (timeout was {}ms)",
            wait_ms, reset_timeout_ms);
    }
}

// Property: HalfOpen allows operations and resets on success
// INVARIANT: HalfOpen state allows operations and success_count resets
proptest! {
    #[test]
    fn prop_half_open_allows_operations_and_resets_on_success(
        failure_threshold in 1u32..5u32,
        success_threshold in 1u32..5u32,
        reset_timeout_ms in 50u64..100u64,
    ) {
        use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};
        use std::thread;
        use std::time::Duration;

        let config = CircuitConfig::new(failure_threshold, success_threshold, reset_timeout_ms);
        let breaker = CircuitBreaker::new("test-scope", config);

        // Drive to Open state
        let breaker = (0..failure_threshold).fold(breaker, |b, _| b.record_failure());
        prop_assert_eq!(breaker.state, CircuitState::Open);

        // Wait for timeout
        thread::sleep(Duration::from_millis(reset_timeout_ms.saturating_add(10)));

        let breaker = breaker.try_half_open();
        prop_assert_eq!(breaker.state, CircuitState::HalfOpen);

        // Property: HalfOpen allows operations
        prop_assert!(breaker.state.allows_operations(),
            "HalfOpen state should allow operations");

        // Property: success_count should be reset to 0 on entering HalfOpen
        prop_assert_eq!(breaker.success_count, 0,
            "success_count should be 0 when entering HalfOpen");
    }
}

// Property: HalfOpen returns to Open on any failure
// INVARIANT: In HalfOpen state, ANY failure must immediately return to Open
proptest! {
    #[test]
    fn prop_half_open_returns_to_open_on_failure(
        failure_threshold in 1u32..5u32,
        success_threshold in 2u32..5u32,
        reset_timeout_ms in 50u64..100u64,
    ) {
        use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};
        use std::thread;
        use std::time::Duration;

        let config = CircuitConfig::new(failure_threshold, success_threshold, reset_timeout_ms);
        let breaker = CircuitBreaker::new("test-scope", config);

        // Drive to Open, then to HalfOpen
        let breaker = (0..failure_threshold).fold(breaker, |b, _| b.record_failure());
        thread::sleep(Duration::from_millis(reset_timeout_ms.saturating_add(10)));
        let breaker = breaker.try_half_open();
        prop_assert_eq!(breaker.state, CircuitState::HalfOpen);

        // Record a failure in HalfOpen
        let breaker = breaker.record_failure();

        // INVARIANT: Must immediately return to Open
        prop_assert_eq!(breaker.state, CircuitState::Open,
            "HalfOpen should transition to Open on failure");
    }
}

// Property: HalfOpen closes after success_threshold consecutive successes
// INVARIANT: After success_threshold successes in HalfOpen, circuit closes
proptest! {
    #[test]
    fn prop_half_open_closes_after_success_threshold(
        failure_threshold in 1u32..5u32,
        success_threshold in 2u32..5u32,
        reset_timeout_ms in 50u64..100u64,
    ) {
        use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};
        use std::thread;
        use std::time::Duration;

        let config = CircuitConfig::new(failure_threshold, success_threshold, reset_timeout_ms);
        let breaker = CircuitBreaker::new("test-scope", config);

        // Drive to Open, then to HalfOpen
        let breaker = (0..failure_threshold).fold(breaker, |b, _| b.record_failure());
        thread::sleep(Duration::from_millis(reset_timeout_ms.saturating_add(10)));
        let breaker = breaker.try_half_open();
        prop_assert_eq!(breaker.state, CircuitState::HalfOpen);

        // Record successes up to threshold
        let breaker = (0..success_threshold).fold(breaker, |b, _| b.record_success());

        // INVARIANT: Must close after success_threshold successes
        prop_assert_eq!(breaker.state, CircuitState::Closed,
            "HalfOpen should transition to Closed after {} successes",
            success_threshold);

        // Property: opened_at must be None when Closed
        prop_assert!(breaker.opened_at.is_none(),
            "opened_at should be None when circuit is Closed");
    }
}

// Property: opened_at is set when circuit opens
// INVARIANT: When circuit transitions to Open, opened_at MUST be set
proptest! {
    #[test]
    fn prop_opened_at_set_when_circuit_opens(
        failure_threshold in 1u32..10u32,
    ) {
        use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};

        let config = CircuitConfig::new(failure_threshold, 3, 60_000);
        let breaker = CircuitBreaker::new("test-scope", config);

        // Drive to Open state
        let breaker = (0..failure_threshold).fold(breaker, |b, _| b.record_failure());

        // INVARIANT: opened_at must be set when Open
        prop_assert_eq!(breaker.state, CircuitState::Open);
        prop_assert!(breaker.opened_at.is_some(),
            "opened_at MUST be set when circuit is Open");
    }
}

// Property: Closed circuit has no opened_at
// INVARIANT: When circuit is Closed, opened_at MUST be None
proptest! {
    #[test]
    fn prop_closed_circuit_has_no_opened_at(
        failure_threshold in 1u32..5u32,
        success_threshold in 2u32..5u32,
        reset_timeout_ms in 50u64..100u64,
    ) {
        use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};

        let config = CircuitConfig::new(failure_threshold, success_threshold, reset_timeout_ms);
        let breaker = CircuitBreaker::new("test-scope", config);

        // New breaker is Closed
        prop_assert_eq!(breaker.state, CircuitState::Closed);
        prop_assert!(breaker.opened_at.is_none(),
            "opened_at should be None for new Closed circuit");

        // Success keeps it Closed
        let breaker = breaker.record_success();
        prop_assert_eq!(breaker.state, CircuitState::Closed);
        prop_assert!(breaker.opened_at.is_none());
    }
}

/// Property: Timing precision - HalfOpen occurs within acceptable variance
/// INVARIANT: Transition happens within reset_timeout_ms + tolerance
#[test]
fn test_half_open_timing_precision() {
    use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};
    use std::thread;
    use std::time::{Duration, Instant};

    let reset_timeout_ms = 100u64;
    let config = CircuitConfig::new(3, 2, reset_timeout_ms);
    let breaker = CircuitBreaker::new("timing-test", config);

    // Drive to Open
    let breaker = (0..3).fold(breaker, |b, _| b.record_failure());
    assert_eq!(breaker.state, CircuitState::Open);

    // Measure actual transition time
    let start = Instant::now();
    let mut breaker = breaker;

    // Poll until transition (simulating real usage)
    loop {
        breaker = breaker.try_half_open();
        if breaker.state == CircuitState::HalfOpen {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;

    // INVARIANT: Transition should happen between timeout and timeout + 20% tolerance
    assert!(
        elapsed_ms >= reset_timeout_ms,
        "Transition too early: {}ms < {}ms timeout",
        elapsed_ms,
        reset_timeout_ms
    );
    assert!(
        elapsed_ms <= reset_timeout_ms + 50,
        "Transition too late: {}ms > {}ms timeout + 50ms tolerance",
        elapsed_ms,
        reset_timeout_ms
    );
}

/// Property: No timing drift across multiple cycles
/// INVARIANT: Each open-halfopen cycle respects the timeout
#[test]
fn test_no_timing_drift_across_cycles() {
    use oya::types::{CircuitBreaker, CircuitConfig, CircuitState};
    use std::thread;
    use std::time::{Duration, Instant};

    let reset_timeout_ms = 75u64;
    let config = CircuitConfig::new(2, 1, reset_timeout_ms);

    for cycle in 0..3 {
        let breaker = CircuitBreaker::new(&format!("cycle-{}", cycle), config);

        // Drive to Open
        let breaker = (0..2).fold(breaker, |b, _| b.record_failure());
        assert_eq!(breaker.state, CircuitState::Open);

        // Time the transition
        let start = Instant::now();
        thread::sleep(Duration::from_millis(reset_timeout_ms.saturating_add(10)));
        let breaker = breaker.try_half_open();

        let elapsed_ms = start.elapsed().as_millis() as u64;

        // INVARIANT: Each cycle should respect the same timeout
        assert!(
            elapsed_ms >= reset_timeout_ms,
            "Cycle {} transition too early: {}ms < {}ms",
            cycle,
            elapsed_ms,
            reset_timeout_ms
        );
        assert_eq!(breaker.state, CircuitState::HalfOpen, "Cycle {} should reach HalfOpen", cycle);
    }
}

proptest! {
    #[test]
    fn prop_merge_decision_is_deterministic_for_same_input(
        bead_id in "[a-z0-9_-]{1,16}",
        priority in 0u8..5u8,
        lock_present in any::<bool>(),
    ) {
        let candidate = oya::MergeTrainCandidate::new(bead_id.as_str(), priority, Vec::<&str>::new());
        prop_assume!(candidate.is_ok());
        let candidate = match candidate {
            Ok(value) => value,
            Err(_) => return Ok(()),
        };

        let mut locks = std::collections::HashMap::new();
        if lock_present {
            let token = oya::types::LockToken::try_from("lock-token-1");
            prop_assume!(token.is_ok());
            let token = match token {
                Ok(value) => value,
                Err(_) => return Ok(()),
            };
            locks.insert(candidate.bead_id.clone(), token);
        }

        let once = oya::schedule_merge_train_with_decisions(std::slice::from_ref(&candidate), &locks);
        let twice = oya::schedule_merge_train_with_decisions(std::slice::from_ref(&candidate), &locks);

        prop_assert_eq!(once, twice);
    }
}
