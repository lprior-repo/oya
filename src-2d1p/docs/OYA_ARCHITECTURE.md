# oya - Full-Stack Rust SDLC System

> 100x developer throughput - rough idea to production-quality software in hours.

## Vision

100 concurrent beads, AI agent swarms, ~100k LOC/hour generation capacity. One Rust monorepo. Zero external workflow dependencies. Railway-oriented programming throughout.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           oya MONOREPO                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                   │
│  │ oya-  │     │ oya-  │     │ oya-  │                   │
│  │    cli      │────▶│   intent    │────▶│   tdd15     │                   │
│  └─────────────┘     └─────────────┘     └─────────────┘                   │
│         │                   │                   │                           │
│         ▼                   ▼                   ▼                           │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      oya-workflow                              │   │
│  │  (Intra-bead: TDD15 phases, checkpoints, rewind, journal replay)    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      oya-events                                │   │
│  │  (Inter-bead: Event sourcing, pub/sub, coordination)                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     oya-reconciler                             │   │
│  │  (K8s pattern: desired state → reconcile → actual state)            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│         │                                                                   │
│         ├───────────────┬───────────────┬───────────────┐                  │
│         ▼               ▼               ▼               ▼                  │
│  ┌───────────┐   ┌───────────┐   ┌───────────┐   ┌───────────┐            │
│  │oya- │   │oya- │   │oya- │   │oya- │            │
│  │   zjj     │   │   docs    │   │  opencode │   │    ui     │            │
│  │(Zellij)   │   │           │   │           │   │(Plugin)   │            │
│  └───────────┘   └───────────┘   └───────────┘   └───────────┘            │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       oya-core                                 │   │
│  │  (Types, errors, Result extensions, Railway-Oriented primitives)    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          STORAGE LAYER                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│  L1 Hot:    papaya (lock-free HashMap)           ~10-50ns                  │
│  L2 Cache:  moka (TinyLFU)                       ~100ns                    │
│  L3 State:  SurrealDB (embedded/remote)          ~100µs-1ms                │
│  L4 Graph:  SurrealDB graph relations            ~1-5ms                    │
│  L5 Vector: SurrealDB vector search              ~5-20ms                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Crate Structure

```
oya/
├── Cargo.toml                    # Workspace root
├── rust-toolchain.toml           # Pin Rust version
├── .cargo/config.toml            # Build optimizations
│
├── crates/
│   ├── oya-core/           # Foundation types (~500 LOC)
│   ├── oya-workflow/       # Intra-bead engine (~2.5k LOC)
│   ├── oya-events/         # Inter-bead coordination (~2k LOC)
│   ├── oya-reconciler/     # K8s-style reconciliation (~1.5k LOC)
│   ├── oya-intent/         # Requirement decomposition (~8k LOC)
│   ├── oya-tdd15/          # TDD15 phase machine (~1k LOC)
│   ├── oya-zjj/            # Workspace isolation (~2k LOC)
│   ├── oya-docs/           # Documentation indexing (~15k LOC)
│   ├── oya-opencode/       # AI execution bridge (~1k LOC)
│   └── oya-cli/            # Unified CLI (~1k LOC)
│
├── app/
│   └── oya-ui/             # Zellij plugin (WASM) (~3k LOC)
│
└── tests/
    └── integration/              # Cross-crate integration tests
```

**Total Estimated: ~40k LOC**

---

## Crate Specifications

### oya-core (~500 LOC)

Foundation types used across all crates. Zero external deps beyond std.

```rust
// Core error type with Railway-Oriented extensions
pub enum oyaError {
    Workflow(WorkflowError),
    Event(EventError),
    Storage(StorageError),
    Intent(IntentError),
    Reconcile(ReconcileError),
    External(ExternalError),
}

// Result alias
pub type Result<T> = std::result::Result<T, oyaError>;

// Railway extensions
pub trait ResultExt<T, E> {
    fn tap<F: FnOnce(&T)>(self, f: F) -> Self;
    fn tap_err<F: FnOnce(&E)>(self, f: F) -> Self;
    fn and_then_async<U, F, Fut>(self, f: F) -> impl Future<Output = Result<U>>
    where
        F: FnOnce(T) -> Fut,
        Fut: Future<Output = Result<U>>;
}

// Core identifiers
pub struct BeadId(Ulid);
pub struct PhaseId(Ulid);
pub struct WorkflowId(Ulid);
pub struct EventId(Ulid);

// 8-state lifecycle (from nuoc design)
pub enum BeadState {
    Pending,      // Waiting for dependencies
    Scheduled,    // Ready to be claimed
    Ready,        // Claimed, about to run
    Running,      // Actively executing
    Suspended,    // Paused by user
    BackingOff,   // Waiting after failure
    Paused,       // System pause (resource constraint)
    Completed,    // Terminal: success or failure
}

// Transition validation
impl BeadState {
    pub fn can_transition_to(&self, target: &BeadState) -> bool;
    pub fn valid_transitions(&self) -> &[BeadState];
}
```

