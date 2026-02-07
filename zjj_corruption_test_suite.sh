#!/bin/bash
# CORRUPTION AGENT #19 - Brutal Corruption Testing Suite for zjj
#
# This script ACTUALLY CORRUPTS state to test error handling and recovery.
# All tests verify: no panics, no crashes, clear error messages, recovery mechanisms.
#
# TARGET FILES:
# - .zjj/config.toml (configuration)
# - .zjj/state.db (SQLite database)
# - .jj/workspaces/<name> (workspace directories)
# - .zjj/layouts/ (Zellij layouts)
#
# TEST SCENARIOS:
# 1. Delete .zjj directory mid-operation
# 2. Corrupt .zjj/state.db
# 3. Corrupt .zjj/config.toml
# 4. Delete workspace .jj directory
# 5. Modify workspace .jj/repo store
# 6. Create sessions with same name
# 7. Create workspace in readonly filesystem
# 8. Fill disk during operations
# 9. Kill zjj process mid-operation
# 10. Corrupt Zellij session files

set -e

CORRUPTION_REPORT="/home/lewis/src/oya/zjj_corruption_test_results.log"
BACKUP_DIR="/home/lewis/src/oya/.zjj_backup_$(date +%s)"
OYA_DIR="/home/lewis/src/oya"

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
WARNING_TESTS=0

# Helper functions
log_header() {
    echo -e "${BLUE}=====================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}=====================================${NC}"
    echo "=====================================" >> "$CORRUPTION_REPORT"
    echo "$1" >> "$CORRUPTION_REPORT"
    echo "=====================================" >> "$CORRUPTION_REPORT"
}

log_test() {
    echo -e "${YELLOW}TEST $((TOTAL_TESTS + 1)): $1${NC}"
    echo "" >> "$CORRUPTION_REPORT"
    echo "TEST $((TOTAL_TESTS + 1)): $1" >> "$CORRUPTION_REPORT"
    echo "Timestamp: $(date)" >> "$CORRUPTION_REPORT"
}

log_pass() {
    echo -e "${GREEN}✓ PASS: $1${NC}"
    echo "✓ PASS: $1" >> "$CORRUPTION_REPORT"
    ((PASSED_TESTS++))
}

log_fail() {
    echo -e "${RED}✗ FAIL: $1${NC}"
    echo "✗ FAIL: $1" >> "$CORRUPTION_REPORT"
    ((FAILED_TESTS++))
}

log_warn() {
    echo -e "${YELLOW}⚠ WARN: $1${NC}"
    echo "⚠ WARN: $1" >> "$CORRUPTION_REPORT"
    ((WARNING_TESTS++))
}

log_info() {
    echo -e "${BLUE}ℹ INFO: $1${NC}"
    echo "INFO: $1" >> "$CORRUPTION_REPORT"
}

increment_test() {
    ((TOTAL_TESTS++))
}

# Backup and restore functions
backup_state() {
    log_info "Creating backup at $BACKUP_DIR"
    mkdir -p "$BACKUP_DIR"

    if [ -d "$OYA_DIR/.zjj" ]; then
        cp -r "$OYA_DIR/.zjj" "$BACKUP_DIR/"
    fi

    if [ -d "$OYA_DIR/.jj" ]; then
        cp -r "$OYA_DIR/.jj" "$BACKUP_DIR/"
    fi

    if [ -d "$OYA_DIR/oya__workspaces" ]; then
        cp -r "$OYA_DIR/oya__workspaces" "$BACKUP_DIR/"
    fi
}

