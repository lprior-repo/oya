#!/usr/bin/env bash
#
# improve-quality.sh
# Iterative workflow to improve spec quality based on analysis
#
# Usage: ./improve-quality.sh <spec.cue> [--target-score=80]
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
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

header() {
    echo ""
    echo -e "${BOLD}${CYAN}━━━ $1 ━━━${NC}"
    echo ""
}

# Check arguments
if [ $# -lt 1 ]; then
    error "Usage: $0 <spec.cue> [--target-score=80]"
    exit 1
fi

SPEC_FILE="$1"
TARGET_SCORE=80

# Parse optional target score argument
if [ $# -eq 2 ]; then
    TARGET_SCORE="${2#--target-score=}"
fi

# Check if spec file exists
if [ ! -f "$SPEC_FILE" ]; then
    error "Spec file not found: $SPEC_FILE"
    exit 1
fi

# Check if gleam and jq are available
if ! command -v gleam &> /dev/null; then
    error "Gleam is not installed. Please install Gleam first."
    exit 1
fi

if ! command -v jq &> /dev/null; then
    error "jq is not installed. Please install jq for JSON processing."
    exit 1
fi

# Create backup
BACKUP_FILE="${SPEC_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
cp "$SPEC_FILE" "$BACKUP_FILE"
info "Created backup: $BACKUP_FILE"
echo ""

# Initial assessment
header "Initial Quality Assessment"

INITIAL_JSON=$(gleam run -- quality "$SPEC_FILE")
INITIAL_SCORE=$(echo "$INITIAL_JSON" | jq -r '.data.overall_score // 0')

echo "Current Quality Score: $INITIAL_SCORE/100"
echo "Target Quality Score:  $TARGET_SCORE/100"
echo ""

if (( $(echo "$INITIAL_SCORE >= $TARGET_SCORE" | bc -l) )); then
    success "Already meets target quality score!"
    echo ""
    info "Running doctor for optimization suggestions..."
    gleam run -- doctor "$SPEC_FILE"
    exit 0
fi

POINTS_NEEDED=$(echo "$TARGET_SCORE - $INITIAL_SCORE" | bc)
info "Need to improve by $POINTS_NEEDED points"
echo ""

# Dimension breakdown
echo "Dimension Scores:"
echo "$INITIAL_JSON" | jq -r '
    "  Coverage:      \(.data.dimensions.coverage)/100" +
    "\n  Clarity:       \(.data.dimensions.clarity)/100" +
    "\n  Testability:   \(.data.dimensions.testability)/100" +
    "\n  AI Readiness:  \(.data.dimensions.ai_readiness)/100"
'
echo ""

# Identify weakest dimension
WEAKEST_DIM=$(echo "$INITIAL_JSON" | jq -r '
    .data.dimensions |
    to_entries |
    min_by(.value) |
    .key
')
WEAKEST_SCORE=$(echo "$INITIAL_JSON" | jq -r ".data.dimensions.$WEAKEST_DIM")

warning "Weakest dimension: $WEAKEST_DIM ($WEAKEST_SCORE/100)"
echo ""

# Step 1: Run Doctor for prioritized fixes
header "Step 1: Health Analysis"

DOCTOR_JSON=$(gleam run -- doctor "$SPEC_FILE")

# Count issues by severity
CRITICAL_COUNT=$(echo "$DOCTOR_JSON" | jq '[.data.issues[] | select(.severity == "critical")] | length')
WARNING_COUNT=$(echo "$DOCTOR_JSON" | jq '[.data.issues[] | select(.severity == "warning")] | length')
SUGGESTION_COUNT=$(echo "$DOCTOR_JSON" | jq '[.data.issues[] | select(.severity == "suggestion")] | length')

echo "Issues Found:"
echo "  Critical:    $CRITICAL_COUNT"
echo "  Warnings:    $WARNING_COUNT"
echo "  Suggestions: $SUGGESTION_COUNT"
echo ""

if [ "$CRITICAL_COUNT" -gt 0 ]; then
    warning "Critical issues must be addressed first"
    echo ""
    echo "Critical Issues:"
    echo "$DOCTOR_JSON" | jq -r '.data.issues[] |
        select(.severity == "critical") |
        "  🔴 \(.title)\n     Fix: \(.fix)\n     Impact: \(.impact)\n"'
fi

# Step 2: Check for gaps
header "Step 2: Gap Analysis"

GAPS_JSON=$(gleam run -- gaps "$SPEC_FILE")
GAP_COUNT=$(echo "$GAPS_JSON" | jq -r '.data.gap_count // 0')

echo "Found $GAP_COUNT gaps"
echo ""

if [ "$GAP_COUNT" -gt 0 ]; then
    # Prioritize high severity gaps
    HIGH_GAPS=$(echo "$GAPS_JSON" | jq '[.data.gaps[] | select(.severity == "high")] | length')

    if [ "$HIGH_GAPS" -gt 0 ]; then
        echo "High Priority Gaps:"
        echo "$GAPS_JSON" | jq -r '.data.gaps[] |
            select(.severity == "high") |
            "  • [\(.type)] \(.description)\n    → \(.recommendation)\n"'
    fi
fi

# Step 3: Coverage improvements
header "Step 3: Coverage Improvements"

COVERAGE_JSON=$(gleam run -- coverage "$SPEC_FILE")
COVERAGE_SCORE=$(echo "$COVERAGE_JSON" | jq -r '.data.score // 0')

echo "Current Coverage Score: $COVERAGE_SCORE/100"
echo ""

# Identify missing OWASP coverage
MISSING_OWASP=$(echo "$COVERAGE_JSON" | jq -r '
    .data.owasp_coverage |
    to_entries |
    map(select(.value == false)) |
    .[].key'
)

if [ -n "$MISSING_OWASP" ]; then
    echo "Missing OWASP Top 10 Coverage:"
    while IFS= read -r item; do
        echo "  ✗ $item"
    done <<< "$MISSING_OWASP"
    echo ""
    info "Add behaviors to test these security scenarios"
fi

# Check edge case coverage
EMPTY_INPUT_COUNT=$(echo "$COVERAGE_JSON" | jq -r '.data.edge_cases.empty_inputs // 0')
MAX_LENGTH_COUNT=$(echo "$COVERAGE_JSON" | jq -r '.data.edge_cases.max_length_inputs // 0')
SPECIAL_CHAR_COUNT=$(echo "$COVERAGE_JSON" | jq -r '.data.edge_cases.special_characters // 0')
CONCURRENT_COUNT=$(echo "$COVERAGE_JSON" | jq -r '.data.edge_cases.concurrent_requests // 0')

echo "Edge Case Coverage:"
echo "  Empty inputs:        $EMPTY_INPUT_COUNT behaviors"
echo "  Max length inputs:   $MAX_LENGTH_COUNT behaviors"
echo "  Special characters:  $SPECIAL_CHAR_COUNT behaviors"
echo "  Concurrent requests: $CONCURRENT_COUNT behaviors"
echo ""

if [ "$MAX_LENGTH_COUNT" -eq 0 ]; then
    warning "No max length edge cases - add behaviors testing input limits"
fi

if [ "$CONCURRENT_COUNT" -eq 0 ]; then
    warning "No concurrency tests - consider adding race condition behaviors"
fi

# Step 4: Failure mode analysis
header "Step 4: Failure Mode Coverage"

INVERT_JSON=$(gleam run -- invert "$SPEC_FILE")
FAILURE_COUNT=$(echo "$INVERT_JSON" | jq -r '.data.failure_count // 0')

echo "Identified $FAILURE_COUNT potential failure modes"
echo ""

# Show top unmitigated failures
echo "Top Unmitigated Failure Scenarios:"
echo "$INVERT_JSON" | jq -r '.data.failure_modes[] |
    select(.severity == "critical") |
    "  🔴 [\(.category)] \(.scenario)\n     Mitigation: \(.mitigation)\n"' | head -15

# Step 5: Improvement suggestions
header "Step 5: Prioritized Improvements"

IMPROVE_OUTPUT=$(gleam run -- improve "$SPEC_FILE")
echo "$IMPROVE_OUTPUT"
echo ""

# Step 6: Create improvement checklist
header "Step 6: Improvement Checklist"

CHECKLIST_FILE="${SPEC_FILE%.cue}-improvements.md"

cat > "$CHECKLIST_FILE" <<EOF
# Quality Improvement Checklist for $SPEC_FILE

**Current Score:** $INITIAL_SCORE/100
**Target Score:** $TARGET_SCORE/100
**Points Needed:** $POINTS_NEEDED

---

## Critical Issues ($CRITICAL_COUNT)

EOF

if [ "$CRITICAL_COUNT" -gt 0 ]; then
    echo "$DOCTOR_JSON" | jq -r '.data.issues[] |
        select(.severity == "critical") |
        "- [ ] **\(.title)**\n  - Fix: \(.fix)\n  - Impact: \(.impact)\n  - Effort: \(.effort)\n"' >> "$CHECKLIST_FILE"
else
    echo "- [x] No critical issues" >> "$CHECKLIST_FILE"
fi

cat >> "$CHECKLIST_FILE" <<EOF

---

## High Priority Gaps ($HIGH_GAPS)

EOF

if [ "$GAP_COUNT" -gt 0 ]; then
    echo "$GAPS_JSON" | jq -r '.data.gaps[] |
        select(.severity == "high") |
        "- [ ] **[\(.type)] \(.description)**\n  - Action: \(.recommendation)\n"' >> "$CHECKLIST_FILE"
else
    echo "- [x] No high priority gaps" >> "$CHECKLIST_FILE"
fi

cat >> "$CHECKLIST_FILE" <<EOF

---

## Coverage Improvements

### Missing OWASP Coverage
EOF

if [ -n "$MISSING_OWASP" ]; then
    while IFS= read -r item; do
        echo "- [ ] Add $item test behaviors" >> "$CHECKLIST_FILE"
    done <<< "$MISSING_OWASP"
else
    echo "- [x] All OWASP categories covered" >> "$CHECKLIST_FILE"
fi

cat >> "$CHECKLIST_FILE" <<EOF

### Edge Cases
- [$([ "$EMPTY_INPUT_COUNT" -gt 0 ] && echo "x" || echo " ")] Empty input tests
- [$([ "$MAX_LENGTH_COUNT" -gt 0 ] && echo "x" || echo " ")] Max length tests
- [$([ "$SPECIAL_CHAR_COUNT" -gt 0 ] && echo "x" || echo " ")] Special character tests
- [$([ "$CONCURRENT_COUNT" -gt 0 ] && echo "x" || echo " ")] Concurrency tests

---

## Dimension-Specific Improvements

### Coverage (${INITIAL_JSON//[^0-9.]/}/100)
EOF

echo "$IMPROVE_OUTPUT" | grep -A 5 "Coverage" | tail -4 | sed 's/^/- [ ] /' >> "$CHECKLIST_FILE" || echo "- [x] No coverage improvements needed" >> "$CHECKLIST_FILE"

cat >> "$CHECKLIST_FILE" <<EOF

### Clarity (${INITIAL_JSON//[^0-9.]/}/100)
EOF

echo "$IMPROVE_OUTPUT" | grep -A 5 "Clarity" | tail -4 | sed 's/^/- [ ] /' >> "$CHECKLIST_FILE" || echo "- [x] No clarity improvements needed" >> "$CHECKLIST_FILE"

cat >> "$CHECKLIST_FILE" <<EOF

### Testability (${INITIAL_JSON//[^0-9.]/}/100)
EOF

echo "$IMPROVE_OUTPUT" | grep -A 5 "Testability" | tail -4 | sed 's/^/- [ ] /' >> "$CHECKLIST_FILE" || echo "- [x] No testability improvements needed" >> "$CHECKLIST_FILE"

cat >> "$CHECKLIST_FILE" <<EOF

### AI Readiness (${INITIAL_JSON//[^0-9.]/}/100)
EOF

echo "$IMPROVE_OUTPUT" | grep -A 5 "AI" | tail -4 | sed 's/^/- [ ] /' >> "$CHECKLIST_FILE" || echo "- [x] No AI readiness improvements needed" >> "$CHECKLIST_FILE"

cat >> "$CHECKLIST_FILE" <<EOF

---

## Quick Wins (< 15 minutes each)

EOF

echo "$DOCTOR_JSON" | jq -r '.data.issues[] |
    select(.effort == "low") |
    "- [ ] \(.title) (\(.estimated_minutes)min)\n  - \(.fix)\n"' >> "$CHECKLIST_FILE" || echo "- [x] All quick wins completed" >> "$CHECKLIST_FILE"

success "Improvement checklist created: $CHECKLIST_FILE"
echo ""

# Step 7: Suggestions for next steps
header "Recommended Actions"

cat <<EOF
1. Review the improvement checklist:
   cat $CHECKLIST_FILE

2. Start with critical issues (highest impact):
   - Address all $CRITICAL_COUNT critical issues
   - Focus on $WEAKEST_DIM dimension (currently $WEAKEST_SCORE/100)

3. Add missing behaviors:
   - OWASP coverage: $(echo "$MISSING_OWASP" | wc -l) categories
   - Edge cases: $([ "$MAX_LENGTH_COUNT" -eq 0 ] && echo "max length, " || echo "")$([ "$CONCURRENT_COUNT" -eq 0 ] && echo "concurrency" || echo "")

4. After making changes, re-run this script:
   ./improve-quality.sh $SPEC_FILE --target-score=$TARGET_SCORE

5. Track progress:
   - Backup available: $BACKUP_FILE
   - Checklist: $CHECKLIST_FILE
   - Current score: $INITIAL_SCORE/100
   - Target score: $TARGET_SCORE/100

EOF

# Estimate effort
TOTAL_EFFORT=$(echo "$DOCTOR_JSON" | jq '[.data.issues[].estimated_minutes] | add // 0')
echo "Estimated total effort: $TOTAL_EFFORT minutes"
echo ""

if (( $(echo "$TOTAL_EFFORT < 120" | bc -l) )); then
    success "Low effort required - can be completed in one session"
elif (( $(echo "$TOTAL_EFFORT < 240" | bc -l) )); then
    info "Medium effort - plan for 2-4 hours"
else
    warning "High effort - consider breaking into multiple sessions"
fi

echo ""
info "Use 'diff' to compare before/after once improvements are made:"
echo "  diff $BACKUP_FILE $SPEC_FILE"
echo ""
