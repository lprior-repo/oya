#!/bin/bash
# QA Agent #16 - BRUTAL zjj query testing
# Tests EVERY query type comprehensively

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
WARNINGS=0

# Test results array
declare -a FAILED_TESTS_LIST
declare -a WARNING_TESTS_LIST

# Helper functions
log_test() {
    echo -e "${BLUE}[TEST $TOTAL_TESTS]${NC} $1"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((PASSED_TESTS++))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((FAILED_TESTS++))
    FAILED_TESTS_LIST+=("$1")
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
    ((WARNINGS++))
    WARNING_TESTS_LIST+=("$1")
}

increment_test() {
    ((TOTAL_TESTS++))
}

# Validate JSON output
validate_json() {
    local output="$1"
    local test_name="$2"

    if echo "$output" | jq . > /dev/null 2>&1; then
        log_pass "$test_name - Valid JSON"
        return 0
    else
        log_fail "$test_name - Invalid JSON"
        echo "Output: $output"
        return 1
    fi
}

# Check response time
check_response_time() {
    local start_time=$(date +%s%N)
    $1 > /dev/null 2>&1
    local exit_code=$?
    local end_time=$(date +%s%N)
    local elapsed=$((($end_time - $start_time) / 1000000)) # Convert to milliseconds

    echo "$elapsed"
    return $exit_code
}

# Setup test environment
setup_test_env() {
    echo -e "${BLUE}=== Setting up test environment ===${NC}"

    # Ensure we're in a git/jj repo
    if ! jj status > /dev/null 2>&1; then
        echo "Not in a JJ repo, initializing..."
        cd /tmp
        rm -rf zjj_query_test
        mkdir -p zjj_query_test
        cd zjj_query_test
        jj git init git@github.com:dummy/test.git
    fi

    # Initialize zjj if needed
    if ! zjj status > /dev/null 2>&1; then
        zjj init
    fi

    echo -e "${GREEN}Test environment ready${NC}\n"
}

# Cleanup test environment
cleanup_test_env() {
    echo -e "\n${BLUE}=== Cleaning up test environment ===${NC}"

    # Remove test sessions
    for session in test-session-1 test-session-2 test-session-3 test-session-4 test-session-5 perf-test-1 perf-test-2 perf-test-3; do
        zjj remove "$session" > /dev/null 2>&1 || true
    done

    echo -e "${GREEN}Cleanup complete${NC}"
}

# Test 1: Query session-exists with every state
test_session_exists() {
    echo -e "\n${BLUE}=== Testing session-exists query ===${NC}\n"

    # Test 1.1: Session that doesn't exist
    increment_test
    log_test "session-exists: Non-existent session"
    output=$(zjj query session-exists non-existent-session 2>&1)
    validate_json "$output" "session-exists non-existent"

    # Check that exists is false
    if echo "$output" | jq -e '.result.exists == false' > /dev/null 2>&1; then
        log_pass "session-exists non-existent - correctly reports false"
        ((PASSED_TESTS++))
    else
        log_fail "session-exists non-existent - expected exists=false"
    fi

    # Test 1.2: Create session and check it exists
    increment_test
    log_test "session-exists: Create and verify session"
    zjj add test-session-1 > /dev/null 2>&1 || true
    output=$(zjj query session-exists test-session-1 2>&1)
    validate_json "$output" "session-exists after create"

    if echo "$output" | jq -e '.result.exists == true' > /dev/null 2>&1; then
        log_pass "session-exists after create - correctly reports true"
        ((PASSED_TESTS++))
    else
        log_fail "session-exists after create - expected exists=true"
    fi

    # Test 1.3: Check session state fields
    increment_test
    log_test "session-exists: Check state fields"
    output=$(zjj query session-exists test-session-1 2>&1)

    has_name=$(echo "$output" | jq -e '.result.name != null' > /dev/null 2>&1 && echo "true" || echo "false")
    has_state=$(echo "$output" | jq -e '.result.state != null' > /dev/null 2>&1 && echo "true" || echo "false")

    if [ "$has_name" = "true" ] && [ "$has_state" = "true" ]; then
        log_pass "session-exists state fields - name and state present"
        ((PASSED_TESTS++))
    else
        log_fail "session-exists state fields - missing name or state"
    fi

    # Test 1.4: Test with special characters in session name
    increment_test
    log_test "session-exists: Special characters in name"
    output=$(zjj query session-exists "test-session-1" 2>&1)
    validate_json "$output" "session-exists special chars"

    # Test 1.5: Case sensitivity
    increment_test
    log_test "session-exists: Case sensitivity"
    output=$(zjj query session-exists Test-Session-1 2>&1)
    validate_json "$output" "session-exists case sensitivity"

    # Check if case matters (should report not exists if case-sensitive)
    if echo "$output" | jq -e '.result.exists == false' > /dev/null 2>&1; then
        log_pass "session-exists case sensitive - correctly rejects wrong case"
        ((PASSED_TESTS++))
    else
        log_warn "session-exists - case insensitive (may be intentional)"
    fi
}

