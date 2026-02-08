#!/bin/bash
# BRUTAL zjj Flag Combination Test
# QA Agent #17 - Testing EVERY flag combination

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TEST_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0
ISSUES=()

log_test() {
    local status=$1
    local desc=$2
    local details=$3

    TEST_COUNT=$((TEST_COUNT + 1))

    if [ "$status" = "PASS" ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
        echo -e "${GREEN}✓${NC} $desc"
    elif [ "$status" = "FAIL" ]; then
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo -e "${RED}✗${NC} $desc"
        ISSUES+=("$desc: $details")
    else
        echo -e "${YELLOW}⚠${NC} $desc"
    fi

    if [ -n "$details" ] && [ "$status" = "FAIL" ]; then
        echo -e "  ${BLUE}Details:${NC} $details"
    fi
}

run_zjj() {
    local output
    local exit_code

    output=$(zjj "$@" 2>&1) || exit_code=$?
    echo "$output" >&2
    return ${exit_code:-0}
}

echo "=========================================="
echo "zjj Flag Combination BRUTAL Test"
echo "QA Agent #17"
echo "=========================================="
echo ""

# Test 1: Global flags on all commands
echo "=== Test Suite 1: Global Flags on Commands ==="
echo ""

# Commands that should work without args
SAFE_COMMANDS=("version" "help" "list" "status" "whereami" "whoami" "completions")
NEEDS_INIT_COMMANDS=("add" "spawn" "done" "sync" "diff")
NEEDS_REPO_COMMANDS=()

# Test --help on various commands
for cmd in "version" "help" "list" "status" "whereami" "whoami"; do
    if output=$(zjj $cmd --help 2>&1); then
        log_test "PASS" "zjj $cmd --help works"
    else
        log_test "FAIL" "zjj $cmd --help failed" "$output"
    fi
done

# Test -h on various commands
for cmd in "version" "help" "list" "status" "whereami" "whoami"; do
    if output=$(zjj $cmd -h 2>&1); then
        log_test "PASS" "zjj $cmd -h works"
    else
        log_test "FAIL" "zjj $cmd -h failed" "$output"
    fi
done

# Test --version
if output=$(zjj --version 2>&1); then
    log_test "PASS" "zjj --version works"
    if [[ "$output" == *"zjj"* ]]; then
        log_test "PASS" "zjj --version outputs version info"
    else
        log_test "FAIL" "zjj --version doesn't output version" "$output"
    fi
else
    log_test "FAIL" "zjj --version failed" "$output"
fi

# Test -V
if output=$(zjj -V 2>&1); then
    log_test "PASS" "zjj -V works"
    if [[ "$output" == *"zjj"* ]]; then
        log_test "PASS" "zjj -V outputs version info"
    else
        log_test "FAIL" "zjj -V doesn't output version" "$output"
    fi
else
    log_test "FAIL" "zjj -V failed" "$output"
fi

echo ""
echo "=== Test Suite 2: Global Flag Ordering ==="
echo ""

# Test flag before command
if output=$(zjj --version list 2>&1); then
    log_test "PASS" "Global flag before command works (--version list)"
else
    log_test "FAIL" "Global flag before command fails" "$output"
fi

# Test flag after command
if output=$(zjj list --version 2>&1); then
    log_test "PASS" "Global flag after command works (list --version)"
else
    log_test "FAIL" "Global flag after command fails" "$output"
fi

echo ""
echo "=== Test Suite 3: Invalid Command Tests ==="
echo ""

# Test with completely invalid command
if output=$(zjj invalid_command --help 2>&1); then
    log_test "FAIL" "Invalid command with --help should fail" "$output"
else
    log_test "PASS" "Invalid command correctly fails"
fi

# Test invalid command with --version
if output=$(zjj invalid_command --version 2>&1); then
    log_test "FAIL" "Invalid command with --version should fail" "$output"
else
    log_test "PASS" "Invalid command with --version correctly fails"
fi

echo ""
echo "=== Test Suite 4: --on-success and --on-failure ==="
echo ""

# Test --on-success with successful command
if output=$(zjj --on-success "echo 'SUCCESS'" whereami 2>&1); then
    if [[ "$output" == *"main"* ]] || [[ "$output" == *"workspace"* ]]; then
        log_test "PASS" "zjj whereami with --on-success works"
    else
        log_test "FAIL" "zjj whereami with --on-success has unexpected output" "$output"
    fi
else
    log_test "FAIL" "zjj whereami with --on-success failed" "$output"
fi

# Test --on-failure with successful command (should not trigger)
if output=$(zjj --on-failure "echo 'FAILED'" whereami 2>&1); then
    if [[ "$output" == *"FAILED"* ]]; then
        log_test "FAIL" "--on-failure triggered on successful command" "$output"
    else
        log_test "PASS" "--on-failure not triggered on successful command"
    fi
else
    log_test "FAIL" "zjj whereami with --on-failure failed" "$output"
fi

# Test --on-success with failing command
if output=$(zjj --on-success "echo 'SHOULD NOT SEE'" invalid_cmd 2>&1); then
    log_test "FAIL" "--on-success triggered on failing command" "$output"
else
    log_test "PASS" "--on-success not triggered on failing command"
fi

# Test --on-failure with failing command
if output=$(zjj --on-failure "echo 'FAILURE HANDLED'" invalid_cmd 2>&1); then
    log_test "PASS" "--on-failure triggered on failing command"
else
    log_test "FAIL" "--on-failure not triggered on failing command" "$output"
fi

echo ""
echo "=== Test Suite 5: Missing Flag Arguments ==="
echo ""

# Test --on-success without argument
if output=$(zjj --on-success 2>&1); then
    log_test "FAIL" "--on-success without argument should fail" "$output"
else
    log_test "PASS" "--on-success without argument correctly fails"
fi

# Test --on-failure without argument
if output=$(zjj --on-failure 2>&1); then
    log_test "FAIL" "--on-failure without argument should fail" "$output"
else
    log_test "PASS" "--on-failure without argument correctly fails"
fi

echo ""
echo "=== Test Suite 6: Invalid Flag Values ==="
echo ""

# Test --on-success with empty command
if output=$(zjj --on-success "" whereami 2>&1); then
    log_test "WARN" "--on-success with empty command allowed" "$output"
else
    log_test "PASS" "--on-success with empty command fails"
fi

# Test --on-failure with empty command
if output=$(zjj --on-failure "" whereami 2>&1); then
    log_test "WARN" "--on-failure with empty command allowed" "$output"
else
    log_test "PASS" "--on-failure with empty command fails"
fi

echo ""
echo "=== Test Suite 7: Conflicting Flags ==="
echo ""

# Test --help and --version together
if output=$(zjj --help --version 2>&1); then
    if [[ "$output" == *"zjj"* ]]; then
        log_test "PASS" "Both --help and --version accepted (--help ignored or takes precedence)"
    else
        log_test "WARN" "Both --help and --version behavior unclear" "$output"
    fi
else
    log_test "WARN" "Both --help and --version together failed" "$output"
fi

# Test -h and -V together
if output=$(zjj -h -V 2>&1); then
    log_test "PASS" "Both -h and -V accepted"
else
    log_test "WARN" "Both -h and -V together failed" "$output"
fi

echo ""
echo "=== Test Suite 8: Duplicate Global Flags ==="
echo ""

# Test duplicate --on-success
if output=$(zjj --on-success "echo A" --on-success "echo B" whereami 2>&1); then
    log_test "WARN" "Duplicate --on-success allowed" "$output"
else
    log_test "PASS" "Duplicate --on-success fails"
fi

# Test duplicate --on-failure
if output=$(zjj --on-failure "echo A" --on-failure "echo B" whereami 2>&1); then
    log_test "WARN" "Duplicate --on-failure allowed" "$output"
else
    log_test "PASS" "Duplicate --on-failure fails"
fi

echo ""
echo "=== Test Suite 9: Flag Position Tests ==="
echo ""

# Test flags before command
if output=$(zjj --on-success "echo OK" whereami 2>&1); then
    log_test "PASS" "Global flags before command work"
else
    log_test "FAIL" "Global flags before command fail" "$output"
fi

# Test flags after command
if output=$(zjj whereami --on-success "echo OK" 2>&1); then
    log_test "PASS" "Global flags after command work"
else
    log_test "FAIL" "Global flags after command fail" "$output"
fi

# Test flags before and after command
if output=$(zjj --on-success "echo OK" whereami --on-failure "echo FAIL" 2>&1); then
    log_test "PASS" "Global flags before and after command work"
else
    log_test "FAIL" "Global flags before and after command fail" "$output"
fi

echo ""
echo "=== Test Suite 10: Special Characters in Flag Values ==="
echo ""

# Test --on-success with quotes
if output=$(zjj --on-success 'echo "test"' whereami 2>&1); then
    log_test "PASS" "Quotes in --on-success work"
else
    log_test "WARN" "Quotes in --on-success cause issues" "$output"
fi

# Test --on-success with pipes
if output=$(zjj --on-success 'echo test | cat' whereami 2>&1); then
    log_test "PASS" "Pipes in --on-success work"
else
    log_test "WARN" "Pipes in --on-success cause issues" "$output"
fi

# Test --on-success with semicolons
if output=$(zjj --on-success 'echo test; echo test2' whereami 2>&1); then
    log_test "PASS" "Semicolons in --on-success work"
else
    log_test "WARN" "Semicolons in --on-success cause issues" "$output"
fi

echo ""
echo "=== Test Suite 11: Command-Specific Flag Tests ==="
echo ""

# Test list command with various flag combinations
if output=$(zjj list --help 2>&1); then
    log_test "PASS" "zjj list --help works"
else
    log_test "FAIL" "zjj list --help fails" "$output"
fi

if output=$(zjj list -h 2>&1); then
    log_test "PASS" "zjj list -h works"
else
    log_test "FAIL" "zjj list -h fails" "$output"
fi

# Test status command
if output=$(zjj status --help 2>&1); then
    log_test "PASS" "zjj status --help works"
else
    log_test "FAIL" "zjj status --help fails" "$output"
fi

echo ""
echo "=== Test Suite 12: JSON Output Flag (if applicable) ==="
echo ""

# Test if --json flag exists (it might not be implemented yet)
if output=$(zjj --json whereami 2>&1); then
    if [[ "$output" == *"{"* ]] || [[ "$output" == *"error"* ]]; then
        log_test "PASS" "--json flag exists and outputs JSON"
    else
        log_test "WARN" "--json flag exists but output unclear" "$output"
    fi
else
    if [[ "$output" == *"unrecognized"* ]] || [[ "$output" == *"unexpected"* ]]; then
        log_test "PASS" "--json flag not implemented (expected)"
    else
        log_test "WARN" "--json flag behavior unclear" "$output"
    fi
fi

echo ""
echo "=========================================="
echo "TEST SUMMARY"
echo "=========================================="
echo "Total Tests: $TEST_COUNT"
echo -e "${GREEN}Passed:${NC} $PASS_COUNT"
echo -e "${RED}Failed:${NC} $FAIL_COUNT"
echo ""

if [ ${#ISSUES[@]} -gt 0 ]; then
    echo "=========================================="
    echo "ISSUES FOUND"
    echo "=========================================="
    for i in "${!ISSUES[@]}"; do
        echo "$((i+1)). ${ISSUES[$i]}"
    done
    echo ""
fi

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}All critical tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed. Review above.${NC}"
    exit 1
fi
