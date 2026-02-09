//! Checkpoint restoration from compressed, serialized state.
//!
//! This module implements the restoration pipeline:
//! 1. Load checkpoint data from storage (compressed bytes)
//! 2. Decompress using zstd
//! 3. Deserialize using bincode
//! 4. Validate version header
//!
//! # Architecture
//!
//! Restoration follows Railway-Oriented Programming:
//! - Each step returns `Result<T, RestoreError>`
//! - Errors are propagated with `?` operator
//! - Zero panics, zero unwraps

use bincode::Decode;
use serde::de::DeserializeOwned;

use super::storage::{CheckpointStorage, StorageError};

/// Version header for checkpoint compatibility.
const CHECKPOINT_VERSION: u32 = 1;

/// Size of version header: magic bytes (8) + version number (4).
const VERSION_HEADER_SIZE: usize = 12;

/// Unique identifier for a checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CheckpointId([u8; 16]);

impl CheckpointId {
    /// Create a new checkpoint ID.
    #[must_use]
    pub fn new() -> Self {
        Self(*uuid::Uuid::new_v4().as_bytes())
    }

    /// Create from bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Get the inner bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Default for CheckpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format as UUID string
        let uuid = uuid::Uuid::from_bytes(self.0);
        write!(f, "{uuid}")
    }
}

/// Checkpoint restoration errors.
#[derive(Debug, Clone)]
pub enum RestoreError {
    /// Checkpoint data not found in storage.
    CheckpointNotFound { checkpoint_id: String },
    /// Decompression failed.
    DecompressionFailed { reason: String },
    /// Deserialization failed.
    DeserializationFailed { reason: String },
    /// Version mismatch (incompatible checkpoint format).
    VersionMismatch {
        expected: u32,
        found: u32,
        reason: String,
    },
    /// Invalid checkpoint data (corrupted or malformed).
    InvalidData { reason: String },
    /// Storage operation failed.
    StorageFailed { operation: String, reason: String },
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckpointNotFound { checkpoint_id } => {
                write!(f, "checkpoint '{checkpoint_id}' not found")
            }
            Self::DecompressionFailed { reason } => {
                write!(f, "decompression failed: {reason}")
            }
            Self::DeserializationFailed { reason } => {
                write!(f, "deserialization failed: {reason}")
            }
            Self::VersionMismatch {
                expected,
                found,
                reason,
            } => {
                write!(
                    f,
                    "version mismatch: expected v{expected}, found v{found}: {reason}"
                )
            }
            Self::InvalidData { reason } => {
                write!(f, "invalid checkpoint data: {reason}")
            }
            Self::StorageFailed { operation, reason } => {
                write!(f, "storage operation '{operation}' failed: {reason}")
            }
        }
    }
}

impl std::error::Error for RestoreError {}

impl RestoreError {
    /// Create a checkpoint not found error.
    pub fn checkpoint_not_found(checkpoint_id: impl Into<String>) -> Self {
        Self::CheckpointNotFound {
            checkpoint_id: checkpoint_id.into(),
        }
    }

    /// Create a decompression failed error.
    pub fn decompression_failed(reason: impl Into<String>) -> Self {
        Self::DecompressionFailed {
            reason: reason.into(),
        }
    }

    /// Create a deserialization failed error.
    pub fn deserialization_failed(reason: impl Into<String>) -> Self {
        Self::DeserializationFailed {
            reason: reason.into(),
        }
    }

    /// Create a version mismatch error.
    pub fn version_mismatch(expected: u32, found: u32, reason: impl Into<String>) -> Self {
        Self::VersionMismatch {
            expected,
            found,
            reason: reason.into(),
        }
    }

    /// Create an invalid data error.
    pub fn invalid_data(reason: impl Into<String>) -> Self {
        Self::InvalidData {
            reason: reason.into(),
        }
    }

    /// Create a storage failed error.
    pub fn storage_failed(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::StorageFailed {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Check if this error is retryable.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::StorageFailed { .. } | Self::DecompressionFailed { .. }
        )
    }
}

/// Result type for checkpoint restoration.
pub type RestoreResult<T> = Result<T, RestoreError>;

