# Intent CLI - CI/CD Configuration Analysis

**Date:** January 25, 2026
**Repository:** lprior-repo/intent-cli
**Technology Stack:** Gleam (Erlang/OTP), CUE

---

## Executive Summary

Intent CLI currently has **NO formal CI/CD pipeline** (no GitHub Actions, GitLab CI, or other CI configuration). However, it has excellent local automation with git hooks and comprehensive testing infrastructure. The project is well-positioned to implement a robust CI/CD pipeline with minimal effort.

**Key Findings:**
- ✅ Comprehensive test suite with 80+ test files
- ✅ Automated git hooks via bd (beads) for issue tracking sync
- ✅ Gleam format and test commands available
- ✅ Security scanning with gitleaks
- ❌ No CI/CD pipeline configuration
- ❌ No coverage reporting
- ❌ No automated quality gates

---

## 1. Current CI/CD Configuration

### 1.1 CI/CD Platforms

**Status: NOT CONFIGURED**

- **GitHub Actions:** No `.github/workflows/` directory found
- **GitLab CI:** No `.gitlab-ci.yml` file found
- **Travis CI:** No `.travis.yml` file found
- **CircleCI:** No `.circleci/config.yml` file found
- **Azure Pipelines:** No `azure-pipelines.yml` file found

**Repository:** GitHub (lprior-repo/intent-cli) - prime candidate for GitHub Actions

### 1.2 Build System

**Primary Build Tool:** Gleam 1.14.0

```bash
# Build
gleam build

# Test
gleam test

# Format
gleam format .
gleam format --check  # Verify formatting without changes

# Run CLI
gleam run -- <command> [args]
```

**Configuration:** `gleam.toml` (30 lines)
- Target: Erlang/OTP
- Dependencies: 17 Gleam packages
- Dev Dependencies: gleeunit (test framework)

### 1.3 Version Management

**Tool:** mise (formerly rtx)
- Configuration: `mise.toml`
- Managed tools:
  - `cue` - latest
  - `erlang` - latest
  - `gleam` - latest
- Global tools (via `~/.config/mise/config.toml`):
  - `bd` (beads) - Issue tracking
  - `bv` (beads viewer) - Issue visualization
  - `nu` (nushell) - Shell scripting

---

## 2. Testing Infrastructure

### 2.1 Test Framework

