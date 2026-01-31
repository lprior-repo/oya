# Intent CLI

**AI-guided planning framework for rigorous requirement decomposition.**

Intent takes a high-level "north star" goal and guides you through systematic interviews to decompose it into crystal-clear, atomic work items that an AI can execute deterministically.

**The Planning Process:**
- Systematic interviewing uncovers hidden requirements
- EARS patterns eliminate ambiguity in specifications
- KIRK contracts enforce preconditions/postconditions
- Mental Lattices detect gaps and edge cases
- Beads: atomic, perfectly-specified work items

## How It Works

### Step 1: Start with a High-Level Goal

You describe what you want to build in natural language:
- "Build a user authentication system"
- "Create a payment processing flow"
- "Design a notification service"

### Step 2: AI-Guided Interview

The CLI guides you through rigorous decomposition using:

```bash
# Start an interview
intent interview --profile=api --cue

# AI asks systematic questions:
# {
#   action: "ask_question"
#   question: {
#     text: "In one sentence, what should this system do?"
#     pattern: "ubiquitous"
#   }
#   progress: { percent_complete: 0 }
# }
```

You answer, the CLI validates using EARS patterns, then asks the next question. This continues until every edge case is enumerated.

### Step 3: Rigorous Analysis

Multiple analysis dimensions catch what humans miss:

```bash
intent quality spec.cue      # 5-dimension scoring
intent coverage spec.cue    # OWASP + edge cases
intent gaps spec.cue        # Mental model gaps
intent invert spec.cue       # Failure mode analysis
intent effects spec.cue      # Second-order effects
```

### Step 4: Atomic Beads

The CLI generates beads - tiny, perfectly-specified work items:

```cue
beads: [{
    id: "USR-001"
    title: "Implement login endpoint"
    what: "Create POST /login that validates email/password and returns JWT"
    why: "Core authentication for all API access"
    test: "Valid credentials return 200 with JWT; invalid return 401"
    done_when: "All tests pass, endpoint responds correctly"
    edge_cases: ["empty email", "very long password", "unicode characters"]
    dependencies: []
}]
```

Each bead is so detailed that implementing it is purely mechanical - no decisions left to make.

### Step 5: AI Execution Prompts

Generate AI-ready prompts for each bead:

```bash
intent prompt <session-id>

# Outputs detailed prompts like:
# "Implement the login endpoint according to specification USR-001.
#  Input: {email, password}
#  Output: JWT token or error
#  Rules: bcrypt hashing, JWT HS256, 1hr expiry
#  Tests: Valid→200, Wrong password→401..."
```

### For AI: Follow CUE Instructions Exactly

The AI parses the CUE and asks the human that exact question. No improvisation:

```bash
# AI submits human's answer
intent interview --session X --answer "Allow users to log in with email and password"

# CLI processes, outputs next CUE directive
# {
#   action: "ask_question"
#   question: {
#     text: "Who will use this API?"
#     ...
#   }
#   progress: { percent_complete: 20 }
# }
```

### Beads: Atomic Work Units

When the interview completes, CLI generates beads - tiny, perfectly-specified work items:

```cue
beads: [{
    id: "USR-001"
    title: "Implement login endpoint"
    what: "Create POST /login that validates email/password and returns JWT"
    why: "Core authentication for all API access"
    test: "Valid credentials return 200 with JWT; invalid return 401"
    done_when: "All tests pass, endpoint responds correctly"
    edge_cases: ["empty email", "very long password", "unicode characters"]
    dependencies: []
}]
```

## Key Concepts

### EARS Requirements Syntax

Six patterns that eliminate ambiguity:

| Pattern | Template | Use For |
|---------|----------|---------|
| Ubiquitous | THE SYSTEM SHALL [behavior] | Always true |
| Event-Driven | WHEN [trigger] THE SYSTEM SHALL | Cause-effect |
| State-Driven | WHILE [state] THE SYSTEM SHALL | State-dependent |
| Optional | WHERE [condition] THE SYSTEM SHALL | Feature flags |
| Unwanted | IF [condition] THE SYSTEM SHALL NOT | Security |
| Complex | WHILE [state] WHEN [trigger] | Combinations |

