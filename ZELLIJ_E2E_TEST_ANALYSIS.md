# Zellij End-to-End Workflow Test Analysis
## Bead: src-3qbu | Effort: 4 hours

---

## Executive Summary

The task is to implement an **end-to-end workflow test** that validates a complete user workflow from **task creation to integration**. This test should verify the entire lifecycle using the zjj workspace isolation commands (`add`, `sync`, `done`) integrated with the OYA CLI task management system.

The test should follow the workflow:
1. **Create** - User creates a new task with `oya new`
2. **Isolate** - User creates workspace with `oya workspace add` (wraps `zjj add`)
3. **Work** - User works in isolated workspace
4. **Sync** - User syncs workspace with `oya workspace sync` (wraps `zjj sync`)
5. **Complete** - User marks complete with `oya workspace done` (wraps `zjj done`)

---

## Current Test Coverage

### Existing Test Patterns (Token-efficient Reference)

#### 1. **Command Pane Integration Tests**
- **File**: `crates/zellij-frontend/tests/command_pane_integration_test.rs` (676 lines)
- **What it tests**:
  - Command pane lifecycle: open → execute → close
  - Context validation (working directory, command existence)
  - Event serialization (JSON roundtrips)
  - Error handling (nonexistent pane, empty commands, invalid paths)
  - Multiple panes can be tracked simultaneously
- **Pattern Used**: Fixture-based testing with `CommandPaneTestFixture`
- **Error Types**: Enum-based with thiserror macros
- **Assertions**: 50+ test functions with specific error path validation

#### 2. **IPC Pipe Integration Tests**
- **File**: `crates/zellij-frontend/tests/ipc_pipe_integration_test.rs` (100+ lines)
- **What it tests**:
  - Pipe send/receive roundtrips
  - Bidirectional communication
  - JSON serialization through pipes
  - Message ordering and data integrity
- **Pattern Used**: OS-level pipe testing with async message flow

#### 3. **BDD Workflow Tests**
- **File**: `tests/bdd_workflow_completion_test.rs` (234 lines)
- **What it tests**:
  - Task completion status transitions
  - Dependency graph validation (linear, parallel, diamond)
  - ExecutionEngine workflow execution
  - Empty workflow edge case
- **Pattern Used**: Given-When-Then (BDD) with Result types
- **No Panics**: All tests use `Result<(), Box<dyn std::error::Error>>`

### Current Coverage Gaps

**NOT YET TESTED:**
- Complete end-to-end workflow from task creation → workspace isolation → completion
- State persistence across zjj workspace lifecycle
- Task status transitions during workspace operations
- Integration between OYA CLI commands and zjj operations
- Error recovery scenarios (workspace merge conflicts, sync failures)
- State restoration on workspace load
- Concurrent workspace operations

---

## Workflow Concept: Create-to-Completion Lifecycle

### Task States (TaskStatus Enum)

```rust
pub enum TaskStatus {
    Created,                           // Initial state after oya new
    InProgress { stage: String },      // Active in a pipeline stage
    PassedPipeline,                    // All stages complete, ready for integration
    FailedPipeline { stage, reason },  // Failed at a stage
    Integrated,                        // Merged and closed
}
```

### Workspace Lifecycle States

Based on zjj and OYA CLI architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                   WORKFLOW STATE MACHINE                     │
└─────────────────────────────────────────────────────────────┘

1. CREATE
   ↓ oya new --slug my-task
   Task created in TaskStatus::Created state
   Workspace does NOT exist yet (user can work on main)
   
2. ISOLATE  
   ↓ oya workspace add <name> OR zjj add <session>
   Creates isolated jj workspace
   Workspace state: INITIALIZED, ready for commits
   
3. WORK
   User commits changes in isolated workspace
   Task status transitions: Created → InProgress { "implement" }
   Workspace state: MODIFIED with local commits
   
4. SYNC (Optional, Multi-commit Case)
   ↓ oya workspace sync <name> OR zjj sync
   Rebases workspace on main branch
   Brings in any main branch updates
   Resolves conflicts if present
   Workspace state: REBASED
   
5. VERIFY (Integration Pipeline)
   ↓ moon run :ci
   Runs full test/lint/build pipeline
   Task status: InProgress { "lint" } → InProgress { "test" } → ...
   
