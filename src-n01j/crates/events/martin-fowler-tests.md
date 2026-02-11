# Martin Fowler Test Plan: Event Replay Error Recovery

## Happy Path Tests

### test_applies_event_successfully_on_first_attempt
**Given**: A valid event in correct order, state initialized, retry policy configured
**When**: Event is applied with recovery
**Then**:
- Event is applied to state
- Progress tracker incremented by 1
- No retries attempted
- DLQ remains empty
- Returns `Ok(())`

### test_retries_transient_error_with_exponential_backoff
**Given**: Event that fails with transient error on first 2 attempts
**When**: Event is applied with recovery
**Then**:
- First attempt fails with transient error
- Retry 1 after 100ms delay
- Retry 2 after 200ms delay
- Third attempt succeeds
- Total elapsed time >= 300ms (100ms + 200ms)
- Progress tracker incremented by 1
- Returns `Ok(())`

### test_skips_to_dlq_after_max_retries
**Given**: Event that consistently fails with transient error
**When**: Event is applied with recovery (max_retries = 3, DLQ enabled)
**Then**:
- 3 initial attempts + 3 retries = 6 total attempts
- All 6 attempts fail
- Event sent to DLQ with:
  - Event ID
  - Error details
  - Attempt count = 6
  - Timestamp
- Replay continues to next event
- Progress tracker NOT incremented
- DLQ size = 1
- Returns `Ok(())` (DLQ is not an error)

### test_immediately_skips_permanent_error_to_dlq
**Given**: Event that fails with permanent error (e.g., Serialization error)
**When**: Event is applied with recovery (DLQ enabled)
**Then**:
- 0 retries attempted
- Event immediately sent to DLQ
- Error logged with WARN level
- Replay continues to next event
- Progress tracker NOT incremented
- Returns `Ok(())`

### test_applies_multiple_events_with_mixed_outcomes
**Given**: 10 events where:
  - Events 1-3 succeed
  - Event 4 has transient error (1 retry)
  - Events 5-7 succeed
  - Event 8 has permanent error
  - Events 9-10 succeed
**When**: All events applied with recovery
**Then**:
- Events 1-3, 5-7, 9-10 applied to state (8 total)
- Event 4 retried once then succeeds
- Event 8 in DLQ
- Progress tracker shows 10/10 (all processed)
- DLQ size = 1
- Returns `Ok(())`

### test_completes_replay_with_dlq_events
**Given**: 100 events, 5 of which are poison events
**When**: Full replay with recovery
**Then**:
- 95 events applied to state
- 5 events in DLQ
- Replay state = `Complete { events_processed: 100 }`
- Progress = 100%
- DLQ contains all 5 poison events with metadata

## Error Path Tests

### test_fails_on_transient_error_when_dlq_disabled
**Given**: Event that fails with transient error after max retries
**When**: Event is applied with recovery (DLQ disabled)
**Then**:
- Max retries attempted (3)
- Returns `Err(ReplayError::RetriesExhausted)`
- Replay state = `Failed`
- No DLQ entry created
- Progress tracker shows attempted count

### test_fails_on_permanent_error_when_dlq_disabled
**Given**: Event that fails with permanent error
**When**: Event is applied with recovery (DLQ disabled)
**Then**:
- 0 retries attempted
- Returns `Err(ReplayError::Permanent)`
- Replay state = `Failed`
- No DLQ entry created

### test_fails_on_dlq_write_error
**Given**: Event that needs to go to DLQ, but DLQ write fails
**When**: Event is applied with recovery
**Then**:
- Max retries attempted (3) or permanent error detected
- DLQ write attempted
- Returns `Err(ReplayError::DeadLetterQueueFailed)`
- Replay state = `Failed`
- Event NOT in DLQ

### test_fails_on_invalid_retry_config
**Given**: RetryPolicy with invalid config (e.g., max_retries = -1)
**When**: Event is applied with recovery
**Then**:
- Returns `Err(ReplayError::InvalidConfig)`
- No event application attempted
- No retries attempted

### test_propagates_critical_internal_errors
**Given**: Event application fails with `Error::Internal("critical")`
**When**: Event is applied with recovery
**Then**:
- 0 retries attempted (Internal is permanent)
- If DLQ enabled: event sent to DLQ
- If DLQ disabled: returns `Err(ReplayError::Permanent)`

## Edge Case Tests

### test_handles_empty_event_stream
**Given**: Empty iterator of events
**When**: `apply_events_with_recovery` called
**Then**:
- Returns `Ok(())`
- State unchanged
- Progress tracker shows 0/0
- DLQ empty

### test_handles_single_event_success
**Given**: Single event that succeeds
**When**: Event is applied with recovery
**Then**:
- Event applied to state
- Progress = 100%
- No retries
- DLQ empty

### test_handles_all_events_poison
**Given**: 10 events, all permanent errors
**When**: Full replay with recovery (DLQ enabled)
**Then**:
- 0 events applied to state
- 10 events in DLQ
- Progress = 100% (all processed)
- Replay state = `Complete`
- Returns `Ok(())`

