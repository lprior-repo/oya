# Oya Architecture Evaluation: IPC for Zellij + AI Software Creation Flaws

**Date**: 2026-02-08
**Scope**: Full architecture review of `oya-ipc`, Zellij integration, and critical analysis of AI-native SDLC for 100x throughput

---

## Part 1: IPC Architecture for Zellij — Evaluation

### What Exists Today

The `oya-ipc` crate implements a **length-prefixed bincode transport** over `std::io::Read`/`Write` streams:

| Component | Location | Status |
|-----------|----------|--------|
| `IpcTransport<R, W>` | `crates/oya-ipc/src/transport.rs` | Implemented, tested |
| `TransportError` | `crates/oya-ipc/src/error.rs` | Implemented, 7 error variants |
| `GuestMessage` / `HostMessage` | `crates/oya-ipc/src/messages.rs` | Defined but commented out of `lib.rs` |
| `IpcWorkerActorDef` | `crates/orchestrator/src/actors/ipc_worker.rs` | Implemented, stub handlers |
| Duplicate `ipc_messages.rs` | `crates/orchestrator/src/ipc_messages.rs` | Copy of messages.rs in orchestrator |

### Protocol Design: Strengths

1. **Correct protocol choice for Zellij WASM plugins**. stdin/stdout length-prefixed framing is the *only* reliable IPC mechanism for Zellij WASM guest plugins. WebSocket/SSE require a network stack the WASM sandbox doesn't provide. This choice is architecturally sound.

2. **bincode serialization is the right pick**. Sub-microsecond serialization (<1KB), compact wire format, zero-copy-friendly. Matches the "fastest protocol" principle from `BACKEND_ARCHITECTURE.md`.

3. **1MB max payload with explicit validation**. The `MAX_PAYLOAD_SIZE` constant and validation at both send/recv prevents unbounded allocation. Good defensive design.

4. **Clean error taxonomy**. 7 distinct error variants with diagnostic context (byte counts, error codes). No information loss. All `TransportResult<T>` — fits the railway-oriented policy.

5. **BufReader/BufWriter with MAX_FRAME_SIZE capacity**. Pre-allocated buffers prevent repeated allocation on hot paths.

### Protocol Design: Issues

#### CRITICAL: Synchronous blocking I/O in an async actor system

```
IpcTransport<R, W> uses std::io::{Read, Write}
IpcWorkerActorDef is a ractor Actor running on tokio
```

The transport is synchronous. The actor system is async (tokio). Calling `transport.recv()` inside an actor `handle()` method will **block the tokio runtime thread**. This is the single most dangerous design flaw in the IPC layer.

**Fix**: Either:
- (a) Use `tokio::io::{AsyncRead, AsyncWrite}` and rewrite transport as async, OR
- (b) Wrap blocking reads in `tokio::task::spawn_blocking`, OR
- (c) Run the IPC transport on a dedicated `std::thread` and bridge to the actor via `mpsc`

Option (c) is the most pragmatic for Zellij since the plugin API provides real file descriptors (stdin/stdout), not async streams.

#### HIGH: Message types duplicated in two places

`GuestMessage`, `HostMessage`, and all supporting types exist in:
- `crates/oya-ipc/src/messages.rs` (commented out of `lib.rs`)
- `crates/orchestrator/src/ipc_messages.rs` (active, used by `ipc_worker.rs`)

This will inevitably diverge. The canonical location should be `oya-ipc` since it defines the wire protocol. The orchestrator should `use oya_ipc::messages::*`.

#### HIGH: No request-response correlation

`GuestMessage` has no request ID. When the plugin sends `GetBeadList` and `GetBeadDetail` concurrently, there's no way to match responses to requests. The current design assumes strictly sequential request-response, which breaks under any pipelining.

**Fix**: Add a `request_id: u64` field to `GuestMessage` and echo it back in every `HostMessage` response.

#### MEDIUM: `IpcTransport::pair()` is `#[cfg(test)]` only

There's no production mechanism to create an `IpcTransport` from stdin/stdout. The `pair()` function only works in tests. Need a `IpcTransport::from_stdio()` or similar constructor for production Zellij plugin use.

#### MEDIUM: No heartbeat / keepalive

If the Zellij plugin crashes or the pipe breaks, the orchestrator has no mechanism to detect it. The `is_eof()` method checks the buffer, not the underlying stream. Need periodic heartbeat messages with timeout detection.

