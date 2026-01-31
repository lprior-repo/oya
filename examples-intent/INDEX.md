# Intent CLI Examples Index

Complete guide to examples, workflows, and tutorials for Intent CLI.

## Documentation

| File | Purpose | When to Read |
|------|---------|--------------|
| [QUICKSTART.md](QUICKSTART.md) | 5-minute getting started guide | First time using Intent CLI |
| [TUTORIAL.md](TUTORIAL.md) | Comprehensive tutorial with all commands | Learning Intent CLI features |
| [workflows/README.md](workflows/README.md) | Workflow scripts documentation | Setting up automation |

## Quick Navigation

### I want to...

#### ...learn the basics
→ Start with [QUICKSTART.md](QUICKSTART.md)

#### ...create my first spec
→ Run `./workflows/new-api-spec.sh my-api.cue`
→ Or follow [TUTORIAL.md - Workflow 1](TUTORIAL.md#workflow-1-start-from-interview)

#### ...analyze an existing spec
→ Run `./workflows/analyze-existing.sh examples/user-api.cue`
→ Or see [TUTORIAL.md - Workflow 2](TUTORIAL.md#workflow-2-validate-existing-spec)

#### ...improve spec quality
→ Run `./workflows/improve-quality.sh my-api.cue`
→ Or see [TUTORIAL.md - Workflow 5](TUTORIAL.md#workflow-5-improve-your-spec)

#### ...integrate with AI tools
→ Run `./workflows/ai-automation.sh my-api.cue`
→ Or see [TUTORIAL.md - AI Integration](TUTORIAL.md#ai-integration)

#### ...understand all commands
→ Read [TUTORIAL.md - Command Reference](TUTORIAL.md#command-reference)

#### ...see example specs
→ Browse [Example Specs](#example-specs) below

## Example Specs

### user-api.cue
**Purpose**: User management with registration and authentication

**Features**:
- User Registration (create, validate, handle duplicates)
- Authentication (JWT tokens, login, error handling)
- Profile Management (get, update, authorization)

**Highlights**:
- ✓ Security best practices (no password exposure)
- ✓ Clear anti-patterns documentation
- ✓ Behavior dependencies with `requires`
- ✓ Captures for token and ID reuse
- ✓ Structured error responses

**Use as template for**:
- User management systems
- Authentication APIs
- CRUD operations with security

**Try it**:
```bash
gleam run -- validate examples/user-api.cue
gleam run -- quality examples/user-api.cue
gleam run -- gaps examples/user-api.cue
```

### meal-planner-api.cue
**Purpose**: Recipe scraping and meal planning

**Features**:
- Recipe Management (CRUD operations)
- Recipe Scraping (from URLs)
- Meal Planning (weekly plans)
- JSON Export

**Highlights**:
- ✓ Complex nested data structures
- ✓ Array validation patterns
- ✓ External integration (scraping)
- ✓ Nutritional information handling

**Use as template for**:
- Content aggregation APIs
- Recipe/meal planning systems
- Data import/export workflows

**Try it**:
```bash
gleam run -- validate examples/meal-planner-api.cue
gleam run -- invert examples/meal-planner-api.cue
gleam run -- coverage examples/meal-planner-api.cue
```

### array-validation.cue
**Purpose**: Demonstrate array validation patterns

**Features**:
- Array size constraints
- Element type validation
- Nested array handling

**Highlights**:
- ✓ Check rules for arrays
- ✓ Empty array edge cases
- ✓ Max length validation

**Use as template for**:
- List endpoints
- Bulk operations
- Collection validation

### regex-rules.cue
**Purpose**: Demonstrate regex-based validation

**Features**:
- Pattern matching in checks
- String format validation
- Complex regex patterns

**Highlights**:
- ✓ Email validation
- ✓ ID format patterns
- ✓ Custom regex rules

**Use as template for**:
- Format validation
- ID/code patterns
- Custom string rules

### nested-paths.cue
**Purpose**: Complex URL path patterns

**Features**:
- Nested resource paths
- Path parameter validation
- RESTful routing

**Highlights**:
- ✓ Multi-level nesting
- ✓ Path variable substitution
- ✓ Resource relationships

**Use as template for**:
- RESTful APIs
- Nested resources
- Complex routing

### requirements.ears.md
**Purpose**: EARS (Easy Approach to Requirements Syntax) examples

**Features**:
- Ubiquitous requirements (THE SYSTEM SHALL)
- Event-driven (WHEN...SHALL)
- State-driven (WHILE...SHALL)
- Optional (WHERE...SHALL)
- Unwanted (IF...SHALL NOT)

**Use as template for**:
- Requirements gathering
- Converting prose to specs
- Structured requirement docs

**Try it**:
```bash
gleam run -- ears examples/requirements.ears.md
gleam run -- parse examples/requirements.ears.md
```

## Workflow Scripts

All scripts are in `workflows/` directory and are executable.

### new-api-spec.sh
**End-to-end workflow for creating new specs**

```bash
./workflows/new-api-spec.sh my-api.cue
```

**Outputs**:
- `my-api.cue` - The spec
- `my-api-beads.json` - Work items
- `my-api-prompts.json` - AI prompts

**Time**: ~10-15 minutes (including interview)

### analyze-existing.sh
**Deep analysis of existing specs**

```bash
./workflows/analyze-existing.sh examples/user-api.cue
```

**Outputs**:
- `analysis-report.json` - Comprehensive report

**Time**: ~2 minutes

### improve-quality.sh
**Iterative quality improvement**

```bash
./workflows/improve-quality.sh my-api.cue --target-score=80
```

**Outputs**:
- `my-api.cue.backup.*` - Backup
- `my-api-improvements.md` - Checklist

**Time**: ~3 minutes (analysis only, improvements take longer)

### ai-automation.sh
**AI integration pipeline**

```bash
./workflows/ai-automation.sh examples/user-api.cue
```

**Outputs**:
- `ai-output/` directory with 10+ JSON files
- `ai-output/prompts/` - Individual prompt text files
- `ai-output/ai_context.json` - Consolidated context

**Time**: ~2 minutes

## Learning Paths

### Path 1: Complete Beginner
1. Read [QUICKSTART.md](QUICKSTART.md)
2. Run `gleam run -- validate examples/user-api.cue`
3. Run `./workflows/analyze-existing.sh examples/user-api.cue`
4. Read [TUTORIAL.md - Core Concepts](TUTORIAL.md#core-concepts)
5. Create first spec: `gleam run -- interview api`

### Path 2: Spec Author
1. Study [user-api.cue](user-api.cue) structure
2. Read [TUTORIAL.md - Spec Structure](TUTORIAL.md#spec-structure)
3. Run `./workflows/new-api-spec.sh my-api.cue`
4. Iterate with `./workflows/improve-quality.sh my-api.cue`
5. Read [TUTORIAL.md - Best Practices](TUTORIAL.md#best-practices)

### Path 3: Quality Engineer
1. Read [TUTORIAL.md - Command Reference](TUTORIAL.md#command-reference)
2. Run all analysis commands on example specs
3. Study [workflows/analyze-existing.sh](workflows/analyze-existing.sh)
4. Build custom quality gates
5. Integrate with CI/CD

### Path 4: AI Developer
1. Read [TUTORIAL.md - AI Integration](TUTORIAL.md#ai-integration)
2. Run `./workflows/ai-automation.sh examples/user-api.cue`
3. Study output JSON structure
4. Build custom AI workflows
5. Read [workflows/README.md - AI Patterns](workflows/README.md#pattern-4-ai-assisted-implementation)

## Command Categories

### Core Operations (4)
- `validate` - CUE syntax check
- `analyze` - Quality scoring (alias: `quality`)
- `lint` - Anti-pattern detection
- `improve` - Improvement suggestions

### KIRK Analysis (6)
- `quality` - 4-dimension scoring
- `coverage` - OWASP + edge cases
- `gaps` - Missing requirements
- `invert` - Failure modes
- `effects` - Second-order effects
- `ears` - Parse EARS requirements

### Interview (5)
- `interview` - Start session
- `sessions` - List sessions
- `history` - Show snapshots
- `diff` - Compare sessions
- `export` - Export to CUE

### Planning (7)
- `beads` - Generate work items
- `beads-regenerate` - From spec
- `bead-status` - Update status
- `plan` - Execution plan
- `plan-approve` - Approve plan
- `prompt` - AI prompts
- `feedback` - Fix beads from failures

### Utilities (3)
- `doctor` - Health report
- `show` - Display spec
- `help` - CLI help

### Parsing (2)
- `parse` - Quick EARS validation
- `ears` - Full EARS parsing

### AI (1)
- `ai schema` - Action JSON schema

**Total**: 32 commands

## JSON Output Structure

All analysis commands return this structure by default:

```json
{
  "success": true,
  "action": "<command>_result",
  "command": "<command>",
  "data": { /* command-specific */ },
  "errors": [],
  "next_actions": [
    {
      "command": "intent <next-cmd>",
      "reason": "Why to run this next"
    }
  ],
  "metadata": {
    "timestamp": "2024-01-15T10:30:00Z",
    "version": "0.1.0",
    "exit_code": 0
  },
  "spec_path": "examples/user-api.cue"
}
```

## Testing Examples

### Validation
```bash
# Valid spec
gleam run -- validate examples/user-api.cue
# Expected: ✓ Spec is valid

# All examples should be valid
for spec in examples/*.cue; do
    gleam run -- validate "$spec"
done
```

### Quality Analysis
```bash
# High quality spec
gleam run -- quality examples/user-api.cue
# Expected: Score >= 80

# JSON output
gleam run -- quality examples/user-api.cue | jq
```

### Coverage Analysis
```bash
# Check OWASP coverage
gleam run -- coverage examples/user-api.cue | \
    jq '.data.owasp_coverage'

# Count edge cases
gleam run -- coverage examples/user-api.cue | \
    jq '.data.edge_cases'
```

### Gap Detection
```bash
# Find all gaps
gleam run -- gaps examples/user-api.cue | \
    jq '.data.gaps'

# High severity gaps only
gleam run -- gaps examples/user-api.cue | \
    jq '.data.gaps[] | select(.severity == "high")'
```

### Failure Modes
```bash
# Critical failures
gleam run -- invert examples/user-api.cue | \
    jq '.data.failure_modes[] | select(.severity == "critical")'

# By category
gleam run -- invert examples/user-api.cue | \
    jq '.data.failure_modes | group_by(.category) |
        map({category: .[0].category, count: length})'
```

## Integration Examples

### CI/CD Pipeline
```bash
#!/bin/bash
# .github/workflows/spec-quality.sh

SPEC_FILE="api.cue"
MIN_SCORE=75

# Validate
gleam run -- validate "$SPEC_FILE" || exit 1

# Quality check
SCORE=$(gleam run -- quality "$SPEC_FILE" | \
    jq -r '.data.overall_score')

if [ "$SCORE" -lt "$MIN_SCORE" ]; then
    echo "Quality score $SCORE below minimum $MIN_SCORE"
    exit 1
fi

# Check for critical gaps
CRITICAL=$(gleam run -- gaps "$SPEC_FILE" | \
    jq '[.data.gaps[] | select(.severity == "critical")] | length')

if [ "$CRITICAL" -gt 0 ]; then
    echo "Found $CRITICAL critical gaps"
    exit 1
fi

echo "All checks passed"
```

### Pre-commit Hook
```bash
#!/bin/bash
# .git/hooks/pre-commit

for file in $(git diff --cached --name-only | grep '\.cue$'); do
    echo "Validating $file..."
    gleam run -- validate "$file" || exit 1
done
```

### AI Agent Integration
```python
# ai_agent.py
import json
import subprocess

def analyze_spec(spec_file):
    """Get AI context for spec analysis"""
    result = subprocess.run(
        ['./workflows/ai-automation.sh', spec_file],
        capture_output=True
    )

    with open('ai-output/ai_context.json') as f:
        return json.load(f)

def get_implementation_prompts(spec_file):
    """Get prompts for AI implementation"""
    context = analyze_spec(spec_file)
    return context['work_items']['prompts']

# Use it
prompts = get_implementation_prompts('my-api.cue')
for prompt in prompts:
    print(f"Implementing: {prompt['title']}")
    # Send to AI tool
```

## Contributing Examples

To add a new example:

1. Create `examples/new-example.cue`
2. Follow spec structure (see [TUTORIAL.md](TUTORIAL.md#spec-structure))
3. Validate: `gleam run -- validate examples/new-example.cue`
4. Document in this INDEX.md
5. Add description of use case and highlights

## Support

- **Documentation**: Start with [QUICKSTART.md](QUICKSTART.md)
- **Tutorial**: Read [TUTORIAL.md](TUTORIAL.md)
- **Workflows**: See [workflows/README.md](workflows/README.md)
- **Commands**: Run `gleam run -- help`
- **Project**: See main `README.md` in project root

## File Summary

| File | Lines | Purpose |
|------|-------|---------|
| QUICKSTART.md | ~250 | Fast 5-minute intro |
| TUTORIAL.md | ~1200 | Complete guide with examples |
| INDEX.md | ~500 | This file - navigation hub |
| workflows/README.md | ~600 | Workflow documentation |
| workflows/*.sh | ~2500 | Executable workflows (4 scripts) |
| *.cue | ~500-1000 each | Example specs (7 files) |

**Total documentation**: ~5000+ lines
**Example code**: ~5000+ lines
**Complete coverage**: All 32 commands documented with examples
