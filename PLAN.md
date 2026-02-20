# PLAN: src-3g9 - OYA_OPENCODE_BASE_URL URL Validation

## Summary
Fix bug where `OYA_OPENCODE_BASE_URL` environment variable accepts invalid URLs silently. Add comprehensive validation and test coverage.

## Current State
- `opencode_config()` in `src/runtime_tools/http.rs:113-127` reads env var
- `is_valid_http_url()` in `src/main.rs:603-621` validates URLs
- Validation logic exists but lacks test coverage
- No tests for `opencode_config()` function

## Phase 1: Tests (TEST_AGENT)

### File: `src/runtime_tools/http.rs`
Add tests in existing `#[cfg(test)] mod tests` block for `opencode_config()`:
- Default when env not set
- Accepts valid http/https URLs
- Rejects invalid URLs (no scheme, ftp, credentials, path, query, fragment)
- Handles empty/whitespace strings (falls back to default)
- Trims whitespace from URLs

### File: `src/main/tests.rs`
Add tests for `is_valid_http_url`:
- Accepts http, https, with port
- Rejects empty, no scheme, ftp, credentials, path, query, fragment
- Trims whitespace

## Phase 2: Implementation (LOGIC_AGENT)

### Task 1: Verify Implementation Correctness
- Confirm `is_valid_http_url()` in `src/main.rs:603-621` correctly validates all cases
- Confirm `opencode_config()` in `src/runtime_tools/http.rs:113-127` handles edge cases

### Task 2: No Code Changes Expected
- Current implementation appears correct
- Tests should pass with existing code

## Test Strategy & Quality Gates

### Gate 1: Tests Written (RED)
- All tests compile
- All new tests exist in test modules
- Tests cover all edge cases from bead requirements

### Gate 2: Tests Pass (GREEN)
- `moon run :test` passes all tests
- `moon run :check` passes
- `moon run :ci` passes

### Gate 3: Code Quality
- No `unwrap` or `expect` in new code
- No `panic!`, `todo!`, or `unimplemented!`
- All functions return `Result<T, E>` for fallible operations

## Verification Commands
```bash
moon run :test
moon run :check
moon run :ci
```

## Files Modified
- `src/runtime_tools/http.rs` - add tests in existing test module
- `src/main/tests.rs` - add tests for `is_valid_http_url`

## Dependencies
- `reqwest::Url::parse` - existing, used for URL validation
- `std::env::var` - existing, used for environment variable access

## Test Cases (Detailed)

### `opencode_config()` tests:
1. `test_opencode_config_default_when_env_not_set` - verify default URL
2. `test_opencode_config_accepts_valid_http_url` - http://localhost:8080
3. `test_opencode_config_accepts_valid_https_url` - https://api.example.com
4. `test_opencode_config_rejects_invalid_url` - "not-a-url"
5. `test_opencode_config_rejects_url_with_path` - http://localhost:8080/api
6. `test_opencode_config_rejects_url_with_query` - http://localhost:8080?foo=bar
7. `test_opencode_config_rejects_url_with_fragment` - http://localhost:8080#anchor
8. `test_opencode_config_rejects_url_with_credentials` - http://user:pass@localhost:8080
9. `test_opencode_config_rejects_ftp_scheme` - ftp://localhost
10. `test_opencode_config_empty_string_uses_default` - "" falls back
11. `test_opencode_config_whitespace_uses_default` - "   " falls back
12. `test_opencode_config_trims_url` - "  http://localhost:8080  "

### `is_valid_http_url()` tests:
1. `test_is_valid_http_url_accepts_http`
2. `test_is_valid_http_url_accepts_https`
3. `test_is_valid_http_url_accepts_port`
4. `test_is_valid_http_url_rejects_empty`
5. `test_is_valid_http_url_rejects_no_scheme`
6. `test_is_valid_http_url_rejects_ftp`
7. `test_is_valid_http_url_rejects_credentials`
8. `test_is_valid_http_url_rejects_path`
9. `test_is_valid_http_url_rejects_query`
10. `test_is_valid_http_url_rejects_fragment`
11. `test_is_valid_http_url_trims_whitespace`