#### LOW: `clear_buffers()` doesn't actually clear the read buffer

```rust
pub fn clear_buffers(&mut self) {
    let _ = self.reader.fill_buf();  // This reads INTO the buffer, doesn't clear it
    let _ = self.writer.flush();
}
```

`fill_buf()` fills the buffer with more data from the reader — opposite of clearing. To actually drain, you need `self.reader.consume(self.reader.buffer().len())`.

### IPC Architecture Verdict

**The foundation is solid.** Length-prefixed bincode over pipes is the correct choice for Zellij WASM IPC. The transport layer is well-implemented at the byte level. But three things need fixing before production use:

1. Async bridging (blocking I/O + async actors = deadlock risk)
2. Request correlation (no concurrent request support)
3. Type deduplication (two copies of the message types)

---

## Part 2: Biggest Flaws for AI Software Creation at 100x

### The Core Promise

Oya aims to be a **13-agent swarm** (4 Test Writers, 4 Implementers, 4 Reviewers, 1 Planner) running a 9-stage pipeline with event-sourced state, DAG scheduling, and Zellij-based visualization. The thesis: parallel AI agents = 100x developer throughput.

Here are the **hardest, most honest flaws** standing between the current codebase and that 100x goal:

### Flaw 1: The Orchestration Layer Is Vastly Over-Engineered Relative to the Agent Layer

**The ratio is inverted.** The codebase has:
- ~80+ Rust source files for orchestration, scheduling, persistence, replay, supervision, DAG, distribution, messaging, timers, virtual objects
- 0 lines of actual AI agent integration code

The `ipc_worker.rs` returns hardcoded empty vectors. The swarm module uses file-based handoff (`/tmp/bead-contracts-<id>.json`). There is no LLM client, no prompt management, no context window strategy, no tool-use protocol, no agent memory.

**Why this matters for 100x**: The orchestrator is the *plumbing*. The agents are the *value*. You can have a perfect Erlang-style supervision tree, but if the agents can't actually write, test, and review code, throughput is zero. The current architecture optimizes for failure recovery of agents that don't exist yet.

**What 100x actually requires**: A working single-agent loop (prompt → code → test → feedback → iterate) that completes one bead end-to-end. Then parallelize. Right now, the parallelization infrastructure exists but the unit of work doesn't.

### Flaw 2: File-Based Handoff (`/tmp/`) Is a Non-Starter for Reliability

From `src/swarm/mod.rs`:
```
/tmp/bead-contracts-<id>.json
/tmp/bead-ready-to-implement-<id>.json
/tmp/bead-implementation-complete-<id>.json
```

This contradicts every reliability guarantee the rest of the architecture provides:
- Event sourcing with fsync → but handoffs are untracked `/tmp/` files
- Supervision trees with 100% recovery → but `/tmp/` is wiped on reboot
- Idempotent operations → but file creation/deletion isn't atomic

The 850+ beads of architecture documentation describe a system that durably tracks every state transition, then the actual agent communication uses ephemeral temp files.

**Fix**: Agent handoffs should flow through the EventBus/DurableEventStore like everything else. The swarm handoff should be event-sourced bead state transitions, not filesystem operations.

### Flaw 3: No AI Context Management Strategy

The hardest problem in AI-native software creation isn't scheduling — it's **context**. AI agents have finite context windows (128K-200K tokens). A real codebase has millions of lines.

The architecture has no design for:
- **Code retrieval**: How does an agent get the relevant files for a bead?
- **Context budget**: How much of the context window goes to code vs. instructions vs. test output vs. error messages?
- **Incremental context**: When an agent fails and retries, does it get the failure context?
- **Cross-agent context**: When a reviewer rejects code, how does the feedback reach the implementer with enough context to fix it?
- **Codebase indexing**: No embedding store, no AST analysis, no dependency graph for targeted retrieval

Without context management, each agent operates blind. It doesn't matter if you have 13 agents if they can't see the code they need.

### Flaw 4: The 9-Stage Pipeline Is Sequential Thinking Dressed as Parallelism

```
implement → unit-test → coverage → lint → static → integration → security → review → accept
```

This is a waterfall pipeline. "Parallel" means running lint while coverage runs — that's CI parallelism, not AI parallelism. Human developers don't write code in 9 sequential phases.

