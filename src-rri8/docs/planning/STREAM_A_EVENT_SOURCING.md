# Stream A: Event Sourcing + Durability

**Timeline**: Weeks 1-4
**Goal**: Rock-solid event sourcing with fsync, idempotency, checkpoints, and deterministic replay
**LOC Target**: ~3k LOC

## Overview

Stream A establishes the foundation for zero data loss and exactly-once execution through:
- DurableEventStore with bincode serialization and fsync guarantees
- IdempotentExecutor with UUID v5 deterministic keys
- CheckpointManager with zstd compression
- EventSourcingReplay with deterministic state machine

## Planning Session

**Session ID**: `stream-a-event-sourcing`
**Session File**: `~/.local/share/planner/sessions/stream-a-event-sourcing.yml`
**Status**: Complete - 8 beads generated

## Beads Created (8 total)

| Task | Title | Bead ID | Effort | Priority |
|------|-------|---------|--------|----------|
| task-001 | event-sourcing: Define complete SurrealDB schema with fsync | `intent-cli-20260201012642-u2duduno` | 4hr | 1 |
| task-002 | event-sourcing: Implement DurableEventStore with bincode and fsync | `intent-cli-20260201012642-t73sooov` | 4hr | 1 |
| task-003 | event-sourcing: Benchmark fsync overhead and validate performance | `intent-cli-20260201012642-xie2aw1d` | 2hr | 2 |
| task-004 | idempotency: Generate deterministic UUID v5 keys from bead+input | `intent-cli-20260201012642-2sgtpztz` | 2hr | 1 |
| task-005 | idempotency: Implement IdempotentExecutor with cache and DB storage | `intent-cli-20260201012642-z9lgvyom` | 4hr | 1 |
| task-006 | checkpoint: Implement CheckpointManager with zstd compression | `intent-cli-20260201012642-m5hwrwle` | 4hr | 1 |
| task-007 | replay: Implement deterministic EventSourcingReplay state machine | `intent-cli-20260201012642-ioyp3n1s` | 4hr | 1 |
| task-008 | integration: Event sourcing integration tests and validation | `intent-cli-20260201012642-srmhpngx` | 2hr | 1 |

## CUE Validation Schemas

All bead-specific CUE schemas stored in: `/home/lewis/src/oya/.beads/schemas/`

```
intent-cli-20260201012642-u2duduno.cue  (task-001: SurrealDB schema)
intent-cli-20260201012642-t73sooov.cue  (task-002: DurableEventStore)
intent-cli-20260201012642-xie2aw1d.cue  (task-003: Benchmark fsync)
intent-cli-20260201012642-2sgtpztz.cue  (task-004: UUID v5 keys)
intent-cli-20260201012642-z9lgvyom.cue  (task-005: IdempotentExecutor)
intent-cli-20260201012642-m5hwrwle.cue  (task-006: CheckpointManager)
intent-cli-20260201012642-ioyp3n1s.cue  (task-007: EventSourcingReplay)
intent-cli-20260201012642-srmhpngx.cue  (task-008: Integration tests)
```

## Key Technical Decisions

### SurrealDB Configuration
- **Storage**: Embedded kv-rocksdb
- **Sync Mode**: `sync_mode='full'` for fsync guarantees
- **Expected Overhead**: 2-3ms per write (acceptable for zero data loss)

### Schema Design (12 Tables)
1. **state_transition** - Append-only event log with fsync
2. **idempotency_key** - Prevents duplicate execution
3. **checkpoint** - Compressed state snapshots (zstd)
4. **bead** - Bead metadata
5. **workflow_run** - Workflow execution tracking
6. **process** - Worker process tracking
7. **workspace** - Workspace isolation state
8. **schedule** - Scheduled execution
9. **token_bucket** - Rate limiting state
10. **concurrency_limit** - Resource limits
11. **webhook** - Webhook configurations
12. **Graph relations** - depends_on, blocks edges

### Idempotency Strategy
- **Key Generation**: UUID v5 from `namespace(bead_id) + input_hash`
- **Storage**: HashMap with RwLock for in-memory cache
- **Persistence**: SurrealDB `idempotency_key` table
- **Guarantee**: Same input → Same result (no duplicate execution)

### Checkpoint Strategy
- **Frequency**: Auto-checkpoint every 60s + on-demand
- **Compression**: zstd (high compression ratio, fast decompression)
- **Storage**: SurrealDB `checkpoint` table
- **Resume**: Exact state restoration from checkpoint

## Success Criteria

- ✅ Replay 1000 events <5s
- ✅ Idempotency tests pass (same input = same result)
- ✅ fsync overhead measured (2-3ms acceptable)
- ✅ Zero data loss proven via chaos tests
- ✅ Checkpoint/resume cycle completes successfully
- ✅ All contracts validated by CUE schemas

## Dependencies

**Rust Crates**:
- `surrealdb = { version = "2.0", features = ["kv-rocksdb"] }`
- `bincode = "1.3"` - Fast binary serialization
- `uuid = { version = "1.0", features = ["v5"] }` - Deterministic keys
- `zstd = "0.13"` - Checkpoint compression

## Critical Files

```
crates/events/src/durable_store.rs      - DurableEventStore implementation
crates/workflow/src/idempotent.rs       - IdempotentExecutor
crates/workflow/src/checkpoint.rs       - CheckpointManager
crates/events/src/replay.rs             - EventSourcingReplay
schema.surql                            - Complete SurrealDB schema
```

## Quality Gates

All beads enforce:
- Zero unwraps, zero panics (clippy forbid)
- Railway-Oriented Programming (ResultExt, OptionExt from oya-core)
- All errors use `Result<T, Error>` with thiserror
- Proptest for property-based testing
- 100% deterministic replay

## Next Steps

1. Review bead specifications in detail
2. Begin implementation starting with task-001 (SurrealDB schema)
3. Use CUE schemas for post-implementation validation
4. Run integration tests after completing all 8 beads
