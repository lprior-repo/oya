# Interactive Questioning System

## Purpose

The AI needs to understand EXACTLY what to implement. This system ensures that after the human answers all questions, there is ZERO ambiguity remaining.

---

## Question Categories

### Category 1: Requirement Clarification

These resolve ambiguous EARS requirements.

```yaml
question_type: clarification
triggers:
  - ambiguous verbs ("handle", "process", "manage")
  - undefined terms ("appropriate", "suitable", "proper")
  - missing specifics ("quickly", "securely", "efficiently")

template: |
  The requirement says "[EARS statement]"

  Please clarify:
  - [specific question about the ambiguity]

  Options:
  A) [concrete option 1]
  B) [concrete option 2]
  C) [concrete option 3]
  D) Other: [free text]

example:
  requirement: "THE SYSTEM SHALL handle errors appropriately"
  question: |
    The requirement says "handle errors appropriately"

    Please clarify:
    - What constitutes "appropriate" error handling?

    Options:
    A) Return generic 500 error (hide details)
    B) Return specific error code and message
    C) Return error with stack trace (dev only)
    D) Return error with suggested fix
```

---

### Category 2: Edge Case Confirmation

These confirm behavior for unusual inputs.

```yaml
question_type: edge_case
triggers:
  - any input field
  - any output field
  - any state transition

template: |
  For [feature/behavior]:

  How should the system handle these edge cases?

  | Case | Accept? | Response |
  |------|---------|----------|
  | [case 1] | [yes/no] | [if no, what error?] |
  | [case 2] | [yes/no] | [if no, what error?] |
  | [case 3] | [yes/no] | [if no, what error?] |

example:
  behavior: "create-user"
  question: |
    For user creation:

    How should the system handle these edge cases?

    | Case | Accept? | Response |
    |------|---------|----------|
    | Empty string email "" | ? | ? |
    | Email with spaces "user @example.com" | ? | ? |
    | Email 256+ characters | ? | ? |
    | Unicode email "用户@例子.中国" | ? | ? |
    | Email with emoji "user😀@example.com" | ? | ? |
```

---

### Category 3: Business Logic

These clarify domain-specific rules.

```yaml
question_type: business_logic
triggers:
  - domain terms
  - implicit business rules
  - cross-cutting concerns

template: |
  Business Logic Question:

  [context of the decision]

  What is the correct behavior?

  A) [option with business implication 1]
  B) [option with business implication 2]
  C) [option with business implication 3]

example:
  context: "user deletes their account"
  question: |
    Business Logic Question:

    When a user deletes their account, what happens to their content?

    What is the correct behavior?

    A) Hard delete all content (irreversible)
    B) Soft delete, recoverable for 30 days
    C) Transfer ownership to admin
    D) Anonymize content (remove user attribution)
```

---

### Category 4: Security Decisions

These confirm security-critical choices.

```yaml
question_type: security
triggers:
  - authentication
  - authorization
  - data exposure
  - input validation

template: |
  Security Decision Required:

  [security context]

  | Option | Security Level | Trade-off |
  |--------|---------------|-----------|
  | A) [option] | [level] | [trade-off] |
  | B) [option] | [level] | [trade-off] |

  Recommendation: [option] because [reason]

  Your choice: [A/B/Other]

example:
  context: "password storage"
  question: |
    Security Decision Required:

    How should passwords be stored?

    | Option | Security Level | Trade-off |
    |--------|---------------|-----------|
    | A) bcrypt cost 10 | Good | 100ms hash time |
    | B) bcrypt cost 12 | Better | 400ms hash time |
    | C) Argon2id | Best | Requires tuning |

    Recommendation: B) bcrypt cost 12 because it balances security and UX

    Your choice: [A/B/C]
```

---

### Category 5: API Design

These confirm API structure and conventions.

```yaml
question_type: api_design
triggers:
  - endpoint definition
  - request/response structure
  - error format
  - versioning

template: |
  API Design Decision:

  [API context]

  Option A:
  ```json
  [structure A]
  ```

  Option B:
  ```json
  [structure B]
  ```

  Which format should we use?

example:
  context: "error response format"
  question: |
    API Design Decision:

    What error response format should we use?

    Option A (RFC 7807 Problem Details):
    ```json
    {
      "type": "https://api.example.com/errors/validation",
      "title": "Validation Error",
      "status": 400,
      "detail": "Email is invalid",
      "instance": "/users"
    }
    ```

    Option B (Simple):
    ```json
    {
      "error": {
        "code": "VALIDATION_ERROR",
        "message": "Email is invalid",
        "field": "email"
      }
    }
    ```

    Which format should we use?
```

