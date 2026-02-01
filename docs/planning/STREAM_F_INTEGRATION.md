# Stream F: Integration + Chaos Testing

**Duration**: Weeks 5-7 (3 weeks)
**LOC**: ~1.5k
**Priority**: Critical (production readiness validation)

## Overview

Wire everything together and prove rock-solid reliability through system integration, 6 chaos tests, performance benchmarks, load testing with 100 concurrent beads, and memory profiling.

---

## Bead Breakdown

**Total Beads**: 6
**Planning Session**: stream-f-integration (COMPLETE)

| # | Bead ID | Title | Type | Priority | Effort | Description |
|---|---------|-------|------|----------|--------|-------------|
| 1 | intent-cli-20260201020059-jjbgksde | integration: Implement orchestrator initialization and graceful shutdown | task | 1 | 4hr | Implement main orchestrator initialization sequence (SurrealDB, supervision tree, process pool, API server) and graceful shutdown with <30s checkpoint window. Handles SIGINT/SIGTERM properly. |
| 2 | intent-cli-20260201020059-mahvrqrz | integration: End-to-end bead execution integration tests | task | 1 | 4hr | Implement 20+ integration test scenarios covering: single bead, sequential workflow, DAG workflow, concurrent execution, cancellation, failure recovery, idempotent execution. Full stack validation. |
| 3 | intent-cli-20260201020339-ykctqedr | chaos: Implement chaos testing framework with 6 test scenarios | task | 1 | 4hr | Implement chaos testing framework with 6 scenarios: kill actors, DB unavailable, process crash, orchestrator restart, network partition, disk full. Validates 100% recovery rate. |
| 4 | intent-cli-20260201020339-tou8kwbh | perf: Implement performance benchmarks with criterion | task | 2 | 2hr | Implement performance benchmarks for critical paths: event append, idempotency check, checkpoint save/load, actor messaging, queue ops, DAG topological sort. Validates performance targets (event append <3ms, actor messaging <1ms, etc.). |
| 5 | intent-cli-20260201020059-jonmp2v0 | perf: Implement load testing with 100 concurrent beads | task | 1 | 4hr | Implement load test executing 100 concurrent beads simultaneously. Measures throughput (beads/min), latency (p50/p95/p99), validates p99 <10s target (excluding AI time). |
| 6 | intent-cli-20260201020339-fauedab9 | perf: Memory profiling and leak detection | task | 1 | 2hr | Implement memory profiling with heaptrack/valgrind. Runs 1-hour sustained load, monitors RSS every 10s, validates zero memory leaks and RSS stable (variance <10%). |

---

## Week-by-Week Breakdown

### Week 5: System Integration
**Focus**: Wire all components together, end-to-end execution

