# Martin Fowler Test Plan: Checkpoint Restoration Integration

## Happy Path Tests

### test_restores_checkpoint_when_valid_id_provided
**Given**: A checkpoint has been stored with serialized, compressed data
**When**: restore_checkpoint is called with the valid checkpoint ID
**Then**:
- Returns Ok(T) with the original state
- State fields match exactly (round-trip preservation)
- No data loss or corruption

### test_restores_complex_nested_state
**Given**: A checkpoint containing nested structs, Vecs, and Option fields
**When**: restore_checkpoint is called
**Then**:
- All nested structures are preserved
- Vec contents match exactly
- Option fields preserve Some/None correctly
- String fields preserve UTF-8 encoding

### test_restores_multiple_checkpoints_sequentially
**Given**: Multiple checkpoints stored with different IDs
**When**: Each checkpoint is restored sequentially
**Then**:
- Each restoration returns correct independent state
- No state leakage between restorations
- All checkpoints remain accessible

## Error Path Tests

### test_returns_checkpoint_not_found_when_id_invalid
**Given**: Storage doesn't contain the checkpoint ID
**When**: restore_checkpoint is called with non-existent ID
**Then**:
- Returns Err(RestoreError::CheckpointNotFound)
- Error message contains the checkpoint ID
- Storage is not modified

### test_returns_storage_failed_when_storage_unavailable
**Given**: Storage layer returns StorageFailed error
**When**: restore_checkpoint is called
**Then**:
- Returns Err(RestoreError::StorageFailed)
- Error contains operation name ("load") and reason
- Error is marked as retryable (is_retryable() returns true)

### test_returns_decompression_failed_when_data_corrupted
**Given**: Checkpoint data exists but is corrupted (invalid zstd)
**When**: restore_checkpoint attempts decompression
**Then**:
- Returns Err(RestoreError::DecompressionFailed)
- Error message describes the zstd failure
- Error is marked as retryable

### test_returns_version_mismatch_when_header_wrong
**Given**: Checkpoint has version header v99 but current is v1
**When**: validate_version checks the header
**Then**:
- Returns Err(RestoreError::VersionMismatch)
- Error contains expected=1 and found=99
- Error describes incompatibility
- Error is NOT retryable (permanent mismatch)

### test_returns_deserialization_failed_when_type_mismatch
**Given**: Checkpoint contains serialized String but T is u64
**When**: deserialize_checkpoint attempts deserialization
**Then**:
- Returns Err(RestoreError::DeserializationFailed)
- Error message contains bincode error details
- Error is NOT retryable (type mismatch is permanent)

### test_returns_invalid_data_when_truncated
**Given**: Checkpoint data is < 12 bytes (smaller than version header)
**When**: validate_version checks the header
**Then**:
- Returns Err(RestoreError::InvalidData)
- Error message explains "data too small for version header"

## Edge Case Tests

### test_handles_empty_checkpoint_data
**Given**: Checkpoint contains 0 bytes of serialized data (but valid header)
**When**: restore_checkpoint is called
**Then**:
- Successfully deserializes empty state (if T allows)
- Returns Ok(T) with empty/default value

### test_handles_large_checkpoint_data
**Given**: Checkpoint contains 100MB of serialized data
**When**: restore_checkpoint is called
**Then**:
- Successfully decompresses large data
- Memory usage is reasonable (streaming decompression)
- Returns Ok(T) without overflow

### test_handles_zero_uuid_checkpoint_id
**Given**: CheckpointId is all zeros ([0u8; 16])
**When**: restore_checkpoint is called
**Then**:
- Attempts to load from storage (ID is valid format)
- Returns CheckpointNotFound if not stored
- No panic on zero ID

### test_handles_max_uuid_checkpoint_id
**Given**: CheckpointId is all 0xFF ([255u8; 16])
**When**: restore_checkpoint is called
**Then**:
- Attempts to load from storage (ID is valid format)
- Returns CheckpointNotFound if not stored
- No panic on max ID

## Contract Verification Tests

### test_precondition_checkpoint_id_format
**Given**: CheckpointId with 16 bytes
**When**: restore_checkpoint is called
**Then**:
- ID is passed to storage without modification
- No validation panic on 16-byte format