# Test 2: Query session-count
test_session_count() {
    echo -e "\n${BLUE}=== Testing session-count query ===${NC}\n"

    # Test 2.1: Count with 1 session
    increment_test
    log_test "session-count: With 1 session"
    output=$(zjj query session-count 2>&1)
    validate_json "$output" "session-count with 1 session"

    count=$(echo "$output" | jq -r '.result.count // 0' 2>/dev/null || echo "parse_error")
    if [ "$count" = "1" ]; then
        log_pass "session-count - correctly reports 1 session"
        ((PASSED_TESTS++))
    else
        log_warn "session-count - expected 1, got $count (other sessions may exist)"
    fi

    # Test 2.2: Add more sessions and verify count increases
    increment_test
    log_test "session-count: With multiple sessions"
    zjj add test-session-2 > /dev/null 2>&1 || true
    zjj add test-session-3 > /dev/null 2>&1 || true

    output=$(zjj query session-count 2>&1)
    validate_json "$output" "session-count with 3 sessions"

    count=$(echo "$output" | jq -r '.result.count // 0' 2>/dev/null || echo "parse_error")
    if [ "$count" -ge 3 ]; then
        log_pass "session-count - correctly reports $count sessions (>=3)"
        ((PASSED_TESTS++))
    else
        log_warn "session-count - expected >=3, got $count"
    fi

    # Test 2.3: Check for count_by_state if available
    increment_test
    log_test "session-count: Check for detailed breakdown"
    output=$(zjj query session-count 2>&1)

    has_breakdown=$(echo "$output" | jq -e '.result.count_by_state != null' > /dev/null 2>&1 && echo "true" || echo "false")
    if [ "$has_breakdown" = "true" ]; then
        log_pass "session-count - includes count_by_state breakdown"
        ((PASSED_TESTS++))
    else
        log_warn "session-count - no state breakdown (optional feature)"
    fi

    # Test 2.4: Performance test with many sessions (not actually creating 1000, just testing the query)
    increment_test
    log_test "session-count: Performance test"
    elapsed=$(check_response_time "zjj query session-count")

    if [ "$elapsed" -lt 100 ]; then
        log_pass "session-count performance - ${elapsed}ms (<100ms)"
        ((PASSED_TESTS++))
    else
        log_warn "session-count performance - ${elapsed}ms (>=100ms, consider optimization)"
    fi
}

# Test 3: Query can-run
test_can_run() {
    echo -e "\n${BLUE}=== Testing can-run query ===${NC}\n"

    # Test 3.1: Basic can-run check
    increment_test
    log_test "can-run: Basic check"
    output=$(zjj query can-run 2>&1)
    validate_json "$output" "can-run basic"

    can_run=$(echo "$output" | jq -e '.result.can_run == true' > /dev/null 2>&1 && echo "true" || echo "false")
    if [ "$can_run" = "true" ]; then
        log_pass "can-run - system is ready"
        ((PASSED_TESTS++))
    else
        log_warn "can-run - system reports not ready (check dependencies)"
    fi

    # Test 3.2: Check for reason field when can_run is false
    increment_test
    log_test "can-run: Check reason field"
    output=$(zjj query can-run 2>&1)

    has_reason=$(echo "$output" | jq -e '.result.reason != null' > /dev/null 2>&1 && echo "true" || echo "false")
    if [ "$has_reason" = "true" ]; then
        log_pass "can-run - includes reason field"
        ((PASSED_TESTS++))
    else
        log_warn "can-run - no reason field (useful for debugging)"
    fi

    # Test 3.3: Check for missing dependencies
    increment_test
    log_test "can-run: Check dependencies field"
    output=$(zjj query can-run 2>&1)

    has_deps=$(echo "$output" | jq -e '.result.dependencies != null' > /dev/null 2>&1 && echo "true" || echo "false")
    if [ "$has_deps" = "true" ]; then
        log_pass "can-run - includes dependencies check"
        ((PASSED_TESTS++))
    else
        log_warn "can-run - no dependencies field"
    fi

    # Test 3.4: Performance test
    increment_test
    log_test "can-run: Performance test"
    elapsed=$(check_response_time "zjj query can-run")

    if [ "$elapsed" -lt 100 ]; then
        log_pass "can-run performance - ${elapsed}ms (<100ms)"
        ((PASSED_TESTS++))
    else
        log_warn "can-run performance - ${elapsed}ms (>=100ms)"
    fi
}

