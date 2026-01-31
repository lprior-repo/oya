# EARS + KIRK: The Complete AI Planning Workflow

## The Goal

Transform human requirements into perfectly atomic beads that an AI can implement deterministically, with complete test coverage and edge case handling.

**The key insight**: The more structured the questioning, the more deterministic the implementation. Each interview round narrows ambiguity until what remains is a crystal-clear contract.

---

## Phase 1: EARS Requirements Gathering

### The Interview Rounds

EARS uses 6 patterns. Each requires specific questioning to extract complete requirements.

---

### Round 1: Ubiquitous Requirements

**Pattern**: `THE SYSTEM SHALL [behavior]`

These are universal truths - things that must ALWAYS be true.

**Interview Questions**:
```
1. What must ALWAYS happen, regardless of context?
2. What are the core capabilities this system provides?
3. What are the non-negotiable behaviors?
4. What would break if these weren't true?
```

**Example Output**:
```ears
THE SYSTEM SHALL validate all API inputs against schema
THE SYSTEM SHALL log all requests with timestamps
THE SYSTEM SHALL return JSON responses with consistent structure
THE SYSTEM SHALL use HTTPS for all communications
THE SYSTEM SHALL reject requests exceeding 10MB
```

**Generated Beads** (one per requirement):
```yaml
- id: UBQ-001
  title: "Implement input validation against schema"
  type: validation
  effort: 20min
  tests:
    - valid input passes
    - invalid input rejected with 400
    - missing required field rejected
    - wrong type rejected
    - extra fields rejected (if strict)
  edge_cases:
    - empty object {}
    - null values
    - unicode in strings
    - very long strings
    - nested objects depth limit
```

---

### Round 2: Event-Driven Requirements

**Pattern**: `WHEN [trigger] THE SYSTEM SHALL [behavior]`

These define cause-and-effect relationships.

**Interview Questions**:
```
1. What user actions trigger system responses?
2. What external events does the system react to?
3. What time-based events occur?
4. What happens when data changes?
5. What triggers error handling?
```

**Example Output**:
```ears
WHEN user submits registration THE SYSTEM SHALL validate email format
WHEN login succeeds THE SYSTEM SHALL return JWT with 24-hour expiry
WHEN password reset requested THE SYSTEM SHALL send email within 30 seconds
WHEN file uploaded THE SYSTEM SHALL scan for viruses
WHEN payment received THE SYSTEM SHALL update account balance
```

**Generated Beads** (one per trigger-response pair):
```yaml
- id: EVT-001
  title: "Validate email format on registration"
  trigger: "user submits registration"
  behavior: "validate email format"
  effort: 15min
  tests:
    - valid email accepted: "user@example.com"
    - valid email with subdomain: "user@mail.example.com"
    - valid email with plus: "user+tag@example.com"
    - invalid missing @: "userexample.com"
    - invalid missing domain: "user@"
    - invalid missing local: "@example.com"
    - invalid double dot: "user..name@example.com"
  edge_cases:
    - very long local part (64 chars max)
    - very long domain (255 chars max)
    - unicode domain (IDN)
    - empty string
    - null input
```

---

### Round 3: State-Driven Requirements

**Pattern**: `WHILE [state] THE SYSTEM SHALL [behavior]`

These define behavior during specific system states.

**Interview Questions**:
```
1. What states can the system be in?
2. What states can a user be in?
3. What states can a resource be in?
4. How does behavior change based on state?
5. What happens during transitions?
```

**Example Output**:
```ears
WHILE user is authenticated THE SYSTEM SHALL allow API access
WHILE rate limit exceeded THE SYSTEM SHALL return 429
WHILE in maintenance mode THE SYSTEM SHALL return 503
WHILE account is suspended THE SYSTEM SHALL reject all requests
WHILE trial period active THE SYSTEM SHALL show upgrade prompts
```

**Generated Beads**:
```yaml
- id: STA-001
  title: "Allow API access for authenticated users"
  state: "user is authenticated"
  behavior: "allow API access"
  effort: 25min
  tests:
    - valid token grants access
    - expired token denied (401)
    - malformed token denied (401)
    - missing token denied (401)
    - revoked token denied (401)
  edge_cases:
    - token expires during request
    - concurrent requests with same token
    - token with wrong audience
    - token with wrong issuer
```

---

### Round 4: Optional Requirements

**Pattern**: `WHERE [condition] THE SYSTEM SHALL [behavior]`

These define conditional behavior based on feature flags, user types, or configurations.

**Interview Questions**:
```
1. What user roles exist and what can each do?
2. What feature flags control behavior?
3. What configuration options exist?
4. What optional capabilities can be enabled?
5. What third-party integrations are optional?
```