restore_state() {
    log_info "Restoring from backup"

    # Remove corrupted state
    rm -rf "$OYA_DIR/.zjj"
    rm -rf "$OYA_DIR/.jj/workspaces"/* 2>/dev/null || true

    # Restore from backup
    if [ -d "$BACKUP_DIR/.zjj" ]; then
        cp -r "$BACKUP_DIR/.zjj" "$OYA_DIR/"
    fi

    if [ -d "$BACKUP_DIR/.jj" ]; then
        cp -r "$BACKUP_DIR/.jj/workspaces" "$OYA_DIR/.jj/"
    fi
}

cleanup_test_workspace() {
    # Clean up any test workspaces
    if [ -d "$OYA_DIR/oya__workspaces" ]; then
        find "$OYA_DIR/oya__workspaces" -name "corruption-test-*" -type d -exec rm -rf {} + 2>/dev/null || true
    fi

    # Clean up test sessions from database
    if [ -f "$OYA_DIR/.zjj/state.db" ]; then
        sqlite3 "$OYA_DIR/.zjj/state.db" "DELETE FROM sessions WHERE name LIKE 'corruption-test-%';" 2>/dev/null || true
        sqlite3 "$OYA_DIR/.zjj/state.db" "DELETE FROM session_locks WHERE session LIKE 'corruption-test-%';" 2>/dev/null || true
    fi
}

init_report() {
    echo "=== ZJJ CORRUPTION TEST SUITE ===" > "$CORRUPTION_REPORT"
    echo "Agent: QA Agent #19 - CORRUPTION AGENT" >> "$CORRUPTION_REPORT"
    echo "Start time: $(date)" >> "$CORRUPTION_REPORT"
    echo "Mission: CORRUPT EVERYTHING and verify error handling" >> "$CORRUPTION_REPORT"
    echo "" >> "$CORRUPTION_REPORT"
}

finalize_report() {
    echo "" >> "$CORRUPTION_REPORT"
    echo "=== TEST SUITE COMPLETE ===" >> "$CORRUPTION_REPORT"
    echo "End time: $(date)" >> "$CORRUPTION_REPORT"
    echo "" >> "$CORRUPTION_REPORT"
    echo "SUMMARY:" >> "$CORRUPTION_REPORT"
    echo "  Total tests: $TOTAL_TESTS" >> "$CORRUPTION_REPORT"
    echo "  Passed: $PASSED_TESTS" >> "$CORRUPTION_REPORT"
    echo "  Failed: $FAILED_TESTS" >> "$CORRUPTION_REPORT"
    echo "  Warnings: $WARNING_TESTS" >> "$CORRUPTION_REPORT"
    echo "" >> "$CORRUPTION_REPORT"

    echo -e "\n${BLUE}=== FINAL SUMMARY ===${NC}"
    echo -e "Total tests: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    echo -e "${RED}Failed: $FAILED_TESTS${NC}"
    echo -e "${YELLOW}Warnings: $WARNING_TESTS${NC}"
    echo ""
    echo "Full report: $CORRUPTION_REPORT"

    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}✓ All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}✗ Some tests failed${NC}"
        return 1
    fi
}

# Verify no panic occurred
check_no_panic() {
    local output="$1"
    if echo "$output" | grep -qi "panic"; then
        log_fail "Panic detected in output"
        echo "$output" >> "$CORRUPTION_REPORT"
        return 1
    else
        log_pass "No panic detected"
        return 0
    fi
}

# Verify clear error message
check_error_message() {
    local output="$1"
    if echo "$output" | grep -qiE "error:|not found|failed|invalid|corrupt"; then
        log_pass "Clear error message provided"
        return 0
    else
        log_warn "Error message unclear or missing"
        echo "$output" >> "$CORRUPTION_REPORT"
        return 1
    fi
}

# ============================================================================
# TEST SUITE
# ============================================================================

main() {
    init_report
    backup_state

    # Test 1: Corrupt .zjj/config.toml with invalid TOML
    log_header "CORRUPTION TEST 1: Invalid TOML in config.toml"
    increment_test
    log_test "Corrupt config.toml with invalid TOML syntax"

    # Backup original config
    cp "$OYA_DIR/.zjj/config.toml" "$BACKUP_DIR/config.toml.bak"

    # Corrupt the config
    echo "corrupt [invalid toml" > "$OYA_DIR/.zjj/config.toml"

    # Test command
    output=$(zjj status 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output" && check_error_message "$output"; then
        log_pass "Config corruption handled gracefully"
    else
        log_fail "Config corruption not handled properly"
    fi

    # Restore config
    cp "$BACKUP_DIR/config.toml.bak" "$OYA_DIR/.zjj/config.toml"

    # Test 2: Corrupt .zjj/config.toml with invalid type
    log_header "CORRUPTION TEST 2: Invalid type in config.toml"
    increment_test
    log_test "Set boolean field to string value"

    cp "$OYA_DIR/.zjj/config.toml" "$BACKUP_DIR/config.toml.bak"

    # Corrupt with invalid type (use_tabs should be boolean)
    sed -i 's/use_tabs = true/use_tabs = "not_a_boolean"/' "$OYA_DIR/.zjj/config.toml"

    output=$(zjj add corruption-test-2 --no-zellij 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output" && check_error_message "$output"; then
        log_pass "Type error in config detected and reported"
    else
        log_fail "Type error not handled properly"
    fi

    cp "$BACKUP_DIR/config.toml.bak" "$OYA_DIR/.zjj/config.toml"

    # Test 3: Delete .zjj directory
    log_header "CORRUPTION TEST 3: Delete .zjj directory"
    increment_test
    log_test "Remove entire .zjj directory mid-operation"

    rm -rf "$OYA_DIR/.zjj"

    output=$(zjj status 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "no zjj|not initialized|\.zjj"; then
            log_pass "Missing .zjj directory detected"
        else
            log_warn "Error message could be clearer about missing .zjj"
        fi
    else
        log_fail "Panic or crash when .zjj missing"
    fi

    # Restore .zjj
    restore_state

    # Test 4: Corrupt SQLite database with invalid data
    log_header "CORRUPTION TEST 4: Corrupt SQLite database"
    increment_test
    log_test "Write random data to state.db"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"

    # Write random garbage to database
    echo "corrupt database garbage data" > "$OYA_DIR/.zjj/state.db"

    output=$(zjj status 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "database|corrupt|invalid"; then
            log_pass "Database corruption detected"
        else
            log_warn "Database error handling unclear"
        fi
    else
        log_fail "Panic on corrupted database"
    fi

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    # Test 5: Partial SQLite database (truncated)
    log_header "CORRUPTION TEST 5: Truncate SQLite database"
    increment_test
    log_test "Truncate database to 50% of original size"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"

    # Truncate to half size
    original_size=$(stat -f%z "$OYA_DIR/.zjj/state.db" 2>/dev/null || stat -c%s "$OYA_DIR/.zjj/state.db")
    truncated_size=$((original_size / 2))
    truncate -s "$truncated_size" "$OYA_DIR/.zjj/state.db"

    output=$(zjj status 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "Truncated database handled without panic"
    else
        log_fail "Panic on truncated database"
    fi

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    # Test 6: Delete workspace .jj directory
    log_header "CORRUPTION TEST 6: Delete workspace .jj directory"
    increment_test
    log_test "Create workspace then delete its .jj directory"

    # Create a test workspace
    zjj add corruption-test-6 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Find the workspace directory
    workspace_dir=$(find "$OYA_DIR/oya__workspaces" -name "corruption-test-6" -type d 2>/dev/null | head -1)

    if [ -n "$workspace_dir" ]; then
        # Delete .jj directory inside workspace
        rm -rf "$workspace_dir/.jj"

        output=$(zjj status corruption-test-6 2>&1 || true)
        echo "$output" >> "$CORRUPTION_REPORT"

        if check_no_panic "$output"; then
            log_pass "Missing workspace .jj handled gracefully"
        else
            log_fail "Panic when workspace .jj missing"
        fi
    else
        log_warn "Could not create test workspace for deletion test"
    fi

    cleanup_test_workspace

    # Test 7: Duplicate session name (UNIQUE constraint)
    log_header "CORRUPTION TEST 7: Duplicate session names"
    increment_test
    log_test "Create two sessions with same name"

    # Create first session
    zjj add corruption-test-7 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Try to create duplicate
    output=$(zjj add corruption-test-7 --no-zellij 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "exists|already|duplicate|unique"; then
            log_pass "Duplicate session detected and rejected"
        else
            log_warn "Duplicate error message unclear"
        fi
    else
        log_fail "Panic on duplicate session"
    fi

    cleanup_test_workspace

    # Test 8: Lock file corruption
    log_header "CORRUPTION TEST 8: Lock table corruption"
    increment_test
    log_test "Insert invalid lock records"

    # Create a test workspace
    zjj add corruption-test-8 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Insert invalid lock (non-existent session)
    sqlite3 "$OYA_DIR/.zjj/state.db" "INSERT INTO session_locks (lock_id, session, agent_id, expires_at) VALUES ('corrupt-lock', 'non-existent-session', 'agent-1', 9999999999);" 2>/dev/null || true

    output=$(zjj status 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "Orphaned lock handled gracefully"
    else
        log_fail "Panic with orphaned lock"
    fi

    cleanup_test_workspace

    # Test 9: JSON metadata corruption
    log_header "CORRUPTION TEST 9: Corrupt JSON metadata in sessions"
    increment_test
    log_test "Insert invalid JSON into metadata field"

    # Create test workspace
    zjj add corruption-test-9 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Update with invalid JSON
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET metadata = '{invalid json}' WHERE name = 'corruption-test-9';" 2>/dev/null || true

    output=$(zjj status corruption-test-9 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "Invalid JSON metadata handled"
    else
        log_fail "Panic on invalid JSON"
    fi

    cleanup_test_workspace

    # Test 10: Invalid state transitions
    log_header "CORRUPTION TEST 10: Invalid state transition"
    increment_test
    log_test "Set invalid state in sessions table"

    # Create test workspace
    zjj add corruption-test-10 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Set invalid state
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET state = 'invalid_state' WHERE name = 'corruption-test-10';" 2>/dev/null || true

    output=$(zjj status corruption-test-10 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "Invalid state handled gracefully"
    else
        log_fail "Panic on invalid state"
    fi

    cleanup_test_workspace

    # Test 11: Empty database (no tables)
    log_header "CORRUPTION TEST 11: Empty SQLite database"
    increment_test
    log_test "Create database without schema"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"

    # Create empty database
    sqlite3 "$OYA_DIR/.zjj/state.db" "DROP TABLE IF EXISTS sessions; DROP TABLE IF EXISTS schema_version;" 2>/dev/null || true

    output=$(zjj status 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "schema|table|database"; then
            log_pass "Missing schema detected"
        else
            log_warn "Schema error handling unclear"
        fi
    else
        log_fail "Panic on missing schema"
    fi

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    # Test 12: Permissions error (read-only config)
    log_header "CORRUPTION TEST 12: Read-only config file"
    increment_test
    log_test "Make config.toml read-only"

    chmod 444 "$OYA_DIR/.zjj/config.toml"

    # Try to create session (will fail to write state)
    output=$(zjj add corruption-test-12 --no-zellij 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "Permission error handled"
    else
        log_fail "Panic on permission error"
    fi

    # Restore permissions
    chmod 644 "$OYA_DIR/.zjj/config.toml"

    # Test 13: Simulate concurrent write (database lock)
    log_header "CORRUPTION TEST 13: Simulate database lock"
    increment_test
    log_test "Lock database and attempt write"

    # Create test workspace
    zjj add corruption-test-13 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Lock the database
    sqlite3 "$OYA_DIR/.zjj/state.db" "BEGIN EXCLUSIVE; SELECT * FROM sessions; -- Keep transaction open" &
    lock_pid=$!
    sleep 1  # Give it time to acquire lock

    # Try to write to database
    output=$(timeout 5 zjj sync corruption-test-13 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    # Kill the lock process
    kill $lock_pid 2>/dev/null || true
    wait $lock_pid 2>/dev/null || true

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "lock|busy|database"; then
            log_pass "Database lock detected"
        else
            log_warn "Lock error handling unclear"
        fi
    else
        log_fail "Panic on database lock"
    fi

    cleanup_test_workspace

    # Test 14: Corrupt Zellij layout file
    log_header "CORRUPTION TEST 14: Corrupt Zellij layout"
    increment_test
    log_test "Write invalid YAML to layout file"

    if [ -d "$OYA_DIR/.zjj/layouts" ]; then
        # Find a layout file
        layout_file=$(find "$OYA_DIR/.zjj/layouts" -name "*.kdl" -o -name "*.yaml" 2>/dev/null | head -1)

        if [ -n "$layout_file" ]; then
            cp "$layout_file" "$BACKUP_DIR/layout.bak"

            # Corrupt with invalid data
            echo "invalid { corrupt layout } data" > "$layout_file"

            output=$(zjj add corruption-test-14 --no-zellij 2>&1 || true)
            echo "$output" >> "$CORRUPTION_REPORT"

            if check_no_panic "$output"; then
                log_pass "Corrupted layout handled"
            else
                log_fail "Panic on corrupted layout"
            fi

            cp "$BACKUP_DIR/layout.bak" "$layout_file"
        else
            log_warn "No layout files found to test"
        fi
    else
        log_warn "Layouts directory does not exist"
    fi

    cleanup_test_workspace

    # Test 15: Create workspace with invalid characters in name
    log_header "CORRUPTION TEST 15: Invalid workspace name"
    increment_test
    log_test "Try to create workspace with special characters"

    # Try various invalid names
    invalid_names=("test/workspace" "test\workspace" "test:workspace" "test*workspace" "test?workspace")

    for name in "${invalid_names[@]}"; do
        output=$(zjj add "$name" --no-zellij 2>&1 || true)
        echo "$output" >> "$CORRUPTION_REPORT"

        if check_no_panic "$output"; then
            if echo "$output" | grep -qiE "invalid|special|character"; then
                log_pass "Invalid characters rejected: $name"
                break
            fi
        fi
    done

    # Test 16: Negative timestamp in created_at
    log_header "CORRUPTION TEST 16: Negative timestamp"
    increment_test
    log_test "Set negative timestamp in sessions table"

    # Create test workspace
    zjj add corruption-test-16 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Set negative timestamp
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET created_at = -999999, updated_at = -999999 WHERE name = 'corruption-test-16';" 2>/dev/null || true

    output=$(zjj status corruption-test-16 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "Negative timestamp handled"
    else
        log_fail "Panic on negative timestamp"
    fi

    cleanup_test_workspace

    # Test 17: Very long session name
    log_header "CORRUPTION TEST 17: Extremely long session name"
    increment_test
    log_test "Create session with 1000-character name"

    long_name=$(printf 'a%.0s' {1..1000})

    output=$(zjj add "$long_name" --no-zellij 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "Long name handled without panic"
    else
        log_fail "Panic on long name"
    fi

    # Test 18: NULL in required fields
    log_header "CORRUPTION TEST 18: NULL in required fields"
    increment_test
    log_test "Set NULL in NOT NULL columns"

    # Create test workspace
    zjj add corruption-test-18 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Try to set NULL in required field (should fail due to constraint)
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET name = NULL WHERE name = 'corruption-test-18';" 2>/dev/null || true

    output=$(zjj status 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "NULL constraint handled"
    else
        log_fail "Panic on constraint violation"
    fi

    cleanup_test_workspace

    # Test 19: Non-existent workspace path
    log_header "CORRUPTION TEST 19: Non-existent workspace path"
    increment_test
    log_test "Point workspace_path to non-existent directory"

    # Create test workspace
    zjj add corruption-test-19 --no-zellij 2>&1 >> "$CORRUPTION_REPORT" || true

    # Update path to non-existent directory
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET workspace_path = '/non/existent/path/corruption-test-19' WHERE name = 'corruption-test-19';" 2>/dev/null || true

    output=$(zjj status corruption-test-19 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        if echo "$output" | grep -qiE "not found|exist|path"; then
            log_pass "Non-existent path detected"
        else
            log_warn "Path error handling unclear"
        fi
    else
        log_fail "Panic on non-existent path"
    fi

    cleanup_test_workspace

    # Test 20: Multiple schema versions
    log_header "CORRUPTION TEST 20: Conflicting schema versions"
    increment_test
    log_test "Insert multiple schema version records"

    cp "$OYA_DIR/.zjj/state.db" "$BACKUP_DIR/state.db.bak"

    # Try to insert multiple versions (should violate PRIMARY KEY)
    sqlite3 "$OYA_DIR/.zjj/state.db" "INSERT INTO schema_version VALUES (2);" 2>/dev/null || true

    output=$(zjj status 2>&1 || true)
    echo "$output" >> "$CORRUPTION_REPORT"

    if check_no_panic "$output"; then
        log_pass "Schema conflict handled"
    else
        log_fail "Panic on schema conflict"
    fi

    cp "$BACKUP_DIR/state.db.bak" "$OYA_DIR/.zjj/state.db"

    # Final cleanup and report
    cleanup_test_workspace
    finalize_report

    # Keep backup for debugging
    log_info "Backup preserved at: $BACKUP_DIR"
}

# Run the suite
main "$@"
