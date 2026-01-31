# KIRK Implementation Plan for Intent CLI

## Executive Summary

This document provides concrete implementation steps to evolve Intent CLI into KIRK - the ultimate planning, contract, and testing tool.

---

## Quick Wins (Can Implement Now)

### 1. Add `why` Enforcement

Every check should explain its purpose. Update the validator:

```gleam
// In validator.gleam - add check
fn validate_check_has_why(check: Check) -> Result(Nil, ValidationIssue) {
  case string.is_empty(check.why) {
    True -> Error(MissingWhy(check.rule))
    False -> Ok(Nil)
  }
}
```

**Impact:** Forces specification clarity, improves AI implementation guidance.

### 2. Add Inversion Checklist Command

New CLI command to generate anti-pattern suggestions:

```bash
intent invert <spec.cue>
```

Output:
```
Analyzing spec for missing inversions...

SECURITY INVERSIONS MISSING:
  [ ] SQL injection payload in request body
  [ ] XSS payload in user-controlled fields
  [ ] JWT tampering/expiry handling

USABILITY INVERSIONS MISSING:
  [ ] Empty list pagination response
  [ ] Maximum request size exceeded

INTEGRATION INVERSIONS MISSING:
  [ ] Concurrent modification conflict
  [ ] Idempotency key collision

Suggested behaviors to add: 8
```

### 3. Quality Score Command

```bash
intent quality <spec.cue> --json
```

```json
{
  "completeness": 95,
  "testability": 100,
  "clarity": 85,
  "security_coverage": 70,
  "overall": 87,
  "issues": [
    {"field": "get-profile.checks.email.why", "issue": "missing"},
    {"category": "security", "issue": "no rate-limit testing"}
  ]
}
```

---

## Phase 1: Enhanced Mental Model Fields

### 1.1 Schema Changes

Add to `schema/intent.cue`:

```cue
#Spec: {
    // ... existing fields ...

    // NEW: Inversion thinking
    inversions?: {
        security_failures?: [...string]
        usability_failures?: [...string]
        integration_failures?: [...string]
    }

    // NEW: Pre-mortem analysis
    pre_mortem?: {
        assumed_failure: string
        likely_causes: [...{
            cause: string
            probability: "high" | "medium" | "low"
            mitigation: string
        }]
    }
}

#Behavior: {
    // ... existing fields ...

    // NEW: Second-order effects
    second_order_effects?: [...string]

    // NEW: Explicit preconditions
    preconditions?: {
        auth_required?: bool
        required_fields?: [...string]
        field_constraints?: {[string]: string}
    }
}
```

### 1.2 Parser Updates

```gleam
// In parser.gleam - add new type
pub type Inversions {
  Inversions(
    security_failures: List(String),
    usability_failures: List(String),
    integration_failures: List(String),
  )
}

pub type PreMortem {
  PreMortem(
    assumed_failure: String,
    likely_causes: List(LikelyCause),
  )
}

pub type LikelyCause {
  LikelyCause(
    cause: String,
    probability: String,
    mitigation: String,
  )
}
```

### 1.3 Interview Enhancement

Add new interview rounds:

```gleam
// In interview.gleam
pub const interview_rounds = [
  // Existing rounds...

  // NEW: Inversion round
  Round(
    name: "inversion",
    questions: [
      "What would make this API fail catastrophically?",
      "What security vulnerabilities could exist?",
      "What would frustrate users the most?",
      "What integration issues might downstream systems face?",
    ],
  ),

  // NEW: Pre-mortem round
  Round(
    name: "pre_mortem",
    questions: [
      "Imagine this API launched and failed after 1 week. Why?",
      "What's the most likely cause of that failure?",
      "What would you do differently if you knew that would happen?",
    ],
  ),
]
```

---

## Phase 2: Token-Efficient Compact Format

### 2.1 Design Compact Intent Notation (CIN)

Grammar (EBNF):
```
spec     = header features rules
header   = "SPEC" name version NL description NL
feature  = "F" name NL behaviors
behavior = "B" name intent NL request response
request  = method path [headers] [body] NL
response = status NL checks
check    = field ":" rule [why] NL
```

Example:
```
SPEC "User API" 1.0.0
User management with auth

F "Registration"
B create-user "New user creates account"
  POST /users {"email":"u@e.com","password":"Pass1!"}
  201
    id: uuid "Unique identifier"
    password: absent "Security"
    created_at: iso8601 "Audit"
  >> user_id: id

B duplicate-email "Email uniqueness enforced"
  <- create-user
  POST /users {"email":"u@e.com","password":"Other1!"}
  409
    error.code: eq EMAIL_EXISTS "Client handling"
```

Syntax elements:
- `<<` = requires (dependency)
- `>>` = captures
- `:` separates field from rule
- Text in quotes after rule = why

