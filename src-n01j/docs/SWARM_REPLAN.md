# Swarm Architecture Replan - Using Full OYA Application

## Architecture Decision

**Build swarm as part of the existing `crates/orchestrator/` infrastructure**, leveraging:
- Ractor actor framework and supervision trees
- Existing AgentSwarm patterns for pool management
- SurrealDB persistence layer for work queue
- CLI as orchestration interface only

## Component Mapping

### Existing Infrastructure to Leverage

| Location | Component | Purpose |
|----------|-----------|---------|
| `crates/orchestrator/src/actors/supervisor/` | SupervisorActor | One-for-one supervision for swarm agents |
| `crates/orchestrator/src/agent_swarm/` | AgentPool, AgentHandle | Manage agent lifecycle and health |
| `crates/orchestrator/src/persistence/bead_store.rs` | BeadStore | Persist bead state in SurrealDB |
| `crates/orchestrator/src/persistence/` | CheckpointStore | Recovery and resume capability |
| `crates/orchestrator/src/actors/` | UniverseSupervisor | Root supervision for swarm hierarchy |

## New Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    UniverseSupervisor                          │
│  (existing root supervisor in orchestrator crate)              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  SwarmOrchestratorActor                         │
│  - NEW: Top-level swarm coordinator                            │
│  - Spawns SwarmSupervisor (manages 13 agents)                  │
│  - Connects to BeadStore for persistence                       │
│  - Monitors completion target (25 beads)                       │
│  - Handles start/stop/status commands from CLI                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   SwarmSupervisor                               │
│  - NEW: Supervises 13 swarm agents                             │
│  - One-for-one supervision strategy                            │
│  - Automatic restart on agent failure                          │
│  - Meltdown detection (too many failures)                     │
└─────────────────────────────────────────────────────────────────┘
           │           │           │           │           │
           ▼           ▼           ▼           ▼           ▼
    ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
    │   Test  │ │   Test  │ │   Test  │ │   Test  │ │ Planner │
    │ Writer 1│ │ Writer 2│ │ Writer 3│ │ Writer 4│ │         │
    └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘
           │           │           │           │           │
           ▼           ▼           ▼           ▼           ▼
    ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
    │  Imp 1  │ │  Imp 2  │ │  Imp 3  │ │  Imp 4  │
    └─────────┘ └─────────┘ └─────────┘ └─────────┘
           │           │           │           │
           ▼           ▼           ▼           ▼
    ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
    │ Rev 1   │ │ Rev 2   │ │ Rev 3   │ │ Rev 4   │
    └─────────┘ └─────────┘ └─────────┘ └─────────┘

                              │
                              ▼
                    ┌─────────────────┐
                    │   SurrealDB     │
                    │   (Persistence) │
                    │  - beads table  │
                    │  - swarm_state  │
                    │  - contracts    │
                    └─────────────────┘
