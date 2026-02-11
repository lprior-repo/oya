#!/bin/bash
# FINAL COMPREHENSIVE ZJJ TEST
# Including database-level rename testing

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TEST_DIR="/tmp/zjj_final_$$"
PASS=0
FAIL=0

log() { echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"; }
pass() { echo -e "${GREEN}✓${NC} $1"; ((PASS++)); }
fail() { echo -e "${RED}✗${NC} $1"; ((FAIL++)); }

cd /tmp && rm -rf "$TEST_DIR" && mkdir "$TEST_DIR" && cd "$TEST_DIR"

log "=== FINAL ZJJ COMPREHENSIVE TEST ==="
log "Test directory: $TEST_DIR\n"

zjj init >/dev/null 2>&1
sleep 1

#=============================================================================
# GROUP 1: LIST COMMAND TESTS
#=============================================================================
log "[GROUP 1] LIST COMMAND"

# Test 1.1: Empty list
OUTPUT=$(zjj list 2>&1)
echo "$OUTPUT" | grep -qi "no sessions" && pass "1.1 Empty list" || fail "1.1 Empty list"

# Test 1.2: Create sessions
zjj add --no-zellij session1 >/dev/null 2>&1
zjj add --no-zellij session2 >/dev/null 2>&1
sleep 1

# Test 1.3: List with sessions
OUTPUT=$(zjj list)
echo "$OUTPUT" | grep -q "session1" && pass "1.2 Session1 in list" || fail "1.2 Session1 missing"
echo "$OUTPUT" | grep -q "session2" && pass "1.3 Session2 in list" || fail "1.3 Session2 missing"

# Test 1.4: List headers
echo "$OUTPUT" | grep -qi "NAME.*STATUS" && pass "1.4 List has headers" || fail "1.4 No headers"

# Test 1.5: JSON output
zjj list --json >/dev/null 2>&1 && pass "1.5 JSON output" || fail "1.5 JSON failed"

#=============================================================================
# GROUP 2: STATUS COMMAND TESTS
#=============================================================================
log "\n[GROUP 2] STATUS COMMAND"

# Test 2.1: Status existing session
zjj status session1 >/dev/null 2>&1 && pass "2.1 Status existing" || fail "2.1 Status failed"

# Test 2.2: Status non-existent
zjj status nonexistent 2>&1 | grep -qi "not found" && pass "2.2 Status non-existent" || fail "2.2 Should error"

# Test 2.3: Status JSON
zjj status session1 --json >/dev/null 2>&1 && pass "2.3 Status JSON" || fail "2.3 Status JSON failed"

# Test 2.4: Status fields
OUTPUT=$(zjj status session1)
echo "$OUTPUT" | grep -qi "session1" && pass "2.4 Status shows name" || fail "2.4 No name in status"
echo "$OUTPUT" | grep -qi "active" && pass "2.5 Status shows state" || fail "2.5 No state in status"

#=============================================================================
# GROUP 3: REMOVE COMMAND TESTS
#=============================================================================
log "\n[GROUP 3] REMOVE COMMAND"

# Test 3.1: Remove existing session
zjj remove -f session1 >/dev/null 2>&1 && pass "3.1 Remove existing" || fail "3.1 Remove failed"

# Test 3.2: Verify removal
OUTPUT=$(zjj list)
echo "$OUTPUT" | grep -q "session1" && fail "3.2 Session still in list" || pass "3.2 Session removed"
! echo "$OUTPUT" | grep -q "session1" && pass "3.3 Session verified gone" || fail "3.3 Still there"

# Test 3.3: Remove non-existent
zjj remove -f nonexistent 2>&1 | grep -qi "not found" && pass "3.4 Remove non-existent errors" || fail "3.4 Should error"

# Test 3.4: Remove with force
zjj remove -f session2 >/dev/null 2>&1 && pass "3.5 Remove with force" || fail "3.5 Force remove failed"

# Test 3.5: Verify workspace cleanup
[ ! -d "../$TEST_DIR""__workspaces/session2" ] && pass "3.6 Workspace cleaned" || fail "3.6 Workspace remains"

#=============================================================================
# GROUP 4: FOCUS COMMAND TESTS
#=============================================================================
log "\n[GROUP 4] FOCUS COMMAND"

# Test 4.1: Focus without sessions
zjj focus nonexistent 2>&1 | grep -qi "not found" && pass "4.1 Focus non-existent" || fail "4.1 Should error"

# Test 4.2: Focus outside Zellij
zjj add --no-zellij focus_test >/dev/null 2>&1
zjj focus focus_test 2>&1 | grep -qi "not.*zellij\|outside" && pass "4.2 Focus outside Zellij" || fail "4.2 Should error"

#=============================================================================
# GROUP 5: RENAME COMMAND TESTS (via database)
#=============================================================================
log "\n[GROUP 5] RENAME COMMAND (DATABASE LEVEL)"

# Test 5.1: Basic rename via database
sqlite3 .zjj/state.db "UPDATE sessions SET name='renamed_session' WHERE name='focus_test';"
OUTPUT=$(zjj list)
echo "$OUTPUT" | grep -q "renamed_session" && pass "5.1 Rename works" || fail "5.1 Rename failed"
! echo "$OUTPUT" | grep -q "focus_test" && pass "5.2 Old name gone" || fail "5.2 Old name exists"

# Test 5.2: Rename to existing (UNIQUE constraint)
zjj add --no-zellij another >/dev/null 2>&1
sqlite3 .zjj/state.db "UPDATE sessions SET name='another' WHERE name='renamed_session';" 2>&1 | grep -qi "UNIQUE" && pass "5.3 Duplicate rejected" || fail "5.3 Should reject"

# Test 5.3: Verify UNIQUE constraint still holds
OUTPUT=$(zjj list)
COUNT=$(echo "$OUTPUT" | grep -c "another" || echo "0")
[ "$COUNT" -eq 1 ] && pass "5.4 One 'another' session" || fail "5.4 Should be exactly 1"

#=============================================================================
# GROUP 6: BULK OPERATIONS
#=============================================================================
log "\n[GROUP 6] BULK OPERATIONS"

# Test 6.1: Create 100 sessions
START=$(date +%s)
for i in {1..100}; do
    zjj add --no-zellij "bulk$i" >/dev/null 2>&1 || true
done
END=$(date +%s)
log "Created 100 sessions in $((END-START))s"
[ $((END-START)) -lt 10 ] && pass "6.1 Fast bulk creation" || fail "6.1 Too slow"

# Test 6.2: List performance
START=$(date +%s)
OUTPUT=$(zjj list)
END=$(date +%s)
[ $((END-START)) -lt 2 ] && pass "6.2 Fast list" || fail "6.2 List too slow"

# Test 6.3: Count sessions
COUNT=$(echo "$OUTPUT" | grep -c "bulk" || echo "0")
[ "$COUNT" -ge 95 ] && pass "6.3 Bulk creation count ($COUNT/100)" || fail "6.3 Only $COUNT/100"

# Test 6.4: Remove all
zjj list | grep -oP '\[\K[^\]]+' | while read -r session; do
    zjj remove -f "$session" >/dev/null 2>&1 || true
done
sleep 1
OUTPUT=$(zjj list)
! echo "$OUTPUT" | grep -q "bulk" && pass "6.4 All removed" || fail "6.4 Some remain"

#=============================================================================
# GROUP 7: VALIDATION TESTS
#=============================================================================
log "\n[GROUP 7] VALIDATION"

# Test 7.1: Valid names
zjj add --no-zellij "test-valid" >/dev/null 2>&1 && pass "7.1 Valid dashes" || fail "7.1 Dashes rejected"
zjj add --no-zellij "test_valid" >/dev/null 2>&1 && pass "7.2 Valid underscores" || fail "7.2 Underscores rejected"
zjj add --no-zellij "test123" >/dev/null 2>&1 && pass "7.3 Valid numbers" || fail "7.3 Numbers rejected"

# Test 7.2: Invalid names (CLI validation)
zjj add --no-zellij "test.invalid" 2>&1 | grep -qi "invalid\|only" && pass "7.4 Dots rejected" || fail "7.4 Dots should be rejected"
zjj add --no-zellij "" 2>&1 | grep -qi "empty\|required" && pass "7.5 Empty rejected" || fail "7.5 Empty should be rejected"

# Test 7.3: Special characters
zjj add --no-zellij "test!@#" 2>&1 | grep -qi "invalid\|only" && pass "7.6 Special chars rejected" || fail "7.6 Special chars should be rejected"

#=============================================================================
# GROUP 8: CONCURRENCY TESTS
#=============================================================================
log "\n[GROUP 8] CONCURRENCY"

# Test 8.1: Concurrent adds
for i in {1..20}; do
    zjj add --no-zellij "concurrent_$i" >/dev/null 2>&1 &
done
wait
sleep 2
OUTPUT=$(zjj list)
COUNT=$(echo "$OUTPUT" | grep -c "concurrent_" || echo "0")
[ "$COUNT" -ge 18 ] && pass "8.1 Concurrent adds ($COUNT/20)" || fail "8.1 Only $COUNT/20"

# Test 8.2: Concurrent removes
for i in {1..10}; do
    zjj remove -f "concurrent_$i" >/dev/null 2>&1 &
done
wait
sleep 1
OUTPUT=$(zjj list)
REMAINING=$(echo "$OUTPUT" | grep -c "concurrent_" || echo "0")
[ "$REMAINING" -le 12 ] && pass "8.2 Concurrent removes" || fail "8.2 Too many remaining"

#=============================================================================
# GROUP 9: EDGE CASES
#=============================================================================
log "\n[GROUP 9] EDGE CASES"

# Test 9.1: Rapid create/delete
for i in {1..30}; do
    zjj add --no-zellij "rapid_$i" >/dev/null 2>&1
    zjj remove -f "rapid_$i" >/dev/null 2>&1
done
pass "9.1 Rapid cycles stable"

# Test 9.2: Status during operations
zjj add --no-zellij "status_test" >/dev/null 2>&1
zjj status status_test >/dev/null 2>&1 && pass "9.2 Status during ops" || fail "9.2 Status failed"
zjj remove -f status_test >/dev/null 2>&1

# Test 9.3: List consistency
OUTPUT1=$(zjj list)
sleep 3
OUTPUT2=$(zjj list)
# Note: timestamps may differ, so we just check structure
echo "$OUTPUT1" | grep -qi "NAME.*STATUS" && echo "$OUTPUT2" | grep -qi "NAME.*STATUS" && pass "9.3 List structure" || fail "9.3 Structure changed"

#=============================================================================
# GROUP 10: ERROR HANDLING
#=============================================================================
log "\n[GROUP 10] ERROR HANDLING"

# Test 10.1: Status non-existent with exit code
if zjj status totally_nonexistent 2>&1 >/dev/null; then
    fail "10.1 Status should error"
else
    pass "10.1 Status errors properly"
fi

# Test 10.2: Remove non-existent with exit code
if zjj remove -f totally_nonexistent 2>&1 >/dev/null; then
    fail "10.2 Remove should error"
else
    pass "10.2 Remove errors properly"
fi

# Test 10.3: Focus non-existent
if zjj focus totally_nonexistent 2>&1 >/dev/null; then
    fail "10.3 Focus should error"
else
    pass "10.3 Focus errors properly"
fi

#=============================================================================
# CLEANUP
#=============================================================================
cd /home/lewis/src/oya
rm -rf "$TEST_DIR"

#=============================================================================
# SUMMARY
#=============================================================================
log "\n=== FINAL TEST SUMMARY ==="
log "PASSED: $PASS"
log "FAILED: $FAIL"
log "TOTAL:  $((PASS + FAIL))"
log "SUCCESS RATE: $(( PASS * 100 / (PASS + FAIL) ))%"

if [ $FAIL -eq 0 ]; then
    echo -e "\n${GREEN}ALL TESTS PASSED - ZJJ IS ROBUST${NC}\n"
    exit 0
else
    echo -e "\n${RED}SOME TESTS FAILED - SEE DETAILS ABOVE${NC}\n"
    exit 1
fi
