# AI Planning Tools & Deterministic AI Research

## Executive Summary

This document synthesizes research on the best AI planning tools and techniques for making AI code generation deterministic through specifications, contracts, and invariants.

**Key Findings:**
1. **AWS Kiro** uses EARS (Easy Approach to Requirements Syntax) for unambiguous specs
2. **GitHub Speckit** provides `/specify` → `/plan` → `/tasks` → `/implement` workflow
3. **Constrained Decoding** forces LLMs to generate valid JSON/structured output
4. **Design by Contract** (preconditions/postconditions/invariants) reduces ambiguity
5. **Property-Based Testing** verifies specifications hold across all inputs

---

## Part 1: AI Planning Tools

### 1.1 AWS Kiro

**Overview:** AWS Kiro is a spec-driven AI IDE that transforms how developers work with AI by requiring structured specifications before code generation.

**Three-File Spec Structure:**
```
project/
├── requirements.md    # EARS-format requirements
├── design.md          # System architecture & data flow
└── tasks.md           # Implementation checklist
```

**EARS Format (Easy Approach to Requirements Syntax):**

EARS provides unambiguous requirement patterns that eliminate natural language ambiguity:

| Pattern | Template | Example |
|---------|----------|---------|
| **Ubiquitous** | THE SYSTEM SHALL [behavior] | THE SYSTEM SHALL validate all inputs |
| **Event-Driven** | WHEN [trigger] THE SYSTEM SHALL [behavior] | WHEN user submits form THE SYSTEM SHALL validate email format |
| **State-Driven** | WHILE [state] THE SYSTEM SHALL [behavior] | WHILE user is authenticated THE SYSTEM SHALL allow API access |
| **Optional** | WHERE [condition] THE SYSTEM SHALL [behavior] | WHERE rate limit exceeded THE SYSTEM SHALL return 429 |
| **Unwanted** | IF [condition] THEN THE SYSTEM SHALL NOT [behavior] | IF user is banned THEN THE SYSTEM SHALL NOT allow login |
| **Complex** | WHILE [state] WHEN [trigger] THE SYSTEM SHALL [behavior] | WHILE in maintenance mode WHEN user requests THE SYSTEM SHALL return 503 |

**Why EARS Works:**
- Eliminates ambiguous words ("should", "may", "could")
- Forces identification of trigger conditions
- Makes negative requirements explicit
- Provides consistent parsing for AI

**Kiro CLI Commands:**
```bash
kiro spec init          # Initialize spec structure
kiro spec validate      # Check EARS syntax
kiro implement          # Generate code from specs
kiro test               # Run property-based tests
```

### 1.2 GitHub Speckit

**Overview:** Open-source toolkit for spec-driven development, supporting multiple AI agents (Claude, Gemini, Copilot, ChatGPT).

**Workflow:**
```
/specify  →  /plan  →  /tasks  →  /implement
    ↓          ↓         ↓           ↓
 Gather    Design    Break     Generate
 context   solution  down      code
```

**Slash Commands:**

| Command | Purpose | Output |
|---------|---------|--------|
| `/specify` | Extract requirements from context | `requirements.md` |
| `/plan` | Design system architecture | `design.md` |
| `/tasks` | Create implementation checklist | `tasks.md` |
| `/implement` | Generate code from specs | Source files |

**Speckit Features:**
- Agent-agnostic (works with any LLM)
- Version control integration
- Diff-based spec updates
- Test generation from specs

### 1.3 Other Notable Tools

**Cursor Rules:**
- Project-level AI configuration
- Custom instructions per codebase
- Style and convention enforcement

**Claude Projects:**
- Persistent context across conversations
- Project-specific knowledge base
- Custom system prompts

**Copilot Workspace:**
- Issue-to-PR automation
- Multi-file editing
- Spec-based implementation

---

## Part 2: Deterministic AI Techniques

### 2.1 Constrained Decoding