**Primary Framework:** gleeunit (Gleam's built-in testing)

```bash
# Run all tests
gleam test

# Run with output
gleam test --target erlang
```

### 2.2 Test Organization

**Test Directory Structure:**
```
test/
├── intent/                    # Module-specific tests (20+ files)
│   ├── checker/              # Rule checker tests
│   ├── bead_workflow_test.gleam
│   ├── diff_test.gleam
│   ├── errors_test.gleam
│   └── ...
├── run_integration_tests.sh   # CLI integration tests
├── test_cli_comprehensive.sh   # Comprehensive CLI tests
└── test_helpers.gleam         # Test utilities
```

**Test File Count:** 80+ test files
- Many disabled/broken test files (`.broken`, `.skip`, `.disabled` suffixes)
- Active tests: ~40-50 files

### 2.3 Integration Tests

**Script 1:** `test/run_integration_tests.sh` (770 lines)

**Features:**
- Systematically tests all 33 CLI commands
- Exit code validation (per AGENTS.md specification)
- JSON structure validation (AI CLI Ergonomics v1.1)
- Colored output and detailed reports
- Category-based testing:
  - core_spec (7 tests)
  - interview (7 tests)
  - beads (2 tests)
  - history_sessions (2 tests)
  - kirk (7 tests)
  - ai (2 tests)
  - plan (2 tests)
  - phase (10 tests)
  - misc (4 tests)

**Usage:**
```bash
# Run all tests
./test/run_integration_tests.sh

# Run specific category
./test/run_integration_tests.sh --category kirk

# Verbose mode
./test/run_integration_tests.sh --verbose

# Use custom spec file
./test/run_integration_tests.sh --spec-file examples/api.cue
```

**Script 2:** `test/test_cli_comprehensive.sh` (425 lines)

**Features:**
- 8 test suites covering all CLI commands
- JSON validation for all commands
- Detailed reporting with success rates
- Exit with proper codes for CI integration

**Exit Codes (per AGENTS.md):**
- `0` - Success
- `1` - General failure (tests failed, linting warnings)
- `2` - Blocked behaviors
- `3` - Invalid input (file not found, parse error)
- `4` - Usage error (missing args, invalid flags)

### 2.4 Coverage Reporting

**Status: NOT CONFIGURED**

- Gleam does not have built-in coverage reporting
- No `.cover`, `.coverdata`, or `cover.spec` files found
- `.gitignore` contains:
  ```
  *.cover
  cover/
  ```
  (Suggests coverage was planned but not implemented)

**Opportunity:** Consider using `rebar3 cover` (Erlang coverage tool) or explore Gleam ecosystem for coverage solutions.

---

## 3. Quality Gates & Automation

### 3.1 Git Hooks

**Implementation:** bd (beads) - Issue tracking system with automated hooks

**Hook Types Installed:**

#### Pre-Commit Hook (`.git/hooks/pre-commit`)
```bash
#!/bin/sh
# bd-shim v1
# bd-hooks-version: 0.41.0

exec bd hooks run pre-commit "$@"
```

**Purpose:** Flush pending changes to `.beads/issues.jsonl` before commit
- Ensures issue state is always in sync with code state
- Prevents "stranded" issue state

#### Pre-Push Hook (`.git/hooks/pre-push`)
```bash
#!/bin/sh
# bd-shim v1
exec bd hooks run pre-push "$@"
```

**Purpose:** Prevent pushing stale JSONL files
- Validates that `.beads/issues.jsonl` is up-to-date
- Ensures remote repository has consistent issue state

#### Post-Checkout Hook (`.git/hooks/post-checkout`)
```bash
#!/bin/sh
# bd-shim v1
exec bd hooks run post-checkout "$@"
```

**Purpose:** Import JSONL after branch checkout
- Automatically syncs local db from JSONL when switching branches

#### Post-Merge Hook (`.git/hooks/post-merge`)
```bash
#!/bin/sh
# bd-shim v1
exec bd hooks run post-merge "$@"
```

**Purpose:** Import JSONL after pull/merge
- Keeps local issue state synchronized after updates

**Git Attributes:** `.gitattributes`
```
# Use bd merge for beads JSONL files
.beads/issues.jsonl merge=beads
```

### 3.2 Code Formatting

**Tool:** Gleam Format

```bash
# Format all files
gleam format .

# Check formatting (no changes)
gleam format --check

# Format specific files
gleam format src/*.gleam
```

**Status:** Available but not automated
- No pre-commit hook for formatting
- No CI gate for formatting

### 3.3 Linting

**Status:** LIMITED

- Gleam has compiler warnings (type checking, unused variables)
- No external linter configured
- Custom linting via CLI commands:
  - `intent lint <spec.cue>` - Check CUE specs for anti-patterns
  - `intent doctor <spec.cue>` - Prioritized improvements

### 3.4 Security Scanning

**Tool:** gitleaks 8.30.0

**Configuration:** `.gitleaksignore` (42 lines)

**Purpose:** Detect secrets, API keys, credentials in code

**False Positives Configured:**
1. Moon build cache hashes (look like hex API keys)
2. Test JWT tokens (intentionally fake)
3. Example CUE specs with fake API keys
4. Documentation examples

**Status:** Available but not automated
- No CI integration
- Manual execution only: `gitleaks detect`

### 3.5 Dependency Management

**Tool:** Gleam + Hex (Erlang package manager)

**Manifest:** `manifest.toml` (42 lines)
- 17 dependencies
- Automatically updated by Gleam
- No dependency scanning automation

**Opportunity:** Consider adding dependency vulnerability scanning (e.g., `mix hex.audit` for Erlang packages)

---

## 4. Build & Release Pipeline

### 4.1 Build Process

**Current Build Commands:**
```bash
# Build project
gleam build

# Run tests
gleam test

# Run CLI
gleam run -- <command>
```

**Build Artifacts:**
- `build/` - Erlang compilation artifacts
- `_build/` - Additional build files

### 4.2 Release Process

**Status: NOT AUTOMATED**

- No release automation found
- No version management in CI
- Manual release process likely

**Potential Tools:**
- Gleam's built-in release support
- `erlang.mk` or `rebar3` for Erlang releases

---

## 5. Workflow Scripts

### 5.1 Example Workflows

**Location:** `examples/workflows/`

**Available Scripts:**
1. `new-api-spec.sh` - Complete workflow for creating new API specs
2. `analyze-existing.sh` - Analyze existing specifications
3. `improve-quality.sh` - Improve spec quality iteratively
4. `ai-automation.sh` - AI-driven automation

**Example: `new-api-spec.sh`** (213 lines)

**Workflow Steps:**
1. Interactive interview to define API
2. Export to CUE specification
3. Validate spec syntax
4. Quality analysis (score >= 80)
5. Coverage gap detection
6. OWASP Top 10 coverage check
7. Inversion analysis (failure modes)
8. Improvement suggestions
9. Generate work beads
10. Generate AI prompts

**Quality Gates:**
- Quality Score: >= 80 (good), >= 60 (acceptable), < 60 (low)
- Coverage Score: >= 70 (good security coverage)
- Gap Count: 0 (excellent), > 5 (needs iteration)

### 5.2 Utility Scripts

**Location:** `scripts/`

1. `kirk-loop.sh` - KIRK analysis loop (4,616 bytes)
2. `tdd15-parallel-orchestrator.nu` - Nushell orchestrator for parallel test execution (13,876 bytes)

---

## 6. Automation Opportunities

### 6.1 High Priority (Quick Wins)

#### 1. GitHub Actions Pipeline

**Create `.github/workflows/ci.yml`:**

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install mise
        uses: jdx/mise-action@v2
        with:
          version: latest

      - name: Cache mise
        uses: actions/cache@v3
        with:
          path: ~/.local/share/mise
          key: ${{ runner.os }}-mise-${{ hashFiles('**/mise.toml') }}

      - name: Install dependencies
        run: mise install

      - name: Build
        run: gleam build

      - name: Check formatting
        run: gleam format --check

      - name: Run tests
        run: gleam test

      - name: Run integration tests
        run: ./test/run_integration_tests.sh

      - name: Security scan (gitleaks)
        uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GITLEAKS_LICENSE: ${{ secrets.GITLEAKS_LICENSE }}
```

**Estimated Effort:** 1-2 hours
**Impact:** Immediate CI/CD pipeline, catches formatting and test failures

#### 2. Pre-Commit Formatting Hook

**Add to `.git/hooks/pre-commit` or use `husky`/`pre-commit` framework:**

```bash
#!/bin/bash
# Check Gleam formatting
echo "Checking Gleam formatting..."
if ! gleam format --check; then
    echo "❌ Code not formatted. Run: gleam format ."
    exit 1
fi
echo "✓ Code is formatted"
```

**Estimated Effort:** 30 minutes
**Impact:** Ensures consistent formatting before commits

#### 3. Automated Security Scanning

**Add to GitHub Actions:**

```yaml
      - name: Gitleaks security scan
        uses: gitleaks/gitleaks-action@v2
        with:
          config-path: .gitleaks.toml
          verbose: true
```

**Estimated Effort:** 30 minutes
**Impact:** Prevents secrets from being committed

### 6.2 Medium Priority (Enhanced Quality)

#### 4. Test Coverage Reporting

**Investigate options:**
- Erlang's `rebar3 cover` integration
- Explore Gleam coverage tools (community)
- Consider `covertool` for converting Erlang coverage to LCOV

**Example with rebar3:**
```bash
# Create rebar.config for coverage
{plugins, [covertool]}.  # Convert Erlang cover to LCOV
{cover_enabled, true}.
{cover_opts, [verbose]}.
```

**Add to GitHub Actions:**
```yaml
      - name: Generate coverage report
        run: |
          gleam test --cover
          # Convert to LCOV format
          # Upload to Codecov/Coveralls

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
```

**Estimated Effort:** 2-4 hours (research + implementation)
**Impact:** Visibility into test coverage, quality metrics

#### 5. Dependency Vulnerability Scanning

**Options:**
- Use `mix hex.audit` for Hex packages
- GitHub Dependabot
- Snyk

**Add to GitHub Actions:**
```yaml
      - name: Security audit
        run: mix hex.audit
```

**Estimated Effort:** 1 hour
**Impact:** Catch vulnerable dependencies early

#### 6. Linting Automation

**Add pre-commit or CI gate:**

```yaml
      - name: Run Gleam with warnings as errors
        run: gleam build --warnings-as-errors
```

**Estimated Effort:** 30 minutes
**Impact:** Catch code quality issues early

### 6.3 Low Priority (Nice to Have)

#### 7. Automated Release Pipeline

**Create `.github/workflows/release.yml`:**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build release
        run: |
          gleam build
          # Create release artifacts
          # Package as tarball

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            intent-cli-*.tar.gz
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Estimated Effort:** 3-4 hours
**Impact:** Automated releases, consistent versioning

#### 8. Performance Benchmarking

**Add benchmarking test suite:**

```yaml
      - name: Run benchmarks
        run: gleam run --benchmark
```

**Estimated Effort:** 2-3 hours
**Impact:** Track performance over time

#### 9. Documentation Site

**Use:** Gleam Docs, GitHub Pages, or Docusaurus

**Estimated Effort:** 4-8 hours
**Impact:** Better developer experience

---

## 7. Recommendations

### 7.1 Immediate Actions (Week 1)

1. **Implement GitHub Actions CI Pipeline** (Priority: CRITICAL)
   - Build, test, formatting check
   - Run integration tests
   - Security scan with gitleaks

2. **Add Pre-Commit Formatting Hook** (Priority: HIGH)
   - Ensures consistent code style
   - Prevents formatting issues in PRs

3. **Document Testing Strategy** (Priority: MEDIUM)
   - Create `TESTING.md` with test organization
   - Document how to run tests locally
   - Add CI/CD troubleshooting guide

### 7.2 Short-term Actions (Week 2-4)

4. **Implement Coverage Reporting** (Priority: MEDIUM)
   - Research best practices for Gleam/Erlang coverage
   - Set up coverage reporting in CI
   - Set coverage thresholds (e.g., 70%)

5. **Add Dependency Scanning** (Priority: MEDIUM)
   - Configure Dependabot or similar
   - Run security audits in CI

6. **Enable Issue Tracking Sync in CI** (Priority: LOW)
   - Ensure `.beads/issues.jsonl` is always committed
   - Add bd status checks to CI

### 7.3 Long-term Actions (Month 2+)

7. **Automated Release Pipeline** (Priority: LOW)
   - Tag-triggered releases
   - Automated changelog generation
   - Binary distributions

8. **Performance Monitoring** (Priority: LOW)
   - Benchmark suite
   - Performance regression detection

9. **Documentation Automation** (Priority: LOW)
   - Auto-generate API docs
   - Publish to GitHub Pages

---

## 8. CI/CD Best Practices Checklist

### Current Status

| Practice | Status | Notes |
|----------|--------|-------|
| Automated testing | ✅ Available | Gleam test + integration tests |
| CI pipeline | ❌ Missing | No GitHub Actions/GitLab CI |
| Pre-commit hooks | ✅ Partial | bd hooks only, no formatting |
| Code formatting | ⚠️ Available | Not automated |
| Coverage reporting | ❌ Missing | No coverage data |
| Security scanning | ⚠️ Available | Gitleaks not in CI |
| Dependency scanning | ❌ Missing | No automated checks |
| Release automation | ❌ Missing | Manual releases |
| Documentation generation | ❌ Missing | No auto-docs |
| Performance monitoring | ❌ Missing | No benchmarks |

### Target State (3 Months)

| Practice | Target | Implementation |
|----------|--------|----------------|
| Automated testing | ✅ | Already done |
| CI pipeline | ✅ | GitHub Actions with all checks |
| Pre-commit hooks | ✅ | Format + bd hooks |
| Code formatting | ✅ | Automated in CI + pre-commit |
| Coverage reporting | ✅ | Coveralls/Codecov integration |
| Security scanning | ✅ | Gitleaks in CI |
| Dependency scanning | ✅ | Dependabot + audits |
| Release automation | ⚠️ | Basic automation (tag-triggered) |
| Documentation generation | ⚠️ | Gleam docs + GitHub Pages |
| Performance monitoring | ⚠️ | Basic benchmarking |

---

## 9. File Structure Summary

### CI/CD Related Files

```
intent-cli/
├── .github/
│   └── workflows/          # ❌ NOT FOUND (needs creation)
│       ├── ci.yml          # Main CI pipeline
│       ├── release.yml     # Release automation
│       └── security.yml    # Security scanning
├── .git/
│   └── hooks/
│       ├── pre-commit      # ✅ bd shim (issue tracking)
│       ├── pre-push        # ✅ bd shim (stale check)
│       ├── post-checkout   # ✅ bd shim (sync on checkout)
│       └── post-merge      # ✅ bd shim (sync on merge)
├── .gitattributes          # ✅ bd merge driver
├── .gitignore              # ✅ Coverage patterns (prepared)
├── .gitleaksignore         # ✅ False positives config
├── gleam.toml              # ✅ Build config
├── mise.toml               # ✅ Tool versioning
├── manifest.toml           # ✅ Dependency manifest
├── test/
│   ├── run_integration_tests.sh    # ✅ CLI integration tests
│   ├── test_cli_comprehensive.sh  # ✅ Comprehensive tests
│   └── test_helpers.gleam         # ✅ Test utilities
└── examples/
    └── workflows/         # ✅ Example automation scripts
        ├── new-api-spec.sh
        ├── analyze-existing.sh
        ├── improve-quality.sh
        └── ai-automation.sh
```

---

## 10. Commands Reference

### Development

```bash
# Build
gleam build

# Test
gleam test

# Format
gleam format .
gleam format --check

# Run CLI
gleam run -- <command> [args]
```

### Testing

```bash
# Unit tests
gleam test

# Integration tests
./test/run_integration_tests.sh
./test/test_cli_comprehensive.sh

# Specific category
./test/run_integration_tests.sh --category kirk
```

### Quality Gates

```bash
# Format check
gleam format --check

# Security scan
gitleaks detect --source . --config .gitleaks.toml

# Beads issue tracking sync
bd sync
```

### Manual Quality Checks

```bash
# Validate spec
gleam run -- validate api.cue

# Quality analysis
gleam run -- quality api.cue

# Coverage gaps
gleam run -- gaps api.cue

# OWASP coverage
gleam run -- coverage api.cue

# Failure modes
gleam run -- invert api.cue

# Anti-patterns
gleam run -- lint api.cue

# Prioritized improvements
gleam run -- doctor api.cue
```

---

## 11. Next Steps

### For Implementation

1. **Create GitHub repository** (if not already public)
2. **Enable GitHub Actions**
3. **Copy `.github/workflows/ci.yml`** from section 6.1
4. **Test CI pipeline** with a sample push
5. **Iterate and refine** based on failures
6. **Document the process** for team members

### For Team Adoption

1. **Training session** on CI/CD workflow
2. **Update `CONTRIBUTING.md`** with CI/CD guidelines
3. **Create pull request template** with CI checks
4. **Set up branch protection rules** (require CI to pass)
5. **Monitor CI results** and fix flaky tests

---

## Conclusion

Intent CLI has excellent testing infrastructure and local automation but lacks formal CI/CD pipelines. Implementing GitHub Actions is the highest priority and will provide immediate benefits. The project's comprehensive test suite and automated git hooks provide a solid foundation for building a robust CI/CD pipeline.

**Key Strengths:**
- Comprehensive test coverage (80+ test files)
- Well-organized integration tests
- Automated issue tracking sync via bd
- Security scanning tools available
- Multiple workflow automation examples

**Key Gaps:**
- No CI/CD pipeline configuration
- No automated quality gates
- No coverage reporting
- No automated releases
- No dependency scanning

**Quick Wins:**
1. GitHub Actions CI pipeline (1-2 hours)
2. Pre-commit formatting hook (30 minutes)
3. Security scanning in CI (30 minutes)

With minimal effort, Intent CI can have a production-ready CI/CD pipeline that catches issues early and ensures code quality.
