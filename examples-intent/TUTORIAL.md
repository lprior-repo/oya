# Intent CLI Tutorial

Welcome to Intent CLI - an AI-guided planning framework that helps you transform vague goals into crystal-clear, atomic work items through systematic interviewing and rigorous decomposition.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Core Concepts](#core-concepts)
3. [Basic Workflows](#basic-workflows)
4. [Command Reference](#command-reference)
5. [Advanced Usage](#advanced-usage)
6. [AI Integration](#ai-integration)
7. [Error Handling](#error-handling)

---

## Getting Started

### Prerequisites

- Gleam installed
- Basic understanding of REST APIs
- Familiarity with JSON/CUE syntax

### Quick Start

```bash
# Build the CLI
gleam build

# Validate an example spec
gleam run -- validate examples/user-api.cue

# Check spec quality
gleam run -- quality examples/user-api.cue

# Get help
gleam run -- help
```

---

## Core Concepts

### What is a Spec?

A **Spec** (specification) is a CUE file that defines:
- API behaviors (requests and expected responses)
- Validation rules (response checks)
- Features and their intentions
- Success criteria and anti-patterns

### Spec Structure

Every Intent spec contains these required fields:

```cue
spec: intent.#Spec & {
    name:             "Your API Name"
    description:      "What this API does"
    version:          "1.0.0"
    audience:         "Who uses this API"
    success_criteria: ["List of criteria"]
    config: {
        base_url:   "http://localhost:8080"
        timeout_ms: 5000
    }
    features: [...]      // At least one feature
    rules: [...]         // Validation rules
    anti_patterns: [...]  // What to avoid
    ai_hints: {...}      // Implementation guidance
}
```

### Key Terminology

- **Feature**: A high-level capability (e.g., "User Registration")
- **Behavior**: A specific test case (e.g., "successful-registration")
- **Check**: A validation rule on responses (e.g., "password must be absent")
- **Capture**: Extract values from responses for later use
- **Requires**: Dependencies between behaviors (execution order)

---

## Basic Workflows

### Workflow 1: Start from Interview

Create a new API spec through guided questions.

```bash
# Start an interview session
gleam run -- interview api

# Answer questions to build your spec
# The tool will guide you through:
# - API purpose and audience
# - Features and behaviors
# - Success criteria
# - Error handling patterns

# List your sessions
gleam run -- sessions --profile=api

# Export to a CUE spec
gleam run -- export <session-id> --output=my-api.cue
```

**Example Output:**
```
Starting API interview session...
Session ID: session_abc123

Question 1: What is the primary purpose of this API?
> Manage user accounts with registration and authentication

Question 2: Who is the intended audience?
> Mobile and web applications
...
```

### Workflow 2: Validate Existing Spec

Check if your spec is syntactically correct.

```bash
# Validate CUE syntax
gleam run -- validate examples/user-api.cue
```

**Success Output:**
```
✓ Spec is valid
```

**Error Output:**
```
✗ Validation failed:
  field not allowed: extra_field
  at line 23
```

### Workflow 3: Analyze Quality

Get a comprehensive quality score across multiple dimensions.

```bash
# Analyze spec quality
gleam run -- quality examples/user-api.cue
```

**JSON Output Structure:**
```json
{
  "success": true,
  "action": "quality_result",
  "command": "quality",
  "data": {
    "overall_score": 85.0,
    "dimensions": {
      "coverage": 90.0,
      "clarity": 85.0,
      "testability": 80.0,
      "ai_readiness": 85.0
    },
    "details": {
      "feature_count": 3,
      "behavior_count": 12,
      "check_count": 24
    }
  },
  "next_actions": [
    {
      "command": "intent gaps examples/user-api.cue",
      "reason": "Identify missing coverage areas"
    }
  ]
}
```

### Workflow 4: Find Gaps

Detect missing requirements and coverage holes.

```bash
# Find gaps in your spec
gleam run -- gaps examples/user-api.cue
```

**Example Output:**
```json
{
  "success": true,
  "action": "gaps_result",
  "data": {
    "gaps": [
      {
        "type": "security",
        "severity": "high",
        "description": "No rate limiting behavior defined",
        "recommendation": "Add behavior testing rate limit responses"
      },
      {
        "type": "coverage",
        "severity": "medium",
        "description": "Missing edge case: empty request body",
        "recommendation": "Test POST /users with empty body"
      }
    ],
    "gap_count": 2
  }
}
```

### Workflow 5: Improve Your Spec

Get prioritized suggestions for improvement.

```bash
# Get improvement suggestions
gleam run -- improve examples/user-api.cue

# Get detailed health report with fixes
gleam run -- doctor examples/user-api.cue
```

**Example Output:**
```json
{
  "success": true,
  "data": {
    "suggestions": [
      {
        "title": "Add rate limiting behavior",
        "priority": "high",
        "impact": "Security",
        "effort": "Low",
        "reasoning": "Prevent abuse and DoS attacks"
      },
      {
        "title": "Test pagination edge cases",
        "priority": "high",
        "impact": "Reliability",
        "effort": "Medium",
        "reasoning": "Ensure robust handling of large datasets"
      }
    ]
  }
}
```

---

## Command Reference

### Core Spec Operations

#### validate

Check if spec has valid CUE syntax.

```bash
gleam run -- validate <spec.cue>
```

**When to use**: Before committing changes, in CI/CD pipelines

**Example:**
```bash
gleam run -- validate examples/user-api.cue
```

#### show

Display spec details in a readable format.

```bash
gleam run -- show <spec.cue>
```

**Example Output:**
```json
{
  "success": true,
  "data": {
    "spec": {
      "name": "User Management API",
      "version": "1.0.0",
      "audience": "Mobile and web clients"
    },
    "features": [
      {
        "name": "User Registration",
        "behaviors": ["successful-registration", "duplicate-email-rejected"]
      }
    ]
  }
}
```

#### analyze / quality

Analyze spec quality across 4 dimensions. (`analyze` is an alias for `quality`)

```bash
gleam run -- quality <spec.cue>
```

**Dimensions:**
- **Coverage**: How well behaviors cover the feature space
- **Clarity**: Intent statements, documentation quality
- **Testability**: Check definitions, validation rules
- **AI Readiness**: Hints, examples, anti-patterns for AI implementation

#### lint

Detect anti-patterns in your spec.

```bash
gleam run -- lint <spec.cue>
```

#### improve

Get actionable improvement suggestions.

```bash
gleam run -- improve <spec.cue>
```

### KIRK Analysis

KIRK (Knowledge Integration & Requirements Knowledge) provides deep analysis.

#### coverage

Analyze OWASP Top 10 and edge case coverage.

```bash
gleam run -- coverage <spec.cue>
```

**Example Output:**
```json
{
  "success": true,
  "action": "coverage_result",
  "data": {
    "owasp_coverage": {
      "injection": true,
      "broken_authentication": true,
      "sensitive_data_exposure": true,
      "xml_external_entities": false,
      "broken_access_control": true,
      "security_misconfiguration": false
    },
    "edge_cases": {
      "empty_inputs": 3,
      "max_length_inputs": 0,
      "special_characters": 2,
      "concurrent_requests": 0
    },
    "score": 65
  }
}
```

#### gaps

Find missing requirements using 5 gap detection models.

```bash
gleam run -- gaps <spec.cue>
```

**Gap Types:**
- Inversion gaps (missing failure modes)
- Second-order gaps (missing side effects)
- Checklist gaps (OWASP, edge cases)
- Coverage gaps (untested paths)
- Security gaps (authentication, authorization)

#### invert

Analyze failure modes and anti-patterns.

```bash
gleam run -- invert <spec.cue>
```

**Example Output:**
```json
{
  "success": true,
  "action": "invert_result",
  "data": {
    "failure_modes": [
      {
        "category": "security",
        "scenario": "SQL injection via email field",
        "mitigation": "Add behavior: malicious-email-rejected",
        "severity": "critical"
      },
      {
        "category": "usability",
        "scenario": "Race condition on simultaneous registration",
        "mitigation": "Document idempotency guarantees",
        "severity": "high"
      }
    ],
    "failure_count": 24
  }
}
```

#### effects

Identify second-order effects and side-effect behaviors.

```bash
gleam run -- effects <spec.cue>
```

**Example Output:**
```json
{
  "success": true,
  "data": {
    "effects": [
      {
        "cause": "User registration",
        "effects": [
          "Email verification sent",
          "Welcome email queued",
          "Analytics event fired"
        ],
        "verified": 1,
        "total": 3
      }
    ],
    "orphans": 2,
    "circular": 0
  }
}
```

#### ears

Parse EARS (Easy Approach to Requirements Syntax) requirements.

```bash
gleam run -- ears <requirements.md> [--output=cue|json]
```

**Example Input (`requirements.ears.md`):**
```markdown
# Requirements

THE SYSTEM SHALL validate all inputs
WHEN user registers THE SYSTEM SHALL send confirmation email
WHILE user is authenticated THE SYSTEM SHALL allow profile access
WHERE user is admin THE SYSTEM SHALL show admin panel
IF token is expired THEN THE SYSTEM SHALL NOT authorize requests
```

**Example Output (CUE):**
```cue
// Generated from EARS requirements

behaviors: [
  {
    name: "validate-inputs"
    intent: "System validates all inputs"
    type: "ubiquitous"
  },
  {
    name: "registration-sends-email"
    intent: "When user registers, send confirmation email"
    type: "event_driven"
    trigger: "user registers"
  }
]
```

### Interview Workflow

#### interview

Start an interactive specification discovery session.

```bash
gleam run -- interview <profile> [--resume=<session-id>] [--export=spec.cue]
```

**Profiles:**
- `api`: REST/GraphQL API development
- `cli`: Command-line tool development

**Example:**
```bash
# Start new API interview
gleam run -- interview api

# Resume previous session
gleam run -- interview api --resume=session_xyz789

# Start and export immediately when done
gleam run -- interview api --export=my-api.cue
```

#### sessions

List all interview sessions.

```bash
gleam run -- sessions [--profile=api|cli]
```

**Example Output:**
```
Interview Sessions

Recent Sessions:
  ID: session_abc123
  Profile: api
  Status: complete
  Created: 2024-01-15 10:30:00
  Questions: 15/15 answered

  ID: session_xyz789
  Profile: api
  Status: in_progress
  Created: 2024-01-16 14:22:00
  Questions: 8/15 answered
```

#### history

Show interview session snapshots.

```bash
gleam run -- history
```

#### diff

Compare two interview sessions.

```bash
gleam run -- diff <session-id1> <session-id2>
```

**Example Output:**
```
Comparing session_abc123 → session_xyz789

Changes:
  + Added feature: "Rate Limiting"
  ~ Modified: success_criteria[1]
    - "Users can authenticate"
    + "Users can authenticate with JWT tokens"
  - Removed behavior: "test-placeholder"
```

#### export

Export interview session to CUE spec.

```bash
gleam run -- export <session-id> [--output=spec.cue]
```

### Beads & Planning

Beads are atomic 5-30 minute work units generated from specs.

#### beads

Generate work items from an interview session.

```bash
gleam run -- beads <session-id> [--max-items=N]
```

**Example Output:**
```json
{
  "success": true,
  "action": "beads_result",
  "data": {
    "beads": [
      {
        "id": "bead_001",
        "type": "design",
        "title": "Design User schema",
        "description": "Define User entity with id, email, password_hash fields",
        "estimated_minutes": 15,
        "dependencies": []
      },
      {
        "id": "bead_002",
        "type": "implement",
        "title": "Implement POST /users endpoint",
        "description": "Create registration endpoint with validation",
        "estimated_minutes": 25,
        "dependencies": ["bead_001"]
      }
    ],
    "bead_count": 12,
    "total_minutes": 240
  }
}
```

#### beads-regenerate

Regenerate beads from a CUE spec (not session).

```bash
gleam run -- beads-regenerate <spec.cue>
```

#### bead-status

Update execution status of individual beads.

```bash
gleam run -- bead-status --bead-id <id> --status success|failed|blocked [--reason 'text']
```

**Examples:**
```bash
# Mark bead as successful
gleam run -- bead-status --bead-id bead_002 --status success

# Mark as failed with reason
gleam run -- bead-status --bead-id bead_003 --status failed --reason 'Missing dependency library'

# Mark as blocked
gleam run -- bead-status --bead-id bead_004 --status blocked --reason 'Waiting for API key'
```

#### plan

Generate execution plan with health analysis, waves, and beads.

```bash
gleam run -- plan <session-id> [--rounds=1..5]
```

**Rounds** refer to the 5-round mental model:
1. EARS (skeleton + patterns)
2. Contracts (response checks)
3. Inversion (anti-patterns + errors)
4. Effects (dependencies + verification)
5. Pre-mortem (AI hints + pitfalls)

**Example Output:**
```json
{
  "success": true,
  "data": {
    "health": {
      "overall_score": 85,
      "round_scores": {
        "round_1": 100,
        "round_2": 100,
        "round_3": 80,
        "round_4": 75,
        "round_5": 70
      }
    },
    "waves": [
      {
        "wave_id": 1,
        "beads": ["bead_001", "bead_002"],
        "parallel": true
      },
      {
        "wave_id": 2,
        "beads": ["bead_003", "bead_004", "bead_005"],
        "parallel": true,
        "blocks_on": [1]
      }
    ]
  }
}
```

#### plan-approve

Approve and finalize execution plan.

```bash
gleam run -- plan-approve <session-id> [--yes] [--notes 'text']
```

**Example:**
```bash
# Approve with confirmation prompt
gleam run -- plan-approve session_abc123

# Auto-approve without prompt
gleam run -- plan-approve session_abc123 --yes --notes 'Looks good, proceeding'
```

#### prompt

Generate AI implementation prompts from beads.

```bash
gleam run -- prompt <session-id> [--max-items=N]
```

**Example Output:**
```json
{
  "success": true,
  "data": {
    "prompts": [
      {
        "bead_id": "bead_002",
        "title": "Implement POST /users endpoint",
        "prompt": "Implement a user registration endpoint:\n\nEndpoint: POST /users\n\nInput:\n- email (string, validated)\n- password (string, min 8 chars)\n- name (string, optional)\n\nValidation:\n- Email format check\n- Password strength (1 uppercase, 1 number, 1 special char)\n- Duplicate email check\n\nResponse:\n- 201: {id, email, name, created_at}\n- 400: {error: {code, message}}\n- 409: {error: {code: 'EMAIL_EXISTS', message}}\n\nSecurity:\n- Hash password with bcrypt (cost >= 10)\n- Never return password in response\n- Use prefixed random ID (usr_xxxxx)\n\nContext:\nSee examples/user-api.cue behaviors:\n- successful-registration\n- duplicate-email-rejected\n- invalid-email-rejected"
      }
    ]
  }
}
```

#### feedback

Generate fix beads from test failures.

```bash
gleam run -- feedback --results <check-output.json>
```

**Example Input (`check-output.json`):**
```json
{
  "failures": [
    {
      "behavior": "successful-registration",
      "check": "password",
      "expected": "absent",
      "actual": "present",
      "path": "response.body.password"
    }
  ]
}
```

**Example Output:**
```json
{
  "success": true,
  "data": {
    "fix_beads": [
      {
        "id": "fix_001",
        "title": "Fix password exposure in registration response",
        "priority": "critical",
        "description": "Remove password field from POST /users response body",
        "estimated_minutes": 10,
        "related_behavior": "successful-registration",
        "related_check": "password"
      }
    ]
  }
}
```

### Utilities

#### doctor

Get prioritized health report with actionable fixes.

```bash
gleam run -- doctor <spec.cue>
```

**Example Output:**
```json
{
  "success": true,
  "data": {
    "critical_issues": [
      {
        "issue": "Missing security behavior: Rate limiting",
        "impact": "High",
        "fix": "Add behavior testing 429 responses",
        "effort": "20 minutes"
      }
    ],
    "warnings": [
      {
        "issue": "Weak check in Authentication.successful-login",
        "suggestion": "Use 'valid JWT' instead of 'token exists'"
      }
    ]
  }
}
```

### AI Commands

#### ai schema

Generate action JSON schema documentation for AI tools.

```bash
gleam run -- ai schema
```

**Use case**: Integrate Intent CLI with AI agents, provide schema for automated workflows.

---

## Advanced Usage

### Chaining Commands

Use JSON output with `jq` for powerful workflows:

```bash
# Get quality score and check if it meets threshold
SCORE=$(gleam run -- quality api.cue | jq '.data.overall_score')
if [ "$SCORE" -lt 80 ]; then
  echo "Quality below threshold, running doctor..."
  gleam run -- doctor api.cue
fi

# Extract high-priority gaps
gleam run -- gaps api.cue | \
  jq '.data.gaps[] | select(.severity == "high")'

# Count failure modes by category
gleam run -- invert api.cue | \
  jq '.data.failure_modes | group_by(.category) |
      map({category: .[0].category, count: length})'
```

### Behavior Dependencies

Use `requires` to control execution order:

```cue
behaviors: [
  {
    name: "create-user"
    // ... first behavior
    captures: {
      user_id: "response.body.id"
    }
  },
  {
    name: "login-user"
    requires: ["create-user"]  // Run after create-user
    request: {
      path: "/auth/login"
      // Can use captured values
      body: {email: "from create-user"}
    }
  },
  {
    name: "get-profile"
    requires: ["login-user"]  // Run after login
    request: {
      path: "/users/${user_id}"  // Use captured user_id
      headers: {
        "Authorization": "Bearer ${auth_token}"  // Use captured token
      }
    }
  }
]
```

### Captures and Variable Substitution

Extract values and reuse them:

```cue
{
  name: "create-resource"
  response: {
    status: 201
  }
  captures: {
    resource_id: "response.body.id"
    resource_url: "response.headers.Location"
  }
}

// Later behavior
{
  name: "update-resource"
  requires: ["create-resource"]
  request: {
    path: "/resources/${resource_id}"
    // Use captured value in path
  }
}
```

### Complex Check Rules

Validation rules support various operators:

```cue
checks: {
  "id": {
    rule: "string matching usr_[a-z0-9]+"
    why: "Prefixed random ID format"
  }
  "email": {
    rule: "equals request.body.email"
    why: "Echo input back"
  }
  "password": {
    rule: "absent"
    why: "Never expose passwords"
  }
  "created_at": {
    rule: "valid ISO8601 datetime"
    why: "Timestamp format"
  }
  "age": {
    rule: "integer >= 18"
    why: "Age validation"
  }
  "token": {
    rule: "valid JWT"
    why: "Proper token format"
  }
  "items": {
    rule: "non-empty array"
    why: "At least one item"
  }
  "status": {
    rule: "one of [active, pending, inactive]"
    why: "Valid enum value"
  }
}
```

---

## AI Integration

### Using JSON Output for Automation

All KIRK and analysis commands support structured JSON output for machine-readable output.

**Action JSON Schema:**
```json
{
  "success": true,
  "action": "<command>_result",
  "command": "<command>",
  "data": { /* command-specific data */ },
  "errors": [],
  "next_actions": [
    {
      "command": "intent <next-command>",
      "reason": "Why this is the logical next step"
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

### AI Workflow Example

```bash
# 1. Generate spec from interview
gleam run -- interview api --export=api.cue

# 2. Check quality
QUALITY=$(gleam run -- quality api.cue)
echo "$QUALITY" | jq '.data.overall_score'

# 3. Find gaps
gleam run -- gaps api.cue > gaps.json

# 4. Generate beads
gleam run -- beads <session-id> > beads.json

# 5. Create AI prompts
gleam run -- prompt <session-id> > prompts.json

# 6. Feed prompts to AI implementation tool
cat prompts.json | jq -r '.data.prompts[].prompt' | \
  your-ai-tool implement

# 7. Run tests and collect failures
# (your test runner outputs failures.json)

# 8. Generate fix beads
gleam run -- feedback --results failures.json > fixes.json
```

### Next Actions Guidance

Commands suggest logical next steps via `next_actions`:

```bash
# After quality check, follow suggestions
RESULT=$(gleam run -- quality api.cue)
NEXT=$(echo "$RESULT" | jq -r '.next_actions[0].command')
echo "Running: $NEXT"
eval "$NEXT"
```

**Common Next Actions:**
- After `quality`: Run `gaps` to find coverage holes
- After `gaps`: Run `improve` for suggestions
- After `improve`: Run `doctor` for detailed fixes
- After `interview`: Run `beads` to generate work items
- After `beads`: Run `prompt` for AI implementation

---

## Error Handling

### Common Errors and Solutions

#### 1. Validation Error: Missing Required Field

**Error:**
```
✗ Validation failed:
  field `audience` is required
  at spec.cue:5
```

**Solution:**
Add the missing required field:
```cue
spec: intent.#Spec & {
  name: "My API"
  description: "..."
  audience: "Mobile and web clients"  // Add this
  // ... rest of spec
}
```

#### 2. Validation Error: Empty Behaviors

**Error:**
```
✗ Validation failed:
  features[0].behaviors: list must have at least 1 item
```

**Solution:**
Every feature needs at least one behavior:
```cue
features: [
  {
    name: "User Registration"
    description: "..."
    behaviors: [  // Must have at least one
      {
        name: "successful-registration"
        intent: "User can create account"
        request: { /* ... */ }
        response: { /* ... */ }
      }
    ]
  }
]
```

#### 3. Invalid Check Rule

**Error:**
```
✗ Check failed:
  Unknown rule operator: "is equal to"
  Use: "equals"
```

**Solution:**
Use correct check rule syntax:
```cue
checks: {
  "email": {
    rule: "equals request.body.email"  // ✓ Correct
    // Not: "is equal to request.body.email"  ✗ Wrong
  }
}
```

#### 4. Circular Dependency

**Error:**
```
✗ Error: Circular dependency detected
  behavior-a requires behavior-b
  behavior-b requires behavior-c
  behavior-c requires behavior-a
```

**Solution:**
Remove circular dependency:
```cue
behaviors: [
  {name: "behavior-a", requires: []},           // Remove requires
  {name: "behavior-b", requires: ["behavior-a"]},
  {name: "behavior-c", requires: ["behavior-b"]}
]
```

#### 5. Interview Session Not Found

**Error:**
```
✗ Session not found: session_xyz123
```

**Solution:**
List available sessions:
```bash
gleam run -- sessions --profile=api
# Use an existing session ID or start a new interview
```

#### 6. Bead ID Not Found

**Error:**
```
✗ Bead not found: bead_999
```

**Solution:**
Generate beads first, then use valid IDs:
```bash
# Generate beads
gleam run -- beads <session-id>

# Use bead IDs from output
gleam run -- bead-status --bead-id bead_001 --status success
```

### JSON Error Format

Errors are structured:

```json
{
  "success": false,
  "action": "quality_result",
  "command": "quality",
  "data": {},
  "errors": [
    {
      "code": "VALIDATION_ERROR",
      "message": "field `audience` is required",
      "location": "spec.cue:5"
    }
  ],
  "metadata": {
    "exit_code": 1
  }
}
```

---

## Best Practices

### 1. Start with Intent Statements

Every behavior should have a clear `intent`:

```cue
{
  name: "duplicate-email-rejected"
  intent: "Cannot register with an email that's already taken"  // ✓ Clear
  // Not: intent: "Test duplicate"  ✗ Vague
}
```

### 2. Use Descriptive Check Whys

Explain the purpose of each check:

```cue
checks: {
  "password": {
    rule: "absent"
    why: "SECURITY: Never expose passwords"  // ✓ Clear purpose
    // Not: why: "Check password"  ✗ Vague
  }
}
```

### 3. Document Anti-Patterns

Include `bad_example` and `good_example`:

```cue
anti_patterns: [
  {
    name: "password-in-response"
    description: "NEVER return password in any response"
    bad_example: {
      id: "usr_123"
      password: "secret123"  // ✗
    }
    good_example: {
      id: "usr_123"
      // password field absent  ✓
    }
    why: "Passwords must never be exposed, even hashed"
  }
]
```

### 4. Use Captures for Reusability

Extract values once, reuse many times:

```cue
{
  name: "create-user"
  captures: {
    user_id: "response.body.id"
    auth_token: "response.body.token"
  }
}
// Later behaviors can use ${user_id} and ${auth_token}
```

### 5. Leverage Requires for Order

Build dependency chains:

```cue
behaviors: [
  {name: "register", requires: []},
  {name: "verify-email", requires: ["register"]},
  {name: "login", requires: ["verify-email"]},
  {name: "get-profile", requires: ["login"]}
]
```

### 6. Keep Behaviors Focused

One behavior = one test case:

```cue
// ✓ Good: Focused behaviors
{name: "successful-registration", intent: "Valid registration succeeds"}
{name: "duplicate-email-rejected", intent: "Duplicate email returns 409"}

// ✗ Bad: Testing multiple things
{name: "registration-tests", intent: "Test all registration scenarios"}
```

### 7. Use Tags for Organization

Tag behaviors for filtering and reporting:

```cue
{
  name: "sql-injection-test"
  tags: ["security", "owasp", "injection"]
  // ...
}
```

---

## Next Steps

1. **Try the workflows**: Run the example scripts in `examples/workflows/`
2. **Explore examples**: Study `examples/user-api.cue` and `examples/meal-planner-api.cue`
3. **Create your first spec**: Use `gleam run -- interview api` to get started
4. **Integrate with CI/CD**: Add `intent validate` and `intent quality` to your pipeline
5. **Automate with AI**: Use JSON output for automated analysis and implementation

---

## Resources

- **Examples**: `examples/` directory
- **Workflows**: `examples/workflows/` directory
- **CLAUDE.md**: Complete command reference
- **Schema**: CUE schema at `github.com/intent-cli/intent/schema:intent`

---

## Getting Help

```bash
# General help
gleam run -- help

# Command-specific help
gleam run -- <command> --help

# Example specs
ls examples/*.cue

# Workflow scripts
ls examples/workflows/*.sh
```

Happy testing!
