#!/bin/bash
# CORRUPTION AGENT #19 - Focused Corruption Testing Suite for zjj
# Version 2 - Simplified with proper timeouts and process management

set -e

CORRUPTION_REPORT="/home/lewis/src/oya/zjj_corruption_test_v2_results.log"
BACKUP_DIR="/home/lewis/src/oya/.zjj_backup_$(date +%s)"
OYA_DIR="/home/lewis/src/oya"
TIMEOUT_SECONDS=10

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
WARNING_TESTS=0

# Helper functions
log_test_start() {
    local name="$1"
    echo -e "${BLUE}TEST $((TOTAL_TESTS + 1)): $name${NC}"
    echo "" >> "$CORRUPTION_REPORT"
    echo "========================================" >> "$CORRUPTION_REPORT"
    echo "TEST $((TOTAL_TESTS + 1)): $name" >> "$CORRUPTION_REPORT"
    echo "Timestamp: $(date)" >> "$CORRUPTION_REPORT"
    echo "========================================" >> "$CORRUPTION_REPORT"
}

log_pass() {
    local msg="$1"
    echo -e "${GREEN}✓ PASS: $msg${NC}"
    echo "✓ PASS: $msg" >> "$CORRUPTION_REPORT"
    ((PASSED_TESTS++))
    ((TOTAL_TESTS++))
}

log_fail() {
    local msg="$1"
    echo -e "${RED}✗ FAIL: $msg${NC}"
    echo "✗ FAIL: $msg" >> "$CORRUPTION_REPORT"
    ((FAILED_TESTS++))
    ((TOTAL_TESTS++))
}

log_warn() {
    local msg="$1"
    echo -e "${YELLOW}⚠ WARN: $msg${NC}"
    echo "⚠ WARN: $msg" >> "$CORRUPTION_REPORT"
    ((WARNING_TESTS++))
    ((TOTAL_TESTS++))
}

# Run command with timeout
run_safe() {
    local cmd="$1"
    local desc="$2"

    echo "Running: $cmd" >> "$CORRUPTION_REPORT"

    # Use timeout command to prevent hanging
    timeout "$TIMEOUT_SECONDS" bash -c "$cmd" >> "$CORRUPTION_REPORT" 2>&1 || local exit_code=$?

    if [ ${exit_code:-0} -eq 124 ]; then
        echo "[TIMEOUT after ${TIMEOUT_SECONDS}s]" >> "$CORRUPTION_REPORT"
        echo "TIMEOUT"
        return 124
    elif [ ${exit_code:-0} -eq 141 ]; then
        echo "[TERMINATED]" >> "$CORRUPTION_REPORT"
        echo "TERMINATED"
        return 141
    else
        echo "[Exit code: ${exit_code:-0}]" >> "$CORRUPTION_REPORT"
        echo "${exit_code:-0}"
        return ${exit_code:-0}
    fi
}

# Backup and restore
backup_state() {
    echo -e "${BLUE}Creating backup...${NC}"
    mkdir -p "$BACKUP_DIR"

    [ -d "$OYA_DIR/.zjj" ] && cp -r "$OYA_DIR/.zjj" "$BACKUP_DIR/"
    [ -d "$OYA_DIR/.jj" ] && cp -r "$OYA_DIR/.jj" "$BACKUP_DIR/"
    [ -d "$OYA_DIR/oya__workspaces" ] && cp -r "$OYA_DIR/oya__workspaces" "$BACKUP_DIR/"
}