### test_handles_backoff_capping
**Given**: Event that requires many retries (max_retries = 10)
**When**: Event applied with backoff configuration: base=100ms, max=500ms
**Then**:
- Retry delays: 100ms, 200ms, 400ms, 500ms, 500ms, 500ms, ...
- Delay never exceeds 500ms
- All 10 retries attempted
- Total delay >= 2900ms (sum of capped delays)

### test_handles_zero_max_retries
**Given**: RetryPolicy with max_retries = 0
**When**: Event fails with transient error
**Then**:
- 0 retries attempted
- Event sent to DLQ immediately
- Total attempts = 1

### test_handles_progress_tracker_none
**Given**: Event applied with recovery, tracker = None
**When**: Event succeeds
**Then**:
- Event applied to state
- No panic on tracker.update()
- Returns `Ok(())`

### test_handles_dlq_none
**Given**: RetryPolicy with DLQ disabled, dlq = None
**When**: Event fails with permanent error
**Then**:
- Returns `Err(ReplayError::Permanent)`
- No panic on dlq.push()
- Replay state = `Failed`

### test_handles_event_ordering_violation
**Given**: Event out of order (ULID earlier than last applied)
**When**: Event is applied with recovery
**Then**:
- Returns `Err(ReplayError::Permanent)` (ApplyError::OutOfOrder wrapped)
- No retries attempted (ordering errors are permanent)
- If DLQ enabled: event sent to DLQ

### test_handles_concurrent_replay_isolation
**Given**: Two independent replays with separate ApplyContexts
**When**: Both replays process events concurrently
**Then**:
- Each replay maintains separate ApplyContext
- No cross-contamination of last_events
- Both replays complete successfully

## Contract Verification Tests

### test_precondition_event_ordering_validated
**Given**: Event out of order
**When**: `apply_event_with_recovery` called
**Then**:
- `ApplyContext::is_in_order` checked first
- Returns error before any retry logic
- No state mutation

### test_postcondition_retry_count_never_exceeds_max
**Given**: Transient error with max_retries = 3
**When**: Event applied with recovery
**Then**:
- Total attempts = 1 (initial) + 3 (retries) = 4
- Never exceeds 4 attempts
- After 4th failure: DLQ or fail

### test_postcondition_backoff_always_bounded
**Given**: Any retry attempt number
**When**: Backoff delay calculated
**Then**:
- delay >= base_backoff_ms
- delay <= max_backoff_ms
- Formula: delay = min(base * 2^attempt, max)

### test_postcondition_only_successful_events_increment_progress
**Given**: 5 events: 2 succeed, 2 fail then DLQ, 1 permanent DLQ
**When**: All events applied with recovery
**Then**:
- Progress tracker.count() = 2 (only successes)
- Total processed = 5
- DLQ size = 3

### test_postcondition_dlq_contains_only_poison_events
**Given**: 10 events with various outcomes
**When**: Replay completes
**Then**:
- Every DLQ entry has attempt > max_retries OR permanent error
- No successfully applied events in DLQ
- All DLQ events have valid error details

### test_postcondition_state_consistency_maintained
**Given**: State machine starts in `Replaying { events_processed: 0, events_total: 100 }`
**When**: Event applied successfully
**Then**:
- State transitions to `Replaying { events_processed: 1, events_total: 100 }`
- State updated atomically (no partial updates)

### test_postcondition_all_or_nothing_event_application
**Given**: Event that fails mid-application
**When**: Event applied with recovery
**Then**:
- Either event fully applied OR state rolled back
- No partial state mutation
- State consistent before and after attempt

### test_invariant_dlq_consistency_with_enable_flag
**Given**: DLQ disabled in config
**When**: Any event fails (permanent or max retries)
**Then**:
- Returns `Err(ReplayError::RetriesExhausted)` or `Err(ReplayError::Permanent)`
- No DLQ entry created
- Replay fails

## Given-When-Then Scenarios

### Scenario 1: Transient Network Timeout Recovery

**Given**:
- Event store temporarily unavailable (connection timeout)
- RetryPolicy: max_retries=3, base_backoff=100ms, max_backoff=5000ms, DLQ enabled
- Event is valid and in correct order

**When**:
- Event applied with recovery
- First 2 attempts timeout
- Third attempt succeeds

**Then**:
- Total attempts = 3
- Delays: 100ms, 200ms (exponential backoff)
- Event applied to state
- Progress incremented
- DLQ empty
- Total time >= 300ms

### Scenario 2: Corrupted Event Data (Permanent Error)

**Given**:
- Event has invalid/corrupted data
- RetryPolicy: max_retries=3, DLQ enabled

**When**:
- Event applied with recovery
- First attempt fails with `Error::Serialization`

**Then**:
- 0 retries attempted (permanent error)
- Event sent to DLQ immediately
- DLQ entry contains:
  - Event ID
  - Error: "Serialization error: invalid data format"
  - Attempt = 1
  - Timestamp
