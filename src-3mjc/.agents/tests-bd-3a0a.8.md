# Martin Fowler Test Plan: Stage Lifecycle IPC Integration

## Happy Path Tests

### IpcWorker Event Forwarding

- `test_subscribe_to_stage_events_returns_subscription_id_when_event_bus_ready`
  - Given: EventBus is running and accessible
  - When: IpcWorker calls subscribe_to_stage_events()
  - Then: Returns unique subscription ID
  - And: Subscription is active in EventBus registry

- `test_stage_started_event_broadcasts_to_all_connected_clients`
  - Given: EventBus emits StageStarted event for bead-123
  - And: 3 UI clients are connected via IPC
  - When: Event forwarder receives the event
  - Then: All 3 clients receive HostMessage::StageStarted
  - And: Message contains correct bead_id, stage, attempt, timestamp

- `test_stage_completed_event_updates_ui_with_success_symbol`
  - Given: Bead-456 is in Implement stage
  - When: EventBus emits StageCompleted event
  - Then: UI displays checkmark (✓) next to bead-456
  - And: Stage is marked as completed in detail view
  - And: Artifact reference is accessible (if provided)

- `test_stage_failed_event_shows_failure_details_in_ui`
  - Given: Bead-789 fails Validate stage with Major severity
  - When: EventBus emits StageFailed event
  - Then: UI displays X symbol next to bead-789
  - And: Failure reason is visible in detail pane
  - And: Severity is color-coded (yellow for Major)

- `test_stage_reentry_event_updates_ui_symbols_for_both_stages`
  - Given: Bead-101 fails Review and reenters Implement
  - When: EventBus emits StageReentry event
  - Then: Review stage shows reentry symbol (↩)
  - And: Implement stage shows started symbol (🔄)
  - And: Attempt counter is incremented (2nd attempt)

- `test_validation_ran_event_displays_command_results`
  - Given: Bead-202 runs `moon run :ci` validation
  - When: EventBus emits ValidationRan event (passed=true, exit_code=0)
  - Then: UI displays checkmark next to Validate stage
  - And: Command output is viewable in detail pane
  - And: Exit code 0 is displayed

- `test_recursion_exhausted_event_marks_bead_as_blocked`
  - Given: Bead-303 exhausts 15 attempts at Review stage
  - When: EventBus emits RecursionExhausted event
  - Then: UI displays blocked symbol (🚫) next to bead-303
  - And: Bead detail shows "Total attempts: 15"
  - And: Retry button is disabled

### Message Serialization

- `test_stage_started_message_serializes_to_less_than_1kb`
  - Given: StageStarted event with max-length bead_id
  - When: Event is converted to HostMessage and serialized
  - Then: Serialized size < 1024 bytes
  - And: Deserialization produces identical message

- `test_stage_failed_message_truncates_feedback_at_256_chars`
  - Given: StageFailed event with 500-character feedback
  - When: Event is converted to HostMessage
  - Then: Feedback is truncated to 256 characters
  - And: Truncation indicator ("...") is appended

### UI State Management

- `test_multiple_stage_events_update_ui_in_correct_order`
  - Given: Bead-404 receives StageStarted → StageCompleted events
  - When: Both events are processed
  - Then: UI shows completed state (not intermediate started state)
  - And: Event order is preserved (no race conditions)

- `test_ui_displays_correct_stage_symbols_for_all_stages`
  - Given: Bead-505 completes Plan, in Implement, pending Validate
  - When: UI renders bead row
  - Then: Plan shows ✓, Implement shows 🔄, Validate shows ○
  - And: All symbols are aligned and readable

## Error Path Tests

### EventBus Integration

- `test_subscribe_returns_error_when_event_bus_not_ready`
  - Given: EventBus is not started or unreachable
  - When: IpcWorker calls subscribe_to_stage_events()
  - Then: Returns Err(IpcBridgeError::EventBusNotReady)
  - And: Error includes duration since first retry

- `test_event_serialization_failure_drops_event_and_logs`
  - Given: Stage event contains invalid UTF-8 in feedback field
  - When: Event forwarder attempts to convert to HostMessage
  - Then: Event is dropped (not sent to clients)
  - And: Error is logged with event_type and reason

- `test_broadcast_overflow_tracked_and_reported`
  - Given: Event channel capacity is 100
  - And: 150 events are published faster than clients can consume
  - When: Overflow occurs
  - Then: IpcBridgeError::BroadcastOverflow is returned
  - And: dropped_events count is accurate (50)

### IPC Communication

- `test_client_disconnect_stops_sending_to_that_client_only`
  - Given: 2 UI clients connected (Client-A, Client-B)
  - When: Client-A disconnects during stage update
  - Then: Client-B continues receiving events
  - And: Client-A disconnect is logged
  - And: Pending updates for Client-A are discarded

- `test_unknown_bead_id_logged_and_event_dropped`
  - Given: Stage event for bead-999 (not in local cache)
  - When: Event is received
  - Then: Event is dropped (not sent to UI)
  - And: Warning is logged with bead_id
  - And: No panic or crash occurs