**Example Output**:
```ears
WHERE user has admin role THE SYSTEM SHALL allow user management
WHERE API key provided THE SYSTEM SHALL bypass rate limiting
WHERE 2FA enabled THE SYSTEM SHALL require OTP verification
WHERE premium plan active THE SYSTEM SHALL unlock advanced features
WHERE webhook configured THE SYSTEM SHALL send event notifications
```

**Generated Beads**:
```yaml
- id: OPT-001
  title: "Allow admin role to manage users"
  condition: "user has admin role"
  behavior: "allow user management"
  effort: 30min
  tests:
    - admin can list users
    - admin can create user
    - admin can update user
    - admin can delete user
    - non-admin cannot list users (403)
    - non-admin cannot create user (403)
  edge_cases:
    - admin trying to delete self
    - admin trying to demote self
    - last admin in system
    - role changed during session
```

---

### Round 5: Unwanted Requirements

**Pattern**: `IF [condition] THEN THE SYSTEM SHALL NOT [behavior]`

These define negative constraints - things that must NEVER happen.

**Interview Questions**:
```
1. What security violations must be prevented?
2. What data must never be exposed?
3. What actions must be blocked?
4. What states must be avoided?
5. What would be catastrophic if it happened?
```

**Example Output**:
```ears
IF user is banned THEN THE SYSTEM SHALL NOT allow login
IF token is expired THEN THE SYSTEM SHALL NOT authorize requests
IF password compromised THEN THE SYSTEM SHALL NOT accept it
IF account is deleted THEN THE SYSTEM SHALL NOT allow recovery
IF request is malformed THEN THE SYSTEM SHALL NOT process it
```

**Generated Beads**:
```yaml
- id: UNW-001
  title: "Block login for banned users"
  condition: "user is banned"
  must_not: "allow login"
  effort: 15min
  tests:
    - banned user login rejected
    - correct error message returned
    - no session created
    - attempt logged for audit
  edge_cases:
    - user banned mid-session
    - temporary vs permanent ban
    - ban expires exactly now
    - multiple login attempts
  anti_patterns:
    - MUST NOT: reveal ban reason to user
    - MUST NOT: allow password reset bypass
```

---

### Round 6: Complex Requirements

**Pattern**: `WHILE [state] WHEN [trigger] THE SYSTEM SHALL [behavior]`

These combine state and event conditions.

**Interview Questions**:
```
1. What behaviors depend on both state AND events?
2. What happens during state transitions?
3. What cascading effects exist?
4. What race conditions could occur?
```

**Example Output**:
```ears
WHILE logged in WHEN session expires THE SYSTEM SHALL redirect to login
WHILE in transaction WHEN error occurs THE SYSTEM SHALL rollback changes
WHILE uploading WHEN connection lost THE SYSTEM SHALL retry 3 times
WHILE rate limited WHEN limit resets THE SYSTEM SHALL resume normal
```

**Generated Beads**:
```yaml
- id: CPX-001
  title: "Handle session expiry during login"
  state: "logged in"
  trigger: "session expires"
  behavior: "redirect to login"
  effort: 20min
  tests:
    - expired session redirects to login
    - current request state preserved
    - user sees appropriate message
    - no sensitive data in redirect
  edge_cases:
    - expire during form submission
    - expire during file upload
    - multiple tabs open
    - remember me option
```

---

## Phase 2: KIRK Contract Generation

After EARS requirements are gathered, KIRK contracts are generated.

### Contract Structure

```cue
behavior: {
    name: "create-user"

    // From EARS
    ears: {
        pattern: "EventDriven"
        trigger: "user submits registration"
        shall: "create account and send verification email"
    }

    // KIRK Preconditions
    preconditions: {
        auth_required: false
        required_fields: ["email", "password", "name"]
        field_constraints: {
            email: "valid RFC 5322 format"
            password: "min 8 chars, 1 upper, 1 number, 1 special"
            name: "1-100 non-empty characters"
        }
    }

    // KIRK Postconditions
    postconditions: {
        state_changes: [
            "User record created in database",
            "Password hashed with bcrypt cost 12",
            "Verification email queued",
        ]
        response_guarantees: {
            id: "non-null, matches usr_[a-z0-9]+"
            password: "absent from response"
            created_at: "within last 5 seconds"
        }
    }

    // KIRK Invariants
    invariants: [
        "Email uniqueness enforced",
        "Password never stored in plain text",
        "Password never returned in any response",
    ]

    // Inversion analysis
    inversions: {
        security: [
            "duplicate-email → 409",
            "weak-password → 400",
            "sql-injection → 400",
        ]
        usability: [
            "missing-email → 400",
            "invalid-email → 400",
        ]
    }
}
```