---

### Category 6: Integration

These confirm external system integration.

```yaml
question_type: integration
triggers:
  - external API calls
  - database operations
  - message queues
  - caching

template: |
  Integration Decision:

  [integration context]

  Questions:
  1. [question about connection]
  2. [question about failure handling]
  3. [question about timeouts]

  For each, choose:
  A) [conservative option]
  B) [balanced option]
  C) [aggressive option]

example:
  context: "email sending via SMTP"
  question: |
    Integration Decision:

    Email sending via external SMTP service

    Questions:
    1. Timeout for SMTP connection?
       A) 5 seconds (fast fail)
       B) 30 seconds (balanced)
       C) 60 seconds (patient)

    2. Retry on failure?
       A) No retry (fail fast)
       B) 3 retries with backoff
       C) Infinite retry with dead letter queue

    3. Queue or synchronous?
       A) Synchronous (block until sent)
       B) Queue (return immediately)
```

---

## Question Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      INTERVIEW STATE MACHINE                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│    ┌──────────┐                                                         │
│    │  START   │                                                         │
│    └────┬─────┘                                                         │
│         │                                                                │
│         ↓                                                                │
│    ┌──────────────┐     ┌──────────────┐                                │
│    │ PARSE EARS   │ ──> │ EXTRACT      │                                │
│    │ Requirements │     │ Ambiguities  │                                │
│    └──────────────┘     └──────┬───────┘                                │
│                                │                                         │
│         ┌──────────────────────┼──────────────────────┐                 │
│         ↓                      ↓                      ↓                 │
│    ┌──────────┐         ┌──────────┐          ┌──────────┐              │
│    │Clarify   │         │ Confirm  │          │ Security │              │
│    │ Terms    │         │ Edge     │          │ Choices  │              │
│    └────┬─────┘         │ Cases    │          └────┬─────┘              │
│         │               └────┬─────┘               │                    │
│         └────────────────────┼─────────────────────┘                    │
│                              ↓                                           │
│                    ┌──────────────────┐                                 │
│                    │ All Questions    │                                 │
│                    │ Answered?        │                                 │
│                    └────────┬─────────┘                                 │
│                             │                                            │
│              ┌──────────────┴──────────────┐                            │
│              │ NO                          │ YES                        │
│              ↓                             ↓                            │
│         ┌──────────┐               ┌──────────────┐                     │
│         │ Ask Next │               │ Generate     │                     │
│         │ Question │               │ KIRK Contract│                     │
│         └────┬─────┘               └──────┬───────┘                     │
│              │                            │                              │
│              └────────────────────────────┼─────────────────────────>   │
│                                           ↓                              │
│                                    ┌──────────────┐                     │
│                                    │ Generate     │                     │
│                                    │ Atomic Beads │                     │
│                                    └──────────────┘                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Answer File Format (CUE)

Answers are stored in a CUE file for non-interactive mode:

```cue
answers: {
    // Requirement Clarifications
    clarifications: {
        "error_handling": "B"  // Return specific error code and message
        "validation_level": "A"  // Syntax only
    }

    // Edge Case Decisions
    edge_cases: {
        "empty_email": {
            accept: false
            response: 400
            message: "Email is required"
        }
        "unicode_email": {
            accept: true
            response: 201
        }
        "long_email": {
            accept: false
            response: 400
            message: "Email exceeds maximum length"
        }
    }

    // Business Logic
    business: {
        "account_deletion": "B"  // Soft delete, recoverable for 30 days
        "content_ownership": "D"  // Anonymize content
    }

    // Security Choices
    security: {
        "password_hashing": "B"  // bcrypt cost 12
        "token_expiry": "24h"
        "rate_limiting": {
            requests_per_minute: 60
            burst: 10
        }
    }

    // API Design
    api: {
        "error_format": "A"  // RFC 7807
        "pagination": "cursor"  // vs offset
        "versioning": "header"  // vs path
    }

    // Integration
    integration: {
        "smtp_timeout": "B"  // 30 seconds
        "smtp_retry": "B"  // 3 retries with backoff
        "smtp_queue": "B"  // Queue (return immediately)
    }
}
```

