#!/bin/bash
# Quick test to demonstrate the bugs found in zjj bookmark commands

echo "========================================="
echo "ZJJ BOOKMARK BUG REPORT"
echo "========================================="
echo ""

# BUG 1: bookmark move allows moving non-existent bookmarks
echo "BUG #1: bookmark move allows moving non-existent bookmarks"
echo "----------------------------------------------------------"
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
jj git init > /dev/null 2>&1
jj config set --repo user.name "Test User" > /dev/null 2>&1
jj config set --repo user.email "test@example.com" > /dev/null 2>&1
echo "initial" > file.txt
jj commit -m "initial" > /dev/null 2>&1

REV=$(jj log --no-graph -r @ -T commit_id)
echo "Attempting to move non-existent bookmark 'does-not-exist' to revision $REV..."
zjj bookmark move --to "$REV" does-not-exist

EXIT_CODE=$?
echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo "❌ BUG CONFIRMED: Command succeeded (exit code 0)"
    echo "   Expected: Should fail with error"
    echo "   Actual: Successfully 'moved' non-existent bookmark"
else
    echo "✓ Bug fixed or not reproducible"
fi
echo ""

cd /
rm -rf "$TEST_DIR"

# BUG 2: bookmark list --json returns error
echo "BUG #2: bookmark list --json returns structured error"
echo "------------------------------------------------------"
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
jj git init > /dev/null 2>&1
jj config set --repo user.name "Test User" > /dev/null 2>&1
jj config set --repo user.email "test@example.com" > /dev/null 2>&1
echo "initial" > file.txt
jj commit -m "initial" > /dev/null 2>&1

echo "Running: zjj bookmark list --json"
echo ""
zjj bookmark list --json

EXIT_CODE=$?
echo ""
if [ $EXIT_CODE -ne 0 ]; then
    echo "❌ BUG CONFIRMED: --json flag causes error (exit code $EXIT_CODE)"
    echo "   Expected: Should succeed and return JSON array of bookmarks"
    echo "   Actual: Returns error about 'can only flatten structs and maps'"
else
    echo "✓ Bug fixed or not reproducible"
fi
echo ""

cd /
rm -rf "$TEST_DIR"

# BUG 3: bookmark --help may not work correctly
echo "BUG #3: bookmark --help exits with error"
echo "----------------------------------------"
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
jj git init > /dev/null 2>&1

echo "Running: zjj bookmark --help"
zjj bookmark --help
EXIT_CODE=$?
echo ""
echo "Exit code: $EXIT_CODE"
if [ $EXIT_CODE -ne 0 ]; then
    echo "❌ BUG CONFIRMED: --help exits with error code $EXIT_CODE"
    echo "   Expected: Exit code 0"
else
    echo "✓ Help works correctly"
fi
echo ""

cd /
rm -rf "$TEST_DIR"

echo "========================================="
echo "SUMMARY"
echo "========================================="
echo "3 potential bugs identified in zjj bookmark commands"
echo ""
echo "Recommendations:"
echo "1. Add validation to bookmark move to check if bookmark exists"
echo "2. Fix JSON serialization in bookmark list to handle arrays"
echo "3. Ensure --help flags exit with code 0 for all subcommands"
