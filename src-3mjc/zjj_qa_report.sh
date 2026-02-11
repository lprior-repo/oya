#!/bin/bash
# BRUTAL zjj Flag Combination Test - Final Report Generator
# QA Agent #17

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

TEST_RESULTS=()

run_test() {
    local desc="$1"
    local cmd="$2"
    local expect="$3"  # "success" or "fail"

    local output
    local exit_code

    output=$(eval "$cmd" 2>&1) || exit_code=$?

    if [ "$expect" = "success" ]; then
        if [ -z "$exit_code" ]; then
            TEST_RESULTS+=("PASS|$desc")
            echo -e "${GREEN}✓${NC} $desc"
        else
            TEST_RESULTS+=("FAIL|$desc")
            echo -e "${RED}✗${NC} $desc"
            echo -e "  ${BLUE}Exit:${NC} $exit_code"
            echo -e "  ${BLUE}Output:${NC} ${output:0:200}"
        fi
    else
        if [ -n "$exit_code" ]; then
            TEST_RESULTS+=("PASS|$desc")
            echo -e "${GREEN}✓${NC} $desc"
        else
            TEST_RESULTS+=("FAIL|$desc")
            echo -e "${RED}✗${NC} $desc"
            echo -e "  ${BLUE}Expected failure but succeeded${NC}"
        fi
    fi
}

echo -e "${BOLD}=========================================="
echo "zjj Flag Combination BRUTAL Test"
echo "QA Agent #17 - Final Report"
echo -e "==========================================${NC}"
echo ""

echo -e "${BOLD}Test Suite 1: Global Flags${NC}"
echo "-------------------"
run_test "zjj --version works" "zjj --version" "success"
run_test "zjj -V works" "zjj -V" "success"
run_test "zjj --help works" "zjj --help" "success"
run_test "zjj -h works" "zjj -h" "success"
echo ""

echo -e "${BOLD}Test Suite 2: Command --help Flags${NC}"
echo "-------------------------------"
run_test "zjj list --help works" "zjj list --help" "success"
run_test "zjj status --help works" "zjj status --help" "success"
run_test "zjj whereami --help works" "zjj whereami --help" "success"
run_test "zjj whoami --help works" "zjj whoami --help" "success"
run_test "zjj add --help works" "zjj add --help" "success"
run_test "zjj spawn --help works" "zjj spawn --help" "success"
run_test "zjj done --help works" "zjj done --help" "success"
echo ""

echo -e "${BOLD}Test Suite 3: --on-success Flag${NC}"
echo "---------------------------"
run_test "zjj --on-success before command works" "zjj --on-success 'echo TEST' whereami" "success"
run_test "zjj --on-success after command works" "zjj whereami --on-success 'echo TEST'" "success"
run_test "zjj --on-success without argument fails" "zjj --on-success" "fail"
run_test "zjj --on-success not triggered on fail" "zjj --on-success 'echo X' invalid-cmd" "fail"
echo ""

echo -e "${BOLD}Test Suite 4: --on-failure Flag${NC}"
echo "----------------------------"
run_test "zjj --on-failure with success command works" "zjj --on-failure 'echo TEST' whereami" "success"
run_test "zjj --on-failure after command works" "zjj whereami --on-failure 'echo TEST'" "success"
run_test "zjj --on-failure without argument fails" "zjj --on-failure" "fail"
echo ""

echo -e "${BOLD}Test Suite 5: Flag Ordering${NC}"
echo "-----------------------"
run_test "Global flag before command rejected" "zjj --version list" "fail"
run_test "Global flag after command rejected" "zjj list --version" "fail"
echo ""

echo -e "${BOLD}Test Suite 6: Invalid Inputs${NC}"
echo "------------------------"
run_test "Invalid command fails" "zjj invalid-command" "fail"
run_test "Invalid command with --help fails" "zjj invalid-command --help" "fail"
run_test "Invalid flag fails" "zjj --invalid-flag" "fail"
echo ""

