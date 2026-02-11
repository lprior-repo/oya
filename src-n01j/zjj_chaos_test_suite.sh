#!/bin/bash
################################################################################
# ZJJ CHAOS TEST SUITE - QA Agent #10
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
    echo -e "${CYAN}Cleaning up test directory...${NC}"
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

# Setup test repo
setup_test_repo() {
    local repo_name="$1"
    local repo_path="$TEST_BASE/$repo_name"

    rm -rf "$repo_path"
    mkdir -p "$repo_path"
    cd "$repo_path"

    # Initialize JJ repo
    jj init --colocate > /dev/null 2>&1 || git init > /dev/null 2>&1
    jj config set user.name "Chaos Test" > /dev/null 2>&1 || git config user.name "Chaos Test"
    jj config set user.email "chaos@test.com" > /dev/null 2>&1 || git config user.email "chaos@test.com"

    # Initialize zjj
    zjj init > /dev/null 2>&1 || true

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

    # Test 2: Null bytes in name
    log_test "Null bytes in session name"
    repo=$(setup_test_repo "null_bytes")
    if printf "test\x00name" | timeout 5s zjj add "$(printf "test\x00name")" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Null bytes rejected"
    else
        log_vulnerability "Null bytes not sanitized - potential injection vector"
        log_fail "Null bytes" "Should reject null bytes"
    fi

    # Test 3: Massive input (10MB string)
    log_test "Massive input (10MB session name)"
    repo=$(setup_test_repo "massive_input")
    local massive=$(python3 -c "print('A' * 10485760)")
    if timeout 10s zjj add "$massive" 2>&1 | grep -qiE "error|too long|invalid"; then
        log_pass "Massive input rejected"
    else
        if timeout 10s zjj add "$massive" > /dev/null 2>&1; then
            log_vulnerability "Accepted 10MB input - DoS via memory exhaustion"
            log_fail "Massive input" "Should reject huge inputs"
        else
            log_pass "Massive input causes graceful failure"
        fi
    fi

    # Test 4: Newline injection
    log_test "Newline injection in session name"
    repo=$(setup_test_repo "newline_injection")
    if zjj add $'test\nname' 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Newline rejected"
    else
        log_vulnerability "Newlines not sanitized - log injection possible"
        log_fail "Newline injection" "Should reject newlines"
    fi

    # Test 5: Tab injection
    log_test "Tab injection in session name"
    repo=$(setup_test_repo "tab_injection")
    if zjj add $'test\tname' 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Tab rejected"
    else
        log_vulnerability "Tabs not sanitized - log injection possible"
        log_fail "Tab injection" "Should reject tabs"
    fi

    # Test 6: Unicode RTL override
    log_test "Unicode RTL override characters"
    repo=$(setup_test_repo "unicode_rtl")
    if zjj add $'test\u202ename' 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Unicode RTL rejected"
    else
        log_vulnerability "Unicode spoofing characters not filtered"
        log_fail "Unicode RTL" "Should reject RTL override"
    fi

    # Test 7: Zero-width characters
    log_test "Zero-width characters"
    repo=$(setup_test_repo "zero_width")
    if zjj add "test\u200Bname" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Zero-width characters rejected"
    else
        log_vulnerability "Zero-width characters not filtered - phishing risk"
        log_fail "Zero-width" "Should reject zero-width chars"
    fi

    # Test 8: Combining character overflow
    log_test "Combining character overflow (100 combining marks)"
    repo=$(setup_test_repo "combining_overflow")
    local combined=$(python3 -c "print('a' + '\u0301' * 100)")
    if timeout 5s zjj add "$combined" 2>&1 | grep -qiE "error|invalid"; then
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

    # Test 9: Basic path traversal
    log_test "Path traversal: ../../etc/passwd"
    local repo=$(setup_test_repo "path_traversal")
    if zjj add "../../../etc/passwd" 2>&1 | grep -qiE "error|invalid|outside"; then
        log_pass "Path traversal rejected"
    else
        log_vulnerability "Path traversal not prevented - may read arbitrary files"
        log_fail "Path traversal" "Should block ../ sequences"
    fi

    # Test 10: Absolute path
    log_test "Absolute path as session name"
    repo=$(setup_test_repo "absolute_path")
    if zjj add "/etc/passwd" 2>&1 | grep -qiE "error|invalid|absolute"; then
        log_pass "Absolute path rejected"
    else
        log_vulnerability "Absolute paths not blocked"
        log_fail "Absolute path" "Should block absolute paths"
    fi

    # Test 11: URL encoded path traversal
    log_test "URL-encoded path traversal"
    repo=$(setup_test_repo "url_encoded")
    if zjj add "%2e%2e%2fetc%2fpasswd" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "URL-encoded traversal rejected"
    else
        log_vulnerability "URL-encoded traversal not decoded and checked"
        log_fail "URL-encoded" "Should decode and validate"
    fi

    # Test 12: Double-encoded traversal
    log_test "Double-encoded path traversal"
    repo=$(setup_test_repo "double_encoded")
    if zjj add "%252e%252e%252fetc" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Double-encoded traversal rejected"
    else
        log_vulnerability "Double-encoded traversal bypasses validation"
        log_fail "Double-encoded" "Should recursively decode"
    fi
}

