# The Mental Lattice Framework

## World-Class Planning for AI-Assisted Development

This document synthesizes KIRK, EARS, Munger's mental models, and formal methods into a unified framework that enables AI to "one-shot" complex implementations through structured mental lattices.

---

## The Core Insight

> "I think it is undeniably true that the human brain works in models. The trick is to have your brain work better than the other person's brain because it understands the most fundamental models — ones that do the most work." — Charlie Munger

The goal: **Transform vague human requirements into machine-verifiable contracts that an AI can implement deterministically on the first attempt.**

---

## The Five Mental Lattices

### Lattice 1: EARS (Requirements Syntax)

EARS (Easy Approach to Requirements Syntax) eliminates natural language ambiguity through five structured patterns:

```
┌─────────────────────────────────────────────────────────────────────────┐
│  PATTERN         │  TEMPLATE                                            │
├─────────────────────────────────────────────────────────────────────────┤
│  Ubiquitous      │  THE SYSTEM SHALL [behavior]                        │
│  Event-Driven    │  WHEN [trigger] THE SYSTEM SHALL [behavior]         │
│  State-Driven    │  WHILE [state] THE SYSTEM SHALL [behavior]          │
│  Optional        │  WHERE [condition] THE SYSTEM SHALL [behavior]      │
│  Unwanted        │  IF [condition] THEN THE SYSTEM SHALL NOT [behavior]│
│  Complex         │  WHILE [state] WHEN [trigger] THE SYSTEM SHALL ...  │
└─────────────────────────────────────────────────────────────────────────┘
```

**Why it works for AI:**
- Eliminates ambiguous words ("should", "may", "could")
- Forces identification of trigger conditions
- Makes negative requirements explicit
- Provides consistent parsing grammar

**Implementation**: `src/intent/kirk/ears_parser.gleam` (637 lines, fully functional)

---

### Lattice 2: KIRK Contracts (Design by Contract)

KIRK (Knowledge-Informed Requirements & Kontract) applies Bertrand Meyer's Design by Contract to API specifications:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         PRECONDITIONS                                    │
│  "What must be true BEFORE the function executes"                       │
│  - auth_required: true                                                   │
│  - required_fields: ["email", "password"]                               │
│  - field_constraints: { email: "valid format", password: "min 8 chars" }│
├─────────────────────────────────────────────────────────────────────────┤
│                         FUNCTION BODY                                    │
│  The actual implementation - AI generates this                          │
├─────────────────────────────────────────────────────────────────────────┤
│                        POSTCONDITIONS                                    │
│  "What must be true AFTER the function executes"                        │
│  - state_changes: ["User created in DB", "Password hashed"]             │
│  - response_guarantees: { id: "non-null UUID", password: "absent" }     │
├─────────────────────────────────────────────────────────────────────────┤
│                          INVARIANTS                                      │
│  "What must ALWAYS be true for this object"                             │
│  - "Passwords never appear in responses"                                │
│  - "All timestamps are ISO8601"                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

**Why it works for AI:**
- Machine-checkable requirements
- Self-documenting contracts
- Automatic test generation from postconditions
- Runtime verification of invariants

**Implementation**: `schema/kirk.cue`, `schema/kirk.proto`

---

### Lattice 3: Inversion Thinking (Failure Analysis)

> "Invert, always invert." — Carl Jacobi / Charlie Munger