**Clippy enforcement:**
```rust
#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]
```

---

### oya-workflow (~2.5k LOC)

Intra-bead workflow engine. Manages TDD15 phases within a single bead.

```rust
// Phase definition
pub struct Phase {
    pub id: PhaseId,
    pub name: String,
    pub handler: Box<dyn PhaseHandler>,
    pub timeout: Duration,
    pub retries: u32,
}

// Phase handler trait
#[async_trait]
pub trait PhaseHandler: Send + Sync {
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput>;
    async fn rollback(&self, ctx: &PhaseContext) -> Result<()>;
    fn checkpoint_data(&self) -> Option<Vec<u8>>;
}

// Workflow definition
pub struct Workflow {
    pub id: WorkflowId,
    pub phases: Vec<Phase>,
    pub current_phase: usize,
    pub checkpoints: Vec<Checkpoint>,
    pub journal: Journal,
}

// Checkpoint for rewind capability
pub struct Checkpoint {
    pub phase_id: PhaseId,
    pub timestamp: DateTime<Utc>,
    pub state: Vec<u8>,  // rkyv serialized
    pub inputs: Vec<u8>,
    pub outputs: Option<Vec<u8>>,
}

// Journal for replay (event sourcing within bead)
pub struct Journal {
    entries: Vec<JournalEntry>,
}

pub enum JournalEntry {
    PhaseStarted { phase_id: PhaseId, timestamp: DateTime<Utc> },
    PhaseCompleted { phase_id: PhaseId, output: Vec<u8>, timestamp: DateTime<Utc> },
    PhaseFailed { phase_id: PhaseId, error: String, timestamp: DateTime<Utc> },
    CheckpointCreated { checkpoint: Checkpoint },
    RewindInitiated { to_phase: PhaseId, reason: String },
}

// Workflow engine
pub struct WorkflowEngine {
    storage: Arc<dyn WorkflowStorage>,
    executor: Arc<dyn PhaseExecutor>,
}

impl WorkflowEngine {
    pub async fn run(&self, workflow: Workflow) -> Result<WorkflowResult>;
    pub async fn rewind(&self, workflow_id: WorkflowId, to_phase: PhaseId) -> Result<()>;
    pub async fn replay(&self, workflow_id: WorkflowId) -> Result<WorkflowResult>;
    pub async fn checkpoint(&self, workflow_id: WorkflowId) -> Result<Checkpoint>;
}
```

**Key features:**
- Phase-based execution with typed handlers
- Automatic checkpointing at phase boundaries
- Rewind to any previous checkpoint
- Journal replay for debugging/recovery
- Timeout and retry per phase

---

### oya-events (~2k LOC)

Inter-bead coordination via event sourcing.

```rust
// Event types
pub enum BeadEvent {
    Created { bead_id: BeadId, spec: BeadSpec },
    StateChanged { bead_id: BeadId, from: BeadState, to: BeadState },
    PhaseCompleted { bead_id: BeadId, phase_id: PhaseId, output: PhaseOutput },
    DependencyResolved { bead_id: BeadId, dependency_id: BeadId },
    Failed { bead_id: BeadId, error: oyaError },
    Completed { bead_id: BeadId, result: BeadResult },
}

// Event store (append-only)
#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: BeadEvent) -> Result<EventId>;
    async fn read(&self, from: EventId) -> Result<Vec<BeadEvent>>;
    async fn read_for_bead(&self, bead_id: BeadId) -> Result<Vec<BeadEvent>>;
    async fn subscribe(&self) -> Result<EventSubscription>;
}

// Subscription for real-time updates
pub struct EventSubscription {
    receiver: tokio::sync::broadcast::Receiver<BeadEvent>,
}

// Event bus for pub/sub
pub struct EventBus {
    store: Arc<dyn EventStore>,
    subscribers: DashMap<String, Vec<Sender<BeadEvent>>>,
}

impl EventBus {
    pub async fn publish(&self, event: BeadEvent) -> Result<EventId>;
    pub async fn subscribe(&self, pattern: &str) -> Result<Receiver<BeadEvent>>;
    pub async fn replay_from(&self, event_id: EventId) -> Result<Vec<BeadEvent>>;
}

// Projections (materialized views from events)
pub struct BeadProjection {
    pub bead_id: BeadId,
    pub current_state: BeadState,
    pub current_phase: Option<PhaseId>,
    pub dependencies: Vec<BeadId>,
    pub blocked_by: Vec<BeadId>,
    pub history: Vec<StateTransition>,
}

#[async_trait]
pub trait Projection: Send + Sync {
    type State;
    fn apply(&self, state: &mut Self::State, event: &BeadEvent);
    async fn rebuild(&self, store: &dyn EventStore) -> Result<Self::State>;
}
```

