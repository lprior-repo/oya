#!/usr/bin/env bash
# Gatekeeper Agent - Continuous QA and Landing Loop
# This script implements the gatekeeper workflow for beads

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

# Check if bead is ready for gatekeeping
check_ready_for_gatekeeper() {
    local bead_id="$1"

    log_info "Checking bead $bead_id for gatekeeper readiness..."

    # Get bead status
    local status
    status=$(br show "$bead_id" 2>&1 | grep -oP '(?<=\[● P1 · )[A-Z_]+' || echo "UNKNOWN")

    # Only process in_progress beads
    if [[ "$status" != "IN_PROGRESS" ]]; then
        log_info "Bead $bead_id not in progress (status: $status), skipping"
        return 1
    fi

    # Check if it has the building label
    local labels
    labels=$(br show "$bead_id" 2>&1 | grep -oP '(?<=Labels: ).*' || echo "")

    if [[ "$labels" == *"stage:building"* ]]; then
        log_info "Bead $bead_id is in building stage - not ready for gatekeeping yet"
        return 1
    fi

    return 0
}

# QA: Check for forbidden patterns (unwrap, expect, panic)
qa_check_forbidden_patterns() {
    local bead_id="$1"

    log_info "Running forbidden pattern checks..."

    # Search for unwrap(), expect(), panic!, todo!, unimplemented!
    local forbidden
    forbidden=$(rg \
        --type rust \
        --glob '!*.spec.rs' \
        --glob '!*test*.rs' \
        --glob '!benches/*.rs' \
        --glob '!examples/*.rs' \
        '\b(unwrap\(\)|expect\(|panic!|todo!|unimplemented!)' \
        crates/ 2>/dev/null | wc -l)

    if [[ "$forbidden" -gt 0 ]]; then
        log_error "Found $forbidden instances of forbidden patterns (unwrap/expect/panic/todo/unimplemented)"
        echo "Forbidden patterns found:"
        rg \
            --type rust \
            --glob '!*.spec.rs' \
            --glob '!*test*.rs' \
            --glob '!benches/*.rs' \
            --glob '!examples/*.rs' \
            '\b(unwrap\(\)|expect\(|panic!|todo!|unimplemented!)' \
            crates/ 2>/dev/null
        return 1
    fi

    log_success "No forbidden patterns found"
    return 0
}

# QA: Run moon quick check
qa_run_moon_quick() {
    log_info "Running moon run :quick (format + lint check)..."

    if moon run :quick 2>&1; then
        log_success "Moon quick check passed"
        return 0
    else
        log_error "Moon quick check failed"
        return 1
    fi
}

# Claim bead for gatekeeping
claim_bead() {
    local bead_id="$1"

    log_info "Claiming bead $bead_id for gatekeeping..."

    if br update "$bead_id" --status in_progress --label "stage:gatekeeping" 2>&1; then
        log_success "Bead $bead_id claimed"
        return 0
    else
        log_error "Failed to claim bead $bead_id"
        return 1
    fi
}

# Complete bead gatekeeping and land
complete_bead() {
    local bead_id="$1"

    log_info "Completing bead $bead_id..."

    # Sync beads
    log_info "Syncing beads..."
    br sync --flush-only

    # Commit changes (if any)
    if jj status 2>&1 | grep -q "There are pending changes"; then
        log_info "Committing changes..."
        jj commit -m "gatekeeper: QA pass for $bead_id"
    fi

    # Push to remote
    log_info "Pushing to remote..."
    if jj git push 2>&1; then
        log_success "Changes pushed successfully"
    else
        log_error "Failed to push changes"
        return 1
    fi

    # Close bead
    log_info "Closing bead $bead_id..."
    if br close "$bead_id" 2>&1; then
        log_success "Bead $bead_id closed successfully"
        return 0
    else
        log_error "Failed to close bead $bead_id"
        return 1
    fi
}

# Main gatekeeper workflow for a single bead
process_bead() {
    local bead_id="$1"

    log_info "========================================"
    log_info "Processing bead: $bead_id"
    log_info "========================================"

    # Step 1: Claim the bead
    if ! claim_bead "$bead_id"; then
        log_error "Failed to claim bead, skipping"
        return 1
    fi

    # Step 2: Run QA checks
    log_info "Running QA checks..."

    # Check for forbidden patterns
    if ! qa_check_forbidden_patterns "$bead_id"; then
        log_error "QA check failed: forbidden patterns found"
        log_warning "Bead $bead_id needs fixes before landing"
        br update "$bead_id" --label "qa-failed:forbidden-patterns"
        return 1
    fi

    # Run moon quick check
    if ! qa_run_moon_quick; then
        log_error "QA check failed: moon quick check failed"
        log_warning "Bead $bead_id needs fixes before landing"
        br update "$bead_id" --label "qa-failed:moon-quick"
        return 1
    fi

    # Step 3: All checks passed - complete the bead
    if ! complete_bead "$bead_id"; then
        log_error "Failed to complete bead $bead_id"
        return 1
    fi

    log_success "Bead $bead_id successfully gated and landed!"
    return 0
}

# Main loop
main() {
    log_info "Starting Gatekeeper Agent loop..."
    log_info "Looking for beads labeled 'stage:ready-gatekeeper'..."

    while true; do
        # Look for beads ready for gatekeeping
        local ready_beads
        ready_beads=$(br list --label "stage:ready-gatekeeper" 2>&1 | grep -oP '(?<=◑ )[a-z0-9-]+' || true)

        if [[ -z "$ready_beads" ]]; then
            log_info "No beads ready for gatekeeping, waiting 30s..."
            sleep 30
            continue
        fi

        # Process each ready bead (one at a time for now)
        echo "$ready_beads" | while read -r bead_id; do
            if [[ -n "$bead_id" ]]; then
                process_bead "$bead_id"
            fi
        done

        # Small delay between processing cycles
        sleep 5
    done
}

# Run main function
main "$@"