Instead of asking "what should work?", systematically ask "what could fail?"

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       SECURITY INVERSIONS                                │
├─────────────────────────────────────────────────────────────────────────┤
│ • auth-bypass         - Accessing without authentication     → 401      │
│ • expired-token       - Using expired tokens                 → 401      │
│ • wrong-user-access   - Accessing another user's resources   → 403      │
│ • privilege-escalation- Admin actions as regular user        → 403      │
│ • sql-injection       - Malicious query parameters           → 400      │
│ • xss-payload         - XSS in user-controlled fields        → 400      │
│ • rate-limit-exceeded - Too many requests                    → 429      │
├─────────────────────────────────────────────────────────────────────────┤
│                       USABILITY INVERSIONS                               │
├─────────────────────────────────────────────────────────────────────────┤
│ • not-found           - Non-existent resources               → 404      │
│ • invalid-format      - Malformed request data               → 400      │
│ • missing-required    - Omitted required fields              → 400      │
│ • duplicate-create    - Creating duplicates                  → 409      │
│ • empty-list          - Edge case for empty results          → 200      │
├─────────────────────────────────────────────────────────────────────────┤
│                      INTEGRATION INVERSIONS                              │
├─────────────────────────────────────────────────────────────────────────┤
│ • idempotency         - Retry behavior                       → 200      │
│ • timeout-handling    - Long operation timeout               → 504      │
│ • version-mismatch    - API version compatibility            → 400      │
│ • method-not-allowed  - Wrong HTTP method                    → 405      │
└─────────────────────────────────────────────────────────────────────────┘
```

**Why it works for AI:**
- Comprehensive failure mode enumeration
- Generates negative test cases automatically
- Catches edge cases before implementation
- OWASP Top 10 coverage built-in

**Implementation**: `src/intent/kirk/inversion_checker.gleam` (490 lines)

---

### Lattice 4: Second-Order Thinking (Consequence Tracing)

Every action has consequences beyond its immediate effect. Trace them:

```
┌─────────────────────────────────────────────────────────────────────────┐
│ BEHAVIOR: delete-user                                                    │
├─────────────────────────────────────────────────────────────────────────┤
│ FIRST ORDER:  User record is deleted                                    │
├─────────────────────────────────────────────────────────────────────────┤
│ SECOND ORDER:                                                            │
│   • All user's items become orphaned                                    │
│   • Active sessions must be invalidated                                 │
│   • Audit log entries reference non-existent user                       │
│   • Analytics data loses attribution                                    │
│   • Shared resources need ownership transfer                            │
│   • Pending payments need cancellation                                  │
│   • Email subscriptions need cleanup                                    │
├─────────────────────────────────────────────────────────────────────────┤
│ CONSEQUENCE CHECKS:                                                      │
│   • get-deleted-user-items → expect 404 or empty                        │
│   • use-deleted-user-token → expect 401                                 │
│   • access-shared-resource → expect new owner or 404                    │
└─────────────────────────────────────────────────────────────────────────┘
```

**Why it works for AI:**
- Surfaces hidden dependencies
- Generates integration test scenarios
- Prevents cascade failures
- Documents system-wide effects

---

### Lattice 5: Pre-Mortem Analysis (Risk Prediction)

Gary Klein's prospective hindsight: Imagine the project has failed, then work backwards.

```
┌─────────────────────────────────────────────────────────────────────────┐
│ PRE-MORTEM: "The API launch failed catastrophically after 1 week"       │
├─────────────────────────────────────────────────────────────────────────┤
│ LIKELY CAUSES:                                                           │
│                                                                          │
│ 1. Rate limiting was too aggressive for legitimate users                │
│    Probability: HIGH                                                     │
│    Mitigation: Start generous, instrument, then tighten                 │
│                                                                          │
│ 2. JWT tokens expired during long operations                            │
│    Probability: MEDIUM                                                   │
│    Mitigation: Refresh mechanism, or longer expiry for specific ops     │
│                                                                          │
│ 3. Mobile clients cached stale auth tokens                              │
│    Probability: HIGH                                                     │
│    Mitigation: Clear 401 handling docs, force token refresh             │
│                                                                          │
│ 4. No graceful degradation when database overloaded                     │
│    Probability: MEDIUM                                                   │
│    Mitigation: Connection pooling, circuit breakers, queuing            │
│                                                                          │
│ 5. Error messages exposed sensitive internal state                      │
│    Probability: LOW but CRITICAL                                         │
│    Mitigation: Sanitize all error responses, separate internal logs     │
└─────────────────────────────────────────────────────────────────────────┘
```

**Why it works for AI:**
- Forces consideration of failure modes
- Generates defensive code patterns
- Creates monitoring/alerting requirements
- Documents known risks

---

## The Quality Dimensions

KIRK measures spec quality across 5 empirically-validated dimensions:

```
┌─────────────────────────────────────────────────────────────────────────┐
│ DIMENSION      │ MEASUREMENT                              │ WEIGHT     │
├─────────────────────────────────────────────────────────────────────────┤
│ Completeness   │ Fields filled / Total fields             │ 20%        │
│ Consistency    │ No conflicting rules (0 = 100%)          │ 20%        │
│ Testability    │ Behaviors with checks / Total behaviors  │ 25%        │
│ Clarity        │ 'why' fields present + intent length     │ 15%        │
│ Security       │ Security behaviors + anti-patterns       │ 20%        │
└─────────────────────────────────────────────────────────────────────────┘
```

**Target Scores:**
- Completeness: 100%
- Consistency: 100%
- Testability: 100%
- Clarity: 100% (every check has a 'why')
- Security: 80%+ (OWASP coverage)
- **Overall: 90%+**

**Implementation**: `src/intent/kirk/quality_analyzer.gleam` (626 lines)

---

## The Execution Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         HUMAN LAYER                                      │
│  Natural language requirements (EARS syntax)                            │
│  Mental model prompts (inversion, pre-mortem, 2nd order)                │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                          SPEC LAYER                                      │
│  CUE schema (source of truth)                                           │
│  KIRK contracts (pre/post/invariants)                                   │
│  Quality scoring and gap detection                                      │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                       VALIDATION LAYER                                   │
│  Inversion analysis (security, usability, integration)                  │
│  Coverage analysis (methods, status codes, edge cases)                  │
│  OWASP Top 10 checklist                                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                          AI LAYER                                        │
│  Compact format (CIN) for token efficiency (~50% reduction)             │
│  Structured prompts from specs                                          │
│  Constrained decoding for determinism                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                        OUTPUT LAYER                                      │
│  Atomic beads (5-30 min work items)                                     │
│  Execution plan with dependencies                                       │
│  Human approval checkpoint                                              │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                       FEEDBACK LAYER                                     │
│  Bead results (success/failed/blocked)                                  │
│  Regeneration from feedback                                             │
│  Iterative improvement                                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Token-Efficient Formats

### Standard CUE vs Compact Intent Notation (CIN)

**CUE (~500 tokens):**
```cue
behaviors: [{
    name: "create-user"
    intent: "Create a new user account"
    request: {
        method: "POST"
        path: "/users"
        body: {"email": "test@example.com"}
    }
    response: {
        status: 201
        checks: {
            id: {rule: "is uuid", why: "Unique identifier"}
            password: {rule: "absent", why: "Security"}
        }
    }
}]
```

**CIN (~250 tokens, 50% reduction):**
```
[create-user] Create new user account
  POST /users {"email":"test@example.com"}
  -> 201
  ? id: is uuid "Unique identifier"
  ? password: absent "Security"
