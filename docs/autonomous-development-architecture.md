# The Autonomous Development Triangle: A Complete Implementation Architecture

## Digital Twins + High-Quality Specs + Behavioral Scenarios

---

## Part 1: The Conceptual Model — Why These Three and Only These Three

These three concepts form a **closed triangle** that solves the fundamental problem of autonomous software development. Remove any one leg and the whole thing collapses:

```
                    ┌─────────────────────┐
                    │  HIGH-QUALITY SPECS  │
                    │  (What to build)     │
                    └──────────┬──────────┘
                               │
                    Human writes│spec
                               │
                  ┌────────────▼────────────┐
                  │                          │
          ┌───────┴───────┐    ┌────────────┴────────────┐
          │ DIGITAL TWIN  │    │  BEHAVIORAL SCENARIOS    │
          │ UNIVERSE      │    │  (Holdout validation)    │
          │ (Where to     │    │                          │
          │  develop)     │    │  Agent CANNOT see these  │
          └───────┬───────┘    └────────────┬────────────┘
                  │                          │
                  │   AI Agent develops      │  Scenario runner
                  │   against twins         │  validates output
                  │                          │
                  └──────────┬───────────────┘
                             │
                    ┌────────▼────────┐
                    │  FEEDBACK LOOP  │
                    │  (Iterate until │
                    │   all pass)     │
                    └─────────────────┘
```

**Without specs**: The agent doesn't know what to build. It guesses. Guesses produce software that "works" by the agent's definition, not the customer's.

**Without twins**: The agent can't test integrations. It either hits production (dangerous), mocks everything (unrealistic), or skips integration testing (broken software).

**Without holdout scenarios**: The agent writes code that passes its own tests but doesn't actually work. It's the ML overfitting problem applied to software development — the agent "teaches to the test" because it can see the test.

The triangle creates an **adversarial development environment** where the agent is:
- **Informed** (by specs) about what to build
- **Empowered** (by twins) to develop and test realistically
- **Honestly evaluated** (by hidden scenarios) against real behavioral expectations

---

## Part 2: The Autonomous Development Lifecycle

Before diving into each component, here's the full lifecycle that ties them together:

```
PHASE 1: SPECIFICATION                    [HUMAN-DRIVEN]
  ├─ Human writes spec ──────────────────► Spec Registry
  ├─ Spec linter validates quality ──────► Quality Gate (score ≥ 80)
  └─ Human writes holdout scenarios ─────► Encrypted Scenario Vault
                                            (separate repo, separate access)

PHASE 2: UNIVERSE SETUP                   [AUTOMATED]
  ├─ Read spec.context.dependencies ─────► Identify needed twins
  ├─ Spin up twin instances ─────────────► Each twin runs as HTTP server
  ├─ Load initial state into twins ──────► Deterministic starting point
  ├─ Configure webhook routing ──────────► Twins can fire webhooks to app
  └─ Signal "universe ready" ────────────► Orchestrator proceeds

PHASE 3: AUTONOMOUS DEVELOPMENT           [AI AGENT]
  ├─ Agent receives: spec + twin URLs ───► Agent workspace
  ├─ Agent does NOT receive: scenarios ──► Information barrier enforced
  ├─ Agent writes code ──────────────────► Against twin endpoints
  ├─ Agent writes its own tests ─────────► These are "training" tests
  ├─ Agent runs tests against twins ─────► Iterates until green
  └─ Agent signals "done" ──────────────► Artifact produced

PHASE 4: BEHAVIORAL VALIDATION            [AUTOMATED, AGENT-BLIND]
  ├─ Scenario runner takes artifact ─────► Deploys into fresh twin universe
  ├─ Runs ALL holdout scenarios ─────────► Black-box, external
  ├─ Collects results ──────────────────► Pass/fail per scenario
  ├─ IF all pass ────────────────────────► ACCEPTED ✓
  ├─ IF some fail, iterations < max ─────► Sanitize feedback, go to Phase 3
  └─ IF some fail, iterations ≥ max ────► ESCALATE to human

PHASE 5: FEEDBACK (on failure)             [AUTOMATED, SANITIZED]
  ├─ Raw failure details ────────────────► Sanitization pipeline
  ├─ Remove: exact requests, assertions ─► Prevent scenario leakage
  ├─ Preserve: behavior category, hints ─► Agent can reason about fix
  └─ Send to agent ─────────────────────► Next iteration begins
```

---

## Part 3: High-Quality Specifications — The Bottleneck That Matters

### 3.1 Why Specs Are the New Bottleneck

When humans write code, ambiguous specs are handled by **human judgment**. A developer reads "add password reset" and fills in dozens of implicit decisions:
- What happens if the email doesn't exist? (Don't reveal that — security)
- How long does the token last? (Industry standard: 15-60 minutes)
- What's the email format? (Professional, branded, clear CTA)
- Rate limiting? (Obviously, or we get abuse)

An AI agent **cannot make these customer-centric decisions**. It makes **software-centric decisions**: whatever produces valid code fastest. The result: technically correct, behaviorally wrong.

The spec must resolve **every decision point** that a human developer would resolve with judgment.

### 3.2 The Spec Schema

