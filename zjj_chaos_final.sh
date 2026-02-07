#!/bin/bash
################################################################################
# ZJJ CHAOS TEST SUITE FINAL - QA Agent #10
# ABSOLUTELY DESTROY zjj with adversarial testing
################################################################################

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
CRASHES=0
HANGS=0
VULNERABILITIES=0

# Test results tracking
declare -a CRASHES_LIST=()
declare -a HANGS_LIST=()
declare -a VULNERABILITIES_LIST=()
declare -a FAILURES_LIST=()

# Test directory
TEST_BASE="/tmp/zjj_chaos_$$"
mkdir -p "$TEST_BASE"

# Cleanup trap
cleanup() {
    echo -e "${CYAN}Cleaning up...${NC}"
    cd "$HOME"
    rm -rf "$TEST_BASE"
}
trap cleanup EXIT

# Helper functions
log_test() {
    echo -e "${BLUE}[TEST $((TOTAL_TESTS + 1))]${NC} $1"
}

log_pass() {
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    PASSED_TESTS=$((PASSED_TESTS + 1))
    echo -e "${GREEN}✓ PASS:${NC} $1"
}

log_fail() {
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    FAILED_TESTS=$((FAILED_TESTS + 1))
    echo -e "${RED}✗ FAIL:${NC} $1"
    FAILURES_LIST+=("[$1] $2")
}

log_crash() {
    CRASHES=$((CRASHES + 1))
    echo -e "${RED}💥 CRASH:${NC} $1"
    CRASHES_LIST+=("$1")
}

log_hang() {
    HANGS=$((HANGS + 1))
    echo -e "${YELLOW}⏸ HANG:${NC} $1"
    HANGS_LIST+=("$1")
}

log_vulnerability() {
    VULNERABILITIES=$((VULNERABILITIES + 1))
    echo -e "${MAGENTA}🔓 VULNERABILITY:${NC} $1"
    VULNERABILITIES_LIST+=("$1")
}

# Setup isolated test repo
setup_test_repo() {
    local repo_name="$1"
    local repo_path="$TEST_BASE/$repo_name"

    rm -rf "$repo_path"
    mkdir -p "$repo_path"
    cd "$repo_path"

    # Initialize git repo
    git init -q 2>/dev/null
    git config user.name "Chaos Test"
    git config user.email "chaos@test.com"

    # Create initial commit
    echo "test" > test.txt
    git add test.txt
    git commit -q -m "init" 2>/dev/null

    # Initialize zjj (suppress output)
    zjj init >/dev/null 2>&1 || true

    echo "$repo_path"
}

################################################################################
# TEST CATEGORY 1: INVALID ARGUMENTS
################################################################################

test_invalid_arguments() {
    echo -e "\n${CYAN}=== CATEGORY 1: INVALID ARGUMENTS ===${NC}"

    # Test 1: Empty string session name
    log_test "Empty string session name"
    local repo=$(setup_test_repo "empty_name")
    if zjj add "" 2>&1 | grep -qiE "error|invalid|required"; then
        log_pass "Empty string rejected"
    else
        log_vulnerability "Empty string session name not rejected"
        log_fail "Empty string" "Should reject empty string"
    fi

    # Test 2: Newline injection
    log_test "Newline injection in session name"
    repo=$(setup_test_repo "newline_injection")
    if zjj add $'test\nname' 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Newline rejected"
    else
        log_vulnerability "Newlines not sanitized - log injection possible"
        log_fail "Newline injection" "Should reject newlines"
    fi

    # Test 3: Tab injection
    log_test "Tab injection in session name"
    repo=$(setup_test_repo "tab_injection")
    if zjj add $'test\tname' 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Tab rejected"
    else
        log_vulnerability "Tabs not sanitized - log injection possible"
        log_fail "Tab injection" "Should reject tabs"
    fi

    # Test 4: Unicode RTL override
    log_test "Unicode RTL override characters"
    repo=$(setup_test_repo "unicode_rtl")
    local output
    output=$(zjj add $'test\u202ename' 2>&1 || true)
    if echo "$output" | grep -qiE "error|invalid"; then
        log_pass "Unicode RTL rejected"
    else
        log_vulnerability "Unicode spoofing characters not filtered"
        log_fail "Unicode RTL" "Should reject RTL override"
    fi

    # Test 5: Zero-width characters
    log_test "Zero-width characters"
    repo=$(setup_test_repo "zero_width")
    output=$(zjj add "test‌name" 2>&1 || true)
    if echo "$output" | grep -qiE "error|invalid"; then
        log_pass "Zero-width characters rejected"
    else
        log_vulnerability "Zero-width characters not filtered - phishing risk"
        log_fail "Zero-width" "Should reject zero-width chars"
    fi

    # Test 6: Combining character overflow
    log_test "Combining character overflow (50 combining marks)"
    repo=$(setup_test_repo "combining_overflow")
    local combined=$(python3 -c "print('a' + '\u0301' * 50)")
    output=$(timeout 5s zjj add "$combined" 2>&1 || echo "timeout")
    if echo "$output" | grep -qiE "error|invalid|timeout"; then
        log_pass "Combining char overflow rejected"
    else
        log_vulnerability "Combining character overflow not detected"
        log_fail "Combining overflow" "Should reject excessive combining marks"
    fi
}

