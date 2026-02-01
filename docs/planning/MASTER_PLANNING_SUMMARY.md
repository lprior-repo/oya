# Oya Rock-Solid Orchestrator - Master Planning Summary

**Date**: 2026-02-01
**Status**: Planning Complete for Streams A-D, Streams E-F initialized
**Total Beads Created**: 30 beads across 4 streams
**Total LOC Target**: ~32k LOC (14k existing → 32k final)

## Executive Summary

This document summarizes the complete planning for the Oya rock-solid orchestrator implementation. We've decomposed the 7-week, 6-stream parallel implementation into 30 atomic beads (with more to come for Streams E-F) using the planner skill with full CUE validation.

## Planning Approach

**Tool**: Planner skill with nushell state machine
**Validation**: Two-stage CUE schema validation
- Stage 1: Task JSON validation during planning (task-schema.cue)
- Stage 2: Bead-specific validation after implementation (bead-specific .cue files)

**Template**: 16-section enhanced bead template
- EARS requirements (ubiquitous, event-driven, unwanted)
- KIRK contracts (preconditions, postconditions, invariants)
- Comprehensive tests (happy, error, edge cases)
- Research requirements and implementation phases
- Anti-hallucination rules and context survival

## Stream Status Matrix

| Stream | Name | Weeks | Beads | Status | Session File |
|--------|------|-------|-------|--------|--------------|
| A | Event Sourcing + Durability | 1-4 | 8 | ✅ Complete | `stream-a-event-sourcing.yml` |
| B | Actor System + Supervision | 1-4 | 8 | ✅ Complete | `stream-b-actor-system.yml` |
| C | DAG Engine + Scheduling + Merge Queue | 3-5 | 8 | ✅ Complete | `stream-c-dag-scheduling.yml` |
| D | Process Pool + Workspace Isolation | 2-4 | 6 | ✅ Complete | `stream-d-process-pool.yml` |
| E | Tauri Desktop UI + axum Backend | 3-6 | TBD | 🟡 Initialized | `stream-e-tauri-ui.yml` |
| F | Integration + Chaos Testing | 5-7 | TBD | 🟡 Initialized | `stream-f-integration.yml` |

**Total**: 30 beads planned, ~10-12 more needed for Streams E-F

## All Created Beads (30 total)

### Stream A: Event Sourcing + Durability (8 beads)

| Bead ID | Title | Effort |
|---------|-------|--------|
| `intent-cli-20260201012642-u2duduno` | event-sourcing: Define complete SurrealDB schema with fsync | 4hr |
| `intent-cli-20260201012642-t73sooov` | event-sourcing: Implement DurableEventStore with bincode and fsync | 4hr |
| `intent-cli-20260201012642-xie2aw1d` | event-sourcing: Benchmark fsync overhead and validate performance | 2hr |
| `intent-cli-20260201012642-2sgtpztz` | idempotency: Generate deterministic UUID v5 keys from bead+input | 2hr |
| `intent-cli-20260201012642-z9lgvyom` | idempotency: Implement IdempotentExecutor with cache and DB storage | 4hr |
| `intent-cli-20260201012642-m5hwrwle` | checkpoint: Implement CheckpointManager with zstd compression | 4hr |
| `intent-cli-20260201012642-ioyp3n1s` | replay: Implement deterministic EventSourcingReplay state machine | 4hr |
| `intent-cli-20260201012642-srmhpngx` | integration: Event sourcing integration tests and validation | 2hr |

### Stream B: Actor System + Supervision (8 beads)

| Bead ID | Title | Effort |
|---------|-------|--------|
| `intent-cli-20260201013602-jvq0wsqm` | actor-system: Study ractor 0.15 and implement ping/pong example | 2hr |
| `intent-cli-20260201013602-pmhnlxc6` | supervision: Implement UniverseSupervisor with one_for_one strategy | 4hr |
| `intent-cli-20260201013602-oxbdslfu` | storage-actors: Implement StateManagerActor and EventStoreActor | 4hr |
| `intent-cli-20260201013602-mgtchiyn` | worker-actor: Implement BeadWorkerActor for bead lifecycle execution | 4hr |
| `intent-cli-20260201013700-jtnlsu5x` | queue-actors: Implement FIFOQueueActor and PriorityQueueActor | 4hr |
| `intent-cli-20260201013602-hh3jm2uw` | rate-limiter-actor: Implement token bucket rate limiter | 2hr |
| `intent-cli-20260201013602-9zu2gjgt` | reconciliation-actor: Implement K8s-style ReconciliationLoopActor | 4hr |
| `intent-cli-20260201013700-n3vsj0pd` | supervision-tests: Chaos tests for 100% supervision recovery | 4hr |

