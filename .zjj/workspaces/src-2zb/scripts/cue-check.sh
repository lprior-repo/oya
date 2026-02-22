#!/usr/bin/env bash
# CUE Truth Gate: Validate implementation proofs against schemas
#
# This script enforces the contract that no bead closes without cue vet success.
# It validates implementation.cue files against their corresponding schemas.
#
# Exit codes:
#   0 - All proofs valid or no proofs to validate
#   1 - One or more proofs failed validation

set -euo pipefail

SCHEMAS_DIR=".beads/schemas"
IMPL_DIR=".beads/implementation"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
	echo -e "${GREEN}[CUE]${NC} $1"
}

log_warn() {
	echo -e "${YELLOW}[CUE]${NC} $1"
}

log_error() {
	echo -e "${RED}[CUE ERROR]${NC} $1"
}

# Check if cue is available
if ! command -v cue &>/dev/null; then
	log_error "cue command not found. Please install CUE: https://cuelang.org/docs/install/"
	exit 1
fi

# Check if implementation directory exists
if [[ ! -d "$IMPL_DIR" ]]; then
	log_info "No implementation proofs directory found at $IMPL_DIR"
	log_info "CUE truth gate passed (no proofs to validate)"
	exit 0
fi

# Find all implementation proof files
found_proofs=0
failed=0

for proof_file in "$IMPL_DIR"/*.cue; do
	# Skip if no files match
	[[ -e "$proof_file" ]] || continue

	found_proofs=$((found_proofs + 1))

	# Extract bead ID from filename (e.g., implementation-oya-20260220154435-2mulm3xo.cue)
	# or from the content
	bead_id=$(basename "$proof_file" .cue | sed 's/^implementation-//')

	# Find matching schema - try multiple patterns
	schema_file=""
	for pattern in "$SCHEMAS_DIR/${bead_id}.cue" "$SCHEMAS_DIR/oya-${bead_id}.cue"; do
		if [[ -f "$pattern" ]]; then
			schema_file="$pattern"
			break
		fi
	done

	if [[ -z "$schema_file" ]]; then
		# Try to extract bead_id from the proof file content
		extracted_id=$(grep -oP 'bead_id:\s*"\K[^"]+' "$proof_file" 2>/dev/null || true)
		if [[ -n "$extracted_id" ]]; then
			for pattern in "$SCHEMAS_DIR/${extracted_id}.cue" "$SCHEMAS_DIR/oya-${extracted_id}.cue"; do
				if [[ -f "$pattern" ]]; then
					schema_file="$pattern"
					break
				fi
			done
		fi
	fi

	if [[ -z "$schema_file" ]]; then
		log_error "No schema found for proof: $proof_file"
		failed=$((failed + 1))
		continue
	fi

	log_info "Validating: $(basename "$proof_file") against $(basename "$schema_file")"

	# Run cue vet
	if cue vet "$schema_file" "$proof_file" 2>&1; then
		log_info "PASSED: $(basename "$proof_file")"
	else
		log_error "FAILED: $(basename "$proof_file")"
		log_error "Schema: $schema_file"
		failed=$((failed + 1))
	fi
done

# Summary
echo ""
if [[ $found_proofs -eq 0 ]]; then
	log_info "No implementation proofs found to validate"
	log_info "CUE truth gate passed (no proofs required)"
	exit 0
fi

if [[ $failed -gt 0 ]]; then
	log_error "CUE validation failed: $failed of $found_proofs proofs failed"
	exit 1
fi

log_info "CUE truth gate passed: all $found_proofs proof(s) valid"
exit 0