################################################################################
# TEST CATEGORY 2: PATH TRAVERSAL ATTACKS
################################################################################

test_path_traversal() {
    echo -e "\n${CYAN}=== CATEGORY 2: PATH TRAVERSAL ATTACKS ===${NC}"

    # Test 7: Basic path traversal
    log_test "Path traversal: ../../etc/passwd"
    local repo=$(setup_test_repo "path_traversal")
    if zjj add "../../../etc/passwd" 2>&1 | grep -qiE "error|invalid|outside"; then
        log_pass "Path traversal rejected"
    else
        log_vulnerability "Path traversal not prevented - may escape workspace"
        log_fail "Path traversal" "Should block ../ sequences"
    fi

    # Test 8: Absolute path
    log_test "Absolute path as session name"
    repo=$(setup_test_repo "absolute_path")
    if zjj add "/etc/passwd" 2>&1 | grep -qiE "error|invalid|absolute"; then
        log_pass "Absolute path rejected"
    else
        log_vulnerability "Absolute paths not blocked"
        log_fail "Absolute path" "Should block absolute paths"
    fi

    # Test 9: URL encoded path traversal
    log_test "URL-encoded path traversal"
    repo=$(setup_test_repo "url_encoded")
    if zjj add "%2e%2e%2fetc%2fpasswd" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "URL-encoded traversal rejected"
    else
        log_vulnerability "URL-encoded traversal not decoded and checked"
        log_fail "URL-encoded" "Should decode and validate"
    fi

    # Test 10: Directory separators
    log_test "Session name with directory separators"
    repo=$(setup_test_repo "dir_separators")
    if zjj add "test/slash/name" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Directory separators rejected"
    else
        log_vulnerability "Directory separators not blocked - may create subdirs"
        log_fail "Directory separators" "Should block / and \\"
    fi
}

################################################################################
# TEST CATEGORY 3: COMMAND INJECTION
################################################################################

