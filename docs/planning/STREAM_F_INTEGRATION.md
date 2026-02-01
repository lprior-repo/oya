# Stream F: Integration + Chaos Testing

**Timeline**: Weeks 5-7
**Goal**: Wire everything together and prove rock-solid reliability
**LOC Target**: ~1.5k LOC
**Status**: Session initialized, beads NOT YET CREATED

## Overview

Stream F integrates all streams and validates system reliability:
- System initialization and graceful shutdown
- End-to-end bead execution tests
- 6 chaos test scenarios (100% recovery required)
- Performance benchmarks and load testing
- Memory profiling (no leaks)

## Planning Session

**Session ID**: `stream-f-integration`
**Session File**: `~/.local/share/planner/sessions/stream-f-integration.yml`
**Status**: INITIALIZED - Beads NOT yet generated

## Week-by-Week Breakdown

### Week 5: System Integration

**Components to integrate**:
1. **Main Orchestrator** (`src/main.rs`)
   - Initialize SurrealDB connection
   - Spawn UniverseSupervisor (Stream B)
   - Spawn tier-1 supervisors
   - Warm process pool (Stream D)
   - Start ReconciliationLoopActor
   - Start UIBridgeActor (WebSocket server from Stream E)
   - Listen on `:8080` for REST API + WebSocket

2. **Graceful Shutdown** (`src/shutdown.rs`)
   - SIGINT/SIGTERM signal handling
   - CancellationToken propagation
   - Checkpoint in-flight beads (30s timeout)
   - Flush event log to disk (fsync)
   - Kill all workers (SIGTERM → SIGKILL after 5s)
   - Optional: Clean up workspaces

3. **End-to-End Bead Execution**
   - Create bead → Schedule → Assign → Execute → Checkpoint → Complete
   - Event sourcing throughout (all state transitions logged)
   - Idempotency checks (no duplicate execution)
   - Resource cleanup (workspaces, worker processes)

### Week 6: Chaos Testing Framework

**6 Required Chaos Tests** (100% pass rate required):

#### Test 1: Kill Random Actors
- **Scenario**: Kill 10 random actors every 5s for 5 minutes
- **Expected**: System stabilizes, beads complete successfully
- **Success**: 100% recovery rate, no lost work

#### Test 2: Database Unavailable
- **Scenario**: Stop SurrealDB for 10 seconds
- **Expected**: Events buffer in memory, flush on recovery
- **Success**: Zero data loss, all events persisted

#### Test 3: Process Crash Mid-Execution
- **Scenario**: Kill OpenCode subprocess during bead execution
- **Expected**: Reconciler detects, spawns new worker, resumes from checkpoint
- **Success**: Bead completes successfully without re-execution

#### Test 4: Orchestrator Restart
- **Scenario**: Kill orchestrator process, restart immediately
- **Expected**: All in-flight beads resume from checkpoint
- **Success**: No lost work, all beads complete

#### Test 5: Network Partition (simulated)
- **Scenario**: Block OpenCode HTTP requests for 30 seconds
- **Expected**: Retry logic kicks in, eventual success
- **Success**: All beads complete despite network issues

#### Test 6: Disk Full (simulated)
- **Scenario**: Return ENOSPC on SurrealDB writes
- **Expected**: Graceful error, no corruption
- **Success**: System recovers when space becomes available

### Week 7: Performance Testing + Load Tests

**Benchmarks** (using criterion):
1. Event append latency (fsync overhead)
2. Idempotency check latency
3. Checkpoint save/load time
4. Actor message passing latency
5. Queue enqueue/dequeue latency
6. DAG topological sort time

**Load Test Scenarios**:
1. **100 Concurrent Beads**
   - Spawn 100 beads simultaneously
   - Measure: throughput (beads/min), latency (p50/p95/p99)
   - Target: p99 <10s (excluding AI time)

2. **1000 Sequential Beads**
   - Execute 1000 beads sequentially
   - Measure: total time, memory usage
   - Target: <1 hour total, stable RSS

3. **Complex DAG (100 nodes)**
   - Create workflow with 100-node DAG
   - Measure: scheduling time, execution time
   - Target: Correct topological order, no deadlocks

**Memory Profiling**:
- Tool: heaptrack or valgrind massif
- Duration: 1-hour sustained load
- Target: No memory leaks (RSS stable)
- Metrics: Heap allocations, peak RSS, leak detection

## Success Criteria

- ✅ Integrated system initialization (src/main.rs)
- ✅ Graceful shutdown implementation (<30s)
- ✅ REST API working (all endpoints)
- ✅ 6 chaos tests (100% pass rate)
- ✅ Load test: 100 beads, p99 <10s
- ✅ Performance benchmarks (all targets met)
- ✅ Memory profiling (no leaks, RSS stable)
- ✅ End-to-end integration tests (20+ scenarios)

## Integration Test Scenarios (20+ Required)

### Basic Flow
1. Create single bead, verify execution completes
2. Create workflow with 3 beads, verify sequential execution
3. Create workflow with DAG (diamond), verify correct order
4. Cancel running bead, verify graceful stop

### Error Handling
5. Submit workflow with cycle, verify rejection
6. Bead execution fails, verify retry logic
7. Worker unhealthy, verify reconciler respawns
8. Database write fails, verify retry with backoff

### Event Sourcing
9. Replay events, verify state reconstruction
10. Idempotency: execute same bead twice, verify cached result
11. Checkpoint mid-execution, resume, verify continuation
12. Event log integrity after crash

### Workspace Isolation
13. 10 concurrent beads, verify no file conflicts
14. Workspace cleanup on success
15. Workspace cleanup on failure/panic
16. Orphaned workspace detection and cleanup

### Scheduling
17. Priority queue: high-priority bead executes first
18. Round-robin queue: fair tenant distribution
19. Rate limiting: respect token bucket limits
20. Sticky assignment: soft mode prefers same worker

### UI Integration
21. WebSocket: real-time bead status updates
22. REST API: create bead via POST
23. REST API: cancel bead via POST
24. DAG visualization: accurate state representation

## Dependencies

**Testing Crates**:
- `criterion = "0.5"` - Performance benchmarks
- `proptest = "1.0"` - Property-based testing
- `tokio-test = "0.4"` - Async testing utilities
- `heaptrack` or `valgrind` - Memory profiling (system tools)

## Critical Files (To Be Created)

```
src/main.rs                             - Main orchestrator initialization
src/shutdown.rs                         - Graceful shutdown logic
src/api.rs                              - REST API integration
tests/integration/                      - Integration test suite
tests/chaos/                            - Chaos test scenarios
benches/                                - Performance benchmarks
```

## Next Steps

1. **Continue planning**: Create 6-8 atomic beads for Stream F
2. **Task breakdown**:
   - Orchestrator initialization
   - Graceful shutdown
   - 6 chaos tests (one bead per test or grouped)
   - Performance benchmarking
   - Load testing harness
3. **CUE schemas**: Generate validation schemas for each bead
4. **Implementation order**: Integration first, then chaos tests, finally benchmarks

## Notes

- **Critical path**: Stream F depends on ALL previous streams (A, B, C, D, E)
- **Chaos tests are mandatory**: 100% pass rate required for MVP
- **Performance targets**: p99 <10s is non-negotiable
- **Memory leaks**: Zero tolerance (must be verified)
- **Integration**: Final validation of entire system