### KIRK Contracts

Design by Contract for APIs:
- **Preconditions**: What must be true before
- **Postconditions**: What must be true after
- **Invariants**: What must always be true

### Mental Lattices

Five thinking tools catch what humans miss:
1. **Inversion**: "What would make this fail?"
2. **Second-Order**: "What happens after that?"
3. **Pre-Mortem**: "Why did this fail?"
4. **Checklist**: "What did we miss?"
5. **Circle of Competence**: "What's in scope?"

## Commands

```bash
# Core
intent check <spec.cue> --target <url>   # Run tests against API
intent validate <spec.cue>                # Validate spec syntax

# Interview (AI-driven)
intent interview --profile api --cue      # Start interview, output CUE
intent interview --session X --answer Y   # Submit answer, get next directive
intent beads <session> --cue              # Generate beads as CUE

# KIRK Analysis
intent quality <spec.cue>     # Quality scores (5 dimensions)
intent invert <spec.cue>      # What failure cases are missing?
intent coverage <spec.cue>    # HTTP method/status coverage
intent gaps <spec.cue>        # Gap detection via mental models

# EARS
intent ears <requirements.md> --output cue   # Parse EARS to CUE
```

## Workflows

### AI-Driven Spec Analysis Pipeline

When analyzing a spec (either for initial review or continuous improvement), follow this progressive workflow:

```bash
# 1. Syntax Validation (exit 0 = valid)
intent validate api.cue

# 2. Quality Baseline (get overall scores)
intent quality api.cue

# 3. Coverage Analysis (OWASP Top 10 + edge cases)
intent coverage api.cue

# 4. Gap Detection (mental model gaps)
intent gaps api.cue

# 5. Prioritized Fixes (actionable improvements)
intent doctor api.cue

# 6. Detailed Suggestions (specific fixes)
intent improve api.cue
```

**Why This Order?**

1. **validate**: Fast syntax check before deeper analysis
2. **quality**: Establishes baseline scores across 5 dimensions
3. **coverage**: Identifies security and edge case coverage gaps
4. **gaps**: Uses mental models to find missing requirements
5. **doctor**: Prioritizes all findings by impact
6. **improve**: Provides specific, actionable suggestions

**Machine-Readable Output**:

All analysis commands output structured JSON by default for programmatic processing. JSON output includes:
- Structured data (scores, findings, gaps)
- `next_actions` array with suggested follow-up commands
- Consistent exit codes for CI/CD integration

**Example: Full Analysis Script**

```bash
#!/bin/bash
SPEC="api.cue"

# Run analysis pipeline
intent validate $SPEC || exit 3
intent quality $SPEC > quality.json
intent coverage $SPEC > coverage.json
intent gaps $SPEC > gaps.json
intent doctor $SPEC > doctor.json

# Parse results (example with jq)
jq '.data.overall_score' quality.json
jq '.data.security_coverage' coverage.json
jq '.data.gap_count' gaps.json
jq '.data.recommendations[0]' doctor.json
```

### Spec Execution Workflow

```bash
# 1. Validate spec structure
intent validate api.cue

# 2. Check spec against target API
intent check api.cue --target=https://api.example.com

# 3. On failures, get improvement suggestions
intent doctor api.cue

# 4. After fixes, verify quality improved
intent quality api.cue
```

**Environment Variables**:

```bash
# Set default target URL
export INTENT_TARGET_URL=https://api.staging.example.com
intent check api.cue  # Uses INTENT_TARGET_URL automatically

# Allow localhost for development
export INTENT_ALLOW_LOCALHOST=true
intent check api.cue --target=http://localhost:8080
```

## Installation

```bash
# Build from source
gleam build

# Run
gleam run -- check examples/user-api.cue --target=http://localhost:8080
```