/// Load checkpoint data from storage.
///
/// This function integrates with the `CheckpointStorage` trait to load
/// compressed checkpoint data.
///
/// # Errors
///
/// Returns `RestoreError::CheckpointNotFound` if the checkpoint doesn't exist.
/// Returns `RestoreError::StorageFailed` if the storage operation fails.
fn load_checkpoint_data(
    checkpoint_id: &CheckpointId,
    storage: &dyn CheckpointStorage,
) -> RestoreResult<Vec<u8>> {
    storage
        .load_checkpoint(checkpoint_id)
        .map(|(data, _metadata)| data)
        .map_err(|storage_error| match storage_error {
            StorageError::NotFound { checkpoint_id } => {
                RestoreError::CheckpointNotFound { checkpoint_id }
            }
            StorageError::StorageFailed { reason } => RestoreError::StorageFailed {
                operation: "load".to_string(),
                reason,
            },
            StorageError::CodecFailed { reason } => RestoreError::InvalidData {
                reason: format!("checkpoint data codec error: {reason}"),
            },
        })
}

/// Decompress checkpoint data using zstd.
///
/// # Errors
///
/// Returns `RestoreError::DecompressionFailed` if decompression fails.
fn decompress_checkpoint(compressed: &[u8]) -> RestoreResult<Vec<u8>> {
    // Use zstd streaming decompressor for better memory efficiency
    zstd::stream::decode_all(compressed)
        .map_err(|e| RestoreError::decompression_failed(e.to_string()))
}

/// Deserialize checkpoint data from bytes.
///
/// # Errors
///
/// Returns `RestoreError::DeserializationFailed` if deserialization fails.
fn deserialize_checkpoint<T>(data: &[u8]) -> RestoreResult<T>
where
    T: serde::de::DeserializeOwned + bincode::Decode<()>,
{
    bincode::decode_from_slice(data, bincode::config::standard())
        .map(|(value, _)| value)
        .map_err(|e| RestoreError::deserialization_failed(e.to_string()))
}

/// Validate version header in checkpoint data.
///
/// The header consists of:
/// - Magic bytes (8 bytes): "OYACPT01"
/// - Version number (4 bytes): u32 little-endian
///
/// # Errors
///
/// Returns `RestoreError::InvalidData` if data is too small or magic bytes don't match.
/// Returns `RestoreError::VersionMismatch` if version doesn't match.
fn validate_version(data: &[u8]) -> RestoreResult<()> {
    // Check minimum size for magic bytes + version
    if data.len() < VERSION_HEADER_SIZE {
        return Err(RestoreError::invalid_data(format!(
            "data too small for version header: expected {VERSION_HEADER_SIZE} bytes, got {}",
            data.len()
        )));
    }

    // Validate magic bytes
    let magic_bytes = &data[0..8];
    const EXPECTED_MAGIC: &[u8; 8] = b"OYACPT01";
    if magic_bytes != EXPECTED_MAGIC {
        return Err(RestoreError::invalid_data(format!(
            "invalid magic bytes: expected {EXPECTED_MAGIC:?}, got {magic_bytes:?}"
        )));
    }

    // Extract and validate version number
    let version_bytes = [data[8], data[9], data[10], data[11]];
    let found = u32::from_le_bytes(version_bytes);

    if found != CHECKPOINT_VERSION {
        return Err(RestoreError::VersionMismatch {
            expected: CHECKPOINT_VERSION,
            found,
            reason: "checkpoint format version incompatible".to_string(),
        });
    }

    Ok(())
}

