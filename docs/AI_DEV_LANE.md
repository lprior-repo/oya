# AI Dev Lane: Conservative, Governed AI Development

> AI as a well-governed junior engineer in a Rust mono‑repo.
> Version: 1.0 | Updated: 2026-02

---

## Philosophy of Control

1. **All AI work happens inside deterministic workflows**, not free-roaming agents.
2. **Tests, types, and contracts—not the LLM—are the source of truth.**
3. **Every stage is ephemeral**; all state lives in code, specs, tests, and logs.
4. **RAG and memory are tools for better planning**, never substitutes for gates.
5. **Humans own intent** (via a strong planner UI); the lane owns execution within strict bounds.
6. **Scope starts narrow** (small Rust changes) and expands only as we earn trust.

This is the "conservative, correct side" of AI development: deterministic workflows, small scoped tasks, tests/typing as ground truth, explicit gates. The "wild" side would be agents with long-lived memory making broad, unbounded changes with weak tests and vague evals.

---

## System Architecture

### Language and Stack
- **Rust only**, written in a purely functional style (see `FUNCTIONAL_RUST.md`)
- Structured mono‑repo with workspaces (JJ workspaces, moon‑style monorepo management)
- CI/CD with a merge queue as the final arbiter for main
- Full‑stack Rust: Axum/Actix backends, CLI tools, and Dioxus/WASM frontends

### Authority Boundaries

| Component | Role |
|-----------|------|
| **OYA/Restate** | Orchestration runtime—decides stage transitions, retries, terminal outcomes |
| **OpenCode Adapter** | Subprocess execution per stage, returns structured output only |
| **Moon** | CI/CD wrapper—executes validation gates, emits pass/fail evidence |
| **jj** | Workspace isolation and merge-flow primitive |
| **Beads (br)** | Intake and lifecycle source of truth for all work items |
| **Codanna MCP** | Code discovery (symbols, callers, calls, impact, dependency tracing) |
| **Sled** | Persistence baseline for run and evidence state |

### Observability
- **OpenObserve**: UI at `http://localhost:5080`, OTLP gRPC at `localhost:4317`
- **Restate**: UI at `http://localhost:9070`, Ingress at `http://localhost:8080`

---

## The Planner Companion App

### Purpose

The planner companion is a **hard front door** to the pipeline. Before a single line of AI code is written, all feature work must pass through it.

### Responsibilities

1. **Grills the user** to produce a high-quality spec:
   - ATDD scenarios with concrete Given/When/Then examples
   - Affected bounded contexts and DDD module boundaries
   - Functional/non-functional constraints (perf, compatibility, security)
   - Explicit scope: what is in and what is out

2. **Validates and rejects vague or unsafe specs**:
   - Too broad or cross‑cutting? Suggest splitting.
   - Ambiguous acceptance criteria? Demand refinement.
   - Risky scope? Escalate to human review.

3. **Outputs a structured spec artifact** that flows directly into the pipeline as a first‑class input.

4. **Is itself a Dioxus UI** built and maintained by the lane (eats its own cooking).

### Spec Quality Gates

The planner enforces spec quality before work begins:
- **Completeness**: Every dependency has error handling, every state transition has invariant checks
- **Clarity**: No ambiguous language ("as appropriate", "if needed", "etc.")
- **Security**: Enumeration prevention, rate limiting specified
- **Testability**: All outcomes are externally observable

---

## Pipeline Stages

### Stage 0: Spec / Planner UI
- Companion planner app grills the user
- Produces structured spec artifact: ATDD scenarios, DDD scope, constraints, risk tier
- Rejects or splits oversized or vague changes

### Stage 1: Scout Phase
- **Codanna MCP** analyzes the Rust workspace/monorepo
- Identifies bounded contexts, modules, types, traits, and functions relevant to the spec
- Builds a token‑efficient context pack
- Runs in parallel across bounded contexts where possible

### Stage 2: ATDD Phase
- Convert spec into concrete ATDD tests the AI does not see directly
- Confirm they fail first (red gate)
- Tests define acceptance criteria, not implementation