echo -e "${BOLD}Test Suite 7: Empty Values${NC}"
echo "----------------------"
run_test "zjj --on-success with empty string" "zjj --on-success '' whereami" "success"
run_test "zjj --on-failure with empty string" "zjj --on-failure '' whereami" "success"
run_test "zjj --on-success with whitespace" "zjj --on-success '   ' whereami" "success"
echo ""

echo -e "${BOLD}Test Suite 8: Special Characters${NC}"
echo "----------------------------"
run_test "Single quotes in --on-success" "zjj --on-success 'echo test' whereami" "success"
run_test "Double quotes in --on-success" "zjj --on-success \"echo test\" whereami" "success"
run_test "Pipes in --on-success" "zjj --on-success 'echo test | cat' whereami" "success"
run_test "Command substitution in --on-success" "zjj --on-success 'echo \$(echo nested)' whereami" "success"
echo ""

echo -e "${BOLD}Test Suite 9: Duplicate Flags${NC}"
echo "------------------------"
run_test "Duplicate --on-success rejected" "zjj --on-success 'echo A' --on-success 'echo B' whereami" "fail"
run_test "Duplicate --on-failure rejected" "zjj --on-failure 'echo A' --on-failure 'echo B' whereami" "fail"
run_test "Both --on-success and --on-failure" "zjj --on-success 'echo A' --on-failure 'echo B' whereami" "success"
echo ""

echo -e "${BOLD}Test Suite 10: JSON Output${NC}"
echo "----------------------"
run_test "zjj list --json works" "zjj list --json" "success"
run_test "zjj status --json works" "zjj status --json" "success"
run_test "zjj whereami --json works" "zjj whereami --json" "success"
run_test "zjj whoami --json works" "zjj whoami --json" "success"
echo ""

echo -e "${BOLD}Test Suite 11: Command-Specific Flags${NC}"
echo "--------------------------------"
run_test "zjj list --all works" "zjj list --all" "success"
run_test "zjj list --verbose works" "zjj list --verbose" "success"
run_test "zjj list -v works" "zjj list -v" "success"
run_test "zjj list --bead filter" "zjj list --bead test-123" "success"
run_test "zjj list --agent filter" "zjj list --agent test-agent" "success"
run_test "zjj list --state filter" "zjj list --state active" "success"
echo ""

echo -e "${BOLD}Test Suite 12: Command Arguments${NC}"
echo "-----------------------------"
run_test "zjj whereami works" "zjj whereami" "success"
run_test "zjj whoami works" "zjj whoami" "success"
run_test "zjj add without args fails" "zjj add" "fail"
run_test "zjj spawn without args fails" "zjj spawn" "fail"
run_test "zjj done works" "zjj done --dry-run" "success"
echo ""

echo -e "${BOLD}Test Suite 13: Edge Cases${NC}"
echo "-----------------------"
run_test "Long command in --on-success" "zjj --on-success 'echo AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA' whereami" "success"
run_test "Semicolons in --on-success" "zjj --on-success 'echo first; echo second' whereami" "success"
run_test "Both --help and -v on list" "zjj list --help -v" "success"
echo ""

echo ""
echo -e "${BOLD}=========================================="
echo "TEST SUMMARY"
echo -e "==========================================${NC}"

TOTAL=${#TEST_RESULTS[@]}
PASS=0
FAIL=0

for result in "${TEST_RESULTS[@]}"; do
    status=${result%%|*}
    if [ "$status" = "PASS" ]; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
    fi
done

echo "Total Tests: $TOTAL"
echo -e "${GREEN}Passed:${NC} $PASS"
echo -e "${RED}Failed:${NC} $FAIL"
echo ""

if [ $FAIL -gt 0 ]; then
    echo -e "${BOLD}Failed Tests:${NC}"
    echo "----------------"
    for result in "${TEST_RESULTS[@]}"; do
        status=${result%%|*}
        desc=${result#*|}
        if [ "$status" = "FAIL" ]; then
            echo -e "${RED}✗${NC} $desc"
        fi
    done
    echo ""
fi

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}${BOLD}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}${BOLD}✗ Some tests failed${NC}"
    exit 1
fi