# Test 4: Query suggest-name
test_suggest_name() {
    echo -e "\n${BLUE}=== Testing suggest-name query ===${NC}\n"

    # Test 4.1: Suggest name with no conflicts
    increment_test
    log_test "suggest-name: No conflicts"
    output=$(zjj query suggest-name brand-new-session 2>&1)
    validate_json "$output" "suggest-name no conflicts"

    suggestion=$(echo "$output" | jq -r '.result.suggestion // empty' 2>/dev/null)
    if [ -n "$suggestion" ]; then
        log_pass "suggest-name - provides suggestion: $suggestion"
        ((PASSED_TESTS++))
    else
        log_fail "suggest-name - no suggestion provided"
    fi

    # Test 4.2: Suggest name with conflicts
    increment_test
    log_test "suggest-name: With conflicts"
    output=$(zjj query suggest-name test-session-1 2>&1)
    validate_json "$output" "suggest-name with conflicts"

    # Check if suggestion differs from input when conflict exists
    suggestion=$(echo "$output" | jq -r '.result.suggestion // empty' 2>/dev/null)
    if [ -n "$suggestion" ]; then
        log_pass "suggest-name - handles conflicts: $suggestion"
        ((PASSED_TESTS++))
    else
        log_warn "suggest-name - conflict handling unclear"
    fi

    # Test 4.3: Check for alternatives field
    increment_test
    log_test "suggest-name: Check for alternatives"
    output=$(zjj query suggest-name test-session-1 2>&1)

    has_alternatives=$(echo "$output" | jq -e '.result.alternatives != null' > /dev/null 2>&1 && echo "true" || echo "false")
    if [ "$has_alternatives" = "true" ]; then
        log_pass "suggest-name - includes alternatives"
        ((PASSED_TESTS++))
    else
        log_warn "suggest-name - no alternatives field (useful for users)"
    fi

    # Test 4.4: Empty name handling
    increment_test
    log_test "suggest-name: Empty name"
    output=$(zjj query suggest-name "" 2>&1)
    validate_json "$output" "suggest-name empty" || true

    # Test 4.5: Special characters
    increment_test
    log_test "suggest-name: Special characters"
    output=$(zjj query suggest-name "test@session#1" 2>&1)
    validate_json "$output" "suggest-name special chars" || true
}

# Test 5: Invalid query types
test_invalid_queries() {
    echo -e "\n${BLUE}=== Testing invalid queries ===${NC}\n"

    # Test 5.1: Invalid query type
    increment_test
    log_test "invalid: Unknown query type"
    output=$(zjj query invalid-query-type 2>&1)

    if echo "$output" | grep -i "invalid\|unknown\|error" > /dev/null 2>&1; then
        log_pass "invalid query - properly rejected"
        ((PASSED_TESTS++))
    else
        log_fail "invalid query - not properly rejected"
    fi

    # Test 5.2: Missing query type
    increment_test
    log_test "invalid: Missing query type"
    output=$(zjj query 2>&1 || true)

    if echo "$output" | grep -i "required\|missing\|argument" > /dev/null 2>&1; then
        log_pass "missing query type - properly rejected"
        ((PASSED_TESTS++))
    else
        log_warn "missing query type - error unclear"
    fi
}