### Stage 3: Contract Phase
- Infer and encode a formal contract: types, trait signatures, DDD boundaries, LoC constraints
- Feed contract into implement phase as a hard constraint
- Rust types, traits, and functional constraints form explicit boundaries

### Stage 4: Red Phase
- Run tests: confirm new tests fail for the right reasons
- Existing tests must still pass
- Property tests (if configured) must fail appropriately

### Stage 5: Implement Phase
- Planner decides which files/functions to touch
- **Parallel codegen**: multiple patches, multiple bounded contexts, all in parallel
- **LLM routing**: assigns the right model per task (planning, patch generation, small refactors)
- **RAG**: available as read‑only context (architecture docs, ADRs, past diffs)
  - All RAG queries and returned docs are logged
- All output is patches, not full‑file rewrites

### Stage 6: Green Phase
- `moon run :fmt` → `moon run :clippy` → `moon run :test` → property tests (where configured)
- Failures route back to implement with targeted constraints
- All retries logged and visible in the workflow UI

### Stage 7: Review Phase
- AI code review over the diff: contract violations, DDD leakage, safety, style
- Structured summary and inline comments generated
- Issues route back to implement/green with comments as constraints

### Stage 8: Merge Queue
- PR opened with spec, ATDD, planner rationale, and review summary attached
- Merge queue re‑tests and serializes to main

---

## Validation Techniques

### Layered Validation Stack

| Layer | Tool | Purpose |
|-------|------|---------|
| **Compiler + Lints** | `moon run :build`, `moon run :fmt`, `moon run :clippy` | Baseline correctness |
| **ATDD Tests** | Hidden from AI | Acceptance criteria validation |
| **Unit + Integration** | `moon run :test` | Functional correctness |
| **Property Tests** | proptest | Invariant validation |
| **Mutation Tests** | cargo-mutants | Test suite quality (periodic) |
| **AI Review** | Codex reviewer | Contract/style/leakage checks |
| **Merge Queue** | Full re-run | Integration safety |

### Property-Based Testing

Property tests (proptest/quickcheck) validate AI‑written code against deep invariants, not just examples.

**Where they shine:**
- Core domain logic: pricing rules, unit conversions, parsers, state machines
- Algebraic properties: associativity, commutativity, idempotence, error behavior
- Serialization, encoding, and boundary behavior

**Conservative integration:**
- Define properties manually for important modules; let AI generate implementation, not the property
- For AI‑touched code in those modules, run proptest as part of the green gate
- Treat a failing property test as a hard stop or "retry with constraints" trigger

**Caveats:**
- Property tests are randomized; they increase confidence, not proof
- They add runtime to CI, so scope to critical code paths or run heavier campaigns on nightly jobs

### Mutation Testing

Mutation testing checks if tests are actually strong: mutate code; if tests still pass, they're weak.

**Where it fits:**
- Periodic quality audits of modules AI frequently edits
- Ensuring that ATDD + unit + property tests actually kill mutants in AI‑touched code

**Conservative integration:**
- **Not on every CI run** (too heavy)
- Use:
  - Scheduled jobs (nightly/weekly) on important modules
  - A "mutation test lane" you can run on demand after big AI refactors
- Feed mutation results back:
  - If mutants survive, mark tests as weak
  - Prefer adding tests before letting AI touch that area again

**Caveats:**
- Mutation testing tools for Rust are still maturing
- Must avoid running blindly on entire repo—pick specific targets

---

## Risk Tiering

Not all changes warrant the same level of scrutiny. The lane applies heavier checks only to higher‑risk changes.

### Tier 1: Low Risk

**Characteristics:**
- Small, isolated changes
- No core domain logic
- No security implications
- Single bounded context

**Gates:**
- `moon run :quick` (fmt + clippy + unit tests)
- ATDD tests
- AI code review
- Merge queue

**Models:** Fast, cheaper models for codegen

### Tier 2: Medium Risk

**Characteristics:**
- Multiple files or bounded contexts
- Core domain logic touched
- API changes
- Potential performance impact