**Tasks**:
1. **Orchestrator Initialization** (Bead #1):
   - Init SurrealDB connection
   - Spawn UniverseSupervisor with tier-1 supervisors
   - Warm process pool (20 workers)
   - Start ReconciliationLoopActor (1s tick)
   - Start axum API server on :8080
   - Test: Full stack starts cleanly

2. **Graceful Shutdown** (Bead #1):
   - SIGINT/SIGTERM signal handling
   - Cancel all new work (CancellationToken)
   - Checkpoint in-flight beads (30s timeout)
   - Flush event log to disk (fsync)
   - Kill all workers (SIGTERM → wait 5s → SIGKILL)
   - Test: Shutdown completes <30s, no data loss

3. **End-to-End Integration Tests** (Bead #2):
   - Single bead execution (create → schedule → execute → complete)
   - Workflow with 3 sequential beads (dependency chain)
   - Workflow with DAG (diamond pattern, 4 beads)
   - 10 concurrent beads (parallel execution)
   - Idempotent execution (same input → same result)
   - Cancel running bead
   - Bead execution fails (verify error handling)
   - Worker crash mid-execution (verify recovery)
   - Database unavailable (verify buffering)
   - Test: All 20+ scenarios pass

### Week 6: Chaos Testing
**Focus**: Prove 100% recovery from failures

**Chaos Scenarios** (Bead #3):
1. **Kill Random Actors**:
   - Kill 10 random actors every 5s for 5 minutes
   - Verify: Supervisors restart actors, system stabilizes
   - Success: 100% recovery rate

2. **Database Unavailable**:
   - Stop SurrealDB for 10s
   - Verify: Events buffer in memory, flush on recovery
   - Success: Zero data loss

3. **Process Crash Mid-Execution**:
   - Kill OpenCode server during bead execution
   - Verify: Reconciler detects, spawns new worker, resumes from checkpoint
   - Success: Bead completes successfully

4. **Orchestrator Restart**:
   - Kill orchestrator process, restart immediately
   - Verify: All in-flight beads resume from checkpoint
   - Success: No lost work

5. **Network Partition** (simulated):
   - Block OpenCode HTTP requests for 30s
   - Verify: Retry logic kicks in, eventual success
   - Success: All beads complete

6. **Disk Full** (simulated):
   - Return ENOSPC on SurrealDB writes
   - Verify: Graceful error, no corruption
   - Success: System recovers when space available

### Week 7: Performance Testing & Load Tests
**Focus**: Validate performance targets and scalability

**Performance Benchmarks** (Bead #4):
- Event append latency (target: <3ms with fsync)
- Idempotency check latency (target: <1ms)
- Checkpoint save/load time (target: <100ms)
- Actor message passing latency (target: <1ms)
- Queue enqueue/dequeue latency (target: <1ms)
- DAG topological sort (100 nodes, target: <10ms)
- Tool: criterion benchmarks with statistical significance

**Load Testing** (Bead #5):
- Execute 100 concurrent beads
- Measure throughput (beads/min)
- Measure latency (p50/p95/p99)
- Target: p99 <10s (excluding AI time)
- Verify: No queue starvation, fair scheduling

**Memory Profiling** (Bead #6):
- 1-hour sustained load test
- Monitor RSS every 10s
- Tool: heaptrack or valgrind massif
- Verify: No memory leaks (RSS stable, variance <10%)
- Profile: Allocation hot paths, optimize if needed

---

## Quality Gates

### System Integration Gates (Week 5)
- ✅ Orchestrator starts all components successfully
- ✅ Graceful shutdown completes in <30s
- ✅ All 20+ integration tests pass
- ✅ No resource leaks after shutdown
- ✅ Event log is complete and durable

### Chaos Testing Gates (Week 6)
- ✅ All 6 chaos tests pass
- ✅ 100% recovery rate achieved
- ✅ No data loss in any scenario
- ✅ Recovery time <2min (p99)
- ✅ Chaos tests are repeatable and deterministic

### Performance Gates (Week 7)
- ✅ All benchmarks meet targets
- ✅ Load test: 100 beads complete successfully
- ✅ Load test: p99 latency <10s
- ✅ Memory profiling: Zero leaks detected
- ✅ Memory profiling: RSS stable over 1 hour

---

## Success Criteria

### MVP Complete When:
- ✅ Integrated system initialization (src/main.rs)
- ✅ Graceful shutdown implementation (<30s)
- ✅ REST API working (4 endpoints)
- ✅ 6 chaos tests pass (100% recovery)
- ✅ Load test: 100 concurrent beads, p99 <10s
- ✅ Performance benchmarks: All targets met
- ✅ Memory profiling: No leaks, RSS stable
- ✅ 20+ integration tests passing

---

## Critical Files

### Integration (Week 5)
- `src/main.rs` - Orchestrator initialization and shutdown
- `src/shutdown.rs` - Graceful shutdown implementation
- `src/api.rs` - REST API server (axum integration)
- `tests/integration/mod.rs` - Integration test framework
- `tests/integration/e2e_test.rs` - End-to-end test scenarios

### Chaos Testing (Week 6)
- `tests/chaos/mod.rs` - Chaos testing framework
- `tests/chaos/kill_actors.rs` - Test 1: Kill random actors
- `tests/chaos/db_unavailable.rs` - Test 2: Database unavailable
- `tests/chaos/process_crash.rs` - Test 3: Process crash
- `tests/chaos/orchestrator_restart.rs` - Test 4: Orchestrator restart
- `tests/chaos/network_partition.rs` - Test 5: Network partition
- `tests/chaos/disk_full.rs` - Test 6: Disk full

### Performance Testing (Week 7)
- `benches/performance.rs` - Criterion benchmarks
- `tests/load/mod.rs` - Load testing framework
- `tests/load/concurrent_100.rs` - 100 concurrent beads test
- `scripts/memory_profile.sh` - Memory profiling script

---

## Testing Infrastructure

### Tools
- **criterion**: Performance benchmarks with statistical significance
- **tokio-test**: Async testing for actors
- **proptest**: Property-based testing (event replay, idempotency)
- **heaptrack**: Memory profiling and leak detection
- **Custom chaos framework**: Process injection, network simulation

### Test Harnesses
- Integration test framework with setup/teardown
- Chaos test framework with failure injection helpers
- Load test framework with metrics collection (Prometheus)
- Memory profiling automation script

---

## Risk Mitigation

### Risk 1: Chaos Tests Unstable
**Mitigation**:
- Make chaos tests deterministic (seed RNG)
- Use retry logic for transient failures
- Document expected recovery time ranges
- If flaky, increase timeouts (but fail if exceed limits)

### Risk 2: Performance Targets Missed
**Mitigation**:
- Profile early (Week 7, not at end)
- Optimize hot paths (event append, actor messaging)
- Use arena allocators for short-lived data (bumpalo)
- If still slow, document and defer optimization to post-MVP

### Risk 3: Memory Leaks at Scale
**Mitigation**:
- Aggressive cleanup (drop workspaces ASAP)
- Clear caches periodically (TTL-based)
- Use Weak references where appropriate (prevent cycles)
- If leak found, fix immediately (zero tolerance)

---

## Dependencies

### Stream Dependencies
- **Requires**: Streams A (event sourcing), B (actor system), C (DAG scheduling), D (process pool), E (UI)
- **Blocks**: None (final integration stream)

### External Dependencies
- `criterion = "0.5"` - Performance benchmarks
- `heaptrack` or `valgrind` - Memory profiling
- `tokio-test = "0.4"` - Async testing
- `proptest = "1.0"` - Property-based testing

---

## Planning Session Details

**Session ID**: stream-f-integration
**Status**: COMPLETE
**Created**: 2026-02-01T01:47:35
**Tasks Generated**: 6
**Beads Created**: 6
**CUE Schemas**: 6 (all in `.beads/schemas/`)

**State File**: `~/.local/share/planner/sessions/stream-f-integration.yml`

---

## Next Steps

1. **Verify planning**: Review all 6 bead specifications
2. **CUE validation**: Ensure all schemas pass validation
3. **Update master summary**: Update MASTER_PLANNING_SUMMARY.md with final totals
4. **Begin implementation**: Start with Stream A (event sourcing foundation)
5. **Track progress**: Use bead system to track completion of all 42 beads

**Ẹpa OYA. Let the storm complete. Rock-solid orchestrator awaits.**