```

## Data Model (SurrealDB)

### Tables

**beads** (extend existing)
```surreal
{
    id: "src-abc",
    title: "Implement feature X",
    status: "pending",  // pending, in_progress, complete, failed
    assigned_to: "test-writer-1",
    phase: "contract",  // contract, implementation, review
    contract: {
        error_variants: [...],
        preconditions: [...],
        postconditions: [...],
        test_plan: "..."
    },
    workspace: "zjj-session-123",
    test_results: {...},
    commit_hash: "abc123",
    retry_count: 0,
    created_at: <datetime>,
    updated_at: <datetime>
}
```

**swarm_state** (new)
```surreal
{
    id: "swarm-1",
    status: "running",  // starting, running, stopping, stopped, completed
    target_beads: 25,
    landed_beads: 5,
    active_agents: {
        test_writers: 4,
        implementers: 4,
        reviewers: 4,
        planner: 1
    },
    started_at: <datetime>,
    estimated_completion: <datetime>
}
```

## Message Protocol

### CLI → SwarmOrchestratorActor

```rust
pub enum SwarmCommand {
    Start {
        target_beads: usize,
        test_writers: usize,
        implementers: usize,
        reviewers: usize,
        planner: bool,
        reply: oneshot::Sender<SwarmStatus>,
    },
    Stop {
        graceful: bool,
        reply: oneshot::Sender<Result<(), SwarmError>>,
    },
    GetStatus {
        reply: oneshot::Sender<SwarmStatus>,
    },
}
```

### SwarmOrchestratorActor → SwarmSupervisor

```rust
pub enum SwarmSupervisorMessage {
    SpawnAgents {
        config: SwarmAgentConfig,
    },
    AgentFailed {
        agent_id: String,
        agent_type: SwarmAgentType,
        bead_id: Option<String>,
        error: String,
    },
    BeadComplete {
        bead_id: String,
        phase: BeadPhase,
        result: BeadResult,
    },
    Shutdown,
}
```

### TestWriterAgent Messages

```rust
pub enum TestWriterMessage {
    GetNextBead {
        reply: oneshot::Sender<Option<BeadWork>>,
    },
    WriteContract {
        bead_id: String,
        contract: TestContract,
    },
    ClaimBead {
        bead_id: String,
    },
}
```

## Agent Implementation

### File Structure

```
crates/orchestrator/src/
├── swarm/
│   ├── mod.rs                    # Swarm module exports
│   ├── orchestrator_actor.rs     # SwarmOrchestratorActor
│   ├── supervisor_actor.rs       # SwarmSupervisor
│   ├── agents/
│   │   ├── mod.rs                # Agent module
│   │   ├── test_writer.rs        # TestWriterActor
│   │   ├── implementer.rs        # ImplementerActor
│   │   ├── reviewer.rs           # ReviewerActor
│   │   └── planner.rs            # PlannerActor
│   ├── messages.rs               # Swarm message types
│   ├── config.rs                 # Swarm configuration
│   └── persistence.rs            # Swarm-specific persistence
└── lib.rs                        # Export swarm module
```

### TestWriterActor

```rust
pub struct TestWriterActor;

#[async_trait]
impl Actor for TestWriterActor {
    type Msg = TestWriterMessage;
    type State = TestWriterState;
    type Arguments = TestWriterArgs;

    async fn pre_start(&self, myself: ActorRef<Self::Msg>, args: Self::Arguments)
        -> Result<Self::State, ActorProcessingErr>
    {
        // Initialize bv client, br client
        // Load skills (rust-contract, functional-rust)
        Ok(TestWriterState::new(args))
    }