**Gates:**
- Everything in Tier 1
- Property tests (if configured for touched modules)
- Integration tests
- Stricter clippy lints

**Models:** Mid-tier models for planning and review; flagship for complex refactors

### Tier 3: High Risk

**Characteristics:**
- Cross-cutting changes
- Security-sensitive code
- Core domain invariants
- Performance-critical paths
- Large scope (many files)

**Gates:**
- Everything in Tier 2
- Property tests mandatory
- Flagship model for implementation and review
- Human review escalation
- Post-merge monitoring period

**Models:** Flagship models throughout

### Risk Determination

The planner app determines risk tier based on:
- Scope breadth (files, modules, bounded contexts)
- Domain sensitivity (core logic, invariants, security)
- Change type (new feature vs. refactor vs. fix)
- Historical patterns (has this area been problematic?)

---

## Workflow UI

The pipeline is not just a headless CI job. It is **visually orchestrated** in a workflow UI that mirrors step functions and tools like n8n.

### Visual Design

- Each stage is a **visible node** in the workflow graph
- Edges represent data flow: spec, context pack, contracts, diffs, test results, review output
- Each node has:
  - Configurable inputs and parameters (model, token budget, retry policy, risk tier)
  - Live status (running, passed, failed, retrying)
  - Logs and artifacts attached per run

### Failure Routing

- Retries and failure routing are visible as explicit branches, not hidden logic
- Parallelism is visible: parallel codegen, multi‑context runs, concurrent test phases show as parallel branches

### Operational Interface

The UI enforces conservative rigor by making the entire process **transparent, auditable, and configurable** without touching code. It serves as the primary operational interface for teams using the lane.

---

## RAG Integration

RAG is powerful if it's just another tool, not a hidden brain.

### Read-Only Context

Use RAG as read‑only context, not implicit memory:

**Index:**
- Repo docs (architecture, READMEs, ADRs)
- Coding standards, DDD rules, style guides
- Past accepted planner outputs and diffs

**At planning or contract phase:**
- AI queries that index to refine understanding of:
  - Which bounded context to use
  - Preferred patterns (e.g., how new endpoints are usually wired)
  - Edge cases or domain rules

### Explicit Logging

- Log both queries and documents used
- Behavior is auditable and replayable

### No Bypass

Even if RAG gives excellent domain hints, gates remain:
- Types, ATDD, tests, clippy, AI review
- RAG improves plan quality, not trust level

---

## Flagship Models for Structured Refactors

Using strongest‑available models for certain steps is smart if you control what they're allowed to change.

### Good Uses

- **Structured refactors**: Extract function, split files, rename APIs, migrate from pattern A to B
- **Test amplification**: Improve or extend tests around AI‑touched code
- **Contract enforcement**: Rewriting code to better match DDD and style constraints, under LoC limits

### Constraints

These models operate inside the same lane:
1. Planner decides "refactor step needed"
2. High‑end model generates a patch, not arbitrary code
3. Full validation: format → clippy → tests → property tests (if present) → AI review

### When to Use

- Context is large/complex but still bounded
- Cross‑function refactors requiring maximum reasoning quality

---

## LLM Routing

Massive parallelism is a first‑class design goal. LLM routing assigns the right model to the right task.

### Model Categories

| Category | Use Cases | Characteristics |
|----------|-----------|-----------------|
| **Fast** | Simple patches, small refactors, test generation | Low cost, high throughput |
| **Mid** | Planning, standard implementation, review | Balanced cost/quality |
| **Flagship** | Complex refactors, high-risk changes, contract enforcement | Maximum reasoning, highest cost |

### Routing Logic

The planner determines model assignment based on:
- Risk tier of the change
- Complexity of the task (files, functions, bounded contexts)
- Historical success rates for similar tasks
- Token efficiency requirements

---

## ATDD Workflow: Two-Phase Development

The lane separates concerns into two distinct phases, each handled by a specialized agent role.

### Phase 1: AcceptanceTest (TEST_AGENT)

**Goal:** Write failing acceptance tests that specify desired behavior

