# Intent CLI Quick Start Guide

Get started with Intent CLI in 5 minutes.

## Installation

```bash
# Clone and build
git clone <repo-url>
cd intent-cli
gleam build
```

## 30-Second Test Drive

```bash
# Validate an example spec
gleam run -- validate examples/user-api.cue

# Check quality
gleam run -- quality examples/user-api.cue

# Find gaps
gleam run -- gaps examples/user-api.cue | jq
```

## Create Your First Spec (5 minutes)

```bash
# Start interactive interview
gleam run -- interview api

# Answer the questions:
# - What is your API for?
# - Who will use it?
# - What are the main features?
# - What behaviors should it support?

# Export to CUE spec
gleam run -- sessions  # Get your session ID
gleam run -- export <session-id> --output=my-api.cue

# Validate it
gleam run -- validate my-api.cue
```

## Common Commands Cheat Sheet

### Analysis Commands
```bash
# Validate CUE syntax
gleam run -- validate <spec.cue>

# Quality score (4 dimensions)
gleam run -- quality <spec.cue>

# OWASP coverage
gleam run -- coverage <spec.cue>

# Find missing requirements
gleam run -- gaps <spec.cue>

# Failure mode analysis
gleam run -- invert <spec.cue>

# Get improvement suggestions
gleam run -- improve <spec.cue>

# Health report with fixes
gleam run -- doctor <spec.cue>
```

### Interview & Planning
```bash
# Start interview
gleam run -- interview api

# List sessions
gleam run -- sessions --profile=api

# Export session to spec
gleam run -- export <session-id> --output=spec.cue

# Generate work items
gleam run -- beads <session-id>

# Generate AI prompts
gleam run -- prompt <session-id>

# Create execution plan
gleam run -- plan <session-id>
```

### Workflow Scripts
```bash
# Complete new spec workflow
./examples/workflows/new-api-spec.sh my-api.cue

# Analyze existing spec
./examples/workflows/analyze-existing.sh examples/user-api.cue

# Improve quality
./examples/workflows/improve-quality.sh my-api.cue --target-score=80

# AI automation pipeline
./examples/workflows/ai-automation.sh my-api.cue
```

## Understanding Your Spec

A spec has these key parts:

```cue
spec: intent.#Spec & {
  name: "My API"              // Required
  description: "..."          // Required
  audience: "..."             // Required
  success_criteria: [...]     // Required

  config: {                   // Required
    base_url: "..."
    timeout_ms: 5000
  }

  features: [                 // At least one
    {
      name: "Feature Name"
      description: "..."
      behaviors: [            // At least one
        {
          name: "behavior-name"
          intent: "What this tests"
          request: {...}
          response: {
            status: 200
            checks: {...}     // Validation rules
          }
        }
      ]
    }
  ]

  rules: [...]               // Global rules
  anti_patterns: [...]       // What to avoid
  ai_hints: {...}           // Implementation hints
}
```

## Quality Scores Explained

- **Coverage** (0-100): How well behaviors cover the feature space
- **Clarity** (0-100): Intent statements, documentation quality
- **Testability** (0-100): Check definitions, validation rules
- **AI Readiness** (0-100): Hints, examples, anti-patterns

**Target Scores:**
- Production: ≥ 80
- Development: ≥ 60
- Prototype: ≥ 40

## Common Workflows

### 1. New API Project
```bash
# Create spec
./examples/workflows/new-api-spec.sh my-api.cue

# Review quality
gleam run -- quality my-api.cue

# If score < 80, improve
./examples/workflows/improve-quality.sh my-api.cue
```

### 2. Existing Spec Review
```bash
# Deep analysis
./examples/workflows/analyze-existing.sh spec.cue

# Check score
gleam run -- quality spec.cue | jq '.data.overall_score'

# Get improvements
gleam run -- doctor spec.cue
```

### 3. AI-Assisted Development
```bash
# Generate AI artifacts
./examples/workflows/ai-automation.sh spec.cue

# Use prompts
cat ai-output/prompts/bead_001.txt | ai-tool implement

# Or use consolidated context
cat ai-output/ai_context.json | ai-tool analyze
```

## JSON Output

All analysis commands output structured JSON by default for machine-readable output:

```bash
# Get JSON output
gleam run -- quality spec.cue > quality.json

# Extract specific data
jq '.data.overall_score' quality.json
jq '.data.dimensions' quality.json
jq '.next_actions' quality.json
```

## Next Actions

Commands suggest logical next steps:

```bash
# Get next actions from quality check
gleam run -- quality spec.cue | jq '.next_actions'

# Output:
# [
#   {
#     "command": "intent gaps spec.cue",
#     "reason": "Find coverage gaps"
#   },
#   {
#     "command": "intent invert spec.cue",
#     "reason": "Analyze failure modes"
#   }
# ]
```

## Examples

Study these example specs:

```bash
# User management API
cat examples/user-api.cue

# Meal planning API
cat examples/meal-planner-api.cue

# Run analysis on examples
gleam run -- quality examples/user-api.cue
gleam run -- gaps examples/meal-planner-api.cue
```

## Troubleshooting

### Validation fails
```bash
# Check CUE syntax
gleam run -- validate spec.cue

# Common issues:
# - Missing required fields (name, description, audience, etc.)
# - Empty behaviors array
# - Invalid check rule syntax
```

### Low quality score
```bash
# Get detailed report
gleam run -- doctor spec.cue

# Or use improvement workflow
./examples/workflows/improve-quality.sh spec.cue
```

### No session found
```bash
# Create interview session first
gleam run -- interview api

# Then get session ID
gleam run -- sessions
```

## Getting Help

```bash
# General help
gleam run -- help

# View tutorial
cat examples/TUTORIAL.md

# View workflows
ls examples/workflows/
cat examples/workflows/README.md
```

## Next Steps

1. **Read the tutorial**: `examples/TUTORIAL.md`
2. **Try workflows**: `examples/workflows/`
3. **Study examples**: `examples/*.cue`
4. **Build your spec**: Start with `gleam run -- interview api`

Happy testing!