---

## Phase 3: Atomic Bead Generation

Each KIRK contract generates multiple atomic beads.

### Bead Atomicity Rules

1. **Time-boxed**: 5-30 minutes maximum
2. **Single concern**: One behavior, one outcome
3. **Testable**: Clear pass/fail criteria
4. **Independent**: Minimal dependencies
5. **Complete**: All edge cases enumerated

### Bead Template

```yaml
id: "UBQ-001-01"
title: "Create user with valid input"
parent: "UBQ-001"

# What the AI implements
implementation:
  function: "createUser"
  file: "src/handlers/users.gleam"
  lines_estimate: 20-40

# Clear input/output contract
contract:
  input:
    method: POST
    path: /users
    body:
      email: "user@example.com"
      password: "SecurePass123!"
      name: "Test User"
  output:
    status: 201
    body:
      id: "usr_abc123"
      email: "user@example.com"
      name: "Test User"
      created_at: "2025-01-07T10:00:00Z"
    headers:
      Content-Type: "application/json"

# Tests to write
tests:
  - name: "creates user with valid input"
    type: happy_path
    priority: P0
  - name: "returns 400 for missing email"
    type: validation
    priority: P0
  - name: "returns 400 for invalid email"
    type: validation
    priority: P0
  - name: "returns 400 for weak password"
    type: validation
    priority: P1
  - name: "returns 409 for duplicate email"
    type: conflict
    priority: P0
  - name: "hashes password before storing"
    type: security
    priority: P0
  - name: "never returns password in response"
    type: security
    priority: P0

# Edge cases to handle
edge_cases:
  - name: "email with plus addressing"
    input: "user+tag@example.com"
    expected: "accepted"
  - name: "unicode name"
    input: {name: "用户"}
    expected: "accepted"
  - name: "maximum length email"
    input: "a{254 chars}.example.com"
    expected: "rejected 400"
  - name: "empty name"
    input: {name: ""}
    expected: "rejected 400"
  - name: "whitespace-only name"
    input: {name: "   "}
    expected: "rejected 400"

# Dependencies
requires: []
blocked_by: []

# Metadata for AI
ai_hints:
  patterns:
    - "Use Result type for error handling"
    - "Validate at controller boundary"
    - "Hash password before DB insert"
  anti_patterns:
    - "Do not return password field"
    - "Do not use sequential IDs"
    - "Do not store plain text passwords"
  similar_to: ["update-user", "delete-user"]
```

---

## Phase 4: Interactive Clarification Loop

Before implementation, the AI asks clarifying questions to ensure complete understanding.

### Question Categories

**1. Ambiguity Resolution**
```
Q: The requirement says "validate email format" - which level of validation?
   A) Syntax only (has @ and domain)
   B) DNS check (domain exists)
   C) Deliverability check (mailbox exists)

Q: "Password must be strong" - what are the exact rules?
   A) Minimum 8 characters
   B) + 1 uppercase
   C) + 1 number
   D) + 1 special character
   E) + not in common passwords list
```

**2. Edge Case Confirmation**
```
Q: Should the system accept these edge cases?
   - Empty arrays in list endpoints: [yes/no]
   - Null values in optional fields: [yes/no]
   - Unicode in identifiers: [yes/no]
   - Very long inputs (>10KB): [yes/no]
```

**3. Error Handling Preferences**
```
Q: How should validation errors be returned?
   A) Single error, first found
   B) All errors in array
   C) Errors grouped by field

Q: What HTTP status for malformed JSON?
   A) 400 Bad Request
   B) 422 Unprocessable Entity
```

**4. Integration Decisions**
```
Q: How should authentication work?
   A) JWT in Authorization header
   B) Session cookies
   C) API key in header

Q: Rate limiting strategy?
   A) Per-user limits
   B) Per-IP limits
   C) Per-endpoint limits
```

---

## Phase 5: Bead Execution Workflow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          BEAD LIFECYCLE                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐       │
│  │  READY   │ ──> │ CLAIMED  │ ──> │ WORKING  │ ──> │ TESTING  │       │
│  └──────────┘     └──────────┘     └──────────┘     └──────────┘       │
│       ↑                                  │               │              │
│       │                                  │               ↓              │
│       │           ┌──────────┐           │         ┌──────────┐        │
│       └────────── │  FAILED  │ <─────────┴───────  │  DONE    │        │
│                   └──────────┘                     └──────────┘        │
│                        │                                                │
│                        ↓                                                │
│                   ┌──────────┐                                          │
│                   │ BLOCKED  │                                          │
│                   └──────────┘                                          │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### State Definitions