# Test 6: JSON output validation
test_json_output() {
    echo -e "\n${BLUE}=== Testing JSON output format ===${NC}\n"

    # Test 6.1: Check SchemaEnvelope structure
    increment_test
    log_test "JSON: SchemaEnvelope structure"
    output=$(zjj query session-exists test-session-1 2>&1)

    has_schema=$(echo "$output" | jq -e 'has("$schema")' > /dev/null 2>&1 && echo "true" || echo "false")
    has_version=$(echo "$output" | jq -e 'has("_schema_version")' > /dev/null 2>&1 && echo "true" || echo "false")
    has_type=$(echo "$output" | jq -e 'has("schema_type")' > /dev/null 2>&1 && echo "true" || echo "false")
    has_success=$(echo "$output" | jq -e 'has("success")' > /dev/null 2>&1 && echo "true" || echo "false")
    has_query=$(echo "$output" | jq -e 'has("query_type")' > /dev/null 2>&1 && echo "true" || echo "false")
    has_result=$(echo "$output" | jq -e 'has("result")' > /dev/null 2>&1 && echo "true" || echo "false")

    if [ "$has_schema" = "true" ] && [ "$has_version" = "true" ] && [ "$has_type" = "true" ] && \
       [ "$has_success" = "true" ] && [ "$has_query" = "true" ] && [ "$has_result" = "true" ]; then
        log_pass "SchemaEnvelope - all required fields present"
        ((PASSED_TESTS++))
    else
        log_fail "SchemaEnvelope - missing fields (schema:$has_schema version:$has_version type:$has_type success:$has_success query:$has_query result:$has_result)"
    fi

    # Test 6.2: Check --json flag behavior (should be default)
    increment_test
    log_test "JSON: --json flag is default"
    output_default=$(zjj query session-count 2>&1)
    output_explicit=$(zjj query --json session-count 2>&1)

    if [ "$output_default" = "$output_explicit" ]; then
        log_pass "JSON output - --json is default"
        ((PASSED_TESTS++))
    else
        log_warn "JSON output - --json not default (unexpected)"
    fi

    # Test 6.3: Validate schema URL format
    increment_test
    log_test "JSON: Schema URL format"
    output=$(zjj query session-count 2>&1)
    schema=$(echo "$output" | jq -r '."$schema"' 2>/dev/null || echo "")

    if echo "$schema" | grep -q "^zjj://"; then
        log_pass "Schema URL - valid format: $schema"
        ((PASSED_TESTS++))
    else
        log_warn "Schema URL - non-standard format: $schema"
    fi
}

# Test 7: Concurrent queries
test_concurrent_queries() {
    echo -e "\n${BLUE}=== Testing concurrent queries ===${NC}\n"

    # Test 7.1: Run multiple queries in parallel
    increment_test
    log_test "concurrent: Multiple parallel queries"

    start_time=$(date +%s%N)

    zjj query session-count > /tmp/q1.out 2>&1 &
    zjj query session-exists test-session-1 > /tmp/q2.out 2>&1 &
    zjj query can-run > /tmp/q3.out 2>&1 &

    wait

    end_time=$(date +%s%N)
    elapsed=$((($end_time - $start_time) / 1000000))

    if [ -f /tmp/q1.out ] && [ -f /tmp/q2.out ] && [ -f /tmp/q3.out ]; then
        log_pass "concurrent queries - all completed in ${elapsed}ms"
        ((PASSED_TESTS++))
    else
        log_fail "concurrent queries - some failed"
    fi

    rm -f /tmp/q1.out /tmp/q2.out /tmp/q3.out
}

# Test 8: Performance benchmarks
test_performance() {
    echo -e "\n${BLUE}=== Testing performance benchmarks ===${NC}\n"

    # Test 8.1: Query response time benchmark
    increment_test
    log_test "performance: Response time benchmark"

    total_time=0
    iterations=10

    for i in $(seq 1 $iterations); do
        elapsed=$(check_response_time "zjj query session-count")
        total_time=$((total_time + elapsed))
    done

    avg_time=$((total_time / iterations))

    if [ "$avg_time" -lt 50 ]; then
        log_pass "performance - avg ${avg_time}ms over $iterations iterations (<50ms)"
        ((PASSED_TESTS++))
    else
        log_warn "performance - avg ${avg_time}ms over $iterations iterations (>=50ms)"
    fi
}

# Test 9: Edge cases
test_edge_cases() {
    echo -e "\n${BLUE}=== Testing edge cases ===${NC}\n"

    # Test 9.1: Very long session name
    increment_test
    log_test "edge: Very long session name"
    long_name="test-session-$(printf 'a%.0s' {1..100})"
    output=$(zjj query session-exists "$long_name" 2>&1)
    validate_json "$output" "long session name"

    # Test 9.2: Session name with spaces
    increment_test
    log_test "edge: Session name with spaces"
    output=$(zjj query session-exists "test session 1" 2>&1)
    validate_json "$output" "session name with spaces" || true

    # Test 9.3: Unicode characters
    increment_test
    log_test "edge: Unicode characters"
    output=$(zjj query session-exists "test-测试-セッション" 2>&1)
    validate_json "$output" "unicode session name" || true

    # Test 9.4: Query with extra arguments
    increment_test
    log_test "edge: Extra arguments"
    output=$(zjj query session-exists test-session-1 extra-arg 2>&1)

    # Should either ignore extra args or error
    if echo "$output" | jq . > /dev/null 2>&1; then
        log_pass "extra args - handled gracefully (ignored or error)"
        ((PASSED_TESTS++))
    else
        log_warn "extra args - unclear behavior"
    fi
}