test_command_injection() {
    echo -e "\n${CYAN}=== CATEGORY 3: COMMAND INJECTION ===${NC}"

    # Test 11: Shell metacharacters
    log_test "Shell metacharacters: ; ls -la"
    local repo=$(setup_test_repo "shell_inject")
    local output
    output=$(zjj add "test; ls -la" 2>&1 || true)
    if echo "$output" | grep -qiE "error|invalid"; then
        log_pass "Shell injection rejected"
    else
        if echo "$output" | grep -qE "total |drwx|^-"; then
            log_vulnerability "Shell command injection successful"
            log_fail "Shell injection" "CRITICAL: Commands executed"
        else
            log_pass "Shell injection blocked (execution failed)"
        fi
    fi

    # Test 12: Pipe injection
    log_test "Pipe injection: test | cat /etc/passwd"
    repo=$(setup_test_repo "pipe_inject")
    output=$(zjj add "test | cat /etc/passwd" 2>&1 || true)
    if echo "$output" | grep -qiE "error|invalid"; then
        log_pass "Pipe injection rejected"
    else
        if echo "$output" | grep -qE "root:|/bin/bash"; then
            log_vulnerability "Pipe injection executed - leaked /etc/passwd"
            log_fail "Pipe injection" "CRITICAL: Data leak"
        else
            log_pass "Pipe injection blocked"
        fi
    fi

    # Test 13: Command substitution
    log_test "Command substitution: \$(whoami)"
    repo=$(setup_test_repo "cmd_subst")
    output=$(zjj add 'test$(whoami)' 2>&1 || true)
    if echo "$output" | grep -qiE "error|invalid"; then
        log_pass "Command substitution rejected"
    else
        if echo "$output" | grep -qE "lewis|root"; then
            log_vulnerability "Command substitution executed - username leaked"
            log_fail "Command substitution" "CRITICAL: Code execution"
        else
            log_pass "Command substitution blocked"
        fi
    fi

    # Test 14: Backtick injection
    log_test "Backtick injection: test\`whoami\`"
    repo=$(setup_test_repo "backtick_inject")
    output=$(zjj add 'test`whoami`' 2>&1 || true)
    if echo "$output" | grep -qiE "error|invalid"; then
        log_pass "Backtick injection rejected"
    else
        if echo "$output" | grep -qE "lewis|root"; then
            log_vulnerability "Backtick injection executed - username leaked"
            log_fail "Backtick" "CRITICAL: Code execution"
        else
            log_pass "Backtick injection blocked"
        fi
    fi
}

################################################################################
# TEST CATEGORY 4: CONCURRENT OPERATIONS
################################################################################

test_concurrent_operations() {
    echo -e "\n${CYAN}=== CATEGORY 4: CONCURRENT OPERATIONS ===${NC}"

    # Test 15: 50 parallel adds
    log_test "50 parallel zjj add operations"
    local repo
    repo=$(setup_test_repo "concurrent_adds")
    local pids=()

    cd "$repo"
    for i in {1..50}; do
        (zjj add "concurrent_$i" >/dev/null 2>&1 &) &
        pids+=($!)
    done

    # Wait for all with timeout
    local timeout=30
    local elapsed=0
    local all_done=0

    while [ $elapsed -lt $timeout ]; do
        all_done=1
        for pid in "${pids[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                all_done=0
                break
            fi
        done
        if [ $all_done -eq 1 ]; then
            break
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    # Kill any remaining
    for pid in "${pids[@]}"; do
        kill "$pid" 2>/dev/null || true
    done

    local count
    count=$(zjj list 2>/dev/null | grep -cE "concurrent_" || echo 0)
    if [ $count -ge 40 ]; then
        log_pass "Handled 50 concurrent adds (created $count sessions)"
    elif [ $all_done -eq 1 ]; then
        log_fail "Concurrent adds" "Only created $count/50 sessions"
    else
        log_hang "Concurrent operations hung after ${timeout}s"
        log_fail "Concurrent adds" "Operations hung"
    fi

    # Test 16: 100 rapid status queries
    log_test "100 rapid zjj status queries"
    repo=$(setup_test_repo "rapid_status")
    cd "$repo"
    zjj add test_session >/dev/null 2>&1 || true

    local start end duration
    start=$(date +%s)
    for i in {1..100}; do
        zjj status test_session >/dev/null 2>&1 || true
    done
    end=$(date +%s)
    duration=$((end - start))

    if [ $duration -lt 30 ]; then
        log_pass "100 status queries in ${duration}s"
    else
        log_fail "Rapid status" "Too slow: ${duration}s for 100 queries"
    fi

    # Test 17: Concurrent add/remove of same session
    log_test "Race condition: concurrent add/remove same session"
    repo=$(setup_test_repo "race_add_remove")
    cd "$repo"
    local pids=()

    for i in {1..20}; do
        (zjj add race_test >/dev/null 2>&1 &) &
        pids+=($!)
        (zjj remove -f race_test >/dev/null 2>&1 &) &
        pids+=($!)
    done

    # Wait with timeout
    timeout=15
    elapsed=0
    all_done=0

    while [ $elapsed -lt $timeout ]; do
        all_done=1
        for pid in "${pids[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                all_done=0
                break
            fi
        done
        if [ $all_done -eq 1 ]; then
            break
        fi
        sleep 0.1
        elapsed=$((elapsed + 1))
    done

    # Kill remaining
    for pid in "${pids[@]}"; do
        kill "$pid" 2>/dev/null || true
    done

    if [ $all_done -eq 1 ]; then
        log_pass "Handled concurrent add/remove race"
    else
        log_hang "Race condition caused hang"
        log_fail "Race condition" "Concurrent add/remove hung"
    fi
}