**Problem:** LLMs generate probabilistic output, leading to inconsistent code.

**Solution:** Constrain token generation to valid grammar.

**JSON Schema Enforcement:**
```python
from outlines import generate

schema = {
    "type": "object",
    "properties": {
        "name": {"type": "string"},
        "age": {"type": "integer", "minimum": 0}
    },
    "required": ["name", "age"]
}

# Only valid JSON matching schema can be generated
response = generate.json(model, schema)(prompt)
```

**How It Works:**
1. Parse schema into finite state automaton
2. At each token, mask invalid continuations
3. Only tokens leading to valid output are sampled
4. Guarantees 100% schema compliance

**Tools:**
- **Outlines** (Python): JSON/regex-constrained generation
- **Guidance** (Microsoft): Template-based constraints
- **LMQL** (ETH Zurich): Query language for LLMs
- **SGLang** (Stanford): Structured generation at scale

### 2.2 Design by Contract (DbC)

**Origin:** Bertrand Meyer, Eiffel programming language (1986)

**Three Components:**

```
┌─────────────────────────────────────────────────────────┐
│                   PRECONDITIONS                         │
│  "What must be true BEFORE the function executes"       │
│  Example: user_id != null, amount > 0                   │
├─────────────────────────────────────────────────────────┤
│                   FUNCTION BODY                         │
│  The actual implementation                              │
├─────────────────────────────────────────────────────────┤
│                  POSTCONDITIONS                         │
│  "What must be true AFTER the function executes"        │
│  Example: balance == old_balance - amount               │
├─────────────────────────────────────────────────────────┤
│                    INVARIANTS                           │
│  "What must ALWAYS be true for this object"             │
│  Example: balance >= 0                                  │
└─────────────────────────────────────────────────────────┘
```

**Application to AI:**
```yaml
behavior: withdraw_funds
preconditions:
  - auth_required: true
  - required_fields: [account_id, amount]
  - field_constraints:
      amount: "> 0"
      account_id: "uuid format"

postconditions:
  - state_changes: ["balance decreased by amount"]
  - response_guarantees:
      new_balance: "== old_balance - amount"

invariants:
  - "balance >= 0"
  - "transaction_log.length increased by 1"
```

**Benefits:**
- Machine-checkable requirements
- Self-documenting code
- Automatic test generation
- Runtime verification

### 2.3 Property-Based Testing

**Concept:** Instead of testing specific examples, test that properties hold for ALL inputs.

**Traditional Testing:**
```python
def test_reverse():
    assert reverse([1, 2, 3]) == [3, 2, 1]
    assert reverse([]) == []
```

**Property-Based Testing:**
```python
@given(lists(integers()))
def test_reverse_properties(lst):
    # Property 1: Reversing twice gives original
    assert reverse(reverse(lst)) == lst

    # Property 2: Length preserved
    assert len(reverse(lst)) == len(lst)

    # Property 3: First becomes last
    if lst:
        assert reverse(lst)[-1] == lst[0]
```

**Tools:**
- **QuickCheck** (Haskell): Original property-based testing
- **Hypothesis** (Python): Mature PBT library
- **fast-check** (TypeScript): JavaScript/TypeScript PBT
- **PropEr** (Erlang): Erlang property testing

**Deriving Properties from Specs:**
```yaml
# Spec
behavior: create_user
response:
  status: 201
  checks:
    id: "is uuid"
    email: "matches input.email"

# Generated Properties
@given(valid_user_input())
def test_create_user_properties(input):
    response = create_user(input)

    assert response.status == 201
    assert is_valid_uuid(response.body["id"])
    assert response.body["email"] == input["email"]
```

### 2.4 Formal Specification Languages

**TLA+ (Temporal Logic of Actions):**
```tla
---- MODULE BankAccount ----
VARIABLES balance

Init == balance = 0

Deposit(amount) ==
    /\ amount > 0
    /\ balance' = balance + amount

Withdraw(amount) ==
    /\ amount > 0
    /\ balance >= amount
    /\ balance' = balance - amount

TypeInvariant == balance >= 0
====
```

