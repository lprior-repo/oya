# Martin Fowler Test Plan: IPC Stage Update Wiring

## Overview
This test plan specifies **executable test scenarios** for verifying that stage lifecycle events flow from EventBus through IpcWorker to Zellij UI, replacing sample data with live IPC-based stage progression.

## Test Principles
- **Expressive names**: Test functions describe behavior unambiguously
- **Given-When-Then**: Clear setup, action, and assertion structure
- **Coverage**: Happy path, error path, edge cases, contract verification
- **No implementation details**: Tests specify WHAT, not HOW

---

## Happy Path Tests

### IPC Worker Event Conversion

#### `test_stage_started_event_converts_to_host_message`
**Given**: A `BeadEvent::StageStarted` event with valid bead_id, stage, attempt, timestamp
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::StageStarted)`
- `bead_id` matches original event
- `stage` is "implement" (StageKind::Implement converted to string)
- `attempt` matches original event (1)
- `timestamp` is Unix epoch seconds (u64)

#### `test_stage_completed_event_converts_with_artifact_ref`
**Given**: A `BeadEvent::StageCompleted` event with `artifact_ref = Some("artifacts/contract.md")`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::StageCompleted)`
- `artifact_ref` is `Some("artifacts/contract.md")` (preserved)

#### `test_stage_failed_event_truncates_long_feedback_to_256_chars`
**Given**: A `BeadEvent::StageFailed` event with `feedback = "x".repeat(300)`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::StageFailed)`
- `feedback.len()` == 256
- `feedback.ends_with("...")` (truncation indicator)

#### `test_stage_reentry_event_converts_with_reason`
**Given**: A `BeadEvent::StageReentry` event from "review" to "plan" with reason "major issues"
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::StageReentry)`
- `from_stage` is "review"
- `to_stage` is "plan"
- `reason` is "major issues" (truncated if > 256 chars)

#### `test_validation_ran_event_converts_with_exit_code`
**Given**: A `BeadEvent::ValidationRan` event with `passed = false`, `exit_code = 1`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::ValidationRan)`
- `passed` is `false`
- `exit_code` is `1`
- `output` is truncated to 256 chars if needed

#### `test_recursion_exhausted_event_converts_with_last_stage`
**Given**: A `BeadEvent::RecursionExhausted` event with `total_attempts = 15`, `last_stage = StageKind::Review`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::RecursionExhausted)`
- `total_attempts` is `15`
- `last_stage` is "review" (converted to string)

---

### Zellij IPC Client Receive

#### `test_recv_returns_stage_started_message`
**Given**: stdin contains valid JSON `{"type":"stage_started","bead_id":"bd-3a0a.8","stage":"implement","attempt":1,"timestamp":1739097600}`
**When**: `recv()` is called
**Then**:
- Returns `Ok(HostMessage::StageStarted)`
- `bead_id` is `"bd-3a0a.8"`
- `stage` is `"implement"`
- stdin buffer advanced past message

#### `test_recv_returns_stage_completed_message`
**Given**: stdin contains valid JSON `{"type":"stage_completed","bead_id":"bd-3a0a.8","stage":"implement","artifact_ref":"artifacts/contract.md","timestamp":1739097660}`
**When**: `recv()` is called
**Then**:
- Returns `Ok(HostMessage::StageCompleted)`
- `artifact_ref` is `Some("artifacts/contract.md")`

#### `test_recv_returns_stage_failed_message`
**Given**: stdin contains valid JSON `{"type":"stage_failed","bead_id":"bd-3a0a.8","stage":"validate","feedback":"3 tests failed","severity":"minor","timestamp":1739097720}`
**When**: `recv()` is called
**Then**:
- Returns `Ok(HostMessage::StageFailed)`
- `feedback` is `"3 tests failed"`
- `severity` is `"minor"`

---

### TaskRow Update from IPC

#### `test_update_from_ipc_sets_running_stage_on_stage_started`
**Given**:
- TaskRow with `slug = "bd-3a0a.8"`, `stage = None`
- HostMessage::StageStarted for "bd-3a0a.8" with `stage = "implement"`
**When**: `update_from_ipc()` is called
**Then**:
- `task.stage` is `Some("implement")`
- No other fields modified