6. COMPLETE
   ↓ oya workspace done <name> OR zjj done <name>
   Merges workspace to main via jj
   Pushes to remote (MANDATORY)
   Cleans up workspace/session
   Task status: PassedPipeline → Integrated
   Workspace state: DELETED
```

### Key State Transitions to Test

| Current State | Operation | Expected State | Must Verify |
|---|---|---|---|
| None | `oya new` | `TaskStatus::Created` | Task file created, no workspace yet |
| `Created` | `oya workspace add` | Workspace initialized | jj workspace created, Zellij tab created |
| Workspace `MODIFIED` | `oya workspace sync` | Workspace `REBASED` | Main branch updates merged, conflicts handled |
| Workspace `REBASED` | `oya workspace done` | Task `Integrated` | Merged to main, pushed to remote, workspace deleted |
| Any | `oya workspace status` | Status string | Current workspace state readable |
| Any | `oya workspace remove --force` | Workspace deleted | Session cleanup, no dangling files |

---

## What an End-to-End Workflow Test Should Verify

### 1. **Happy Path: Complete Workflow Execution**

```gherkin
SCENARIO: User creates task, works in isolation, syncs, and completes

GIVEN a clean repository with no tasks
WHEN user executes:
  1. oya new --slug "my-feature" --language rust
  2. oya workspace add my-feature-work
  3. [user edits files and commits in workspace]
  4. oya workspace sync my-feature-work
  5. oya workspace done my-feature-work

THEN:
  - Task exists with TaskStatus::Integrated
  - Main branch contains workspace commits
  - Workspace session deleted from .jj/workspaces
  - Git remote has new commit
  - Exit code is 0 for all commands
```

### 2. **State Persistence & Restoration**

```gherkin
SCENARIO: Workspace state persists and can be restored

GIVEN a workspace with local commits
WHEN user closes Zellij tab and reopens
  AND executes: oya workspace status my-feature-work

THEN:
  - Workspace state restored from .jj/workspaces storage
  - Local commits are still present
  - Selected task/pane information recovers
  - Status message shows correct workspace name
```

### 3. **Error Path: Sync with Conflicts**

```gherkin
SCENARIO: Workspace sync detects and reports conflicts

GIVEN workspace with local commits
  AND main branch has conflicting changes
  
WHEN user executes: oya workspace sync --force

THEN:
  - Error returned with conflict locations
  - Workspace remains in MODIFIED state (not rebased)
  - User can resolve manually or abort
```

### 4. **Error Path: Nonexistent Workspace**

```gherkin
SCENARIO: Commands fail gracefully for missing workspaces

WHEN user executes: oya workspace done nonexistent-workspace

THEN:
  - Error returned: "Workspace not found"
  - Task status unchanged
  - No side effects (no partial cleanup)
  - Exit code non-zero
```

### 5. **Concurrent Workspaces**

```gherkin
SCENARIO: Multiple workspaces can coexist and be managed independently

GIVEN 3 workspace sessions for different beads
  
WHEN user executes:
  1. oya workspace list
  2. oya workspace status workspace-1
  3. oya workspace status workspace-2
  
THEN:
  - List shows all 3 sessions
  - Status correctly reports each workspace state
  - Operations on workspace-1 don't affect workspace-2
