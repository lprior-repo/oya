#!/bin/bash
# BRUTAL QA TEST FOR ZJJ SESSION MANAGEMENT
# QA Agent #2 - Testing: list, status, remove, rename, focus

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TEST_DIR="/tmp/zjj_brutal_test_$$"
PASS_COUNT=0
FAIL_COUNT=0

log() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"
}

pass() {
    echo -e "${GREEN}✓ PASS${NC} $1"
    ((PASS_COUNT++))
}

fail() {
    echo -e "${RED}✗ FAIL${NC} $1"
    ((FAIL_COUNT++))
}

warn() {
    echo -e "${YELLOW}⚠ WARN${NC} $1"
}

run_cmd() {
    local cmd="$1"
    local should_fail="${2:-false}"
    log "Running: $cmd"

    if eval "$cmd" 2>&1; then
        local exit_code=$?
        if [ "$should_fail" = "true" ]; then
            fail "Command should have failed but succeeded: $cmd"
            return 1
        else
            pass "Command succeeded: $cmd"
            return 0
        fi
    else
        local exit_code=$?
        if [ "$should_fail" = "true" ]; then
            pass "Command failed as expected: $cmd (exit $exit_code)"
            return 0
        else
            fail "Command failed unexpectedly: $cmd (exit $exit_code)"
            return 1
        fi
    fi
}

# Setup
setup() {
    log "Setting up test environment in $TEST_DIR"
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"

    # Initialize zjj (it will create JJ repo if needed)
    run_cmd "zjj init" || true

    # Clear any existing sessions
    zjj list 2>/dev/null | grep -oP '\[\K[^\]]+' | xargs -I {} zjj remove -y {} 2>/dev/null || true
}

cleanup() {
    log "Cleaning up test directory"
    cd /home/lewis/src/oya
    rm -rf "$TEST_DIR"
}

# Test 1: List with 0 sessions
test_list_zero_sessions() {
    log "\n=== TEST 1: List with 0 sessions ==="
    local output
    output=$(run_cmd "zjj list")
    log "Output: $output"
}

# Test 2: Create and list 1 session
test_list_one_session() {
    log "\n=== TEST 2: List with 1 session ==="
    run_cmd "zjj add test-session-1"
    local output
    output=$(run_cmd "zjj list")
    if echo "$output" | grep -q "test-session-1"; then
        pass "Session found in list"
    else
        fail "Session NOT found in list"
    fi
}

# Test 3: Status of single session
test_status_single() {
    log "\n=== TEST 3: Status of single session ==="
    run_cmd "zjj status test-session-1"
}

# Test 4: Rename session
test_rename_basic() {
    log "\n=== TEST 4: Basic rename ==="
    run_cmd "zjj rename test-session-1 renamed-session-1"

    local output
    output=$(zjj list)
    if echo "$output" | grep -q "renamed-session-1"; then
        pass "Renamed session found"
    else
        fail "Renamed session NOT found"
    fi

    if echo "$output" | grep -q "test-session-1"; then
        fail "Old name still present"
    else
        pass "Old name removed"
    fi
}

# Test 5: Rename to existing name
test_rename_to_existing() {
    log "\n=== TEST 5: Rename to existing name (should fail) ==="
    run_cmd "zjj add test-session-2"
    run_cmd "zjj rename renamed-session-1 test-session-2" "true"
}

# Test 6: Rename non-existent session
test_rename_nonexistent() {
    log "\n=== TEST 6: Rename non-existent session (should fail) ==="
    run_cmd "zjj rename nonexistent newname" "true"
}

# Test 7: Rename with special characters
test_rename_special_chars() {
    log "\n=== TEST 7: Rename with special characters ==="
    run_cmd "zjj add test-session-3"
    run_cmd "zjj rename test-session-3 session-with-dashes"
    run_cmd "zjj rename session-with-dashes session_with_underscores"
    run_cmd "zjj rename session_with_underscores session.with.dots" || true
}

# Test 8: Remove active session
test_remove_active() {
    log "\n=== TEST 8: Remove active session ==="
    run_cmd "zjj add to-be-removed"
    run_cmd "zjj remove -y to-be-removed"

    local output
    output=$(zjj list)
    if echo "$output" | grep -q "to-be-removed"; then
        fail "Removed session still in list"
    else
        pass "Removed session not in list"
    fi
}

# Test 9: Remove non-existent session
test_remove_nonexistent() {
    log "\n=== TEST 9: Remove non-existent session (should fail) ==="
    run_cmd "zjj remove -y nonexistent-session" "true"
}

# Test 10: Create 100 sessions
test_massive_list() {
    log "\n=== TEST 10: Create 100 sessions ==="
    local start_time=$(date +%s)

    for i in {1..100}; do
        zjj add "bulk-session-$i" >/dev/null 2>&1 || warn "Failed to create bulk-session-$i"
    done

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    log "Created 100 sessions in ${duration}s"

    local output
    output=$(zjj list)
    local count
    count=$(echo "$output" | grep -c "bulk-session" || echo "0")
    log "Found $count bulk sessions"
}