```yaml
# specs/schema/spec.schema.yaml
# This is the canonical structure every spec must follow

specification:
  # ─── IDENTITY ───────────────────────────────────────────────
  identity:
    id: string          # Unique identifier (e.g., "spec-password-reset")
    version: string     # Semantic version (e.g., "2.1.0")
    status: enum        # draft | review | approved | implemented | deprecated
    author: string      # Who wrote this spec
    created: datetime
    updated: datetime
    supersedes: string?  # ID of previous version, if any
    
  # ─── INTENT ─────────────────────────────────────────────────
  # WHY does this feature exist? What problem does it solve?
  intent:
    problem_statement: string
      # "Users who forget their password are permanently locked out.
      #  Our support team handles 200+ password reset requests per week,
      #  costing $15 per ticket. Self-service reset would eliminate 90%
      #  of these tickets."
      
    success_criteria: string[]
      # - "Users can reset their password without contacting support"
      # - "Average reset completion time under 3 minutes"
      # - "Zero security incidents from the reset flow"
      
    non_goals: string[]
      # - "We are NOT implementing SMS-based reset in this version"
      # - "We are NOT changing the password complexity requirements"
      # - "We are NOT implementing 'magic link' passwordless login"
    
  # ─── CONTEXT ────────────────────────────────────────────────
  # What does the agent need to know about the existing system?
  context:
    system_dependencies: 
      - service: string       # e.g., "sendgrid"
        purpose: string       # e.g., "Send password reset emails"
        twin_available: bool  # Does a twin exist for this service?
        
    existing_behaviors: string[]
      # - "Users authenticate via email + password (no SSO)"
      # - "User emails are unique per tenant"
      # - "Sessions are JWT-based with 24h expiry"
      
    constraints: string[]
      # - "Must work in all supported browsers (Chrome, Firefox, Safari, Edge)"
      # - "Reset token must be cryptographically random, minimum 256 bits"
      # - "All password reset events must be audit-logged"
      # - "Response time < 500ms for all endpoints"
      
    invariants: string[]
      # - "A user's email address must remain unique across the tenant"
      # - "Password hashes must use bcrypt with cost factor ≥ 12"
      # - "Reset tokens must be single-use (consumed on successful reset)"
      # - "Old password must remain valid until new password is set"
      
    glossary:
      # Define domain terms the agent needs to understand
      reset_token: "A cryptographically random, time-limited, single-use
                    token sent via email that authorizes a password change"
      token_expiry: "15 minutes from token generation"
      
  # ─── BEHAVIORS ──────────────────────────────────────────────
  # The core behavioral specifications (Given/When/Then format)
  behaviors:
    - id: "request-reset"
      description: "User requests a password reset"
      given:
        - "A user account exists with email 'user@example.com'"
        - "The user has a verified email address"
        - "No rate limit has been reached for this email"
      when: "The user submits a password reset request for 'user@example.com'"
      then:
        - "The system generates a reset token (256-bit random, URL-safe base64)"
        - "The token is stored with: user_id, created_at, expires_at (now + 15min)"
        - "An email is sent via SendGrid to 'user@example.com'"
        - "The email contains a link: {base_url}/reset-password?token={token}"
        - "The API responds with HTTP 200 and body: { message: 'If an account 
           with that email exists, a reset link has been sent.' }"
      edge_cases:
        - id: "nonexistent-email"
          when: "The user submits a reset request for an email that doesn't exist"
          then:
            - "The API responds with the SAME 200 response (prevent enumeration)"
            - "No email is sent"
            - "The response time is artificially similar (prevent timing attacks)"
            
        - id: "unverified-email"
          when: "The user's email exists but is not verified"
          then:
            - "The API responds with the same 200 response"
            - "No reset email is sent"
            - "An audit log entry is created: 'reset_attempted_unverified'"
            
        - id: "rate-limited"
          when: "More than 3 reset requests for the same email in 15 minutes"
          then:
            - "The API responds with HTTP 429 Too Many Requests"
            - "Response includes Retry-After header"
            - "An audit log entry: 'reset_rate_limited'"
            
        - id: "sendgrid-failure"
          when: "SendGrid API returns a 5xx error"
          then:
            - "The token is still created and stored"
            - "The email send is queued for retry (max 3 retries, exponential backoff)"
            - "The API still responds with 200 (user shouldn't know about infra issues)"
            - "An alert is triggered if all retries fail"
            
    - id: "complete-reset"
      description: "User completes the password reset"
      given:
        - "A valid, non-expired reset token exists for the user"
      when: "The user submits a new password with the token"
      then:
        - "The new password is validated against complexity rules"
        - "The password hash is updated in the database"
        - "The reset token is marked as consumed"
        - "All existing sessions for this user are invalidated"
        - "An audit log entry: 'password_reset_completed'"
        - "A confirmation email is sent to the user"
        - "The API responds with HTTP 200"
      edge_cases:
        - id: "expired-token"
          when: "The token has expired (created_at + 15min < now)"
          then:
            - "API responds with HTTP 410 Gone"
            - "Response body: { error: 'token_expired', action: 'request_new_reset' }"
            - "The token is marked as expired in the database"
            
        - id: "already-used-token"
          when: "The token has already been consumed"
          then:
            - "API responds with HTTP 410 Gone"
            - "Response body: { error: 'token_already_used' }"
            
        - id: "weak-password"
          when: "The new password doesn't meet complexity requirements"
          then:
            - "API responds with HTTP 422 Unprocessable Entity"
            - "Response body includes specific validation failures"
            - "The token is NOT consumed (user can retry)"
            
        - id: "concurrent-reset"
          when: "Two requests arrive simultaneously with the same token"
          then:
            - "Exactly one succeeds (optimistic locking on token consumption)"
            - "The other receives HTTP 410"
            
  # ─── DATA MODEL ─────────────────────────────────────────────
  data_model:
    entities:
      - name: PasswordResetToken
        fields:
          - { name: id, type: uuid, generated: true }
          - { name: user_id, type: uuid, foreign_key: users.id }
          - { name: token_hash, type: string, description: "bcrypt hash of the token" }
          - { name: created_at, type: datetime }
          - { name: expires_at, type: datetime }
          - { name: consumed_at, type: datetime, nullable: true }
          - { name: status, type: enum, values: [active, consumed, expired] }
        indexes:
          - { fields: [token_hash], unique: true }
          - { fields: [user_id, status] }
          
    state_transitions:
      - entity: PasswordResetToken
        from: active
        to: consumed
        trigger: "Successful password reset"
        side_effects: 
          - "User.password_hash updated"
          - "All User sessions invalidated"
          
      - entity: PasswordResetToken
        from: active
        to: expired
        trigger: "Token age > 15 minutes"
        side_effects: none
        
  # ─── API CONTRACT ───────────────────────────────────────────
  api_contract:
    endpoints:
      - method: POST
        path: /api/v1/auth/reset-password/request
        authentication: none  # Must be accessible to unauthenticated users
        rate_limit: "3 requests per email per 15 minutes"
        request:
          body:
            email: { type: string, format: email, required: true }
        responses:
          200: { description: "Request accepted (always, to prevent enumeration)" }
          422: { description: "Invalid email format" }
          429: { description: "Rate limit exceeded", headers: { Retry-After: integer } }
          
      - method: POST
        path: /api/v1/auth/reset-password/complete
        authentication: none  # Token in body serves as authentication
        request:
          body:
            token: { type: string, required: true }
            new_password: { type: string, required: true, min_length: 12 }
        responses:
          200: { description: "Password successfully reset" }
          410: { description: "Token expired or already used" }
          422: { description: "Password doesn't meet requirements" }
          
    events_emitted:
      - name: password.reset.requested
        payload: { user_id: uuid, requested_at: datetime }
      - name: password.reset.completed
        payload: { user_id: uuid, completed_at: datetime }
      - name: password.reset.failed
        payload: { user_id: uuid, reason: string, failed_at: datetime }
        
    events_consumed: []  # This feature doesn't consume external events
    
  # ─── ACCEPTANCE CRITERIA ────────────────────────────────────
  # These are the VISIBLE criteria (agent can see these)
  # They are SUBSET of the holdout scenarios
  acceptance_criteria:
    - id: ac-01
      behavior_ref: request-reset
      criterion: "Valid email triggers reset email delivery"
    - id: ac-02
      behavior_ref: request-reset.nonexistent-email
      criterion: "Nonexistent email returns same response as valid email"
    - id: ac-03
      behavior_ref: complete-reset
      criterion: "Valid token allows password change"
    - id: ac-04
      behavior_ref: complete-reset.expired-token
      criterion: "Expired token returns 410 with recovery action"
    - id: ac-05
      behavior_ref: complete-reset.concurrent-reset
      criterion: "Concurrent resets are handled safely"
```

### 3.3 The Spec Linter

The linter runs **before** the agent starts work. It catches spec quality issues early.

