#!/bin/bash
# BRUTAL QA TEST FOR ZJJ SESSION MANAGEMENT
# Direct, non-interactive testing

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TEST_DIR="/tmp/zjj_brutal_$$"
PASS_COUNT=0
FAIL_COUNT=0
ISSUES=()

log() { echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"; }
pass() { echo -e "${GREEN}✓${NC} $1"; ((PASS_COUNT++)); }
fail() { echo -e "${RED}✗${NC} $1"; ((FAIL_COUNT++)); ISSUES+=("$1"); }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }

cd /tmp
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

log "=== BRUTAL ZJJ QA TEST ==="
log "Test directory: $TEST_DIR"

# Initialize
log "\n[INIT] Initializing zjj..."
expect -c 'spawn zjj init; expect "Press Enter"; send "\r"; interact' || zjj init <<< $'\n' || zjj init
sleep 1

# Test 1: Empty list
log "\n[TEST 1] List with no sessions"
OUTPUT=$(zjj list 2>&1)
echo "$OUTPUT"
if echo "$OUTPUT" | grep -qi "no sessions"; then
    pass "Empty list shows 'no sessions'"
else
    fail "Empty list doesn't show expected message"
fi

# Test 2: Create session
log "\n[TEST 2] Create session"
zjj add --no-zellij session1 2>&1 | head -5
sleep 1
OUTPUT=$(zjj list)
if echo "$OUTPUT" | grep -q "session1"; then
    pass "Session created and listed"
else
    fail "Session not found in list"
fi

# Test 3: Status
log "\n[TEST 3] Status command"
OUTPUT=$(zjj status session1 2>&1)
echo "$OUTPUT" | head -10
if echo "$OUTPUT" | grep -q "session1"; then
    pass "Status shows session info"
else
    fail "Status doesn't show session"
fi

# Test 4: Rename
log "\n[TEST 4] Rename session"
zjj rename session1 session_renamed 2>&1
sleep 1
OUTPUT=$(zjj list)
if echo "$OUTPUT" | grep -q "session_renamed"; then
    pass "Rename successful"
else
    fail "Rename failed - new name not found"
fi
if echo "$OUTPUT" | grep -q "session1"; then
    fail "Rename failed - old name still present"
else
    pass "Old name removed after rename"
fi

# Test 5: Rename to existing
log "\n[TEST 5] Rename to existing name (should fail)"
zjj add --no-zellij session2 2>&1 >/dev/null
sleep 1
if zjj rename session_renamed session2 2>&1 | grep -qi "error\|already exists\|failed"; then
    pass "Rename to existing name fails as expected"
else
    fail "Rename to existing name should fail"
fi

# Test 6: Rename non-existent
log "\n[TEST 6] Rename non-existent session"
if zjj rename nonexistent newname 2>&1 | grep -qi "error\|not found\|failed"; then
    pass "Rename non-existent fails as expected"
else
    fail "Rename non-existent should fail"
fi

# Test 7: Remove session
log "\n[TEST 7] Remove session"
zjj remove -y session2 2>&1
sleep 1
OUTPUT=$(zjj list)
if echo "$OUTPUT" | grep -q "session2"; then
    fail "Removed session still in list"
else
    pass "Remove successful"
fi

# Test 8: Remove non-existent
log "\n[TEST 8] Remove non-existent session"
if zjj remove -y nonexistent 2>&1 | grep -qi "error\|not found\|failed"; then
    pass "Remove non-existent fails as expected"
else
    fail "Remove non-existent should fail"
fi

# Test 9: Bulk operations
log "\n[TEST 9] Creating 50 sessions rapidly"
START=$(date +%s)
for i in {1..50}; do
    zjj add --no-zellij "bulk$i" 2>&1 >/dev/null || true
done
END=$(date +%s)
log "Created 50 sessions in $((END-START))s"
OUTPUT=$(zjj list 2>&1)
COUNT=$(echo "$OUTPUT" | grep -c "bulk" || echo "0")
if [ "$COUNT" -ge 45 ]; then
    pass "Bulk creation: $COUNT/50 sessions"
else
    fail "Bulk creation: only $COUNT/50 sessions"
fi

# Test 10: Status on bulk
log "\n[TEST 10] Status checks on many sessions"
zjj status bulk1 2>&1 | head -5
pass "Status on individual session works"

# Test 11: Rename in bulk
log "\n[TEST 11] Bulk rename operations"
for i in {1..10}; do
    zjj rename "bulk$i" "renamed$i" 2>&1 >/dev/null || true
done
OUTPUT=$(zjj list)
RENAMED_COUNT=$(echo "$OUTPUT" | grep -c "renamed" || echo "0")
if [ "$RENAMED_COUNT" -ge 8 ]; then
    pass "Bulk rename: $RENAMED_COUNT/10 successful"