---

## Interactive Mode Commands

```bash
# Start interactive interview
intent interview --interactive

# Resume interview from saved state
intent interview --resume session_123

# Load pre-filled answers
intent interview --answers answers.cue

# Strict mode (fail on missing answers)
intent interview --answers answers.cue --strict

# Export unanswered questions
intent interview --export-questions questions.cue
```

---

## Question Priority

Questions are asked in this order:

1. **P0: Blocking** - Cannot proceed without this
   - Security decisions
   - Data model decisions
   - API contract decisions

2. **P1: Important** - Significantly affects implementation
   - Error handling strategy
   - Edge case behavior
   - Integration patterns

3. **P2: Nice to have** - Can use sensible defaults
   - Formatting choices
   - Logging verbosity
   - Performance tuning

---

## Default Answers

If strict mode is off, these defaults are used:

```yaml
defaults:
  # Error handling
  error_format: "simple"  # {error: {code, message}}
  error_detail: "minimal"  # Don't expose stack traces

  # Validation
  validation_strictness: "moderate"  # Accept reasonable variations
  unicode_support: true

  # Security
  password_hash: "bcrypt_12"
  token_expiry: "24h"
  rate_limit: "60/min"

  # API
  pagination: "cursor"
  versioning: "none"

  # Integration
  timeout: "30s"
  retry: "3x_backoff"
  async: true
```

---

## Question Generation Rules

### Rule 1: One Question Per Ambiguity
```
BAD:  "How should we handle email validation, password strength, and error messages?"
GOOD: Three separate questions, one for each concern
```

### Rule 2: Concrete Options
```
BAD:  "What level of security do you want?"
GOOD: "Which password hashing algorithm? A) bcrypt cost 10, B) bcrypt cost 12, C) Argon2id"
```

### Rule 3: Show Trade-offs
```
BAD:  "Use bcrypt or Argon2?"
GOOD: "bcrypt (100ms, widely supported) vs Argon2id (50ms, memory-hard, newer)"
```

### Rule 4: Include Recommendation
```
BAD:  "Pick A, B, or C"
GOOD: "Recommendation: B because [reason]. Your choice: A/B/C"
```

### Rule 5: Allow Custom Input
```
BAD:  "A or B?"
GOOD: "A, B, or Other: [describe your preference]"
```

---

## Question Tracking

```yaml
session:
  id: "sess_abc123"
  started: "2025-01-07T10:00:00Z"

  questions:
    asked: 12
    answered: 10
    skipped: 1
    pending: 1

  coverage:
    clarifications: 100%
    edge_cases: 80%
    security: 100%
    api_design: 50%
    integration: 0%

  blockers:
    - question_id: "SEC-003"
      category: "security"
      text: "What happens when rate limit exceeded?"
      reason: "Cannot implement rate limiting without this answer"
```

---

## Integration with Beads

After all questions are answered, beads are generated with complete information:

```yaml
bead:
  id: "USR-001"
  title: "Create user endpoint"

  # All ambiguities resolved
  resolved:
    - question: "email validation level"
      answer: "syntax only"
      impact: "no DNS lookup required"

    - question: "password hashing"
      answer: "bcrypt cost 12"
      impact: "~400ms per hash"

    - question: "error format"
      answer: "RFC 7807"
      impact: "structured problem details"

  # Tests generated from answers
  tests:
    - "invalid email format returns 400 with type=validation"
    - "weak password returns 400 with detail explaining requirements"
    - "successful creation returns 201 with no password field"

  # Edge cases confirmed
  edge_cases:
    - input: "user@例子.中国"
      expected: "accepted (unicode emails enabled)"
    - input: ""
      expected: "400 with detail='Email is required'"

  # No more questions needed
  blockers: []
```

---

*The goal: By the time a bead reaches the AI, every possible question has been answered, every edge case has been enumerated, and the implementation is purely mechanical translation from specification to code.*