```yaml
# specs/linter/rules.yaml
rules:
  # ─── COMPLETENESS RULES ──────────────────────────────────
  - id: SPEC-001
    name: every-dependency-has-error-handling
    severity: error
    description: >
      Every external service in context.system_dependencies must have 
      at least one edge_case in behaviors that handles that service failing.
    check: |
      for each dep in spec.context.system_dependencies:
        assert exists behavior.edge_case where 
          description contains dep.service AND 
          (description contains "failure" OR "error" OR "unavailable" OR "timeout")
          
  - id: SPEC-002
    name: every-state-transition-has-invariant-check
    severity: error
    description: >
      Every state transition must reference at least one invariant
      that holds after the transition.
    check: |
      for each transition in spec.data_model.state_transitions:
        assert exists invariant in spec.context.invariants where
          invariant references transition.entity
          
  - id: SPEC-003
    name: every-endpoint-specifies-auth
    severity: error
    description: >
      Every API endpoint must explicitly specify its authentication 
      requirement (even if "none").
    check: |
      for each endpoint in spec.api_contract.endpoints:
        assert endpoint.authentication is not null
        
  - id: SPEC-004
    name: every-behavior-has-acceptance-criterion
    severity: warning
    description: >
      Every behavior (including edge cases) should have at least one
      acceptance criterion that validates it.
    check: |
      for each behavior in spec.behaviors (including edge_cases):
        assert exists criterion in spec.acceptance_criteria where
          criterion.behavior_ref matches behavior.id
          
  # ─── CLARITY RULES ───────────────────────────────────────
  - id: SPEC-010
    name: no-ambiguous-language
    severity: warning
    description: >
      Spec text should not contain ambiguous phrases that leave 
      decisions to the implementer.
    banned_phrases:
      - "as appropriate"
      - "if needed"
      - "as necessary"
      - "etc."
      - "and so on"
      - "obviously"
      - "simply"
      - "just"
      - "should probably"
      - "might want to"
      - "use your judgment"
      - "common sense"
      - "standard practice"
      - "the usual way"
      
  - id: SPEC-011
    name: concrete-error-responses
    severity: error
    description: >
      Error responses must specify exact HTTP status codes and 
      response body structure, not just "return an error".
    check: |
      for each edge_case in spec.behaviors.*.edge_cases:
        assert edge_case.then contains specific HTTP status code
        assert edge_case.then contains response body description
        
  # ─── SECURITY RULES ──────────────────────────────────────
  - id: SPEC-020
    name: enumeration-prevention
    severity: error
    description: >
      Endpoints that accept user identifiers (email, username) must 
      specify identical responses for existing and non-existing users.
    check: |
      for each endpoint accepting user identifiers:
        assert exists edge_case for "user not found"
        assert "not found" response is identical to success response
        
  - id: SPEC-021
    name: rate-limiting-specified
    severity: warning
    description: >
      Write endpoints should specify rate limiting behavior.
    check: |
      for each endpoint where method in [POST, PUT, PATCH, DELETE]:
        assert endpoint.rate_limit is specified
        
  # ─── TESTABILITY RULES ───────────────────────────────────
  - id: SPEC-030
    name: behaviors-are-observable
    severity: error
    description: >
      Every 'then' clause must describe an externally observable outcome,
      not an internal implementation detail.
    check: |
      for each then_clause in spec.behaviors.*.then:
        assert then_clause describes one of:
          - HTTP response (status, body, headers)
          - External service call (to a twin)
          - Observable state change (via API query)
          - Event emission
        assert then_clause does NOT describe:
          - Internal variable values
          - Log messages
          - Code structure
```

### 3.4 Spec Quality Report Output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  SPEC QUALITY REPORT: spec-password-reset v2.1.0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Overall Score: 91/100   [APPROVED - Ready for autonomous implementation]

  ┌─────────────────┬───────┬────────────────────────────────┐
  │ Category        │ Score │ Details                        │
  ├─────────────────┼───────┼────────────────────────────────┤
  │ Completeness    │ 94    │ 16/17 behaviors have edge cases│
  │ Clarity         │ 88    │ 1 ambiguous phrase detected    │
  │ Security        │ 95    │ Enumeration prevention: ✓      │
  │ Testability     │ 90    │ All outcomes are observable    │
  │ Data Model      │ 88    │ All transitions have invariants│
  └─────────────────┴───────┴────────────────────────────────┘

  ERRORS (must fix):
    None

  WARNINGS (should fix):
    ⚠ SPEC-004: Edge case "concurrent-reset" has no acceptance criterion
    ⚠ SPEC-010: Line 142: "standard practice" is ambiguous — specify exactly

  SUGGESTIONS:
    💡 Consider adding behavior for: "What if user changes email while 
       reset is pending?"
    💡 Consider specifying: maximum password length (prevent DoS via 
       bcrypt with very long passwords)
    💡 Consider adding: audit log format specification
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## Part 4: Digital Twin Universe — The Safe Development Environment

### 4.1 Twin Architecture

Each digital twin is a **stateful HTTP server** that simulates an external service's API. Unlike mocks (which return canned responses) or stubs (which have no state), twins maintain **realistic state** that evolves across requests.

```
┌──────────────────────────────────────────────────────┐
│                    TWIN ANATOMY                       │
│                                                       │
│  ┌─────────────────────────────────────────────┐     │
│  │  CONTRACT LAYER                              │     │
│  │  (OpenAPI spec of the real service)          │     │
│  │  - Validates all incoming requests           │     │
│  │  - Rejects malformed requests                │     │
│  │  - Enforces required fields, types, formats  │     │
│  └──────────────┬──────────────────────────────┘     │
│                 │                                      │
│  ┌──────────────▼──────────────────────────────┐     │
│  │  BEHAVIOR LAYER                              │     │
│  │  (Declarative handler definitions)           │     │
│  │  - Maps endpoints to state operations        │     │
│  │  - Generates realistic IDs                   │     │
│  │  - Applies business rules                    │     │
│  │  - Queues webhook events                     │     │
│  └──────────────┬──────────────────────────────┘     │
│                 │                                      │
│  ┌──────────────▼──────────────────────────────┐     │
│  │  STATE LAYER                                 │     │
│  │  (In-memory database with collections)       │     │
│  │  - CRUD operations on entities              │     │
│  │  - Supports queries and filters              │     │
│  │  - Snapshot and restore capability          │     │
│  │  - Transactional consistency                 │     │
│  └──────────────┬──────────────────────────────┘     │
│                 │                                      │
│  ┌──────────────▼──────────────────────────────┐     │
│  │  EVENT LAYER                                 │     │
│  │  (Webhook and event simulation)              │     │
│  │  - Queues events from state changes          │     │
│  │  - Delivers webhooks with configurable delay │     │
│  │  - Supports retry on delivery failure         │     │
│  │  - Event log for debugging                   │     │
│  └─────────────────────────────────────────────┘     │
│                                                       │
│  ┌─────────────────────────────────────────────┐     │
│  │  CHAOS LAYER (Optional)                      │     │
│  │  - Random latency injection                  │     │
│  │  - Intermittent 5xx errors                   │     │
│  │  - Rate limiting simulation                  │     │
│  │  - Timeout simulation                         │     │
│  └─────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────┘
```

### 4.2 Declarative Twin Definition

Twins are defined declaratively in YAML. The twin runtime reads these definitions and creates fully functional HTTP servers. This means **adding a new twin requires zero code** — just a YAML definition and the real service's OpenAPI spec.