```

**Implementation**: `src/intent/kirk/compact_format.gleam` (699 lines)

---

## The Interview Matrix (5x5)

Systematic requirement gathering across 5 rounds and 5 perspectives:

```
┌───────────────┬────────────┬────────────┬────────────┬────────────┬────────────┐
│               │   USER     │ DEVELOPER  │    OPS     │  SECURITY  │  BUSINESS  │
├───────────────┼────────────┼────────────┼────────────┼────────────┼────────────┤
│ CORE INTENT   │ What       │ What are   │ What scale │ What data  │ What       │
│               │ problem?   │ components?│ needed?    │ sensitive? │ metrics?   │
├───────────────┼────────────┼────────────┼────────────┼────────────┼────────────┤
│ ERROR CASES   │ What       │ What       │ What       │ What       │ What       │
│               │ frustrates?│ breaks?    │ alarms?    │ exposes?   │ costs?     │
├───────────────┼────────────┼────────────┼────────────┼────────────┼────────────┤
│ EDGE CASES    │ What's     │ What's     │ What's     │ What's     │ What's     │
│               │ unusual?   │ untested?  │ rare?      │ unexpected?│ seasonal?  │
├───────────────┼────────────┼────────────┼────────────┼────────────┼────────────┤
│ SECURITY      │ What do    │ What       │ What       │ What       │ What's     │
│               │ they fear? │ validates? │ monitors?  │ attacks?   │ liable?    │
├───────────────┼────────────┼────────────┼────────────┼────────────┼────────────┤
│ OPERATIONS    │ What       │ What       │ What       │ What       │ What       │
│               │ recovers?  │ scales?    │ fails?     │ audits?    │ grows?     │
└───────────────┴────────────┴────────────┴────────────┴────────────┴────────────┘
```

---

## CLI Commands (Current + Planned)

### Currently Implemented
```bash
intent check <spec> --target <url>    # Execute spec against target
intent validate <spec>                 # Validate spec syntax
intent show <spec>                     # Display spec details
intent lint <spec>                     # Check style issues
intent analyze <spec>                  # Deep analysis
```

### KIRK Commands (Planned)
```bash
# Mental Models
intent invert <spec>          # Inversion analysis (what could fail?)
intent premortem <spec>       # Pre-mortem analysis (why did it fail?)
intent effects <spec>         # Second-order consequence tracing

