# Simplified State Model Design

**Date**: 2026-02-19
**Status**: Approved
**Target**: Reduce Restate operations from 140+ to ~13 per pipeline run

## Problem

Current OyaOrchestrator executes 140+ Restate operations per pipeline:
- `set()` for every intermediate state update
- `set()` for each individual event (event_0001, event_0002, etc.)
- `get()` to read back state repeatedly
- `set()` for per-gate artifacts
- `call()` to usage tracker per stage
- `run()` for timestamps throughout

This creates high cardinality in the KVP store and slows execution.

## Solution

**Stage-level recovery with batched state persistence**

Execute each stage completely in-memory, accumulate ALL stage data, then persist **one rich artifact** after stage completion. Only persist checkpoints at stage boundaries.

## Architecture

### Operation Count: 13 Total

**Startup (2 ops):**
1. `set("run_request")` - Initial run metadata
2. `set("state")` - Initial orchestrator state

**Per Stage (6 stages × 1.5 = 9 ops):**
- `run(|| timestamp)` - Stage start timestamp
- Execute stage (no Restate ops, accumulate in-memory)
- `run(|| timestamp)` - Stage end timestamp
- `set("stage_N", {...})` - Single rich payload with everything

**Shutdown (2 ops):**
12. `set("state")` - Final completion state
13. `set("timeline")` - Lean timeline for D2 visualization

### Stage Artifact Schema

Each `stage_N` entry contains:

```rust
struct StageArtifact {
    stage: String,           // "plan", "contract", etc.
    attempt: u32,

    // Timing
    timing: StageTiming {
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        duration_ms: u64,
    },

    // Workspace (if applicable)
    workspace: Option<WorkspaceLifecycle> {
        name: String,
        queue_command: String,
        queue_passed: bool,
        queue_exit_code: i32,
        add_command: String,
        add_passed: bool,
        add_exit_code: i32,
    },

    // Inputs
    input: StageInput {
        run_id: String,
        bead_id: String,
        context: String,
        model: String,
        last_failure: Option<FailureSnapshot>,
    },

    // AI interaction
    prompt: String,          // Full prompt sent to AI
    output: StageOutput {
        success: bool,
        exit_code: i32,
        full_log: String,    // Complete stdout/stderr
        feedback: String,
        contract_document: Option<String>,
        implementation_code: Option<String>,
        test_results: Option<String>,
        adversarial_report: Option<String>,
    },

    // Task tracking
    task_tracking: Option<TaskTracking> {
        tasks_created: Vec<String>,
        tasks_updated: Vec<String>,
        tasks_completed: Vec<String>,
        task_states: HashMap<String, TaskState>,
    },

    // Gates
    gates: Vec<GateResult> {
        gate: String,
        state_key: String,
        artifact_id: String,
        passed: bool,
        exit_code: i32,
    },

    // Outcome
    status: String,          // "completed" | "failed"
}
```

### Timeline Schema

Lean JSON array for D2 diagram generation:

```rust
[
  {event: "RunStarted", at: "2026-02-19T20:38:15Z"},
  {event: "StageCompleted", stage: "plan", attempt: 1, duration_ms: 72000, at: "..."},
  {event: "StageCompleted", stage: "contract", attempt: 1, duration_ms: 45000, at: "..."},
  {event: "StageCompleted", stage: "acceptance_test", attempt: 1, duration_ms: 123000, at: "..."},
  {event: "StageCompleted", stage: "implementation", attempt: 1, duration_ms: 89000, at: "..."},
  {event: "StageCompleted", stage: "qa", attempt: 1, duration_ms: 34000, at: "..."},
  {event: "RunShipped", total_duration_ms: 363000, at: "..."}
]
```

## Eliminated Operations

- ✅ No `set("state")` updates during stages (was 6+ ops)
- ✅ No `get("timeline")` and `get("event_seq")` reads (was 12+ ops)
- ✅ No `set("event_XXXX")` individual event storage (was 11+ ops)
- ✅ No `set("timeline")` appends (was 6+ ops)
- ✅ No `call(usage_tracker)` per stage (was 6 ops)
- ✅ No per-gate `set()` operations (was 18+ ops)

## Recovery Behavior

**Before:** Crash during stage → restart from beginning of stage (no progress lost within stage)

**After:** Crash during stage → restart entire stage from scratch
- Crash during Plan → restart Plan
- Plan completes → `set("plan_1")` → crash → restart Contract (Plan preserved)
- Each stage artifact is a recovery checkpoint

Trade-off: Accept losing work within a stage for simpler state management and fewer operations.

## KVP Storage

**Before:** 40+ entries
- `state`, `run_request`, `event_seq`, `timeline`
- `plan_1_input`, `plan_1_workspace`, `plan_1_prompt`, `plan_1_result`, `plan_1_skill_output`, `plan_1_gate_compiles`, `plan_1_gate_acceptance_tests_are_red`, `plan_1_event`
- Same pattern for each stage

**After:** 8 entries
- `state`, `run_request`, `timeline`
- `plan_1`, `contract_1`, `acceptance_test_1`, `implementation_1`, `qa_1`

Each `stage_N` contains all data that was previously split across 8+ keys.

## Implementation Changes

1. **Remove intermediate `set()` calls** in `pipeline/state.rs`
   - Delete `mark_stage_running()` that sets state before each stage
   - Delete `append_timeline()` calls during execution
   - Delete `set_json_state()` for individual artifacts

2. **Create `StageArtifact` struct** in `orchestrator_types.rs`
   - Combine all stage data into one type
   - Include timing, workspace, input, prompt, output, gates, task_tracking

3. **Batch timeline construction**
   - Accumulate timeline entries in memory
   - Single `set("timeline")` at end with complete array

4. **Remove `get()` calls**
   - No need to read `timeline` or `event_seq` during execution
   - No need to read back state we just wrote

5. **Optional: Remove usage tracking calls**
   - Remove `tracker.get_active_model()` and `tracker.report_outcome()`
   - Or batch into single call at end

## Files Modified

- `src/orchestrator_types.rs` - Add `StageArtifact` struct
- `src/pipeline/state.rs` - Remove intermediate sets, add batch operations
- `src/pipeline/mod.rs` - Update workflow to use batched artifacts
- `src/types/timeline.rs` - Simplify timeline to lean array

## Testing

- Verify operation count via Restate UI (should show ~13 ops)
- Test recovery: crash during stage → verify restart from stage beginning
- Verify all stage data present in single artifact
- Generate D2 diagram from lean timeline

## Rollout Plan

1. Add new `StageArtifact` types alongside existing (no breaking changes)
2. Update workflow to accumulate in-memory artifacts
3. Switch from granular sets to batched sets
4. Remove old granular set code
5. Verify operation count in Restate UI
6. Update documentation