    async fn handle(&self, myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State)
        -> Result<(), ActorProcessingErr>
    {
        match message {
            TestWriterMessage::GetNextBead { reply } => {
                // Call bv --robot-triage
                // Parse JSON output
                // Send BeadWork back
            }
            TestWriterMessage::WriteContract { bead_id, contract } => {
                // Use rust-contract skill
                // Write to SurrealDB beads table
                // Mark bead as ready_for_implementation
            }
        }
        Ok(())
    }
}
```

## CLI Integration

### Command Handler (src/commands.rs)

```rust
async fn cmd_swarm(
    target: usize,
    test_writers: usize,
    implementers: usize,
    reviewers: usize,
    planner: bool,
    continuous_deployment: bool,
    dry_run: bool,
    resume: Option<String>,
    format: String,
) -> Result<()> {
    // Load config from .oya/swarm.toml
    let config = load_swarm_config()?;

    if dry_run {
        print_dry_run(&config);
        return Ok(());
    }

    // Connect to orchestrator (via IPC or spawn in-process)
    let orchestrator = connect_to_orchestrator().await?;

    // Send Start command
    let (tx, rx) = oneshot::channel();
    orchestrator.send(SwarmCommand::Start {
        target_beads: target,
        test_writers,
        implementers,
        reviewers,
        planner,
        reply: tx,
    })?;

    // Wait for acknowledgement
    let status = rx.await?;

    // Monitor progress
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let swarm_status = get_swarm_status(&orchestrator).await?;

        print_progress(&swarm_status);

        if swarm_status.landed_beads >= swarm_status.target_beads {
            break;
        }
    }

    Ok(())
}
```

## Implementation Phases

### Phase 1: Persistence Layer (Priority 1)
1. Extend `beads` table schema for swarm fields
2. Create `swarm_state` table
3. Implement `SwarmStore` in `crates/orchestrator/src/persistence/swarm_store.rs`
4. Add migrations for new tables

### Phase 2: Message Protocol (Priority 1)
1. Create `crates/orchestrator/src/swarm/messages.rs`
2. Define `SwarmCommand`, `SwarmSupervisorMessage`
3. Define agent-specific messages
4. Add status/query message types

### Phase 3: SwarmOrchestratorActor (Priority 2)
1. Create `crates/orchestrator/src/swarm/orchestrator_actor.rs`
2. Implement Start/Stop/GetStatus handlers
3. Connect to SurrealDB persistence
4. Spawn SwarmSupervisor
5. Track completion target

### Phase 4: SwarmSupervisor (Priority 2)
1. Create `crates/orchestrator/src/swarm/supervisor_actor.rs`
2. Extend existing SupervisorActor for swarm agents
3. Implement one-for-one supervision
4. Add meltdown detection
5. Handle agent failures and restart

### Phase 5: Agent Implementations (Priority 3)
1. **TestWriterActor** (`test_writer.rs`)
   - bv --robot-triage integration
   - rust-contract skill invocation
   - Contract storage in SurrealDB
2. **ImplementerActor** (`implementer.rs`)
   - Poll for ready beads
   - zjj spawn workspace
   - moon ci gates
   - continuous-deployment enforcement
3. **ReviewerActor** (`reviewer.rs`)
   - red-queen QA
   - moon quick gates
   - landing skill
   - zjj done workspace
4. **PlannerActor** (`planner.rs`)
   - Contract coordination
   - Martin Fowler test philosophy

### Phase 6: CLI Integration (Priority 4)
1. Add swarm commands to existing orchestrator CLI
2. Implement status monitoring
3. Add signal handling (Ctrl+C)
4. JSON output support

### Phase 7: Testing (Priority 5)
1. Unit tests for each agent type
2. Integration tests for full flow
3. Chaos tests (agent failures)
4. Performance tests

## Key Differences from Original Plan

| Aspect | Original Plan | New Plan |
|--------|--------------|----------|
| **Actor System** | New ractor actors in src/swarm/ | Extend orchestrator crate |
| **Supervision** | Custom implementation | Use existing SupervisorActor |
| **Persistence** | File-based handoffs in /tmp/ | SurrealDB tables |
| **Agent Pool** | Custom WorkQueue | Use existing AgentPool |
| **CLI** | Direct orchestration | Send commands to orchestrator |
| **State** | In-memory + files | Database backed with recovery |
| **Monitoring** | File polling | Actor message queries |
| **Failure Recovery** | Handoff file cleanup | Database state + supervisor restart |

## Benefits of New Architecture

1. **Leverages Existing Infrastructure**
   - Reuses battle-tested supervision patterns
   - Built on proven actor framework
   - Uses existing persistence layer

2. **Better Scalability**
   - Database-backed state survives restarts
   - Can monitor multiple swarms simultaneously
   - Easier to add new agent types

3. **Cleaner Separation**
   - CLI is thin (orchestration only)
   - All logic in orchestrator crate
   - Persistence layer abstracts storage

4. **Production Ready**
   - SurrealDB provides ACID guarantees
   - Supervision trees handle failures
   - Built-in recovery and resume

## Next Steps

1. **Confirm architecture** with stakeholders
2. **Design database schema** for swarm tables
3. **Define message protocol** between actors
4. **Implement persistence layer** (SwarmStore)
5. **Create SwarmOrchestratorActor**
6. **Implement agents one by one**
7. **Wire up CLI commands**
8. **Test and iterate**

This approach builds on the solid foundation of the orchestrator crate rather than creating a parallel system.