**Key features:**
- Append-only event store (SQLite + FalkorDB for graph queries)
- Pub/sub for real-time coordination
- Projections for materialized views
- Full replay capability

---

### oya-reconciler (~1.5k LOC)

K8s-style reconciliation loop for bead management.

```rust
// Desired state declaration
pub struct DesiredState {
    pub beads: HashMap<BeadId, BeadSpec>,
    pub dependencies: HashMap<BeadId, Vec<BeadId>>,
}

// Actual state (computed from events)
pub struct ActualState {
    pub beads: HashMap<BeadId, BeadProjection>,
    pub running_count: usize,
    pub pending_count: usize,
}

// Reconciler actions
pub enum ReconcileAction {
    CreateBead(BeadSpec),
    StartBead(BeadId),
    StopBead(BeadId),
    RetryBead(BeadId),
    MarkComplete(BeadId, BeadResult),
    UpdateDependencies(BeadId, Vec<BeadId>),
}

// Reconciler
pub struct Reconciler {
    event_bus: Arc<EventBus>,
    executor: Arc<BeadExecutor>,
    max_concurrent: usize,
}

impl Reconciler {
    /// Core reconciliation loop
    pub async fn reconcile(&self, desired: &DesiredState) -> Result<Vec<ReconcileAction>> {
        let actual = self.compute_actual_state().await?;
        let actions = self.diff(desired, &actual);
        self.apply_actions(actions).await
    }

    /// Compute diff between desired and actual
    fn diff(&self, desired: &DesiredState, actual: &ActualState) -> Vec<ReconcileAction>;

    /// Apply actions with concurrency control
    async fn apply_actions(&self, actions: Vec<ReconcileAction>) -> Result<Vec<ReconcileAction>>;
}

// Continuous reconciliation
pub struct ReconciliationLoop {
    reconciler: Arc<Reconciler>,
    interval: Duration,
    stop_signal: tokio::sync::watch::Receiver<bool>,
}

impl ReconciliationLoop {
    pub async fn run(&self) -> Result<()> {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.interval) => {
                    self.reconciler.reconcile(&self.get_desired_state().await?).await?;
                }
                _ = self.stop_signal.changed() => break,
            }
        }
        Ok(())
    }
}
```

**Key features:**
- Declarative desired state
- Computed actual state from event projections
- Diff-based action generation
- Continuous reconciliation loop
- Concurrency limits respected

---

### oya-tdd15 (~1k LOC)

TDD15 phase definitions and routing logic.

```rust
// TDD15 phases
pub enum TDD15Phase {
    Triage,           // 1. Assess complexity
    Research,         // 2. Gather context
    Plan,             // 3. Design approach
    Verify,           // 4. Validate plan
    Red,              // 5. Write failing test
    Green,            // 6. Minimal implementation
    Refactor,         // 7. Clean up
    MutationFirst,    // 8. Mutation testing
    Implement,        // 9. Full implementation
    VerifyCriteria,   // 10. Check acceptance
    FPGates,          // 11. Functional programming checks
    QA,               // 12. Quality assurance
    MutationSecond,   // 13. Final mutation testing
    Consistency,      // 14. Style/pattern check
    Liability,        // 15. Security/legal review
    Landing,          // 16. Merge preparation
}

// Complexity routing
pub enum ComplexityRoute {
    Simple,   // Skip: Research, Plan, MutationFirst
    Medium,   // Skip: Research
    Complex,  // Full 16 phases
}

impl TDD15Phase {
    pub fn phases_for_route(route: ComplexityRoute) -> Vec<TDD15Phase>;
    pub fn next_phase(&self, route: ComplexityRoute) -> Option<TDD15Phase>;
    pub fn is_skipped(&self, route: ComplexityRoute) -> bool;
}

// Phase handlers
pub struct TriageHandler;
pub struct ResearchHandler;
pub struct PlanHandler;
// ... etc

// TDD15 workflow builder
pub struct TDD15WorkflowBuilder {
    bead_spec: BeadSpec,
    route: ComplexityRoute,
}

impl TDD15WorkflowBuilder {
    pub fn new(bead_spec: BeadSpec) -> Self;
    pub fn with_route(mut self, route: ComplexityRoute) -> Self;
    pub fn build(self) -> Workflow;
}
```