- Replay continues to next event
- Progress NOT incremented

### Scenario 3: Max Retries Exhausted Without DLQ

**Given**:
- Event store locked (contention)
- RetryPolicy: max_retries=3, DLQ disabled

**When**:
- Event applied with recovery
- All 4 attempts fail (1 initial + 3 retries)

**Then**:
- Returns `Err(ReplayError::RetriesExhausted)`
- Replay state transitions to `Failed`
- DLQ empty (disabled)
- Progress shows attempted event
- Error contains:
  - Event ID
  - Last error details
  - Total attempts = 4

### Scenario 4: Successful Replay with Mixed Events

**Given**:
- 100 events total
- 90 normal events
- 5 transient failures (lock contention, retries succeed)
- 3 permanent failures (corrupted data)
- 2 persistent transient failures (exhaust retries)
- RetryPolicy: max_retries=3, DLQ enabled

**When**:
- Full replay with recovery

**Then**:
- Replay state = `Complete { events_processed: 100 }`
- 95 events applied to state (90 normal + 5 transient recovered)
- 5 events in DLQ (3 permanent + 2 exhausted)
- Progress = 100%
- DLQ contains:
  - 3 permanent errors (attempt = 1)
  - 2 exhausted errors (attempt = 4)
- Total retry delays applied correctly
- All DLQ events logged

### Scenario 5: Backoff Capping at Maximum

**Given**:
- Event that requires 10 retries
- RetryPolicy: base=100ms, max=500ms
- Event store has extreme latency

**When**:
- Event applied with recovery
- All 10 retries fail

**Then**:
- Retry delays: 100ms, 200ms, 400ms, 500ms, 500ms, 500ms, 500ms, 500ms, 500ms, 500ms
- 4 retries capped at 500ms
- Total delay >= 4700ms
- Final attempt #11 (1 initial + 10 retries)
- DLQ or fail based on DLQ enable flag

## Integration Tests

### test_integration_with_replay_state_machine
**Given**: Replay in `Replaying { events_processed: 50, events_total: 100 }`
**When**: Event applied successfully
**Then**: State transitions to `Replaying { events_processed: 51, events_total: 100 }`

### test_integration_with_apply_context
**Given**: ApplyContext with last_event = "event-A"
**When**: Event "event-B" applied successfully
**Then**: ApplyContext.last_event = "event-B"

### test_integration_with_progress_tracker
**Given**: Tracker with events_total=100, update_interval=10
**When**: 10 events applied successfully
**Then**: Progress channel updated with:
- events_processed = 10
- percent_complete = 10.0
- eta = Some(positive duration)

### test_integration_with_checkpoint_resume
**Given**: Replay resumed from checkpoint at event 50
**When**: Events 51-60 applied with recovery
**Then**:
- ApplyContext loaded with last_event = event-50
- Events 51-60 validated in order
- State machine progresses correctly

## Performance Tests

### test_backoff_delays_are_non_blocking
**Given**: Async runtime (tokio)
**When**: Event retry with backoff
**Then**: Backoff uses `tokio::time::sleep` (not blocking `std::thread::sleep`)

### test_concurrent_replays_dont_block_each_other
**Given**: 10 concurrent replays
**When**: All experience transient errors
**Then**: All replays make progress concurrently (no sequential blocking)

### test_dlq_writes_are_fast
**Given**: 1000 poison events
**When**: All sent to DLQ
**Then**: Total DLQ write time < 100ms (async non-blocking)

## Metrics and Observability Tests

### test_logs_dlq_events_at_warn_level
**Given**: Event sent to DLQ
**When**: DLQ push called
**Then**: Log entry at WARN level with:
- Event ID
- Bead ID
- Error message
- Attempt count
- Timestamp

### test_tracks_retry_count_in_metrics
**Given**: Event that requires 2 retries
**When**: Event applied successfully
**Then**: Metric "replay_retry_count" incremented by 2

### test_tracks_dlq_size_metric
**Given**: Replay with 3 poison events
**When**: Replay completes
**Then**: Metric "replay_dlq_size" = 3

### test_tracks_backoff_delay_metric
**Given**: Retry with 200ms backoff
**When**: Backoff executed
**Then**: Metric "replay_backoff_delay_ms" = 200

## Security Tests

### test_dlq_event_data_sanitized
**Given**: Event with sensitive data in payload
**When**: Event sent to DLQ
**Then**: DLQ entry contains:
- Event ID
- Bead ID
- Error
- Redacted/sanitized event data (no sensitive info)

### test_prevents_dlq_injection_attacks
**Given**: Malicious event with SQL injection in error message
**When**: Event sent to DLQ
**Then**: Error message escaped/sanitized
- No SQL executed when DLQ writes to database

### test_validates_event_size_before_dlq_write
**Given**: Event with 10MB payload
**When**: Event sent to DLQ
**Then**:
- Event rejected or truncated
- No OOM from DLQ write
- Returns error or logs warning
