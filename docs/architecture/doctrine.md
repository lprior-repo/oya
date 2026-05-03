# Oya Architectural Doctrine

> **Storm Goddess. Transformer. Gatekeeper. The feminine force that takes what it needs.**

This document defines the unified structural blueprint for Oya, a Restate-backed visualizer plus OpenCode runtime plus strict Rust quality gate. Restate is the main orchestrator for durable lifecycle execution, retries, and service boundaries; Oya uses it to automate bead-based task execution with AI-driven code generation, live trace inspection, and Moon-enforced Rust verification gates.

First-class architecture decisions are tracked in `docs/adr/`.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [The Nine Streams](#the-nine-streams)
3. [Module Structure](#module-structure)
4. [Core Architectural Patterns](#core-architectural-patterns)
5. [Restate Integration](#restate-integration)
6. [State Machine](#state-machine)
7. [Pipeline Stages](#pipeline-stages)
8. [Quality Gates](#quality-gates)
9. [Dependencies](#dependencies)
10. [Deployment Architecture](#deployment-architecture)

---

## Architecture Overview

Oya is a **Restate-backed durable visual OpenCode runtime** that:

- Picks ready beads from Steve Yegge's beads system
- Creates isolated Git branches or Git worktrees for each bead when isolation is required
- Runs AI execution via OpenCode CLI
- Visualizes lifecycle state and live OpenCode traces in the Dioxus frontend
- Executes quality gates via Moon
- Manages PR creation and merging
- Handles retries, failures, and state transitions

### Key Characteristics

- **Durable Execution**: All state transitions are persisted by Restate
- **Restate-Orchestrated**: Restate is the authoritative runtime for lifecycle execution, retries, and handler boundaries
- **Functional Core**: Pure functions with effect interpreters
- **Zero Panics**: `#![deny(clippy::unwrap_used)]` enforced
- **Test-First**: TDD workflow with sealed acceptance tests
- **Gate-Driven**: Quality gates block advancement at each stage
- **Visual Runtime**: Frontend visibility is a core correctness surface for vibe-coded Rust

---

## The Nine Streams

Oya's architecture reflects the nine aspects of Oya (Yoruba storm goddess):

```
┌─────────────────────────────────────────────────────────────┐
│                     THE NINE STREAMS                        │
└─────────────────────────────────────────────────────────────┘

1. CLI Layer (The Mouth)
   └── oya-cli (src/cli/)
       └── Command dispatch: init, doctor, lifecycle, status, cancel

2. Type System (The Heart)
   └── oya-core (src/lifecycle/types/)
       └── Bead, Lifecycle, Error, Model, PR, Repo, Timeout, Workspace

3. Workflow Engine (The Pulse)
   └── lifecycle/workflow/ (src/lifecycle/workflow/)
       └── DAG execution, step runner, transitions, progress

4. Effect System (The Rhythm)
   └── lifecycle/effects/ (src/lifecycle/effects/)
       └── Pure effect generation, effect interpreter

5. Restate Runtime (The Pulse)
   └── restate-adapter (src/restate_oya/)
       └── Handlers, OpenCode integration, tracing

6. Telemetry Layer (The Witness)
   └── lifecycle/telemetry (src/lifecycle/telemetry.rs)
       └── OpenTelemetry traces, metrics, logs

7. State Store (The Memory)
   └── fjall-store (Fjall embedded DB)
       └── Durable run state, evidence, artifacts

8. Quality Gates (The Judgment)
   └── gate-runner (Moon tasks)
       └── fmt, clippy, test, build, coverage, mutants

9. Workspace Plane (The Hands)
   └── workspace-plane (Git/GitHub integration)
       └── Branch isolation, rebase, PR management
```

---

## Module Structure

```
src/
├── main.rs                    # Entry point with tokio runtime
├── lib.rs                     # Library root (lifecycle + restate_oya)
│
├── cli/                       # Command-line interface
│   ├── mod.rs
│   ├── args.rs               # Clap argument definitions
│   ├── commands.rs           # Command dispatch
│   ├── doctor.rs             # Runtime validation checks
│   ├── doctor/
│   │   ├── commands.rs       # Doctor subcommands
│   │   └── repo.rs           # Repository checks
│   ├── init.rs               # Runtime bootstrap
│   ├── repo.rs               # Repository operations
│   └── restate.rs            # Restate CLI wrapper
│
├── lifecycle/                # Core lifecycle orchestration
│   ├── mod.rs
│   ├── effects.rs            # Effect generation (pure)
│   ├── effects/
│   │   └── run.rs            # Run execution effects
│   ├── telemetry.rs          # OpenTelemetry integration
│   ├── transitions.rs        # State transitions
│   │
│   ├── types/                # Domain types
│   │   ├── mod.rs
│   │   ├── bead.rs           # Bead value object
│   │   ├── error.rs          # Error taxonomy
│   │   ├── lifecycle.rs      # Lifecycle states
│   │   ├── model.rs          # Model selection
│   │   ├── pr.rs             # PR metadata
│   │   ├── repo.rs           # Repository info
│   │   ├── timeout.rs        # Timeout configuration
│   │   └── workspace.rs      # Workspace context
│   │
│   └── workflow/             # Workflow execution engine
│       ├── mod.rs
│       ├── dag.rs            # Directed acyclic graph
│       ├── execution.rs      # Execution orchestrator
│       ├── execution/
│       │   ├── details.rs    # Step details
│       │   ├── resolve.rs    # Conflict resolution
│       │   ├── step_runner.rs # Step execution
│       │   └── transitions.rs # Execution transitions
│       ├── finalize.rs       # Finalization logic
│       ├── progress.rs       # Progress tracking
│       ├── steps.rs          # Step definitions
│       ├── tests.rs          # Workflow tests
│       └── types.rs          # Workflow types
│
└── restate_oya/              # Restate service layer
    ├── mod.rs
    ├── handlers.rs           # Service handlers (Oya, OyaMemory, OyaService)
    ├── handlers_tests.rs     # Handler tests
    ├── opencode.rs           # OpenCode CLI adapter
    ├── trace.rs              # Distributed tracing
    └── types.rs              # Restate-specific types
```

### Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `cli/` | Parse args, dispatch commands, user-facing output |
| `lifecycle/types/` | Domain model: beads, errors, states, configs |
| `lifecycle/workflow/` | DAG execution, step orchestration, state machine |
| `lifecycle/effects/` | Pure effect generation (no I/O) |
| `lifecycle/telemetry/` | Traces, metrics, logs via OpenTelemetry |
| `restate_oya/` | Restate handlers, durable execution, retries |

---

## Core Architectural Patterns

### 1. Functional Core / Imperative Shell

```
┌─────────────────────────────────────────────────────────┐
│                  IMPERATIVE SHELL                        │
│  (Effect Interpreter - performs I/O)                    │
│                                                          │
│  ┌──────────────┐   ┌──────────────┐   ┌─────────────┐ │
│  │  Restate     │   │  OpenCode    │   │  Moon       │ │
│  │  Handler     │   │  CLI         │   │  Runner     │ │
│  └──────┬───────┘   └──────┬───────┘   └─────┬───────┘ │
│         │                  │                  │         │
│         └──────────────────┴──────────────────┘         │
│                            │                             │
│                            ▼                             │
└─────────────────────────────────────────────────────────┘
                             │
                             │ Vector<Effect>
                             │
                             ▼
┌─────────────────────────────────────────────────────────┐
│                   FUNCTIONAL CORE                        │
│  (Pure Functions - no I/O, returns Effects)             │
│                                                          │
│  ┌─────────────────────────────────────────────────┐   │
│  │  workflow::execute_stage()                       │   │
│  │  effects::generate_effects()                     │   │
│  │  transitions::apply_transition()                 │   │
│  └─────────────────────────────────────────────────┘   │
│                                                          │
│  Returns: Vector<Effect>                                │
│  - CreateWorkspace                                      │
│  - RunOpenCode                                          │
│  - RunMoonGate                                          │
│  - CreatePR                                             │
│  - MergeBranch                                          │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Pattern Rules:**

- **Core**: Pure functions, no I/O, return `Vector<Effect>`
- **Shell**: Effect interpreters, perform I/O, async allowed
- **Data**: Use `im::Vector`, `im::HashMap` (O(1) clone)
- **Flow**: Use `tap::Pipe` for chaining, never intermediate variables

### 2. Effect as Data

```rust
pub enum Effect {
    CreateWorkspace { bead_id: String },
    RunOpenCode { prompt: String, session: String },
    RunMoonGate { task: String },
    CreatePR { title: String, body: String },
    MergeBranch { branch: String },
    Log { level: LogLevel, msg: String },
}

// Pure function - returns instructions
fn execute_stage(bead: Bead, stage: Stage) -> Vector<Effect> {
    Vector::from(vec![
        Effect::Log { level: Info, msg: format!("Starting {}", stage) },
        Effect::RunOpenCode { prompt: stage.prompt(), session: bead.id },
        Effect::RunMoonGate { task: ":ci" },
    ])
}
```

### 3. Railway-Oriented Programming

```rust
pub fn process_bead(bead: Bead) -> Result<Vector<Effect>, BeadError> {
    bead
        .pipe(validate_bead)?
        .pipe(check_dependencies)?
        .pipe(prepare_workspace)?
        .pipe(execute_pipeline)?
        .pipe(Ok)
}
```

---

## Restate Integration

### Service Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    RESTATE RUNTIME                       │
│  (Durable Execution Engine)                             │
└─────────────────────────────────────────────────────────┘
         │
         │ Ingress API (http://127.0.0.1:8080)
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│                   OYA SERVICES                           │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  1. Oya Workflow (Virtual Object)                       │
│     - run()              → Start bead lifecycle         │
│     - get_lifecycle()    → Query current state          │
│     - cancel()           → Cancel execution             │
│                                                          │
│  2. OyaMemory (Virtual Object)                          │
│     - start()            → Initialize memory            │
│     - run_pipeline()     → Execute pipeline stages      │
│                                                          │
│  3. OyaService (Service)                                 │
│     - get_lifecycle()    → Query status                 │
│     - cancel()           → Cancel workflow              │
│                                                          │
└─────────────────────────────────────────────────────────┘
         │
         │ Handler Service (http://127.0.0.1:9180)
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│                   OYA HANDLERS                           │
│  (src/restate_oya/handlers.rs)                          │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  - Durable state management                             │
│  - Automatic retries on failure                         │
│  - Sleep/awake without blocking threads                 │
│  - Invocation tracking                                  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Handler Implementation

```rust
#[restate_sdk::object]
pub struct Oya;

impl Oya {
    #[handler]
    pub async fn run(&self, ctx: ObjectContext, req: RunRequest) -> Result<RunResponse, Error> {
        // Durable execution - state persisted automatically
        let bead_id = &req.bead_id;
        
        // Initialize lifecycle
        ctx.run(|_| {
            let lifecycle = Lifecycle::new(bead_id);
            Ok(lifecycle)
        }).await?;
        
        // Execute stages with durability
        for stage in Stage::all() {
            ctx.run(|ctx| self.execute_stage(ctx, stage)).await?;
        }
        
        Ok(RunResponse { status: "shipped".into() })
    }
}
```

### Endpoints

| Endpoint | Purpose |
|----------|---------|
| `http://127.0.0.1:8080/Oya/<key>/run` | Start workflow |
| `http://127.0.0.1:8080/OyaService/get_lifecycle` | Query status |
| `http://127.0.0.1:8080/OyaService/cancel` | Cancel workflow |
| `http://127.0.0.1:9070` | Restate Admin UI |

---

## State Machine

### Lifecycle States

```
┌──────────┐
│ PENDING  │  Bead submitted, waiting to start
└────┬─────┘
     │ start
     ▼
┌──────────┐
│ RUNNING  │  Currently executing a stage
└────┬─────┘
     │ stage_complete
     ▼
┌──────────┐
│ WAITING  │  Between stages, validating
└────┬─────┘
     │
     ├─ gate_passed ──→ RUNNING (next stage)
     │
     ├─ gate_failed ──→ RUNNING (retry)
     │
     ├─ max_retries ──→ BLOCKED
     │
     └─ final_gate ──→ SHIPPED

┌──────────┐
│ BLOCKED  │  Max retries exceeded, needs intervention
└──────────┘

┌──────────┐
│ SHIPPED  │  Merged to main, all gates passed
└──────────┘

┌──────────┐
│ FAILED   │  Unrecoverable error
└──────────┘
```

### State Transitions

| From | To | Trigger |
|------|----|---------| 
| Pending | Running | `oya lifecycle --bead <id>` |
| Running | Waiting | Stage execution complete |
| Waiting | Running | Gate passed, next stage |
| Waiting | Running | Gate failed, retry available |
| Waiting | Blocked | Max retries exceeded |
| Waiting | Shipped | ShipGate passed + merge complete |
| Any | Failed | Unrecoverable error |

---

## Pipeline Stages

### Stage Flow

```
┌──────────────────────────────────────────────────────────┐
│                    PIPELINE FLOW                          │
└──────────────────────────────────────────────────────────┘

┌───────────┐
│ EXPLORE   │  Research codebase, gather context
│ (1 min)   │  - Read relevant files
└─────┬─────┘  - Understand dependencies
      │
      ▼
┌───────────┐
│ CONTRACT  │  Generate specification
│ (2 min)   │  - Create contract-spec.md
└─────┬─────┘  - Define preconditions/postconditions
      │
      ▼
┌───────────┐
│ RED       │  Write failing tests
│ (5 min)   │  - Create acceptance tests
└─────┬─────┘  - Seal tests (no modifications after)
      │
      ▼
┌───────────┐
│ GREEN     │  Make tests pass
│ (10 min)  │  - Implement production code
└─────┬─────┘  - All tests must pass
      │
      ▼
┌───────────┐
│ WITNESS   │  Hidden scenario validation
│ (3 min)   │  - Run holdout scenarios
└─────┬─────┘  - Verify edge cases
      │
      ▼
┌───────────┐
│ SHIPGATE  │  Final validation + merge
│ (2 min)   │  - All quality gates pass
└─────┬─────┘  - Create PR
      │        - Merge to main
      │
      ▼
┌───────────┐
│  SHIPPED  │  Complete
└───────────┘
```

### Stage Characteristics

| Stage | Duration | Model | Gates | Output |
|-------|----------|-------|-------|--------|
| Explore | 1 min | Fast | None | Context |
| Contract | 2 min | Fast | Compiles | contract-spec.md |
| Red | 5 min | Balanced | Compiles | Failing tests |
| Green | 10 min | Balanced | Tests pass | Implementation |
| Witness | 3 min | Capable | Holdouts pass | Evidence |
| ShipGate | 2 min | Best | All gates + merge | Shipped |

### Retry Logic

```
Stage execution
     │
     ▼
Attempt 1
     │
     ├─ gate_passed ──→ Next Stage
     │
     └─ gate_failed
           │
           ▼
       Attempt 2 (adjusted context)
           │
           ├─ gate_passed ──→ Next Stage
           │
           └─ gate_failed
                 │
                 ▼
             Attempt 3 (more context)
                 │
                 ├─ gate_passed ──→ Next Stage
                 │
                 └─ gate_failed ──→ BLOCK (max retries)
```

---

## Quality Gates

### Gate Hierarchy

```
┌──────────────────────────────────────────────────────────┐
│                  QUALITY GATES                            │
└──────────────────────────────────────────────────────────┘

┌─────────────────┐
│ FORMAT          │  moon run :fmt
│ (fast)          │  All code formatted correctly
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ LINT            │  moon run :clippy
│ (fast)          │  Zero warnings, deny unwrap/panic
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ COMPILE         │  moon run :check
│ (medium)        │  Zero compilation errors
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ TEST            │  moon run :test
│ (slow)          │  All tests pass
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ COVERAGE        │  moon run :coverage
│ (slow)          │  >= 80% coverage (optional)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ MUTANTS         │  moon run :mutants
│ (very slow)     │  Kill all mutants (optional)
└─────────────────┘
```

### Moon Task Mapping

| Moon Task | Gate | Underlying Check |
|-----------|------|---------|
| `moon run :fmt` | Format | Rust formatting |
| `moon run :clippy` | Lint | Rust lint rules |
| `moon run :check` | Compile | Rust compile check |
| `moon run :test` | Test | Rust test suite |
| `moon run :ci` | CI Pipeline | `fmt → clippy → check → test` |
| `moon run :quick` | Quick Check | `fmt → clippy` |

### Stage → Gate Mapping

| Stage | Required Gates |
|-------|---------------|
| Contract | Compile |
| Red | Compile |
| Green | Compile + Test |
| Witness | Holdout Scenarios |
| ShipGate | CI (fmt + clippy + check + test) + PR created + Merged |

---

## Dependencies

### External Services

```
┌─────────────────────────────────────────────────────────┐
│              EXTERNAL DEPENDENCIES                       │
└─────────────────────────────────────────────────────────┘

┌──────────────────┐
│ Steve Yegge's    │  Task tracking
│ Beads (bd)       │  - bd ready
│                  │  - bd show <id>
│                  │  - bd update <id>
│                  │  - bd close <id>
└──────────────────┘

┌──────────────────┐
│ Restate          │  Durable execution
│ (managed local)  │  - State management
│                  │  - Automatic retries
│                  │  - Invocation tracking
│                  │
│  Ports:          │
│  - 8080 (ingress)│
│  - 9070 (admin)  │
└──────────────────┘

┌──────────────────┐
│ OpenCode CLI     │  AI execution
│                  │  - opencode run
│                  │  - Session management
│                  │  - Event streaming
└──────────────────┘

┌──────────────────┐
│ Git/GitHub       │  Workspace isolation and PR flow
│                  │  - git switch
│                  │  - git worktree add
│                  │  - git rebase
│                  │  - git push
└──────────────────┘

┌──────────────────┐
│ Moon             │  Build system
│                  │  - moon run :ci
│                  │  - moon run :test
│                  │  - Task orchestration
└──────────────────┘

┌──────────────────┐
│ GitHub CLI (gh)  │  Repository operations
│                  │  - gh pr create
│                  │  - gh repo view
└──────────────────┘

┌──────────────────┐
│ Fjall            │  Embedded database
│                  │  - Durable state
│                  │  - Run evidence
└──────────────────┘

┌──────────────────┐
│ OpenTelemetry    │  Observability
│  (Optional)      │  - Traces
│                  │  - Metrics
│                  │  - Logs
│                  │
│  Endpoint:       │
│  localhost:4318  │
└──────────────────┘
```

### Rust Crates

| Crate | Purpose |
|-------|---------|
| `restate-sdk` | Restate service definitions |
| `im` | Immutable data structures (O(1) clone) |
| `tap` | Pipe operator for functional chains |
| `itertools` | Functional iteration adapters |
| `strum` | Enum superpowers |
| `thiserror` | Typed error definitions |
| `tokio` | Async runtime |
| `serde` | Serialization |
| `tracing` | Structured logging |
| `clap` | CLI argument parsing |
| `fjall` | Embedded database |
| `reqwest` | HTTP client |

---

## Deployment Architecture

### Local Development

```
┌─────────────────────────────────────────────────────────┐
│                LOCAL RUNTIME                             │
└─────────────────────────────────────────────────────────┘

┌──────────────────┐       ┌──────────────────┐
│  User            │       │  Managed         │
│  Terminal        │       │  Restate         │
│                  │       │  (fresh state)   │
│  $ oya init      │──────▶│                  │
│  $ oya lifecycle │       │  - Port 8080     │
│  $ oya status    │       │  - Port 9070     │
└──────────────────┘       └──────────────────┘
         │                          │
         │                          │
         ▼                          ▼
┌──────────────────┐       ┌──────────────────┐
│  oya.service     │       │  Restate         │
│  (systemd)       │       │  Ingress         │
│                  │       │                  │
│  Port 9180       │◀──────│  localhost:8080  │
└──────────────────┘       └──────────────────┘
         │
         │ Uses
         ▼
┌──────────────────┐
│  OpenCode CLI    │
│  Git CLI         │
│  Moon            │
│  gh CLI          │
│  bd (beads)      │
└──────────────────┘
```

### Bootstrap Sequence

```
$ oya init

1. Stop user-systemd Restate services
   └─ systemctl --user stop restate.service

2. Start managed Restate (fresh local state)
   └─ restate-server --base-dir .oya-lite/restate-data --auto-provision=true

3. Start oya.service
   └─ systemctl --user restart oya.service
   └─ Wait for http://127.0.0.1:9180/discover

4. Register handlers with Restate
   └─ restate deployments register http://127.0.0.1:9180

5. Verify services
   └─ Check: Oya, OyaMemory, OyaService present

6. Health checks
   └─ http://127.0.0.1:8080/restate/health
```

### Runtime Validation

```
$ oya doctor

Checks:
┌─────────────────────────────────┐
│ Ingress Reachability            │
│ http://127.0.0.1:8080           │
└─────────────────────────────────┘
┌─────────────────────────────────┐
│ Admin UI Reachability           │
│ http://127.0.0.1:9070           │
└─────────────────────────────────┘
┌─────────────────────────────────┐
│ Handler Service Reachability    │
│ http://127.0.0.1:9180/discover  │
└─────────────────────────────────┘
┌─────────────────────────────────┐
│ Restate Services Registered     │
│ - Oya                           │
│ - OyaMemory                     │
│ - OyaService                    │
└─────────────────────────────────┘
┌─────────────────────────────────┐
│ Moon Tasks Present              │
│ - :quick                        │
│ - :ci                           │
│ - :test                         │
└─────────────────────────────────┘
┌─────────────────────────────────┐
│ GitHub Repo Detection           │
│ gh repo view --json nameWithOwner│
└─────────────────────────────────┘
```

---

## Code Quality Standards

### Mandatory Lints

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]
```

### Error Handling

- **Never** use `unwrap()` or `expect()`
- **Always** use `Result<T, E>` with `?` operator
- **Always** define typed errors with `thiserror`
- **Always** propagate errors up the stack

### Function Constraints

- **Max 40 lines** per function (clippy::too_many_lines)
- **Max 5 arguments** per function (clippy::too_many_arguments)
- **Pure core**: No I/O in business logic functions
- **Effect returns**: Return `Vector<Effect>` from pure functions

### Testing Requirements

- **TDD workflow**: Tests written before implementation
- **Sealed tests**: Red stage tests cannot be modified
- **No mocks**: Use real dependencies in tests
- **Coverage**: Aim for >= 80% (enforced in CI)

---

## Observability

### OpenTelemetry Integration

```
┌─────────────────────────────────────────────────────────┐
│              OBSERVABILITY STACK                         │
└─────────────────────────────────────────────────────────┘

Oya Service
     │
     │ OTLP (gRPC/HTTP)
     │
     ▼
┌──────────────────┐
│ OpenTelemetry    │
│ Collector        │
│                  │
│ Port 4317 (gRPC) │
│ Port 4318 (HTTP) │
└────────┬─────────┘
         │
         │
         ▼
┌──────────────────┐
│ OpenObserve      │
│ (UI)             │
│                  │
│ Port 5080        │
└──────────────────┘

Exported:
- Traces (workflow execution)
- Metrics (stage durations, success rates)
- Logs (structured events)
```

### Instrumentation Points

| Component | Instrumentation |
|-----------|----------------|
| Workflow execution | Span per stage |
| OpenCode calls | Span with events |
| Moon gates | Span with exit codes |
| State transitions | Events with state changes |
| Error handling | Error events with stack traces |

---

## Security Considerations

### Code Safety

- **Forbid unsafe code**: `#![forbid(unsafe_code)]`
- **Checked arithmetic**: No unchecked integer operations
- **Input validation**: All external inputs validated
- **Error messages**: No secrets in error output

### Dependency Security

- **Audit**: `cargo audit` in CI
- **Pin versions**: Exact versions in Cargo.lock
- **Update strategy**: Regular dependency updates

### Runtime Security

- **Local-only**: Services bind to 127.0.0.1 only
- **No authentication**: Local development only (not production)
- **Managed process**: Restate runs as a local managed subprocess bound to `127.0.0.1`

---

## Performance Characteristics

### Restate Durability

- **State persistence**: Automatic on every handler call
- **Retry overhead**: Minimal (deterministic replay)
- **Sleep efficiency**: Non-blocking, no thread pool

### Immutable Data Structures

- **Clone cost**: O(1) for `im::Vector` and `im::HashMap`
- **Memory efficiency**: Structural sharing
- **Concurrency**: Lock-free reads

### Pipeline Throughput

- **Single bead**: ~23 minutes (sum of stage durations)
- **Parallel beads**: 100 concurrent (Restate handles concurrency)
- **Retry overhead**: +30% per retry (additional context gathering)

---

## Extension Points

### Adding New Stages

1. Define stage in `lifecycle/types/lifecycle.rs`
2. Add effect generator in `lifecycle/effects.rs`
3. Create handler in `restate_oya/handlers.rs`
4. Update workflow DAG in `lifecycle/workflow/dag.rs`
5. Add quality gates to Moon tasks
6. Update documentation

### Adding New Effects

1. Add variant to `Effect` enum
2. Create pure generator function in `lifecycle/effects/`
3. Add interpreter in shell layer
4. Write tests for effect generation

### Adding New Gates

1. Define Moon task in `.moon/tasks.yml`
2. Add to CI pipeline
3. Update stage gate mapping
4. Document in quality gates section

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Restate not reachable | Managed Restate stopped | `oya init` |
| Service not registered | Handler not started | `oya init` |
| Bead blocked | Max retries exceeded | Check logs, fix issue, retry |
| Tests failing | Implementation incomplete | Continue green stage |
| Merge conflict | Main diverged | `git fetch origin && git rebase origin/main` |

### Diagnostic Commands

```bash
# Check Restate services
restate services list

# Check deployments
restate deployments list

# View invocations
restate sql "SELECT * FROM sys_invocation ORDER BY modified_at DESC LIMIT 10"

# Check service health
curl http://127.0.0.1:8080/restate/health

# View logs
restate logs tail
```

---

## References

### External Documentation

- [Restate Documentation](https://docs.restate.dev/)
- [Functional Rust Guide](./FUNCTIONAL_RUST.md)
- [Ubiquitous Language](./UBIQUITOUS_LANGUAGE.md)
- [Beads System](./BEADS.md)
- [Quality Gates](./QUALITY_GATES.md)

### Internal References

- AGENTS.md - Development workflow
- WHY_OYA.md - Philosophy and naming
- contract-spec.md - Design contracts
- martin-fowler-tests.md - Test specifications

---

## Changelog

### Version 1.0 (2026-03-07)

- Initial architectural doctrine documentation
- Unified structural blueprint
- The Nine Streams architecture
- Restate integration patterns
- Pipeline stage definitions
- Quality gate hierarchy
- Deployment architecture

---

**Ẹpa OYA. The storm is here.**