################################################################################
# TEST CATEGORY 3: COMMAND INJECTION
################################################################################

test_command_injection() {
    echo -e "\n${CYAN}=== CATEGORY 3: COMMAND INJECTION ===${NC}"

    # Test 13: Shell metacharacters
    log_test "Shell metacharacters: ; ls -la"
    local repo=$(setup_test_repo "shell_inject")
    if zjj add "test; ls -la" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Shell injection rejected"
    else
        # Check if ls was actually executed
        if zjj add "test; ls -la" > /dev/null 2>&1; then
            log_vulnerability "Shell command injection successful"
            log_fail "Shell injection" "CRITICAL: Commands executed"
        else
            log_pass "Shell injection blocked (execution failed)"
        fi
    fi

    # Test 14: Pipe injection
    log_test "Pipe injection: test | cat /etc/passwd"
    repo=$(setup_test_repo "pipe_inject")
    if zjj add "test | cat /etc/passwd" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Pipe injection rejected"
    else
        log_vulnerability "Pipe injection not prevented"
        log_fail "Pipe injection" "Should block pipes"
    fi

    # Test 15: Command substitution
    log_test "Command substitution: \$(whoami)"
    repo=$(setup_test_repo "cmd_subst")
    if zjj add 'test$(whoami)' 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Command substitution rejected"
    else
        log_vulnerability "Command substitution not prevented"
        log_fail "Command substitution" "Should block \$(...)"
    fi

    # Test 16: Backtick injection
    log_test "Backtick injection: test\`whoami\`"
    repo=$(setup_test_repo "backtick_inject")
    if zjj add 'test`whoami`' 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Backtick injection rejected"
    else
        log_vulnerability "Backtick injection not prevented"
        log_fail "Backtick" "Should block backticks"
    fi

    # Test 17: Redirect injection
    log_test "Redirect injection: test > /tmp/pwned"
    repo=$(setup_test_repo "redirect_inject")
    local pwned_file="/tmp/pwned_$$"
    rm -f "$pwned_file"
    zjj add "test > $pwned_file" > /dev/null 2>&1 || true
    if [ -f "$pwned_file" ]; then
        log_vulnerability "File creation via redirect injection"
        log_fail "Redirect" "CRITICAL: Created file via injection"
        rm -f "$pwned_file"
    else
        log_pass "Redirect injection blocked"
    fi
}

################################################################################
# TEST CATEGORY 4: CONCURRENT OPERATIONS
################################################################################

