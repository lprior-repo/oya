#!/bin/bash
# Brutal QA Test for zjj workspace operations
# Tests: sync, diff, switch, whereami

set -e

REPORT="/home/lewis/src/oya/zjj_brutal_qa_results.log"
echo "=== ZJJ BRUTAL QA TEST SUITE ===" > "$REPORT"
echo "Start time: $(date)" >> "$REPORT"
echo "" >> "$REPORT"

# Helper functions
log_cmd() {
    echo ">>> $1" >> "$REPORT"
    echo "Command: $1" | tee -a "$REPORT"
}

run_cmd() {
    local cmd="$1"
    local desc="$2"
    echo "" >> "$REPORT"
    echo "TEST: $desc" >> "$REPORT"
    log_cmd "$cmd"
    local start=$(date +%s%N)
    eval "$cmd" >> "$REPORT" 2>&1 || true
    local exit_code=$?
    local end=$(date +%s%N)
    local duration=$(( (end - start) / 1000000 ))
    echo "Exit code: $exit_code" >> "$REPORT"
    echo "Duration: ${duration}ms" >> "$REPORT"
    return $exit_code
}

check_output() {
    if [ -f "$REPORT" ]; then
        echo "✓ Output logged to $REPORT"
    fi
}

# Test 1: whereami from main
run_cmd "zjj whereami" "whereami from main"

# Test 2: whereami JSON output
run_cmd "zjj whereami --json" "whereami JSON format"

# Test 3: Create workspace without Zellij
run_cmd "zjj add qa-test-brutal --no-zellij" "Create workspace without Zellij"

# Test 4: diff on clean workspace
run_cmd "zjj diff qa-test-brutal --stat" "diff stat on clean workspace"

# Test 5: diff JSON on clean workspace
run_cmd "zjj diff qa-test-brutal --json" "diff JSON on clean workspace"

# Test 6: sync clean workspace
run_cmd "zjj sync qa-test-brutal" "sync clean workspace"

# Test 7: Create file in workspace
echo "test data in workspace" > ../oya__workspaces/qa-test-brutal/test_file.txt

# Test 8: Commit in workspace
run_cmd "cd ../oya__workspaces/qa-test-brutal && jj commit -m 'test: add file'" "Commit in workspace"

# Test 9: Check diff after commit
run_cmd "zjj diff qa-test-brutal --stat" "diff after workspace commit"

# Test 10: Create conflict - modify same file in main
echo "test data in main" > /home/lewis/src/oya/test_file.txt
run_cmd "jj commit -m 'test: conflicting change in main'" "Commit conflicting change in main"

# Test 11: sync with conflict
run_cmd "zjj sync qa-test-brutal" "sync with potential conflict"

# Test 12: Check workspace status after sync
run_cmd "cd ../oya__workspaces/qa-test-brutal && jj status" "Workspace status after sync"

# Test 13: Test switch to non-existent workspace
run_cmd "zjj switch non-existent-workspace 2>&1" "switch to non-existent workspace"

# Test 14: Test sync on non-existent workspace
run_cmd "zjj sync non-existent-workspace 2>&1" "sync non-existent workspace"

# Test 15: Test diff on non-existent workspace
run_cmd "zjj diff non-existent-workspace 2>&1" "diff non-existent workspace"

# Test 16: Create multiple concurrent workspaces
run_cmd "zjj add concurrent-1 --no-zellij" "Create concurrent workspace 1"
run_cmd "zjj add concurrent-2 --no-zellij" "Create concurrent workspace 2"
run_cmd "zjj add concurrent-3 --no-zellij" "Create concurrent workspace 3"

# Test 17: List all sessions
run_cmd "zjj list" "List all sessions"

# Test 18: Sync all workspaces
run_cmd "zjj sync concurrent-1" "Sync concurrent workspace 1"
run_cmd "zjj sync concurrent-2" "Sync concurrent workspace 2"
run_cmd "zjj sync concurrent-3" "Sync concurrent workspace 3"

# Test 19: Remove workspace
run_cmd "zjj remove qa-test-brutal --force" "Remove workspace"

# Test 20: Clean stale sessions
run_cmd "zjj clean --dry-run" "Clean stale sessions (dry-run)"

echo "" >> "$REPORT"
echo "=== TEST SUITE COMPLETE ===" >> "$REPORT"
echo "End time: $(date)" >> "$REPORT"

echo "Results logged to: $REPORT"
cat "$REPORT"
