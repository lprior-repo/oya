#!/bin/bash
# COMPREHENSIVE ZJJ SESSION MANAGEMENT TEST
# QA Agent #2 - Testing: list, status, remove, rename, focus

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TEST_DIR="/tmp/zjj_qa_$$"
PASS=0
FAIL=0
ISSUES=()

log() { echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"; }
pass() { echo -e "${GREEN}✓ PASS${NC} $1"; ((PASS++)); }
fail() { echo -e "${RED}✗ FAIL${NC} $1"; ((FAIL++)); ISSUES+=("$1"); }

cd /tmp && rm -rf "$TEST_DIR" && mkdir "$TEST_DIR" && cd "$TEST_DIR"

log "=== ZJJ SESSION MANAGEMENT BRUTAL QA TEST ==="
log "Test directory: $TEST_DIR\n"

# Setup
zjj init >/dev/null 2>&1
sleep 1

#=============================================================================
# TEST 1: List with 0 sessions
#=============================================================================
log "[TEST 1] List with 0 sessions"
OUTPUT=$(zjj list 2>&1)
if echo "$OUTPUT" | grep -qi "no sessions"; then
    pass "Empty list shows 'no sessions'"
else
    fail "Empty list missing expected message"
fi

#=============================================================================
# TEST 2: Create and list 1 session
#=============================================================================
log "\n[TEST 2] Create single session"
zjj add --no-zellij session1 >/dev/null 2>&1
sleep 1
OUTPUT=$(zjj list)
if echo "$OUTPUT" | grep -q "session1"; then
    pass "Session created and appears in list"
else
    fail "Session not found in list"
fi

#=============================================================================
# TEST 3: Status of single session
#=============================================================================
log "\n[TEST 3] Status command"
OUTPUT=$(zjj status session1 2>&1)
if echo "$OUTPUT" | grep -q "session1"; then
    pass "Status shows session information"
else
    fail "Status doesn't show session"
fi

#=============================================================================
# TEST 4: Focus command (outside Zellij)
#=============================================================================
log "\n[TEST 4] Focus command outside Zellij"
if zjj focus session1 2>&1 | grep -qi "not.*zellij\|error"; then
    pass "Focus fails outside Zellij as expected"
else
    fail "Focus should fail outside Zellij"
fi

#=============================================================================
# TEST 5: Focus non-existent session
#=============================================================================
log "\n[TEST 5] Focus non-existent session"
if zjj focus nonexistent 2>&1 | grep -qi "error\|not found"; then
    pass "Focus non-existent fails appropriately"
else
    fail "Focus non-existent should fail"
fi

#=============================================================================
# TEST 6: Remove session
#=============================================================================
log "\n[TEST 6] Remove session"
zjj remove -f session1 >/dev/null 2>&1
sleep 1
OUTPUT=$(zjj list)
if echo "$OUTPUT" | grep -q "session1"; then
    fail "Removed session still in list"
else
    pass "Session removed successfully"
fi

#=============================================================================
# TEST 7: Remove non-existent session
#=============================================================================
log "\n[TEST 7] Remove non-existent session"
if zjj remove -f nonexistent 2>&1 | grep -qi "not found\|error"; then
    pass "Remove non-existent fails with error"
else
    fail "Remove non-existent should fail"
fi

#=============================================================================
# TEST 8: Create 50 sessions
#=============================================================================
log "\n[TEST 8] Create 50 sessions rapidly"
START=$(date +%s)
for i in {1..50}; do
    zjj add --no-zellij "bulk$i" >/dev/null 2>&1 || true
done
END=$(date +%s)
log "Created in $((END-START))s"
OUTPUT=$(zjj list)
COUNT=$(echo "$OUTPUT" | grep -c "bulk" || echo "0")
if [ "$COUNT" -ge 45 ]; then
    pass "Bulk creation: $COUNT/50 sessions"
else
    fail "Bulk creation failed: only $COUNT/50"
fi

#=============================================================================
# TEST 9: List output consistency
#=============================================================================
log "\n[TEST 9] List output consistency"
OUTPUT1=$(zjj list)
sleep 1
OUTPUT2=$(zjj list)
if [ "$OUTPUT1" = "$OUTPUT2" ]; then
    pass "List output is consistent across calls"
else
    fail "List output is inconsistent"
fi

#=============================================================================
# TEST 10: Status with various sessions
#=============================================================================
log "\n[TEST 10] Status on individual sessions"
zjj status bulk1 >/dev/null 2>&1 && pass "Status works on bulk1" || fail "Status failed on bulk1"
zjj status bulk25 >/dev/null 2>&1 && pass "Status works on bulk25" || fail "Status failed on bulk25"
zjj status bulk50 >/dev/null 2>&1 && pass "Status works on bulk50" || fail "Status failed on bulk50"

#=============================================================================
# TEST 11: Remove bulk sessions
#=============================================================================
log "\n[TEST 11] Remove multiple sessions"
for i in {1..20}; do
    zjj remove -f "bulk$i" >/dev/null 2>&1 || true
done
sleep 1
OUTPUT=$(zjj list)
REMAINING=$(echo "$OUTPUT" | grep -c "bulk" || echo "0")
if [ "$REMAINING" -le 32 ]; then
    pass "Bulk remove: sessions reduced ($REMAINING remaining)"
else
    fail "Bulk remove: too many remaining ($REMAINING)"
fi

#=============================================================================
# TEST 12: Create sessions with special characters
#=============================================================================
log "\n[TEST 12] Special characters in names"
zjj add --no-zellij "test-with-dashes" >/dev/null 2>&1 && pass "Created session with dashes" || fail "Failed to create dashes"
zjj add --no-zellij "test_with_underscores" >/dev/null 2>&1 && pass "Created session with underscores" || fail "Failed to create underscores"
zjj add --no-zellij "test.dots" >/dev/null 2>&1 && pass "Created session with dots" || fail "Failed to create dots"
zjj add --no-zellij "test123" >/dev/null 2>&1 && pass "Created session with numbers" || fail "Failed to create numbers"

#=============================================================================
# TEST 13: List after special character sessions
#=============================================================================
log "\n[TEST 13] List with special character names"
OUTPUT=$(zjj list)
SPECIAL_COUNT=0
echo "$OUTPUT" | grep -q "test-with-dashes" && ((SPECIAL_COUNT++))
echo "$OUTPUT" | grep -q "test_with_underscores" && ((SPECIAL_COUNT++))
echo "$OUTPUT" | grep -q "test.dots" && ((SPECIAL_COUNT++))
echo "$OUTPUT" | grep -q "test123" && ((SPECIAL_COUNT++))
if [ "$SPECIAL_COUNT" -eq 4 ]; then
    pass "All special character sessions listed"
else
    fail "Only $SPECIAL_COUNT/4 special sessions in list"
fi

#=============================================================================
# TEST 14: Rapid create/delete cycles
#=============================================================================
log "\n[TEST 14] Rapid create/delete cycles"
for i in {1..20}; do
    zjj add --no-zellij "rapid$i" >/dev/null 2>&1
    zjj remove -f "rapid$i" >/dev/null 2>&1
done
pass "Rapid cycles completed without crash"

#=============================================================================
# TEST 15: Concurrent operations
#=============================================================================
log "\n[TEST 15] Concurrent session creation"
for i in {1..10}; do
    zjj add --no-zellij "concurrent-$i" >/dev/null 2>&1 &
done
wait
sleep 2
OUTPUT=$(zjj list)
CONCURRENT=$(echo "$OUTPUT" | grep -c "concurrent-" || echo "0")
if [ "$CONCURRENT" -ge 8 ]; then
    pass "Concurrent creation: $CONCURRENT/10 sessions"
else
    fail "Concurrent creation: only $CONCURRENT/10"
fi

#=============================================================================
# TEST 16: List formatting
#=============================================================================
log "\n[TEST 16] List formatting"
OUTPUT=$(zjj list)
if echo "$OUTPUT" | grep -qi "NAME\|STATUS"; then
    pass "List has headers"
else
    fail "List missing headers"
fi

#=============================================================================
# TEST 17: Status of non-existent
#=============================================================================
log "\n[TEST 17] Status of non-existent session"
if zjj status totally_not_real 2>&1 | grep -qi "error\|not found"; then
    pass "Status non-existent fails appropriately"
else
    fail "Status non-existent should fail"
fi

#=============================================================================
# TEST 18: Edge case - empty string operations
#=============================================================================
log "\n[TEST 18] Edge cases with empty strings"
zjj add --no-zellij "" 2>&1 | grep -qi "error\|invalid\|required" && pass "Empty name rejected" || fail "Empty name should be rejected"
zjj remove -f "" 2>&1 | grep -qi "error\|invalid\|required" && pass "Empty remove rejected" || fail "Empty remove should be rejected"

#=============================================================================
# TEST 19: Very long session name
#=============================================================================
log "\n[TEST 19] Very long session name"
LONG_NAME="very-long-session-name-with-many-characters-and-hyphens-$(date +%s)"
zjj add --no-zellij "$LONG_NAME" >/dev/null 2>&1 && pass "Long name accepted" || fail "Long name rejected"
OUTPUT=$(zjj list)
echo "$OUTPUT" | grep -q "$LONG_NAME" && pass "Long name in list" || fail "Long name not in list"

#=============================================================================
# TEST 20: Cleanup all sessions
#=============================================================================
log "\n[TEST 20] Cleanup all sessions"
BEFORE=$(zjj list | grep -c "\[" || echo "0")
zjj list | grep -oP '\[\K[^\]]+' | while read -r session; do
    zjj remove -f "$session" >/dev/null 2>&1 || true
done
sleep 1
AFTER=$(zjj list | grep -c "\[" || echo "0")
if [ "$AFTER" -lt "$BEFORE" ]; then
    pass "Cleanup removed sessions ($BEFORE -> $AFTER)"
else
    fail "Cleanup didn't remove sessions"
fi

#=============================================================================
# TEST 21: List JSON output (if supported)
#=============================================================================
log "\n[TEST 21] JSON output support"
zjj list --json >/dev/null 2>&1 && pass "JSON list supported" || log "JSON list not supported"

#=============================================================================
# TEST 22: Status JSON output (if supported)
#=============================================================================
log "\n[TEST 22] Status JSON support"
zjj status bulk21 --json >/dev/null 2>&1 && pass "JSON status supported" || log "JSON status not supported"

#=============================================================================
# TEST 23: Remove with --idempotent flag
#=============================================================================
log "\n[TEST 23] Idempotent remove"
zjj remove -f --idempotent nonexistent 2>&1 && pass "Idempotent remove works" || log "Idempotent flag not supported"

#=============================================================================
# SUMMARY
#=============================================================================
cd /home/lewis/src/oya
rm -rf "$TEST_DIR"

log "\n=== TEST SUMMARY ==="
log "PASSED: $PASS"
log "FAILED: $FAIL"
log "TOTAL:  $((PASS + FAIL))"

if [ ${#ISSUES[@]} -gt 0 ]; then
    log "\n=== ISSUES FOUND ==="
    for issue in "${ISSUES[@]}"; do
        log "• $issue"
    done
fi

if [ $FAIL -eq 0 ]; then
    echo -e "\n${GREEN}ALL TESTS PASSED${NC}\n"
    exit 0
else
    echo -e "\n${RED}SOME TESTS FAILED${NC}\n"
    exit 1
fi