**What 100x AI development actually looks like**:
- Agent writes code AND tests simultaneously (not test-first-then-implement as separate phases)
- Linting and static analysis happen in the agent's tool loop, not as a separate pipeline stage
- Review is continuous feedback, not a gate after implementation
- The unit of parallelism is *beads* (tasks), not *stages within a bead*

The current design adds latency (9 sequential stages per bead) rather than removing it. A single agent with good tools could do implement+test+lint in one pass.

### Flaw 5: No Feedback Loop Between Agent Output and Orchestrator

The architecture describes a one-way flow: Scheduler → assign bead → Worker → execute pipeline → report result. But AI agents need **iterative feedback**:

- Test fails → agent reads failure → agent fixes code → test again
- Review rejects → feedback sent → agent revises → review again
- Compilation error → agent reads error → agent fixes → compile again

This inner loop is where 100x happens. The current DAG model treats beads as atomic units that succeed or fail. Real AI development is 10-50 iterations per bead. The orchestrator needs to support **intra-bead iteration** natively, not just inter-bead scheduling.

### Flaw 6: SurrealDB as Primary Store Is a Risk

SurrealDB is used for everything: event store, bead state, rate limiting, checkpoints, worker assignments, schedules, webhooks. It's the single point of failure.

Concerns:
- SurrealDB is a young database (v2) with limited production track record at scale
- The `kv-rocksdb` backend is good, but SurrealDB's query layer adds overhead
- 13 tables with complex joins for a system that fundamentally needs fast append-only event logs and key-value lookups
- Event sourcing workloads (append-heavy, sequential reads) are better served by purpose-built stores

For 100x throughput, the storage layer needs to handle potentially thousands of events per second from 13 concurrent agents. SurrealDB may work, but it's an unproven bet.

### Flaw 7: Scope Creep in Architecture Documentation

850+ beads of architecture documentation for a system that doesn't yet complete a single bead end-to-end. The docs describe:
- ML-based adaptive rate limiting
- Distributed execution with mDNS/etcd
- HashiCorp Vault integration
- Prometheus metrics
- Force-directed graph layouts
- Cherry-pick stages across versions

These are P2/P3 features documented at P0 detail level. The documentation-to-implementation ratio suggests the project is in an architecture astronaut phase. Every hour spent on ML rate limiting docs is an hour not spent on getting one agent to write one function.

---

## Part 3: What Would Actually Get to 100x

### Priority Stack (in order)

1. **Build one working agent loop**: One LLM agent that can take a bead specification, read relevant code, write an implementation, run tests, iterate on failures, and land the change. This is the atomic unit. Without it, nothing else matters.

2. **Context retrieval system**: AST-aware code indexing that gives each agent the minimal relevant context for its bead. This is the multiplier. Good context = fewer iterations = faster completion.

3. **Intra-bead iteration protocol**: The orchestrator needs to support agent retry loops natively. Not "bead failed, re-queue from scratch" but "agent is on iteration 7 of 15, test still failing on edge case X."

4. **Replace file handoffs with event-sourced transitions**: Agent communication through EventBus, not `/tmp/` files. This gets you the reliability guarantees the rest of the architecture already provides.

5. **Fix the IPC async bridge**: Spawn a dedicated thread for `IpcTransport` blocking I/O, bridge to actors via channels. Add request correlation IDs.

6. **Then parallelize**: Once one agent works reliably, run N agents on N beads. The orchestrator/scheduler/DAG infrastructure you've already built handles this.

7. **Then add the rest**: Rate limiting, metrics, distributed execution, etc. — these are scaling features for a system that works, not prerequisites.

---

## Summary

| Area | Verdict |
|------|---------|
| **IPC Transport** | Solid byte-level protocol. Fix async bridging, add request IDs, deduplicate types. |
| **Orchestration** | Over-built for current needs. Erlang patterns + Restate patterns are good long-term, but no agent actually uses them yet. |
| **Agent System** | Non-existent. This is the critical gap. |
| **Storage** | Functional but risky SurrealDB bet. Works for now. |
| **Documentation** | Comprehensive but premature. Describes a system 10x more complex than what's built. |
| **Path to 100x** | Build one agent loop → context retrieval → iteration protocol → parallelize. The plumbing exists; the engine doesn't. |

The architecture's bones are good. The Erlang-inspired supervision, event sourcing, and DAG scheduling will pay off at scale. But the project needs to invert its priority: **build the agent that does the work first, then orchestrate many of them**.