## Common Issues

### Flag Syntax

**IMPORTANT**: All flags require `--flag=value` syntax (with equals sign), not `--flag value`.

```bash
# ✅ CORRECT
intent check api.cue --target=https://api.com
intent interview --profile=api --cue=true

# ❌ WRONG
intent check api.cue --target https://api.com
intent interview --profile api
```

**Why**: The CLI uses Glint which only supports the `=` syntax. Using spaces will cause "flag has no assigned value" errors.

**Tip**: Boolean flags can omit the value if true:
```bash
intent interview --cue                # Same as --cue=true
```

### Command Aliases and Differences

**parse vs ears** - Both parse EARS requirements but serve different purposes:

```bash
# parse: Quick validation with pattern counts
intent parse requirements.md
# Output: ✓ Parsed 5 ubiquitous requirements, ✓ Parsed 3 event-driven requirements...

# ears: Detailed analysis with multiple output formats
intent ears requirements.md               # Detailed box format
intent ears requirements.md --output=cue  # Generate CUE spec
intent ears requirements.md --output=json # Machine-readable output
```

**When to use**:
- `parse`: Quick validation during editing, see pattern distribution
- `ears`: Full analysis, generating specs, or detailed requirement review

**analyze vs quality** - Identical output:

```bash
# analyze: Alias for quality
intent analyze api.cue

# quality: Standard command
intent quality api.cue
```

## Exit Codes

Intent CLI uses semantic exit codes to enable machine-readable error handling:

| Code | Meaning | Use Cases | CI/CD Action |
|------|---------|-----------|--------------|
| 0 | Success | Spec valid, tests pass, analysis complete | Continue pipeline |
| 1 | General failure | Tests failed, linting warnings found | Fail pipeline, review needed |
| 2 | Blocked behaviors | Check command found blocked behaviors | Investigate blocking issues |
| 3 | Invalid input | File not found, CUE parse error | Fix file path or syntax |
| 4 | Usage error | Missing required args, invalid flags | Fix command invocation |

**Examples**:

```bash
# Success (exit 0)
intent validate api.cue && echo "Valid spec"

# General failure (exit 1)
intent quality api.cue || echo "Quality issues found"

# Invalid input (exit 3)
intent validate missing.cue || echo "File not found or invalid"

# Usage error (exit 4)
intent check || echo "Missing required spec argument"
```

**CI/CD Integration**:

```yaml
# GitHub Actions example
- name: Validate API spec
  run: intent validate api.cue
  # Fails pipeline on exit code 1, 2, 3, or 4

- name: Quality check (non-blocking)
  run: intent quality api.cue || true
  # Continue pipeline even on failure

- name: Check for blocked behaviors
  run: |
    intent check api.cue --target=${{ secrets.API_URL }}
    if [ $? -eq 2 ]; then
      echo "::warning::Blocked behaviors detected"
    fi
```

## Project Structure

```
src/intent/
├── interview.gleam        # Interview engine (722 lines)
├── bead_templates.gleam   # Bead generation
├── kirk/
│   ├── ears_parser.gleam      # EARS → behaviors
│   ├── quality_analyzer.gleam # 5-dimension scoring
│   ├── inversion_checker.gleam # What could fail?
│   └── coverage_analyzer.gleam # Test coverage
└── ...

schema/
├── questions.cue          # Interview questions database
├── ai_protocol.cue        # AI directive schemas (coming)
├── kirk.cue              # KIRK contract types
└── intent.cue            # Core spec schema

docs/
├── MENTAL_LATTICE_FRAMEWORK.md   # Theory
├── EARS_KIRK_WORKFLOW.md         # Workflow
└── INTERACTIVE_QUESTIONING.md    # Question system
```

## The Goal

> By the time a bead reaches the AI, every possible question has been answered, every edge case has been enumerated, and the implementation is purely mechanical translation from specification to code.

**This is deterministic AI-assisted development.**

## Status