### 2.2 Bidirectional Converter

```bash
# CUE to CIN (for AI prompts)
intent compact <spec.cue> > spec.cin

# CIN to CUE (from AI output)
intent expand <spec.cin> > spec.cue
```

### 2.3 Token Benchmarks

| Format | user-api.cue | Tokens | Reduction |
|--------|--------------|--------|-----------|
| CUE    | 484 lines    | ~4200  | baseline  |
| JSON   | 650 lines    | ~5100  | +21%      |
| CIN    | 180 lines    | ~2100  | -50%      |

---

## Phase 3: AI Implementation Consistency

### 3.1 Structured Prompt Generator

New command:
```bash
intent prompt <spec.cue> --behavior create-user --format openai
```

Output:
```json
{
  "role": "system",
  "content": "You are implementing an API endpoint. Follow the contract exactly."
},
{
  "role": "user",
  "content": "## Implement: create-user\n\n### Contract\n..."
}
```

### 3.2 Implementation Validation

After AI generates code, validate against spec:

```bash
# Start mock server with AI-generated code
intent validate <spec.cue> --implementation ./src/routes/users.ts
```

Checks:
- Route paths match spec
- Response schemas compatible
- Error codes present
- Security patterns followed

### 3.3 Enhanced ai_hints Structure

```cue
ai_hints: {
    // Exact implementation patterns
    code_patterns: {
        error_handling: """
            try {
              // operation
            } catch (error) {
              if (error instanceof ValidationError) {
                return res.status(400).json({ error: { code: error.code, message: error.message } });
              }
              throw error;
            }
            """

        auth_middleware: """
            const authMiddleware = (req, res, next) => {
              const token = req.headers.authorization?.replace('Bearer ', '');
              if (!token) return res.status(401).json({ error: { code: 'UNAUTHORIZED' } });
              // verify
            };
            """
    }

    // Type definitions
    type_definitions: {
        typescript: """
            interface User {
              id: string;  // Format: usr_[a-z0-9]+
              email: string;
              name: string;
              created_at: string;  // ISO8601
              updated_at: string;  // ISO8601
            }
            """
    }

    // Database schema
    database_schema: {
        postgresql: """
            CREATE TABLE users (
              id VARCHAR(20) PRIMARY KEY,
              email VARCHAR(255) UNIQUE NOT NULL,
              password_hash VARCHAR(60) NOT NULL,
              name VARCHAR(100) NOT NULL,
              created_at TIMESTAMPTZ DEFAULT NOW(),
              updated_at TIMESTAMPTZ DEFAULT NOW()
            );
            """
    }
}
```

---

## Phase 4: Quality Metrics System

### 4.1 Quality Analyzer Module

```gleam
// New file: src/intent/quality_analyzer.gleam

pub type QualityReport {
  QualityReport(
    completeness: Float,
    consistency: Float,
    testability: Float,
    clarity: Float,
    security: Float,
    overall: Float,
    issues: List(QualityIssue),
  )
}

pub fn analyze_quality(spec: Spec) -> QualityReport {
  let completeness = calculate_completeness(spec)
  let consistency = check_consistency(spec)
  let testability = measure_testability(spec)
  let clarity = assess_clarity(spec)
  let security = evaluate_security_coverage(spec)

  QualityReport(
    completeness: completeness,
    consistency: consistency,
    testability: testability,
    clarity: clarity,
    security: security,
    overall: weighted_average([
      #(completeness, 0.2),
      #(consistency, 0.2),
      #(testability, 0.25),
      #(clarity, 0.15),
      #(security, 0.2),
    ]),
    issues: collect_issues(spec),
  )
}
```

### 4.2 Coverage Report

```gleam
pub type CoverageReport {
  CoverageReport(
    methods: Dict(String, Int),
    status_codes: Dict(String, Int),
    paths: Dict(String, List(String)),
    edge_cases: EdgeCaseCoverage,
  )
}

pub type EdgeCaseCoverage {
  EdgeCaseCoverage(
    tested: List(String),
    suggested: List(String),
  )
}
```

### 4.3 OWASP Top 10 Checklist

```gleam
pub const owasp_top_10 = [
  #("A01", "Broken Access Control", ["unauthorized-access", "privilege-escalation"]),
  #("A02", "Cryptographic Failures", ["password-exposure", "weak-encryption"]),
  #("A03", "Injection", ["sql-injection", "command-injection", "xss"]),
  #("A04", "Insecure Design", ["business-logic-flaws"]),
  #("A05", "Security Misconfiguration", ["verbose-errors", "default-credentials"]),
  #("A06", "Vulnerable Components", []),
  #("A07", "Auth Failures", ["brute-force", "session-fixation"]),
  #("A08", "Data Integrity", ["deserialization", "ci-cd-tampering"]),
  #("A09", "Logging Failures", ["insufficient-logging"]),
  #("A10", "SSRF", ["ssrf-payload"]),
]

pub fn check_owasp_coverage(spec: Spec) -> List(#(String, Bool)) {
  // Check if behaviors exist that test each category
}
```

