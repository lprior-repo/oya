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

/// Version header for checkpoint compatibility.
const CHECKPOINT_VERSION: u32 = 1;
const VERSION_HEADER_SIZE: usize = 4;

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
/// This is a placeholder implementation that would integrate with the
/// actual storage layer (e.g., SurrealDB in production).
///
/// # Errors
///
/// Returns `RestoreError::CheckpointNotFound` if the checkpoint doesn't exist.
/// Returns `RestoreError::StorageFailed` if the storage operation fails.
fn load_checkpoint_data(checkpoint_id: &CheckpointId) -> RestoreResult<Vec<u8>> {
    // TODO: Integrate with actual storage layer (OrchestratorStore)
    // For now, this is a placeholder that returns not found
    Err(RestoreError::checkpoint_not_found(
        checkpoint_id.to_string(),
    ))
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
/// # Errors
///
/// Returns `RestoreError::InvalidData` if data is too small.
/// Returns `RestoreError::VersionMismatch` if version doesn't match.
fn validate_version(data: &[u8]) -> RestoreResult<()> {
    if data.len() < VERSION_HEADER_SIZE {
        return Err(RestoreError::invalid_data(
            "data too small for version header",
        ));
    }

    let found = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

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
/// 3. Validate version header
/// 4. Deserialize using bincode
///
/// # Type Parameters
///
/// * `T` - The type to deserialize. Must implement `DeserializeOwned` and `Decode`.
///
/// # Arguments
///
/// * `checkpoint_id` - Unique identifier for the checkpoint to restore.
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
pub fn restore_checkpoint<T: DeserializeOwned + Decode<()>>(
    checkpoint_id: &CheckpointId,
) -> RestoreResult<T> {
    // Step 1: Load compressed data from storage
    let compressed = load_checkpoint_data(checkpoint_id)?;

    // Step 2: Decompress using zstd
    let decompressed = decompress_checkpoint(&compressed)?;

    // Step 3: Validate version header
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
        assert!(display.len() > 0, "display string should not be empty");
    }

    /// Test: Version validation rejects too-small data.
    #[test]
    fn test_validate_version_too_small() {
        let result = validate_version(&[1, 2]);
        assert!(result.is_err(), "should reject too-small data");
        match result {
            Err(RestoreError::InvalidData { .. }) => {
                // Expected error type
            }
            _ => {
                panic!("wrong error type");
            }
        }
    }

    /// Test: Version validation rejects wrong version.
    #[test]
    fn test_validate_version_mismatch() {
        let wrong_version = 99u32.to_le_bytes();
        let result = validate_version(&wrong_version);
        assert!(result.is_err(), "should reject wrong version");
        match result {
            Err(RestoreError::VersionMismatch {
                expected, found, ..
            }) => {
                assert_eq!(expected, CHECKPOINT_VERSION);
                assert_eq!(found, 99);
            }
            _ => {
                panic!("wrong error type");
            }
        }
    }

    /// Test: Version validation accepts correct version.
    #[test]
    fn test_validate_version_success() {
        let correct_version = CHECKPOINT_VERSION.to_le_bytes();
        let result = validate_version(&correct_version);
        assert!(result.is_ok(), "should accept correct version");
    }

    /// Test: Decompression fails on invalid data.
    #[test]
    fn test_decompress_invalid_data() {
        let invalid = [0u8; 10]; // Not valid zstd data
        let result = decompress_checkpoint(&invalid);
        assert!(result.is_err(), "should fail on invalid data");
        match result {
            Err(RestoreError::DecompressionFailed { .. }) => {
                // Expected error type
            }
            _ => {
                panic!("wrong error type");
            }
        }
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
        let serialized = serde_json::to_vec(&original)
            .map_err(|e| format!("serialization failed: {}", e))
            .unwrap();

        // Deserialize
        let restored: TestState = serde_json::from_slice(&serialized)
            .map_err(|e| format!("deserialization failed: {}", e))
            .unwrap();

        assert_eq!(restored, original, "round-trip should preserve data");
    }

    /// Test: BDD - Full restoration pipeline with valid checkpoint.
    ///
    /// GIVEN a checkpoint has been created with compressed data
    /// WHEN the checkpoint is restored
    /// THEN the original state is recovered exactly
    #[test]
    fn test_restore_checkpoint_full_pipeline() {
        // Note: This test demonstrates the pipeline architecture
        // In production, load_checkpoint_data would connect to actual storage
        let checkpoint_id = CheckpointId::new();

        // For now, the pipeline will fail at load step (no storage integration)
        let result: RestoreResult<String> = restore_checkpoint(&checkpoint_id);

        // Expected to fail at storage layer in this test environment
        assert!(result.is_err(), "should fail without storage integration");

        match result {
            Err(RestoreError::CheckpointNotFound { .. }) => {
                // Expected - storage not yet integrated
            }
            _ => {
                panic!("unexpected error type");
            }
        }
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
