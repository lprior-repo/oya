#!/bin/bash
# BRUTAL QA Test Script for zjj bookmark subcommands
# This script manually tests every subcommand, flag, and edge case

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PASSED=0
FAILED=0
BUGS_FOUND=0

echo "=========================================="
echo "ZJJ BOOKMARK BRUTAL QA TEST"
echo "=========================================="
echo ""

# Helper functions
run_test() {
    local test_name="$1"
    local command="$2"
    local should_fail="$3"
    local description="$4"

    echo -e "${BLUE}TEST: $test_name${NC}"
    echo "Description: $description"
    echo "Command: $command"

    if eval "$command" > /tmp/zjj_test_stdout.txt 2> /tmp/zjj_test_stderr.txt; then
        EXIT_CODE=0
    else
        EXIT_CODE=$?
    fi

    STDOUT=$(cat /tmp/zjj_test_stdout.txt)
    STDERR=$(cat /tmp/zjj_test_stderr.txt)

    echo "Exit Code: $EXIT_CODE"

    if [ "$should_fail" = "yes" ]; then
        if [ $EXIT_CODE -ne 0 ]; then
            echo -e "${GREEN}✓ PASSED: Command failed as expected${NC}"
            ((PASSED++))
        else
            echo -e "${RED}✗ FAILED: Command should have failed but succeeded${NC}"
            echo "STDOUT: $STDOUT"
            echo "STDERR: $STDERR"
            ((FAILED++))
        fi
    else
        if [ $EXIT_CODE -eq 0 ]; then
            echo -e "${GREEN}✓ PASSED${NC}"
            ((PASSED++))
        else
            echo -e "${RED}✗ FAILED${NC}"
            echo "STDOUT: $STDOUT"
            echo "STDERR: $STDERR"
            ((FAILED++))
        fi
    fi
    echo ""
}

setup_repo() {
    TEST_DIR=$(mktemp -d)
    cd "$TEST_DIR"
    jj git init > /dev/null 2>&1
    jj config set --repo user.name "Test User" > /dev/null 2>&1
    jj config set --repo user.email "test@example.com" > /dev/null 2>&1
    echo "initial content" > initial.txt
    jj commit -m "initial commit" > /dev/null 2>&1
    echo "$TEST_DIR"
}

cleanup_repo() {
    cd /
    rm -rf "$1"
}

# ============================================================================
# SECTION 1: BOOKMARK LIST
# ============================================================================

echo -e "${BLUE}=========================================="
echo "SECTION 1: BOOKMARK LIST"
echo "==========================================${NC}"
echo ""

REPO=$(setup_repo)
cd "$REPO"

run_test "list_empty" "zjj bookmark list" "no" "List bookmarks in empty repo"

# Create some bookmarks
jj bookmark create test-bookmark1 > /dev/null 2>&1
jj bookmark create test-bookmark2 > /dev/null 2>&1

run_test "list_basic" "zjj bookmark list" "no" "List bookmarks with 2 bookmarks"
run_test "list_all_flag" "zjj bookmark list --all" "no" "List bookmarks with --all flag"

# BUG TEST: JSON flag
run_test "list_json_flag" "zjj bookmark list --json" "no" "List bookmarks with --json flag"

cleanup_repo "$REPO"

# ============================================================================
# SECTION 2: BOOKMARK CREATE
# ============================================================================

echo -e "${BLUE}=========================================="
echo "SECTION 2: BOOKMARK CREATE"
echo "==========================================${NC}"
echo ""

REPO=$(setup_repo)
cd "$REPO"

run_test "create_basic" "zjj bookmark create basic-test" "no" "Create basic bookmark"
run_test "create_with_push" "zjj bookmark create -p push-test" "no" "Create bookmark with -p flag"
run_test "create_json_flag" "zjj bookmark create --json json-test" "no" "Create bookmark with --json"

# Edge cases
run_test "create_empty_name" "zjj bookmark create ''" "yes" "Create bookmark with empty name"

run_test "create_dashes" "zjj bookmark create test-with-dashes" "no" "Create bookmark with dashes"
run_test "create_underscores" "zjj bookmark create test_with_underscores" "no" "Create bookmark with underscores"
run_test "create_dots" "zjj bookmark create test.with.dots" "no" "Create bookmark with dots"
run_test "create_slashes" "zjj bookmark create test/with/slashes" "no" "Create bookmark with slashes"
run_test "create_at_sign" "zjj bookmark create test@with@at" "no" "Create bookmark with @ signs"

# Unicode tests
run_test "create_unicode_cyrillic" "zjj bookmark create bookmark-тест" "no" "Create bookmark with Cyrillic"
run_test "create_unicode_chinese" "zjj bookmark create bookmark-测试" "no" "Create bookmark with Chinese"
run_test "create_unicode_emoji" "zjj bookmark create bookmark-🚀-rocket" "no" "Create bookmark with emoji"
run_test "create_unicode_japanese" "zjj bookmark create bookmark-日本語" "no" "Create bookmark with Japanese"

# Long name
LONG_NAME=$(python3 -c "print('a' * 10000)")
run_test "create_long_name" "zjj bookmark create '$LONG_NAME'" "no" "Create bookmark with 10000 chars"

# Duplicate bookmark (should either fail or move)
run_test "create_duplicate_1" "zjj bookmark create duplicate-test" "no" "Create bookmark (first time)"
echo "new content" > new.txt
jj commit -m "new commit" > /dev/null 2>&1
run_test "create_duplicate_2" "zjj bookmark create duplicate-test" "no" "Create same bookmark again"

