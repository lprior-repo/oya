# Test Suite Architecture

## Philosophy: "Test the Tests"

Since we rely heavily on mocks, we need verification that mocks match reality.
This test suite has multiple layers of defense:

## Layers

### Layer 0: Tool Contracts (`tests/contracts/`)
Formal specifications of what external tools return:
- Exit codes for each command
- Output formats (JSON schemas)
- Error conditions
- Version compatibility

### Layer 1: Contract Verification (`tests/contract_verify.rs`)
Tests that verify our mocks match real tool behavior:
```rust
#[test]
#[ignore = "runs real tools"]
fn verify_moon_check_exit_codes() {
    // Run real moon, compare to contract
    // Fails if moon behavior changes
}
```

### Layer 2: State Machine Tests (`tests/state_machine.rs`)
Exhaustive verification of RunState transitions:
- Generate all possible state paths
- Verify each transition is valid
- Check final states match expectations

### Layer 3: Property Tests (`tests/properties.rs`)
Using `proptest` to generate random valid inputs:
- Hundreds of test cases per property
- Finds edge cases humans miss
- Invariants must hold for ALL inputs

### Layer 4: Orchestrator Unit Tests (`tests/orchestrator.rs`)
Traditional unit tests using verified mocks.

### Layer 5: Mutation Testing
Using `cargo-mutants` to verify test quality:
- Introduces bugs automatically
- Tests must catch the bugs
- Measures test effectiveness

## Running

```bash
# Fast unit tests (uses mocks)
moon run :test

# Verify mocks match reality (runs real tools)
cargo test --test contract_verify -- --ignored

# Property tests (1000s of cases)
cargo test --test properties

# Mutation testing (measures test quality)
cargo mutants --baseline run
```

## CI Strategy

1. Every PR: Unit tests + property tests (fast)
2. Daily: Contract verification (catches tool updates)
3. Weekly: Mutation testing (measures test quality)

## Key Files

- `src/testkit/` - Testing infrastructure
- `tests/contracts/` - Tool behavior specs
- `tests/snapshots/` - Golden files for regression
- `.cargo/mutants.toml` - Mutation testing config