#### `test_update_from_ipc_sets_completed_stage_on_stage_completed`
**Given**:
- TaskRow with `slug = "bd-3a0a.8"`, `stage = Some("implement")`
- HostMessage::StageCompleted for "bd-3a0a.8" with `stage = "implement"`
**When**: `update_from_ipc()` is called
**Then**:
- `task.stage` is `Some("implement")` (marks completion)

#### `test_update_from_ipc_sets_failed_stage_with_detail_on_stage_failed`
**Given**:
- TaskRow with `slug = "bd-3a0a.8"`, `stage = Some("validate")`
- HostMessage::StageFailed for "bd-3a0a.8" with `stage = "validate"`, `feedback = "3 tests failed"`
**When**: `update_from_ipc()` is called
**Then**:
- `task.stage` is `Some("validate: 3 tests failed")` (includes detail)

#### `test_update_from_ipc_sets_reentry_stage_on_stage_reentry`
**Given**:
- TaskRow with `slug = "bd-3a0a.8"`, `stage = Some("review")`
- HostMessage::StageReentry for "bd-3a0a.8" with `to_stage = "plan"`, `reason = "major issues"`
**When**: `update_from_ipc()` is called
**Then**:
- `task.stage` is `Some("plan")` (reentered earlier stage)

#### `test_update_from_ipc_is_idempotent_for_same_stage_event`
**Given**:
- TaskRow with `slug = "bd-3a0a.8"`, `stage = Some("implement")`
- HostMessage::StageStarted for "bd-3a0a.8" with `stage = "implement"` (same as current)
**When**: `update_from_ipc()` is called twice
**Then**:
- `task.stage` remains `Some("implement")` (no change)
- No errors returned

---

### Stage Symbol Mapping

#### `test_stage_symbol_returns_running_for_in_progress_stage`
**Given**: `status = "in_progress"`, `stage = Some("implement")`
**When**: `stage_symbol_from_status()` is called
**Then**: Returns `'◐'` (running symbol)

#### `test_stage_symbol_returns_complete_for_passed_status`
**Given**: `status = "passed"`, `stage = None`
**When**: `stage_symbol_from_status()` is called
**Then**: Returns `'●'` (complete symbol)

#### `test_stage_symbol_returns_failed_for_failed_status_with_stage`
**Given**: `status = "failed"`, `stage = Some("validate: 3 tests failed")`
**When**: `stage_symbol_from_status()` is called
**Then**: Returns `'✗'` (failed symbol)

#### `test_stage_symbol_returns_pending_for_created_status`
**Given**: `status = "created"`, `stage = None`
**When**: `stage_symbol_from_status()` is called
**Then**: Returns `'○'` (pending symbol)

#### `test_stage_symbol_returns_question_mark_for_unknown_stage_name`
**Given**: `status = "in_progress"`, `stage = Some("unknown-stage")`
**When**: `stage_symbol_from_status()` is called
**Then**: Returns `'?'` (unknown symbol)

---

### Renderer Integration

#### `test_render_pipeline_view_displays_running_symbol_for_active_stage`
**Given**:
- TaskRow with `status = "in_progress"`, `stage = Some("implement: writing code")`
- Renderer with colors enabled
**When**: `render_pipeline_view()` is called
**Then**:
- Output contains `"◐ ◇ implement [████░░░░]  50%"` (running symbol)
- Output contains `"↳ writing code"` (stage detail on next line)

#### `test_render_pipeline_view_displays_complete_symbol_for_finished_stages`
**Given**:
- TaskRow with `status = "in_progress"`, `stage = Some("lint")`
- Renderer with colors enabled
**When**: `render_pipeline_view()` is called
**Then**:
- Output contains `"● ◇ implement [████████] 100%"` (complete)
- Output contains `"● ∆ unit-test [████████] 100%"` (complete)
- Output contains `"◐ ≋ lint        [████░░░░]  50%"` (running)

#### `test_render_pipeline_view_displays_failed_symbol_for_failed_stage`
**Given**:
- TaskRow with `status = "failed"`, `stage = Some("validate: 3 tests failed")`
- Renderer with colors enabled
**When**: `render_pipeline_view()` is called
**Then**:
- Output contains `"✗ ◎ validate"` (failed symbol)
- Output contains `"↳ 3 tests failed"` (feedback detail)

