# PLAN: src-3ag - Add --version Flag to CLI

## Summary
Add `--version` flag support to the OYA CLI. Currently `oya --version` returns error exit code 2.

## Current State
- `src/main.rs:797-802` defines `Cli` struct with `#[command(name, about)]`
- Missing `version` attribute in `#[command(...)]` derive macro
- Package version: `0.1.0` (from `Cargo.toml:3`)

## Phase 1: Tests (TEST_AGENT)

### File: `src/main/tests.rs`
Add e2e validation tests for version flag:
- `test_version_flag_outputs_version` - `oya --version` exits 0 with "oya 0.1.0"
- `test_version_short_flag_outputs_version` - `oya -V` exits 0 with version
- `test_version_flag_with_other_args_fails` - version is mutually exclusive
- `test_version_output_format` - verify format matches "oya <version>"

### File: `src/main.rs`
Add inline test in `#[cfg(test)]` block:
- Verify `Cli::parse_from(["oya", "--version"])` behavior

## Phase 2: Implementation (LOGIC_AGENT)

### Task 1: Add version attribute
File: `src/main.rs:798`
```rust
// FROM:
#[command(name = "oya", about = "OYA Orchestrator - AI governance runtime")]
// TO:
#[command(name = "oya", about = "OYA Orchestrator - AI governance runtime", version)]
```

### Task 2: Verify clap derive behavior
- Confirm `--version` and `-V` work correctly
- Confirm version string uses `env!("CARGO_PKG_VERSION")`

## Test Strategy & Quality Gates

### Gate 1: Tests Written (RED)
- All tests compile
- Tests verify version flag behavior
- Tests fail before implementation

### Gate 2: Tests Pass (GREEN)
- `moon run :test` passes all tests
- `moon run :check` passes
- `moon run :ci` passes

### Gate 3: E2E Validation
```bash
./target/debug/oya --version   # Exit 0, output: oya 0.1.0
./target/debug/oya -V          # Exit 0, output: oya 0.1.0
./target/debug/oya --help      # Shows -V, --version in options
```

## Verification Commands
```bash
moon run :test
moon run :check
moon run :ci
cargo run -- --version
```

## Files Modified
- `src/main.rs:798` - add `version` to `#[command(...)]` attribute

## Dependencies
- `clap::Parser` - existing, derives version from `CARGO_PKG_VERSION`

## Test Cases (Detailed)

### Version flag tests:
1. `test_version_flag_exits_zero` - `oya --version` exits with code 0
2. `test_version_short_flag_exits_zero` - `oya -V` exits with code 0
3. `test_version_output_contains_package_name` - output contains "oya"
4. `test_version_output_contains_version_number` - output contains "0.1.0"
5. `test_help_shows_version_option` - `oya --help` shows `-V, --version`

## Risk Assessment
- **Low risk**: Single-line change to existing derive macro
- **No breaking changes**: Adds new flag, doesn't modify existing behavior
- **Clap handles everything**: No custom version logic needed