### Stream C: DAG Engine + Scheduling + Merge Queue (8 beads)

| Bead ID | Title | Effort |
|---------|-------|--------|
| `intent-cli-20260201014210-onbpr00o` | dag: Implement WorkflowDAG with petgraph DiGraph | 4hr |
| `intent-cli-20260201014210-eyeydwsf` | dag: Implement Kahn's algorithm for topological sort | 2hr |
| `intent-cli-20260201014210-6wxrwadt` | dag: Implement Tarjan's algorithm for cycle detection | 4hr |
| `intent-cli-20260201014210-u8yonjrv` | queue: Implement LIFOQueueActor for depth-first scheduling | 2hr |
| `intent-cli-20260201014210-ucmumtbr` | queue: Implement RoundRobinQueueActor for fair tenant scheduling | 4hr |
| `intent-cli-20260201014210-hjahahar` | dag: Implement SurrealDB graph queries for dependency resolution | 4hr |
| `intent-cli-20260201014210-0vjoinp5` | scheduler: Implement SchedulerActor with DAG maintenance | 4hr |
| `intent-cli-20260201014210-qnjb0bbj` | merge-queue: Implement PR lifecycle management with bead integration | 4hr |

### Stream D: Process Pool + Workspace Isolation (6 beads)

| Bead ID | Title | Effort |
|---------|-------|--------|
| `intent-cli-20260201014712-70yhgsj1` | process-pool: Implement ProcessPoolActor with subprocess management | 4hr |
| `intent-cli-20260201014712-nirdcff7` | opencode: Implement subprocess wrapper with HTTP + SSE communication | 4hr |
| `intent-cli-20260201014713-y9wal8p7` | process-pool: Implement heartbeat monitoring for dead worker detection | 2hr |
| `intent-cli-20260201014713-6wwfbzye` | zjj: Implement WorkspaceManager for isolated jj workspaces | 4hr |
| `intent-cli-20260201014713-mgcop1nx` | worker: Integrate WorkspaceManager with BeadWorkerActor | 2hr |
| `intent-cli-20260201014713-j3tktekl` | sticky: Implement sticky worker assignment with soft/hard modes | 4hr |

## CUE Schema Storage

**Location**: `/home/lewis/src/oya/.beads/schemas/`
**Total Schemas**: 30 bead-specific CUE files
**Purpose**: Post-implementation validation (Stage 2)

Each CUE schema enforces:
- Contract verification (preconditions, postconditions, invariants)
- Test verification (happy path, error path, edge cases)
- Code quality (no unwraps, no panics, CI passing)
- Implementation completeness

## Planning Session Files

**Location**: `~/.local/share/planner/sessions/`

All planning sessions stored as YAML:
```
stream-a-event-sourcing.yml      - 8 tasks, COMPLETE
stream-b-actor-system.yml        - 8 tasks, COMPLETE
stream-c-dag-scheduling.yml      - 8 tasks, COMPLETE
stream-d-process-pool.yml        - 6 tasks, COMPLETE
stream-e-tauri-ui.yml            - Initialized, tasks pending
stream-f-integration.yml         - Initialized, tasks pending
```

## Detailed Documentation

Each stream has comprehensive planning documentation:

- `docs/planning/STREAM_A_EVENT_SOURCING.md`
- `docs/planning/STREAM_B_ACTOR_SYSTEM.md`
- `docs/planning/STREAM_C_DAG_SCHEDULING.md`
- `docs/planning/STREAM_D_PROCESS_POOL.md`
- `docs/planning/STREAM_E_TAURI_UI.md` (stub)
- `docs/planning/STREAM_F_INTEGRATION.md` (stub)

## Critical Architecture Decisions