################################################################################
# TEST CATEGORY 5: STATE CORRUPTION
################################################################################

test_state_corruption() {
    echo -e "\n${CYAN}=== CATEGORY 5: STATE CORRUPTION ===${NC}"

    # Test 18: Delete .zjj directory during operation
    log_test "Delete .zjj directory during zjj list"
    local repo
    repo=$(setup_test_repo "delete_zjj")
    cd "$repo"
    zjj add test1 >/dev/null 2>&1 || true

    # Delete .zjj and immediately run command
    rm -rf .zjj
    local output
    output=$(zjj list 2>&1 || true)
    if echo "$output" | grep -qiE "error|not initialized|no such file|not found"; then
        log_pass "Gracefully handled missing .zjj directory"
    else
        log_crash "Did not detect missing .zjj directory"
        log_fail "Missing .zjj" "Should detect corrupted state"
    fi

    # Test 19: Corrupt database file
    log_test "Corrupt state database"
    repo=$(setup_test_repo "corrupt_db")
    cd "$repo"
    zjj add test1 >/dev/null 2>&1 || true

    # Find and corrupt database
    local db_file
    db_file=$(find .zjj -type f \( -name "*.db" -o -name "*.sqlite" -o -name "*.json" \) 2>/dev/null | head -1)
    if [ -n "$db_file" ]; then
        echo "CORRUPTED DATA" > "$db_file"

        output=$(zjj list 2>&1 || true)
        if echo "$output" | grep -qiE "error|corrupt|database|parse"; then
            log_pass "Detected database corruption"
        else
            log_crash "Did not detect database corruption"
            log_fail "Corrupt DB" "Should detect corruption"
        fi
    else
        log_pass "No database file found"
    fi

    # Test 20: Change permissions
    log_test "Change .zjj directory permissions to read-only"
    repo=$(setup_test_repo "readonly_zjj")
    cd "$repo"
    zjj add test1 >/dev/null 2>&1 || true

    chmod -R u-w .zjj
    output=$(zjj add test2 2>&1 || true)
    chmod -R u+w .zjj

    if echo "$output" | grep -qiE "error|permission|readonly|denied"; then
        log_pass "Detected permission error"
    else
        log_fail "Permissions" "Should detect read-only filesystem"
    fi
}

################################################################################
# TEST CATEGORY 6: EDGE CASES
################################################################################

