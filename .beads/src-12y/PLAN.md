# PLAN: src-12y - Fix circuit breaker test using 0-second timeout hack

## Status: CLOSED (replan for validation)

## Problem
Test `scenario_circuit_half_opens_after_timeout` uses 0-second timeout to bypass real timeout testing. This gives false confidence that timeout logic works when it's not actually tested.

## Current Code
- `src/types/health.rs:130-142` - `try_half_open()` method checks elapsed time vs `reset_timeout_ms`
- `src/usage/tests.rs:268-291` - Existing circuit breaker tests (state transitions, allows_operations)

## Target Test Location
- `src/types/health.rs` - Add inline test module for circuit breaker timeout behavior

---

## Implementation Steps

### Step 1: Create test module in health.rs
- Add `#[cfg(test)] mod tests` block to `src/types/health.rs`
- Use `std::thread::sleep` and `std::time::Duration` for realistic timeouts

### Step 2: Write timeout transition test
```rust
#[test]
fn test_circuit_half_opens_after_timeout() {
    let config = CircuitConfig::new(1, 1, 50); // 50ms timeout
    let mut cb = CircuitBreaker::new("test", config);
    
    // Open the circuit
    cb = cb.record_failure();
    assert_eq!(cb.state, CircuitState::Open);
    assert!(cb.opened_at.is_some());
    
    // Before timeout: stays Open
    cb = cb.clone().try_half_open();
    assert_eq!(cb.state, CircuitState::Open, "should stay open before timeout");
    
    // Wait for timeout
    std::thread::sleep(std::time::Duration::from_millis(60));
    
    // After timeout: transitions to HalfOpen
    cb = cb.try_half_open();
    assert_eq!(cb.state, CircuitState::HalfOpen, "should be half-open after timeout");
}
```

### Step 3: Add edge case tests
- Test: circuit stays Open if timeout not reached
- Test: HalfOpen resets success_count on transition
- Test: HalfOpen can go back to Open on failure

### Step 4: Run CI validation
```bash
moon run :ci
```

---

## Test Strategy

### Acceptance Criteria (from issue)
1. Test uses realistic timeout (e.g., 50ms)
2. Test actually waits for timeout (thread::sleep)
3. Test verifies circuit stays open before timeout
4. Test verifies circuit transitions after timeout
5. Test is not flaky (50ms is enough margin)

### Quality Gates
- [ ] `moon run :quick` passes
- [ ] `moon run :ci` passes
- [ ] No clippy warnings
- [ ] Test is deterministic (not flaky)

---

## Files Modified
- `src/types/health.rs` - Add test module

## Commands
```bash
# Run tests
moon run :test

# Full CI
moon run :ci
```

---

## Risk Mitigation
- Use 50ms timeout with 60ms sleep to avoid flakiness on slow CI
- Clone circuit breaker before try_half_open to preserve state for "before timeout" check
