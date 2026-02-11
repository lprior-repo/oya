# Recursive DAG Orchestration Plan

**Purpose**: Transform Oya into a Claude Code CLI orchestrator with recursive intra-bead DAG, event-driven stage gates, IPC-controlled Zellij UI, and end-to-end agent slot management.

**Handoff**: Each numbered bead below is an independent work unit for an agent. Dependencies are explicit. Tests are contracts — write them RED first, then implement GREEN.

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        CLI (oya storm)                           │
│  Parses args → creates BeadOrchestrator → runs event loop       │
└──────────────┬───────────────────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────────────────┐
│                     BeadOrchestrator                             │
│  Owns: AgentSlotPool, SchedulerActor, EventBus, IpcBridge       │
│  Loop: poll ready beads → assign to idle slots → handle events  │
└──────┬──────────┬──────────────┬─────────────────┬──────────────┘
       │          │              │                 │
       ▼          ▼              ▼                 ▼
┌───────────┐ ┌────────────┐ ┌──────────┐ ┌──────────────────┐
│AgentSlot  │ │ Scheduler  │ │ EventBus │ │ IpcBridge        │
│[0..N]     │ │ Actor      │ │ (pub/sub)│ │ (→ Zellij UI)    │
│           │ │            │ │          │ │                  │
│Claude CLI │ │Inter-bead  │ │Drives    │ │Forwards events   │
│process    │ │DAG (acyclic│ │recursion │ │to oya-ui plugin  │
│per stage  │ │Kahn's)     │ │+ gates   │ │via IpcTransport  │
└───────────┘ └────────────┘ └──────────┘ └──────────────────┘
```

### Two-Level DAG

```
Level 1: INTER-BEAD DAG (acyclic, existing WorkflowDAG)
  BeadA ──→ BeadC ──→ BeadE
  BeadB ──→ BeadC
  BeadD (independent)

Level 2: INTRA-BEAD STATE MACHINE (cyclic, event-driven, per bead)
  Research ⇄ Plan ⇄ Implement ⇄ Review → Validate → Accept
  Transitions driven by StageEvents through EventBus
  Bounded by RecursionPolicy
```

---

## Beads (Work Units)

### Bead 1: `BeadStateMachine` (events crate)

**File**: `crates/events/src/stage.rs`
**Modify**: `crates/events/src/lib.rs` (add `pub mod stage;` and re-exports)
**Dependencies**: None (pure logic, no I/O)
**Crate**: `oya-events`

#### Types to Create

```
StageKind           enum: Research | Plan | Implement | Review | Validate | Accept
Severity            enum: Minor | Major | Fundamental
TransitionReason    enum: Completed | GateFailed { feedback: String, severity: Severity } | Timeout | ManualOverride { reason: String }
StageTransition     struct: { from: StageKind, to: StageKind, reason: TransitionReason, timestamp: DateTime<Utc> }
ExhaustionPolicy    enum: Fail | ParkForHuman
RecursionPolicy     struct: { max_total_attempts: u32, max_stage_retries: u32, max_research_retries: u32, on_exhaustion: ExhaustionPolicy }
StateMachineError   enum: TotalAttemptsExhausted | StageRetriesExhausted | AlreadyTerminal | InvalidReentry
BeadStateMachine    struct: { bead_id: BeadId, current_stage: StageKind, stage_attempts: [u32; 6], total_attempts: u32, history: Vec<StageTransition>, policy: RecursionPolicy }
```

#### Methods

```
StageKind::next(self) -> Option<StageKind>              // forward progression
StageKind::requires_agent(self) -> bool                  // Research/Plan/Implement/Review = true, Validate/Accept = false
StageKind::is_terminal(self) -> bool                     // Accept only
BeadStateMachine::new(bead_id) -> Self                   // starts at Research
BeadStateMachine::with_policy(bead_id, policy) -> Self
BeadStateMachine::current_stage(&self) -> StageKind
BeadStateMachine::enter_stage(&mut self) -> Result<(), StateMachineError>   // record attempt, check bounds
BeadStateMachine::advance(&mut self) -> Result<StageTransition, StateMachineError>  // move forward
BeadStateMachine::reenter(&mut self, target, feedback, severity) -> Result<StageTransition, StateMachineError>  // loop back
BeadStateMachine::reentry_target_for_severity(severity) -> StageKind  // Minor→Implement, Major→Plan, Fundamental→Research
BeadStateMachine::is_complete(&self) -> bool
BeadStateMachine::history(&self) -> &[StageTransition]
BeadStateMachine::stage_attempts(&self, stage) -> u32
BeadStateMachine::total_attempts(&self) -> u32
```

#### Tests (24 tests)

```
// Construction & initial state
test_new_starts_at_research                          // current_stage() == Research, total_attempts() == 0
test_new_is_not_complete                             // is_complete() == false
test_with_policy_uses_custom_policy                  // policy fields match input
test_default_policy_values                           // max_total=15, max_stage=3, max_research=1, exhaustion=ParkForHuman

// Forward progression
test_advance_research_to_plan                        // advance() from Research → to == Plan
test_advance_plan_to_implement                       // advance() from Plan → to == Implement
test_advance_implement_to_review                     // advance() from Implement → to == Review
test_advance_review_to_validate                      // advance() from Review → to == Validate
test_advance_validate_to_accept                      // advance() from Validate → to == Accept
test_full_forward_progression                        // Research→Plan→Implement→Review→Validate→Accept, is_complete()==true
test_advance_past_accept_errors                      // advance() at Accept → AlreadyTerminal error

// Re-entry (recursion)
test_reenter_review_to_implement_minor               // from Review, severity Minor → lands at Implement
test_reenter_review_to_plan_major                    // from Review, severity Major → lands at Plan
test_reenter_review_to_research_fundamental          // from Review, severity Fundamental → lands at Research
test_reenter_validate_to_implement                   // from Validate on CI failure → Implement
test_reenter_cannot_go_forward                       // reenter from Research to Plan → InvalidReentry error
test_reenter_cannot_go_to_same_stage                 // reenter from Review to Review → InvalidReentry error
test_reentry_target_for_severity                     // Minor→Implement, Major→Plan, Fundamental→Research

// Recursion bounds
test_total_attempts_exhaustion                       // after max_total_attempts, enter_stage() → TotalAttemptsExhausted
test_stage_retries_exhaustion                        // after max_stage_retries for one stage → StageRetriesExhausted
test_research_retries_uses_separate_limit            // research has its own lower limit (max_research_retries)
test_enter_stage_increments_counters                 // stage_attempts(current) and total_attempts both increment

// History tracking
test_history_records_forward_transitions             // advance() appends to history with Completed reason
test_history_records_reentry_transitions             // reenter() appends to history with GateFailed reason
test_history_preserves_order                         // history[0] is first transition, history[n] is latest
```

---

### Bead 2: Stage Events (events crate)

**File**: `crates/events/src/event.rs` (modify existing)
**Modify**: `crates/events/src/lib.rs` (re-exports)
**Dependencies**: Bead 1 (needs `StageKind`, `Severity`)
**Crate**: `oya-events`

#### New Variants to Add to `BeadEvent`

```rust
// Add to existing BeadEvent enum:
StageStarted {
    event_id: EventId,
    bead_id: BeadId,
    stage: StageKind,
    attempt: u32,
    timestamp: DateTime<Utc>,
},
StageCompleted {
    event_id: EventId,
    bead_id: BeadId,
    stage: StageKind,
    artifact_ref: Option<String>,  // path or ID of stage output
    timestamp: DateTime<Utc>,
},
StageFailed {
    event_id: EventId,
    bead_id: BeadId,
    stage: StageKind,
    feedback: String,
    severity: Severity,
    timestamp: DateTime<Utc>,
},
StageReentry {
    event_id: EventId,
    bead_id: BeadId,
    from_stage: StageKind,
    to_stage: StageKind,
    reason: String,
    attempt: u32,
    timestamp: DateTime<Utc>,
},
ValidationRan {
    event_id: EventId,
    bead_id: BeadId,
    passed: bool,
    output: String,
    command: String,
    exit_code: i32,
    timestamp: DateTime<Utc>,
},
RecursionExhausted {
    event_id: EventId,
    bead_id: BeadId,
    total_attempts: u32,
    last_stage: StageKind,
    timestamp: DateTime<Utc>,
},
```

#### Constructor Methods to Add

```
BeadEvent::stage_started(bead_id, stage, attempt) -> Self
BeadEvent::stage_completed(bead_id, stage, artifact_ref) -> Self
BeadEvent::stage_failed(bead_id, stage, feedback, severity) -> Self
BeadEvent::stage_reentry(bead_id, from_stage, to_stage, reason, attempt) -> Self
BeadEvent::validation_ran(bead_id, passed, output, command, exit_code) -> Self
BeadEvent::recursion_exhausted(bead_id, total_attempts, last_stage) -> Self
```

#### Update `event_type()` Method

Add match arms returning: `"stage_started"`, `"stage_completed"`, `"stage_failed"`, `"stage_reentry"`, `"validation_ran"`, `"recursion_exhausted"`

#### Update `bead_id()` Method

Add match arms for all new variants.

#### Tests (12 tests)

```
// Construction
test_stage_started_constructor                       // fields populated correctly, event_type() == "stage_started"
test_stage_completed_constructor                     // fields populated correctly
test_stage_failed_constructor                        // fields populated correctly, severity preserved
test_stage_reentry_constructor                       // from/to stages correct
test_validation_ran_constructor_pass                  // passed == true
test_validation_ran_constructor_fail                  // passed == false, output preserved
test_recursion_exhausted_constructor                  // total_attempts and last_stage correct

// Serialization round-trip (bincode)
test_stage_started_bincode_roundtrip                 // to_bincode → from_bincode == original
test_stage_failed_bincode_roundtrip                  // severity survives serialization
test_stage_reentry_bincode_roundtrip                 // from/to stages survive
test_validation_ran_bincode_roundtrip                // output string survives

// EventPattern matching
test_stage_events_match_by_bead_pattern              // EventPattern::ByBead matches stage events
```

---

### Bead 3: Projection Updates (events crate)

**File**: `crates/events/src/projection.rs` (modify existing)
**Dependencies**: Bead 1, Bead 2
**Crate**: `oya-events`

#### Changes to `BeadProjection`

Add fields:
```
current_stage: Option<StageKind>       // which stage the bead is in
stage_attempts: HashMap<StageKind, u32> // attempt counts per stage
last_stage_feedback: Option<String>     // most recent gate feedback
```

#### Changes to `AllBeadsProjection::apply()`

Handle new event variants:
- `StageStarted` → set `current_stage`, increment stage_attempts
- `StageCompleted` → update current_stage to next
- `StageFailed` → record feedback
- `StageReentry` → update current_stage to target
- `RecursionExhausted` → set state to a terminal/parked state

#### Tests (6 tests)

```
test_projection_tracks_current_stage                 // after StageStarted, current_stage == that stage
test_projection_increments_stage_attempts            // after 3 StageStarted for Implement, count == 3
test_projection_records_feedback                     // after StageFailed, last_stage_feedback has the text
test_projection_updates_on_reentry                   // after StageReentry, current_stage == target
test_projection_handles_recursion_exhausted           // bead enters terminal state
test_projection_rebuild_from_events                  // rebuild() produces same state as incremental apply
```

---

### Bead 4: `StageGate` (orchestrator crate)

**File**: `crates/orchestrator/src/stage_gate.rs`
**Modify**: `crates/orchestrator/src/lib.rs` (add `pub mod stage_gate;`)
**Dependencies**: Bead 1 (needs `StageKind`, `Severity`, `BeadStateMachine`)
**Crate**: `orchestrator`

#### Types

```
StageOutput         struct: { stage: StageKind, success: bool, output: String, exit_code: Option<i32>, duration_ms: u64 }
GateDecision        enum: Proceed { next_stage: StageKind } | Reenter { stage: StageKind, feedback: String, severity: Severity } | Fail { reason: String } | Exhausted { policy: ExhaustionPolicy }
StageGate           struct: { policy: RecursionPolicy }
```

#### Methods

```
StageGate::new(policy: RecursionPolicy) -> Self
StageGate::evaluate(&self, machine: &BeadStateMachine, output: StageOutput) -> GateDecision
```

#### Gate Logic (pure function, no I/O)

```
If output.success:
    If machine.current_stage().next() is Some(next):
        → Proceed { next_stage: next }
    Else:
        → Proceed { next_stage: Accept }  // terminal

If !output.success:
    If machine.current_stage() == Validate:
        → Reenter { stage: Implement, feedback: output.output, severity: Minor }
    If machine.current_stage() == Review:
        Parse output for severity keywords:
            "fundamental" / "wrong approach" / "misunderstood" → Severity::Fundamental
            "major" / "redesign" / "significant" → Severity::Major
            else → Severity::Minor
        target = reentry_target_for_severity(severity)
        → Reenter { stage: target, feedback: output.output, severity }
    Else:
        → Fail { reason: output.output }

Before returning Reenter, check if machine would exceed bounds:
    machine.clone().reenter(target, ...) → if StateMachineError → Exhausted { policy }
```

#### Tests (14 tests)

```
// Success cases
test_gate_success_advances_research_to_plan          // success at Research → Proceed to Plan
test_gate_success_advances_implement_to_review       // success at Implement → Proceed to Review
test_gate_success_at_validate_advances_to_accept     // success at Validate → Proceed to Accept
test_gate_success_full_forward_chain                 // each stage success → next stage

// Review rejection cases
test_gate_review_reject_minor_to_implement           // failed Review, no severity keywords → Implement
test_gate_review_reject_major_to_plan                // failed Review, "redesign" in output → Plan
test_gate_review_reject_fundamental_to_research      // failed Review, "wrong approach" in output → Research

// Validation failure
test_gate_validate_fail_to_implement                 // failed Validate → Reenter Implement with CI output

// Non-review failure
test_gate_implement_fail_is_hard_fail                // failed Implement (process crash) → Fail

// Exhaustion
test_gate_exhausted_when_bounds_exceeded             // machine at max retries → Exhausted
test_gate_exhausted_uses_policy                      // Exhausted carries the ExhaustionPolicy

// Edge cases
test_gate_empty_output_on_failure                    // empty output string still produces valid decision
test_gate_very_long_output_truncated_in_feedback     // output > some limit gets truncated
test_gate_preserves_exit_code_in_feedback            // exit_code appears in feedback string
```

---

### Bead 5: `StageContextBuilder` (orchestrator crate)

**File**: `crates/orchestrator/src/context_builder.rs`
**Modify**: `crates/orchestrator/src/lib.rs` (add `pub mod context_builder;`)
**Dependencies**: Bead 1 (needs `StageKind`)
**Crate**: `orchestrator`

#### Types

```
BeadContext         struct: { bead_id: BeadId, spec: String, relevant_files: Vec<PathBuf>, upstream_artifacts: Vec<String> }
StagePrompt         struct: { stage: StageKind, prompt_text: String, allowed_tools: Vec<String>, timeout: Duration }
StageContextBuilder struct: { project_root: PathBuf, claude_md_path: Option<PathBuf> }
```

#### Methods

```
StageContextBuilder::new(project_root: PathBuf) -> Self
StageContextBuilder::with_claude_md(self, path: PathBuf) -> Self
StageContextBuilder::build_prompt(
    &self,
    stage: StageKind,
    context: &BeadContext,
    artifacts: &HashMap<StageKind, String>,   // outputs from previous stages
    feedback: Option<&str>,                    // gate feedback on retry
) -> Result<StagePrompt, ContextError>
```

#### Per-Stage Prompt Templates

Each stage gets a different prompt structure:

- **Research**: "Analyze bead spec: {spec}. Find relevant source files in {project_root}. List dependencies, risks, and affected modules. Output structured research."
- **Plan**: "Given research: {research_artifact}. Create implementation plan. List files to modify, test strategy, edge cases."
- **Implement**: "Implement bead {bead_id}. Plan: {plan_artifact}. {feedback_if_retry}. Run `moon run :quick` when done."
- **Review**: "Review diff for bead {bead_id}. Plan was: {plan_artifact}. Spec: {spec}. Check correctness, edge cases, zero-unwrap. Verdict: PASS or REJECT with severity (minor/major/fundamental) and feedback."
- **Validate**: N/A (runs shell command, no prompt)
- **Accept**: N/A (marks complete, no prompt)

#### Tests (10 tests)

```
// Prompt generation
test_research_prompt_includes_spec                   // spec text appears in prompt
test_plan_prompt_includes_research_artifact           // research output passed through
test_implement_prompt_includes_plan                  // plan artifact in prompt
test_implement_retry_prompt_includes_feedback         // gate feedback appears when retrying
test_review_prompt_includes_diff_and_plan            // both plan and "diff" instruction present
test_validate_returns_no_prompt                      // Validate stage → prompt_text is empty / special marker
test_accept_returns_no_prompt                        // Accept stage → same

// Context building
test_context_builder_reads_claude_md                 // CLAUDE.md content injected into prompts
test_context_builder_missing_claude_md_ok            // no CLAUDE.md → still produces valid prompt
test_prompt_timeout_varies_by_stage                  // Research gets longer timeout than Review
```

---

### Bead 6: `AgentSlotActor` (orchestrator crate)

**File**: `crates/orchestrator/src/actors/agent_slot.rs`
**Modify**: `crates/orchestrator/src/actors/mod.rs` (add module + re-exports)
**Dependencies**: Bead 1, Bead 4, Bead 5
**Crate**: `orchestrator`

#### Types

```
AgentSlotConfig     struct: { slot_id: String, claude_cmd: String, validation_cmd: String, session_timeout: Duration }
AgentSlotStatus     enum: Idle | ExecutingStage { bead_id: BeadId, stage: StageKind, pid: u32 } | Validating { bead_id: BeadId } | Cooldown
AgentSlotState      struct: { config: AgentSlotConfig, status: AgentSlotStatus, state_machine: Option<BeadStateMachine>, stage_artifacts: HashMap<StageKind, String>, context_builder: Arc<StageContextBuilder>, gate: Arc<StageGate> }
AgentSlotMessage    enum: AssignBead { bead_id, context: BeadContext } | StageProcessExited { exit_code, stdout, stderr } | ValidationComplete { passed, output } | Timeout | Release
AgentSlotEffect     enum: SpawnProcess { cmd, args, env } | EmitEvent { event: BeadEvent } | RequestNextBead | ValidationNeeded { bead_id }
AgentSlotActorDef   struct (implements ractor::Actor)
```

#### Actor Lifecycle

```
pre_start:
    Initialize with Idle status
    Store config, context_builder, gate references

handle(AssignBead):
    Create BeadStateMachine for bead
    Set status = ExecutingStage
    enter_stage() on state machine
    Build prompt via context_builder
    Effect: SpawnProcess (claude CLI)
    Effect: EmitEvent(StageStarted)

handle(StageProcessExited):
    Create StageOutput from exit code + stdout
    Run gate.evaluate(machine, output)
    Match GateDecision:
        Proceed { next }:
            If next == Validate:
                Set status = Validating
                Effect: ValidationNeeded
            Elif next == Accept:
                Effect: EmitEvent(Completed)
                Set status = Idle
                Effect: RequestNextBead
            Else:
                machine.advance()
                machine.enter_stage()
                Build prompt for next stage
                Effect: SpawnProcess
                Effect: EmitEvent(StageStarted)
        Reenter { stage, feedback, severity }:
            machine.reenter(stage, feedback, severity)
            machine.enter_stage()
            Build prompt with feedback
            Effect: SpawnProcess
            Effect: EmitEvent(StageReentry)
        Fail:
            Effect: EmitEvent(Failed)
            Set status = Idle
            Effect: RequestNextBead
        Exhausted:
            Effect: EmitEvent(RecursionExhausted)
            Set status = Idle
            Effect: RequestNextBead

handle(ValidationComplete):
    If passed:
        machine.advance() (Validate → Accept)
        Effect: EmitEvent(Completed)
        Set status = Idle
        Effect: RequestNextBead
    Else:
        Create StageOutput { success: false, output }
        Run through gate (→ Reenter Implement)
        Effect: EmitEvent(ValidationRan)
        machine.reenter(Implement, ...)
        Build implement prompt with CI output as feedback
        Effect: SpawnProcess
        Effect: EmitEvent(StageReentry)

handle(Timeout):
    Kill child process
    Effect: EmitEvent(StageFailed with timeout)
    Treat as failure through gate

handle(Release):
    Kill child process if running
    Set status = Idle
```

#### Tests (16 tests)

```
// State transitions
test_slot_starts_idle                                // initial status == Idle
test_assign_bead_transitions_to_executing            // AssignBead → ExecutingStage
test_stage_exit_success_advances_stage               // exit 0 → next stage
test_stage_exit_failure_triggers_gate                 // exit non-0 → gate evaluation
test_validation_pass_completes_bead                  // ValidationComplete(true) → Idle + Completed event
test_validation_fail_reenters_implement              // ValidationComplete(false) → back to Implement
test_release_returns_to_idle                         // Release while executing → Idle

// Effect generation
test_assign_emits_spawn_and_stage_started            // AssignBead produces SpawnProcess + EmitEvent(StageStarted)
test_completion_emits_completed_event                // final advance → EmitEvent(Completed)
test_failure_emits_failed_event                      // gate Fail → EmitEvent(Failed)
test_reentry_emits_reentry_event                     // gate Reenter → EmitEvent(StageReentry)
test_exhaustion_emits_recursion_exhausted             // gate Exhausted → EmitEvent(RecursionExhausted)
test_idle_after_completion_requests_next_bead         // Completed → RequestNextBead effect

// Timeout handling
test_timeout_kills_process_and_fails                 // Timeout → SpawnProcess killed + StageFailed

// Artifact tracking
test_stage_artifacts_accumulated                     // each StageCompleted stores artifact_ref
test_implement_retry_gets_previous_feedback          // feedback from gate appears in next prompt
```

---

### Bead 7: `BeadOrchestrator` (orchestrator crate)

**File**: `crates/orchestrator/src/bead_orchestrator.rs`
**Modify**: `crates/orchestrator/src/lib.rs` (add `pub mod bead_orchestrator;`)
**Dependencies**: Bead 6, existing `SchedulerActor`, existing `EventBus`
**Crate**: `orchestrator`

#### Types

```
OrchestratorConfig  struct: {
    max_concurrent_agents: usize,
    recursion_policy: RecursionPolicy,
    validation_cmd: String,
    session_timeout: Duration,
    claude_cmd: String,
    project_root: PathBuf,
}
OrchestratorStatus  struct: {
    total_beads: usize,
    completed: usize,
    failed: usize,
    in_progress: usize,
    parked: usize,
    idle_slots: usize,
}
BeadOrchestrator    struct: {
    config: OrchestratorConfig,
    slots: Vec<ActorRef<AgentSlotMessage>>,
    scheduler: ActorRef<SchedulerMessage>,
    event_bus: Arc<EventBus>,
    ipc_bridge: Option<IpcBridge>,
}
```

#### Methods

```
BeadOrchestrator::new(config, event_bus, scheduler) -> Result<Self>
BeadOrchestrator::spawn_slots(&mut self) -> Result<()>                    // create N AgentSlotActors
BeadOrchestrator::run(&mut self) -> Result<OrchestratorStatus>            // main event loop
BeadOrchestrator::assign_ready_beads(&self) -> Result<usize>              // poll scheduler, assign to idle slots
BeadOrchestrator::handle_event(&mut self, event: BeadEvent) -> Result<()> // route events
BeadOrchestrator::status(&self) -> OrchestratorStatus
BeadOrchestrator::shutdown(&mut self) -> Result<()>                       // graceful shutdown
```

#### Event Loop Logic

```
loop:
    1. Poll scheduler for ready beads (GetAllReadyBeads)
    2. Find idle agent slots
    3. For each (ready_bead, idle_slot): send AssignBead
    4. Wait for events from EventBus subscription
    5. On Completed: update scheduler (OnBeadCompleted), check if all done
    6. On Failed/RecursionExhausted: log, update status
    7. On RequestNextBead from slot: trigger step 1-3 again
    8. If all beads complete: break
    9. Forward all events to IpcBridge (if connected)
```

#### Tests (10 tests)

```
// Slot management
test_spawn_creates_n_slots                           // max_concurrent_agents=4 → 4 slots created
test_idle_slots_receive_assignments                  // ready bead + idle slot → AssignBead sent
test_no_assignment_when_all_slots_busy               // all slots executing → no assignment

// Event routing
test_completed_event_updates_scheduler               // Completed → OnBeadCompleted forwarded to scheduler
test_completed_frees_slot_for_next                   // Completed → slot becomes idle → next bead assigned
test_failed_event_updates_status                     // Failed → status.failed increments
test_exhausted_event_updates_status                  // RecursionExhausted → status.parked increments

// Lifecycle
test_all_beads_complete_terminates_loop              // all beads done → run() returns
test_shutdown_stops_all_slots                        // shutdown() → all slots get Release
test_status_reflects_current_state                   // status() counts match actual state
```

---

### Bead 8: IPC Bridge + Zellij Wiring (orchestrator + oya-ipc + oya-ui)

**Files**:
- `crates/orchestrator/src/ipc_bridge.rs` (new)
- `crates/oya-ipc/src/messages.rs` (modify — uncomment and extend)
- `crates/oya-ipc/src/lib.rs` (modify — re-export messages)
- `crates/oya-ui/src/plugin.rs` (modify — replace sample data with IPC)
- `crates/oya-ui/src/render.rs` (modify — render stage progress)
- `crates/orchestrator/src/lib.rs` (add `pub mod ipc_bridge;`)

**Dependencies**: Bead 1, Bead 2, Bead 7
**Crates**: `orchestrator`, `oya-ipc`, `oya-ui`

#### 8a: Extend IPC Messages

**File**: `crates/oya-ipc/src/messages.rs`

Add to `HostMessage`:
```
StageUpdate {
    bead_id: String,
    stage: String,           // StageKind as string
    status: String,          // "started" | "completed" | "failed" | "reentry"
    attempt: u32,
    feedback: Option<String>,
    timestamp: u64,
}
```

Add to `GuestMessage`:
```
RequestBeadStages { bead_id: String }     // query current stage info for a bead
PauseBead { bead_id: String }             // pause execution
ResumeBead { bead_id: String }            // resume paused bead
```

Add to `HostMessage`:
```
BeadStages {
    bead_id: String,
    current_stage: String,
    attempts: Vec<(String, u32)>,          // (stage_name, attempt_count)
    history: Vec<StageTransitionInfo>,
}

StageTransitionInfo {
    from: String,
    to: String,
    reason: String,
    timestamp: u64,
}
```

Uncomment messages module in `lib.rs` and add re-exports.

#### 8b: IPC Bridge (orchestrator side)

**File**: `crates/orchestrator/src/ipc_bridge.rs`

```
IpcBridge           struct: { event_rx: broadcast::Receiver<BeadEvent>, host_tx: mpsc::Sender<HostMessage> }
```

Methods:
```
IpcBridge::new(event_bus: &EventBus) -> (Self, mpsc::Receiver<HostMessage>)
IpcBridge::run(&mut self) -> Result<()>               // loop: recv BeadEvent → convert → send HostMessage
IpcBridge::convert_event(event: BeadEvent) -> Option<HostMessage>  // map BeadEvent variants to HostMessage variants
```

Conversion logic:
```
StageStarted     → StageUpdate { status: "started" }
StageCompleted   → StageUpdate { status: "completed" }
StageFailed      → StageUpdate { status: "failed", feedback: Some(...) }
StageReentry     → StageUpdate { status: "reentry", feedback: Some(...) }
Completed        → BeadStateChanged { to_state: "completed" }
Failed           → SystemAlert { level: Error }
RecursionExhausted → SystemAlert { level: Critical }
ValidationRan    → StageUpdate { status: if passed "completed" else "failed" }
```

#### 8c: Zellij UI Updates

**File**: `crates/oya-ui/src/plugin.rs`

Replace `sample_beads: Vec<SampleBead>` with:
```
beads: Vec<BeadDisplayInfo>       // populated from IPC
stage_updates: HashMap<String, StageDisplayInfo>  // latest stage per bead
```

**File**: `crates/oya-ui/src/render.rs`

Update `render_pipeline_view` to show actual stage progress from IPC data instead of hardcoded stages. Each stage shows: `○` (pending), `◐` (running), `●` (completed), `✗` (failed), `↩` (re-entered).

#### Tests (14 tests)

```
// IPC message serialization (oya-ipc)
test_stage_update_serialization_roundtrip            // StageUpdate survives serde_json round-trip
test_request_bead_stages_serialization               // GuestMessage::RequestBeadStages serializes
test_bead_stages_response_serialization              // HostMessage::BeadStages serializes
test_stage_transition_info_serialization             // StageTransitionInfo serializes

// IPC Bridge conversion (orchestrator)
test_bridge_converts_stage_started                   // BeadEvent::StageStarted → HostMessage::StageUpdate
test_bridge_converts_stage_completed                 // BeadEvent::StageCompleted → HostMessage::StageUpdate
test_bridge_converts_stage_failed                    // BeadEvent::StageFailed → HostMessage::StageUpdate with feedback
test_bridge_converts_stage_reentry                   // BeadEvent::StageReentry → HostMessage::StageUpdate
test_bridge_converts_completed                       // BeadEvent::Completed → HostMessage::BeadStateChanged
test_bridge_converts_recursion_exhausted             // BeadEvent::RecursionExhausted → HostMessage::SystemAlert
test_bridge_ignores_irrelevant_events                // BeadEvent::MetadataUpdated → None

// UI rendering (oya-ui)
test_render_pipeline_shows_stage_progress            // stages render with correct symbols
test_render_reentry_shows_loop_indicator             // re-entered stage shows ↩ symbol
test_render_pipeline_empty_when_no_data              // no IPC data → placeholder message
```

---

### Bead 9: CLI Wiring (`oya storm`)

**File**: `src/commands/storm.rs` (new)
**Modify**: `src/commands/mod.rs` or `src/cli.rs` (add storm subcommand)
**Modify**: `src/main.rs` (wire command)
**Dependencies**: Bead 7, Bead 8
**Crate**: `oya` (root)

#### Command Signature

```
oya storm [OPTIONS]
    --agents <N>           Number of concurrent agent slots (default: 4)
    --timeout <DURATION>   Per-stage timeout (default: 30m)
    --claude-cmd <PATH>    Path to claude CLI (default: "claude")
    --validation-cmd <CMD> Validation command (default: "moon run :ci")
    --workflow <FILE>      Workflow definition file (beads + dependencies)
    --max-retries <N>      Max total retries per bead (default: 15)
    --dry-run              Show what would execute without running
```

#### Logic

```
1. Parse args into OrchestratorConfig
2. Load workflow file (JSON/YAML with bead specs and dependency edges)
3. Create InMemoryEventStore + EventBus
4. Create SchedulerActor, register workflow, add beads and dependencies
5. Create BeadOrchestrator with N agent slots
6. Optionally create IpcBridge for Zellij UI
7. Run orchestrator event loop
8. On completion: print summary (completed, failed, parked)
9. Exit with code 0 if all passed, 1 if any failed
```

#### Workflow File Format

```json
{
    "workflow_id": "wf-feature-xyz",
    "beads": [
        {
            "id": "bead-auth",
            "spec": "Implement JWT authentication middleware",
            "depends_on": []
        },
        {
            "id": "bead-api",
            "spec": "Add REST endpoints for user management",
            "depends_on": ["bead-auth"]
        }
    ]
}
```

#### Tests (8 tests)

```
// Workflow file parsing
test_parse_workflow_file_valid                        // valid JSON → workflow with beads and deps
test_parse_workflow_file_missing_beads                // no beads field → error
test_parse_workflow_file_invalid_dependency           // depends_on references non-existent bead → error
test_parse_workflow_file_cyclic_dependency            // cycle in depends_on → error

// Config construction
test_config_from_args_defaults                       // no flags → default values
test_config_from_args_custom                         // all flags set → custom values

// Dry run
test_dry_run_prints_plan_no_execution                // --dry-run → prints beads and exits
test_dry_run_shows_dependency_order                  // --dry-run → topological order printed
```

---

## Dependency Graph (Build Order)

```
Bead 1: BeadStateMachine (pure logic, zero deps)
   │
   ├──→ Bead 2: Stage Events (extends BeadEvent)
   │       │
   │       └──→ Bead 3: Projection Updates
   │
   ├──→ Bead 4: StageGate (pure logic)
   │
   └──→ Bead 5: StageContextBuilder
          │
          └──→ Bead 6: AgentSlotActor (depends on 1, 4, 5)
                 │
                 └──→ Bead 7: BeadOrchestrator (depends on 6)
                        │
                        ├──→ Bead 8: IPC Bridge + Zellij (depends on 2, 7)
                        │
                        └──→ Bead 9: CLI Wiring (depends on 7, 8)
```

**Parallelizable**:
- Beads 2, 4, 5 can run in parallel (all depend only on Bead 1)
- Bead 3 can run in parallel with 4, 5 (depends on 2, not on 4/5)
- Beads 8, 9 are sequential (9 depends on 8)

**Critical path**: 1 → 4 → 6 → 7 → 8 → 9

---

## Files Created/Modified Summary

| Action | File | Bead |
|--------|------|------|
| CREATE | `crates/events/src/stage.rs` | 1 |
| MODIFY | `crates/events/src/lib.rs` | 1, 2 |
| MODIFY | `crates/events/src/event.rs` | 2 |
| MODIFY | `crates/events/src/projection.rs` | 3 |
| CREATE | `crates/orchestrator/src/stage_gate.rs` | 4 |
| CREATE | `crates/orchestrator/src/context_builder.rs` | 5 |
| CREATE | `crates/orchestrator/src/actors/agent_slot.rs` | 6 |
| MODIFY | `crates/orchestrator/src/actors/mod.rs` | 6 |
| CREATE | `crates/orchestrator/src/bead_orchestrator.rs` | 7 |
| MODIFY | `crates/orchestrator/src/lib.rs` | 4, 5, 7, 8 |
| CREATE | `crates/orchestrator/src/ipc_bridge.rs` | 8 |
| MODIFY | `crates/oya-ipc/src/messages.rs` | 8 |
| MODIFY | `crates/oya-ipc/src/lib.rs` | 8 |
| MODIFY | `crates/oya-ui/src/plugin.rs` | 8 |
| MODIFY | `crates/oya-ui/src/render.rs` | 8 |
| CREATE | `src/commands/storm.rs` | 9 |
| MODIFY | `src/cli.rs` or `src/commands/mod.rs` | 9 |

**Total**: 8 new files, 9 modified files, 9 beads, ~114 tests

---

## Quality Gates Per Bead

Every bead must pass before merging:

1. `moon run :quick` (fmt + clippy) — zero warnings
2. All tests pass (RED first, then GREEN)
3. Zero `unwrap()`, zero `expect()`, zero `panic!()` in production code
4. `#[cfg(test)]` modules may use `#![allow(clippy::unwrap_used)]`
5. All public types have doc comments
6. No new dependencies unless explicitly listed above

---

## What Gets Deleted (After All Beads Land)

| Remove | Reason |
|--------|--------|
| `src/swarm/` directory | Replaced by BeadOrchestrator + AgentSlotActor |
| `crates/orchestrator/src/ipc_messages.rs` | Canonical types now in `oya-ipc/src/messages.rs` |
| `crates/orchestrator/src/actors/ipc_worker.rs` | Replaced by `ipc_bridge.rs` |

**Note**: Do NOT delete these until all 9 beads have landed and passed CI. Keep them compiling during the transition.