---

## Error Path Tests

### IPC Worker Event Conversion Errors

#### `test_event_to_host_message_returns_error_for_non_stage_event`
**Given**: A `BeadEvent::Created` event (not a stage lifecycle variant)
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Err(IpcBridgeError::EventSerializationFailed)`
- `event_type` is `"Created"`
- `reason` contains "Not a stage lifecycle event"

#### `test_event_to_host_message_returns_error_for_empty_bead_id`
**Given**: A `BeadEvent::StageStarted` event with `bead_id = ""`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Err(IpcBridgeError::InvalidEventPayload)`
- `missing_field` is `"bead_id"`

#### `test_event_to_host_message_returns_error_for_zero_timestamp`
**Given**: A `BeadEvent::StageStarted` event with `timestamp = DateTime::from_timestamp(0, 0)`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Err(IpcBridgeError::InvalidEventPayload)`
- `missing_field` is `"timestamp"`

---

### Zellij IPC Client Receive Errors

#### `test_recv_returns_error_on_eof()
**Given**: stdin is closed (no data available)
**When**: `recv()` is called
**Then**:
- Returns `Err(IpcError::ConnectionFailed)`
- Error message contains "Connection closed"

#### `test_recv_returns_error_on_invalid_json()
**Given**: stdin contains `"{"` (incomplete JSON)
**When**: `recv()` is called
**Then**:
- Returns `Err(IpcError::Transport)`
- Error message contains "Deserialization failed"

#### `test_recv_returns_error_on_unknown_message_type()
**Given**: stdin contains `'{"type":"unknown_type","bead_id":"bd-3a0a.8"}'`
**When**: `recv()` is called
**Then**:
- Returns `Err(IpcError::Transport)`
- Error message contains "unknown variant"

---

### TaskRow Update Errors

#### `test_update_from_ipc_returns_error_for_bead_not_found`
**Given**:
- Empty UI state (no tasks)
- HostMessage::StageStarted for `bead_id = "bd-3a0a.8"` (not in state)
**When**: `update_from_ipc()` is called
**Then**:
- Returns `Err(TaskUpdateError::BeadNotFound)`
- `bead_id` is `"bd-3a0a.8"`

#### `test_update_from_ipc_returns_error_for_invalid_stage_format()`
**Given**:
- TaskRow with `slug = "bd-3a0a.8"`
- HostMessage::StageStarted with `stage = ""` (empty string)
**When**: `update_from_ipc()` is called
**Then**:
- Returns `Err(TaskUpdateError::InvalidStageFormat)`
- `reason` contains "empty stage name"

---

## Edge Case Tests

### Boundary Conditions

#### `test_event_to_host_message_handles_empty_feedback_gracefully()
**Given**: A `BeadEvent::StageFailed` event with `feedback = ""`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::StageFailed)`
- `feedback` is `""` (empty string preserved)

#### `test_event_to_host_message_handles_256_char_feedback_exactly()`
**Given**: A `BeadEvent::StageFailed` event with `feedback = "x".repeat(256)`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::StageFailed)`
- `feedback.len()` == 256 (no truncation)
- `feedback.ends_with("...")` is `false` (no suffix added)

#### `test_event_to_host_message_handles_257_char_feedback_with_truncation()`
**Given**: A `BeadEvent::StageFailed` event with `feedback = "x".repeat(257)`
**When**: `event_to_host_message()` is called
**Then**:
- Returns `Ok(HostMessage::StageFailed)`
- `feedback.len()` == 256
- `feedback.ends_with("...")` is `true`

---

### Empty State

#### `test_update_from_ipc_handles_empty_task_list()`
**Given**:
- Empty UI state (no tasks)
- Any HostMessage stage event
**When**: `update_from_ipc()` is called
**Then**:
- Returns `Err(TaskUpdateError::BeadNotFound)`
- No panics, no crashes

#### `test_render_pipeline_view_handles_empty_task_list()`
**Given**: Empty task list (`&[]`)
**When**: `render_pipeline_view()` is called
**Then**:
- Returns `"Select a task to view pipeline."`
- No panics, no crashes

---

### Out-of-Order Events

