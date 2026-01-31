# Intent CLI Workflow Scripts

This directory contains ready-to-use workflow scripts that demonstrate common Intent CLI usage patterns. Each script automates a complete workflow and can be customized for your needs.

## Available Workflows

### 1. new-api-spec.sh - Create New API Spec

Complete end-to-end workflow for creating a new API specification from scratch.

**Usage:**
```bash
./new-api-spec.sh [output-file.cue]
```

**What it does:**
1. Starts interactive interview session
2. Exports interview to CUE spec
3. Validates the generated spec
4. Analyzes quality (with scoring)
5. Identifies coverage gaps
6. Checks OWASP Top 10 coverage
7. Analyzes failure modes
8. Generates improvement suggestions
9. Creates work beads (implementation tasks)
10. Generates AI prompts for implementation

**Example:**
```bash
# Create a new API spec
./new-api-spec.sh my-api.cue

# Output files:
# - my-api.cue (the spec)
# - my-api-beads.json (work items)
# - my-api-prompts.json (AI prompts)
```

**When to use:**
- Starting a new API project
- Defining API contract before implementation
- Creating spec-driven development workflow

---

### 2. analyze-existing.sh - Comprehensive Analysis

Deep analysis of an existing specification with detailed reporting.

**Usage:**
```bash
./analyze-existing.sh <spec.cue> [--report-file=report.json]
```

**What it does:**
1. Validates spec syntax
2. Quality analysis (4 dimensions)
3. Lint checking (anti-patterns)
4. OWASP + edge case coverage
5. Gap detection (5 gap types)
6. Failure mode analysis (24 patterns)
7. Second-order effects
8. Health report with fixes
9. Improvement suggestions
10. Generates comprehensive JSON report

**Example:**
```bash
# Analyze existing spec
./analyze-existing.sh examples/user-api.cue

# Custom report location
./analyze-existing.sh examples/user-api.cue --report-file=./reports/user-api-report.json
```

**When to use:**
- Reviewing existing specs
- Pre-deployment quality checks
- CI/CD integration
- Regular health assessments

**Report Structure:**
```json
{
  "spec_file": "examples/user-api.cue",
  "timestamp": "2024-01-15T10:30:00Z",
  "analyses": {
    "validation": {"status": "pass"},
    "quality": {"overall_score": 85, "dimensions": {...}},
    "coverage": {"score": 75, "owasp_coverage": {...}},
    "gaps": {"gap_count": 3, "gaps": [...]},
    "inversion": {"failure_count": 12, "failure_modes": [...]},
    "effects": {...},
    "doctor": {...}
  },
  "summary": {"grade": "B"}
}
```

---

### 3. improve-quality.sh - Quality Improvement Workflow

Iterative workflow to improve spec quality based on analysis feedback.

**Usage:**
```bash
./improve-quality.sh <spec.cue> [--target-score=80]
```

**What it does:**
1. Creates backup of spec
2. Runs initial quality assessment
3. Identifies weakest dimension
4. Runs health analysis (doctor)
5. Detects gaps
6. Checks coverage improvements
7. Analyzes failure modes
8. Generates prioritized improvement list
9. Creates markdown checklist
10. Estimates effort required

**Example:**
```bash
# Improve spec to target score of 80
./improve-quality.sh my-api.cue --target-score=80

# Output files:
# - my-api.cue.backup.20240115_103000 (backup)
# - my-api-improvements.md (checklist)
```

**When to use:**
- After initial spec creation
- When quality score is below threshold
- Before production deployment
- Regular maintenance and updates

**Checklist Format:**
```markdown
# Quality Improvement Checklist

**Current Score:** 65/100
**Target Score:** 80/100
**Points Needed:** 15

## Critical Issues (2)
- [ ] **Add rate limiting behavior**
  - Fix: Create behavior testing 429 responses
  - Impact: Security
  - Effort: Low (20min)

## High Priority Gaps (3)
- [ ] **[security] No SQL injection tests**
  - Action: Add behavior with malicious input

## Coverage Improvements
### Missing OWASP Coverage
- [ ] Add XML External Entities test behaviors
- [ ] Add Security Misconfiguration test behaviors

## Quick Wins (< 15 minutes each)
- [ ] Add intent statements to behaviors (5min)
- [ ] Document error codes (10min)
```