- Core CLI: Working
- Interview Engine: Working
- KIRK Analysis: Working
- EARS Parser: Working
- AI-CUE Protocol: In Progress (see beads)

## Documentation

### For AI Agents & Tool Builders

- **[JSON Schema Documentation](docs/JSON_SCHEMA.md)** - Complete reference for machine-readable JSON output
  - All command schemas (validate, check, quality, coverage, gaps, invert, effects, doctor, etc.)
  - TypeScript interfaces for type-safe integration
  - Error handling patterns
  - CI/CD integration examples
  - Python and Node.js usage examples

- **[TypeScript Definitions](schema/intent-cli.d.ts)** - Type-safe interfaces for all JSON responses

- **[JSON Schema Files](schema/json-schema/)** - Formal JSON Schema definitions for validation

- **[Integration Examples](examples/)** - Practical code examples
  - `json-integration.ts` - TypeScript/Node.js integration patterns
  - `json-integration.py` - Python integration patterns

#### Piping JSON Output

**Problem**: When running Intent CLI with `gleam run`, compilation progress messages pollute stdout, breaking JSON parsing:

```bash
# ❌ BROKEN - Progress messages corrupt JSON
gleam run -- validate api.cue | jq '.data.valid'
# Output: jq: parse error: Invalid numeric literal at line 1, column 12

# The actual output is:
# {"success":true,"action":"validate_result",...}
#    Compiled in 0.05s
#     Running intent.main
```

**Solution**: Use `--no-print-progress` flag to suppress Gleam's build output:

```bash
# ✅ WORKS - Clean JSON output
gleam run --no-print-progress -- validate api.cue | jq '.data.valid'
# Output: true
```

**Common Examples**:

```bash
# Extract quality score
gleam run --no-print-progress -- quality api.cue | jq '.data.overall_score'
# Output: 86

# Check if spec is valid
gleam run --no-print-progress -- validate api.cue | jq '.success'
# Output: true

# Get suggestions from doctor command
gleam run --no-print-progress -- doctor api.cue | jq '.data.suggestions'

# Extract specific fields from analysis
gleam run --no-print-progress -- quality api.cue | jq '{
  score: .data.overall_score,
  timestamp: .metadata.timestamp,
  valid: .success
}'
# Output:
# {
#   "score": 86,
#   "timestamp": "2026-01-30T07:51:03.490-06:00",
#   "valid": true
# }

# CI/CD pipeline integration
SCORE=$(gleam run --no-print-progress -- quality api.cue | jq '.data.overall_score')
if [ "$SCORE" -lt 80 ]; then
  echo "Quality score too low: $SCORE"
  exit 1
fi
```

**Which commands output JSON?**

All Intent CLI commands output structured JSON by default:
- `validate` - Validation results
- `check` - Test execution results with behavior outcomes
- `quality` / `analyze` - 5-dimension quality scores
- `coverage` - OWASP and edge case coverage analysis
- `gaps` - Mental model gap detection
- `invert` - Failure mode analysis
- `effects` - Second-order effects analysis
- `doctor` - Prioritized recommendations
- `improve` - Specific improvement suggestions
- `ears` / `parse` - EARS requirement parsing (with `--output=json`)

**Note**: The `--no-print-progress` flag is a **Gleam runtime flag**, not an Intent CLI flag. It must appear before the `--` separator that precedes Intent CLI arguments.

### For Humans

- **[User Guide](docs/USER_GUIDE.md)** - Getting started and common workflows
- **[API Reference](docs/API_REFERENCE.md)** - Command-line interface reference
- **[EARS Workflow](docs/EARS_KIRK_WORKFLOW.md)** - Requirements → Testing workflow
- **[Spec Format](docs/SPEC_FORMAT.md)** - CUE specification structure

### Architecture

- **[Architecture Analysis](docs/ARCHITECTURE_ANALYSIS.md)** - System design overview
- **[FFI Side Effects](docs/FFI_SIDE_EFFECTS.md)** - Foreign function interface documentation

## License

Apache 2.0