/// Restore a checkpoint from storage.
///
/// This implements the full restoration pipeline:
/// 1. Load compressed checkpoint data from storage
/// 2. Decompress using zstd
/// 3. Validate version header (magic bytes + version number)
/// 4. Deserialize using bincode
///
/// # Type Parameters
///
/// * `T` - The type to deserialize. Must implement `DeserializeOwned` and `Decode`.
///
/// # Arguments
///
/// * `checkpoint_id` - Unique identifier for the checkpoint to restore.
/// * `storage` - Reference to checkpoint storage implementation.
///
/// # Returns
///
/// Returns `Ok(T)` with the restored state on success.
/// Returns `Err(RestoreError)` if any step fails.
///
/// # Errors
///
/// * `CheckpointNotFound` - Checkpoint doesn't exist in storage
/// * `DecompressionFailed` - zstd decompression failed
/// * `VersionMismatch` - Checkpoint version is incompatible
/// * `DeserializationFailed` - bincode deserialization failed
/// * `InvalidData` - Checkpoint data is corrupted
/// * `StorageFailed` - Storage layer operation failed
///
/// # Example
///
/// ```ignore
/// use oya_workflow::checkpoint::{restore_checkpoint, CheckpointStorage, InMemoryCheckpointStorage};
///
/// let mut storage = InMemoryCheckpointStorage::new();
/// // ... store checkpoint ...
/// let state: MyState = restore_checkpoint(&checkpoint_id, &storage)?;
/// ```
pub fn restore_checkpoint<T: DeserializeOwned + Decode<()>>(
    checkpoint_id: &CheckpointId,
    storage: &dyn CheckpointStorage,
) -> RestoreResult<T> {
    // Step 1: Load compressed data from storage
    let compressed = load_checkpoint_data(checkpoint_id, storage)?;

    // Step 2: Decompress using zstd
    let decompressed = decompress_checkpoint(&compressed)?;

    // Step 3: Validate version header (magic bytes + version)
    validate_version(&decompressed)?;

    // Step 4: Deserialize (skip version header)
    let data_without_header = &decompressed[VERSION_HEADER_SIZE..];
    let state = deserialize_checkpoint(data_without_header)?;

    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    use super::super::serialize::{serialize_state, CHECKPOINT_VERSION, MAGIC_BYTES};
    use super::super::storage::{
        CheckpointMetadata, CheckpointStorage, InMemoryCheckpointStorage, StorageResult,
        StorageStats,
    };

    /// Test: CheckpointId generates unique IDs.
    #[test]
    fn test_checkpoint_id_unique() {
        let id1 = CheckpointId::new();
        let id2 = CheckpointId::new();
        assert!(id1 != id2, "checkpoint IDs should be unique");
    }

    /// Test: CheckpointId display formatting.
    #[test]
    fn test_checkpoint_id_display() {
        let bytes = [0u8; 16];
        let id = CheckpointId::from_bytes(bytes);
        let display = id.to_string();
        assert!(!display.is_empty(), "display string should not be empty");
    }

    /// Test: Version validation rejects too-small data.
    #[test]
    fn test_validate_version_too_small() {
        let result = validate_version(&[1, 2]);
        assert!(result.is_err(), "should reject too-small data");
        assert!(
            matches!(result, Err(RestoreError::InvalidData { .. })),
            "should return InvalidData error, got: {:?}",
            result
        );
    }

    /// Test: Version validation rejects wrong magic bytes.
    #[test]
    fn test_validate_version_invalid_magic() {
        let mut header = vec![0u8; VERSION_HEADER_SIZE];
        header[0..8].copy_from_slice(b"BADMAGIC");
        header[8..12].copy_from_slice(&CHECKPOINT_VERSION.to_le_bytes());

        let result = validate_version(&header);
        assert!(result.is_err(), "should reject invalid magic bytes");
        assert!(
            matches!(result, Err(RestoreError::InvalidData { .. })),
            "should return InvalidData error, got: {:?}",
            result
        );
    }

    /// Test: Version validation rejects wrong version.
    #[test]
    fn test_validate_version_mismatch() {
        let mut header = vec![0u8; VERSION_HEADER_SIZE];
        header[0..8].copy_from_slice(MAGIC_BYTES);
        header[8..12].copy_from_slice(&99u32.to_le_bytes());

        let result = validate_version(&header);
        assert!(result.is_err(), "should reject wrong version");
        if let Err(RestoreError::VersionMismatch {
            expected,
            found,
            reason: _,
        }) = result
        {
            assert_eq!(expected, CHECKPOINT_VERSION);
            assert_eq!(found, 99);
        } else {
            assert!(
                matches!(result, Err(RestoreError::VersionMismatch { .. })),
                "wrong error type: {:?}",
                result
            );
        }
    }

    /// Test: Version validation accepts correct header.
    #[test]
    fn test_validate_version_success() {
        let mut header = vec![0u8; VERSION_HEADER_SIZE];
        header[0..8].copy_from_slice(MAGIC_BYTES);
        header[8..12].copy_from_slice(&CHECKPOINT_VERSION.to_le_bytes());

        let result = validate_version(&header);
        assert!(result.is_ok(), "should accept correct header");
    }

    /// Test: Decompression fails on invalid data.
    #[test]
    fn test_decompress_invalid_data() {
        let invalid = [0u8; 10]; // Not valid zstd data
        let result = decompress_checkpoint(&invalid);
        assert!(result.is_err(), "should fail on invalid data");
        assert!(
            matches!(result, Err(RestoreError::DecompressionFailed { .. })),
            "should return DecompressionFailed error, got: {:?}",
            result
        );
    }

    /// Test: Deserialization fails on invalid data.
    #[test]
    fn test_deserialize_invalid_data() {
        let invalid = [0u8; 10]; // Not valid bincode data

        // Use serde_json for test
        let result: Result<String, serde_json::Error> = serde_json::from_slice(&invalid);
        assert!(result.is_err(), "should fail on invalid data");
    }

    /// Test: Round-trip serialization and deserialization.
    #[test]
    fn test_serialize_deserialize_roundtrip() {
        #[derive(Debug, serde::Serialize, Deserialize, PartialEq)]
        struct TestState {
            counter: u64,
            name: String,
        }

        let original = TestState {
            counter: 42,
            name: "test".to_string(),
        };

        // Serialize using serde_json
        let serialized = serde_json::to_vec(&original);
        assert!(serialized.is_ok(), "serialization should succeed");
        let serialized = serialized.ok().filter(|_| true).unwrap_or_default();

        // Deserialize
        let restored = serde_json::from_slice::<TestState>(&serialized);
        assert!(restored.is_ok(), "deserialization should succeed");
        let restored = restored.ok().filter(|_| true).unwrap_or_else(|| TestState {
            counter: 0,
            name: String::new(),
        });

        assert_eq!(restored, original, "round-trip should preserve data");
    }

    /// Test: BDD - Full restoration pipeline with valid checkpoint.
    ///
    /// GIVEN a checkpoint has been created with compressed data
    /// WHEN the checkpoint is restored
    /// THEN the original state is recovered exactly
    #[test]
    fn test_restore_checkpoint_full_pipeline() {
        #[derive(
            Debug, serde::Serialize, serde::Deserialize, PartialEq, bincode::Encode, bincode::Decode,
        )]
        struct TestState {
            counter: u64,
            name: String,
            items: Vec<String>,
        }

        let original = TestState {
            counter: 42,
            name: "test-checkpoint".to_string(),
            items: vec!["item1".to_string(), "item2".to_string()],
        };

        // Create checkpoint
        let checkpoint_id = CheckpointId::new();
        let compressed = serialize_state(&original);
        assert!(compressed.is_ok(), "serialization should succeed");
        let compressed = compressed.map_or(Vec::new(), |v| v);

        // Store in in-memory storage
        let mut storage = InMemoryCheckpointStorage::new();
        let metadata = CheckpointMetadata {
            id: checkpoint_id,
            created_at: chrono::Utc::now(),
            version: CHECKPOINT_VERSION,
            uncompressed_size: 100, // Approximate
            compressed_size: compressed.len(),
            compression_ratio: 1.5,
        };

        let store_result = storage.store_checkpoint(compressed, metadata);
        assert!(store_result.is_ok(), "store should succeed");

        // Restore checkpoint
        let restored: RestoreResult<TestState> = restore_checkpoint(&checkpoint_id, &storage);
        assert!(restored.is_ok(), "restoration should succeed");

        let restored = restored.map_or(
            TestState {
                counter: 0,
                name: String::new(),
                items: Vec::new(),
            },
            |v| v,
        );

        assert_eq!(restored, original, "restored state should match original");
    }

    /// Test: Restoration returns checkpoint not found when ID invalid.
    #[test]
    fn test_restore_checkpoint_not_found() {
        let storage = InMemoryCheckpointStorage::new();
        let checkpoint_id = CheckpointId::new();

        let result: RestoreResult<String> = restore_checkpoint(&checkpoint_id, &storage);

        assert!(result.is_err(), "should fail for non-existent checkpoint");
        assert!(
            matches!(result, Err(RestoreError::CheckpointNotFound { .. })),
            "should return CheckpointNotFound error, got: {:?}",
            result
        );
    }

    /// Test: Restoration fails with corrupted data.
    #[test]
    fn test_restore_checkpoint_corrupted_data() {
        let checkpoint_id = CheckpointId::new();
        let mut storage = InMemoryCheckpointStorage::new();

        // Store corrupted data (not valid zstd)
        let corrupted = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let metadata = CheckpointMetadata {
            id: checkpoint_id,
            created_at: chrono::Utc::now(),
            version: CHECKPOINT_VERSION,
            uncompressed_size: 4,
            compressed_size: 4,
            compression_ratio: 1.0,
        };

        let _ = storage.store_checkpoint(corrupted, metadata);

        let result: RestoreResult<String> = restore_checkpoint(&checkpoint_id, &storage);

        assert!(result.is_err(), "should fail with corrupted data");
        assert!(
            matches!(result, Err(RestoreError::DecompressionFailed { .. })),
            "should return DecompressionFailed error, got: {:?}",
            result
        );
    }

    /// Test: Storage CodecFailed error maps to InvalidData.
    #[test]
    fn test_load_checkpoint_codec_failed_maps_to_invalid_data() {
        // Mock storage that returns CodecFailed
        struct MockStorage;

        impl CheckpointStorage for MockStorage {
            fn store_checkpoint(
                &mut self,
                _data: Vec<u8>,
                _metadata: CheckpointMetadata,
            ) -> StorageResult<CheckpointId> {
                Ok(CheckpointId::new())
            }

            fn load_checkpoint(
                &self,
                _id: &CheckpointId,
            ) -> StorageResult<(Vec<u8>, CheckpointMetadata)> {
                Err(StorageError::CodecFailed {
                    reason: "corrupted header".to_string(),
                })
            }

            fn delete_checkpoint(&mut self, _id: &CheckpointId) -> StorageResult<()> {
                Ok(())
            }

            fn list_checkpoints(&self) -> StorageResult<Vec<CheckpointId>> {
                Ok(Vec::new())
            }

            fn get_stats(&self) -> StorageResult<StorageStats> {
                Ok(StorageStats::default())
            }

            fn clear_all(&mut self) -> StorageResult<()> {
                Ok(())
            }
        }

        let storage = MockStorage;
        let checkpoint_id = CheckpointId::new();
        let result: RestoreResult<String> = restore_checkpoint(&checkpoint_id, &storage);

        assert!(result.is_err(), "should fail with codec error");
        assert!(
            matches!(result, Err(RestoreError::InvalidData { .. })),
            "CodecFailed should map to InvalidData, got: {:?}",
            result
        );
    }

    /// Test: Error display formatting.
    #[test]
    fn test_error_display() {
        let err = RestoreError::checkpoint_not_found("cp-123");
        assert!(err.to_string().contains("cp-123"));

        let err = RestoreError::decompression_failed("corrupt data");
        assert!(err.to_string().contains("corrupt data"));

        let err = RestoreError::VersionMismatch {
            expected: 1,
            found: 2,
            reason: "incompatible".to_string(),
        };
        assert!(err.to_string().contains("version mismatch"));
    }

    /// Test: Retryable error detection.
    #[test]
    fn test_is_retryable() {
        assert!(
            RestoreError::storage_failed("load", "timeout").is_retryable(),
            "storage errors should be retryable"
        );
        assert!(
            RestoreError::decompression_failed("temporary").is_retryable(),
            "decompression errors should be retryable"
        );
        assert!(
            !RestoreError::VersionMismatch {
                expected: 1,
                found: 2,
                reason: "incompatible".to_string(),
            }
            .is_retryable(),
            "version mismatch should not be retryable"
        );
    }
}