---

### oya-intent (~8k LOC)

Port of intent-cli. Requirement decomposition with EARS and KIRK.

```rust
// EARS pattern types
pub enum EARSPattern {
    Ubiquitous { requirement: String },
    EventDriven { when: String, the_system: String, shall: String },
    StateDriven { while_in: String, the_system: String, shall: String },
    Optional { where_condition: String, the_system: String, shall: String },
    Unwanted { if_condition: String, the_system: String, shall: String },
    Complex { patterns: Vec<EARSPattern> },
}

// KIRK analysis
pub struct KIRKAnalysis {
    pub coverage: CoverageScore,      // How complete
    pub quality: QualityScore,        // How well-formed
    pub gaps: Vec<Gap>,               // What's missing
    pub inversion: Vec<Inversion>,    // Negation analysis
    pub effects: Vec<Effect>,         // Side effects
    pub empathy: EmpathyScore,        // User perspective
    pub readiness: ReadinessScore,    // Implementation readiness
}

// Bead generation
pub struct BeadSpec {
    pub id: BeadId,
    pub title: String,
    pub requirements: Vec<EARSPattern>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub dependencies: Vec<BeadId>,
    pub complexity: ComplexityRoute,
    pub kirk_analysis: KIRKAnalysis,
}

// Intent decomposer
pub struct IntentDecomposer {
    llm: Arc<dyn LLMClient>,
}

impl IntentDecomposer {
    pub async fn decompose(&self, intent: &str) -> Result<Vec<BeadSpec>>;
    pub async fn analyze_ears(&self, requirement: &str) -> Result<EARSPattern>;
    pub async fn analyze_kirk(&self, specs: &[BeadSpec]) -> Result<KIRKAnalysis>;
}
```

---

### oya-zjj (~2k LOC)

Move existing zjj. Workspace isolation: 1 bead = 1 jj worktree + Zellij session.

```rust
// Already battle-tested - minimal changes needed
pub struct Workspace {
    pub id: WorkspaceId,
    pub bead_id: BeadId,
    pub jj_worktree: PathBuf,
    pub zellij_session: String,
    pub status: WorkspaceStatus,
}

pub struct WorkspaceManager {
    storage: Arc<dyn WorkspaceStorage>,  // SQLite
}

impl WorkspaceManager {
    pub async fn create(&self, bead_id: BeadId) -> Result<Workspace>;
    pub async fn attach(&self, workspace_id: WorkspaceId) -> Result<()>;
    pub async fn detach(&self, workspace_id: WorkspaceId) -> Result<()>;
    pub async fn destroy(&self, workspace_id: WorkspaceId) -> Result<()>;
    pub async fn list(&self) -> Result<Vec<Workspace>>;
}
```

---

### oya-docs (~15k LOC)

Move centralized-docs. Documentation indexing v5.0.

```rust
// Already production-ready - wrap as crate
pub struct DocIndex {
    tantivy: TantivyIndex,      // Full-text search
    hnsw: HNSWIndex,            // Semantic search
    graph: PetGraph,            // Document relationships
}

impl DocIndex {
    pub async fn index(&self, doc: Document) -> Result<DocId>;
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>>;
    pub async fn semantic_search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>>;
    pub async fn related(&self, doc_id: DocId) -> Result<Vec<DocId>>;
}
```

---

### oya-opencode (~1k LOC)

Bridge to opencode for AI execution.

```rust
// Opencode client
pub struct OpencodeClient {
    base_url: Url,
    client: reqwest::Client,
}

impl OpencodeClient {
    pub async fn execute(&self, prompt: &str) -> Result<ExecutionResult>;
    pub async fn stream(&self, prompt: &str) -> Result<impl Stream<Item = StreamChunk>>;
}

// AI executor for phases
pub struct AIExecutor {
    opencode: Arc<OpencodeClient>,
}

#[async_trait]
impl PhaseHandler for AIExecutor {
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
        let prompt = ctx.generate_prompt();
        let result = self.opencode.execute(&prompt).await?;
        Ok(PhaseOutput::from(result))
    }
}
```

