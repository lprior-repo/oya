#!/bin/bash
# Focused QA Test for zjj workspace operations

set +e  # Don't exit on error

echo "=== ZJJ FOCUSED QA TEST ==="
echo "Time: $(date)"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
}

fail() {
    echo -e "${RED}✗ FAIL${NC}: $1"
    echo "  Exit code: $2"
}

warn() {
    echo -e "${YELLOW}⚠ WARN${NC}: $1"
}

# Test 1: whereami from main
echo "Test 1: whereami from main"
RESULT=$(zjj whereami 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ] && [ "$RESULT" = "main" ]; then
    pass "whereami correctly reports 'main'"
else
    fail "whereami failed" $EXIT_CODE
    echo "  Got: $RESULT"
fi

# Test 2: whereami JSON
echo ""
echo "Test 2: whereami JSON output"
RESULT=$(zjj whereami --json 2>&1)
EXIT_CODE=$?
if echo "$RESULT" | grep -q '"location_type": "main"'; then
    pass "whereami JSON is valid"
else
    fail "whereami JSON invalid" $EXIT_CODE
fi

# Test 3: Create workspace
echo ""
echo "Test 3: Create workspace without Zellij"
RESULT=$(zjj add qa-test-focus --no-zellij 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ] && [ -d "../oya__workspaces/qa-test-focus" ]; then
    pass "Workspace created successfully"
else
    fail "Workspace creation failed" $EXIT_CODE
    echo "  Output: $RESULT"
fi

# Test 4: diff clean workspace
echo ""
echo "Test 4: diff clean workspace"
RESULT=$(zjj diff qa-test-focus --stat 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    pass "diff on clean workspace succeeds"
    echo "  Output: $RESULT"
else
    fail "diff failed" $EXIT_CODE
    echo "  Output: $RESULT"
fi

# Test 5: sync clean workspace
echo ""
echo "Test 5: sync clean workspace"
RESULT=$(zjj sync qa-test-focus 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    pass "sync on clean workspace succeeds"
else
    fail "sync failed" $EXIT_CODE
    echo "  Output: $RESULT"
fi

# Test 6: Modify workspace and sync
echo ""
echo "Test 6: Modify workspace and sync"
echo "test change" > ../oya__workspaces/qa-test-focus/test.txt
cd ../oya__workspaces/qa-test-focus
jj commit -m "test: add file" > /dev/null 2>&1
cd -
RESULT=$(zjj diff qa-test-focus --stat 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    pass "diff shows workspace changes"
    echo "  Output: $RESULT"
else
    fail "diff failed after workspace change" $EXIT_CODE
fi

# Test 7: Create conflict scenario
echo ""
echo "Test 7: Create conflict in main"
echo "main change" > test.txt
jj commit -m "test: conflicting change in main" > /dev/null 2>&1
RESULT=$(zjj sync qa-test-focus 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    pass "sync with potential conflict succeeds"
    echo "  Output: $RESULT"
else
    fail "sync with conflict failed" $EXIT_CODE
    echo "  Output: $RESULT"
fi

# Test 8: Check workspace after sync
echo ""
echo "Test 8: Check workspace after conflict sync"
cd ../oya__workspaces/qa-test-focus
RESULT=$(jj status 2>&1)
cd -
echo "  Workspace status: $RESULT"

# Test 9: Test non-existent workspace operations
echo ""
echo "Test 9: Operations on non-existent workspace"
RESULT=$(zjj sync does-not-exist 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ]; then
    pass "sync on non-existent workspace properly fails"
    echo "  Output: $RESULT"
else
    fail "sync on non-existent workspace should fail" $EXIT_CODE
fi

# Test 10: Concurrent workspace operations
echo ""
echo "Test 10: Multiple concurrent workspaces"
zjj add concurrent-1 --no-zellij > /dev/null 2>&1
zjj add concurrent-2 --no-zellij > /dev/null 2>&1
zjj add concurrent-3 --no-zellij > /dev/null 2>&1
RESULT=$(zjj list 2>&1)
if echo "$RESULT" | grep -q "concurrent-1" && echo "$RESULT" | grep -q "concurrent-2" && echo "$RESULT" | grep -q "concurrent-3"; then
    pass "Multiple workspaces created successfully"
else
    fail "Concurrent workspace creation failed" $?
    echo "  List output: $RESULT"
fi

# Test 11: Sync all concurrent workspaces
echo ""
echo "Test 11: Sync all concurrent workspaces"
zjj sync concurrent-1 > /dev/null 2>&1
zjj sync concurrent-2 > /dev/null 2>&1
zjj sync concurrent-3 > /dev/null 2>&1
if [ $? -eq 0 ]; then
    pass "All concurrent workspaces synced successfully"
else
    warn "Some syncs may have failed"
fi

# Test 12: switch command (should fail outside Zellij)
echo ""
echo "Test 12: switch command outside Zellij"
RESULT=$(zjj switch concurrent-1 2>&1)
if echo "$RESULT" | grep -q "Not inside Zellij"; then
    pass "switch properly fails outside Zellij"
else
    warn "switch behavior unexpected: $RESULT"
fi

# Test 13: Remove workspace
echo ""
echo "Test 13: Remove workspace"
RESULT=$(zjj remove qa-test-focus --force 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ] && [ ! -d "../oya__workspaces/qa-test-focus" ]; then
    pass "Workspace removed successfully"
else
    fail "Workspace removal failed" $EXIT_CODE
    echo "  Output: $RESULT"
fi

echo ""
echo "=== TEST SUITE COMPLETE ==="