```yaml
# twins/catalog/sendgrid-twin/definition.yaml
twin:
  name: sendgrid
  display_name: "SendGrid Email Service"
  version: "v3"
  description: "Simulates SendGrid's email sending API with template support"
  
  # The real service's OpenAPI spec — used for contract validation
  openapi: ./sendgrid-v3-openapi.yaml
  
  # Authentication simulation
  auth:
    type: bearer
    header: Authorization
    prefix: "Bearer "
    # Accept any token matching this pattern (or a fixed test token)
    valid_pattern: "SG\\..+"
    unauthorized_response:
      status: 401
      body: { errors: [{ message: "authorization required" }] }
  
  # State collections (the twin's "database")
  state:
    collections:
      messages:
        schema:
          id: { type: string, generated: true, prefix: "msg_" }
          to: { type: array, items: { type: object } }
          from: { type: object }
          subject: { type: string }
          content: { type: array }
          template_id: { type: string, nullable: true }
          status: { type: string, default: "sent" }
          created_at: { type: datetime, auto: true }
          
      templates:
        schema:
          id: { type: string }
          name: { type: string }
          versions: { type: array }
          
      # Special collection: the "outbox" — all sent emails for inspection
      outbox:
        schema:
          message_id: { type: string }
          to_email: { type: string }
          from_email: { type: string }
          subject: { type: string }
          body_text: { type: string }
          body_html: { type: string }
          sent_at: { type: datetime }
          
  # Endpoint handlers
  handlers:
    # ─── Send Email ─────────────────────────────────────────
    POST /v3/mail/send:
      description: "Send an email"
      action: create
      collection: messages
      
      # Transform request body to collection record
      transform:
        to: request.personalizations[0].to
        from: request.from
        subject: request.personalizations[0].subject
        content: request.content
        template_id: request.template_id
        
      # Side effect: also add to the outbox (for scenario inspection)
      side_effects:
        - action: create
          collection: outbox
          data:
            message_id: ${response.id}
            to_email: ${request.personalizations[0].to[0].email}
            from_email: ${request.from.email}
            subject: ${request.personalizations[0].subject}
            body_html: ${request.content[?(@.type=='text/html')].value}
            body_text: ${request.content[?(@.type=='text/plain')].value}
            
      response:
        status: 202
        headers:
          X-Message-Id: ${response.id}
        body: null  # SendGrid returns empty body on success
        
      webhooks:
        - event: email.delivered
          delay: 500ms
          payload:
            email: ${request.personalizations[0].to[0].email}
            event: delivered
            sg_message_id: ${response.id}
            timestamp: ${now}
            
    # ─── Get Email Activity ────────────────────────────────
    GET /v3/messages:
      description: "List sent messages"
      action: list
      collection: messages
      pagination:
        limit_param: limit
        offset_param: offset
        default_limit: 10
        
    # ─── Templates ──────────────────────────────────────────
    GET /v3/templates:
      action: list
      collection: templates
      
    GET /v3/templates/{id}:
      action: read
      collection: templates
      not_found:
        status: 404
        body: { errors: [{ message: "resource not found" }] }
        
  # ─── Inspection API (Twin-only, not part of real SendGrid) ──
  # These endpoints let scenarios inspect twin state
  inspection:
    GET /__twin/outbox:
      description: "Get all sent emails (for test assertions)"
      action: list
      collection: outbox
      
    GET /__twin/outbox/latest:
      description: "Get the most recently sent email"
      action: read_latest
      collection: outbox
      
    DELETE /__twin/state:
      description: "Reset all twin state"
      action: reset_all
      
    GET /__twin/health:
      description: "Health check"
      action: health
      
  # Error simulation rules
  error_simulation:
    rate_limit:
      enabled: true
      requests_per_second: 10
      response:
        status: 429
        body: { errors: [{ message: "rate limit exceeded" }] }
```

### 4.3 Universe Composition

Individual twins compose into a **Universe** — a coordinated environment where twins interact with each other and with the application under development.

```yaml
# twins/universe/manifests/password-reset-universe.yaml
universe:
  name: password-reset-dev
  description: "Development universe for password reset feature"
  
  # ─── TWIN INSTANCES ──────────────────────────────────────
  twins:
    postgres:
      twin: postgres-twin
      config:
        databases:
          - name: appdb
            migrations: ${APP_MIGRATIONS_DIR}
            
    sendgrid:
      twin: sendgrid-twin
      config:
        api_key: SG.twin_password_reset_test
        
  # ─── APPLICATION ──────────────────────────────────────────
  application:
    # The agent's built artifact
    source: ${AGENT_WORKSPACE}
    build_command: "cargo build --release"
    run_command: "./target/release/app serve"
    
    # Environment variables — point to twin endpoints
    env:
      DATABASE_URL: "postgresql://twin:twin@${postgres.host}:${postgres.port}/appdb"
      SENDGRID_API_KEY: "${sendgrid.config.api_key}"
      SENDGRID_BASE_URL: "${sendgrid.endpoint}"  # Override production URL
      APP_BASE_URL: "http://localhost:8080"
      RESET_TOKEN_EXPIRY_MINUTES: "15"
      
    # Health check to know when app is ready
    health_check:
      path: /health
      interval: 1s
      timeout: 30s
      
  # ─── WEBHOOK ROUTING ─────────────────────────────────────
  webhooks:
    - from: sendgrid
      events: ["email.*"]
      to: application
      path: /webhooks/sendgrid
      
  # ─── INITIAL STATE ───────────────────────────────────────
  # Pre-populate twins with data needed for development/testing
  initial_state:
    postgres:
      appdb:
        users:
          - id: "550e8400-e29b-41d4-a716-446655440001"
            email: "alice@example.com"
            email_verified: true
            password_hash: "$2b$12$..."  # hash of "OldPassword123!"
            created_at: "2024-01-01T00:00:00Z"
            
          - id: "550e8400-e29b-41d4-a716-446655440002"
            email: "bob@example.com"
            email_verified: false
            password_hash: "$2b$12$..."
            created_at: "2024-01-15T00:00:00Z"
            
    sendgrid:
      templates:
        - id: "d-resetpassword001"
          name: "Password Reset"
          versions:
            - subject: "Reset your password"
              html_content: "<h1>Reset Password</h1><a href='{{reset_url}}'>Click here</a>"
              
  # ─── NETWORKING ───────────────────────────────────────────
  networking:
    mode: bridge  # All services on the same virtual network
    
    # Port mappings (host:container)
    ports:
      application: 8080:8080
      postgres: 5433:5432
      sendgrid: 9001:443
      
  # ─── LIFECYCLE ────────────────────────────────────────────
  lifecycle:
    startup_order:
      - postgres        # Database first
      - sendgrid        # Email service
      - application     # App last (needs all dependencies)
      
    shutdown_order:
      - application
      - sendgrid
      - postgres
      
    # Auto-reset between test runs
    reset_between_runs: true
    
    # Snapshot state after initial setup for fast resets
    snapshot_after_init: true
```

### 4.4 Universe Runtime Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                      UNIVERSE RUNTIME                             │
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ postgres-twin│  │sendgrid-twin │  │  APPLICATION │          │
│  │  :5432       │  │  :443        │  │  :8080       │          │
│  │              │  │              │  │              │          │
│  │  State:      │  │  State:      │  │  Connects to:│          │
│  │  - users     │  │  - templates │  │  - postgres  │          │
│  │  - tokens    │  │  - outbox    │  │  - sendgrid  │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                  │                  │                   │
│         └──────────────────┼──────────────────┘                   │
│                            │                                      │
│  ┌─────────────────────────▼─────────────────────────────────┐   │
│  │              WEBHOOK RELAY / EVENT BUS                     │   │
│  │                                                             │   │
│  │  sendgrid.email.delivered ──► POST app:8080/webhooks/sgrid │   │
│  │  app.password.reset.completed ──► (logged, no consumer)  │   │
│  │                                                             │   │
│  │  Event Log: [t=0ms] universe.started                       │   │
│  │             [t=50ms] postgres.ready                         │   │
│  │             [t=80ms] sendgrid.ready                        │   │
│  │             [t=2000ms] application.ready                    │   │
│  │             [t=2100ms] universe.ready ✓                    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              STATE MANAGER                                   │   │
│  │                                                             │   │
│  │  snapshot("initial") → saved at t=2100ms                   │   │
│  │  restore("initial") → all twins reset to snapshot          │   │
│  │  export() → JSON dump of all twin states                   │   │
│  │  import(state) → load state from JSON                      │   │
│  └─────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