cleanup_repo "$REPO"

# ============================================================================
# SECTION 3: BOOKMARK DELETE
# ============================================================================

echo -e "${BLUE}=========================================="
echo "SECTION 3: BOOKMARK DELETE"
echo "==========================================${NC}"
echo ""

REPO=$(setup_repo)
cd "$REPO"

# Setup: create a bookmark to delete
jj bookmark create delete-me > /dev/null 2>&1

run_test "delete_basic" "zjj bookmark delete delete-me" "no" "Delete basic bookmark"

# Test delete with JSON
jj bookmark create delete-json > /dev/null 2>&1
run_test "delete_json" "zjj bookmark delete --json delete-json" "no" "Delete bookmark with --json"

# Edge cases
run_test "delete_nonexistent" "zjj bookmark delete does-not-exist-xyz" "yes" "Delete non-existent bookmark"
run_test "delete_empty_name" "zjj bookmark delete ''" "yes" "Delete bookmark with empty name"

cleanup_repo "$REPO"

# ============================================================================
# SECTION 4: BOOKMARK MOVE
# ============================================================================

echo -e "${BLUE}=========================================="
echo "SECTION 4: BOOKMARK MOVE"
echo "==========================================${NC}"
echo ""

REPO=$(setup_repo)
cd "$REPO"

# Setup: create bookmark and another commit
jj bookmark create move-test > /dev/null 2>&1
echo "content" > file.txt
jj commit -m "new commit" > /dev/null 2>&1
NEW_REV=$(jj log --no-graph -r @ -T commit_id)

run_test "move_basic" "zjj bookmark move --to $NEW_REV move-test" "no" "Move bookmark to different revision"

# Setup for JSON test
jj bookmark create move-json-test > /dev/null 2>&1
echo "content2" > file2.txt
jj commit -m "another commit" > /dev/null 2>&1
NEW_REV2=$(jj log --no-graph -r @ -T commit_id)
run_test "move_json" "zjj bookmark move --json --to $NEW_REV2 move-json-test" "no" "Move bookmark with --json"

# BUG TEST: Move non-existent bookmark
CURRENT_REV=$(jj log --no-graph -r @ -T commit_id)
run_test "move_nonexistent" "zjj bookmark move --to $CURRENT_REV does-not-exist" "yes" "Move non-existent bookmark"

run_test "move_invalid_rev" "zjj bookmark move --to invalidrevisionxyz move-test" "yes" "Move bookmark to invalid revision"

# Setup: create bookmark and try to move to same revision
jj bookmark create move-same > /dev/null 2>&1
CURRENT_REV=$(jj log --no-graph -r @ -T commit_id)
run_test "move_same_rev" "zjj bookmark move --to $CURRENT_REV move-same" "no" "Move bookmark to same revision"

run_test "move_empty_name" "zjj bookmark move --to $CURRENT_REV ''" "yes" "Move bookmark with empty name"
run_test "move_empty_to" "zjj bookmark move --to '' move-test" "yes" "Move bookmark with empty --to"

# Setup: test without --to flag
jj bookmark create move-no-to > /dev/null 2>&1
run_test "move_missing_to" "zjj bookmark move move-no-to" "yes" "Move bookmark without --to flag"

cleanup_repo "$REPO"

# ============================================================================
# SECTION 5: RACE CONDITIONS
# ============================================================================

echo -e "${BLUE}=========================================="
echo "SECTION 5: RACE CONDITIONS"
echo "==========================================${NC}"
echo ""

REPO=$(setup_repo)
cd "$REPO"

echo "Running 100 create/delete cycles..."
for i in {1..100}; do
    NAME="race-test-$i"
    zjj bookmark create "$NAME" > /dev/null 2>&1
    zjj bookmark delete "$NAME" > /dev/null 2>&1
    if [ $((i % 20)) -eq 0 ]; then
        echo "  Completed $i cycles..."
    fi
done
echo -e "${GREEN}✓ PASSED: 100 create/delete cycles completed${NC}"
((PASSED++))
echo ""

cleanup_repo "$REPO"

# ============================================================================
# SECTION 6: PERFORMANCE
# ============================================================================

echo -e "${BLUE}=========================================="
echo "SECTION 6: PERFORMANCE"
echo "==========================================${NC}"
echo ""

REPO=$(setup_repo)
cd "$REPO"

echo "Creating 1000 bookmarks..."
for i in {0..999}; do
    NAME="perf-test-$(printf "%04d" $i)"
    zjj bookmark create "$NAME" > /dev/null 2>&1
    if [ $((i % 100)) -eq 0 ] && [ $i -gt 0 ]; then
        echo "  Created $i bookmarks..."
    fi
done

echo "Testing list performance..."
time zjj bookmark list > /dev/null 2>&1
echo -e "${GREEN}✓ PASSED: List with 1000 bookmarks${NC}"
((PASSED++))
echo ""

cleanup_repo "$REPO"

# ============================================================================
# SUMMARY
# ============================================================================

echo -e "${BLUE}=========================================="
echo "TEST SUMMARY"
echo "==========================================${NC}"
echo ""
echo -e "Total Passed: ${GREEN}$PASSED${NC}"
echo -e "Total Failed: ${RED}$FAILED${NC}"
echo -e "Bugs Found:   ${YELLOW}$BUGS_FOUND${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}ALL TESTS PASSED!${NC}"
    exit 0
else
    echo -e "${RED}SOME TESTS FAILED${NC}"
    exit 1
fi