**Alloy (Relational Logic):**
```alloy
sig User {
    accounts: set Account
}

sig Account {
    balance: Int,
    owner: one User
}

fact NoNegativeBalance {
    all a: Account | a.balance >= 0
}

pred Withdraw[a: Account, amount: Int] {
    amount > 0
    a.balance >= amount
    a.balance' = a.balance - amount
}
```

**Z Notation (Set Theory):**
```
─── WithdrawOK ───────────────────────────
ΔAccount
amount?: ℕ
────────────────────────────────────────
amount? ≤ balance
balance' = balance - amount?
──────────────────────────────────────────
```

**When to Use:**
- TLA+: Distributed systems, concurrency
- Alloy: Data model constraints, security properties
- Z: Safety-critical systems, formal verification

---

## Part 3: Token-Efficient Specification Formats

### 3.1 Compact Intent Notation (CIN)

**Standard JSON (~500 tokens):**
```json
{
  "behaviors": [{
    "name": "create-user",
    "intent": "Create a new user account",
    "request": {
      "method": "POST",
      "path": "/users",
      "body": {"email": "test@example.com"}
    },
    "response": {
      "status": 201,
      "checks": {
        "id": {"rule": "is uuid"},
        "email": {"rule": "== input.email"}
      }
    }
  }]
}
```

**CIN Format (~250 tokens, 50% reduction):**
```
SPEC UserAPI v1.0.0

[create-user] Create new user account
  POST /users {"email":"test@example.com"}
  -> 201
  ? id: is uuid
  ? email: == input.email
```

**CIN Syntax:**
```
SPEC {name} {version}

[{behavior-name}] {intent}
  <- {dependency}          # requires
  {METHOD} {path} {body}   # request
  -> {status}              # response status
  ? {field}: {rule}        # checks
  >> {var}: {path}         # captures
```

### 3.2 TOON (Token-Optimized Object Notation)

**Research from Stanford NLP shows 40-60% token reduction:**

```
# Standard
{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]}

# TOON
users:[{id:1,name:Alice},{id:2,name:Bob}]
```

**Techniques:**
- Remove quotes around simple keys/values
- Use symbols for common patterns
- Compress repeated structures
- Reference previous values

### 3.3 Protobuf Text Format

**Binary protobuf is most efficient but not human-readable. Text format balances both:**

```protobuf
spec {
  name: "UserAPI"
  version: "1.0.0"

  features {
    name: "Authentication"
    behaviors {
      name: "login"
      intent: "Authenticate user credentials"
      request {
        method: POST
        path: "/auth/login"
      }
      response {
        status: 200
      }
    }
  }
}
```

---

## Part 4: Synthesis - The Ideal System

### 4.1 Recommended Stack

```
┌─────────────────────────────────────────────────────────┐
│                    HUMAN LAYER                          │
│  EARS requirements + Munger mental models               │
│  (inversion, second-order thinking, checklists)         │
├─────────────────────────────────────────────────────────┤
│                   SPEC LAYER                            │
│  CUE schema (source of truth)                           │
│  Design by Contract (pre/post/invariants)               │
├─────────────────────────────────────────────────────────┤
│                  VALIDATION LAYER                       │
│  Property-based testing                                 │
│  Formal verification (TLA+/Alloy for critical paths)   │
├─────────────────────────────────────────────────────────┤
│                    AI LAYER                             │
│  Compact format (CIN) for token efficiency              │
│  Constrained decoding for determinism                   │
│  Structured output enforcement                          │
├─────────────────────────────────────────────────────────┤
│                   OUTPUT LAYER                          │
│  Protobuf for cross-language interop                    │
│  JSON for debugging                                     │
│  Code generation with contracts embedded                │
└─────────────────────────────────────────────────────────┘
```