```

---

## Key Workflow Commands to Test

### Command: `oya new`
**Source**: `src/commands/new.rs:48-85`
**What it does**: Creates task + optional zjj workspace
**Preconditions**:
- Slug is lowercase alphanumeric + hyphens only
- No path traversal (e.g., `../etc/passwd` rejected)
- Task record location exists
**Postconditions**:
- Task created with `TaskStatus::Created`
- Task record saved to `.oya/tasks.json`
- Workspace created via `zjj spawn <slug>` (if not `--skip-workspace`)

### Command: `oya workspace add <name>`
**Source**: `src/commands/workspace.rs:239-243`
**What it does**: Wraps `zjj add <name>` command
**Preconditions**:
- zjj CLI available in PATH or `ZJJ_PATH` env var
- Working directory is repo root or `--root` specified
**Postconditions**:
- jj workspace created in `.jj/workspaces/<name>`
- Zellij session created + tab opened (if running under Zellij)
- Workspace ready for commits

### Command: `oya workspace sync <name>`
**Source**: `src/commands/workspace.rs:206-214`
**What it does**: Wraps `zjj sync <name>` command with optional `--force`
**Preconditions**:
- Workspace exists
- jj is available
**Postconditions**:
- Workspace rebased on main branch
- New commits from main brought into workspace
- Conflict detection (can use `--force` to override)

### Command: `oya workspace done <name>`
**Source**: `src/commands/workspace.rs:217-225`
**What it does**: Wraps `zjj done <name>` command, merges + cleanup
**Preconditions**:
- Workspace exists
- jj and git are available
**Postconditions**:
- Workspace merged to main via `jj rebase -d main`
- Changes pushed to remote with `jj git push`
- Workspace session deleted from `.jj/workspaces`
- Task status updated to `Integrated`
- Exit code 0 if successful

### Command: `oya workspace list`
**Source**: `src/commands/workspace.rs:192-196`
**What it does**: Wraps `zjj list --json` command
**Preconditions**: None (even if no workspaces exist)
**Postconditions**: Returns JSON array of workspace sessions

### Command: `oya workspace status <name>`
**Source**: `src/commands/workspace.rs:199-203`
**What it does**: Wraps `zjj status <name>` command
**Preconditions**: Workspace exists
**Postconditions**: Returns workspace state string (INITIALIZED, MODIFIED, REBASED, MERGED)

---

## Code Locations Reference

### Main Components

| Component | Location | Purpose |
|---|---|---|
| **Workspace Commands** | `src/commands/workspace.rs:1-258` | CLI command definitions + handlers |
| **New Command** | `src/commands/new.rs:48-120` | Task creation + workspace spawn |
| **Task Domain** | `crates/pipeline/src/domain.rs` | TaskStatus enum, Task struct |
| **State Module** | `crates/zellij-frontend/src/state.rs:1-100+` | State persistence (StateSnapshot, StateError) |
| **Command Pane Tests** | `crates/zellij-frontend/tests/command_pane_integration_test.rs` | Test fixture pattern (baseline) |
| **BDD Tests** | `tests/bdd_workflow_completion_test.rs` | Given-When-Then pattern (baseline) |

### Relevant State Structures

**StateSnapshot** (persistence layer):
```rust
pub struct StateSnapshot {
    pub version: u32,
    pub tasks: Vec<TaskRow>,
    pub selected_index: usize,
    pub focused_pane: PaneType,
    pub plugin_state: PluginState,
    pub status_message: Option<String>,
    pub timestamp: u64,
}
```
**File**: `crates/zellij-frontend/src/state.rs:77-99`

**TaskStatus** (state machine):
```rust
pub enum TaskStatus {
    Created,
    InProgress { stage: String },
    PassedPipeline,
    FailedPipeline { stage: String, reason: String },
    Integrated,
}
```
**File**: `crates/pipeline/src/domain.rs`

---

## Test Implementation Strategy

### Recommended Test Structure

**File**: `crates/zellij-frontend/tests/e2e_workflow_test.rs` (New)

**Module Structure**:
```
1. Setup Fixtures
   - TestEnvironment: temp repo, task files, mock git
   - WorkflowScenario: preconditions + state tracking
   
2. Happy Path Tests (5-7 tests)
   - test_new_task_creates_record()
   - test_add_workspace_creates_jj_workspace()
   - test_workspace_sync_rebases_on_main()
   - test_workspace_done_merges_and_pushes()
   - test_full_workflow_create_to_integration()
   - test_list_shows_all_workspaces()
   - test_status_returns_workspace_state()

3. State Persistence Tests (3-4 tests)
   - test_workspace_state_persists_on_close_and_reopen()
   - test_state_snapshot_serialization_roundtrip()
   - test_task_status_transitions_during_workflow()
   
4. Error Path Tests (5-6 tests)
   - test_new_rejects_invalid_slug()
   - test_new_rejects_path_traversal()
   - test_sync_detects_conflicts()
   - test_done_fails_for_nonexistent_workspace()
   - test_sync_fails_if_workspace_not_found()
   - test_all_commands_fail_gracefully_without_zjj()

5. Concurrent Workspace Tests (2-3 tests)
   - test_multiple_workspaces_coexist()
   - test_workspace_operations_dont_interfere()
   - test_list_returns_all_sessions()