#### `test_update_from_ipc_handles_out_of_order_stage_events()`
**Given**:
- TaskRow with `stage = Some("implement")`
- HostMessage::StageCompleted for "validate" (skips ahead)
**When**: `update_from_ipc()` is called
**Then**:
- `task.stage` is `Some("validate")` (accepts out-of-order)
- No errors returned

#### `test_update_from_ipc_handles_duplicate_stage_started_events()`
**Given**:
- TaskRow with `stage = Some("implement")`
- HostMessage::StageStarted for "implement" (duplicate)
**When**: `update_from_ipc()` is called twice
**Then**:
- `task.stage` remains `Some("implement")` (idempotent)
- No errors returned

---

### Special Characters

#### `test_update_from_ipc_handles_stage_with_newline_in_feedback()`
**Given**: HostMessage::StageFailed with `feedback = "error\nwith\nnewlines"`
**When**: `update_from_ipc()` is called
**Then**:
- Newlines are escaped or truncated (no UI corruption)
- `task.stage` is `Some("validate: error with newlines")` (sanitized)

#### `test_update_from_ipc_handles_stage_with_unicode_in_detail()`
**Given**: HostMessage::StageStarted with `stage = "implement: writing 日本語 code"`
**When**: `update_from_ipc()` is called
**Then**:
- Unicode preserved correctly
- `task.stage` is `Some("implement: writing 日本語 code")`

---

### Concurrent Access

#### `test_event_forwarder_handles_multiple_receivers()`
**Given**:
- Broadcast channel with 3 active receivers
- EventBus emits 100 stage events
**When**: `event_forwarder()` processes events
**Then**:
- All 100 events received by all 3 receivers
- No events dropped (channel buffer = 100)