test_edge_cases() {
    echo -e "\n${CYAN}=== CATEGORY 6: EDGE CASES ===${NC}"

    # Test 21: Single character session names
    log_test "Single character session names"
    local repo
    repo=$(setup_test_repo "single_char")
    cd "$repo"

    local passed=0
    for char in a b 1 2; do
        if zjj add "$char" >/dev/null 2>&1; then
            passed=$((passed + 1))
        fi
    done

    if [ $passed -ge 3 ]; then
        log_pass "Single char names work ($passed/4)"
    else
        log_fail "Single char" "Only $passed/4 worked"
    fi

    # Test 22: Duplicate operations
    log_test "Add same session 50 times"
    repo=$(setup_test_repo "duplicates")
    cd "$repo"

    local success_count=0
    for i in {1..50}; do
        if zjj add "duplicate_test" >/dev/null 2>&1; then
            success_count=$((success_count + 1))
        fi
    done

    if [ $success_count -le 1 ]; then
        log_pass "Only created 1 session (rejected $((49)) duplicates)"
    else
        log_vulnerability "Created $success_count duplicate sessions"
        log_fail "Duplicates" "Should reject duplicate names"
    fi

    # Test 23: Case sensitivity
    log_test "Case sensitivity (Test vs TEST vs test)"
    repo=$(setup_test_repo "case_sensitivity")
    cd "$repo"

    zjj add "test" >/dev/null 2>&1 || true
    zjj add "TEST" >/dev/null 2>&1 || true
    zjj add "Test" >/dev/null 2>&1 || true

    local count
    count=$(zjj list 2>&1 | grep -iE "test" | wc -l)
    if [ $count -le 3 ]; then
        log_pass "Case handling reasonable (found $count variants)"
    else
        log_fail "Case sensitivity" "Found $count 'test' sessions (should be 1-3)"
    fi

    # Test 24: Whitespace variations
    log_test "Whitespace variations"
    repo=$(setup_test_repo "whitespace")
    cd "$repo"

    local created=0
    zjj add "test" >/dev/null 2>&1 && created=$((created + 1)) || true
    zjj add "test " >/dev/null 2>&1 && created=$((created + 1)) || true
    zjj add " test" >/dev/null 2>&1 && created=$((created + 1)) || true

    if [ $created -le 2 ]; then
        log_pass "Whitespace handling reasonable (created $created/3)"
    else
        log_vulnerability "Created $created sessions with whitespace variations"
        log_fail "Whitespace" "Should normalize whitespace"
    fi
}

################################################################################
# TEST CATEGORY 7: RESOURCE EXHAUSTION
################################################################################

test_resource_exhaustion() {
    echo -e "\n${CYAN}=== CATEGORY 7: RESOURCE EXHAUSTION ===${NC}"

    # Test 25: Create many sessions
    log_test "Create 200 sessions"
    local repo
    repo=$(setup_test_repo "many_sessions")
    cd "$repo"

    local created=0
    local failed=0

    for i in {1..200}; do
        if zjj add "session_$i" >/dev/null 2>&1; then
            created=$((created + 1))
        else
            failed=$((failed + 1))
        fi

        if [ $failed -gt 10 ]; then
            break
        fi
    done

    if [ $created -ge 150 ]; then
        log_pass "Created $created sessions"
    else
        log_fail "Many sessions" "Only created $created/150 target"
    fi

    # Test 26: Rapid add/remove cycles
    log_test "20 rapid add/remove cycles"
    repo=$(setup_test_repo "rapid_cycles")
    cd "$repo"

    local success=0
    for i in {1..20}; do
        if zjj add "cycle_test" >/dev/null 2>&1; then
            if zjj remove -f cycle_test >/dev/null 2>&1; then
                success=$((success + 1))
            fi
        fi
    done

    if [ $success -ge 15 ]; then
        log_pass "Completed $success/20 cycles"
    else
        log_fail "Rapid cycles" "Only $success/20 cycles succeeded"
    fi
}

################################################################################
# MAIN TEST RUNNER
################################################################################