---

## Part 5: Behavioral Scenarios — The Holdout Set

### 5.1 The Information Barrier

This is the most critical architectural property. The scenarios must be **physically inaccessible** to the AI agent during development.

```
┌─────────────────────────────────────────────────────────────┐
│                    INFORMATION BARRIER                        │
│                                                               │
│   AGENT SIDE (can access)     │    HOLDOUT SIDE (cannot)     │
│   ─────────────────────────   │    ──────────────────────    │
│                               │                               │
│   ✓ Spec (full detail)       │    ✗ Scenario definitions     │
│   ✓ Twin endpoints           │    ✗ Scenario assertions      │
│   ✓ Twin inspection APIs     │    ✗ Scenario step sequences  │
│   ✓ Acceptance criteria      │    ✗ Expected HTTP responses  │
│   ✓ Own test results         │    ✗ Edge case test data      │
│   ✓ Sanitized feedback       │    ✗ Raw validation results   │
│                               │                               │
│   ENFORCEMENT LAYERS:         │                               │
│   1. Filesystem isolation     │    Separate repository        │
│   2. Network isolation        │    Different network segment  │
│   3. Process isolation        │    Runs as separate service   │
│   4. API isolation            │    No scenario read endpoints │
│   5. Credential isolation     │    Different access keys      │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 Scenario Schema

```yaml
# scenarios/schema/scenario.schema.yaml
scenario:
  # ─── IDENTITY ───────────────────────────────────────────
  id: string                    # e.g., "scn-reset-expired-token-001"
  spec_ref: string              # Which spec this validates
  spec_version: string          # Which version of the spec
  category: string              # e.g., "error-handling", "happy-path", "security"
  visibility: enum              # "holdout" | "visible" | "regression"
  priority: enum                # "critical" | "high" | "medium" | "low"
  
  description: string           # Human-readable description
  rationale: string             # Why this scenario exists / what gap it covers
  
  # ─── PREREQUISITES ─────────────────────────────────────
  setup:
    universe: string            # Which universe manifest to use
    initial_state: string       # Named state snapshot to load
    preconditions:              # Assertions that must hold before test starts
      - description: string
        check: assertion        # API call + expected result
        
  # ─── STEPS ─────────────────────────────────────────────
  steps:
    - id: string                # Step identifier for debugging
      description: string       # What this step does
      action: action_spec       # The action to perform
      assertions: assertion[]   # What to verify after this step
      extractions: extraction[] # Values to extract for later steps
      
  # ─── TEARDOWN ──────────────────────────────────────────
  teardown:
    reset_universe: bool        # Reset all twin state after scenario
    custom_cleanup: action[]    # Additional cleanup steps
```

### 5.3 Full Scenario Examples

```yaml
# scenarios/vault/password-reset/happy-path.yaml
# 
# THIS FILE LIVES IN A SEPARATE REPOSITORY
# THE AI AGENT NEVER SEES THIS FILE
#
scenario:
  id: scn-reset-happy-001
  spec_ref: spec-password-reset
  spec_version: "2.1.0"
  category: happy-path
  visibility: holdout
  priority: critical
  
  description: "Complete password reset flow from request to successful login with new password"
  rationale: >
    This is the primary use case. If this doesn't work, nothing else matters.
    Tests the full chain: request → email → token → reset → login.
  
  setup:
    universe: password-reset-dev
    initial_state: baseline
    preconditions:
      - description: "Alice's account exists and is accessible"
        check:
          action: query_twin
          twin: postgres
          query: "SELECT id, email, email_verified FROM users WHERE email = 'alice@example.com'"
          expect:
            rows: 1
            row_0:
              email_verified: true
              
  steps:
    # Step 1: Request password reset
    - id: request-reset
      description: "Alice requests a password reset"
      action:
        type: http
        method: POST
        url: ${application.endpoint}/api/v1/auth/reset-password/request
        headers:
          Content-Type: application/json
        body:
          email: "alice@example.com"
      assertions:
        - type: status
          expected: 200
        - type: body_json
          path: $.message
          expected: "If an account with that email exists, a reset link has been sent."
          
    # Step 2: Verify email was sent
    - id: check-email-sent
      description: "Verify that a reset email was sent via SendGrid twin"
      action:
        type: http
        method: GET
        url: ${sendgrid.endpoint}/__twin/outbox/latest
      assertions:
        - type: body_json
          path: $.to_email
          expected: "alice@example.com"
        - type: body_json
          path: $.subject
          operator: contains
          expected: "Reset"  # Not exact — allows some flexibility
      extractions:
        - name: reset_url
          from: body_json
          path: $.body_html
          regex: 'href="([^"]*reset-password[^"]*)"'
          # Extracts the reset URL from the email HTML
          
    # Step 3: Extract token from URL
    - id: extract-token
      description: "Extract the reset token from the email link"
      action:
        type: extract
        from: ${reset_url}
        regex: 'token=([^&]+)'
      extractions:
        - name: reset_token
          group: 1
          
    # Step 4: Complete password reset
    - id: complete-reset
      description: "Alice uses the token to set a new password"
      action:
        type: http
        method: POST
        url: ${application.endpoint}/api/v1/auth/reset-password/complete
        headers:
          Content-Type: application/json
        body:
          token: ${reset_token}
          new_password: "NewSecurePassword456!"
      assertions:
        - type: status
          expected: 200
          
    # Step 5: Verify login with new password works
    - id: login-new-password
      description: "Alice can log in with the new password"
      action:
        type: http
        method: POST
        url: ${application.endpoint}/api/v1/auth/login
        headers:
          Content-Type: application/json
        body:
          email: "alice@example.com"
          password: "NewSecurePassword456!"
      assertions:
        - type: status
          expected: 200
        - type: body_json
          path: $.token
          operator: exists
          
    # Step 6: Verify old password no longer works
    - id: login-old-password-fails
      description: "Alice cannot log in with the old password"
      action:
        type: http
        method: POST
        url: ${application.endpoint}/api/v1/auth/login
        headers:
          Content-Type: application/json
        body:
          email: "alice@example.com"
          password: "OldPassword123!"
      assertions:
        - type: status
          expected: 401
          
  teardown:
    reset_universe: true

