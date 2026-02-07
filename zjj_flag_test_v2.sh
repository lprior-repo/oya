#!/bin/bash
# BRUTAL zjj Flag Combination Test v2
# QA Agent #17 - Testing EVERY flag combination

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

TEST_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0
ISSUES=()
CRITICAL_ISSUES=()

log_test() {
    local status=$1
    local desc=$2
    local details=$3
    local is_critical=$4

    TEST_COUNT=$((TEST_COUNT + 1))

    if [ "$status" = "PASS" ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
        echo -e "${GREEN}✓${NC} $desc"
    elif [ "$status" = "FAIL" ]; then
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo -e "${RED}✗${NC} $desc"
        ISSUES+=("$desc")
        if [ "$is_critical" = "true" ]; then
            CRITICAL_ISSUES+=("$desc: $details")
        fi
    elif [ "$status" = "WARN" ]; then
        WARN_COUNT=$((WARN_COUNT + 1))
        echo -e "${YELLOW}⚠${NC} $desc"
    else
        echo -e "${CYAN}ℹ${NC} $desc"
    fi

    if [ -n "$details" ]; then
        echo -e "  ${BLUE}→${NC} $details"
    fi
}

echo "=========================================="
echo "zjj Flag Combination BRUTAL Test v2"
echo "QA Agent #17"
echo "=========================================="
echo ""

# Test Suite 1: Basic Global Flags
echo "=== Test Suite 1: Basic Global Flags ==="
echo ""

# Test --version
if output=$(zjj --version 2>&1); then
    if [[ "$output" == *"zjj"* ]]; then
        log_test "PASS" "zjj --version works and outputs version" "$output"
    else
        log_test "FAIL" "zjj --version doesn't output version info" "$output" "true"
    fi
else
    log_test "FAIL" "zjj --version command failed" "$output" "true"
fi

# Test -V
if output=$(zjj -V 2>&1); then
    if [[ "$output" == *"zjj"* ]]; then
        log_test "PASS" "zjj -V works and outputs version" "$output"
    else
        log_test "FAIL" "zjj -V doesn't output version info" "$output" "true"
    fi
else
    log_test "FAIL" "zjj -V command failed" "$output" "true"
fi

# Test --help
if output=$(zjj --help 2>&1); then
    if [[ "$output" == *"Usage:"* ]] && [[ "$output" == *"Commands:"* ]]; then
        log_test "PASS" "zjj --help shows usage info" "Shows Usage and Commands sections"
    else
        log_test "WARN" "zjj --help output format unusual" "$output"
    fi
else
    log_test "FAIL" "zjj --help command failed" "$output" "true"
fi

# Test -h
if output=$(zjj -h 2>&1); then
    if [[ "$output" == *"Usage:"* ]]; then
        log_test "PASS" "zjj -h shows usage info" "Shows Usage section"
    else
        log_test "WARN" "zjj -h output format unusual" "$output"
    fi
else
    log_test "FAIL" "zjj -h command failed" "$output" "true"
fi

echo ""
echo "=== Test Suite 2: Command-Specific --help ==="
echo ""

# Test various commands with --help
for cmd in "list" "status" "whereami" "whoami" "add" "spawn" "done"; do
    if output=$(zjj $cmd --help 2>&1); then
        if [[ "$output" == *"Usage:"* ]] || [[ "$output" == *"usage"* ]]; then
            log_test "PASS" "zjj $cmd --help works"
        else
            log_test "WARN" "zjj $cmd --help has unusual output" "$output"
        fi
    else
        log_test "FAIL" "zjj $cmd --help failed" "$output" "false"
    fi
done

echo ""
echo "=== Test Suite 3: --on-success Flag ==="
echo ""

# Test --on-success with successful command
if output=$(zjj --on-success "echo SUCCESS_TRIGGERED" whereami 2>&1); then
    if [[ "$output" == *"SUCCESS_TRIGGERED"* ]]; then
        log_test "PASS" "--on-success executes on successful command"
    elif [[ "$output" == *"main"* ]] || [[ "$output" == *"workspace"* ]]; then
        log_test "WARN" "--on-success may not execute (command succeeded but callback not seen)" "$output"
    else
        log_test "WARN" "--on-success behavior unclear" "$output"
    fi
else
    log_test "FAIL" "--on-success with successful command failed" "$output" "false"
fi

# Test --on-success with failing command
if output=$(zjj --on-success "echo SHOULD_NOT_HAPPEN" invalid-command 2>&1); then
    log_test "FAIL" "--on-success triggered on failing command" "$output" "true"
else
    log_test "PASS" "--on-success not triggered on failing command"
fi

# Test --on-success without argument
if output=$(zjj --on-success 2>&1); then
    log_test "FAIL" "--on-success without argument should fail" "$output" "true"
else
    if [[ "$output" == *"requires a value"* ]] || [[ "$output" == *"unexpected"* ]]; then
        log_test "PASS" "--on-success without argument correctly fails"
    else
        log_test "WARN" "--on-success without argument failed unexpectedly" "$output"
    fi
fi

echo ""
echo "=== Test Suite 4: --on-failure Flag ==="
echo ""

# Test --on-failure with successful command (should NOT trigger)
if output=$(zjj --on-failure "echo FAILURE_TRIGGERED" whereami 2>&1); then
    if [[ "$output" == *"FAILURE_TRIGGERED"* ]]; then
        log_test "FAIL" "--on-failure triggered on successful command" "$output" "true"
    else
        log_test "PASS" "--on-failure not triggered on successful command"
    fi
else
    log_test "FAIL" "--on-failure with successful command failed" "$output" "false"
fi

# Test --on-failure with failing command (SHOULD trigger)
if output=$(zjj --on-failure "echo FAILURE_HANDLED" invalid-command 2>&1); then
    if [[ "$output" == *"FAILURE_HANDLED"* ]]; then
        log_test "PASS" "--on-failure executes on failing command"
    else
        log_test "WARN" "--on-failure behavior unclear" "$output"
    fi
else
    if [[ "$output" == *"unrecognized"* ]] || [[ "$output" == *"invalid"* ]]; then
        log_test "WARN" "--on-failure may not execute (command failed but callback not seen)" "$output"
    else
        log_test "FAIL" "--on-failure with failing command failed" "$output" "false"
    fi
fi

# Test --on-failure without argument
if output=$(zjj --on-failure 2>&1); then
    log_test "FAIL" "--on-failure without argument should fail" "$output" "true"
else
    if [[ "$output" == *"requires a value"* ]] || [[ "$output" == *"unexpected"* ]]; then
        log_test "PASS" "--on-failure without argument correctly fails"
    else
        log_test "WARN" "--on-failure without argument failed unexpectedly" "$output"
    fi
fi

echo ""
echo "=== Test Suite 5: Flag Ordering and Position ==="
echo ""

# Test global flag before command
if output=$(zjj --version list 2>&1); then
    log_test "WARN" "Global flag before command (--version list) may not work as expected" "$output"
else
    log_test "INFO" "Global flag before command (--version list) not allowed"
fi

# Test global flag after command (should fail for most commands)
if output=$(zjj list --version 2>&1); then
    log_test "WARN" "Global flag after command (list --version) unexpectedly works" "$output"
else
    if [[ "$output" == *"unexpected"* ]]; then
        log_test "PASS" "Global flag after command (list --version) correctly rejected"
    else
        log_test "INFO" "Global flag after command rejected"
    fi
fi

# Test --on-success before command
if output=$(zjj --on-success "echo OK" whereami 2>&1); then
    log_test "PASS" "--on-success before command works"
else
    log_test "FAIL" "--on-success before command fails" "$output" "false"
fi

# Test --on-success after command
if output=$(zjj whereami --on-success "echo OK" 2>&1); then
    log_test "PASS" "--on-success after command works"
else
    log_test "FAIL" "--on-success after command fails" "$output" "false"
fi

echo ""
echo "=== Test Suite 6: Invalid Inputs ==="
echo ""

# Test invalid command
if output=$(zjj invalid-command 2>&1); then
    log_test "FAIL" "Invalid command should fail" "$output" "true"
else
    if [[ "$output" == *"unrecognized"* ]]; then
        log_test "PASS" "Invalid command correctly fails with error"
    else
        log_test "INFO" "Invalid command fails"
    fi
fi

# Test invalid command with --help
if output=$(zjj invalid-command --help 2>&1); then
    log_test "FAIL" "Invalid command with --help should fail" "$output" "true"
else
    log_test "PASS" "Invalid command with --help correctly fails"
fi

# Test invalid flag
if output=$(zjj --invalid-flag 2>&1); then
    log_test "FAIL" "Invalid flag should fail" "$output" "true"
else
    if [[ "$output" == *"unexpected"* ]] || [[ "$output" == *"unrecognized"* ]]; then
        log_test "PASS" "Invalid flag correctly fails"
    else
        log_test "INFO" "Invalid flag fails"
    fi
fi

echo ""
echo "=== Test Suite 7: Empty and Whitespace Values ==="
echo ""

# Test --on-success with empty string
if output=$(zjj --on-success "" whereami 2>&1); then
    log_test "WARN" "--on-success with empty string allowed" "$output"
else
    log_test "PASS" "--on-success with empty string fails"
fi

# Test --on-failure with empty string
if output=$(zjj --on-failure "" whereami 2>&1); then
    log_test "WARN" "--on-failure with empty string allowed" "$output"
else
    log_test "PASS" "--on-failure with empty string fails"
fi

# Test --on-success with whitespace
if output=$(zjj --on-success "   " whereami 2>&1); then
    log_test "WARN" "--on-success with whitespace allowed" "$output"
else
    log_test "PASS" "--on-success with whitespace fails"
fi

echo ""
echo "=== Test Suite 8: Special Characters in Flag Values ==="
echo ""

# Test --on-success with single quotes
if output=$(zjj --on-success 'echo test' whereami 2>&1); then
    log_test "PASS" "Single quotes in --on-success work"
else
    log_test "WARN" "Single quotes in --on-success cause issues" "$output"
fi

# Test --on-success with double quotes
if output=$(zjj --on-success "echo test" whereami 2>&1); then
    log_test "PASS" "Double quotes in --on-success work"
else
    log_test "WARN" "Double quotes in --on-success cause issues" "$output"
fi

# Test --on-success with pipes
if output=$(zjj --on-success 'echo test | cat' whereami 2>&1); then
    log_test "PASS" "Pipes in --on-success accepted"
else
    log_test "WARN" "Pipes in --on-success cause issues" "$output"
fi

# Test --on-success with command substitution
if output=$(zjj --on-success 'echo $(date)' whereami 2>&1); then
    log_test "PASS" "Command substitution in --on-success accepted"
else
    log_test "WARN" "Command substitution in --on-success causes issues" "$output"
fi

echo ""
echo "=== Test Suite 9: Duplicate and Conflicting Flags ==="
echo ""

# Test duplicate --on-success
if output=$(zjj --on-success "echo A" --on-success "echo B" whereami 2>&1); then
    log_test "WARN" "Duplicate --on-success allowed (last one may win)" "$output"
else
    log_test "PASS" "Duplicate --on-success rejected"
fi

# Test duplicate --on-failure
if output=$(zjj --on-failure "echo A" --on-failure "echo B" whereami 2>&1); then
    log_test "WARN" "Duplicate --on-failure allowed (last one may win)" "$output"
else
    log_test "PASS" "Duplicate --on-failure rejected"
fi

# Test both --on-success and --on-failure
if output=$(zjj --on-success "echo OK" --on-failure "echo FAIL" whereami 2>&1); then
    log_test "PASS" "Both --on-success and --on-failure allowed together"
else
    log_test "WARN" "Both --on-success and --on-failure rejected" "$output"
fi

# Test --help and --version together
if output=$(zjj --help --version 2>&1); then
    log_test "WARN" "Both --help and --version allowed together" "$output"
else
    log_test "INFO" "Both --help and --version rejected"
fi

echo ""
echo "=== Test Suite 10: Command-Specific JSON Flag ==="
echo ""

# Test --json on commands that support it
for cmd in "list" "status" "whereami" "whoami"; do
    if output=$(zjj $cmd --json 2>&1); then
        if [[ "$output" == *"{"* ]] || [[ "$output" == *"["* ]] || [[ "$output" == *"$schema"* ]]; then
            log_test "PASS" "zjj $cmd --json outputs JSON format"
        elif [[ "$output" == *"error"* ]] || [[ "$output" == *"unrecognized"* ]]; then
            log_test "WARN" "zjj $cmd --json has error: $output"
        else
            log_test "INFO" "zjj $cmd --json output unclear"
        fi
    else
        log_test "INFO" "zjj $cmd --json command failed"
    fi
done

# Test --json on commands that might not support it
if output=$(zjj add --json 2>&1); then
    if [[ "$output" == *"{"* ]]; then
        log_test "PASS" "zjj add --json works"
    else
        log_test "INFO" "zjj add --json behavior unclear"
    fi
else
    log_test "INFO" "zjj add --json failed"
fi

echo ""
echo "=== Test Suite 11: Command-Specific Flags ==="
echo ""

# Test list command specific flags
if output=$(zjj list --all 2>&1); then
    log_test "PASS" "zjj list --all works"
else
    log_test "WARN" "zjj list --all failed" "$output"
fi

if output=$(zjj list --verbose 2>&1); then
    log_test "PASS" "zjj list --verbose works"
else
    log_test "WARN" "zjj list --verbose failed" "$output"
fi

if output=$(zjj list -v 2>&1); then
    log_test "PASS" "zjj list -v works"
else
    log_test "WARN" "zjj list -v failed" "$output"
fi

# Test status command specific flags
if output=$(zjj status --watch 2>&1); then
    # This might hang, so we'll timeout it
    timeout 1 bash -c "echo q | zjj status --watch" >&/dev/null 2>&1
    if [ $? -eq 124 ] || [ $? -eq 0 ]; then
        log_test "PASS" "zjj status --watch works (timed out as expected)"
    else
        log_test "WARN" "zjj status --watch behavior unusual"
    fi
else
    log_test "INFO" "zjj status --watch failed"
fi

echo ""
echo "=== Test Suite 12: Edge Cases ==="
echo ""

# Test very long --on-success command
long_cmd=$(python3 -c "print('echo ' + 'A' * 1000)")
if output=$(zjj --on-success "$long_cmd" whereami 2>&1); then
    log_test "PASS" "Very long --on-success command accepted"
else
    log_test "WARN" "Very long --on-success command rejected" "$output"
fi

# Test --on-success with newlines (should fail)
if output=$(zjj --on-success "echo line1
echo line2" whereami 2>&1); then
    log_test "WARN" "Newlines in --on-success allowed" "$output"
else
    log_test "PASS" "Newlines in --on-success rejected"
fi

# Test multiple commands with semicolons
if output=$(zjj --on-success "echo first; echo second" whereami 2>&1); then
    log_test "PASS" "Semicolon-separated commands in --on-success accepted"
else
    log_test "WARN" "Semicolon-separated commands in --on-success rejected" "$output"
fi

echo ""
echo "=== Test Suite 13: Command Argument Interactions ==="
echo ""

# Test whereami with various argument combinations
if output=$(zjj whereami 2>&1); then
    if [[ "$output" == *"main"* ]] || [[ "$output" == *"workspace"* ]]; then
        log_test "PASS" "zjj whereami works"
    else
        log_test "WARN" "zjj whereami output unexpected: $output"
    fi
else
    log_test "FAIL" "zjj whereami failed" "$output" "false"
fi

if output=$(zjj whoami 2>&1); then
    log_test "PASS" "zjj whoami works"
else
    log_test "WARN" "zjj whoami failed" "$output"
fi

# Test commands that require arguments
if output=$(zjj add 2>&1); then
    log_test "WARN" "zjj add without arguments should fail" "$output"
else
    if [[ "$output" == *"required"* ]] || [[ "$output" == *"expected"* ]]; then
        log_test "PASS" "zjj add without arguments correctly fails"
    else
        log_test "INFO" "zjj add without arguments fails"
    fi
fi

if output=$(zjj spawn 2>&1); then
    log_test "WARN" "zjj spawn without arguments should fail" "$output"
else
    if [[ "$output" == *"required"* ]] || [[ "$output" == *"expected"* ]]; then
        log_test "PASS" "zjj spawn without arguments correctly fails"
    else
        log_test "INFO" "zjj spawn without arguments fails"
    fi
fi

echo ""
echo "=========================================="
echo "TEST SUMMARY"
echo "=========================================="
echo "Total Tests: $TEST_COUNT"
echo -e "${GREEN}Passed:${NC} $PASS_COUNT"
echo -e "${YELLOW}Warnings:${NC} $WARN_COUNT"
echo -e "${RED}Failed:${NC} $FAIL_COUNT"
echo ""

if [ ${#ISSUES[@]} -gt 0 ]; then
    echo "=========================================="
    echo "ALL ISSUES FOUND"
    echo "=========================================="
    for i in "${!ISSUES[@]}"; do
        echo "$((i+1)). ${ISSUES[$i]}"
    done
    echo ""
fi

if [ ${#CRITICAL_ISSUES[@]} -gt 0 ]; then
    echo "=========================================="
    echo -e "${RED}CRITICAL ISSUES${NC}"
    echo "=========================================="
    for i in "${!CRITICAL_ISSUES[@]}"; do
        echo "$((i+1)). ${CRITICAL_ISSUES[$i]}"
    done
    echo ""
fi

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
elif [ ${#CRITICAL_ISSUES[@]} -eq 0 ]; then
    echo -e "${YELLOW}⚠ Some non-critical tests failed or warnings detected${NC}"
    exit 0
else
    echo -e "${RED}✗ Critical failures detected!${NC}"
    exit 1
fi