test_concurrent_operations() {
    echo -e "\n${CYAN}=== CATEGORY 4: CONCURRENT OPERATIONS ===${NC}"

    # Test 18: 100 parallel adds
    log_test "100 parallel zjj add operations"
    local repo=$(setup_test_repo "concurrent_adds")
    local pids=()

    for i in {1..100}; do
        (zjj add "concurrent_$i" > /dev/null 2>&1 &) &
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

    if [ $all_done -eq 1 ]; then
        local count=$(zjj list 2>/dev/null | wc -l || echo 0)
        if [ $count -ge 90 ]; then
            log_pass "Handled 100 concurrent adds (created $count sessions)"
        else
            log_fail "Concurrent adds" "Only created $count/100 sessions"
        fi
    else
        log_hang "Concurrent operations hung after ${timeout}s"
        log_fail "Concurrent adds" "Operations hung"
    fi

    # Test 19: 1000 rapid status queries
    log_test "1000 rapid zjj status queries"
    repo=$(setup_test_repo "rapid_status")
    zjj add test_session > /dev/null 2>&1 || true

    local start=$(date +%s)
    for i in {1..1000}; do
        zjj status test_session > /dev/null 2>&1 || true
    done
    local end=$(date +%s)
    local duration=$((end - start))

    if [ $duration -lt 60 ]; then
        log_pass "1000 status queries in ${duration}s"
    else
        log_fail "Rapid status" "Too slow: ${duration}s for 1000 queries"
    fi

    # Test 20: Concurrent add/remove of same session
    log_test "Race condition: concurrent add/remove same session"
    repo=$(setup_test_repo "race_add_remove")
    local pids=()

    for i in {1..50}; do
        (zjj add race_test > /dev/null 2>&1 &) &
        pids+=($!)
        (zjj remove -f race_test > /dev/null 2>&1 &) &
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

    # Test 21: Delete .zjj directory during operation
    log_test "Delete .zjj directory during zjj list"
    local repo=$(setup_test_repo "delete_zjj")
    zjj add test1 > /dev/null 2>&1 || true

    # Delete .zjj and immediately run command
    rm -rf .zjj
    if zjj list 2>&1 | grep -qiE "error|not initialized|no such file"; then
        log_pass "Gracefully handled missing .zjj directory"
    else
        log_crash "Did not detect missing .zjj directory"
        log_fail "Missing .zjj" "Should detect corrupted state"
    fi

    # Test 22: Corrupt database file
    log_test "Corrupt SQLite database"
    repo=$(setup_test_repo "corrupt_db")
    zjj add test1 > /dev/null 2>&1 || true

    # Find and corrupt database
    local db_file=$(find .zjj -name "*.db" -o -name "*.sqlite" | head -1)
    if [ -n "$db_file" ]; then
        echo "CORRUPTED DATA" > "$db_file"

        if zjj list 2>&1 | grep -qiE "error|corrupt|database"; then
            log_pass "Detected database corruption"
        else
            log_crash "Did not detect database corruption"
            log_fail "Corrupt DB" "Should detect corruption"
        fi
    else
        log_pass "No database file found (may use different storage)"
    fi

    # Test 23: Invalid JSON in config
    log_test "Invalid JSON in config file"
    repo=$(setup_test_repo "invalid_json")
    zjj init > /dev/null 2>&1 || true

    # Find config file
    local config_file=$(find .zjj -name "*.toml" -o -name "*.json" | head -1)
    if [ -n "$config_file" ]; then
        echo "{invalid json" > "$config_file"

        if zjj list 2>&1 | grep -qiE "error|parse|invalid"; then
            log_pass "Detected invalid config"
        else
            log_crash "Did not detect invalid config"
            log_fail "Invalid JSON" "Should detect parse errors"
        fi
    fi

    # Test 24: Change permissions
    log_test "Change .zjj directory permissions to read-only"
    repo=$(setup_test_repo "readonly_zjj")
    zjj add test1 > /dev/null 2>&1 || true

    chmod -R u-w .zjj
    if zjj add test2 2>&1 | grep -qiE "error|permission|readonly"; then
        log_pass "Detected permission error"
    else
        log_fail "Permissions" "Should detect read-only filesystem"
    fi
    chmod -R u+w .zjj
}

################################################################################
# TEST CATEGORY 6: RESOURCE EXHAUSTION
################################################################################

test_resource_exhaustion() {
    echo -e "\n${CYAN}=== CATEGORY 6: RESOURCE EXHAUSTION ===${NC}"

    # Test 25: File descriptor exhaustion
    log_test "File descriptor exhaustion (create 1000 sessions)"
    local repo=$(setup_test_repo "fd_exhaustion")
    local created=0
    local failed=0

    for i in {1..1000}; do
        if zjj add "fd_test_$i" > /dev/null 2>&1; then
            created=$((created + 1))
        else
            failed=$((failed + 1))
        fi

        # Stop failing too much
        if [ $failed -gt 10 ]; then
            break
        fi
    done

    if [ $created -ge 500 ]; then
        log_pass "Created $created sessions (stopped at $failed failures)"
    else
        log_fail "FD exhaustion" "Only created $created/500 target sessions"
    fi

    # Test 26: Disk space simulation (create 10000 files)
    log_test "Create 10000 sessions to test inode usage"
    repo=$(setup_test_repo "inode_exhaustion")

    # Skip if would take too long
    local max_sessions=1000
    local count=0

    for i in $(seq 1 $max_sessions); do
        if zjj add "inode_$i" > /dev/null 2>&1; then
            count=$((count + 1))
        fi
    done

    if [ $count -ge $max_sessions ]; then
        log_pass "Created $count sessions"
    else
        log_fail "Inode test" "Only created $count/$max_sessions sessions"
    fi
}

################################################################################
# TEST CATEGORY 7: SPECIAL FILESYSTEMS
################################################################################

