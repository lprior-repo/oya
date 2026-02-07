#!/bin/bash
# Edge Case and Race Condition Testing for zjj

set +e

echo "=== ZJJ EDGE CASE TEST SUITE ==="
echo "Time: $(date)"
echo ""

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
}

fail() {
    echo -e "${RED}✗ FAIL${NC}: $1"
}

warn() {
    echo -e "${YELLOW}⚠ WARN${NC}: $1"
}

# Test 1: Sync with uncommitted changes in workspace
echo "Test 1: Sync with uncommitted changes"
zjj add uncommitted-test --no-zellij > /dev/null 2>&1
echo "uncommitted change" > ../oya__workspaces/uncommitted-test/uncommitted.txt
RESULT=$(zjj sync uncommitted-test 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    pass "Sync with uncommitted changes succeeds"
    echo "  Output: $RESULT"
else
    fail "Sync with uncommitted changes failed" $EXIT_CODE
    echo "  Output: $RESULT"
fi

# Test 2: Sync identical workspace (no changes)
echo ""
echo "Test 2: Sync identical workspace (no changes)"
RESULT=$(zjj sync uncommitted-test 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    pass "Sync of identical workspace succeeds"
    echo "  Output: $RESULT"
else
    fail "Sync of identical workspace failed" $EXIT_CODE
fi

# Test 3: Rapid concurrent syncs
echo ""
echo "Test 3: Rapid concurrent syncs (race condition test)"
zjj add race-test-1 --no-zellij > /dev/null 2>&1
zjj add race-test-2 --no-zellij > /dev/null 2>&1
zjj add race-test-3 --no-zellij > /dev/null 2>&1

# Sync all three in quick succession
zjj sync race-test-1 > /dev/null 2>&1 &
PID1=$!
zjj sync race-test-2 > /dev/null 2>&1 &
PID2=$!
zjj sync race-test-3 > /dev/null 2>&1 &
PID3=$!

wait $PID1
EXIT1=$?
wait $PID2
EXIT2=$?
wait $PID3
EXIT3=$?

if [ $EXIT1 -eq 0 ] && [ $EXIT2 -eq 0 ] && [ $EXIT3 -eq 0 ]; then
    pass "Concurrent syncs succeeded without race conditions"
else
    fail "Concurrent syncs encountered race conditions"
    echo "  Exit codes: $EXIT1, $EXIT2, $EXIT3"
fi

# Test 4: Diff during sync
echo ""
echo "Test 4: Diff during sync operation"
zjj add diff-during-sync --no-zellij > /dev/null 2>&1
cd ../oya__workspaces/diff-during-sync
echo "change" > test.txt
jj commit -m "test" > /dev/null 2>&1
cd -

# Start sync in background and immediately request diff
zjj sync diff-during-sync > /dev/null 2>&1 &
SYNC_PID=$!
sleep 0.1  # Small delay to let sync start
RESULT=$(zjj diff diff-during-sync --stat 2>&1)
wait $SYNC_PID
SYNC_EXIT=$?

if [ $SYNC_EXIT -eq 0 ]; then
    pass "Diff during sync succeeded"
    echo "  Diff output: $RESULT"
else
    warn "Diff during sync had issues (exit: $SYNC_EXIT)"
fi

# Test 5: Workspace with detached HEAD state
echo ""
echo "Test 5: Workspace with detached HEAD"
zjj add detached-test --no-zellij > /dev/null 2>&1
cd ../oya__workspaces/detached-test
# Create a detached state by checking out an old commit
OLD_COMMIT=$(jj log --limit 1 --no-graph | head -1 | awk '{print $1}')
jj edit "$OLD_COMMIT" > /dev/null 2>&1
cd -

RESULT=$(zjj sync detached-test 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    pass "Sync with detached HEAD succeeded"
    echo "  Output: $RESULT"
else
    fail "Sync with detached HEAD failed" $EXIT_CODE
    echo "  Output: $RESULT"
fi

# Test 6: Switch between non-existent workspaces
echo ""
echo "Test 6: Switch operations"
RESULT=$(zjj switch non-existent-1 2>&1)
if echo "$RESULT" | grep -q "Not inside Zellij"; then
    pass "Switch properly fails outside Zellij"
else
    warn "Switch behavior unexpected: $RESULT"
fi

# Test 7: Diff with non-existent workspace
echo ""
echo "Test 7: Diff with non-existent workspace"
RESULT=$(zjj diff does-not-exist --stat 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ]; then
    pass "Diff of non-existent workspace properly fails"
else
    fail "Diff of non-existent workspace should fail" $EXIT_CODE
fi

# Test 8: whereami from workspace
echo ""
echo "Test 8: whereami from workspace directory"
cd ../oya__workspaces/race-test-1
RESULT=$(zjj whereami 2>&1)
cd -
if echo "$RESULT" | grep -q "workspace:"; then
    pass "whereami correctly detects workspace"
    echo "  Output: $RESULT"
else
    # This might be expected if whereami only works from main
    warn "whereami from workspace: $RESULT"
fi

# Test 9: Create workspace with same name (should fail)
echo ""
echo "Test 9: Create duplicate workspace name"
zjj add duplicate-name --no-zellij > /dev/null 2>&1
RESULT=$(zjj add duplicate-name --no-zellij 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ]; then
    pass "Duplicate workspace creation properly fails"
    echo "  Output: $RESULT"
else
    fail "Duplicate workspace creation should fail" $EXIT_CODE
fi

# Test 10: Sync workspace that doesn't exist
echo ""
echo "Test 10: Sync non-existent workspace"
RESULT=$(zjj sync definitely-does-not-exist 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ]; then
    pass "Sync of non-existent workspace properly fails"
    echo "  Output: $RESULT"
else
    fail "Sync of non-existent workspace should fail" $EXIT_CODE
fi

# Test 11: Check for database corruption recovery
echo ""
echo "Test 11: Database corruption handling"
# Backup database
cp .zjj/state.db .zjj/state.db.backup
# Corrupt database
echo "corruption" > .zjj/state.db
RESULT=$(zjj list 2>&1)
EXIT_CODE=$?

# Restore database
cp .zjj/state.db.backup .zjj/state.db
rm .zjj/state.db.backup

if [ $EXIT_CODE -ne 0 ]; then
    pass "Corrupted database properly detected"
    echo "  Output: $RESULT"
else
    warn "Corruption detection might not work: $EXIT_CODE"
fi

# Test 12: Remove workspace with uncommitted changes
echo ""
echo "Test 12: Remove workspace with uncommitted changes"
zjj add remove-test --no-zellij > /dev/null 2>&1
echo "change" > ../oya__workspaces/remove-test/change.txt
RESULT=$(zjj remove remove-test --force 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ] && [ ! -d "../oya__workspaces/remove-test" ]; then
    pass "Workspace with uncommitted changes removed successfully"
else
    fail "Workspace removal failed" $EXIT_CODE
    echo "  Output: $RESULT"
fi

# Cleanup
echo ""
echo "Cleanup: Removing test workspaces"
for workspace in uncommitted-test race-test-1 race-test-2 race-test-3 diff-during-sync detached-test duplicate-name; do
    zjj remove $workspace --force > /dev/null 2>&1
done
echo "Cleanup complete"

echo ""
echo "=== EDGE CASE TEST SUITE COMPLETE ==="