```

### Testing Patterns to Use

From existing codebase:

1. **Fixture Pattern** (from command_pane_integration_test.rs):
   - Create `WorkflowTestFixture` struct
   - Methods: `new()`, `create_task()`, `add_workspace()`, `sync_workspace()`, etc.
   - Handles cleanup in Drop

2. **Result Type Pattern** (from bdd_workflow_completion_test.rs):
   - All tests return `Result<(), Box<dyn std::error::Error>>`
   - No panics, no unwraps
   - Error propagation with `?` operator

3. **State Assertions**:
   - Verify TaskStatus transitions
   - Check file presence (task record, workspace directory)
   - Validate StateSnapshot structure
   - Confirm exit codes

---

## Contracts (What Must Be True)

### Preconditions (Must Be Set Up)
- [P1] Repository is a valid jj workspace (`.jj` directory exists)
- [P2] Git remote is configured (needed for push in `done`)
- [P3] Task creation path exists and is writable (`.oya/tasks.json`)
- [P4] zjj CLI is available in PATH or `ZJJ_PATH` env var
- [P5] Test has write access to filesystem (temp directories)

### Postconditions (Must Be True After Test)
- [PO1] Task record persisted with correct TaskStatus
- [PO2] Workspace created in `.jj/workspaces/<name>` if `add` called
- [PO3] Commands return exit code 0 on success, non-zero on failure
- [PO4] No orphaned workspaces or dangling jj state after `done`
- [PO5] Task status transitioned correctly: Created → InProgress → PassedPipeline → Integrated

### Invariants (Always True)
- [I1] Single TaskStatus per task (no conflicts)
- [I2] Single jj workspace per session name
- [I3] No panics or unwraps in any code path
- [I4] StateSnapshot version always matches STATE_VERSION (1)
- [I5] All file I/O uses Result<T, Error> propagation
- [I6] Workspace state matches `.jj/workspaces/<name>` filesystem state

---

## Summary for Implementation

### What Must Be Built

1. **Test File**: `crates/zellij-frontend/tests/e2e_workflow_test.rs`
2. **Test Fixture**: `WorkflowTestFixture` with lifecycle methods
3. **Test Scenarios**: 15-20 test functions covering happy path, state persistence, errors, concurrency
4. **Mock Setup**: Minimal git/jj mocking (use real jj if possible, skip if not available)

### What Must Be Verified

| Aspect | How to Verify |
|---|---|
| Task creation | File exists at `.oya/tasks.json`, TaskStatus::Created |
| Workspace isolation | `.jj/workspaces/<name>` directory created |
| State persistence | StateSnapshot serializes/deserializes, selected_index preserved |
| Sync behavior | Workspace rebased on main, new commits visible |
| Completion | Workspace merged to main, pushed to remote, deleted, task status updated |
| Error handling | All error paths return Err, exit codes non-zero, no side effects |
| Concurrency | Multiple workspaces independent, list shows all |

### Acceptance Criteria (Green = Done)

- [ ] All 15-20 test functions passing
- [ ] Zero panics, zero unwraps in test code
- [ ] State transitions verified at each step
- [ ] Error paths tested (conflicts, missing workspace, no zjj, invalid slug)
- [ ] `moon run :quick` passes (6-7ms with cache)
- [ ] `moon run :ci` passes (full pipeline)
- [ ] Test coverage >80% for workspace command handlers
- [ ] Documentation (contract) updated

---

## References

- **Workflow Docs**: `/docs/03_WORKFLOW.md` (7 lines, summary of workflow)
- **Workspace Spec**: `src/commands/workspace.rs:29-257` (8 commands, 258 lines)
- **New Command**: `src/commands/new.rs:44-120` (task creation)
- **Command Pane Tests**: `crates/zellij-frontend/tests/command_pane_integration_test.rs:1-676` (fixture pattern)
- **BDD Tests**: `tests/bdd_workflow_completion_test.rs:1-234` (Given-When-Then pattern)
- **State Module**: `crates/zellij-frontend/src/state.rs:1-100+` (StateSnapshot, StateError)
- **Task Domain**: `crates/pipeline/src/domain.rs` (TaskStatus enum)

---

## Estimated Effort Breakdown

**Total: 4 hours**

- **Research & Understand** (30 min): Read existing tests, workspace code, state module
- **Test Fixture Design** (30 min): Design WorkflowTestFixture, mock helpers
- **Happy Path Tests** (1.5 hours): Implement 7 tests covering full workflow
- **State Persistence** (45 min): Implement 3-4 tests for state snapshot
- **Error Paths** (45 min): Implement 5-6 tests for all error scenarios
- **Concurrency & Edge Cases** (20 min): Implement 2-3 tests for multiple workspaces
- **Documentation & Cleanup** (20 min): Update contract, ensure no panics