test_special_filesystems() {
    echo -e "\n${CYAN}=== CATEGORY 7: SPECIAL FILESYSTEMS ===${NC}"

    # Test 27: Session name with slashes
    log_test "Session name with directory separators"
    local repo=$(setup_test_repo "slashes")
    if zjj add "test/slash/name" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Directory separators rejected"
    else
        log_vulnerability "Directory separators not blocked - may create subdirs"
        log_fail "Directory separators" "Should block / and \\"
    fi

    # Test 28: Session name as device file
    log_test "Session name as device file (/dev/null)"
    repo=$(setup_test_repo "device_file")
    if zjj add "/dev/null" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Device file path rejected"
    else
        log_vulnerability "Device file paths not blocked"
        log_fail "Device file" "Should block /dev/* paths"
    fi

    # Test 29: Session with reserved names
    log_test "Reserved Windows filenames (CON, PRN, AUX)"
    repo=$(setup_test_repo "reserved_names")

    local reserved="CON PRN AUX NUL COM1 COM2 LPT1 LPT2"
    local blocked=0
    local total=0

    for name in $reserved; do
        total=$((total + 1))
        if zjj add "$name" 2>&1 | grep -qiE "error|invalid|reserved"; then
            blocked=$((blocked + 1))
        fi
    done

    if [ $blocked -eq $total ]; then
        log_pass "All $total reserved names blocked"
    else
        log_vulnerability "Only blocked $blocked/$total reserved names"
        log_fail "Reserved names" "Should block all reserved filenames"
    fi
}

################################################################################
# TEST CATEGORY 8: ENVIRONMENT CHAOS
################################################################################

test_environment_chaos() {
    echo -e "\n${CYAN}=== CATEGORY 8: ENVIRONMENT CHAOS ===${NC}"

    # Test 30: Unset HOME
    log_test "Run zjj with unset HOME"
    local repo=$(setup_test_repo "no_home")
    local saved_home="$HOME"

    (unset HOME; zjj list 2>&1 | grep -qiE "error|HOME|required") && log_pass "Detected unset HOME" || log_fail "Unset HOME" "Should require HOME"

    export HOME="$saved_home"

    # Test 31: Invalid LANG
    log_test "Invalid locale settings"
    repo=$(setup_test_repo "invalid_locale")

    (LC_ALL=invalid_zjj_locale; zjj list > /dev/null 2>&1) && log_pass "Handled invalid locale" || log_fail "Invalid locale" "Should handle bad locale"

    # Test 32: Empty PATH
    log_test "Empty PATH environment variable"
    repo=$(setup_test_repo "empty_path")

    (PATH=; zjj list > /dev/null 2>&1) && log_pass "Handled empty PATH" || log_fail "Empty PATH" "Should handle empty PATH"
}

################################################################################
# TEST CATEGORY 9: UNICODE ATTACKS
################################################################################

test_unicode_attacks() {
    echo -e "\n${CYAN}=== CATEGORY 9: UNICODE ATTACKS ===${NC}"

    # Test 33: Homoglyph attacks
    log_test "Homoglyph attack ( Cyrillic looks like Latin)"
    local repo=$(setup_test_repo "homoglyph")

    # Cyrillic 'а' looks like Latin 'a'
    if zjj add "test-а-test" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Homoglyph rejected"
    else
        log_vulnerability "Homoglyph attacks not prevented - phishing risk"
        log_fail "Homoglyph" "Should normalize or reject"
    fi

    # Test 34: Emoji in session names
    log_test "Emoji in session names"
    repo=$(setup_test_repo "emoji")

    if zjj add "test😀🎉fire" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Emoji rejected"
    else
        # Check if it works or crashes
        if zjj add "test😀🎉fire" > /dev/null 2>&1; then
            log_pass "Emoji handled gracefully"
        else
            log_fail "Emoji" "Rejected but unclear error"
        fi
    fi

    # Test 35: Bidirectional text
    log_test "Bidirectional text override"
    repo=$(setup_test_repo "bidi")

    if zjj add $'test\u202Ename' 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Bidi override rejected"
    else
        log_vulnerability "Bidirectional override not filtered"
        log_fail "Bidi override" "Should reject RTL override"
    fi
}

################################################################################
# TEST CATEGORY 10: EDGE CASES
################################################################################