# Test 11: Focus commands
test_focus_basic() {
    log "\n=== TEST 11: Focus commands ==="
    run_cmd "zjj focus bulk-session-1" "true"  # Will fail outside Zellij
    run_cmd "zjj focus nonexistent" "true"
}

# Test 12: Status during operations
test_status_during_ops() {
    log "\n=== TEST 12: Status checks ==="
    run_cmd "zjj status bulk-session-1"

    # Try various status flags
    run_cmd "zjj status --all" "false" || run_cmd "zjj status -a" "false" || true
    run_cmd "zjj status --active" "false" || run_cmd "zjj status -A" "false" || true
}

# Test 13: Rapid create/delete
test_rapid_operations() {
    log "\n=== TEST 13: Rapid create/delete cycles ==="

    for i in {1..20}; do
        zjj add "rapid-$i" >/dev/null 2>&1 || true
        zjj remove -y "rapid-$i" >/dev/null 2>&1 || true
    done

    log "Rapid cycles completed"
    run_cmd "zjj list"
}

# Test 14: Unicode names
test_unicode_names() {
    log "\n=== TEST 14: Unicode and special characters ==="
    run_cmd "zjj add 'session-café'" || warn "Failed to create café session"
    run_cmd "zjj add 'session-test-123'" || warn "Failed to create test-123 session"
    run_cmd "zjj add 'session_test'" || warn "Failed to create session_test"

    local output
    output=$(zjj list)
    log "List output after unicode tests"
}

# Test 15: Empty names and edge cases
test_edge_cases() {
    log "\n=== TEST 15: Edge cases ==="
    run_cmd "zjj rename '' newname" "true" || warn "Empty rename failed differently"
    run_cmd "zjj rename bulk-session-1 ''" "true" || warn "Rename to empty failed differently"
    run_cmd "zjj remove -y ''" "true" || warn "Remove empty failed differently"
}

# Test 16: Concurrent operations simulation
test_concurrent_simulation() {
    log "\n=== TEST 16: Simulated concurrent operations ==="

    # Create multiple sessions rapidly
    for i in {1..10}; do
        zjj add "concurrent-a-$i" >/dev/null 2>&1 &
    done
    wait

    # Try to rename them while creating more
    for i in {1..10}; do
        zjj rename "concurrent-a-$i" "concurrent-b-$i" >/dev/null 2>&1 &
        zjj add "concurrent-c-$i" >/dev/null 2>&1 &
    done
    wait

    log "Concurrent simulation completed"
    run_cmd "zjj list"
}

# Test 17: List output formatting
test_list_formatting() {
    log "\n=== TEST 17: List output formatting ==="
    run_cmd "zjj list"
    run_cmd "zjj list --json" "false" || run_cmd "zjj list -j" "false" || warn "JSON list not supported"
    run_cmd "zjj list --verbose" "false" || run_cmd "zjj list -v" "false" || warn "Verbose list not supported"
}

# Test 18: Status with filters
test_status_filters() {
    log "\n=== TEST 18: Status with filters ==="
    run_cmd "zjj status --active" "false" || run_cmd "zjj status -A" "false" || true
    run_cmd "zjj status --inactive" "false" || run_cmd "zjj status -I" "false" || true
}

# Test 19: Rename with same name
test_rename_same_name() {
    log "\n=== TEST 19: Rename to same name ==="
    run_cmd "zjj add test-rename-same"
    run_cmd "zjj rename test-rename-same test-rename-same" "true" || run_cmd "zjj rename test-rename-same test-rename-same" "false"
    run_cmd "zjj remove -y test-rename-same"
}

# Test 20: Cleanup all sessions
test_cleanup_all() {
    log "\n=== TEST 20: Cleanup all sessions ==="

    # Get all sessions and remove them
    zjj list 2>/dev/null | grep -oP '\[\K[^\]]+' | while read -r session; do
        zjj remove -y "$session" 2>/dev/null || warn "Failed to remove $session"
    done

    run_cmd "zjj list"
}

# Main execution
main() {
    log "=== ZJJ BRUTAL QA TEST STARTING ==="
    log "Test directory: $TEST_DIR"

    setup

    test_list_zero_sessions
    test_list_one_session
    test_status_single
    test_rename_basic
    test_rename_to_existing
    test_rename_nonexistent
    test_rename_special_chars
    test_remove_active
    test_remove_nonexistent
    test_massive_list
    test_focus_basic
    test_status_during_ops
    test_rapid_operations
    test_unicode_names
    test_edge_cases
    test_concurrent_simulation
    test_list_formatting
    test_status_filters
    test_rename_same_name
    test_cleanup_all

    cleanup

    log "\n=== TEST SUMMARY ==="
    log "PASSED: $PASS_COUNT"
    log "FAILED: $FAIL_COUNT"
    log "TOTAL: $((PASS_COUNT + FAIL_COUNT))"

    if [ $FAIL_COUNT -eq 0 ]; then
        log -e "${GREEN}ALL TESTS PASSED${NC}"
        exit 0
    else
        log -e "${RED}SOME TESTS FAILED${NC}"
        exit 1
    fi
}

# Run main
main