restore_state() {
    echo -e "${BLUE}Restoring from backup...${NC}"
    rm -rf "$OYA_DIR/.zjj"
    [ -d "$BACKUP_DIR/.zjj" ] && cp -r "$BACKUP_DIR/.zjj" "$OYA_DIR/"

    if [ -d "$OYA_DIR/.jj/workspaces" ]; then
        rm -rf "$OYA_DIR/.jj/workspaces"/*
    fi
    [ -d "$BACKUP_DIR/.jj/workspaces" ] && cp -r "$BACKUP_DIR/.jj/workspaces" "$OYA_DIR/.jj/"
}

cleanup_test_artifacts() {
    # Clean up test workspaces
    find "$OYA_DIR/oya__workspaces" -name "corruption-test-*" -type d -exec rm -rf {} + 2>/dev/null || true

    # Clean up test sessions from database
    if [ -f "$OYA_DIR/.zjj/state.db" ]; then
        sqlite3 "$OYA_DIR/.zjj/state.db" "DELETE FROM sessions WHERE name LIKE 'corruption-test-%';" 2>/dev/null || true
        sqlite3 "$OYA_DIR/.zjj/state.db" "DELETE FROM session_locks WHERE session LIKE 'corruption-test-%';" 2>/dev/null || true
    fi
}

check_no_panic() {
    local output="$1"
    if echo "$output" | grep -qi "panic"; then
        return 1
    else
        return 0
    fi
}

check_error_message() {
    local output="$1"
    if echo "$output" | grep -qiE "error:|not found|failed|invalid|corrupt"; then
        return 0
    else
        return 1
    fi
}

# ============================================================================
# CORRUPTION TESTS
# ============================================================================

main() {
    echo "=== ZJJ CORRUPTION TEST SUITE V2 ===" > "$CORRUPTION_REPORT"
    echo "Agent: QA Agent #19 - CORRUPTION AGENT" >> "$CORRUPTION_REPORT"
    echo "Start time: $(date)" >> "$CORRUPTION_REPORT"
    echo "Timeout: ${TIMEOUT_SECONDS}s per command" >> "$CORRUPTION_REPORT"
    echo "" >> "$CORRUPTION_REPORT"

    backup_state

    # Test 1: Corrupt config.toml (invalid TOML syntax)
    log_test_start "CORRUPTION TEST 1: Invalid TOML syntax in config.toml"

    cp "$OYA_DIR/.zjj/config.toml" "$BACKUP_DIR/config.toml.bak"
    echo "corrupt [invalid toml" > "$OYA_DIR/.zjj/config.toml"

    output=$(run_safe "zjj status 2>&1" "config corruption")
    result=$?

    cp "$BACKUP_DIR/config.toml.bak" "$OYA_DIR/.zjj/config.toml"

    if check_no_panic "$output"; then
        if check_error_message "$output"; then
            log_pass "Invalid TOML detected and reported"
        else
            log_warn "Invalid TOML handled but error message unclear"
        fi
    else
        log_fail "Panic detected on invalid TOML"
    fi

    # Test 2: Corrupt config.toml (invalid type)
    log_test_start "CORRUPTION TEST 2: Invalid type in config.toml (boolean as string)"

    cp "$OYA_DIR/.zjj/config.toml" "$BACKUP_DIR/config.toml.bak"
    sed -i 's/use_tabs = true/use_tabs = "not_a_boolean"/' "$OYA_DIR/.zjj/config.toml"

    output=$(run_safe "zjj status 2>&1" "type error in config")
    result=$?

    cp "$BACKUP_DIR/config.toml.bak" "$OYA_DIR/.zjj/config.toml"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qi "parse error"; then
            log_pass "Type error caught with parse error message"
        else
            log_warn "Type error handled but message unclear"
        fi
    else
        log_fail "Panic on type error in config"
    fi

    # Test 3: Delete .zjj directory entirely
    log_test_start "CORRUPTION TEST 3: Delete .zjj directory"

    rm -rf "$OYA_DIR/.zjj"

    output=$(run_safe "zjj status 2>&1" "missing .zjj directory")
    result=$?

    restore_state

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "zjj|not init|no .zjj"; then
            log_pass "Missing .zjj directory detected"
        else
            log_warn "Missing .zjj handled but message unclear"
        fi
    else
        log_fail "Panic when .zjj missing"
    fi

    # Test 4: Corrupt SQLite database (write garbage)
    log_test_start "CORRUPTION TEST 4: Write garbage to state.db"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"
    echo "corrupt database garbage data" > "$OYA_DIR/.zjj/state.db"

    output=$(run_safe "zjj status 2>&1" "corrupted database")
    result=$?

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "database|corrupt|sql"; then
            log_pass "Database corruption detected"
        else
            log_warn "Database corruption handled but message unclear"
        fi
    else
        log_fail "Panic on corrupted database"
    fi

    # Test 5: Truncate SQLite database
    log_test_start "CORRUPTION TEST 5: Truncate state.db to 50% size"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"
    original_size=$(stat -c%s "$OYA_DIR/.zjj/state.db" 2>/dev/null || stat -f%z "$OYA_DIR/.zjj/state.db")
    truncated_size=$((original_size / 2))
    truncate -s "$truncated_size" "$OYA_DIR/.zjj/state.db"

    output=$(run_safe "zjj status 2>&1" "truncated database")
    result=$?

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    if check_no_panic "$output"; then
        log_pass "Truncated database handled without panic"
    else
        log_fail "Panic on truncated database"
    fi

    # Test 6: Create duplicate session name
    log_test_start "CORRUPTION TEST 6: Duplicate session name (UNIQUE constraint)"

    zjj add corruption-test-6 --no-zellij >/dev/null 2>&1 || true
    output=$(run_safe "zjj add corruption-test-6 --no-zellij 2>&1" "duplicate session")
    result=$?

    cleanup_test_artifacts

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "exists|already|duplicate|unique"; then
            log_pass "Duplicate session rejected with clear message"
        else
            log_warn "Duplicate handled but message unclear"
        fi
    else
        log_fail "Panic on duplicate session"
    fi

    # Test 7: Invalid JSON in metadata
    log_test_start "CORRUPTION TEST 7: Invalid JSON in session metadata"

    zjj add corruption-test-7 --no-zellij >/dev/null 2>&1 || true
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET metadata = '{invalid json}' WHERE name = 'corruption-test-7';" 2>/dev/null || true

    output=$(run_safe "zjj status corruption-test-7 2>&1" "invalid JSON")
    result=$?

    cleanup_test_artifacts

    if check_no_panic "$output"; then
        log_pass "Invalid JSON metadata handled gracefully"
    else
        log_fail "Panic on invalid JSON"
    fi

    # Test 8: Invalid state value
    log_test_start "CORRUPTION TEST 8: Invalid state value (CHECK constraint violation)"

    zjj add corruption-test-8 --no-zellij >/dev/null 2>&1 || true
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET state = 'invalid_state' WHERE name = 'corruption-test-8';" 2>/dev/null || true

    output=$(run_safe "zjj status corruption-test-8 2>&1" "invalid state")
    result=$?

    cleanup_test_artifacts

    if check_no_panic "$output"; then
        log_pass "Invalid state handled without panic"
    else
        log_fail "Panic on invalid state"
    fi

    # Test 9: Empty database (no schema)
    log_test_start "CORRUPTION TEST 9: Database without schema tables"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"
    sqlite3 "$OYA_DIR/.zjj/state.db" "DROP TABLE IF EXISTS sessions; DROP TABLE IF EXISTS schema_version;" 2>/dev/null || true

    output=$(run_safe "zjj status 2>&1" "missing schema")
    result=$?

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "schema|table|no such"; then
            log_pass "Missing schema detected"
        else
            log_warn "Missing schema handled but message unclear"
        fi
    else
        log_fail "Panic on missing schema"
    fi

    # Test 10: Read-only config file
    log_test_start "CORRUPTION TEST 10: Read-only config.toml"

    chmod 444 "$OYA_DIR/.zjj/config.toml"

    output=$(run_safe "zjj add corruption-test-10 --no-zellij 2>&1" "read-only config")
    result=$?

    chmod 644 "$OYA_DIR/.zjj/config.toml"

    if check_no_panic "$output"; then
        log_pass "Permission error handled gracefully"
    else
        log_fail "Panic on permission error"
    fi

    cleanup_test_artifacts

    # Test 11: Negative timestamp
    log_test_start "CORRUPTION TEST 11: Negative timestamp in sessions"

    zjj add corruption-test-11 --no-zellij >/dev/null 2>&1 || true
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET created_at = -999999, updated_at = -999999 WHERE name = 'corruption-test-11';" 2>/dev/null || true

    output=$(run_safe "zjj status corruption-test-11 2>&1" "negative timestamp")
    result=$?

    cleanup_test_artifacts

    if check_no_panic "$output"; then
        log_pass "Negative timestamp handled without panic"
    else
        log_fail "Panic on negative timestamp"
    fi

    # Test 12: Very long session name (buffer overflow test)
    log_test_start "CORRUPTION TEST 12: Extremely long session name (1000 chars)"

    long_name=$(printf 'a%.0s' {1..1000})

    output=$(run_safe "zjj add '$long_name' --no-zellij 2>&1" "long name")
    result=$?

    if check_no_panic "$output"; then
        log_pass "Long name handled without crash"
    else
        log_fail "Panic on long name"
    fi

    # Test 13: Non-existent workspace path
    log_test_start "CORRUPTION TEST 13: Point workspace_path to non-existent directory"

    zjj add corruption-test-13 --no-zellij >/dev/null 2>&1 || true
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET workspace_path = '/non/existent/path/corruption-test-13' WHERE name = 'corruption-test-13';" 2>/dev/null || true

    output=$(run_safe "zjj status corruption-test-13 2>&1" "non-existent path")
    result=$?

    cleanup_test_artifacts

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "not found|exist|path"; then
            log_pass "Non-existent path detected"
        else
            log_warn "Non-existent path handled but message unclear"
        fi
    else
        log_fail "Panic on non-existent path"
    fi

    # Test 14: Special characters in workspace name
    log_test_start "CORRUPTION TEST 14: Special characters in workspace name"

    output=$(run_safe "zjj add 'test/workspace' --no-zellij 2>&1" "special chars")
    result=$?

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "invalid|special|character"; then
            log_pass "Special characters rejected"
        else
            log_warn "Special chars handled but message unclear"
        fi
    else
        log_fail "Panic on special characters"
    fi

    # Test 15: Database file with wrong magic bytes
    log_test_start "CORRUPTION TEST 15: Wrong file type (not SQLite)"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"
    echo "This is not a SQLite database" > "$OYA_DIR/.zjj/state.db"

    output=$(run_safe "zjj status 2>&1" "wrong file type")
    result=$?

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "database|file|magic|header"; then
            log_pass "Wrong file type detected"
        else
            log_warn "Wrong file type handled but message unclear"
        fi
    else
        log_fail "Panic on wrong file type"
    fi

    # Test 16: Multiple schema version entries (PRIMARY KEY violation)
    log_test_start "CORRUPTION TEST 16: Conflicting schema versions"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"
    sqlite3 "$OYA_DIR/.zjj/state.db" "INSERT INTO schema_version VALUES (2);" 2>/dev/null || true

    output=$(run_safe "zjj status 2>&1" "schema conflict")
    result=$?

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    if check_no_panic "$output"; then
        log_pass "Schema version conflict handled"
    else
        log_fail "Panic on schema conflict"
    fi

    # Test 17: NULL in NOT NULL column
    log_test_start "CORRUPTION TEST 17: NULL in NOT NULL column"

    zjj add corruption-test-17 --no-zellij >/dev/null 2>&1 || true

    # Try to set NULL (will fail constraint)
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET name = NULL WHERE name = 'corruption-test-17';" 2>/dev/null || true

    output=$(run_safe "zjj status 2>&1" "NULL constraint")
    result=$?

    cleanup_test_artifacts

    if check_no_panic "$output"; then
        log_pass "NULL constraint violation handled"
    else
        log_fail "Panic on NULL constraint"
    fi

    # Test 18: Checkpoint table corruption
    log_test_start "CORRUPTION TEST 18: Invalid checkpoint data"

    zjj add corruption-test-18 --no-zellij >/dev/null 2>&1 || true
    sqlite3 "$OYA_DIR/.zjj/state.db" "INSERT INTO checkpoints (session_id, checkpoint_data) VALUES (99999, 'invalid checkpoint');" 2>/dev/null || true

    output=$(run_safe "zjj status 2>&1" "checkpoint corruption")
    result=$?

    cleanup_test_artifacts

    if check_no_panic "$output"; then
        log_pass "Checkpoint corruption handled"
    else
        log_fail "Panic on checkpoint corruption"
    fi

    # Test 19: Session with non-existent Zellij session
    log_test_start "CORRUPTION TEST 19: Session references non-existent Zellij session"

    zjj add corruption-test-19 --no-zellij >/dev/null 2>&1 || true

    # The session exists but Zellij doesn't know about it (created with --no-zellij)
    output=$(run_safe "zjj switch corruption-test-19 2>&1" "missing Zellij session")
    result=$?

    cleanup_test_artifacts

    if check_no_panic "$output"; then
        log_pass "Missing Zellij session handled"
    else
        log_fail "Panic on missing Zellij session"
    fi

    # Test 20: Invalid UTF-8 in database fields
    log_test_start "CORRUPTION TEST 20: Invalid UTF-8 in session name"

    # Try to create session with invalid UTF-8
    invalid_name=$'test\x80\x81\x82'  # Invalid UTF-8 bytes

    output=$(run_safe "zjj add '$invalid_name' --no-zellij 2>&1" "invalid UTF-8")
    result=$?

    if check_no_panic "$output"; then
        log_pass "Invalid UTF-8 handled without panic"
    else
        log_fail "Panic on invalid UTF-8"
    fi

    # Final report
    echo "" >> "$CORRUPTION_REPORT"
    echo "========================================" >> "$CORRUPTION_REPORT"
    echo "TEST SUITE COMPLETE" >> "$CORRUPTION_REPORT"
    echo "End time: $(date)" >> "$CORRUPTION_REPORT"
    echo "" >> "$CORRUPTION_REPORT"
    echo "SUMMARY:" >> "$CORRUPTION_REPORT"
    echo "  Total tests: $TOTAL_TESTS" >> "$CORRUPTION_REPORT"
    echo "  Passed: $PASSED_TESTS" >> "$CORRUPTION_REPORT"
    echo "  Failed: $FAILED_TESTS" >> "$CORRUPTION_REPORT"
    echo "  Warnings: $WARNING_TESTS" >> "$CORRUPTION_REPORT"
    echo "========================================" >> "$CORRUPTION_REPORT"

    echo ""
    echo -e "${BLUE}=== FINAL SUMMARY ===${NC}"
    echo "Total tests: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    echo -e "${RED}Failed: $FAILED_TESTS${NC}"
    echo -e "${YELLOW}Warnings: $WARNING_TESTS${NC}"
    echo ""
    echo "Full report: $CORRUPTION_REPORT"
    echo "Backup preserved at: $BACKUP_DIR"

    cleanup_test_artifacts

    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}✓ All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}✗ Some tests failed${NC}"
        return 1
    fi
}

main "$@"
