# Contract Specification: Checkpoint Restoration Integration

## Context
- **Feature**: Integrate CheckpointStorage trait with restore_checkpoint pipeline
- **Domain terms**:
  - `CheckpointId`: 16-byte UUID identifier
  - `CheckpointStorage`: Trait for persistent checkpoint storage
  - `RestoreError`: Restoration failure modes
  - `Serialization pipeline`: bincode → version header → zstd compression
- **Assumptions**:
  - `CheckpointStorage` trait is already implemented (InMemoryCheckpointStorage exists)
  - SurrealDB storage implementation will be added later
  - Version header format: magic bytes (8) + version (4) + serialized data
  - Current CHECKPOINT_VERSION is 1
- **Open questions**:
  - None - storage trait is well-defined

## Preconditions
- Storage backend must be initialized and accessible
- Checkpoint ID must be valid (16 bytes)
- Storage layer must contain the checkpoint data
- Checkpoint must be compressed with zstd
- Checkpoint must have valid version header

## Postconditions
- On success: Returns deserialized state of type T
- On failure: Returns RestoreError with specific variant
- Storage is not mutated (read-only operation)
- Version header is validated before deserialization
- Decompressed data must match expected bincode format

## Invariants
- Version header size is constant: 8 (magic) + 4 (version) = 12 bytes
- Magic bytes must be "OYACPT01"
- CHECKPOINT_VERSION must be 1
- Restoration pipeline order is fixed: load → decompress → validate → deserialize
- All errors are propagated without panics
- No data loss: decompressed data must be identical to original serialized state

## Error Taxonomy

### RestoreError::CheckpointNotFound
- **When**: Storage layer returns NotFound error
- **Semantic**: Checkpoint ID doesn't exist in storage
- **Retryable**: No (ID is invalid or checkpoint was deleted)
- **User action**: Verify checkpoint ID or recreate checkpoint

### RestoreError::StorageFailed
- **When**: Storage layer operation fails (network, I/O, database error)
- **Semantic**: Transient or permanent storage failure
- **Retryable**: Yes (if is_retryable() returns true)
- **User action**: Retry or check storage connectivity

### RestoreError::DecompressionFailed
- **When**: zstd decompression fails (corrupted data, invalid format)
- **Semantic**: Checkpoint data is corrupted or not zstd-compressed
- **Retryable**: Yes (transient corruption possible)
- **User action**: Restore from backup or recreate checkpoint

### RestoreError::VersionMismatch
- **When**: Version header doesn't match CHECKPOINT_VERSION
- **Semantic**: Incompatible checkpoint format (old or future version)
- **Retryable**: No (version mismatch is permanent)
- **User action**: Upgrade system or use migration tool

### RestoreError::DeserializationFailed
- **When**: bincode deserialization fails (type mismatch, corrupted data)
- **Semantic**: Data doesn't match expected type T
- **Retryable**: No (type mismatch is permanent)
- **User action**: Verify type T matches checkpoint schema

### RestoreError::InvalidData
- **When**: Data is too small for version header or malformed
- **Semantic**: Checkpoint data is truncated or corrupted
- **Retryable**: No (data corruption is permanent)
- **User action**: Restore from backup or recreate checkpoint

## Contract Signatures

### Main API
```rust
pub fn restore_checkpoint<T: DeserializeOwned + Decode<()>>(
    checkpoint_id: &CheckpointId,
    storage: &dyn CheckpointStorage,
) -> RestoreResult<T>
```

### Internal pipeline steps
```rust
fn load_checkpoint_data(
    checkpoint_id: &CheckpointId,
    storage: &dyn CheckpointStorage,
) -> RestoreResult<Vec<u8>>

fn decompress_checkpoint(compressed: &[u8]) -> RestoreResult<Vec<u8>>

fn validate_version(data: &[u8]) -> RestoreResult<()>

fn deserialize_checkpoint<T: DeserializeOwned + Decode<()>>(
    data: &[u8],
) -> RestoreResult<T>
```

### Storage integration
```rust
// From CheckpointStorage trait
fn load_checkpoint(&self, id: &CheckpointId)
    -> StorageResult<(Vec<u8>, CheckpointMetadata)>
```

## Non-goals
- Not implementing SurrealDB storage (use existing InMemoryCheckpointStorage for tests)
- Not implementing version migration (version mismatch is fatal)
- Not implementing compression/decompression optimization (use existing zstd streaming)
- Not modifying CheckpointStorage trait (it's already defined)
- Not implementing checkpoint deletion (out of scope)