#### `test_event_forwarder_drops_events_when_buffer_full()`
**Given**:
- Broadcast channel with buffer = 100
- Slow receiver (doesn't consume messages)
- EventBus emits 200 stage events rapidly
**When**: `event_forwarder()` processes events
**Then**:
- Oldest ~100 events dropped (buffer overflow)
- Latest 100 events available to receivers
- No panics or crashes

---

## Contract Verification Tests

### Preconditions

#### `test_precondition_event_bus_subscription_active()
**Given**: EventBus subscription is active
**When**: `event_forwarder()` is called
**Then**:
- Receives events from `subscription.recv().await`
- Forwards to broadcast channel

#### `test_precondition_broadcast_channel_has_capacity()`
**Given**: Broadcast channel with capacity = 100
**When**: 100 messages sent
**Then**:
- All messages queued successfully
- `channel.capacity() > 0`

#### `test_precondition_task_exists_for_update()`
**Given**: UI state contains TaskRow for `bead_id = "bd-3a0a.8"`
**When**: `update_from_ipc()` called with matching bead_id
**Then**:
- Returns `Ok(())`
- Task updated successfully

---

### Postconditions

#### `test_postconversion_host_message_has_valid_fields()`
**Given**: `BeadEvent::StageStarted` with valid fields
**When**: `event_to_host_message()` returns `Ok(msg)`
**Then**:
- `msg.bead_id` is non-empty
- `msg.stage` is non-empty
- `msg.timestamp > 0`

#### `test_postconversion_feedback_truncated_to_max_256_chars()`
**Given**: `BeadEvent::StageFailed` with `feedback = "x".repeat(1000)`
**When**: `event_to_host_message()` returns `Ok(msg)`
**Then**:
- `msg.feedback.len() <= 256`

#### `test_postupdate_task_stage_field_modified()`
**Given**: TaskRow with `stage = None`
**When**: `update_from_ipc()` returns `Ok(())`
**Then**:
- `task.stage` is `Some("implement")` (updated)

---

### Invariants

#### `test_invariant_event_flow_unidirectional()`
**Given**: EventBus → IpcWorker → IPC Channel → Zellij Plugin
**When**: Stage event flows through system
**Then**:
- No reverse flow (plugin never sends stage events)
- All messages flow downstream only

#### `test_invariant_bead_id_mapping_preserved()`
**Given**: `BeadEvent` with `bead_id = "bd-3a0a.8"`
**When**: Converted to `HostMessage`
**Then**:
- `bead_id` is `"bd-3a0a.8"` (string equality, no transformation)

#### `test_invariant_stage_symbol_deterministic()`
**Given**: Same `(status, stage)` tuple
**When**: `stage_symbol_from_status()` called multiple times
**Then**:
- Returns same symbol every time (deterministic)

#### `test_invariant_error_propagation_no_panics()`
**Given**: Any IPC error scenario (EOF, invalid JSON, etc.)
**When**: Error occurs
**Then**:
- Returns `Result<_, IpcError>` (never panics)
- Error logged but system continues

#### `test_invariant_memory_bounded()`
**Given**: Broadcast channel with capacity = 100
**When**: 1000 events sent rapidly
**Then**:
- Memory usage bounded (~100KB for channel)
- No unbounded growth

---

## Given-When-Then Scenarios

### Scenario 1: Stage Progression Through Pipeline

#### Given
- Bead "bd-3a0a.8" in UI state with `status = "created"`, `stage = None`
- EventBus emits stage lifecycle events during execution

#### When
Events arrive in order:
1. `StageStarted { stage: "implement", attempt: 1 }`
2. `StageCompleted { stage: "implement", artifact_ref: Some("artifacts/code.rs") }`
3. `StageStarted { stage: "unit-test", attempt: 1 }`
4. `StageCompleted { stage: "unit-test", artifact_ref: None }`
5. `StageStarted { stage: "coverage", attempt: 1 }`

#### Then
UI renders pipeline view with:
- Row 1: `"● ◇ implement [████████] 100%"` (complete)
- Row 2: `"● ∆ unit-test [████████] 100%"` (complete)
- Row 3: `"◐ ▤ coverage  [████░░░░]  50%"` (running)
- Row 4: `"○ ≋ lint       [░░░░░░░░]   0%"` (pending)

---

### Scenario 2: Stage Failure with Reentry

#### Given
- Bead "bd-3a0a.8" in UI state with `stage = Some("review")`
- EventBus emits failure event

#### When
Events arrive in order:
1. `StageFailed { stage: "review", feedback: "major redesign required", severity: "major" }`
2. `StageReentry { from_stage: "review", to_stage: "plan", reason: "major redesign required", attempt: 2 }`
3. `StageStarted { stage: "plan", attempt: 2 }`

#### Then
UI renders pipeline view with:
- Review row: `"✗ ◌ review"` (failed)
- Plan row: `"◐ ▣ plan     [████░░░░]  50%"` (running, attempt 2)
- Detail line: `"↳ major redesign required"`

---

### Scenario 3: Validation Failure with Feedback

#### Given
- Bead "bd-3a0a.8" in UI state with `stage = Some("validate")`
- EventBus emits validation failure event

#### When
Event arrives:
1. `ValidationRan { passed: false, output: "3 tests failed:\n- test_foo\n- test_bar\n- test_baz", command: "moon run :test", exit_code: 1 }`

#### Then
UI renders pipeline view with:
- Validate row: `"✗ ◎ validate"` (failed)
- Detail line: `"↳ 3 tests failed: - test_foo - t..."` (truncated to fit pane width)

---

### Scenario 4: Recursion Exhausted

#### Given
- Bead "bd-3a0a.8" in UI state with `stage = Some("review")`
- Bead has exceeded recursion limits (15 attempts)

#### When
Event arrives:
1. `RecursionExhausted { total_attempts: 15, last_stage: "review" }`

#### Then
UI renders pipeline view with:
- Review row: `"✗ ◌ review"` (failed)
- Detail line: `"↳ Recursion exhausted after 15 attempts"`
- Status bar: `"OYA UI | Recursion exceeded: bd-3a0a.8 | Contact human"` (alert message)

---

### Scenario 5: IPC Failure Graceful Degradation

#### Given
- Bead "bd-3a0a.8" in UI state with `stage = Some("implement")`
- IPC connection fails (orchestrator crashed)

#### When
1. `recv()` returns `Err(IpcError::ConnectionFailed)` (connection closed)
2. Plugin attempts to reconnect

#### Then
- UI continues to display last known state (`stage = Some("implement")`)
- Status bar shows: `"OYA UI | IPC disconnected | Reconnecting... | q: quit"`
- No panics or crashes
- Plugin remains responsive to user input (navigate, quit, etc.)

---

## Integration Test Scenarios

### End-to-End: EventBus to UI

#### `test_e2e_stage_event_flows_from_event_bus_to_ui()`
**Given**:
- EventBus running with active subscription
- IpcWorker with broadcast channel
- Zellij plugin with IPC client
- TaskRow for "bd-3a0a.8" in UI state

**When**:
1. EventBus emits `BeadEvent::StageStarted { bead_id: "bd-3a0a.8", stage: StageKind::Implement }`
2. IpcWorker receives event via subscription
3. IpcWorker converts to `HostMessage::StageStarted`
4. IpcWorker broadcasts to channel
5. Zellij plugin receives via `recv()`
6. Plugin updates TaskRow via `update_from_ipc()`
7. Plugin renders pipeline view

**Then**:
- UI displays `"◐ ◇ implement [████░░░░]  50%"` (running symbol)
- Stage detail shown: `"↳ Writing code"` (if provided)
- No errors in logs
- Latency < 100ms (event → UI update)

---

## Performance Tests

### Throughput

#### `test_event_forwarder_handles_100_events_per_second()`
**Given**: EventBus emitting 100 stage events/second
**When**: `event_forwarder()` processes events for 10 seconds
**Then**:
- All 1000 events processed
- No events dropped (channel buffer sufficient)
- CPU usage < 50%

#### `test_zellij_ipc_client_handles_1000_messages_per_second()`
**Given**: stdin receiving 1000 HostMessage JSON lines/second
**When**: `recv()` called in loop for 10 seconds
**Then**:
- All 10000 messages parsed successfully
- No buffer overflows
- Memory usage stable (no leaks)

---

### Latency

#### `test_event_to_ui_update_latency_p95_under_100ms()`
**Given**: EventBus to UI rendering pipeline
**When**: 1000 stage events emitted
**Then**:
- p50 latency < 50ms
- p95 latency < 100ms
- p99 latency < 200ms

---

## Test Organization

### Unit Tests
- **File**: `crates/orchestrator/src/actors/ipc_worker_tests.rs`
- **Focus**: Event conversion, error handling, contract verification
- **Isolation**: No external dependencies (mock EventBus, mock IPC)

### Integration Tests
- **File**: `crates/orchestrator/tests/ipc_bridge_integration_test.rs`
- **Focus**: End-to-end event flow, concurrency, failure scenarios
- **Dependencies**: Real EventBus, real IPC channel, mock Zellij plugin

### Property-Based Tests
- **File**: `crates/orchestrator/tests/ipc_bridge_properties.rs`
- **Focus**: Invariants (determinism, memory boundedness, idempotency)
- **Strategy**: Generate random events, verify properties hold

---

## Test Coverage Goals

### Line Coverage
- **Target**: > 90% for IPC bridge code
- **Minimum**: > 80% (acceptable)

### Branch Coverage
- **Target**: > 85% for all error branches
- **Minimum**: > 75% (acceptable)

### Mutation Testing
- **Target**: > 80% mutation score
- **Tool**: cargo-mutants (if available)

---

## Test Execution

### Unit Tests
```bash
# Run all unit tests
moon run :test

# Run only IPC worker tests
moon run :test -- crates/orchestrator/src/actors/ipc_worker_tests.rs
```

### Integration Tests
```bash
# Run all integration tests
moon run :test-integration

# Run only IPC bridge integration
moon run :test-integration -- ipc_bridge_integration_test
```

### Property-Based Tests
```bash
# Run property tests (requires proptest)
moon run :test-property -- ipc_bridge_properties
```

---

## Exit Checklist

- [ ] All happy path tests specified (event conversion, IPC receive, UI update)
- [ ] All error path tests specified (conversion errors, IPC errors, update errors)
- [ ] All edge case tests specified (boundaries, empty state, out-of-order, special chars)
- [ ] All contract verification tests specified (preconditions, postconditions, invariants)
- [ ] All Given-When-Then scenarios specified (5 scenarios covering main flows)
- [ ] Integration test specified (end-to-end event flow)
- [ ] Performance tests specified (throughput, latency)
- [ ] Test organization documented (unit, integration, property)
- [ ] Coverage goals specified (line, branch, mutation)
- [ ] Test execution commands documented