---

## Phase 5: Interview 5x5 Matrix

### 5.1 Structured Interview Flow

```gleam
pub type InterviewMatrix {
  rounds: List(InterviewRound),
  perspectives: List(Perspective),
}

pub type InterviewRound {
  InterviewRound(
    name: String,
    focus: String,
    questions_by_perspective: Dict(String, List(String)),
  )
}

pub const interview_matrix = InterviewMatrix(
  rounds: [
    InterviewRound(
      name: "core_intent",
      focus: "What are we building?",
      questions_by_perspective: dict.from_list([
        #("user", ["What problem does this solve?", "How will users interact with it?"]),
        #("developer", ["What are the main components?", "What's the data model?"]),
        #("ops", ["What scale do we need?", "What SLAs?"]),
        #("security", ["What data is sensitive?", "Who should have access?"]),
        #("business", ["What metrics matter?", "What's the timeline?"]),
      ]),
    ),
    // ... more rounds
  ],
)
```

### 5.2 Gap Detection Enhancement

```gleam
pub type Gap {
  Gap(
    gap_type: GapType,
    description: String,
    severity: Severity,
    suggestion: String,
    mental_model: Option(String),  // Which model revealed this
  )
}

pub type GapType {
  InversionGap      // Missing failure case
  SecondOrderGap    // Missing consequence
  ChecklistGap      // Missing from standard checklist
  CoverageGap       // Missing HTTP method/status/path
  SecurityGap       // Missing security test
}
```

---

## File Structure

```
intent-cli/
├── src/
│   └── intent/
│       ├── mental_models/
│       │   ├── inversion.gleam      # Inversion analysis
│       │   ├── pre_mortem.gleam     # Pre-mortem generation
│       │   └── second_order.gleam   # Dependency effects
│       ├── quality/
│       │   ├── analyzer.gleam       # Quality scoring
│       │   ├── coverage.gleam       # Coverage metrics
│       │   └── owasp.gleam          # Security checklist
│       ├── compact/
│       │   ├── cin_parser.gleam     # CIN format parser
│       │   ├── cin_writer.gleam     # CIN format writer
│       │   └── converter.gleam      # CUE <-> CIN
│       └── ai/
│           ├── prompt_generator.gleam
│           └── code_validator.gleam
├── schema/
│   ├── intent.cue                   # Base schema
│   └── kirk.cue                     # Extended KIRK schema
└── docs/
    ├── KIRK_SPEC_DESIGN.md
    └── KIRK_IMPLEMENTATION_PLAN.md
```

---

## CLI Commands (Final)

```bash
# Existing
intent check <spec> --target <url>
intent validate <spec>
intent show <spec>
intent lint <spec>
intent analyze <spec>
intent interview --profile <api>

# New - Mental Models
intent invert <spec>           # Generate inversion checklist
intent premortem <spec>        # Generate pre-mortem analysis
intent effects <spec>          # Trace second-order effects

# New - Quality
intent quality <spec>          # Quality score report
intent coverage <spec>         # Coverage analysis
intent owasp <spec>            # OWASP checklist

# New - AI Integration
intent compact <spec>          # Convert to CIN format
intent expand <spec.cin>       # Convert from CIN format
intent prompt <spec>           # Generate AI prompts
intent validate-impl <spec> --code <path>  # Validate implementation

# New - Enhanced Interview
intent interview --matrix      # Full 5x5 interview
intent interview --perspective security  # Security-focused
```

---

## Success Metrics

### For Specifications
- 100% of behaviors have `why` explanations
- OWASP coverage score > 80%
- Quality score > 90
- Zero consistency conflicts

### For AI Implementation
- Token usage reduced by 40%+ with CIN
- Implementation matches spec on first attempt > 90%
- Generated code passes all checks without modification

### For Testing
- Edge case coverage > 70%
- All security anti-patterns tested
- Dependency chains validate correctly

---

## Next Steps

1. **Week 1**: Implement `why` enforcement and quality scoring
2. **Week 2**: Add inversion command and interview enhancements
3. **Week 3**: Design and implement CIN format
4. **Week 4**: Build AI prompt generator
5. **Week 5**: Implement 5x5 interview matrix
6. **Week 6**: Quality metrics and OWASP checklist

---

## Beads Integration

Create beads for each implementation item:

```bash
bd create --json << 'EOF'
{
  "title": "Implement why field enforcement in validator",
  "type": "feature",
  "priority": "high",
  "tags": ["kirk", "quality"],
  "description": "Add validation that every check has a non-empty why field"
}
EOF
```