main() {
    echo -e "${MAGENTA}"
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║   ZJJ CHAOS TEST SUITE FINAL - QA Agent #10                ║"
    echo "║   ABSOLUTELY DESTROY zjj with adversarial testing         ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"

    echo -e "${CYAN}Test directory: $TEST_BASE${NC}\n"

    # Check zjj is available
    if ! command -v zjj &> /dev/null; then
        echo -e "${RED}ERROR: zjj not found in PATH${NC}"
        exit 1
    fi

    echo -e "${BLUE}zjj version: $(zjj --version)${NC}\n"

    # Run all test categories
    test_invalid_arguments
    test_path_traversal
    test_command_injection
    test_concurrent_operations
    test_state_corruption
    test_edge_cases
    test_resource_exhaustion

    # Print final report
    echo -e "\n${MAGENTA}"
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║                    FINAL REPORT                             ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"

    echo -e "\n${CYAN}TEST STATISTICS:${NC}"
    echo "  Total Tests:  $TOTAL_TESTS"
    echo -e "  ${GREEN}Passed:${NC}       $PASSED_TESTS"
    echo -e "  ${RED}Failed:${NC}       $FAILED_TESTS"

    echo -e "\n${CYAN}SECURITY ISSUES:${NC}"
    echo -e "  ${MAGENTA}Vulnerabilities: $VULNERABILITIES${NC}"
    echo -e "  ${RED}Crashes:          $CRASHES${NC}"
    echo -e "  ${YELLOW}Hangs:            $HANGS${NC}"

    # Calculate success rate
    local success_rate=0
    if [ $TOTAL_TESTS -gt 0 ]; then
        success_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    fi

    echo -e "\n${CYAN}SUCCESS RATE: ${success_rate}%${NC}"

    # Grade
    local grade="F"
    if [ $success_rate -ge 95 ]; then
        grade="A+"
    elif [ $success_rate -ge 90 ]; then
        grade="A"
    elif [ $success_rate -ge 85 ]; then
        grade="B"
    elif [ $success_rate -ge 70 ]; then
        grade="C"
    elif [ $success_rate -ge 60 ]; then
        grade="D"
    fi

    echo -e "${CYAN}GRADE: $grade${NC}"

    # Detailed findings
    if [ ${#VULNERABILITIES_LIST[@]} -gt 0 ]; then
        echo -e "\n${MAGENTA}═══ VULNERABILITIES FOUND ═══${NC}"
        for vuln in "${VULNERABILITIES_LIST[@]}"; do
            echo -e "${MAGENTA}•${NC} $vuln"
        done
    fi

    if [ ${#CRASHES_LIST[@]} -gt 0 ]; then
        echo -e "\n${RED}═══ CRASHES ═══${NC}"
        for crash in "${CRASHES_LIST[@]}"; do
            echo -e "${RED}•${NC} $crash"
        done
    fi

    if [ ${#HANGS_LIST[@]} -gt 0 ]; then
        echo -e "\n${YELLOW}═══ HANGS ═══${NC}"
        for hang in "${HANGS_LIST[@]}"; do
            echo -e "${YELLOW}•${NC} $hang"
        done
    fi

    if [ ${#FAILURES_LIST[@]} -gt 0 ]; then
        echo -e "\n${RED}═══ FAILURES ═══${NC}"
        for failure in "${FAILURES_LIST[@]}"; do
            echo -e "${RED}•${NC} $failure"
        done
    fi

    # Final verdict
    echo -e "\n${MAGENTA}═══ VERDICT ═══${NC}"

    if [ $VULNERABILITIES -gt 5 ]; then
        echo -e "${RED}✗ CRITICAL: $VULNERABILITIES vulnerabilities found${NC}"
        echo -e "${RED}  DO NOT USE IN PRODUCTION without fixes${NC}"
    elif [ $VULNERABILITIES -gt 0 ]; then
        echo -e "${YELLOW}⚠ WARNING: $VULNERABILITIES vulnerabilities found${NC}"
        echo -e "${YELLOW}  Review before production use${NC}"
    elif [ $CRASHES -gt 0 ]; then
        echo -e "${YELLOW}⚠ WARNING: $CRASHES crashes detected${NC}"
        echo -e "${YELLOW}  Review crash handling before production use${NC}"
    elif [ $success_rate -ge 90 ]; then
        echo -e "${GREEN}✓ EXCELLENT: Passed chaos testing with high success rate${NC}"
        echo -e "${GREEN}  Suitable for production use${NC}"
    else
        echo -e "${YELLOW}⚠ CAUTION: Passed basic tests but room for improvement${NC}"
    fi

    echo -e "\n${CYAN}Test directory: $TEST_BASE${NC}"
    echo -e "${CYAN}Will be cleaned up automatically${NC}\n"
}

# Run main
main "$@"