---
# scenarios/vault/password-reset/expired-token.yaml
scenario:
  id: scn-reset-expired-001
  spec_ref: spec-password-reset
  spec_version: "2.1.0"
  category: error-handling
  visibility: holdout
  priority: critical
  
  description: "Expired reset token returns proper error with recovery guidance"
  rationale: >
    This catches a common agent mistake: not implementing token expiry checks,
    which leads to 500 errors instead of graceful 410 responses.
  
  setup:
    universe: password-reset-dev
    initial_state: baseline
    # Pre-insert an expired token directly into the database twin
    custom_setup:
      - action: insert_twin
        twin: postgres
        table: password_reset_tokens
        data:
          id: "550e8400-e29b-41d4-a716-446655440099"
          user_id: "550e8400-e29b-41d4-a716-446655440001"
          token_hash: "$2b$12$expired_token_hash_here"
          created_at: "2024-01-01T00:00:00Z"      # Way in the past
          expires_at: "2024-01-01T00:15:00Z"     # Already expired
          status: "active"
          
  steps:
    - id: use-expired-token
      description: "Attempt to reset password with expired token"
      action:
        type: http
        method: POST
        url: ${application.endpoint}/api/v1/auth/reset-password/complete
        body:
          token: "expired_test_token_value"
          new_password: "NewSecurePassword456!"
      assertions:
        - type: status
          expected: 410
        - type: body_json
          path: $.error
          expected: "token_expired"
        - type: body_json
          path: $.action
          operator: exists
          # Must tell user what to do next (request a new reset)
          
  teardown:
    reset_universe: true

---
# scenarios/vault/password-reset/enumeration-prevention.yaml
scenario:
  id: scn-reset-enumeration-001
  spec_ref: spec-password-reset
  spec_version: "2.1.0"
  category: security
  visibility: holdout
  priority: critical
  
  description: "Reset request for non-existent email returns identical response to valid email"
  rationale: >
    Security-critical: if responses differ between existing and non-existing emails,
    attackers can enumerate valid email addresses. This catches agents that take
    shortcuts and return 404 for missing emails.

  steps:
    # Step 1: Request reset for EXISTING email, capture response
    - id: existing-email
      description: "Request reset for existing email"
      action:
        type: http
        method: POST
        url: ${application.endpoint}/api/v1/auth/reset-password/request
        body:
          email: "alice@example.com"
      extractions:
        - name: valid_status
          from: status
        - name: valid_body
          from: body
        - name: valid_timing
          from: response_time_ms
          
    # Step 2: Request reset for NON-EXISTING email, capture response
    - id: nonexistent-email
      description: "Request reset for non-existent email"
      action:
        type: http
        method: POST
        url: ${application.endpoint}/api/v1/auth/reset-password/request
        body:
          email: "definitely-not-a-real-user@example.com"
      extractions:
        - name: invalid_status
          from: status
        - name: invalid_body
          from: body
        - name: invalid_timing
          from: response_time_ms
          
    # Step 3: Compare responses — they must be identical
    - id: compare-responses
      description: "Responses for existing and non-existing emails must be identical"
      action:
        type: compare
      assertions:
        - type: equal
          left: ${valid_status}
          right: ${invalid_status}
          message: "HTTP status codes must match"
        - type: equal
          left: ${valid_body}
          right: ${invalid_body}
          message: "Response bodies must be identical"
        - type: timing_similar
          left: ${valid_timing}
          right: ${invalid_timing}
          tolerance_ms: 100
          message: "Response times must be similar (prevent timing attacks)"
          
    # Step 4: Verify no email was sent for non-existent user
    - id: check-no-email
      description: "No email should be sent for non-existent user"
      action:
        type: http
        method: GET
        url: ${sendgrid.endpoint}/__twin/outbox
        params:
          to_email: "definitely-not-a-real-user@example.com"
      assertions:
        - type: body_json
          path: $.length()
          expected: 0
          message: "No email should be sent to non-existent address"
```

### 5.4 Feedback Sanitization Pipeline

When a scenario fails, the raw results contain details the agent should NOT see. The sanitizer transforms raw failures into helpful-but-safe feedback.

```
RAW FAILURE (scenario runner output):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Scenario: scn-reset-enumeration-001 [FAILED]
Step: compare-responses
Assertion: equal(valid_status=200, invalid_status=404) FAILED
  Expected: 200 == 404  →  false
  Left (existing email): HTTP 200, body: {"message": "If an account..."}
  Right (non-existing): HTTP 404, body: {"error": "user_not_found"}
  
Assertion: timing_similar(valid=45ms, invalid=12ms) FAILED
  Difference: 33ms (tolerance: 100ms) — PASSED
  already failed
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

          But status code assertion │
          │  Sanitization Pipeline
          │
          ▼

SANITIZED FEEDBACK (what the agent receives):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Behavioral Validation Result: FAILED (17/20 passed)

Failure 1 of 3:
  Category: Security — User Enumeration Prevention
  Spec Reference: spec-password-reset-v2, behavior "nonexistent-email"
  
  Description: The system reveals whether an email address exists in
  the database based on the HTTP response. This is a security 
  vulnerability that allows attackers to enumerate valid accounts.
  
  Hint: The password reset request endpoint should return identical
  responses (same status code, same body, similar timing) regardless
  of whether the email exists in the system. Review the spec's 
  edge case "nonexistent-email" for the required behavior.
  
  Related Spec Text: "The API responds with the SAME 200 response 
  (prevent enumeration)"

Failure 2 of 3:
  Category: Error Handling — Token Spec Reference: spec-password-reset-v Expiry
 2, behavior "expired-token"
  
  Description: The system does not gracefully handle expired reset 
  tokens. Instead of returning a clear error with recovery guidance,
  the system returns an internal server error.
  
  Hint: Implement explicit token expiry checking before processing 
  the reset. Return HTTP 410 with an error type and an action field 
  that tells the user how to request a new reset.

Failure 3 of 3:
  Category: Session Management — Session Invalidation
  Spec Reference: spec-password-reset-v2, behavior "complete-reset"
  
  Description: After a successful password reset, existing sessions
  for the user are not invalidated. This means an attacker who has
  an active session can continue using it even after the password 
  is changed.
  
  Hint: After updating the password hash, invalidate all existing 
  sessions/tokens for the affected user. The spec states: "All 
  existing sessions for this user are invalidated."
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

Notice what the sanitized feedback **includes**:
- Which behavioral category failed
- Reference to the spec (which the agent already has)
- A natural language description of WHAT went wrong
- A hint about HOW to fix it
- Relevant spec text

And what it **excludes**:
- The exact HTTP requests made
- The exact assertion values
- The step sequence
- The scenario ID or structure
- The specific test data used

### 5.5 Feedback Levels

Organizations can configure how much detail the agent receives:

| Level | Name | Agent Sees | Use When |
|-------|------|-----------|----------|
| 1 | Minimal | "3 of 20 behavioral tests failed" | Maximum holdout security |
| 2 | Categorical | Level 1 + failure categories | Agent needs direction |
| 3 | **Guided** (default) | Level 2 + descriptions + hints + spec refs | **Best balance** |
| 4 | Diagnostic | Level 3 + HTTP status codes (not bodies) | Agent is stuck |
| 5 | Transparent | Full raw failure details | Debugging, not for holdout |

---

## Part 6: The Orchestrator — The State Machine That Ties Everything Together

### 6.1 Pipeline State Machine