### UI Rendering

- `test_no_orchestrator_connection_returns_error_on_stage_update`
  - Given: UI plugin is not connected to orchestrator
  - When: handle_stage_update() is called
  - Then: Returns Err(UiRenderingError::NoOrchestratorConnection)
  - And: UI displays "Disconnected" indicator

- `test_invalid_state_transition_rejected_and_logged`
  - Given: Bead-606 shows Implement as completed
  - When: StageStarted event for Implement arrives (no reentry)
  - Then: Returns Err(UiRenderingError::InvalidTransition)
  - And: UI state is NOT updated
  - And: Error includes from/to states and bead_id

- `test_stage_symbol_not_defined_falls_back_to_question_mark`
  - Given: New StageKind variant added but symbol map not updated
  - When: UI tries to display stage for unknown variant
  - Then: Displays "?" instead of crashing
  - And: Error is logged with stage_kind name

- `test_task_update_failure_does_not_corrupt_other_tasks`
  - Given: UI has 10 tasks in list
  - And: Task at index 5 has corrupted state (nil pointer)
  - When: Stage update for task 5 is processed
  - Then: Other 9 tasks are updated normally
  - And: Task 5 shows error indicator
  - And: UI remains responsive

## Edge Case Tests

### Boundary Values

- `test_attempt_count_overflow_handled_gracefully`
  - Given: Bead-707 has attempt count u32::MAX (4,294,967,295)
  - When: StageReentry event is received
  - Then: Returns Err(IpcBridgeError::AttemptCountOverflow)
  - And: Bead is marked as "Exhausted" instead of incrementing
  - And: User notification is displayed

- `test_empty_feedback_string_serializes_successfully`
  - Given: StageFailed event with empty feedback ("")
  - When: Event is converted to HostMessage
  - Then: Serialization succeeds
  - And: UI shows "Stage failed" with no details

- `test_zero_length_bead_id_rejected`
  - Given: Stage event with bead_id = ""
  - When: Event is processed
  - Then: Event is dropped
  - And: Error is logged: "InvalidEventPayload: missing bead_id"

### Concurrency

- `test_concurrent_stage_events_for_different_beads_do_not_interfere`
  - Given: Bead-808 and Bead-909 both receive stage updates simultaneously
  - When: Events are processed in parallel
  - Then: Both beads update correctly
  - And: No data corruption occurs
  - And: UI renders both updates

- `test_rapid_stage_updates_do_not_cause_ui_flicker`
  - Given: Bead-1010 receives 10 stage updates in 100ms
  - When: UI processes all events
  - Then: Only final state is visible to user
  - And: Intermediate states are debounced
  - And: No visual flickering occurs

### Empty and Null States

- `test_no_ui_clients_connected_does_not_crash_orchestrator`
  - Given: EventBus is active
  - And: No Zellij plugin clients are connected
  - When: Stage events are published
  - Then: Events are logged but no crash occurs
  - And: IpcWorker continues running
  - And: Future clients receive new events

- `test_task_row_with_nil_stage_state_initializes_on_first_event`
  - Given: New bead with no prior stage events
  - When: StageStarted event is received
  - Then: Task row is created with empty stage history
  - And: First stage symbol is displayed
  - And: No "nil pointer" errors occur

### Data Validation

- `test_malformed_timestamp_does_not_crash_ui`
  - Given: Stage event with timestamp = 0 (invalid)
  - When: Event is processed
  - Then: UI displays "Just now" instead of crashing
  - And: Event is logged for investigation

- `test_stage_name_case_sensitivity_normalized`
  - Given: Stage event with stage = "IMPLEMENT" (uppercase)
  - When: Event is processed
  - Then: Stage is normalized to "implement"
  - And: Symbol lookup succeeds
  - And: UI displays correct symbol

## Contract Verification Tests

### Preconditions

- `test_precondition_event_bus_running`
  - Given: EventBus is stopped
  - When: subscribe_to_stage_events() is called
  - Then: Returns Err(IpcBridgeError::EventBusNotReady)
  - And: No subscription is created

- `test_precondition_client_connected_before_broadcast`
  - Given: No IPC clients are connected
  - When: broadcast_stage_update() is called
  - Then: Returns Ok(()) (no-op)
  - And: No panic occurs

### Postconditions

- `test_postcondition_ui_state_updated_after_stage_completed`
  - Given: Bead in "implementing" state
  - When: StageCompleted event processed
  - Then: UI state = "implement: completed"
  - And: Stage symbol = ✓
  - And: Timestamp is recorded

- `test_postcondition_all_clients_receive_same_message`
  - Given: 5 clients connected
  - When: Stage update is broadcast
  - Then: All 5 clients receive identical HostMessage
  - And: Message order is preserved across clients

### Invariants

