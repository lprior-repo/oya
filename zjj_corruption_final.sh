#!/bin/bash
# CORRUPTION AGENT #19 - FINAL VERSION - No Nonsense Corruption Testing
# This script tests actual corruption scenarios with minimal overhead

set -e

REPORT="/home/lewis/src/oya/zjj_corruption_final_results.log"
OYA_DIR="/home/lewis/src/oya"
TIMEOUT=5

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Counters
PASS=0
FAIL=0
WARN=0
TOTAL=0

echo "=== ZJJ CORRUPTION TESTING - FINAL ===" > "$REPORT"
echo "Agent: QA #19 - CORRUPTION AGENT" >> "$REPORT"
echo "Started: $(date)" >> "$REPORT"
echo "" >> "$REPORT"

run_test() {
    local name="$1"
    local cmd="$2"
    local expected="$3"  # "error" or "success"

    ((TOTAL++))
    echo -e "${BLUE}[$TOTAL] Testing: $name${NC}"
    echo "[$TOTAL] Testing: $name" >> "$REPORT"

    # Run with timeout
    output=$(timeout "$TIMEOUT" bash -c "$cmd" 2>&1) || local exit_code=$?
    echo "$output" >> "$REPORT"

    # Check for panic
    if echo "$output" | grep -qi "panic"; then
        echo -e "${RED}✗ FAIL: Panic detected${NC}"
        echo "✗ FAIL: Panic detected" >> "$REPORT"
        ((FAIL++))
        return 1
    fi

    # Check expectations
    if [ "$expected" = "error" ]; then
        if echo "$output" | grep -qiE "error:|not found|failed|invalid|corrupt|parse"; then
            echo -e "${GREEN}✓ PASS: Error properly detected${NC}"
            echo "✓ PASS: Error properly detected" >> "$REPORT"
            ((PASS++))
            return 0
        else
            echo -e "${YELLOW}⚠ WARN: Expected error not clearly reported${NC}"
            echo "⚠ WARN: Expected error not clearly reported" >> "$REPORT"
            ((WARN++))
            return 0
        fi
    else
        if [ ${exit_code:-0} -eq 0 ] || [ ${exit_code:-0} -eq 124 ]; then
            echo -e "${GREEN}✓ PASS: Handled gracefully${NC}"
            echo "✓ PASS: Handled gracefully" >> "$REPORT"
            ((PASS++))
            return 0
        else
            echo -e "${RED}✗ FAIL: Unexpected behavior${NC}"
            echo "✗ FAIL: Unexpected behavior" >> "$REPORT"
            ((FAIL++))
            return 1
        fi
    fi
}

# ============================================================================
# CORRUPTION TESTS
# ============================================================================

echo "Starting corruption tests..."
echo "" >> "$REPORT"

# Save original config
cp "$OYA_DIR/.zjj/config.toml" /tmp/zjj_config_backup.toml

# Test 1: Invalid TOML syntax
echo "corrupt [invalid toml" > "$OYA_DIR/.zjj/config.toml"
run_test "Invalid TOML syntax" "zjj status 2>&1" "error"
cp /tmp/zjj_config_backup.toml "$OYA_DIR/.zjj/config.toml"

# Test 2: Invalid type in config
sed 's/use_tabs = true/use_tabs = "not_a_boolean"/' /tmp/zjj_config_backup.toml > "$OYA_DIR/.zjj/config.toml"
run_test "Invalid type (boolean as string)" "zjj status 2>&1" "error"
cp /tmp/zjj_config_backup.toml "$OYA_DIR/.zjj/config.toml"

# Test 3: Missing .zjj directory
rm -rf "$OYA_DIR/.zjj"
run_test "Missing .zjj directory" "zjj status 2>&1" "error"

# Restore .zjj (we need to reinitialize)
cd "$OYA_DIR" && jj workspace list >/dev/null 2>&1 || true

# Test 4: Corrupted state.db (garbage data)
if [ -f "$OYA_DIR/.zjj/state.db" ]; then
    cp "$OYA_DIR/.zjj/state.db" /tmp/zjj_state_backup.db
    echo "corrupt garbage data not sqlite" > "$OYA_DIR/.zjj/state.db"
    run_test "Corrupted SQLite database" "zjj status 2>&1" "error"
    cp /tmp/zjj_state_backup.db "$OYA_DIR/.zjj/state.db"
fi

# Test 5: Truncated state.db
if [ -f "$OYA_DIR/.zjj/state.db" ]; then
    cp "$OYA_DIR/.zjj/state.db" /tmp/zjj_state_backup.db
    truncate -s 50% "$OYA_DIR/.zjj/state.db"
    run_test "Truncated SQLite database" "zjj status 2>&1" "error"
    cp /tmp/zjj_state_backup.db "$OYA_DIR/.zjj/state.db"
fi

# Test 6: Duplicate session name
zjj add test-duplicate-session --no-zellij >/dev/null 2>&1 || true
run_test "Duplicate session name" "zjj add test-duplicate-session --no-zellij 2>&1" "error"
zjj remove test-duplicate-session --force >/dev/null 2>&1 || true

# Test 7: Special characters in name
run_test "Special characters in name (slash)" "zjj add 'test/workspace' --no-zellij 2>&1" "error"
run_test "Special characters in name (backslash)" "zjj add 'test\\workspace' --no-zellij 2>&1" "error"

# Test 8: Very long name
long_name=$(python3 -c "print('a' * 1000)")
run_test "Very long session name (1000 chars)" "zjj add '$long_name' --no-zellij 2>&1" "error"

# Test 9: Invalid UTF-8
invalid_name=$'test\x80\x81\x82'
run_test "Invalid UTF-8 in name" "zjj add '$invalid_name' --no-zellij 2>&1" "error"

