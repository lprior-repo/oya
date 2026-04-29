# Implementation Plan & Contract Specification: Oya Core & Frontend Boundary

## Context
- **Feature**: Port `oya-lite` features to Oya core and establish clear Core/Frontend architectural boundaries.
- **Domain terms**:
  - `StateDb`: Fjall KV store for persistence (`workflows` + `journal` keyspaces).
  - `Batch Write Helpers`: Utilities for atomic writes to Fjall.
  - `Atomic Sequence Keys`: Monotonically increasing keys for event journals.
  - `Opencode Server Adapter`: HTTP adapter for interacting with OpenCode server.
  - `opencode_output_is_error`: Sanitized error predicate to detect OpenCode failures without leaking secrets.
  - `Oya Core`: The backend engine responsible for decision making, verification, repair, local Fjall persistence, and Restate orchestration.
  - `Oya Frontend`: The UI layer encompassing visualization, read-only Admin dashboards, and the Dioxus WASM workflow editor.
  - `oya-contracts`: Shared crate containing pure DTOs.
  - **DTOs**: `LifecycleRequest`, `LifecycleStatusSnapshot`, `MemorySnapshot`.
  - **Canonical Ports**: Ingress `909`, Admin `9070`, Service `9180`.
- **Assumptions**:
  - `fjall` is used exclusively as the local append-only event source and state persistence in Core.
  - The Core and Frontend are structurally decoupled but communicate via `oya-contracts` DTOs and Restate endpoints.
- **Open questions**:
  - Will the Dioxus frontend be served by Oya Core's HTTP server, or will it remain a completely standalone artifact deployed separately?

## Preconditions
- [ ] `oya-contracts` crate must have zero dependencies on backend-specific libraries (e.g., `fjall`, `restate-sdk`, `tokio::fs`) to ensure WASM compatibility for the frontend.
- [ ] `StateDb` must successfully acquire an exclusive lock on the Fjall data directory (`.oya-lite` or `.oya`) before accepting batch writes.
- [ ] OpenCode server adapter must be instantiated with valid basic auth and endpoint configuration provided by Oya Core.

## Postconditions
- [ ] `StateDb` batch writes guarantee atomic application to both `workflows` and `journal` keyspaces simultaneously.
- [ ] Any OpenCode server errors are uniformly sanitized via `opencode_output_is_error`.
- [ ] Frontend network clients default to the canonical ports: Ingress `909`, Admin `9070`, Service `9180`.
- [ ] All shared state structures (`LifecycleRequest`, `LifecycleStatusSnapshot`, etc.) are consumed exclusively from `oya-contracts`.

## Invariants
- [ ] **Strict Segregation**: The Frontend cannot directly mutate `StateDb`; it only accesses Core via the `oya-contracts` API models or Restate endpoints.
- [ ] **Monotonicity**: Event journal keys must be strictly monotonically increasing (atomic sequence keys).
- [ ] **Secret Hygiene**: Raw OpenCode stack traces or secrets are NEVER leaked to the frontend or stored raw in workflow journals.
- [ ] **Git-Only VCS Doctrine**: Oya SHALL use plain Git for all branch and PR flow. Non-Git workspace systems are out of scope for runtime, CLI, docs, tests, and delivery plans.

## Error Taxonomy
- `Error::PersistenceError(String)` - when a Fjall batch write fails or disk is full.
- `Error::OpenCodeAdapterError(String)` - when the OpenCode server responds with a sanitized error.
- `Error::SequenceGenerationError` - when atomic sequence key generation fails.

## Contract Signatures
```rust
// Core / Shared
pub fn opencode_output_is_error(output: &str) -> bool;

// Persistence
pub fn execute_batch_write(&self, batch: fjall::Batch) -> Result<(), Error>;
pub fn generate_journal_key(&self, bead_id: &str) -> Result<String, Error>;

// oya-contracts
#[derive(Serialize, Deserialize)]
pub struct LifecycleStatusSnapshot { /* fields */ }
```

## Vertical Slice Delivery Strategy

Oya must be built in thin, demoable vertical slices. Horizontal workstreams
(`evidence`, `gates`, `repair`, `frontend`, `docs`) are implementation details
inside each slice, not separate delivery phases. A slice is complete only when it
has a user-visible command or UI path, persisted evidence, and a verification
gate.