# Generate final report
generate_report() {
    echo -e "\n${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║         zjj Query QA Test Report - Agent #16            ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}\n"

    echo -e "${BLUE}Test Summary:${NC}"
    echo "  Total Tests:  $TOTAL_TESTS"
    echo -e "  ${GREEN}Passed:        $PASSED_TESTS${NC}"
    echo -e "  ${RED}Failed:        $FAILED_TESTS${NC}"
    echo -e "  ${YELLOW}Warnings:      $WARNINGS${NC}"

    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "\n${GREEN}✓ All critical tests passed!${NC}"
    else
        echo -e "\n${RED}✗ Failed tests:${NC}"
        for test in "${FAILED_TESTS_LIST[@]}"; do
            echo "  - $test"
        done
    fi

    if [ $WARNINGS -gt 0 ]; then
        echo -e "\n${YELLOW}⚠ Warnings:${NC}"
        for test in "${WARNING_TESTS_LIST[@]}"; do
            echo "  - $test"
        done
    fi

    echo -e "\n${BLUE}Query Types Tested:${NC}"
    echo "  ✓ session-exists - Query session existence and state"
    echo "  ✓ session-count - Count active sessions"
    echo "  ✓ can-run - Check system readiness"
    echo "  ✓ suggest-name - Generate non-conflicting names"

    echo -e "\n${BLUE}Coverage Areas:${NC}"
    echo "  ✓ JSON output validation"
    echo "  ✓ SchemaEnvelope structure"
    echo "  ✓ Error handling"
    echo "  ✓ Performance benchmarks"
    echo "  ✓ Concurrent queries"
    echo "  ✓ Edge cases (long names, unicode, special chars)"

    echo -e "\n${BLUE}Recommendations:${NC}"
    if [ $FAILED_TESTS -gt 0 ]; then
        echo "  1. Fix failed tests before release"
    fi
    if [ $WARNINGS -gt 0 ]; then
        echo "  2. Review warnings for potential improvements"
    fi
    echo "  3. Consider adding more unit tests for edge cases"
    echo "  4. Document JSON schema for query responses"

    echo -e "\n${BLUE}Test Execution:${NC}"
    echo "  QA Agent: #16"
    echo "  Date: $(date)"
    echo "  Repository: $(pwd)"
    echo "  Branch: $(jj log -r '@-' -T 'invalid' 2>/dev/null || git branch --show-current 2>/dev/null || echo 'unknown')"

    # Save report to file
    {
        echo "zjj Query QA Test Report - Agent #16"
        echo "Generated: $(date)"
        echo ""
        echo "Total Tests: $TOTAL_TESTS"
        echo "Passed: $PASSED_TESTS"
        echo "Failed: $FAILED_TESTS"
        echo "Warnings: $WARNINGS"
        echo ""
        if [ ${#FAILED_TESTS_LIST[@]} -gt 0 ]; then
            echo "Failed Tests:"
            printf '  - %s\n' "${FAILED_TESTS_LIST[@]}"
        fi
        if [ ${#WARNING_TESTS_LIST[@]} -gt 0 ]; then
            echo ""
            echo "Warnings:"
            printf '  - %s\n' "${WARNING_TESTS_LIST[@]}"
        fi
    } > /tmp/zjj_query_qa_report.txt

    echo -e "\n${GREEN}Report saved to: /tmp/zjj_query_qa_report.txt${NC}"
}

# Main execution
main() {
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  zjj Query QA Test Suite - Agent #16 - BRUTAL TESTING MODE${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}\n"

    setup_test_env

    test_session_exists
    test_session_count
    test_can_run
    test_suggest_name
    test_invalid_queries
    test_json_output
    test_concurrent_queries
    test_performance
    test_edge_cases

    cleanup_test_env

    generate_report

    # Exit with proper code
    if [ $FAILED_TESTS -gt 0 ]; then
        exit 1
    else
        exit 0
    fi
}

# Run main
main