---

### 4. ai-automation.sh - AI Integration Pipeline

AI-driven analysis pipeline that generates machine-readable JSON for automated workflows.

**Usage:**
```bash
./ai-automation.sh <spec.cue> [--output-dir=./ai-output]
```

**What it does:**
1. Validates spec
2. Quality analysis (JSON)
3. Coverage analysis (JSON)
4. Gap detection (JSON)
5. Failure mode analysis (JSON)
6. Second-order effects (JSON)
7. Health report (JSON)
8. Generates work beads (JSON)
9. Creates AI prompts (JSON + text files)
10. Generates AI action schema
11. Creates consolidated AI context
12. Aggregates next actions

**Example:**
```bash
# Run AI pipeline
./ai-automation.sh examples/user-api.cue

# Custom output directory
./ai-automation.sh examples/user-api.cue --output-dir=./ai-reports
```

**When to use:**
- Integrating with AI tools
- Automated implementation workflows
- CI/CD with AI-assisted development
- Generating prompts for LLMs

**Output Structure:**
```
ai-output/
├── manifest.json                  # Pipeline metadata
├── 01_quality.json               # Quality analysis
├── 02_coverage.json              # Coverage analysis
├── 03_gaps.json                  # Gap detection
├── 04_inversion.json             # Failure modes
├── 05_effects.json               # Second-order effects
├── 06_doctor.json                # Health report
├── 07_beads.json                 # Work items
├── 08_prompts.json               # AI prompts (JSON)
├── 09_ai_schema.json             # Action schema
├── ai_context.json               # Consolidated context
├── next_actions.json             # Recommended actions
└── prompts/                      # Individual prompt files
    ├── bead_001.txt
    ├── bead_002.txt
    └── ...
```

**AI Context Structure:**
```json
{
  "pipeline_id": "pipeline_20240115_103000",
  "spec_file": "examples/user-api.cue",
  "timestamp": "2024-01-15T10:30:00Z",
  "analyses": {
    "quality": {"overall_score": 85, ...},
    "coverage": {"score": 75, ...},
    "gaps": {"gap_count": 3, ...},
    "inversion": {"failure_count": 12, ...},
    "effects": {...},
    "doctor": {...}
  },
  "summary": {
    "quality_score": 85,
    "coverage_score": 75,
    "gap_count": 3,
    "failure_mode_count": 12
  },
  "work_items": {
    "beads": [...],
    "prompts": [...]
  }
}
```

---

## Common Patterns

### Pattern 1: New Project Setup
```bash
# 1. Create spec from interview
./new-api-spec.sh my-api.cue

# 2. Analyze and improve
./improve-quality.sh my-api.cue --target-score=85

# 3. Generate AI implementation pipeline
./ai-automation.sh my-api.cue
```

### Pattern 2: Existing Spec Review
```bash
# 1. Deep analysis
./analyze-existing.sh my-api.cue --report-file=report.json

# 2. Check quality threshold
SCORE=$(jq -r '.analyses.quality.overall_score' report.json)
if [ "$SCORE" -lt 80 ]; then
    # 3. Run improvement workflow
    ./improve-quality.sh my-api.cue --target-score=80
fi
```

### Pattern 3: CI/CD Integration
```bash
# In your CI pipeline:

# 1. Analyze spec
./analyze-existing.sh api.cue --report-file=ci-report.json

# 2. Extract quality score
SCORE=$(jq -r '.analyses.quality.overall_score' ci-report.json)

# 3. Quality gate
if [ "$SCORE" -lt 70 ]; then
    echo "Quality score $SCORE is below threshold 70"
    exit 1
fi

# 4. Check for critical gaps
CRITICAL=$(jq '[.analyses.gaps.gaps[] | select(.severity == "critical")] | length' ci-report.json)
if [ "$CRITICAL" -gt 0 ]; then
    echo "Found $CRITICAL critical gaps"
    exit 1
fi

echo "Quality checks passed"
```