### Slice 0 — Git-Only VCS Baseline
- **User-visible path**: `oya doctor` and repository proof checks.
- **Includes**: removal of non-Git VCS command assumptions from active docs, CLI
  help, lifecycle code, tests, and health checks.
- **Acceptance demo**: Oya builds, tests, and runs with only Git installed; active
  product docs and task plans describe Git-only branch/PR flow.

### Slice 1 — Baseline Oya Health
- **User-visible path**: `oya doctor` and `moon run :ci`.
- **Includes**: cleaned Moon config, canonical ports, health checks, green CI.
- **Acceptance demo**: running `oya doctor` shows Moon/ReState/OpenCode/Fjall
  readiness; `moon run :ci` passes.

### Slice 2 — Evidence-First Run Skeleton
- **User-visible path**: `oya run --bead-id demo --prompt "noop"`.
- **Includes**: run id creation, bead validation, evidence-before-action,
  `RunStarted` + `PromptRecord`, JSON progress, `oya status --run-id`.
- **Acceptance demo**: command exits with a blocked/dry-run style status without
  invoking OpenCode, and `oya evidence check` proves the record chain.

### Slice 3 — Moon Verification Loop
- **User-visible path**: `oya verify --bead-id demo`.
- **Includes**: gate runner, typed Moon failure mapping, bounded stdout/stderr,
  `GateRunStarted`, `GateRunFinished`, `Finding`, and `oya explain`.
- **Acceptance demo**: a real Moon task failure becomes a typed finding instead
  of a generic command error.

### Slice 4 — OpenCode Adapter With Safe Failure
- **User-visible path**: `oya run --bead-id demo --prompt "..." --model bad/model`.
- **Includes**: subprocess/server dual-mode adapter, persisted `AgentRequest`,
  persisted `AgentRun`, sanitized OpenCode failure, no raw stack traces/secrets.
- **Acceptance demo**: invalid model returns a typed sanitized failure and the
  evidence chain remains valid.

### Slice 5 — One-Gate Repair
- **User-visible path**: `oya verify --bead-id demo --repair`.
- **Includes**: repair budgets, bounded repair prompt, category-specific mutation
  scope, repair attempt record, rerun of failed gate plus previously passed gates.
- **Acceptance demo**: a small format/lint failure is repaired and reverified, or
  blocks with `RepairBudgetExhausted` after budget exhaustion.

### Slice 6 — Full Single-Bead Factory Loop
- **User-visible path**: `oya run --bead-id demo-fix --prompt "make failing test pass"`.
- **Includes**: OpenCode attempt, Moon verification, repair loop, final verdict,
  generated report from evidence only.
- **Acceptance demo**: one real bead goes from prompt to green verification with
  `oya report --run-id` showing the proof trail.

### Slice 7 — Restate + Frontend Read-Only Console
- **User-visible path**: `oya serve` plus Oya frontend lifecycle panel.
- **Includes**: shared `oya-contracts` DTOs, canonical ports 909/9070/9180,
  lifecycle status polling, read-only evidence/status display.
- **Acceptance demo**: frontend shows the Slice 6 run status and evidence-derived
  result without duplicating backend workflow definitions.

### Slice 8 — Workspace/Git/PR Flow
- **User-visible path**: successful run creates bookmark/branch and PR after green
  verification.
- **Includes**: workspace ownership, dirty-worktree blocking, Git branch lifecycle,
  non-empty diff validation, branch push, PR creation evidence.
- **Acceptance demo**: green single-bead run opens a PR using Git only; dirty
  workspace blocks before any mutation.

### Slice 9 — Hardening Ratchet
- **User-visible path**: adversarial negative E2E suite and mutation/security gates.
- **Includes**: concurrency invariant, corruption detection, output-limit tests,
  mutation testing policy, security allow-list documentation.
- **Acceptance demo**: concurrent same-bead run returns `BeadAlreadyRunning`,
  evidence corruption blocks repair, and security warnings are intentionally
  documented or removed.

### Slice Rule

No bead should be accepted if it only builds a horizontal subsystem with no
working vertical path. Every bead must declare the earliest slice it advances and
the command/UI demo that proves it.

## Non-goals
- [ ] Implementing the full Dioxus WASM workflow editor functionality in this immediate PR (this plan only defines the boundary, ports, and DTO extraction).
- [ ] Replacing Restate with Fjall (Fjall provides local state persistence; Restate provides durable workflow orchestration).
