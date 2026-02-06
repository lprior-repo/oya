#!/usr/bin/env bash
# Mutation testing script for OYA
# Usage:
#   ./scripts/mutants.sh              # Run on all packages
#   ./scripts/mutants.sh -p oya-core  # Run on specific package
#   ./scripts/mutants.sh --list       # List mutations without running
#   ./scripts/mutants.sh --diff main  # Only mutate changed files vs main

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Check cargo-mutants is installed
if ! command -v cargo-mutants &> /dev/null; then
    echo "Installing cargo-mutants..."
    cargo install cargo-mutants --locked
fi

# Default arguments
ARGS=("$@")

# If no arguments, add some sensible defaults
if [[ ${#ARGS[@]} -eq 0 ]]; then
    # Run with baseline check (ensures tests pass before mutating)
    ARGS=("--workspace" "--baseline" "run")
fi

echo "Running mutation tests..."
echo "  Config: mutants.toml"
echo "  Output: mutants.out/"
echo ""

cargo mutants "${ARGS[@]}"

# Show summary if output exists
if [[ -f "mutants.out/outcomes.json" ]]; then
    echo ""
    echo "=== MUTATION TESTING SUMMARY ==="
    if command -v jq &> /dev/null; then
        jq -r '
            "Total mutants: \(.total_mutants // 0)",
            "  Caught: \(.caught // 0)",
            "  Missed: \(.missed // 0)",
            "  Timeout: \(.timeout // 0)",
            "  Unviable: \(.unviable // 0)",
            "",
            "Mutation score: \(if .total_mutants > 0 then ((.caught // 0) * 100 / .total_mutants | floor) else 0 end)%"
        ' mutants.out/outcomes.json 2>/dev/null || echo "Run complete. Check mutants.out/ for details."
    else
        echo "Run complete. Check mutants.out/ for detailed results."
        echo "Install jq for summary: sudo pacman -S jq"
    fi
fi