**Activities:**
- Identify public API surface
- Write test cases for all scenarios
- Ensure tests compile and FAIL (red state)
- Document expected behavior in test assertions
- DO NOT write implementation code

**Output:** Red test suite that defines acceptance criteria

**Red Gate Criteria:**
- Tests must compile successfully
- All new tests must fail
- Failure messages must be clear and actionable
- No implementation code exists

### Phase 2: Implementation (LOGIC_AGENT)

**Goal:** Make all red tests pass with minimal, functional code

**Activities:**
- Read and understand failing tests
- Write ONLY production code needed to pass tests
- Use Result<T,E> and functional patterns (no unwrap/panic)
- Ensure all tests turn GREEN
- DO NOT modify or add tests

**Output:** Green test suite with production implementation

**Green Gate Criteria:**
- 100% test pass rate
- No clippy warnings
- Code follows functional Rust guidelines

---

## Functional Rust Constraints

All AI-generated code must follow the functional Rust guidelines in `FUNCTIONAL_RUST.md`:

### Mandatory Libraries

| Library | Purpose |
|---------|---------|
| `im` | Immutable data structures (O(1) cloning) |
| `tap` | Pipe operator for functional chaining |
| `itertools` | Functional iteration adaptors |
| `strum` | Enum superpowers |
| `thiserror` | Typed errors |

### Safety Constraints

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]
```

### Size Constraints

- Functions: ≤ 40 lines
- Function arguments: ≤ 5
- Modules touched: ≤ 300 lines per run

---

## Command Surface

### Moon (Build/Test/Lint)

```bash
moon run :quick        # fmt + clippy + fast tests
moon run :ci           # full CI pipeline
moon run :test         # all tests
moon run :fmt-fix      # auto-format
moon run :build        # compile
moon run :check        # type check
moon run :coverage     # test coverage
moon run :mutants-quick # quick mutation testing
```

**NEVER use cargo directly.**

### Beads (Issue Tracking)

```bash
br ready                           # Find actionable work
br show <id>                       # View full details
br update <id> --status in_progress # Claim work
br close <id> --reason "..."       # Close with reason
br sync --flush-only               # Export to JSONL
```

### jj (Workspace Isolation)

```bash
jj workspace add <workspace>       # Create workspace
jj git fetch                       # Sync remote refs
jj rebase                          # Rebase onto latest main
jj bookmark create <name>          # Create bookmark for landing
jj workspace forget <workspace>    # Cleanup workspace
```

### Codanna (Code Discovery)

```bash
codanna_search_symbols             # Find symbols
codanna_find_symbol                # Get symbol details
codanna_get_calls                  # What a function calls
codanna_find_callers               # What calls a function
codanna_analyze_impact             # Full dependency graph
```

---

## Complete Bead Workflow

```bash
# 1. Find work
br ready

# 2. Create workspace
jj workspace add <workspace>

# 3. Claim work
br update <id> --status in_progress

# 4. Run the lane
oya run --bead <id>

# 5. Validate
moon run :ci

# 6. Complete
jj git fetch
jj rebase
jj bookmark create <name>
br close <id>
br sync --flush-only
```

---

## Landing Checklist

Before any change lands:

1. `moon run :ci` passes
2. `jj git fetch && jj rebase` completes
3. `jj bookmark create <name>` prepared for merge
4. `br close <id>` closes the bead
5. `br sync --flush-only` exports state
6. `git add .beads/` and commit

---

## Non-Goals (Current)

- UI/frontend productization beyond the planner companion
- Replacing or removing jj
- Speculative multi-framework support
- UX polish ahead of governance correctness
- "Rewrite whole apps" as a single action (emergent property of many safe runs, not a single action)

---

## References

- `docs/FUNCTIONAL_RUST.md` - Functional Rust guidelines
- `docs/BEADS.md` - Issue tracking reference
- `.moon/tasks.yml` - Moon task definitions
- `ARCHITECTURE_MASTER_PLAN.md` - Master architecture
- `docs/UBIQUITOUS_LANGUAGE.md` - Domain language