---

### oya-cli (~1k LOC)

Unified CLI with JSON-native output.

```rust
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, default_value = "false")]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Command {
    // Intent decomposition
    Intent {
        #[command(subcommand)]
        cmd: IntentCmd,
    },
    // Bead management
    Bead {
        #[command(subcommand)]
        cmd: BeadCmd,
    },
    // Workspace management
    Workspace {
        #[command(subcommand)]
        cmd: WorkspaceCmd,
    },
    // Documentation
    Docs {
        #[command(subcommand)]
        cmd: DocsCmd,
    },
    // Status and monitoring
    Status,
}

// JSON output schema (AI-native)
#[derive(Serialize)]
pub struct CliOutput<T> {
    pub success: bool,
    pub action: String,
    pub data: T,
    pub errors: Vec<CliError>,
    pub next_actions: Vec<NextAction>,
    pub metadata: Metadata,
}
```

---

### oya-ui (~3k LOC)

**Zellij terminal UI plugin** (WASM-based) with "video game UX".

```
crates/oya-ui/
├── src/
│   ├── main.rs           # Zellij plugin entry point (WASM)
│   ├── layout.rs         # UI layout definitions
│   ├── ipc/              # Zellij IPC protocol
│   │   ├── mod.rs        # Host ↔ Guest messages
│   │   ├── protocol.rs   # Message types (bincode)
│   │   └── transport.rs  # Zellij buffer I/O
│   ├── components/       # Reusable UI components
│   │   ├── bead_board.rs     # Kanban-style bead view
│   │   ├── workflow_graph.rs # DAG visualization (ANSI)
│   │   ├── phase_timeline.rs # TDD15 progress
│   │   └── event_stream.rs   # Real-time events
│   └── render.rs         # Terminal rendering logic
└── Cargo.toml
```

**Architecture:**
- **Guest**: Zellij plugin compiled to WASM
- **Host**: Orchestrator runs in separate Rust binary
- **Transport**: Zellij IPC via stdin/stdout buffers (extremely fast)
- **Protocol**: Bincode-serialized messages (~1-10µs latency)
- **Real-time**: Bidirectional streaming (commands → host, events ← host)

```
┌─────────────────────────────────────┐
│   Host: oya-orchestrator (Rust)     │
│   - Actors, DAG, scheduling         │
│   - IPC worker for Zellij comms     │
└──────────┬──────────────────────────┘
           │ Zellij IPC (bincode over buffers)
           │ ~1-10µs latency
           ▼
┌─────────────────────────────────────┐
│   Guest: Zellij Plugin (WASM)       │
│   - Renders UI to terminal          │
│   - Sends commands to host          │
│   - Receives events for re-render   │
└─────────────────────────────────────┘
```

---

## Storage Configuration

### SurrealDB (L3 State)

```surreal
-- Beads table
DEFINE TABLE beads SCHEMAFULL;
DEFINE FIELD id ON beads TYPE record<beads>;
DEFINE FIELD spec ON beads TYPE object;
DEFINE FIELD state ON beads TYPE string;
DEFINE FIELD current_phase ON beads TYPE option<string>;
DEFINE FIELD created_at ON beads TYPE datetime;
DEFINE FIELD updated_at ON beads TYPE datetime;
DEFINE INDEX beads_state ON beads FIELDS state;

-- Events table (append-only)
DEFINE TABLE events SCHEMAFULL;
DEFINE FIELD id ON events TYPE record<events>;
DEFINE FIELD bead_id ON events TYPE record<beads>;
DEFINE FIELD event_type ON events TYPE string;
DEFINE FIELD payload ON events TYPE object;
DEFINE FIELD timestamp ON events TYPE datetime;
DEFINE INDEX events_bead ON events FIELDS bead_id;
DEFINE INDEX events_time ON events FIELDS timestamp;
DEFINE INDEX events_type ON events FIELDS event_type;

-- Checkpoints table
DEFINE TABLE checkpoints SCHEMAFULL;
DEFINE FIELD id ON checkpoints TYPE record<checkpoints>;
DEFINE FIELD workflow_id ON checkpoints TYPE record<workflows>;
DEFINE FIELD phase_id ON checkpoints TYPE string;
DEFINE FIELD state ON checkpoints TYPE object;
DEFINE FIELD timestamp ON checkpoints TYPE datetime;
DEFINE INDEX checkpoints_workflow ON checkpoints FIELDS workflow_id;

-- Journal table
DEFINE TABLE journal SCHEMAFULL;
DEFINE FIELD id ON journal TYPE record<journal>;
DEFINE FIELD workflow_id ON journal TYPE record<workflows>;
DEFINE FIELD entry_type ON journal TYPE string;
DEFINE FIELD payload ON journal TYPE object;
DEFINE FIELD timestamp ON journal TYPE datetime;
DEFINE INDEX journal_workflow ON journal FIELDS workflow_id;

-- Tasks table (system)
DEFINE TABLE tasks SCHEMAFULL;
DEFINE FIELD id ON tasks TYPE record<tasks>;
DEFINE FIELD slug ON tasks TYPE string;
DEFINE FIELD language ON tasks TYPE string;
DEFINE FIELD status ON tasks TYPE string;
DEFINE FIELD priority ON tasks TYPE string;
DEFINE FIELD worktree_path ON tasks TYPE string;
DEFINE FIELD branch ON tasks TYPE string;
DEFINE FIELD created_at ON tasks TYPE datetime;
DEFINE FIELD updated_at ON tasks TYPE datetime;
DEFINE FIELD stages ON tasks TYPE array<object>;
DEFINE INDEX tasks_slug ON tasks FIELDS slug UNIQUE;
DEFINE INDEX tasks_status ON tasks FIELDS status;
DEFINE INDEX tasks_priority ON tasks FIELDS priority;
```

