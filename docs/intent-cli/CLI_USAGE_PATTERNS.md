# Intent CLI Usage Patterns

This document defines consistent usage example patterns for all Intent CLI commands, organized by category.

## Table of Contents

1. [Command Categories](#command-categories)
2. [Testing Commands](#testing-commands)
3. [Quality Commands](#quality-commands)
4. [Interview Commands](#interview-commands)
5. [Planning Commands](#planning-commands)
6. [KIRK Commands](#kirk-commands)
7. [Utility Commands](#utility-commands)
8. [Common Flag Patterns](#common-flag-patterns)

## Command Categories

Intent CLI commands are organized into six categories:

| Category | Purpose | Commands |
|----------|---------|----------|
| Testing | Run tests against APIs | `check`, `validate` |
| Quality | Analyze spec quality | `lint`, `analyze`, `improve`, `doctor` |
| Interview | Guided spec discovery | `interview`, `sessions`, `history`, `diff`, `beads`, `bead-status` |
| Planning | Execution planning | `plan`, `plan-approve`, `beads-regenerate` |
| KIRK | Deep specification analysis | `quality`, `invert`, `coverage`, `gaps`, `effects`, `ears` |
| Utility | Data transformation | `show`, `export`, `parse` |

---

## Testing Commands

Commands that run tests against live APIs.

### check

Run a specification against a target API URL.

```bash
# Basic usage
intent check <spec.cue> --target <url>

# With verbose output
intent check spec.cue --target http://localhost:8080 --verbose

# Filter to specific feature
intent check spec.cue --target http://localhost:8080 --feature "User Management"

# Run only a specific behavior
intent check spec.cue --target http://localhost:8080 --only "create-user"

# Allow localhost for local development
intent check spec.cue --target http://localhost:3000 --allow-localhost

# Quiet mode (errors only)
intent check spec.cue --target http://localhost:8080 --quiet
```

**Flags:**
- `--target <url>` - Target base URL to test against (required)
- `--feature <name>` - Filter to a specific feature
- `--only <name>` - Run only a specific behavior
- `--verbose` - Verbose output with detailed information
- `--quiet` - Quiet output (errors only)
- `--allow-localhost` - Allow localhost URLs (bypasses SSRF protection)

**Exit Codes:**
- `0` - All checks passed
- `1` - One or more checks failed
- `2` - Blocked behaviors detected
- `3` - Invalid specification
- `4` - Error (missing arguments, etc.)

### validate

Validate CUE spec syntax and structure without running tests.

```bash
# Validate spec syntax
intent validate <spec.cue>

# Example
intent validate examples/api.cue
```

---

## Quality Commands

Commands that analyze specification quality without running tests.

### lint

Check spec for anti-patterns and common issues.

```bash
# Basic usage
intent lint <spec.cue>

# Example
intent lint examples/api.cue
```

### analyze

Analyze spec quality and completeness.

```bash
# Basic usage
intent analyze <spec.cue>

# Example
intent analyze examples/api.cue
```

### improve

Get improvement suggestions based on quality analysis and linting.

```bash
# Basic usage
intent improve <spec.cue>

# Example
intent improve examples/api.cue
```

### doctor

Comprehensive health report with prioritized improvements.

```bash
# Basic usage
intent doctor <spec.cue>
```

---

## Interview Commands

Commands for guided specification discovery through structured interviews.

### interview

Start a guided specification discovery session.

```bash
# Start new interview (default: API profile)
intent interview

# Start with specific profile
intent interview --profile <profile>

# Resume existing session
intent interview --resume <session-id>

# Export completed interview to spec
intent interview --profile api --export output.cue

# Non-interactive mode with pre-filled answers
intent interview --profile api --answers answers.cue

# Strict mode (fail on missing answers)
intent interview --profile api --answers answers.cue --strict

# AI agent CUE mode (outputs structured directives)
intent interview --cue --profile api

# Submit answer in CUE mode
intent interview --cue --session <session-id> --answer "THE SYSTEM SHALL validate inputs"

# Dry-run mode (preview without saving)
intent interview --cue --profile api --dry-run
```

**Profiles:**
- `api` - REST API specifications (default)
- `cli` - Command-line interface specifications
- `event` - Event-driven system specifications
- `data` - Data pipeline specifications
- `workflow` - Business workflow specifications
- `ui` - User interface specifications

**Flags:**
- `--profile <type>` - System profile (api, cli, event, data, workflow, ui)
- `--resume <id>` - Resume existing interview session by ID
- `--export <path>` - Export completed interview to spec file
- `--answers <path>` - Path to CUE file with pre-filled answers
- `--strict` - Strict mode (requires --answers)
- `--cue` - Output CUE directives for AI agents
- `--session <id>` - Session ID for CUE mode
- `--answer <text>` - Submit answer (use with --cue --session)
- `--dry-run` - Preview without saving to sessions.jsonl

### sessions

List all interview sessions.

```bash
# List all sessions
intent sessions

# Filter by profile
intent sessions --profile api

# Show only incomplete sessions
intent sessions --incomplete
```

**Flags:**
- `--profile <type>` - Filter by profile (api, cli, event, etc.)
- `--incomplete` - Show only incomplete sessions

### history

View snapshot history for an interview session.

```bash
# View session history
intent history <session-id>

# Example
intent history interview-abc123def456
```

### diff

Compare two interview sessions and show differences.

```bash
# Compare two sessions
intent diff <from-session> <to-session>

# Example
intent diff interview-abc123 interview-def456
```

### beads

Generate work items (beads) from an interview session.

```bash
# Generate beads (JSON output)
intent beads <session-id>
```

### bead-status

Mark bead execution status (success, failed, or blocked).

```bash
# Mark bead as success
intent bead-status --bead-id <id> --status success

# Mark bead as failed with reason
intent bead-status --bead-id <id> --status failed --reason "Validation error"

# Mark bead as blocked (reason required)
intent bead-status --bead-id <id> --status blocked --reason "Waiting for dependency"

# Include session reference
intent bead-status --bead-id <id> --status success --session <session-id>
```

**Flags:**
- `--bead-id <id>` - Bead ID (required)
- `--status <status>` - Status: success, failed, or blocked (required)
- `--reason <text>` - Reason for status (required for blocked)
- `--session <id>` - Session ID

---

## Planning Commands

Commands for execution planning and workflow management.

### plan

Display execution plan for a session.

```bash
# Basic usage
intent plan <session-id>

# JSON output (if format flag is supported, otherwise default)
intent plan <session-id> --format json
```

**Flags:**
- `--format <type>` - Output format: human or json (default: human)

### plan-approve

Approve execution plan for CI/automation.

```bash
# Interactive approval
intent plan-approve <session-id>

# Auto-approve for CI pipelines
intent plan-approve <session-id> --yes

# Add approval notes
intent plan-approve <session-id> --yes --notes "Reviewed and approved"
```

**Flags:**
- `--yes` - Auto-approve for CI (non-interactive)
- `--notes <text>` - Approval notes

### beads-regenerate

Regenerate failed or blocked beads with adjusted approach.

```bash
# Regenerate with default strategy (hybrid)
intent beads-regenerate <session-id>

# Regenerate with specific strategy
intent beads-regenerate <session-id> --strategy inversion
intent beads-regenerate <session-id> --strategy premortem
```

**Strategies:**
- `hybrid` - Use all analysis methods (default)
- `inversion` - Focus on failure mode analysis
- `premortem` - Focus on what could go wrong

**Flags:**
- `--strategy <type>` - Regeneration strategy (hybrid, inversion, premortem)

---

## KIRK Commands

KIRK (Knowledge-Informed Requirements Kernel) commands for deep specification analysis.

### quality

Analyze spec quality across multiple dimensions.

```bash
# Basic usage (JSON output)
intent quality <spec.cue>
```

**Quality Dimensions:**
- Completeness - How complete is the specification?
- Consistency - Are behaviors internally consistent?
- Testability - Can behaviors be easily tested?
- Clarity - Are intents clear and unambiguous?
- Security - Are security concerns addressed?

### invert

Inversion analysis - identify missing failure cases.

```bash
# Basic usage
intent invert <spec.cue>
```

### coverage

Coverage analysis including OWASP Top 10 security coverage.

```bash
# Basic usage
intent coverage <spec.cue>
```

### gaps

Detect specification gaps using mental models.

```bash
# Basic usage
intent gaps <spec.cue>
```

### effects

Analyze second-order effects (consequence tracing).

```bash
# Basic usage
intent effects <spec.cue>
```

### ears

Parse EARS requirements to Intent behaviors.

```bash
# Parse and display (text format)
intent ears <requirements.md>

# Output as CUE spec
intent ears <requirements.md> --output cue

# Output as JSON
intent ears <requirements.md> --output json

# Write to file
intent ears <requirements.md> --output cue --out spec.cue

# Specify spec name for CUE output
intent ears <requirements.md> --output cue --name "MyAPISpec"
```

**EARS Patterns:**
- `THE SYSTEM SHALL [behavior]` - Ubiquitous
- `WHEN [trigger] THE SYSTEM SHALL [behavior]` - Event-Driven
- `WHILE [state] THE SYSTEM SHALL [behavior]` - State-Driven
- `WHERE [condition] THE SYSTEM SHALL [behavior]` - Optional
- `IF [condition] THEN THE SYSTEM SHALL NOT [behavior]` - Unwanted
- `WHILE [state] WHEN [trigger] THE SYSTEM SHALL [behavior]` - Complex

**Flags:**
- `--output <format>` - Output format: text, cue, json (default: text)
- `--out <path>` - Output file path
- `--name <name>` - Spec name for CUE output (default: GeneratedSpec)

---

## Utility Commands

Commands for data transformation and display.

### show

Pretty print a parsed spec.

```bash
# Basic usage
intent show <spec.cue>
```

### export

Export spec to JSON format.

```bash
# Export to stdout
intent export <spec.cue>

# Redirect to file
intent export spec.cue > spec.json
```

### parse

Parse EARS requirements and output structured CUE spec.

```bash
# Parse requirements
intent parse <requirements.md>

# Output to CUE file
intent parse <requirements.md> -o <output.cue>
```

**Flags:**
- `-o <path>` - Output spec file path

---

## Common Flag Patterns

### Output Format Flags

Most commands output JSON by default for machine consumption. Some support specific formats:

```bash
# Format selection (for commands with multiple formats)
intent <command> <args> --format json
intent <command> <args> --output json
```

### File Output

Commands that can write to files:

```bash
# Short form
intent parse requirements.md -o spec.cue

# Long form
intent ears requirements.md --out spec.cue
intent interview --profile api --export spec.cue
```

### Filtering

Commands that support filtering:

```bash
# By feature name
intent check spec.cue --target url --feature "User Management"

# By behavior name
intent check spec.cue --target url --only "create-user"

# By profile
intent sessions --profile api

# By status
intent sessions --incomplete
```

### Verbosity

```bash
# More output
intent check spec.cue --target url --verbose

# Less output
intent check spec.cue --target url --quiet
```

### CI/Automation Mode

```bash
# Auto-approve
intent plan-approve session-id --yes

# JSON output for parsing (default)
intent check spec.cue --target url
intent doctor spec.cue
intent sessions
```

---

## Quick Reference

### Testing Workflow

```bash
# 1. Validate spec syntax
intent validate api.cue

# 2. Analyze quality
intent doctor api.cue

# 3. Run tests
intent check api.cue --target http://localhost:8080
```

### Interview Workflow

```bash
# 1. Start interview
intent interview --profile api

# 2. List sessions (find your session ID)
intent sessions

# 3. Generate work items
intent beads interview-abc123

# 4. View execution plan
intent plan interview-abc123

# 5. Approve plan
intent plan-approve interview-abc123 --yes
```

### KIRK Analysis Workflow

```bash
# 1. Quality analysis
intent quality api.cue

# 2. Coverage check
intent coverage api.cue

# 3. Gap detection
intent gaps api.cue

# 4. Inversion analysis (find missing failure cases)
intent invert api.cue

# 5. Second-order effects
intent effects api.cue
```

### EARS to Spec Workflow

```bash
# 1. Parse EARS requirements
intent ears requirements.md --output text

# 2. Generate CUE spec
intent ears requirements.md --output cue --out api.cue

# 3. Validate generated spec
intent validate api.cue

# 4. Run quality analysis
intent doctor api.cue
```