### 4.2 Best Practices

1. **Start with EARS Requirements**
   ```
   WHEN user submits login form
   THE SYSTEM SHALL validate email format
   AND THE SYSTEM SHALL check password against stored hash
   AND IF credentials valid THE SYSTEM SHALL return JWT token
   AND IF credentials invalid THE SYSTEM SHALL return 401 status
   ```

2. **Add Mental Model Analysis**
   - Inversion: "How could login fail?" → Brute force, SQL injection, session fixation
   - Second-order: "What happens after login?" → Session management, token refresh
   - Checklist: "What security measures?" → Rate limiting, MFA, audit logging

3. **Define Contracts**
   ```yaml
   preconditions:
     - email: "valid format"
     - password: "non-empty"
   postconditions:
     - IF success: "session created, JWT returned"
     - IF failure: "no session, error logged"
   invariants:
     - "password never logged"
     - "failed attempts tracked"
   ```

4. **Generate Properties**
   ```python
   @given(valid_credentials())
   def test_login_success(creds):
       response = login(creds)
       assert response.status == 200
       assert is_valid_jwt(response.body["token"])

   @given(invalid_credentials())
   def test_login_failure(creds):
       response = login(creds)
       assert response.status == 401
       assert "token" not in response.body
   ```

5. **Use Constrained Generation**
   ```python
   response_schema = {
       "oneOf": [
           {"properties": {"token": {"type": "string"}}},
           {"properties": {"error": {"type": "string"}}}
       ]
   }
   ```

### 4.3 Metrics for Success

| Metric | Target | Measurement |
|--------|--------|-------------|
| Spec Completeness | 95%+ | KIRK quality analyzer |
| Error Case Coverage | 30%+ | Inversion ratio |
| OWASP Coverage | 80%+ | Security checklist |
| Token Efficiency | 50%+ reduction | CIN vs JSON comparison |
| Generation Consistency | 100% | Constrained decoding |
| Property Violations | 0 | Property-based tests |

---

## Part 5: Implementation Recommendations

### For Intent CLI (KIRK)

1. **Add EARS Parser**
   - Parse `WHEN/WHILE/THE SYSTEM SHALL` patterns
   - Convert to CUE behaviors automatically
   - Validate against EARS grammar

2. **Integrate Constrained Decoding**
   - Use Outlines or Guidance for code generation
   - Define JSON schemas for all output types
   - Guarantee valid output structure

3. **Add Property Generation**
   - Extract properties from spec checks
   - Generate Hypothesis/QuickCheck tests
   - Run against implementation

4. **Formal Verification (Future)**
   - Export critical behaviors to TLA+
   - Model check for invariant violations
   - Verify concurrent behavior

### CLI Workflow

```bash
# 1. Write EARS requirements
intent ears requirements.md -o spec.cue

# 2. Add mental model analysis
intent analyze spec.cue --models inversion,second-order

# 3. Generate contracts
intent contracts spec.cue -o contracts.cue

# 4. Generate property tests
intent properties spec.cue -o tests/properties_test.gleam

# 5. Compact for AI
intent compact spec.cue -o prompt.cin

# 6. Generate code with constraints
intent generate spec.cue --constrained
```

---

## References

1. Mavin, A. et al. (2009). "Easy Approach to Requirements Syntax (EARS)"
2. Meyer, B. (1986). "Design by Contract"
3. Claessen, K. & Hughes, J. (2000). "QuickCheck: A Lightweight Tool for Random Testing"
4. Lamport, L. (1994). "The Temporal Logic of Actions"
5. Jackson, D. (2002). "Alloy: A Lightweight Object Modelling Notation"
6. Willard, B. et al. (2023). "Efficient Guided Generation for Large Language Models"
7. Munger, C. (1994). "A Lesson on Elementary, Worldly Wisdom"

---

*Document generated as part of KIRK research for Intent CLI*
*Last updated: 2026-01-07*