```
                                        ┌──────────┐
                                        │  START   │
                                        └────┬─────┘
                                             │
                                     ┌───────▼──────────┐
                              ┌──────|   SPEC_REVIEW     |◄──────────┐
                              │      │                    │           │
                              │      │ Run spec linter    │           │
                              │      │ Score ≥ threshold? │           │
                              │      └───────┬──────────┘           │
                              │              │                       │
                          score < 80    score ≥ 80              spec revised
                              │              │                       │
                              ▼              ▼                       │
                     ┌────────────┐  ┌───────────────────┐          │
                     │ SPEC_REJECT│  │  UNIVERSE_SETUP    │          │
                     │            │  │                    │          │
                     │ Return     │  │ Parse dependencies │          │
                     │ quality    │  │ Spin up twins      │          │
                     │ report to  │  │ Load initial state │          │
                     │ human      │  │ Configure webhooks │          │
                     └────────────┘  │ Health check all   │          │
                                     └───────┬───────────┘          │
                                             │                       │
                                             ▼                       │
                                     ┌───────────────────┐          │
                              ┌─────►│   AGENT_DEV        │          │
                              │      │                    │          │
                              │      │ Agent receives:    │          │
                              │      │ - Spec             │          │
                              │      │ - Twin endpoints   │          │
                              │      │ - Feedback (if any)│          │
                              │      │                    │          │
                              │      │ Agent produces:    │          │
                              │      │ - Built artifact   │          │
                              │      └───────┬───────────┘          │
                              │              │                       │
                              │        agent signals                 │
                              │          "done"                      │
                              │              │                       │
                              │              ▼                       │
                              │      ┌───────────────────┐          │
                              │      │   VALIDATION       │          │
                              │      │                    │          │
                              │      │ Deploy artifact    │          │
                              │      │ into fresh universe│          │
                              │      │                    │          │
                              │      │ Run ALL holdout    │          │
                              │      │ scenarios          │          │
                              │      └───────┬───────────┘          │
                              │              │                       │
                              │     ┌────────┼────────┐              │
                              │     │        │        │              │
                              │  all pass  some    iterations       │
                              │     │      fail    ≥ max            │
                              │     │        │        │              │
                              │     ▼        ▼        ▼              │
                              │  ┌──────┐ ┌──────┐ ┌──────────┐    │
                              │  │ACCEPT│ │FEED- │ │ESCALATION│    │
                              │  │      │ │BACK  │ │          │    │
                              │  │Merge │ │      │ │Human gets│    │
                              │  │to    │ │Sani- │ │full logs │    │
                              │  │main  │ │tize  │ │+ scenarios│   │
                              │  │      │ │results│ │          │    │
                              │  └──────┘ │Send  │ │Human can:│────┘
                              │           │to    │ │- Fix spec│
                              └───────────│agent │ │- Fix code│
                                          └──────┘ │- Override│
```

### 6.2 Orchestrator Configuration

```yaml
# orchestrator/pipeline.yaml
pipeline:
  name: autonomous-development
  version: "1.0"
  
  # ─── GLOBAL SETTINGS ────────────────────────────────────
  settings:
    max_agent_iterations: 5
    feedback_level: 3                # Guided (default)
    spec_quality_threshold: 80       # Minimum score to proceed
    agent_timeout_per_iteration: 30m # Max time per development iteration
    total_timeout: 4h                # Max total time for entire pipeline
    
  # ─── STAGE DEFINITIONS ─────────────────────────────────
  stages:
    # Stage 1: Validate the spec
    - name: spec-review
      action: lint-spec
      inputs:
        spec_path: ${SPEC_PATH}
        rules: specs/linter/rules.yaml
      gate:
        condition: "output.score >= settings.spec_quality_threshold"
        on_fail:
          action: reject
          message: "Spec quality score {output.score} below threshold {threshold}"
          artifacts: [output.report]
          
    # Stage 2: Set up the twin universe
    - name: universe-setup
      action: compose-universe
      inputs:
        manifest: ${spec.universe_manifest}
        app_workspace: ${AGENT_WORKSPACE}
      timeout: 5m
      gate:
        condition: "output.all_healthy == true"
        on_fail:
          action: abort
          message: "Twin universe failed to initialize: {output.unhealthy_twins}"
          
    # Stage 3: Agent development loop
    - name: agent-develop
      action: agent-session
      inputs:
        spec: ${SPEC_PATH}
        twin_endpoints: ${universe-setup.output.endpoints}
        workspace: ${AGENT_WORKSPACE}
        feedback: ${validation.output.feedback}  # null on first iteration
      timeout: ${settings.agent_timeout_per_iteration}
      
    # Stage 4: Behavioral validation
    - name: validation
      action: run-scenarios
      inputs:
        artifact: ${agent-develop.output.artifact}
        universe_manifest: ${spec.universe_manifest}
        scenario_vault: ${SCENARIO_VAULT_PATH}/${spec.id}
        feedback_level: ${settings.feedback_level}
      gate:
        condition: "output.all_passed == true"
        on_pass:
          action: accept
          next: acceptance
        on_fail:
          action: check-iterations
          
    # Stage 5a: Check if we should retry or escalate
    - name: check-iterations
      action: evaluate
      condition: "pipeline.iteration_count < settings.max_agent_iterations"
      on_true:
        next: agent-develop  # Go back to agent with feedback
      on_false:
        next: escalation
        
    # Stage 5b: Acceptance
    - name: acceptance
      action: accept-artifact
      inputs:
        artifact: ${agent-develop.output.artifact}
        spec: ${SPEC_PATH}
        validation_report: ${validation.output.report}
      outputs:
        - merge_to_main: true
        - create_pr: true
        - notify_author: true
        
    # Stage 5c: Escalation
    - name: escalation
      action: escalate-to-human
      inputs:
        spec: ${SPEC_PATH}
        all_iteration_logs: ${pipeline.iteration_logs}
        validation_results: ${validation.output.raw_results}  # Full details for human
        agent_code: ${agent-develop.output.artifact}
      outputs:
        - create_issue: true
        - assign_to: ${spec.identity.author}
        - include_diagnosis: true
```

### 6.3 Metrics and Observability

The orchestrator tracks metrics that help improve the system over time:

```yaml
metrics:
  per_spec:
    - spec_id: string
    - spec_quality_score: number
    - iterations_to_pass: number          # How many rounds the agent needed
    - first_pass_rate: percentage          # % of scenarios passing on first try
    - total_development_time: duration     # Wall clock time
    - agent_compute_cost: currency         # Token/compute cost
    - failure_categories: map<string, int> # Which categories failed most
    
  aggregate:
    - avg_iterations_to_pass: number       # Across all specs
    - spec_quality_vs_iterations: correlation  # Higher quality = fewer iterations?
    - most_common_failure_categories: ranked_list
    - agent_improvement_over_time: trend    # Is the agent getting better_coverage: percentage             # What % of external services have?
    - twin twins?
    - scenario_coverage: percentage         # What % of spec behaviors have scenarios?
    
  alerts:
    - if: avg_iterations_to_pass > 3
      then: "Specs may not be detailed enough — review spec quality standards"
    - if: same_failure_category >_week
      then 5_times_in: "Systemic gap — agent consistently fails at {category}"
    - if: twin_coverage < 80%
      then: "Missing twins may be causing integration failures"
```

---

## Part 7: The Agent Interface — What the AI Agent Sees

This section defines the **exact interface** between the orchestrator and the AI agent. The agent is a black box that receives inputs and produces outputs.

### 7.1 Agent Input Package