### SurrealDB Graph Relations (L4 Graph)

```surreal
-- Bead dependency graph using relations
DEFINE TABLE depends_on SCHEMAFULL TYPE RELATION IN beads OUT beads;
DEFINE FIELD blocked_count ON depends_on TYPE int DEFAULT 0;

-- Create dependency relationship
RELATE beads:from->depends_on->beads:to;

-- Query blocked beads
SELECT id, ->depends_on->beads AS dependencies
FROM beads
WHERE state = 'pending'
AND ->depends_on->beads[WHERE state != 'completed'] != NONE;

-- Critical path analysis (recursive graph traversal)
SELECT *,
    <-depends_on<-beads AS blockers,
    ->depends_on->beads AS dependencies,
    count(->depends_on->beads) AS dep_count
FROM beads
ORDER BY dep_count DESC;
```

---

## Performance Targets

| Operation | Target | Implementation |
|-----------|--------|----------------|
| Hot cache read | <50ns | papaya |
| Warm cache read | <100ns | moka |
| State read | <1ms | SurrealDB |
| State write | <2ms | SurrealDB async persist |
| Graph query | <5ms p50 | SurrealDB graph relations |
| Bead startup | <100ms | Pre-warmed pools |
| Phase transition | <10ms | In-memory + async persist |
| Event publish | <1ms | Broadcast + async store |

---

## Dependencies (Battle-Tested Only)

```toml
# Cargo.toml [workspace.dependencies]

# Async runtime
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# Storage
surrealdb = { version = "2", features = ["protocol-ws"] }
surrealdb-core = "2"

# Caching
moka = "0.12"
papaya = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rkyv = { version = "0.8", features = ["validation"] }

# FP primitives
im = "15.1"
itertools = "0.13"
either = "1.13"
frunk = "0.4"
tap = "1.0"

# Allocation
bumpalo = "3"
smallvec = "1"
arrayvec = "0.7"

# CLI
clap = { version = "4", features = ["derive"] }

# IDs
ulid = "1"
```

---

## 3-Phase Development Plan

### Phase 1: Foundation (Week 1-2)
- [ ] Create monorepo structure
- [ ] Implement oya-core
- [ ] Move zjj to oya-zjj
- [ ] Move centralized-docs to oya-docs
- [ ] Basic CLI scaffold

### Phase 2: Orchestration (Week 3-4)
- [ ] Implement oya-workflow
- [ ] Implement oya-events
- [ ] Implement oya-reconciler
- [ ] Implement oya-tdd15
- [ ] Integration tests

### Phase 3: Intelligence (Week 5-6)
- [ ] Port intent-cli to oya-intent
- [ ] Implement oya-opencode bridge
- [ ] Zellij WASM plugin with IPC protocol
- [ ] End-to-end workflow

---

## Next Actions

1. **Create workspace** - `cargo new oya --lib` with workspace config
2. **Move zjj** - Copy and adapt existing battle-tested code
3. **Write oya-core tests** - Farley Discipline: tests first
4. **Implement BeadState** - Core type with transition validation

Ready to execute.
