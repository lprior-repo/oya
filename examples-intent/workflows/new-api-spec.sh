#!/usr/bin/env bash
#
# new-api-spec.sh
# Complete workflow for creating a new API specification from scratch
#
# Usage: ./new-api-spec.sh [output-file.cue]
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if gleam is available
if ! command -v gleam &> /dev/null; then
    error "Gleam is not installed. Please install Gleam first."
    exit 1
fi

# Default output file
OUTPUT_FILE="${1:-my-api.cue}"

info "Starting new API spec workflow..."
echo ""

# Step 1: Interactive Interview
info "Step 1: Interactive Interview"
info "Answer questions to define your API specification"
echo ""

# Start interview session
gleam run -- interview api

# Get the latest session ID
SESSION_ID=$(gleam run -- sessions --profile=api | grep "ID:" | head -1 | awk '{print $2}')

if [ -z "$SESSION_ID" ]; then
    error "Failed to create interview session"
    exit 1
fi

success "Interview session created: $SESSION_ID"
echo ""

# Step 2: Export to CUE spec
info "Step 2: Exporting interview to CUE spec"
gleam run -- export "$SESSION_ID" --output="$OUTPUT_FILE"
success "Spec exported to $OUTPUT_FILE"
echo ""

# Step 3: Validate the generated spec
info "Step 3: Validating spec syntax"
if gleam run -- validate "$OUTPUT_FILE"; then
    success "Spec is valid"
else
    error "Spec validation failed"
    exit 1
fi
echo ""

# Step 4: Quality analysis
info "Step 4: Analyzing spec quality"
QUALITY_JSON=$(gleam run -- quality "$OUTPUT_FILE")
QUALITY_SCORE=$(echo "$QUALITY_JSON" | jq -r '.data.overall_score // 0')

echo "Quality Score: $QUALITY_SCORE/100"

if (( $(echo "$QUALITY_SCORE >= 80" | bc -l) )); then
    success "Quality score is good (>= 80)"
elif (( $(echo "$QUALITY_SCORE >= 60" | bc -l) )); then
    warning "Quality score is acceptable (>= 60)"
else
    warning "Quality score is low (< 60) - consider improvements"
fi
echo ""

# Step 5: Find gaps
info "Step 5: Identifying coverage gaps"
GAPS_JSON=$(gleam run -- gaps "$OUTPUT_FILE")
GAP_COUNT=$(echo "$GAPS_JSON" | jq -r '.data.gap_count // 0')

if [ "$GAP_COUNT" -eq 0 ]; then
    success "No gaps found - excellent coverage!"
else
    warning "Found $GAP_COUNT gaps"
    echo "$GAPS_JSON" | jq -r '.data.gaps[] | "  - [\(.severity)] \(.description)"'
fi
echo ""

# Step 6: OWASP coverage check
info "Step 6: Checking OWASP Top 10 coverage"
COVERAGE_JSON=$(gleam run -- coverage "$OUTPUT_FILE")
COVERAGE_SCORE=$(echo "$COVERAGE_JSON" | jq -r '.data.score // 0')

echo "OWASP Coverage Score: $COVERAGE_SCORE/100"

if (( $(echo "$COVERAGE_SCORE >= 70" | bc -l) )); then
    success "Good security coverage"
else
    warning "Consider adding more security behaviors"
fi
echo ""

# Step 7: Inversion analysis (failure modes)
info "Step 7: Analyzing failure modes"
INVERT_JSON=$(gleam run -- invert "$OUTPUT_FILE")
FAILURE_COUNT=$(echo "$INVERT_JSON" | jq -r '.data.failure_count // 0')

echo "Identified $FAILURE_COUNT potential failure modes"
echo "Top 3 critical failure scenarios:"
echo "$INVERT_JSON" | jq -r '.data.failure_modes[] | select(.severity == "critical") | "  - \(.scenario)"' | head -3
echo ""

# Step 8: Get improvement suggestions
info "Step 8: Getting improvement suggestions"
echo ""
gleam run -- improve "$OUTPUT_FILE"
echo ""

# Step 9: Generate work beads
info "Step 9: Generating work items (beads)"
BEADS_JSON=$(gleam run -- beads "$SESSION_ID")
BEAD_COUNT=$(echo "$BEADS_JSON" | jq -r '.data.bead_count // 0')
TOTAL_MINUTES=$(echo "$BEADS_JSON" | jq -r '.data.total_minutes // 0')

success "Generated $BEAD_COUNT work items (estimated: $TOTAL_MINUTES minutes)"

# Save beads to file
BEADS_FILE="${OUTPUT_FILE%.cue}-beads.json"
echo "$BEADS_JSON" > "$BEADS_FILE"
info "Beads saved to $BEADS_FILE"
echo ""

# Step 10: Generate AI prompts
info "Step 10: Generating AI implementation prompts"
PROMPTS_JSON=$(gleam run -- prompt "$SESSION_ID")
PROMPT_COUNT=$(echo "$PROMPTS_JSON" | jq -r '.data.prompts | length')

# Save prompts to file
PROMPTS_FILE="${OUTPUT_FILE%.cue}-prompts.json"
echo "$PROMPTS_JSON" > "$PROMPTS_FILE"
success "Generated $PROMPT_COUNT AI prompts"
info "Prompts saved to $PROMPTS_FILE"
echo ""

# Summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
success "Workflow complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Generated files:"
echo "  1. Spec:    $OUTPUT_FILE"
echo "  2. Beads:   $BEADS_FILE"
echo "  3. Prompts: $PROMPTS_FILE"
echo ""
echo "Metrics:"
echo "  - Quality Score:     $QUALITY_SCORE/100"
echo "  - Coverage Score:    $COVERAGE_SCORE/100"
echo "  - Gaps Found:        $GAP_COUNT"
echo "  - Failure Modes:     $FAILURE_COUNT"
echo "  - Work Items:        $BEAD_COUNT"
echo "  - Estimated Effort:  $TOTAL_MINUTES minutes"
echo ""

# Next steps
echo "Next steps:"
echo ""
echo "  1. Review improvements:"
echo "     gleam run -- doctor $OUTPUT_FILE"
echo ""
echo "  2. View work plan:"
echo "     gleam run -- plan $SESSION_ID | jq"
echo ""
echo "  3. Approve and start implementation:"
echo "     gleam run -- plan-approve $SESSION_ID --yes"
echo ""
echo "  4. Review AI prompts:"
echo "     cat $PROMPTS_FILE | jq -r '.data.prompts[].prompt'"
echo ""

if (( $(echo "$QUALITY_SCORE < 80" | bc -l) )) || [ "$GAP_COUNT" -gt 5 ]; then
    echo ""
    warning "Consider iterating on the spec to improve quality before implementation"
    echo ""
    echo "  Iteration workflow:"
    echo "    1. gleam run -- interview api --resume=$SESSION_ID"
    echo "    2. Re-run this script: ./new-api-spec.sh $OUTPUT_FILE"
fi

echo ""
