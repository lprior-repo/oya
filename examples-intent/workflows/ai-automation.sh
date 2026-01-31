#!/usr/bin/env bash
#
# ai-automation.sh
# AI-driven analysis and implementation pipeline
# Generates machine-readable JSON for AI tool integration
#
# Usage: ./ai-automation.sh <spec.cue> [--output-dir=./ai-output]
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
    error "Usage: $0 <spec.cue> [--output-dir=./ai-output]"
    exit 1
fi

SPEC_FILE="$1"
OUTPUT_DIR="./ai-output"

# Parse optional output directory
if [ $# -eq 2 ]; then
    OUTPUT_DIR="${2#--output-dir=}"
fi

# Check if spec file exists
if [ ! -f "$SPEC_FILE" ]; then
    error "Spec file not found: $SPEC_FILE"
    exit 1
fi

# Check dependencies
if ! command -v gleam &> /dev/null; then
    error "Gleam is not installed"
    exit 1
fi

if ! command -v jq &> /dev/null; then
    error "jq is not installed"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"
info "Output directory: $OUTPUT_DIR"
echo ""

# Pipeline metadata
PIPELINE_ID="pipeline_$(date +%Y%m%d_%H%M%S)"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

info "Pipeline ID: $PIPELINE_ID"
info "Timestamp: $TIMESTAMP"
echo ""

# Initialize pipeline manifest
MANIFEST_FILE="$OUTPUT_DIR/manifest.json"
cat > "$MANIFEST_FILE" <<EOF
{
  "pipeline_id": "$PIPELINE_ID",
  "spec_file": "$SPEC_FILE",
  "timestamp": "$TIMESTAMP",
  "outputs": {},
  "metadata": {
    "version": "1.0.0",
    "tool": "intent-cli",
    "purpose": "AI-driven analysis and implementation"
  }
}
EOF

# Step 1: Validate
header "1. Validation"

info "Validating spec..."
VALIDATION_OUTPUT=$(gleam run -- validate "$SPEC_FILE" 2>&1)
VALIDATION_EXIT_CODE=$?

if [ $VALIDATION_EXIT_CODE -eq 0 ]; then
    success "Spec is valid"
    VALIDATION_STATUS="pass"
else
    error "Validation failed"
    VALIDATION_STATUS="fail"
    echo "$VALIDATION_OUTPUT"
fi

# Update manifest
TMP_MANIFEST=$(jq --arg status "$VALIDATION_STATUS" \
    '.outputs.validation = {status: $status}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

if [ "$VALIDATION_STATUS" = "fail" ]; then
    error "Cannot proceed with invalid spec"
    exit 1
fi

# Step 2: Quality Analysis
header "2. Quality Analysis"

QUALITY_FILE="$OUTPUT_DIR/01_quality.json"
info "Running quality analysis..."

gleam run -- quality "$SPEC_FILE" > "$QUALITY_FILE"
QUALITY_SCORE=$(jq -r '.data.overall_score // 0' "$QUALITY_FILE")

success "Quality score: $QUALITY_SCORE/100"
info "Output: $QUALITY_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$QUALITY_FILE" \
    '.outputs.quality = {file: $file}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Step 3: Coverage Analysis
header "3. Coverage Analysis (OWASP + Edge Cases)"

COVERAGE_FILE="$OUTPUT_DIR/02_coverage.json"
info "Analyzing coverage..."

gleam run -- coverage "$SPEC_FILE" > "$COVERAGE_FILE"
COVERAGE_SCORE=$(jq -r '.data.score // 0' "$COVERAGE_FILE")

success "Coverage score: $COVERAGE_SCORE/100"
info "Output: $COVERAGE_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$COVERAGE_FILE" \
    '.outputs.coverage = {file: $file}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Step 4: Gap Detection
header "4. Gap Detection"

GAPS_FILE="$OUTPUT_DIR/03_gaps.json"
info "Detecting gaps..."

gleam run -- gaps "$SPEC_FILE" > "$GAPS_FILE"
GAP_COUNT=$(jq -r '.data.gap_count // 0' "$GAPS_FILE")

if [ "$GAP_COUNT" -gt 0 ]; then
    warning "Found $GAP_COUNT gaps"
else
    success "No gaps found"
fi
info "Output: $GAPS_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$GAPS_FILE" --argjson count "$GAP_COUNT" \
    '.outputs.gaps = {file: $file, count: $count}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Step 5: Inversion Analysis
header "5. Failure Mode Analysis"

INVERT_FILE="$OUTPUT_DIR/04_inversion.json"
info "Analyzing failure modes..."

gleam run -- invert "$SPEC_FILE" > "$INVERT_FILE"
FAILURE_COUNT=$(jq -r '.data.failure_count // 0' "$INVERT_FILE")

success "Identified $FAILURE_COUNT failure modes"
info "Output: $INVERT_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$INVERT_FILE" --argjson count "$FAILURE_COUNT" \
    '.outputs.inversion = {file: $file, count: $count}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Step 6: Effects Analysis
header "6. Second-Order Effects"

EFFECTS_FILE="$OUTPUT_DIR/05_effects.json"
info "Analyzing second-order effects..."

gleam run -- effects "$SPEC_FILE" > "$EFFECTS_FILE"
success "Effects analysis complete"
info "Output: $EFFECTS_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$EFFECTS_FILE" \
    '.outputs.effects = {file: $file}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Step 7: Doctor (Health Report)
header "7. Health Report"

DOCTOR_FILE="$OUTPUT_DIR/06_doctor.json"
info "Generating health report..."

gleam run -- doctor "$SPEC_FILE" > "$DOCTOR_FILE"
ISSUE_COUNT=$(jq '[.data.issues] | length' "$DOCTOR_FILE")

success "Found $ISSUE_COUNT improvement opportunities"
info "Output: $DOCTOR_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$DOCTOR_FILE" \
    '.outputs.doctor = {file: $file}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Step 8: Get session ID if available (for beads/prompts)
header "8. Work Item Generation"

# Try to find session ID from sessions
SESSION_ID=$(gleam run -- sessions --profile=api 2>/dev/null | grep "ID:" | head -1 | awk '{print $2}' || echo "")

if [ -n "$SESSION_ID" ]; then
    info "Found session: $SESSION_ID"

    # Generate beads
    BEADS_FILE="$OUTPUT_DIR/07_beads.json"
    info "Generating work beads..."

    gleam run -- beads "$SESSION_ID" > "$BEADS_FILE"
    BEAD_COUNT=$(jq -r '.data.bead_count // 0' "$BEADS_FILE")

    success "Generated $BEAD_COUNT work items"
    info "Output: $BEADS_FILE"

    # Update manifest
    TMP_MANIFEST=$(jq --arg file "$BEADS_FILE" --argjson count "$BEAD_COUNT" \
        '.outputs.beads = {file: $file, count: $count}' "$MANIFEST_FILE")
    echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

    # Generate AI prompts
    PROMPTS_FILE="$OUTPUT_DIR/08_prompts.json"
    info "Generating AI prompts..."

    gleam run -- prompt "$SESSION_ID" > "$PROMPTS_FILE"
    PROMPT_COUNT=$(jq '.data.prompts | length' "$PROMPTS_FILE")

    success "Generated $PROMPT_COUNT AI prompts"
    info "Output: $PROMPTS_FILE"

    # Update manifest
    TMP_MANIFEST=$(jq --arg file "$PROMPTS_FILE" --argjson count "$PROMPT_COUNT" \
        '.outputs.prompts = {file: $file, count: $count}' "$MANIFEST_FILE")
    echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

    # Extract prompts to individual text files for easy AI consumption
    PROMPTS_TEXT_DIR="$OUTPUT_DIR/prompts"
    mkdir -p "$PROMPTS_TEXT_DIR"

    info "Extracting prompts to text files..."
    jq -r '.data.prompts[] | "\(.bead_id)\n---PROMPT---\n\(.prompt)\n"' "$PROMPTS_FILE" | \
        awk -v dir="$PROMPTS_TEXT_DIR" '
            /^bead_/ { id=$0; next }
            /^---PROMPT---$/ { getline; print > (dir "/" id ".txt"); while(getline && NF) print >> (dir "/" id ".txt") }
        '

    PROMPT_FILE_COUNT=$(ls -1 "$PROMPTS_TEXT_DIR" | wc -l)
    success "Extracted $PROMPT_FILE_COUNT prompt files to $PROMPTS_TEXT_DIR/"

else
    warning "No interview session found - skipping beads and prompts"
    info "Run 'gleam run -- interview api' first to generate work items"
fi

# Step 9: Generate AI Schema
header "9. AI Action Schema"

SCHEMA_FILE="$OUTPUT_DIR/09_ai_schema.json"
info "Generating AI action schema..."

gleam run -- ai schema > "$SCHEMA_FILE"
success "AI schema generated"
info "Output: $SCHEMA_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$SCHEMA_FILE" \
    '.outputs.schema = {file: $file}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Step 10: Create consolidated AI context
header "10. Consolidated AI Context"

CONTEXT_FILE="$OUTPUT_DIR/ai_context.json"
info "Building consolidated context for AI agents..."

jq -n \
    --arg pipeline_id "$PIPELINE_ID" \
    --arg spec_file "$SPEC_FILE" \
    --arg timestamp "$TIMESTAMP" \
    --argjson quality "$(cat "$QUALITY_FILE")" \
    --argjson coverage "$(cat "$COVERAGE_FILE")" \
    --argjson gaps "$(cat "$GAPS_FILE")" \
    --argjson inversion "$(cat "$INVERT_FILE")" \
    --argjson effects "$(cat "$EFFECTS_FILE")" \
    --argjson doctor "$(cat "$DOCTOR_FILE")" \
    '{
        pipeline_id: $pipeline_id,
        spec_file: $spec_file,
        timestamp: $timestamp,
        analyses: {
            quality: $quality.data,
            coverage: $coverage.data,
            gaps: $gaps.data,
            inversion: $inversion.data,
            effects: $effects.data,
            doctor: $doctor.data
        },
        summary: {
            quality_score: $quality.data.overall_score,
            coverage_score: $coverage.data.score,
            gap_count: $gaps.data.gap_count,
            failure_mode_count: $inversion.data.failure_count
        }
    }' > "$CONTEXT_FILE"

if [ -n "$SESSION_ID" ]; then
    # Add beads and prompts if available
    TMP_CONTEXT=$(jq \
        --argjson beads "$(cat "$BEADS_FILE")" \
        --argjson prompts "$(cat "$PROMPTS_FILE")" \
        '.work_items = {
            beads: $beads.data.beads,
            prompts: $prompts.data.prompts
        }' "$CONTEXT_FILE")
    echo "$TMP_CONTEXT" > "$CONTEXT_FILE"
fi

success "AI context created"
info "Output: $CONTEXT_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$CONTEXT_FILE" \
    '.outputs.ai_context = {file: $file}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Step 11: Create next actions
header "11. Next Actions"

NEXT_ACTIONS_FILE="$OUTPUT_DIR/next_actions.json"

# Aggregate next_actions from all analyses
jq -n \
    --argjson quality "$(cat "$QUALITY_FILE")" \
    --argjson coverage "$(cat "$COVERAGE_FILE")" \
    --argjson gaps "$(cat "$GAPS_FILE")" \
    --argjson doctor "$(cat "$DOCTOR_FILE")" \
    '{
        next_actions: (
            ($quality.next_actions // []) +
            ($coverage.next_actions // []) +
            ($gaps.next_actions // []) +
            ($doctor.next_actions // [])
        ) | unique_by(.command)
    }' > "$NEXT_ACTIONS_FILE"

ACTION_COUNT=$(jq '.next_actions | length' "$NEXT_ACTIONS_FILE")
success "Generated $ACTION_COUNT next action recommendations"
info "Output: $NEXT_ACTIONS_FILE"

# Update manifest
TMP_MANIFEST=$(jq --arg file "$NEXT_ACTIONS_FILE" \
    '.outputs.next_actions = {file: $file}' "$MANIFEST_FILE")
echo "$TMP_MANIFEST" > "$MANIFEST_FILE"

# Final Summary
header "Pipeline Complete"

cat <<EOF
Pipeline ID: $PIPELINE_ID
Spec File: $SPEC_FILE
Output Directory: $OUTPUT_DIR

Generated Files:
  1. Quality Analysis:     $QUALITY_FILE
  2. Coverage Analysis:    $COVERAGE_FILE
  3. Gap Detection:        $GAPS_FILE
  4. Failure Modes:        $INVERT_FILE
  5. Effects Analysis:     $EFFECTS_FILE
  6. Health Report:        $DOCTOR_FILE
EOF

if [ -n "$SESSION_ID" ]; then
    cat <<EOF
  7. Work Beads:           $BEADS_FILE
  8. AI Prompts:           $PROMPTS_FILE
  9. Prompt Text Files:    $PROMPTS_TEXT_DIR/ ($PROMPT_FILE_COUNT files)
EOF
fi

cat <<EOF
  10. AI Action Schema:    $SCHEMA_FILE
  11. AI Context:          $CONTEXT_FILE
  12. Next Actions:        $NEXT_ACTIONS_FILE
  13. Manifest:            $MANIFEST_FILE

Scores:
  Quality:   $QUALITY_SCORE/100
  Coverage:  $COVERAGE_SCORE/100

Issues:
  Gaps:          $GAP_COUNT
  Failures:      $FAILURE_COUNT
EOF

if [ -n "$SESSION_ID" ]; then
    cat <<EOF
  Work Items:    $BEAD_COUNT
  AI Prompts:    $PROMPT_COUNT
EOF
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
success "AI Pipeline Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# AI Integration Examples
header "AI Integration Examples"

cat <<EOF
1. Use AI context for automated analysis:
   cat $CONTEXT_FILE | your-ai-tool analyze

2. Feed prompts to AI implementation:
   for prompt in $PROMPTS_TEXT_DIR/*.txt; do
       cat \$prompt | your-ai-tool implement
   done

3. Process next actions:
   jq -r '.next_actions[].command' $NEXT_ACTIONS_FILE | while read cmd; do
       echo "Executing: \$cmd"
       \$cmd
   done

4. Filter high-priority gaps for AI triage:
   jq '.analyses.gaps.gaps[] | select(.severity == "high")' $CONTEXT_FILE

5. Extract critical failure modes:
   jq '.analyses.inversion.failure_modes[] | select(.severity == "critical")' $CONTEXT_FILE

6. Get prioritized work items:
   jq '.work_items.beads | sort_by(.estimated_minutes)' $CONTEXT_FILE

7. View all outputs:
   cat $MANIFEST_FILE | jq

8. Generate implementation plan from beads:
   jq -r '.work_items.beads[] | "\(.id): \(.title) (\(.estimated_minutes)min)"' $CONTEXT_FILE
EOF

echo ""
info "All output files are in JSON format for easy AI consumption"
echo ""

# Quality gate check
if (( $(echo "$QUALITY_SCORE < 70" | bc -l) )) || [ "$GAP_COUNT" -gt 10 ]; then
    echo ""
    warning "Quality gate: Consider improving spec before AI implementation"
    echo ""
    echo "Improvement workflow:"
    echo "  ./improve-quality.sh $SPEC_FILE --target-score=80"
    echo ""
fi

success "Pipeline artifacts ready for AI integration"
echo ""
