# Martin Fowler Test Plan

## Happy Path Tests

### Test: given_valid_queue_item_payload_when_cue_validation_runs_then_validation_succeeds
- **Status**: TO BE WRITTEN
- **Given**: Valid QueueItem with all required fields
- **When**: CUE validation runs
- **Then**: Validation passes (no errors)

### Test: given_valid_lock_payload_when_cue_validation_runs_then_validation_succeeds
- **Status**: TO BE WRITTEN
- **Given**: Valid SessionLock with all required fields
- **When**: CUE validation runs
- **Then**: Validation passes (no errors)

### Test: given_queue_item_with_all_fields_when_validating_then_succeeds
- **Status**: TO BE WRITTEN
- **Given**: QueueItem with id, bead_id, priority, sha, freshness_base_rev
- **When**: validate_queue_item is called
- **Then**: Ok(()) returned

## Error Path Tests

### Test: given_missing_bead_id_in_queue_payload_when_validating_then_failure_reports_missing_key
- **Status**: TO BE WRITTEN
- **Given**: QueueItem missing bead_id field
- **When**: CUE validation runs
- **Then**: Validation fails with missing field error

### Test: given_out_of_range_priority_when_validating_then_failure_reports_bounds_violation
- **Status**: TO BE WRITTEN
- **Given**: QueueItem with priority > 10
- **When**: CUE validation runs
- **Then**: Validation fails with bounds error

### Test: given_invalid_sha_format_when_validating_then_failure_reports_format_error
- **Status**: TO BE WRITTEN
- **Given**: QueueItem with non-hex sha
- **When**: CUE validation runs
- **Then**: Validation fails with format error

### Test: given_empty_lock_token_when_validating_then_failure_reports_missing_field
- **Status**: TO BE WRITTEN
- **Given**: SessionLock with empty token
- **When**: CUE validation runs
- **Then**: Validation fails with missing field error

### Test: given_zero_ttl_lock_when_validating_then_failure_reports_invalid_state
- **Status**: TO BE WRITTEN
- **Given**: SessionLock with ttl_seconds = 0
- **When**: CUE validation runs
- **Then**: Validation fails with invalid state error

## Edge Case Tests

### Test: given_queue_item_at_priority_boundary_when_validating_then_succeeds
- **Status**: TO BE WRITTEN
- **Given**: QueueItem with priority = 1 (minimum valid)
- **When**: Validation runs
- **Then**: Passes

### Test: given_queue_item_at_max_priority_when_validating_then_succeeds
- **Status**: TO BE WRITTEN
- **Given**: QueueItem with priority = 10 (maximum valid)
- **When**: Validation runs
- **Then**: Passes

### Test: given_lock_at_expiration_boundary_when_validating_then_correct
- **Status**: TO BE WRITTEN
- **Given**: SessionLock where expires_at = acquired_at + ttl
- **When**: is_expired checked
- **Then**: Correct expiration status

## Contract Verification Tests

### Test: precondition_schema_file_loadable
- **Status**: VERIFIED (file exists in repo)
- **Check**: cue/queue_schema.cue and cue/lock_schema.cue exist

### Test: precondition_json_serializable
- **Status**: VERIFIED (QueueItem has Serialize derive)
- **Check**: QueueItem can serialize to JSON

### Test: postcondition_validation_produces_errors
- **Status**: TO BE IMPLEMENTED
- **Check**: Invalid inputs produce ValidationError

### Test: invariant_schema_version_explicit
- **Status**: TO BE IMPLEMENTED
- **Check**: Schema has explicit version field

### Test: invariant_all_keys_present
- **Status**: TO BE IMPLEMENTED
- **Check**: Required fields defined in schema

## Implementation Files
- Schema files: `cue/queue_schema.cue`, `cue/lock_schema.cue`
- Validation: Add validation functions in src/types/domain.rs
- Tests: src/types/domain.rs (existing test module)

## Test Execution
All tests run via: `moon run :test`