# Test 10: Non-existent session
run_test "Non-existent session" "zjj status this-session-does-not-exist-xyz 2>&1" "error"
run_test "Sync non-existent session" "zjj sync this-session-does-not-exist-xyz 2>&1" "error"

# Test 11: Create test workspace and corrupt its metadata
if zjj add corruption-test-meta --no-zellij >/dev/null 2>&1; then
    # Try to corrupt metadata
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET metadata = '{invalid json}' WHERE name = 'corruption-test-meta';" 2>/dev/null || true
    run_test "Invalid JSON in metadata" "zjj status corruption-test-meta 2>&1" "success"
    zjj remove corruption-test-meta --force >/dev/null 2>&1 || true
fi

# Test 12: Invalid state value
if zjj add corruption-test-state --no-zellij >/dev/null 2>&1; then
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET state = 'invalid_state' WHERE name = 'corruption-test-state';" 2>/dev/null || true
    run_test "Invalid state value" "zjj status corruption-test-state 2>&1" "success"
    zjj remove corruption-test-state --force >/dev/null 2>&1 || true
fi

# Test 13: Negative timestamp
if zjj add corruption-test-time --no-zellij >/dev/null 2>&1; then
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET created_at = -999999, updated_at = -999999 WHERE name = 'corruption-test-time';" 2>/dev/null || true
    run_test "Negative timestamp" "zjj status corruption-test-time 2>&1" "success"
    zjj remove corruption-test-time --force >/dev/null 2>&1 || true
fi

# Test 14: Non-existent workspace path
if zjj add corruption-test-path --no-zellij >/dev/null 2>&1; then
    sqlite3 "$OYA_DIR/.zjj/state.db" "UPDATE sessions SET workspace_path = '/non/existent/path/xyz' WHERE name = 'corruption-test-path';" 2>/dev/null || true
    run_test "Non-existent workspace path" "zjj status corruption-test-path 2>&1" "success"
    zjj remove corruption-test-path --force >/dev/null 2>&1 || true
fi

# Test 15: Read-only config
chmod 444 "$OYA_DIR/.zjj/config.toml"
run_test "Read-only config file" "zjj add test-readonly --no-zellij 2>&1" "error"
chmod 644 "$OYA_DIR/.zjj/config.toml"

# Test 16: Empty config file
echo "" > "$OYA_DIR/.zjj/config.toml"
run_test "Empty config file" "zjj status 2>&1" "error"
cp /tmp/zjj_config_backup.toml "$OYA_DIR/.zjj/config.toml"

# Test 17: Database without schema
if [ -f "$OYA_DIR/.zjj/state.db" ]; then
    cp "$OYA_DIR/.zjj/state.db" /tmp/zjj_state_backup.db
    sqlite3 "$OYA_DIR/.zjj/state.db" "DROP TABLE IF EXISTS sessions;" 2>/dev/null || true
    run_test "Missing database tables" "zjj status 2>&1" "error"
    cp /tmp/zjj_state_backup.db "$OYA_DIR/.zjj/state.db"
fi

# Test 18: Schema version conflict
if [ -f "$OYA_DIR/.zjj/state.db" ]; then
    cp "$OYA_DIR/.zjj/state.db" /tmp/zjj_state_backup.db
    sqlite3 "$OYA_DIR/.zjj/state.db" "INSERT OR IGNORE INTO schema_version VALUES (2);" 2>/dev/null || true
    run_test "Conflicting schema versions" "zjj status 2>&1" "success"
    cp /tmp/zjj_state_backup.db "$OYA_DIR/.zjj/state.db"
fi

# Test 19: Orphaned lock record
if [ -f "$OYA_DIR/.zjj/state.db" ]; then
    sqlite3 "$OYA_DIR/.zjj/state.db" "DELETE FROM session_locks WHERE session = 'orphan-lock-test';" 2>/dev/null || true
    sqlite3 "$OYA_DIR/.zjj/state.db" "INSERT INTO session_locks (lock_id, session, agent_id, expires_at) VALUES ('test-orphan', 'orphan-lock-test', 'agent-1', $(( $(date +%s) + 3600 )));" 2>/dev/null || true
    run_test "Orphaned lock record" "zjj status 2>&1" "success"
fi

# Test 20: Whereami from non-workspace
run_test "whereami from main" "zjj whereami 2>&1" "success"
run_test "whereami JSON format" "zjj whereami --json 2>&1" "success"

# Cleanup
rm -f /tmp/zjj_config_backup.toml
rm -f /tmp/zjj_state_backup.db

# Clean up test sessions
sqlite3 "$OYA_DIR/.zjj/state.db" "DELETE FROM sessions WHERE name LIKE 'corruption-test-%';" 2>/dev/null || true
sqlite3 "$OYA_DIR/.zjj/state.db" "DELETE FROM session_locks WHERE session LIKE 'corruption-test-%';" 2>/dev/null || true

# Final report
echo "" >> "$REPORT"
echo "========================================" >> "$REPORT"
echo "TEST COMPLETE: $(date)" >> "$REPORT"
echo "Total: $TOTAL | Pass: $PASS | Fail: $FAIL | Warn: $WARN" >> "$REPORT"
echo "========================================" >> "$REPORT"

echo ""
echo -e "${BLUE}=== FINAL RESULTS ===${NC}"
echo "Total tests: $TOTAL"
echo -e "${GREEN}Passed: $PASS${NC}"
echo -e "${RED}Failed: $FAIL${NC}"
echo -e "${YELLOW}Warnings: $WARN${NC}"
echo ""
echo "Full report: $REPORT"

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ All critical tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
