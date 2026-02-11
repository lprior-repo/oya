# Stream D: Process Pool + Workspace Isolation

**Timeline**: Weeks 2-4
**Goal**: Warm OpenCode servers with health monitoring, zjj workspace isolation, sticky assignment
**LOC Target**: ~2k LOC

## Overview

Stream D manages OpenCode worker processes and workspace isolation:
- ProcessPoolActor with subprocess management
- OpenCode HTTP + SSE communication wrapper
- Heartbeat monitoring for dead worker detection
- WorkspaceManager for isolated jj workspaces
- Sticky worker assignment (soft and hard modes)

## Planning Session

**Session ID**: `stream-d-process-pool`
**Session File**: `~/.local/share/planner/sessions/stream-d-process-pool.yml`
**Status**: Complete - 6 beads generated

## Beads Created (6 total)

| Task | Title | Bead ID | Effort | Priority |
|------|-------|---------|--------|----------|
| task-001 | process-pool: Implement ProcessPoolActor with subprocess management | `intent-cli-20260201014712-70yhgsj1` | 4hr | 1 |
| task-002 | opencode: Implement subprocess wrapper with HTTP + SSE communication | `intent-cli-20260201014712-nirdcff7` | 4hr | 1 |
| task-003 | process-pool: Implement heartbeat monitoring for dead worker detection | `intent-cli-20260201014713-y9wal8p7` | 2hr | 1 |
| task-004 | zjj: Implement WorkspaceManager for isolated jj workspaces | `intent-cli-20260201014713-6wwfbzye` | 4hr | 1 |
| task-005 | worker: Integrate WorkspaceManager with BeadWorkerActor | `intent-cli-20260201014713-mgcop1nx` | 2hr | 1 |
| task-006 | sticky: Implement sticky worker assignment with soft/hard modes | `intent-cli-20260201014713-j3tktekl` | 4hr | 2 |

## CUE Validation Schemas

All bead-specific CUE schemas stored in: `/home/lewis/src/oya/.beads/schemas/`

```
intent-cli-20260201014712-70yhgsj1.cue  (task-001: ProcessPoolActor)
intent-cli-20260201014712-nirdcff7.cue  (task-002: OpenCode wrapper)
intent-cli-20260201014713-y9wal8p7.cue  (task-003: Heartbeat monitoring)
intent-cli-20260201014713-6wwfbzye.cue  (task-004: WorkspaceManager)
intent-cli-20260201014713-mgcop1nx.cue  (task-005: Workspace integration)
intent-cli-20260201014713-j3tktekl.cue  (task-006: Sticky assignment)
```

## Key Technical Decisions

### Process Pool Management

**ProcessPoolActor State**:
- Data structure: HashMap<ProcessId, WorkerState>
- Worker states: Idle, Claimed, Unhealthy, Dead
- Pool size: Configurable (default 20 workers)
- Lifecycle: Spawn → Claim → Release → Shutdown

**Subprocess Spawning**:
- Command: `tokio::process::Command`
- Binary: OpenCode (HTTP server mode)
- Port allocation: Dynamic or pre-configured range
- Environment: Isolated per worker

**Graceful Shutdown**:
1. Send SIGTERM to all workers
2. Wait 5 seconds for clean shutdown
3. Send SIGKILL to remaining processes
4. Verify all processes terminated (no zombies)

### OpenCode Communication

**HTTP API**:
- `POST /execute` - Execute bead with input
- `GET /health` - Health check endpoint
- `POST /cancel` - Cancel running execution
- Timeout: 5 minutes per request (configurable)

**Server-Sent Events (SSE)**:
- Stream: Real-time output from bead execution
- Format: Newline-delimited JSON
- Reconnection: Automatic on connection drop
- Buffer: In-memory until delivery

**Retry Logic**:
- Strategy: Exponential backoff (1s, 2s, 4s)
- Max retries: 3 for transient failures (5xx)
- Timeout: 5s per attempt
- Fallback: Return error after max retries