### Pattern 4: AI-Assisted Implementation
```bash
# 1. Generate AI artifacts
./ai-automation.sh my-api.cue --output-dir=./ai

# 2. Process prompts with AI tool
for prompt in ./ai/prompts/*.txt; do
    echo "Implementing: $(basename $prompt)"
    cat "$prompt" | your-ai-tool implement > "impl/$(basename $prompt .txt).js"
done

# 3. Use beads for task tracking
jq -r '.work_items.beads[] | "\(.id),\(.title),\(.estimated_minutes)"' ./ai/ai_context.json > tasks.csv
```

---

## Dependencies

All workflows require:
- **Gleam**: Intent CLI runtime
- **jq**: JSON processing
- **bc**: Numeric comparisons (for quality thresholds)

Install on Ubuntu/Debian:
```bash
sudo apt-get install jq bc
```

Install on macOS:
```bash
brew install jq bc
```

---

## Customization

### Adding Custom Checks

Edit any workflow script and add custom validation:

```bash
# Custom quality threshold check
MIN_SCORE=85
if (( $(echo "$QUALITY_SCORE < $MIN_SCORE" | bc -l) )); then
    error "Quality score $QUALITY_SCORE is below minimum $MIN_SCORE"
    exit 1
fi
```

### Custom Output Formats

Convert JSON to other formats:

```bash
# Convert to CSV
jq -r '.data.gaps[] | [.severity, .type, .description] | @csv' gaps.json > gaps.csv

# Convert to YAML (requires yq)
jq . quality.json | yq -P > quality.yaml

# Generate HTML report
jq -r '
    "<h1>Quality Report</h1>" +
    "<p>Score: \(.data.overall_score)/100</p>" +
    "<ul>" +
    (.data.details | to_entries | map("<li>\(.key): \(.value)</li>") | join("")) +
    "</ul>"
' quality.json > report.html
```

### Integration with Other Tools

```bash
# Send results to monitoring
SCORE=$(jq -r '.data.overall_score' quality.json)
curl -X POST https://metrics.example.com/quality \
    -d "score=$SCORE&spec=my-api"

# Slack notification
if [ "$GAP_COUNT" -gt 5 ]; then
    curl -X POST $SLACK_WEBHOOK \
        -d "{\"text\": \"Found $GAP_COUNT gaps in spec $SPEC_FILE\"}"
fi

# GitHub PR comment
gh pr comment $PR_NUMBER --body "Quality Score: $SCORE/100"
```

---

## Troubleshooting

### Script fails with "gleam: command not found"
Install Gleam: https://gleam.run/getting-started/installing/

### Script fails with "jq: command not found"
Install jq: https://stedolan.github.io/jq/download/

### Invalid JSON output
Check that you're running the command correctly:
```bash
# Correct
gleam run -- quality spec.cue

# Wrong (missing spec)
gleam run -- quality
```

### No session ID found
Run interview first:
```bash
gleam run -- interview api
# Then run workflow that needs session ID
```

### Permission denied
Make scripts executable:
```bash
chmod +x *.sh
```

---

## Contributing

To add a new workflow:

1. Create `new-workflow.sh` in this directory
2. Use existing scripts as templates
3. Follow naming convention: `<verb>-<noun>.sh`
4. Add usage documentation at the top
5. Include error handling and status messages
6. Update this README

---

## Examples

See the parent `examples/` directory for sample specs:
- `user-api.cue`: User management API
- `meal-planner-api.cue`: Recipe and meal planning API
- `array-validation.cue`: Array validation patterns
- `regex-rules.cue`: Regex check rules

Run any workflow against these examples:
```bash
./analyze-existing.sh ../user-api.cue
./improve-quality.sh ../meal-planner-api.cue
./ai-automation.sh ../user-api.cue --output-dir=./test-output
```

---

## Support

For help with workflows:
1. Read the main tutorial: `../TUTORIAL.md`
2. Check command help: `gleam run -- <command> --help`
3. Review example specs: `../examples/*.cue`
4. See project README: `../../README.md`