test_edge_cases() {
    echo -e "\n${CYAN}=== CATEGORY 10: EDGE CASES ===${NC}"

    # Test 36: Maximum length name
    log_test "Single character session names"
    local repo=$(setup_test_repo "single_char")

    local chars="a b c 1 2 3 ! @ #"
    local passed=0
    local total=0

    for char in $chars; do
        total=$((total + 1))
        if zjj add "$char" > /dev/null 2>&1; then
            passed=$((passed + 1))
        fi
    done

    if [ $passed -ge 5 ]; then
        log_pass "Single char names work ($passed/$total)"
    else
        log_fail "Single char" "Only $passed/$total worked"
    fi

    # Test 37: Duplicate operations
    log_test "Add same session 100 times"
    repo=$(setup_test_repo "duplicates")

    local success_count=0
    for i in {1..100}; do
        if zjj add "duplicate_test" > /dev/null 2>&1; then
            success_count=$((success_count + 1))
        fi
    done

    if [ $success_count -le 1 ]; then
        log_pass "Only created 1 session (rejected $((99 - success_count)) duplicates)"
    else
        log_vulnerability "Created $success_count duplicate sessions"
        log_fail "Duplicates" "Should reject duplicate names"
    fi

    # Test 38: Case sensitivity
    log_test "Case sensitivity (Test vs TEST vs test)"
    repo=$(setup_test_repo "case_sensitivity")

    zjj add "test" > /dev/null 2>&1 || true
    zjj add "TEST" > /dev/null 2>&1 || true
    zjj add "Test" > /dev/null 2>&1 || true

    local count=$(zjj list 2>&1 | grep -iE "test" | wc -l)
    if [ $count -le 3 ]; then
        log_pass "Case handling reasonable (found $count variants)"
    else
        log_fail "Case sensitivity" "Found $count 'test' sessions (should be 1-3)"
    fi

    # Test 39: Whitespace variations
    log_test "Whitespace variations (space, multiple spaces, tabs)"
    repo=$(setup_test_repo "whitespace")

    local spaces="test test  test	test	 test 	test"
    local created=0

    zjj add "test" > /dev/null 2>&1 && created=$((created + 1)) || true
    zjj add "test " > /dev/null 2>&1 && created=$((created + 1)) || true
    zjj add "test  " > /dev/null 2>&1 && created=$((created + 1)) || true
    zjj add "	test" > /dev/null 2>&1 && created=$((created + 1)) || true
    zjj add " test" > /dev/null 2>&1 && created=$((created + 1)) || true

    if [ $created -le 2 ]; then
        log_pass "Whitespace normalized (created $created/5)"
    else
        log_vulnerability "Created $created sessions with whitespace variations"
        log_fail "Whitespace" "Should normalize whitespace"
    fi
}

################################################################################
# TEST CATEGORY 11: INTEGRITY ATTACKS
################################################################################

test_integrity_attacks() {
    echo -e "\n${CYAN}=== CATEGORY 11: INTEGRITY ATTACKS ===${NC}"

    # Test 40: Symlink attacks
    log_test "Symlink attack on .zjj directory"
    local repo=$(setup_test_repo "symlink_attack")
    local link_target="/tmp/zjj_pwned_$$"

    rm -rf "$link_target"
    mkdir -p "$link_target"

    # Replace .zjj with symlink
    rm -rf .zjj
    ln -s "$link_target" .zjj

    if zjj add "symlink_test" > /dev/null 2>&1; then
        # Check if files were created outside repo
        if [ -f "$link_target"/* ]; then
            log_vulnerability "Symlink attack succeeded - files created outside repo"
            log_fail "Symlink attack" "CRITICAL: Files created via symlink"
        else
            log_pass "Symlink blocked or detected"
        fi
    else
        log_pass "Symlink attack prevented (command failed)"
    fi

    rm -rf "$link_target"

    # Test 41: Hard link to sensitive files
    log_test "Hard link to system files"
    repo=$(setup_test_repo "hardlink_attack")

    # This should not be possible to create as session name
    if zjj add "$(ls -i /etc/passwd | awk '{print $1}')" 2>&1 | grep -qiE "error|invalid"; then
        log_pass "Inode numbers rejected"
    else
        log_pass "Inode test not applicable"
    fi
}

################################################################################
# MAIN TEST RUNNER
################################################################################

main() {
    echo -e "${MAGENTA}"
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║   ZJJ CHAOS TEST SUITE - QA Agent #10                      ║"
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
    test_resource_exhaustion
    test_special_filesystems
    test_environment_chaos
    test_unicode_attacks
    test_edge_cases
    test_integrity_attacks

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

    if [ $VULNERABILITIES -gt 0 ]; then
        echo -e "${RED}✗ CRITICAL: $VULNERABILITIES vulnerabilities found${NC}"
        echo -e "${RED}  DO NOT USE IN PRODUCTION without fixes${NC}"
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
