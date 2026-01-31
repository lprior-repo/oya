# Moon CICD Tools - Complete Indexed Reference

**Purpose**: Centralized, machine-readable catalog of all moon CICD tasks and pipelines from the project configuration.

**Source**: `/home/lewis/src/zjj/moon.yml` + `/home/lewis/src/zjj/docs/02_MOON_BUILD.md`

**Last Updated**: 2026-01-08

---

## Quick Navigation

- **[Individual Tasks](#individual-tasks)** - All 17 single tasks
- **[Composite Pipelines](#composite-pipelines)** - 4 orchestration pipelines
- **[Convenience Commands](#convenience-commands)** - 2 utility commands
- **[Stage Breakdown](#stage-breakdown)** - 7 quality gate stages
- **[Task Dependencies](#task-dependencies)** - Execution graph
- **[Performance Metrics](#performance-metrics)** - Timing data

---

## Individual Tasks

### STAGE 1: Code Formatting & Linting (Fast ~10-15s)

#### `fmt`
- **Command**: `cargo fmt --all --check`
- **Description**: Check code formatting (rustfmt)
- **Cache**: Disabled
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
  - `rustfmt.toml`
- **Outputs**: None
- **Duration**: ~2-3s
- **Auto-fixable**: Yes (use `fmt-fix`)

#### `fmt-fix`
- **Command**: `cargo fmt --all`
- **Description**: Auto-fix code formatting
- **Cache**: Disabled
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
- **Outputs**: None (modifies files in-place)
- **Duration**: ~2-3s
- **Use Case**: Run before committing to auto-correct formatting

#### `clippy`
- **Command**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **Description**: Lint with Clippy (strict mode)
- **Cache**: Enabled
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
  - `.clippy.toml`
- **Outputs**:
  - `target/`
- **Duration**: ~5-10s (cached)
- **Dependencies**: `fmt`
- **Strictness**: `-D warnings` (denies all warnings)

#### `lint`
- **Command**: `cargo doc --no-deps --document-private-items 2>&1 | grep -E '(warning|error)' || true`
- **Description**: Check documentation completeness
- **Cache**: Enabled
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
- **Outputs**: None
- **Duration**: ~5-8s

---

### STAGE 2: Unit & Property-Based Tests (30-45s)

#### `test`
- **Command**: `cargo test --workspace --all-features`
- **Description**: Run all unit tests
- **Cache**: Enabled
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
  - `tests/**/*`
- **Outputs**:
  - `target/test-results.json`
- **Duration**: ~20-30s
- **Dependencies**: `fmt`, `clippy`
- **Parallel Execution**: Yes (via nextest)

#### `test-doc`
- **Command**: `cargo test --doc --workspace --all-features`
- **Description**: Run documentation tests
- **Cache**: Enabled
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
- **Outputs**: None
- **Duration**: ~3-5s
- **Dependencies**: `fmt`

#### `test-properties`
- **Command**: `cargo test --test '*' --features proptest --workspace --all-features -- --test-threads 1`
- **Description**: Run property-based tests exhaustively
- **Cache**: Enabled
- **Inputs**:
  - `src/**/*.rs`
  - `tests/**/*.rs`
  - `Cargo.toml`
- **Outputs**:
  - `target/proptest-results.json`
- **Duration**: ~30-45s
- **Dependencies**: `test`
- **Environment**: `PROPTEST_CASES=10000`
- **Details**: Generates 10,000 randomized test cases per property

---

### STAGE 3: Mutation Testing (2-5 minutes)

#### `mutants`
- **Command**: `sh .moon/scripts/mutation-test.sh`
- **Description**: Run mutation testing to verify test quality
- **Cache**: Enabled
- **Inputs**:
  - `src/**/*.rs`
  - `tests/**/*.rs`
  - `Cargo.toml`
- **Outputs**:
  - `target/mutants.json`
  - `.mutations-report/`
- **Duration**: ~2-5 minutes
- **Dependencies**: `test`
- **Purpose**: Verifies test suite can catch code mutations
- **Failure Indicator**: Tests are insufficient in coverage

---

### STAGE 4: LLM-as-Judge Code Review (30-60s)

#### `llm-judge`
- **Command**: `python3 .moon/scripts/llm-judge.py`
- **Description**: LLM code review (Claude as judge)
- **Cache**: Disabled (always runs)
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
- **Outputs**:
  - `.llm-review-report.json`
  - `.llm-review-report.md`
- **Duration**: ~30-60s
- **Dependencies**: `test`
- **Evaluation Criteria**:
  - Design patterns and anti-patterns
  - Error handling quality
  - Functional programming idioms
  - Type safety and generics usage
  - Performance considerations
  - Security concerns

#### `llm-judge-fix-suggestions`
- **Command**: `python3 .moon/scripts/llm-judge.py --suggest-fixes`
- **Description**: Generate LLM-based improvement suggestions
- **Cache**: Disabled (always runs)
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
- **Outputs**:
  - `.llm-suggestions.json`
  - `.llm-suggestions.md`
- **Duration**: ~30-60s
- **Dependencies**: None (independent)
- **Purpose**: Provides actionable improvement recommendations

---

### STAGE 5: Security & Dependency Checks (15-30s)

#### `audit`
- **Command**: `cargo audit --deny warnings`
- **Description**: Security audit of dependencies
- **Cache**: Enabled
- **Inputs**:
  - `Cargo.lock`
  - `Cargo.toml`
- **Outputs**: None
- **Duration**: ~5-15s
- **Purpose**: Detect vulnerable dependencies
- **Strictness**: `--deny warnings` (fails on any advisory)

#### `deps-check`
- **Command**: `cargo tree --duplicates`
- **Description**: Check for duplicate dependencies
- **Cache**: Enabled
- **Inputs**:
  - `Cargo.lock`
  - `Cargo.toml`
- **Outputs**: None
- **Duration**: ~2-5s
- **Purpose**: Identify dependency bloat

---

### STAGE 6: Build & Artifacts (45-90s)

#### `build`
- **Command**: `cargo build --release --workspace --all-features`
- **Description**: Build release binaries
- **Cache**: Enabled
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
  - `Cargo.lock`
- **Outputs**:
  - `target/release`
  - `bin/`
- **Duration**: ~60-90s (first run), ~30-45s (cached)
- **Dependencies**: `clippy`, `test`
- **Optimization**: Level 3, LTO enabled, debug info stripped

#### `build-docs`
- **Command**: `cargo doc --release --no-deps --document-private-items --all-features`
- **Description**: Generate Rust documentation
- **Cache**: Enabled
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
- **Outputs**:
  - `target/doc`
- **Duration**: ~20-30s
- **Dependencies**: `clippy`
- **Includes**: Private items (for comprehensive docs)

---

### STAGE 7: Continuous Deployment Gates

#### `cd-gates`
- **Command**: `sh .moon/scripts/cd-gates.sh`
- **Description**: Verify CD readiness (deployment prerequisites)
- **Cache**: Disabled (always runs)
- **Inputs**:
  - `src/**/*.rs`
  - `Cargo.toml`
- **Outputs**:
  - `.cd-gates-report.json`
  - `.cd-gates-report.md`
- **Duration**: ~5-15s
- **Dependencies**: `test`, `build`, `llm-judge`
- **Checks**:
  - All tests passed
  - Build successful
  - LLM review approved
  - Documentation complete

---

## Composite Pipelines

### `quick` - Fast Local Check
- **Command**: No-op (orchestrator only)
- **Description**: Fast lint check (format + clippy)
- **Duration**: ~10-15 seconds
- **Use Case**: Before committing or pushing
- **Dependencies**:
  - `fmt`
  - `clippy`
- **What It Runs**:
  1. Code formatting check
  2. Lint checks (strict mode)

### `quality` - Comprehensive Quality Gates
- **Command**: No-op (orchestrator only)
- **Description**: All quality gates (no build)
- **Duration**: ~60 seconds
- **Use Case**: When you want full validation without building
- **Dependencies**:
  - `fmt`
  - `clippy`
  - `lint`
  - `test`
  - `test-doc`
  - `audit`
  - `deps-check`
  - `llm-judge`
- **What It Runs**:
  1. Code formatting check
  2. Lint checks
  3. Documentation completeness
  4. Unit tests
  5. Documentation tests
  6. Security audit
  7. Dependency checking
  8. LLM code review

### `ci` - Full CI Pipeline
- **Command**: No-op (orchestrator only)
- **Description**: Complete CI pipeline (lint, test, build, quality)
- **Duration**: ~2-3 minutes
- **Use Case**: Before pushing to main branch
- **Dependencies**:
  - `quick`
  - `test`
  - `test-properties`
  - `mutants`
  - `build`
  - `build-docs`
  - `audit`
  - `llm-judge`
  - `cd-gates`
- **What It Runs** (in parallel where possible):
  1. Quick check (fmt + clippy)
  2. Unit tests
  3. Property-based tests (10,000 cases)
  4. Mutation testing
  5. Release build
  6. Documentation generation
  7. Security audit
  8. LLM code review
  9. Deployment readiness gates

### `deploy` - Full Pipeline with Deployment
- **Command**: No-op (orchestrator only)
- **Description**: Full CI pipeline + deployment readiness checks
- **Duration**: ~2-3 minutes
- **Use Case**: Full pre-deployment validation
- **Dependencies**:
  - `ci`
  - `cd-gates`
- **What It Runs**:
  1. All `ci` tasks (9 stages)
  2. CD gates verification

---

## Convenience Commands

### `clean`
- **Command**: `cargo clean && rm -rf .llm-*.json .llm-*.md .cd-gates-* .mutations-report`
- **Description**: Clean build artifacts and reports
- **Cache**: Disabled
- **Use Case**: When you need a fresh build or want to clear report files
- **Removes**:
  - All compiled artifacts (`target/`)
  - LLM review reports
  - CD gates reports
  - Mutation testing reports

### `logs`
- **Command**: `sh .moon/scripts/show-reports.sh`
- **Description**: Display quality reports from last run
- **Cache**: Disabled
- **Use Case**: Review reports without re-running tasks
- **Shows**:
  - LLM review results
  - CD gates verification
  - Mutation testing results

---

## Stage Breakdown

The moon CICD pipeline consists of 7 distinct stages, each with specific purposes:

### Stage 1: Code Formatting & Linting
**Purpose**: Enforce code style and basic linting
- **Tasks**: `fmt`, `fmt-fix`, `clippy`, `lint`
- **Duration**: ~10-15s
- **Fail Behavior**: Blocks further stages
- **Automation**: `fmt-fix` can auto-correct formatting

### Stage 2: Unit & Property-Based Testing
**Purpose**: Verify code correctness with comprehensive coverage
- **Tasks**: `test`, `test-doc`, `test-properties`
- **Duration**: ~30-45s
- **Fail Behavior**: Blocks further stages
- **Coverage**: Unit + docs + property-based (10,000 cases)

### Stage 3: Mutation Testing
**Purpose**: Verify test suite quality by introducing mutations
- **Tasks**: `mutants`
- **Duration**: ~2-5 minutes
- **Fail Behavior**: Tests insufficient for production use
- **Philosophy**: Tests must catch bugs, not just pass

### Stage 4: LLM-as-Judge Code Review
**Purpose**: Architectural and design pattern validation
- **Tasks**: `llm-judge`, `llm-judge-fix-suggestions`
- **Duration**: ~30-60s
- **Checks**: Design, error handling, FP idioms, security
- **Fail Behavior**: Must refactor before merge

### Stage 5: Security & Dependency Checks
**Purpose**: Identify vulnerabilities and bloat
- **Tasks**: `audit`, `deps-check`
- **Duration**: ~15-30s
- **Fail Behavior**: Blocks deployment
- **Strictness**: Zero vulnerabilities allowed

### Stage 6: Build & Artifacts
**Purpose**: Create production binaries and documentation
- **Tasks**: `build`, `build-docs`
- **Duration**: ~45-90s
- **Outputs**: Release binaries, API docs
- **Dependencies**: Must pass all prior stages

### Stage 7: Continuous Deployment Gates
**Purpose**: Final verification before deployment
- **Tasks**: `cd-gates`
- **Duration**: ~5-15s
- **Checks**: Deployment prerequisites
- **Dependencies**: All stages must pass

---

## Task Dependencies

### Dependency Graph

```
fmt
  ├── clippy
  │   ├── build
  │   └── build-docs
  └── test
      ├── test-doc
      ├── test-properties
      ├── mutants
      ├── llm-judge
      └── cd-gates

audit (independent)
deps-check (independent)
llm-judge-fix-suggestions (independent)
```

### Execution Order (Simplified)

1. **Parallel Stage 1**: `fmt`, `fmt-fix` (if needed)
2. **Parallel Stage 2**: `clippy`, `lint`
3. **Parallel Stage 3**: `test`, `test-doc`, `audit`, `deps-check`
4. **Parallel Stage 4**: `test-properties`, `llm-judge`
5. **Sequential Stage 5**: `mutants` (requires `test`)
6. **Sequential Stage 6**: `build`, `build-docs` (requires `clippy`)
7. **Sequential Stage 7**: `cd-gates` (requires all prior)

### Critical Path

The longest dependency chain determines minimum pipeline time:

```
fmt → clippy → build → cd-gates (~90s)
   ↓
   test → mutants → cd-gates (~80s)
```

**Bottleneck**: Mutation testing (2-5 minutes) is the actual bottleneck in CI.

---

## Performance Metrics

### Typical Execution Times

| Command | First Run | Cached | Bottleneck |
|---------|-----------|--------|------------|
| `:quick` | 15s | 5s | rustfmt + clippy |
| `:test` | 45s | 25s | Unit tests + properties |
| `:build` | 90s | 45s | Cargo compilation |
| `:ci` | 2-3 min | 1-2 min | Mutation testing |
| `:deploy` | 2-3 min | 1-2 min | Mutation testing |

### Optimization Strategies

1. **Minimal file changes**: Only recompile affected modules
2. **Use `:quick`**: For frequent local checks
3. **Use `:test`**: Before `:build` for fast feedback
4. **Leverage caching**: Let moon cache results
5. **Parallel execution**: Moon automatically parallelizes independent tasks

### Caching Behavior

- **Cached tasks**: Re-use outputs if inputs haven't changed
- **Uncached tasks**: Always re-run (`fmt`, `llm-judge`, `cd-gates`)
- **Input tracking**: Changes to `src/**/*.rs` invalidate most caches
- **Cache location**: `~/.moon/cache` by default

---

## Usage Rules

### The Golden Rule

```bash
✅ ALWAYS use moon run
❌ NEVER use cargo directly
```

### Correct Usage

```bash
# Before committing
moon run :quick       # ~10-15s

# Before pushing
moon run :ci          # ~2-3 min

# For review suggestions
moon run :llm-judge-fix-suggestions

# For deployment
moon run :deploy      # ~2-3 min
```

### Incorrect Usage

```bash
❌ cargo fmt
❌ cargo clippy
❌ cargo test
❌ cargo build
```

---

## Configuration Files

### `/home/lewis/src/zjj/moon.yml`
- **Purpose**: All task definitions
- **Lines**: 355
- **Structure**: 17 individual tasks + 4 composite pipelines + 2 utilities

### `/home/lewis/src/zjj/.moon/workspace.yml`
- **Purpose**: Moon workspace configuration
- **Version**: 1.20
- **Templates**: `.moon/templates`

### `/home/lewis/src/zjj/.moon/toolchain.yml`
- **Purpose**: Rust toolchain specification
- **Version**: `nightly`

### `/home/lewis/src/zjj/rustfmt.toml`
- **Purpose**: Code formatting rules
- **Referenced**: In `fmt` and `fmt-fix` tasks

### `/home/lewis/src/zjj/.clippy.toml`
- **Purpose**: Linting configuration
- **Referenced**: In `clippy` task

---

## Key Concepts

### LLM-as-Judge Pattern

Moon integrates Claude as an architectural reviewer. Instead of just checking syntax, it evaluates:
- Design patterns (correct? idiomatic?)
- Error handling (comprehensive?)
- Functional programming idioms (following library conventions?)
- Type safety (generics used properly?)
- Performance implications (efficient?)
- Security concerns (safe?)

If Claude review fails, code must be refactored before merge.

### Mutation Testing

Mutation testing inserts bugs into code and checks if tests catch them. If mutations aren't killed, tests are insufficient.

Example: Change `if x > 0` to `if x >= 0`. If tests still pass, they don't properly validate that condition.

### Property-Based Testing

Generates 10,000 randomized test cases per property to find edge cases that example-based tests miss.

Example: Sort function tested with 10,000 random arrays instead of hand-written cases.

### Continuous Deployment (ThoughtWorks)

1. Automated quality gates run on every change
2. All gates must pass
3. CD gates verify deployment prerequisites
4. Manual approval (if configured)
5. Automated deployment

---

## Related Documentation

- **[02_MOON_BUILD.md](02_MOON_BUILD.md)** - Building & Testing with Moon (user guide)
- **[03_WORKFLOW.md](03_WORKFLOW.md)** - Daily Workflow (how to use moon in practice)
- **[07_TESTING.md](07_TESTING.md)** - Testing Patterns & Strategies
- **[moon.yml](../moon.yml)** - Raw configuration file

---

## Index Tags

**Keywords**: moon, CICD, CI/CD, build, test, mutation, property-based, LLM-judge, deployment, caching, linting, formatting

**Tasks**: fmt, fmt-fix, clippy, lint, test, test-doc, test-properties, mutants, llm-judge, llm-judge-fix-suggestions, audit, deps-check, build, build-docs, cd-gates

**Pipelines**: quick, quality, ci, deploy

**Utilities**: clean, logs

**Stages**: formatting, linting, testing, mutation, LLM-review, security, build, deployment

**Concepts**: LLM-as-judge, mutation-testing, property-based-testing, continuous-deployment, caching, parallelization