### Zero Data Loss (Stream A)
- SurrealDB with `sync_mode='full'` (fsync on every write)
- Expected overhead: 2-3ms per write
- Bincode serialization for efficiency
- Deterministic event replay

### Exactly-Once Execution (Stream A)
- UUID v5 deterministic idempotency keys
- HashMap cache + SurrealDB persistence
- No duplicate execution guaranteed

### Fault Isolation (Stream B)
- BEAM OTP supervision trees (3 tiers)
- one_for_one restart strategy
- Exponential backoff for repeated failures
- <1s restart time (p99)

### Dependency Resolution (Stream C)
- petgraph DiGraph for DAG representation
- Kahn's algorithm (topological sort, O(V+E))
- Tarjan's algorithm (cycle detection, O(V+E))
- SurrealDB graph queries for ready beads

### Workspace Isolation (Stream D)
- zjj for isolated jj workspaces
- UUID-based unique naming
- Drop trait for guaranteed cleanup
- Orphan detection and periodic cleanup

## Tech Stack Summary

**Core**:
- Rust (async with tokio)
- SurrealDB (embedded kv-rocksdb, fsync)
- ractor 0.15 (BEAM-style actors)
- petgraph 0.6 (graph algorithms)

**Serialization**:
- bincode (fast binary)
- zstd (checkpoint compression)

**UI** (Stream E):
- Tauri 2.0 (desktop app)
- Leptos 0.7 (Rust WASM reactive UI)
- axum 0.7 + tower (backend)

**Testing**:
- tokio-test (async)
- proptest (property-based)
- criterion (benchmarks)

## Success Metrics

### MVP Complete When:
- ✅ 100 concurrent beads execute successfully
- ✅ p99 latency <10s per bead (excluding AI time)
- ✅ Zero data loss (fsync guarantees, tested via chaos)
- ✅ Exactly-once execution (idempotency keys prevent duplicates)
- ✅ Graceful shutdown completes in <30s (checkpoint window)
- ✅ All actors supervised (auto-restart on crash)
- ✅ Absolute replay (deterministic event sourcing)
- ✅ DAG dependency resolution (detects cycles via Tarjan's)
- ✅ All 4 queue strategies implemented and tested
- ✅ Merge queue manages PR lifecycle
- ✅ Workspace isolation (no conflicts in concurrent workspaces)
- ✅ Tauri UI with Step Functions-like visualization
- ✅ Real-time WebSocket updates (<50ms latency)
- ✅ 6 chaos tests pass (100% recovery rate)
- ✅ Load test: 100 beads, p99 <10s
- ✅ No memory leaks (RSS stable over 1hr)

## Next Steps

### Immediate (Streams E-F Planning)
1. Create 8-10 beads for Stream E (Tauri UI + axum)
2. Create 6-8 beads for Stream F (Integration + Chaos)
3. Generate CUE schemas for all new beads
4. Update this master summary

### Implementation Order
1. **Stream A** → Foundation (event sourcing, idempotency, checkpoints)
2. **Stream B** → Actor system (supervision, workers, reconciliation)
3. **Stream C** → Scheduling (DAG, queues, merge queue)
4. **Stream D** → Process management (pool, workspaces, sticky assignment)
5. **Stream E** → User interface (Tauri + Leptos + axum)
6. **Stream F** → Final integration (chaos tests, benchmarks, validation)

### Validation Strategy
1. Run `moon run :quick` after every code change (6-7ms cached)
2. Use CUE schemas for post-implementation validation
3. Run integration tests after completing each stream
4. Run chaos tests continuously during Stream F
5. Memory profiling in final week

## Git Commits

**Latest Commit** (2026-02-01):
```
commit: 87ff1f21
message: Add Streams A, B, C bead planning with CUE validation schemas
files:
  - 24 new CUE schemas in .beads/schemas/
  - Planning session YAML files
  - Updated documentation
```

## References

- Main plan: `/home/lewis/.claude/plans/sorted-toasting-parasol.md`
- Planner skill: `~/.claude/skills/planner/planner.nu`
- Session state: `~/.local/share/planner/sessions/`
- CUE schemas: `/home/lewis/src/oya/.beads/schemas/`
- Documentation: `/home/lewis/src/oya/docs/planning/`

---

**Ẹpa OYA. Let the storm begin. 100x throughput awaits.**