| State | Description | AI Action |
|-------|-------------|-----------|
| READY | Bead is ready to be worked on | Pick up if dependencies met |
| CLAIMED | AI has claimed the bead | Start implementation |
| WORKING | Implementation in progress | Write code |
| TESTING | Running tests | Execute test suite |
| DONE | All tests pass | Mark complete |
| FAILED | Tests failed | Report failure, request feedback |
| BLOCKED | Missing information | Ask clarifying questions |

---

## Phase 6: The Complete Flow

```
User writes natural language requirements
                    ↓
        ┌───────────────────────┐
        │   EARS PARSER         │
        │   6 Interview Rounds  │
        └───────────────────────┘
                    ↓
        Structured EARS Requirements
                    ↓
        ┌───────────────────────┐
        │   KIRK CONTRACTS      │
        │   Pre/Post/Invariants │
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   INVERSION ANALYSIS  │
        │   Security/Usability  │
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   QUALITY CHECK       │
        │   5 Dimensions        │
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   BEAD GENERATION     │
        │   5-30 min atoms      │
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   CLARIFICATION LOOP  │
        │   AI asks questions   │
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   HUMAN APPROVAL      │
        │   Review & Approve    │
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   AI IMPLEMENTATION   │
        │   One bead at a time  │
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   FEEDBACK LOOP       │
        │   Success/Fail/Block  │
        └───────────────────────┘
                    ↓
            Next bead or done
```

---

## CLI Commands for This Workflow

```bash
# Step 1: Parse EARS requirements
intent ears requirements.md --interview

# Step 2: Generate KIRK contracts
intent kirk contracts.ears.md -o spec.cue

# Step 3: Analyze inversions
intent invert spec.cue

# Step 4: Check quality
intent quality spec.cue

# Step 5: Generate beads
intent beads spec.cue -o .beads/

# Step 6: Start clarification loop
intent clarify --session abc123

# Step 7: Approve plan
intent plan abc123
intent plan-approve abc123

# Step 8: Execute (AI picks up beads)
bd ready --json | jq '.beads[0].id' | xargs bd claim

# Step 9: Mark complete/failed
bd close <id> --reason "Implemented with tests"
bd update <id> --status failed --reason "Need clarification on X"
```

---

## Success Metrics

| Metric | Target | Why |
|--------|--------|-----|
| Bead atomicity | 5-30 min each | AI can complete in single session |
| Test coverage | 100% of happy paths | No untested code |
| Edge case coverage | 100% enumerated | No surprises |
| Clarification questions | <5 per feature | Requirements are clear |
| One-shot success rate | >90% | AI understands completely |
| Rework rate | <10% | Specs are complete |

---

## Example: Complete User Registration Flow

### EARS Input
```ears
# Ubiquitous
THE SYSTEM SHALL validate all registration inputs

# Event-Driven
WHEN user submits registration THE SYSTEM SHALL create account
WHEN account created THE SYSTEM SHALL send verification email

# State-Driven
WHILE account unverified THE SYSTEM SHALL limit access
WHILE email sending THE SYSTEM SHALL queue retries

# Optional
WHERE referral code provided THE SYSTEM SHALL apply bonus

# Unwanted
IF email already exists THEN THE SYSTEM SHALL NOT create duplicate
IF password is weak THEN THE SYSTEM SHALL NOT accept it
```

### Generated Beads

```
├── UBQ-001: Implement registration input validation (15min)
│   ├── test: valid email accepted
│   ├── test: invalid email rejected
│   ├── test: valid password accepted
│   ├── test: weak password rejected
│   └── edge: unicode in name
│
├── EVT-001: Create user account on registration (25min)
│   ├── test: account created in database
│   ├── test: password hashed
│   ├── test: user ID generated
│   └── edge: concurrent registration
│
├── EVT-002: Send verification email (20min)
│   ├── test: email queued
│   ├── test: contains verification link
│   ├── test: link expires in 24h
│   └── edge: email send failure
│
├── STA-001: Limit unverified account access (15min)
│   ├── test: can view profile
│   ├── test: cannot post content
│   └── edge: verify during restricted action
│
├── OPT-001: Apply referral bonus (15min)
│   ├── test: valid code applies bonus
│   ├── test: invalid code ignored
│   └── edge: referrer deleted
│
├── UNW-001: Reject duplicate email (10min)
│   ├── test: returns 409
│   ├── test: clear error message
│   └── edge: case sensitivity
│
└── UNW-002: Reject weak password (10min)
    ├── test: returns 400
    ├── test: explains requirements
    └── edge: common password list
```

**Total: 7 beads, ~110 minutes, 25 tests, 8 edge cases**

---

*This workflow ensures that by the time the AI receives a bead, there is zero ambiguity about what to implement, how to test it, and what edge cases to handle.*