```yaml
# This is what the agent receives at the start of each iteration
agent_input:
  # ─── SPECIFICATION ──────────────────────────────────────
  spec:
    path: /workspace/spec.yaml
    # Full spec as defined in Part 3
    
  # ─── TWIN ENDPOINTS ────────────────────────────────────
  environment:
    services:
      sendgrid:
        base_url: "http://sendgrid-twin:443"
        auth_header: "Bearer SG.twin_password_reset_test"
        inspection_url: "http://sendgrid-twin:443/__twin"
        
      postgres:
        connection_string: "postgresql://twin:twin@postgres-twin:5432/appdb"
        
    application:
      base_url: "http://localhost:8080"
      
  # ─── WORKSPACE ──────────────────────────────────────────
  workspace:
    path: /workspace/code
    language: rust        # or whatever the project uses
    build_command: "cargo build"
    test_command: "cargo test"
    run_command: "cargo run -- serve"
    
  # ─── FEEDBACK (null on first iteration) ─────────────────
  feedback: null
  # On subsequent iterations, this contains sanitized failure reports
  
  # ─── CONSTRAINTS ────────────────────────────────────────
  constraints:
    max_duration: "30m"
    must_compile: true
    must_pass_own_tests: true
    code_style: "cargo fmt && cargo clippy"
    
  # ─── ITERATION CONTEXT ─────────────────────────────────
  iteration:
    number: 1
    max: 5
    previous_results: null  # Summary of previous attempt
```

### 7.2 Agent Output Package

```yaml
# This is what the agent produces
agent_output:
  # ─── BUILD ARTIFACT ────────────────────────────────────
  artifact:
    type: docker_image       # or binary, or source_bundle
    path: /workspace/output/app.tar
    
  # ─── SELF-ASSESSMENT ───────────────────────────────────
  self_report:
    compilation: pass
    own_tests: 23/23 pass
    lint: pass
    confidence: "high"       # Agent's self-assessed confidence
    notes: "Implemented all behaviors from spec. Added rate limiting 
            middleware. Used bcrypt for token hashing."
            
  # ─── CODE CHANGES ──────────────────────────────────────
  changes:
    files_created: [...]
    files_modified: [...]
    files_deleted: [...]
    total_lines_added: number
    total_lines_removed: number
```

---

## Part 8: Implementation Roadmap

### Phase 1: MVP — Prove the Triangle (Weeks 1-4)

```
Week 1: Spec System
  ├─ Define spec schema (JSON Schema)
  ├─ Write 1 complete spec (password reset)
  ├─ Build basic spec linter (5 rules)
  └─ Deliverable: spec + linter that outputs quality score

Week 2: Twin System  
  ├─ Build generic HTTP twin framework (Rust)
  │   - Accept request → match route → update state → respond
  ├─ Create 1 twin (SendGrid email)
  ├─ Create Postgres twin (use real Postgres in Docker, just manage state)
  └─ Deliverable: Twin universe that can send/receive HTTP

Week 3: Scenario System
  ├─ Define scenario schema
  ├─ Write 5 scenarios for password reset
  ├─ Build scenario runner (reads YAML, makes HTTP calls, checks assertions)
  ├─ Build feedback sanitizer (Level 3)
  └─ Deliverable: Scenario runner that validates against twin universe

Week 4: Orchestrator + Integration
  ├─ Shell script orchestrator (linear pipeline)
  ├─ Wire: spec lint → universe setup → agent workspace → validation
  ├─ Manual agent test (human plays the agent role)
  ├─ Then: AI agent test (real agent)
  └─ Deliverable: Full loop running end-to-end
```

### Phase 2: Foundation (Months 2-3)

```
- Twin framework: Declarative YAML twin definitions
- Twin catalog: Auth0, Stripe, SendGrid, S3, generic HTTP
- Spec linter: 20+ rules, quality report
- Scenario runner: Parallel execution, extraction, timing assertions
- Orchestrator: Proper state machine (not shell script)
- Feedback: All 5 levels, configurable per org
- Metrics: Basic dashboards
```

### Phase 3: Scale (Months 4-6)

```
- Parallel agent sessions (multiple specs simultaneously)
- Universe snapshots (instant reset between test runs)
- Scenario coverage analysis
- Twin recording mode (record production traffic → replay as twin)
- CI/CD integration (GitHub Actions, GitLab CI)
- Multi-language support (Rust, TypeScript, Python, Go)
```

### Phase 4: Intelligence (Months 6+)

```
- Spec quality prediction (ML model: will this spec succeed on first pass?)
- Auto-scenario generation from specs (with human review)
- Agent performance analytics (which agents do best on which spec types?)
- Twin behavior learning (record real API behavior → auto-update twin)
- Continuous twin validation (does the twin still match the real service?)
```

---

## Part 9: Key Architectural Invariants

These are the properties that must **never** be violated, regardless of implementation phase:

| # | Invariant | Why |
|---|-----------|-----|
| 1 | **Agent never sees scenarios** | Prevents overfitting; the entire holdout concept breaks otherwise |
| 2 | **Twins are stateful** | Stateless mocks don't catch integration bugs; a "create then read" sequence must work |
| 3 | **Specs are validated before agent starts** | An agent working from a bad spec wastes compute and produces bad software |
| 4 | **Feedback is sanitized** | Even partial scenario leakage degrades holdout effectiveness over time |
| 5 | **Universe resets between validation runs** | Non-deterministic state makes failures unreproducible |
| 6 | **Twins validate contracts** | If the twin accepts malformed requests, the agent learns bad habits |
| 7 | **Scenarios test from outside** | Internal tests (unit tests) are the agent's job; scenarios test behavior the way a user would |
| 8 | **Escalation is always possible** | The system must never loop forever; humans are the ultimate fallback |
| 9 | **Every iteration is logged** | Debugging requires seeing what the agent tried and why it failed |
| 10 | **Specs are versioned** | When scenarios fail, you need to know which spec version the agent was working from |

---

## Part 10: The Critical Insight — Why This Works

The reason these three concepts, implemented together, enable autonomous software development is that they solve the **three failure modes** of AI-generated code:

**Failure Mode 1: "Built the wrong thing"** — Solved by **high-quality specs**. When every decision point is resolved in the spec, the agent can't build the wrong thing because "the wrong thing" is explicitly defined as a non-goal and the right thing is explicitly defined as acceptance criteria.

**Failure Mode 2: "Works in isolation, breaks in integration"** — Solved by **digital twins**. The agent develops against realistic simulated services that maintain state, enforce contracts, and fire events. Integration bugs surface during development, not after deployment.

**Failure Mode 3: "Passes tests but doesn't actually work"** — Solved by **behavioral scenarios**. Because the agent can't see the holdout tests, it can't optimize for passing them. It must build software that **genuinely implements the specified behavior**, not software that games a test suite.

The triangle is necessary and sufficient. Each leg addresses exactly one failure mode. Remove any leg and that failure mode returns unchecked.

```
     SPECS ────────────── prevent "built the wrong thing"
       │\
       │ \
       │  \
       │   \
    TWINS ──── SCENARIOS
       │           │
  prevent          prevent
  "breaks in       "passes tests
   integration"     but doesn't work"
```

This is the architecture for autonomous software development. The bottleneck is no longer "can AI write code?" — it clearly can. The bottleneck is "can we give AI the right environment, the right instructions, and the right honest evaluation?" That's what the Digital Twin Universe, High-Quality Specs, and Behavioral Scenarios provide.
