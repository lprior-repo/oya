# Restate Migration: Code Savings Analysis

**Date**: 2026-02-14
**Total codebase**: ~141,440 lines of Rust

## Overview

Restate is a durable execution engine providing retries, idempotency, state
management, workflow orchestration, and messaging as built-in runtime primitives.
This analysis quantifies what it would replace in the OYA codebase.

## Tier 1: Full Elimination (~39,200 lines)

Modules entirely subsumed by Restate's runtime:

| Module | Lines | Restate Replacement |
|--------|------:|---------------------|
| `workflow/checkpoint/` | 3,370 | Durable journaling — automatic step-level checkpoints |
| `workflow/idempotent/` | 2,277 | Built-in idempotency keys on invocations |
| `workflow/engine.rs` + handlers + types | 5,216 | Workflows defined as async functions with automatic ordering/retries |
| `orchestrator/messaging/` | 3,057 | Reliable RPC + exactly-once messaging natively |
| `orchestrator/persistence/` | 2,716 | Built-in K/V state per virtual object (no external DB for orchestration) |
| `orchestrator/actors/` | 12,274 | Virtual objects replace actors; scheduling/queuing handled by runtime |
| `orchestrator/timers/` | 1,730 | `ctx.sleep()` and delayed calls as durable primitives |
| `orchestrator/shutdown.rs` | 523 | Runtime-managed lifecycle |
| `orchestrator/supervision` | 2,394 | Auto-retry with configurable policies (no supervision trees) |
| `events/replay/` | 5,652 | Journal *is* the replay mechanism; recovery is automatic |

## Tier 2: Major Simplification (~15,700 lines saved)

Modules that partially survive but shrink significantly:

| Module | Current | After | Saved | Reasoning |
|--------|--------:|------:|------:|-----------|
| `events/bus.rs` | 913 | ~150 | ~760 | Restate messaging replaces pub/sub; circuit breaker built-in |
| `events/durable_store.rs` | 866 | 0 | 866 | Restate journals durably; no WAL needed |
| `events/event.rs` + `projection.rs` | ~2,700 | ~800 | ~1,900 | Domain types stay; projections/serialization shrink |
| `events/stage.rs` + `stage_gate.rs` | ~700 | ~400 | ~300 | Gate logic stays; state transitions managed by Restate |
| `orchestrator/ipc_bridge.rs` + messages | ~1,100 | ~600 | ~500 | IPC stays (Zellij); conversion simplifies |
| `swarm-coordinator/` | 269 | ~100 | ~170 | Virtual objects model agents more simply |
| `merge-queue/` | 355 | ~150 | ~200 | Keyed virtual objects serialize merge operations |
| Rest of `orchestrator/` | ~17,000 | ~6,000 | ~11,000 | Agent swarm lifecycle heavily simplifies |

## Tier 3: Untouched (~86,500 lines)

Domain/UI/infrastructure Restate doesn't replace:

| Module | Lines | Why It Stays |
|--------|------:|--------------|
| `crates/core/` | ~3,500 | Domain types, error taxonomy |
| `crates/pipeline/` | ~4,000 | CLI task lifecycle (becomes Restate service handlers) |
| `crates/telemetry/` | ~2,500 | OpenTelemetry (Restate integrates, doesn't replace) |
| `crates/oya-web/` | ~8,000 | Web UI layer |
| `crates/zellij-frontend/` | ~12,000 | Terminal UI plugin |
| `crates/oya-ipc/` | ~1,600 | Custom IPC transport |
| `src/` (CLI) | ~5,000 | CLI commands |
| Tests | ~50,000+ | Proportionally reduces with removed production code |

## Summary

| Category | Lines | % of Codebase |
|----------|------:|------:|
| Fully eliminated | ~39,200 | 28% |
| Simplified | ~15,700 | 11% |
| **Total savings** | **~54,900** | **39%** |
| Remaining | ~86,500 | 61% |

## What Changes Architecturally

### Before (current)
- Ractor actor system with supervisor trees
- Custom workflow engine with phase handlers
- Manual checkpoint/restore with Zstd compression
- Hand-rolled idempotency (SHA-256 + UUID v5)
- SurrealDB/RocksDB for orchestration state
- Custom event bus with circuit breakers
- WAL + fsync for durability
- Replay engine with dead letter queues
- Exactly-once delivery tracking with deduplication cache
- Graceful shutdown with 30s checkpoint coordination

### After (with Restate)
- Virtual objects replace actors (one per bead, one per agent)
- Workflows defined as async Rust functions
- Automatic durable journaling (no checkpoint code)
- Built-in idempotency keys
- Restate's internal state replaces SurrealDB for orchestration
- Restate's messaging replaces event bus
- No WAL needed
- No replay engine needed
- Exactly-once semantics built-in
- Restate handles graceful shutdown

### Example: Bead Workflow

```rust
#[restate_sdk::workflow]
impl BeadWorkflow {
    #[shared]
    async fn status(&self, ctx: SharedWorkflowContext<'_>) -> Result<StageKind> {
        ctx.get("stage").await
    }

    async fn run(&self, ctx: WorkflowContext<'_>, spec: BeadSpec) -> Result<BeadResult> {
        let research = ctx.run(|| agent_research(&spec)).await?;
        let plan = ctx.run(|| agent_plan(&research)).await?;
        let result = ctx.run(|| agent_implement(&plan)).await?;

        match stage_gate.evaluate(&result) {
            GateDecision::Proceed { .. } => Ok(result),
            GateDecision::Reenter { stage, .. } => {
                ctx.run(|| self.run_from(stage)).await
            }
            GateDecision::Fail { reason } => Err(reason.into()),
        }
    }
}
```

## Key Trade-off

**Runtime dependency**: Current architecture is fully self-contained (embedded
SurrealDB/RocksDB). Restate requires a separate sidecar process or Restate Cloud.
Orchestration durability moves from your code to Restate's runtime.

**SurrealDB retention**: SurrealDB may still be valuable for domain data (task
metadata, bead specs, historical queries) even if orchestration state moves to
Restate. The persistence crate shrinks but doesn't necessarily disappear entirely.
