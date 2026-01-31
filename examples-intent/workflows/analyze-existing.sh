#!/usr/bin/env bash
#
# analyze-existing.sh
# Comprehensive analysis workflow for an existing API specification
#
# Usage: ./analyze-existing.sh <spec.cue> [--report-file=report.json]
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
    error "Usage: $0 <spec.cue> [--report-file=report.json]"
    exit 1
fi

SPEC_FILE="$1"
REPORT_FILE="analysis-report.json"

# Parse optional report file argument
if [ $# -eq 2 ]; then
    REPORT_FILE="${2#--report-file=}"
fi

# Check if spec file exists
if [ ! -f "$SPEC_FILE" ]; then
    error "Spec file not found: $SPEC_FILE"
    exit 1
fi

# Check if gleam is available
if ! command -v gleam &> /dev/null; then
    error "Gleam is not installed. Please install Gleam first."
    exit 1
fi

# Check if jq is available
if ! command -v jq &> /dev/null; then
    error "jq is not installed. Please install jq for JSON processing."
    exit 1
fi

info "Analyzing spec: $SPEC_FILE"
echo ""

# Initialize report
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
cat > "$REPORT_FILE" <<EOF
{
  "spec_file": "$SPEC_FILE",
  "timestamp": "$TIMESTAMP",
  "analyses": {}
}
EOF

# Step 1: Validate
header "1. Validation"

if gleam run -- validate "$SPEC_FILE" 2>&1; then
    success "Spec is valid"
    VALIDATION_STATUS="pass"
else
    error "Spec validation failed"
    VALIDATION_STATUS="fail"
fi

# Update report
TMP_REPORT=$(jq --arg status "$VALIDATION_STATUS" \
    '.analyses.validation = {status: $status}' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

if [ "$VALIDATION_STATUS" = "fail" ]; then
    error "Cannot proceed with invalid spec. Fix validation errors first."
    exit 1
fi

# Step 2: Quality Analysis
header "2. Quality Analysis"

QUALITY_JSON=$(gleam run -- quality "$SPEC_FILE")
echo "$QUALITY_JSON" | jq -r '
    "Overall Score: \(.data.overall_score)/100\n" +
    "\nDimensions:" +
    "\n  Coverage:      \(.data.dimensions.coverage)/100" +
    "\n  Clarity:       \(.data.dimensions.clarity)/100" +
    "\n  Testability:   \(.data.dimensions.testability)/100" +
    "\n  AI Readiness:  \(.data.dimensions.ai_readiness)/100" +
    "\n\nDetails:" +
    "\n  Features:      \(.data.details.feature_count)" +
    "\n  Behaviors:     \(.data.details.behavior_count)" +
    "\n  Checks:        \(.data.details.check_count)"
'

QUALITY_SCORE=$(echo "$QUALITY_JSON" | jq -r '.data.overall_score // 0')

if (( $(echo "$QUALITY_SCORE >= 80" | bc -l) )); then
    success "Excellent quality score (>= 80)"
elif (( $(echo "$QUALITY_SCORE >= 60" | bc -l) )); then
    warning "Good quality score (>= 60)"
else
    warning "Quality score needs improvement (< 60)"
fi

# Update report
TMP_REPORT=$(jq --argjson quality "$(echo "$QUALITY_JSON" | jq '.data')" \
    '.analyses.quality = $quality' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

# Step 3: Linting
header "3. Lint Analysis"

LINT_OUTPUT=$(gleam run -- lint "$SPEC_FILE" 2>&1 || true)
echo "$LINT_OUTPUT"

LINT_WARNING_COUNT=$(echo "$LINT_OUTPUT" | grep -c "⚠" || echo "0")

if [ "$LINT_WARNING_COUNT" -eq 0 ]; then
    success "No linting warnings"
else
    warning "Found $LINT_WARNING_COUNT linting warnings"
fi

# Update report
TMP_REPORT=$(jq --arg count "$LINT_WARNING_COUNT" \
    '.analyses.lint = {warning_count: ($count | tonumber)}' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

# Step 4: Coverage Analysis
header "4. Coverage Analysis (OWASP + Edge Cases)"

COVERAGE_JSON=$(gleam run -- coverage "$SPEC_FILE")
echo "$COVERAGE_JSON" | jq -r '
    "Coverage Score: \(.data.score)/100\n" +
    "\nOWASP Top 10 Coverage:" +
    "\n  Injection:                    \(if .data.owasp_coverage.injection then "✓" else "✗" end)" +
    "\n  Broken Authentication:        \(if .data.owasp_coverage.broken_authentication then "✓" else "✗" end)" +
    "\n  Sensitive Data Exposure:      \(if .data.owasp_coverage.sensitive_data_exposure then "✓" else "✗" end)" +
    "\n  XML External Entities:        \(if .data.owasp_coverage.xml_external_entities then "✓" else "✗" end)" +
    "\n  Broken Access Control:        \(if .data.owasp_coverage.broken_access_control then "✓" else "✗" end)" +
    "\n  Security Misconfiguration:    \(if .data.owasp_coverage.security_misconfiguration then "✓" else "✗" end)" +
    "\n\nEdge Case Coverage:" +
    "\n  Empty inputs:         \(.data.edge_cases.empty_inputs)" +
    "\n  Max length inputs:    \(.data.edge_cases.max_length_inputs)" +
    "\n  Special characters:   \(.data.edge_cases.special_characters)" +
    "\n  Concurrent requests:  \(.data.edge_cases.concurrent_requests)"
'

COVERAGE_SCORE=$(echo "$COVERAGE_JSON" | jq -r '.data.score // 0')

if (( $(echo "$COVERAGE_SCORE >= 70" | bc -l) )); then
    success "Good coverage score (>= 70)"
else
    warning "Coverage needs improvement (< 70)"
fi

# Update report
TMP_REPORT=$(jq --argjson coverage "$(echo "$COVERAGE_JSON" | jq '.data')" \
    '.analyses.coverage = $coverage' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

# Step 5: Gap Detection
header "5. Gap Detection"

GAPS_JSON=$(gleam run -- gaps "$SPEC_FILE")
GAP_COUNT=$(echo "$GAPS_JSON" | jq -r '.data.gap_count // 0')

echo "Found $GAP_COUNT gaps"
echo ""

if [ "$GAP_COUNT" -gt 0 ]; then
    echo "$GAPS_JSON" | jq -r '.data.gaps[] |
        "[\(.severity | ascii_upcase)] \(.type)\n" +
        "  \(.description)\n" +
        "  → \(.recommendation)\n"'

    # Count by severity
    CRITICAL_GAPS=$(echo "$GAPS_JSON" | jq '[.data.gaps[] | select(.severity == "critical")] | length')
    HIGH_GAPS=$(echo "$GAPS_JSON" | jq '[.data.gaps[] | select(.severity == "high")] | length')
    MEDIUM_GAPS=$(echo "$GAPS_JSON" | jq '[.data.gaps[] | select(.severity == "medium")] | length')

    echo "Gap Breakdown:"
    echo "  Critical: $CRITICAL_GAPS"
    echo "  High:     $HIGH_GAPS"
    echo "  Medium:   $MEDIUM_GAPS"

    if [ "$CRITICAL_GAPS" -gt 0 ]; then
        warning "Address critical gaps before deployment"
    fi
else
    success "No gaps detected - excellent coverage!"
fi

# Update report
TMP_REPORT=$(jq --argjson gaps "$(echo "$GAPS_JSON" | jq '.data')" \
    '.analyses.gaps = $gaps' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

# Step 6: Inversion Analysis (Failure Modes)
header "6. Failure Mode Analysis"

INVERT_JSON=$(gleam run -- invert "$SPEC_FILE")
FAILURE_COUNT=$(echo "$INVERT_JSON" | jq -r '.data.failure_count // 0')

echo "Identified $FAILURE_COUNT potential failure modes"
echo ""

# Show top 5 critical failures
echo "Top Critical Failure Scenarios:"
echo "$INVERT_JSON" | jq -r '.data.failure_modes[] |
    select(.severity == "critical") |
    "  [\(.category | ascii_upcase)] \(.scenario)\n" +
    "    Mitigation: \(.mitigation)\n"' | head -20

# Count by category
SECURITY_FAILURES=$(echo "$INVERT_JSON" | jq '[.data.failure_modes[] | select(.category == "security")] | length')
USABILITY_FAILURES=$(echo "$INVERT_JSON" | jq '[.data.failure_modes[] | select(.category == "usability")] | length')
INTEGRATION_FAILURES=$(echo "$INVERT_JSON" | jq '[.data.failure_modes[] | select(.category == "integration")] | length')

echo "Failure Breakdown:"
echo "  Security:     $SECURITY_FAILURES"
echo "  Usability:    $USABILITY_FAILURES"
echo "  Integration:  $INTEGRATION_FAILURES"

# Update report
TMP_REPORT=$(jq --argjson invert "$(echo "$INVERT_JSON" | jq '.data')" \
    '.analyses.inversion = $invert' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

# Step 7: Effects Analysis
header "7. Second-Order Effects Analysis"

EFFECTS_JSON=$(gleam run -- effects "$SPEC_FILE")
echo "$EFFECTS_JSON" | jq -r '
    if .data.effects then
        "Second-Order Effects:\n" +
        (.data.effects[] |
            "  \(.primary) →\n" +
            (.secondary[] | "    • \(.)") + "\n") +
        "\nOrphan Behaviors (no dependencies): \(.data.orphan_count // 0)\n" +
        "Circular Dependencies: \(.data.circular_count // 0)"
    else
        "No second-order effects data available"
    end
'

# Update report
TMP_REPORT=$(jq --argjson effects "$(echo "$EFFECTS_JSON" | jq '.data')" \
    '.analyses.effects = $effects' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

# Step 8: Doctor (Health Report)
header "8. Health Report & Recommendations"

gleam run -- doctor "$SPEC_FILE"

DOCTOR_JSON=$(gleam run -- doctor "$SPEC_FILE")

# Update report
TMP_REPORT=$(jq --argjson doctor "$(echo "$DOCTOR_JSON" | jq '.data')" \
    '.analyses.doctor = $doctor' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

# Step 9: Improvement Suggestions
header "9. Improvement Suggestions"

gleam run -- improve "$SPEC_FILE"

# Final Summary
header "Analysis Summary"

cat <<EOF
Spec File: $SPEC_FILE
Analysis Date: $TIMESTAMP

Scores:
  Overall Quality:   $QUALITY_SCORE/100
  OWASP Coverage:    $COVERAGE_SCORE/100

Issues:
  Gaps:              $GAP_COUNT
  Failure Modes:     $FAILURE_COUNT
  Lint Warnings:     $LINT_WARNING_COUNT

Report saved to: $REPORT_FILE
EOF

# Grade the spec
GRADE="F"
if (( $(echo "$QUALITY_SCORE >= 90" | bc -l) )); then
    GRADE="A"
elif (( $(echo "$QUALITY_SCORE >= 80" | bc -l) )); then
    GRADE="B"
elif (( $(echo "$QUALITY_SCORE >= 70" | bc -l) )); then
    GRADE="C"
elif (( $(echo "$QUALITY_SCORE >= 60" | bc -l) )); then
    GRADE="D"
fi

echo ""
if [ "$GRADE" = "A" ] || [ "$GRADE" = "B" ]; then
    success "Overall Grade: $GRADE"
elif [ "$GRADE" = "C" ]; then
    warning "Overall Grade: $GRADE"
else
    warning "Overall Grade: $GRADE - Needs improvement"
fi

echo ""
echo "Next Steps:"
echo ""

if [ "$GAP_COUNT" -gt 0 ]; then
    echo "  1. Review gaps report:"
    echo "     cat $REPORT_FILE | jq '.analyses.gaps'"
    echo ""
fi

if (( $(echo "$QUALITY_SCORE < 80" | bc -l) )); then
    echo "  2. Address quality issues:"
    echo "     gleam run -- doctor $SPEC_FILE"
    echo ""
fi

if [ "$FAILURE_COUNT" -gt 10 ]; then
    echo "  3. Review failure modes:"
    echo "     cat $REPORT_FILE | jq '.analyses.inversion.failure_modes'"
    echo ""
fi

echo "  4. View full JSON report:"
echo "     cat $REPORT_FILE | jq"
echo ""

# Add grade to report
TMP_REPORT=$(jq --arg grade "$GRADE" '.summary = {grade: $grade}' "$REPORT_FILE")
echo "$TMP_REPORT" > "$REPORT_FILE"

success "Analysis complete!"
echo ""