### test_postcondition_state_exact_match
**Given**: Original state with specific field values
**When**: State is serialized, stored, then restored
**Then**:
- Restored state equals original state (PartialEq)
- No field modifications
- No precision loss for floats

### test_invariant_version_header_constant
**Given**: Any valid checkpoint
**When**: validate_version is called
**Then**:
- Header size is always 12 bytes (8 magic + 4 version)
- Magic bytes are always "OYACPT01"
- Version is always CHECKPOINT_VERSION (1)

### test_invariant_pipeline_order_fixed
**Given**: restore_checkpoint called
**When**: Pipeline executes
**Then**:
- Storage load happens first
- Decompression happens second
- Version validation happens third
- Deserialization happens last
- Order is never changed

### test_invariant_zero_panics
**Given**: Any failure scenario (not found, corrupted, wrong version)
**When**: Failure occurs
**Then**:
- Function never panics
- Always returns Err(RestoreError)
- No unwrap() or expect() calls

## Given-When-Then Scenarios

### Scenario 1: Full restoration pipeline success
**Given**:
- A workflow state with workflows=["build", "test"], current_phase="build"
- State is serialized with bincode
- Version header "OYACPT01" + v1 is added
- Data is compressed with zstd level 3
- Compressed data is stored in InMemoryCheckpointStorage

**When**:
- restore_checkpoint::<WorkflowState>(checkpoint_id, &storage) is called

**Then**:
- Returns Ok(WorkflowState)
- Restored state.workflows == vec!["build", "test"]
- Restored state.current_phase == "build"
- No data loss or corruption

### Scenario 2: Version mismatch blocks restoration
**Given**:
- Checkpoint stored with version header v99
- Current CHECKPOINT_VERSION is 1

**When**:
- restore_checkpoint is called

**Then**:
- Returns Err(RestoreError::VersionMismatch { expected: 1, found: 99, reason: "incompatible" })
- Deserialization is NOT attempted (pipeline stops at validation)
- Error.is_retryable() returns false

### Scenario 3: Corrupted compression fails gracefully
**Given**:
- Checkpoint stored with corrupted zstd data (random bytes)
- Checkpoint ID exists in storage

**When**:
- restore_checkpoint is called

**Then**:
- Returns Err(RestoreError::DecompressionFailed)
- zstd error message is preserved in reason field
- Error.is_retryable() returns true
- No panic, no abort

### Scenario 4: Missing checkpoint returns clear error
**Given**:
- Checkpoint ID "abc-123" doesn't exist in storage
- Storage is otherwise healthy

**When**:
- restore_checkpoint is called

**Then**:
- Returns Err(RestoreError::CheckpointNotFound { checkpoint_id: "abc-123" })
- Error message is user-friendly: "checkpoint 'abc-123' not found"
- No database panic, no null pointer

### Scenario 5: Type mismatch detected during deserialization
**Given**:
- Checkpoint contains serialized String "hello"
- Code calls restore_checkpoint::<u64>(checkpoint_id, &storage)

**When**:
- restore_checkpoint is called

**Then**:
- Returns Err(RestoreError::DeserializationFailed)
- Error message contains bincode type error details
- Error.is_retryable() returns false (type can't change)
- No unsafe transmute, no UB

## Integration Tests

### test_roundtrip_with_in_memory_storage
**Given**: InMemoryCheckpointStorage with stored checkpoint
**When**: Full roundtrip (serialize → store → load → deserialize)
**Then**: State matches exactly

### test_restoration_after_storage_clear
**Given**: Storage with checkpoint, then cleared
**When**: restore_checkpoint called
**Then**: Returns CheckpointNotFound

## Performance Tests

### test_restoration_performance_large_state
**Given**: 10MB serialized state
**When**: restore_checkpoint called
**Then**:
- Completes in < 5 seconds (reasonable for large state)
- Memory usage is bounded (streaming decompression)

## Regression Tests

### test_no_memory_leak_on_restoration_failure
**Given**: Multiple failed restoration attempts
**When**: Memory is monitored
**Then**: No memory leak (proper cleanup on errors)

### test_concurrent_restoration_different_ids
**Given**: Two threads restoring different checkpoints
**When**: Both call restore_checkpoint concurrently
**Then**: Both succeed independently (no race conditions)