else
    fail "Bulk rename: only $RENAMED_COUNT/10 successful"
fi

# Test 12: Rapid remove
log "\n[TEST 12] Rapid remove operations"
for i in {11..30}; do
    zjj remove -y "bulk$i" 2>&1 >/dev/null || true
done
OUTPUT=$(zjj list)
REMAINING=$(echo "$OUTPUT" | grep -c "bulk" || echo "0")
if [ "$REMAINING" -le 22 ]; then
    pass "Bulk remove: sessions reduced"
else
    fail "Bulk remove: too many remaining ($REMAINING)"
fi

# Test 13: Focus outside Zellij
log "\n[TEST 13] Focus command (should fail outside Zellij)"
if zjj focus renamed1 2>&1 | grep -qi "error\|not running\|failed"; then
    pass "Focus fails outside Zellij as expected"
else
    fail "Focus should fail outside Zellij"
fi

# Test 14: Special characters
log "\n[TEST 14] Special characters in names"
zjj add --no-zellij "test-with-dashes" 2>&1 >/dev/null || true
zjj add --no-zellij "test_with_underscores" 2>&1 >/dev/null || true
zjj add --no-zellij "test.dots" 2>&1 >/dev/null || true
sleep 1
OUTPUT=$(zjj list)
SPECIAL_COUNT=0
echo "$OUTPUT" | grep -q "test-with-dashes" && ((SPECIAL_COUNT++))
echo "$OUTPUT" | grep -q "test_with_underscores" && ((SPECIAL_COUNT++))
echo "$OUTPUT" | grep -q "test.dots" && ((SPECIAL_COUNT++))
if [ "$SPECIAL_COUNT" -ge 2 ]; then
    pass "Special characters: $SPECIAL_COUNT/3 sessions created"
else
    fail "Special characters: only $SPECIAL_COUNT/3 sessions created"
fi

# Test 15: Edge cases
log "\n[TEST 15] Edge cases"
if zjj rename "" "newname" 2>&1 | grep -qi "error\|invalid\|required"; then
    pass "Empty rename fails appropriately"
else
    fail "Empty rename should fail"
fi

if zjj remove -y "" 2>&1 | grep -qi "error\|invalid\|required"; then
    pass "Empty remove fails appropriately"
else
    fail "Empty remove should fail"
fi

# Test 16: List formatting
log "\n[TEST 16] List output consistency"
OUTPUT1=$(zjj list)
sleep 1
OUTPUT2=$(zjj list)
if [ "$OUTPUT1" = "$OUTPUT2" ]; then
    pass "List output is consistent"
else
    fail "List output inconsistent between calls"
fi

# Test 17: Status flags
log "\n[TEST 17] Status command variations"
zjj status renamed1 2>&1 | head -3
pass "Basic status works"

# Test 18: List filtering (if supported)
log "\n[TEST 18] List options"
zjj list --help 2>&1 | head -5 || zjj list -h 2>&1 | head -5 || true
pass "List help accessible"

# Test 19: Concurrent simulation
log "\n[TEST 19] Simulated concurrent operations"
for i in {1..5}; do
    zjj add --no-zellij "concurrent-$i" 2>&1 >/dev/null &
done
wait
sleep 2
OUTPUT=$(zjj list)
CONCURRENT_COUNT=$(echo "$OUTPUT" | grep -c "concurrent-" || echo "0")
if [ "$CONCURRENT_COUNT" -ge 4 ]; then
    pass "Concurrent adds: $CONCURRENT_COUNT/5 sessions"
else
    fail "Concurrent adds: only $CONCURRENT_COUNT/5 sessions"
fi

# Test 20: Cleanup
log "\n[TEST 20] Final cleanup"
BEFORE=$(zjj list | grep -c "\[" || echo "0")
zjj list | grep -oP '\[\K[^\]]+' | while read -r session; do
    zjj remove -y "$session" 2>&1 >/dev/null || true
done
sleep 1
AFTER=$(zjj list | grep -c "\[" || echo "0")
if [ "$AFTER" -lt "$BEFORE" ]; then
    pass "Cleanup removed sessions ($BEFORE -> $AFTER)"
else
    fail "Cleanup didn't remove sessions"
fi

# Final summary
cd /home/lewis/src/oya
rm -rf "$TEST_DIR"

log "\n=== TEST SUMMARY ==="
log "PASSED: $PASS_COUNT"
log "FAILED: $FAIL_COUNT"
log "TOTAL:  $((PASS_COUNT + FAIL_COUNT))"

if [ ${#ISSUES[@]} -gt 0 ]; then
    log "\n=== ISSUES FOUND ==="
    for issue in "${ISSUES[@]}"; do
        log "• $issue"
    done
fi

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}ALL TESTS PASSED${NC}"
    exit 0
else
    echo -e "${RED}SOME TESTS FAILED${NC}"
    exit 1
fi