# Quality
intent quality <spec>         # Quality score report
intent coverage <spec>        # Coverage analysis
intent owasp <spec>          # OWASP Top 10 checklist

# EARS
intent ears <requirements.md> -o <spec.cue>  # Parse EARS to CUE

# AI Integration
intent compact <spec>         # Convert to CIN (token-efficient)
intent expand <spec.cin>      # Convert from CIN
intent prompt <spec>          # Generate AI prompts

# Interview
intent interview --matrix     # Full 5x5 interview
intent interview --answers=file.cue  # Non-interactive mode
```

---

## What Makes This World-Class

### 1. Deterministic Planning
- Clear requirements (EARS eliminates ambiguity)
- Formal contracts (KIRK defines success/failure)
- Atomic work items (beads are 5-30 min, self-contained)
- Machine-checkable (tests generated from specs)

### 2. Comprehensive Coverage
- Happy paths (what should work)
- Error cases (what should fail gracefully)
- Security cases (what attackers try)
- Edge cases (what's unusual but valid)
- Integration cases (what affects other systems)

### 3. Mental Model Integration
- Inversion: "How could this fail?"
- Pre-mortem: "Why did this fail?"
- Second-order: "What happens after?"
- Checklist: "What did we miss?"
- Circle of Competence: "What's in scope?"

### 4. Token Efficiency
- CIN format: 50% token reduction
- Structured prompts: Minimal ambiguity
- Constrained decoding: Guaranteed valid output

### 5. Human + AI Partnership
- Humans write natural requirements (EARS)
- System structures formally (KIRK)
- AI executes autonomously (beads)
- Humans approve before execution
- Feedback improves future planning

---

## Implementation Status

| Component | Status | Location |
|-----------|--------|----------|
| EARS Parser | ✅ Complete | `src/intent/kirk/ears_parser.gleam` |
| Quality Analyzer | ✅ Complete | `src/intent/kirk/quality_analyzer.gleam` |
| Inversion Checker | ✅ Complete | `src/intent/kirk/inversion_checker.gleam` |
| Coverage Analyzer | ✅ Complete | `src/intent/kirk/coverage_analyzer.gleam` |
| Gap Detector | ✅ Complete | `src/intent/kirk/gap_detector.gleam` |
| Compact Format | ✅ Complete | `src/intent/kirk/compact_format.gleam` |
| KIRK Schema | ✅ Complete | `schema/kirk.cue`, `schema/kirk.proto` |
| Interview Mode | 🔄 In Progress | Phase 1 of Improvement Plan |
| Feedback Loop | 📋 Planned | Phase 2 of Improvement Plan |
| CLI Integration | 📋 Planned | Phase 3 of Improvement Plan |

---

## The Vision

An AI planning system that:

1. **Accepts** natural language requirements (EARS syntax)
2. **Structures** them formally (KIRK contracts)
3. **Validates** with mental models (inversion, pre-mortem, 2nd order)
4. **Measures** quality (5-dimension scoring)
5. **Compacts** for AI (50% token reduction)
6. **Atomizes** into work items (5-30 min beads)
7. **Guides** execution (metadata, tools, dependencies)
8. **Enables** human oversight (approval checkpoint)
9. **Learns** from feedback (regeneration loop)
10. **Delivers** world-class planning capability

---

## References

### Mental Models
- [Munger's Latticework](https://fs.blog/mental-models/)
- [Inversion: Avoid Stupidity](https://fs.blog/inversion/)
- [Second-Order Thinking](https://fs.blog/second-order-thinking/)
- [Pre-Mortem Analysis](https://en.wikipedia.org/wiki/Pre-mortem)

### Requirements Engineering
- [EARS: Easy Approach to Requirements Syntax](https://ieeexplore.ieee.org/document/5328509)
- [Design by Contract (Meyer)](https://en.wikipedia.org/wiki/Design_by_contract)
- [INVEST Criteria](https://agilealliance.org/glossary/invest/)

### Formal Methods
- [TLA+ at Amazon](https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf)
- [Alloy Analyzer](https://alloytools.org/)
- [CUE Language](https://cuelang.org/)

### AI & Structured Output
- [Constrained Decoding (Outlines)](https://github.com/outlines-dev/outlines)
- [Structured Outputs Guide](https://platform.openai.com/docs/guides/structured-outputs)

---

*This framework represents the synthesis of cognitive psychology, formal methods, requirements engineering, and AI optimization into a unified system for deterministic AI-assisted development.*