### Heartbeat Monitoring

**Health Check Schedule**:
- Interval: 30 seconds
- Timeout: 5 seconds
- Endpoint: `GET /health`
- Response: 200 OK = healthy

**Failure Detection**:
1. Health check times out (>5s)
2. Mark worker as Unhealthy
3. Emit WorkerUnhealthy event
4. ReconciliationLoopActor respawns worker

**Recovery**:
- New worker spawned by reconciler
- Old worker PID tracked for cleanup
- Sticky assignments updated

### Workspace Isolation (zjj)

**WorkspaceManager**:
- Create isolated jj workspace per bead
- Workspace naming: UUID-based for uniqueness
- Optional Zellij session integration (deferred to post-MVP)
- Cleanup guaranteed via Drop trait

**Workspace Lifecycle**:
```rust
1. create_workspace(bead_id) -> WorkspaceGuard
2. Execute bead in workspace directory
3. Drop WorkspaceGuard -> jj workspace forget
```

**Key Commands**:
- Create: `jj workspace add <uuid>`
- Destroy: `jj workspace forget <uuid>`
- List: `jj workspace list` (for orphan detection)

**Orphan Cleanup**:
- Periodic task: Every 1 hour
- Detection: Workspaces older than 2 hours with no active bead
- Action: `jj workspace forget` + directory removal

### Sticky Assignment

**Soft Sticky Mode**:
- Prefer previous worker if available
- Fallback to any idle worker if previous unhealthy/unavailable
- Use case: Default mode (best effort reuse)

**Hard Sticky Mode**:
- Wait for specific worker (no fallback)
- Timeout: 30 seconds
- Use case: Stateful beads requiring same worker
- Error: Return timeout error if worker unavailable

**Assignment Storage**:
- Table: SurrealDB `worker_assignment`
- Fields: bead_id, worker_id, assigned_at
- Query: O(1) lookup by bead_id
- Expiry: Optional (remove assignments older than 7 days)

## Success Criteria

- ✅ ProcessPoolActor with health monitoring implemented
- ✅ OpenCode subprocess wrapper (HTTP + SSE) working
- ✅ crates/oya-zjj/ workspace isolation functional
- ✅ Sticky assignment (soft and hard modes) working
- ✅ Graceful shutdown completes in <5s
- ✅ Test suite: 20 workers, concurrent workspaces, health checks passing
- ✅ No zombie processes after shutdown
- ✅ Workspace cleanup guaranteed (even on panic)

## Dependencies

**Rust Crates**:
- `tokio = { version = "1.0", features = ["process", "time"] }` - Subprocess management
- `reqwest = { version = "0.12", features = ["json", "stream"] }` - HTTP client
- `eventsource-client = "0.12"` or similar - SSE client
- `uuid = { version = "1.0", features = ["v4"] }` - Workspace naming

## Critical Files

```
crates/orchestrator/src/process_pool.rs     - ProcessPoolActor
crates/opencode/src/worker.rs               - OpenCodeWorker wrapper
crates/orchestrator/src/heartbeat.rs        - Heartbeat monitoring
crates/oya-zjj/src/workspace.rs             - WorkspaceManager
crates/oya-zjj/src/lib.rs                   - zjj crate root
crates/orchestrator/src/sticky.rs           - Sticky assignment
```

## Quality Gates

All beads enforce:
- No zombie processes (verified in tests)
- Workspace cleanup guaranteed (Drop trait)
- Health checks detect failures <35s (30s interval + 5s timeout)
- Sticky hit rate >80% (in soft mode)
- Process spawn/shutdown <1s

## Next Steps

1. Implement ProcessPoolActor with subprocess management
2. Wrap OpenCode HTTP + SSE communication
3. Add heartbeat monitoring
4. Port zjj workspace isolation
5. Integrate workspace lifecycle with BeadWorkerActor
6. Implement sticky assignment strategies