- `test_invariant_event_ordering_preserved_for_single_bead`
  - Given: Bead-1212 receives events: Started → Failed → Reentry → Started
  - When: Events are processed
  - Then: UI receives events in exact order
  - And: No reordering occurs (FIFO guaranteed)

- `test_invariant_attempt_count_never_decreases`
  - Given: Bead-1313 has attempt = 3
  - When: StageStarted event with attempt = 2 is received
  - Then: Event is rejected (invalid attempt)
  - And: Warning is logged
  - And: Attempt count remains at 3

- `test_invariant_message_size_never_exceeds_1kb`
  - Given: Stage event with maximum valid data
  - When: Serialized to bincode
  - Then: Size ≤ 1024 bytes
  - And: If size would exceed, data is truncated

- `test_invariant_bead_exists_before_stage_events`
  - Given: Stage event for bead-1414 (not created yet)
  - When: Event is received
  - Then: Event is dropped
  - And: Error logged: "BeadNotFound"
  - And: No placeholder bead is created

## Given-When-Then Scenarios

### Scenario 1: Successful Stage Completion Flow

**Given:**
- Bead "oya-123" is in Plan stage
- UI client is connected via IPC
- EventBus is running

**When:**
- Plan stage completes
- EventBus emits StageCompleted event
- Event forwarder receives and converts to HostMessage

**Then:**
- UI client receives HostMessage::StageCompleted
- Bead row shows: "✓ Plan 🔄 Implement"
- Attempt counter is not incremented (success)
- Detail view shows completion timestamp
- Artifact ref (if any) is clickable

### Scenario 2: Stage Failure with Reentry

**Given:**
- Bead "oya-234" is in Validate stage (attempt 1)
- Validation fails with "lint errors"
- Severity = Major

**When:**
- EventBus emits StageFailed event
- Then emits StageReentry event (Validate → Implement)
- Attempt counter increments to 2

**Then:**
- UI shows: "✓ Plan ✓ Implement ✗ Validate ↩"
- Validate stage displays red X
- Implement stage displays "🔄 (2nd attempt)"
- Detail view shows failure reason
- User can click to see full error output

### Scenario 3: Recursion Exhaustion

**Given:**
- Bead "oya-345" has failed Review stage 15 times
- Recursion limit is 15 attempts

**When:**
- EventBus emits RecursionExhausted event
- With total_attempts = 15, last_stage = "review"

**Then:**
- UI displays "🚫" symbol next to bead
- All stage symbols are grayed out
- Detail view shows "Recursion exhausted after 15 attempts"
- Retry button is disabled (grayed out)
- Bead status = "blocked"

### Scenario 4: UI Reconnects During Active Stage

**Given:**
- Bead "oya-456" is in Implement stage
- UI client disconnects
- 3 more stage events occur (Implement → Validate → Review)

**When:**
- UI client reconnects
- Sends GetBeadList request

**Then:**
- Orchestrator replies with current state (Review stage)
- UI displays correct current state
- No attempt is made to replay missed events (simplified)
- UI shows "⚠ Reconnected" indicator briefly

### Scenario 5: Multiple Beads Update Simultaneously

**Given:**
- 5 beads are in various stages
- All beads complete their current stage simultaneously

**When:**
- EventBus emits 5 StageCompleted events
- Event forwarder processes all events

**Then:**
- All 5 UI clients receive all 5 updates
- No events are lost (channel capacity respected)
- UI updates all beads atomically
- No visible lag or flicker

### Scenario 6: Validation Command Results

**Given:**
- Bead "oya-567" runs `moon run :ci`
- Command exits with code 1 (tests failed)
- Output = "3 tests failed, 2 passed"

**When:**
- EventBus emits ValidationRan event
- With passed = false, exit_code = 1, output = "3 tests failed..."

**Then:**
- UI displays "✗" next to Validate stage
- Detail view shows: "Command: moon run :ci"
- Detail view shows: "Exit code: 1"
- Detail view shows: "Output: 3 tests failed, 2 passed"
- Output is truncated at 256 chars if longer

## Performance Tests

- `test_100_stage_events_per_second_processed_without_loss`
  - Given: Channel capacity = 100
  - When: 100 stage events are sent in 1 second
  - Then: All 100 events are broadcast
  - And: No overflow errors occur

- `test_ui_render_time_under_16ms_for_50_beads`
  - Given: 50 beads with stage symbols
  - When: UI redraws after stage update
  - Then: Render time < 16ms (60 FPS)
  - And: No frame drops occur

## Integration Tests

- `test_end_to_end_stage_update_from_orchestrator_to_ui`
  - Given: Running orchestrator with EventBus
  - And: Zellij plugin connected via IPC
  - When: Bead completes Plan stage
  - Then: UI displays Plan completion within 100ms
  - And: Full event trace is logged

- `test_reconnection_with_missed_events_shows_current_state`
  - Given: UI disconnected during bead execution
  - And: Bead progressed through 3 stages
  - When: UI reconnects and requests bead list
  - Then: Current stage is displayed (not intermediate states)
  - And: No events are replayed (current state only)
