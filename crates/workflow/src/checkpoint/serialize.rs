//! Checkpoint serialization with compression.
//!
//! This module implements the serialization pipeline:
//! 1. Serialize state using bincode
//! 2. Add version header for compatibility
//! 3. Compress using zstd
//!
//! # Architecture
//!
//! Serialization follows Railway-Oriented Programming:
//! - Each step returns `Result<T, SerializeError>`
//! - Errors are propagated with `?` operator
//! - Zero panics, zero unwraps

use serde::Serialize;

/// Version header for checkpoint compatibility.
pub const CHECKPOINT_VERSION: u32 = 1;
const VERSION_HEADER_SIZE: usize = 4;

/// Magic bytes for checkpoint format identification.
const MAGIC_BYTES: &[u8; 8] = b"OYACPT01";

/// Checkpoint serialization errors.
#[derive(Debug, Clone)]
pub enum SerializeError {
    /// Serialization failed.
    SerializationFailed { reason: String },
    /// Compression failed.
    CompressionFailed { reason: String },
    /// Invalid input data.
    InvalidData { reason: String },
}

impl std::fmt::Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationFailed { reason } => {
                write!(f, "serialization failed: {reason}")
            }
            Self::CompressionFailed { reason } => {
                write!(f, "compression failed: {reason}")
            }
            Self::InvalidData { reason } => {
                write!(f, "invalid data: {reason}")
            }
        }
    }
}

impl std::error::Error for SerializeError {}

impl SerializeError {
    /// Create a serialization failed error.
    pub fn serialization_failed(reason: impl Into<String>) -> Self {
        Self::SerializationFailed {
            reason: reason.into(),
        }
    }

    /// Create a compression failed error.
    pub fn compression_failed(reason: impl Into<String>) -> Self {
        Self::CompressionFailed {
            reason: reason.into(),
        }
    }

    /// Create an invalid data error.
    pub fn invalid_data(reason: impl Into<String>) -> Self {
        Self::InvalidData {
            reason: reason.into(),
        }
    }

    /// Check if this error is retryable.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::CompressionFailed { .. } | Self::InvalidData { .. }
        )
    }
}

/// Result type for checkpoint serialization.
pub type SerializeResult<T> = Result<T, SerializeError>;

/// Serialize state to bytes using bincode.
///
/// # Errors
///
/// Returns `SerializeError::SerializationFailed` if bincode serialization fails.
fn serialize_state_bincode<T: Serialize + bincode::Encode>(state: &T) -> SerializeResult<Vec<u8>> {
    bincode::encode_to_vec(state, bincode::config::standard())
        .map_err(|e| SerializeError::serialization_failed(e.to_string()))
}

/// Add version header to serialized data.
///
/// The header consists of:
/// - Magic bytes (8 bytes): "OYACPT01" for format identification
/// - Version number (4 bytes): u32 little-endian
///
/// # Errors
///
/// This function is infallible but returns Result for API consistency.
fn add_version_header(data: Vec<u8>) -> SerializeResult<Vec<u8>> {
    let mut header = Vec::with_capacity(MAGIC_BYTES.len() + VERSION_HEADER_SIZE + data.len());

    // Add magic bytes
    header.extend_from_slice(MAGIC_BYTES);

    // Add version number (little-endian)
    header.extend_from_slice(&CHECKPOINT_VERSION.to_le_bytes());

    // Add serialized data
    header.extend(data);

    Ok(header)
}

/// Compress data using zstd.
///
/// Uses default compression level (3) for balance between speed and ratio.
///
/// # Errors
///
/// Returns `SerializeError::CompressionFailed` if zstd compression fails.
fn compress_data(data: Vec<u8>) -> SerializeResult<Vec<u8>> {
    super::compression::compress(&data)
        .map_err(|e: crate::error::Error| SerializeError::compression_failed(e.to_string()))
}

/// Serialize workflow state to compressed bytes.
///
/// This implements the full serialization pipeline:
/// 1. Serialize using bincode
/// 2. Add version header (magic bytes + version)
/// 3. Compress using zstd
///
/// # Type Parameters
///
/// * `T` - The type to serialize. Must implement `Serialize`.
///
/// # Arguments
///
/// * `state` - Reference to the state to serialize.
///
/// # Returns
///
/// Returns `Ok(Vec<u8>)` with compressed checkpoint data on success.
/// Returns `Err(SerializeError)` if any step fails.
///
/// # Errors
///
/// * `SerializationFailed` - bincode serialization failed
/// * `CompressionFailed` - zstd compression failed
pub fn serialize_state<T: Serialize + bincode::Encode>(state: &T) -> SerializeResult<Vec<u8>> {
    // Step 1: Serialize using bincode
    let serialized = serialize_state_bincode(state)?;

    // Step 2: Add version header
    let with_header = add_version_header(serialized)?;

    // Step 3: Compress using zstd
    let compressed = compress_data(with_header)?;

    Ok(compressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    /// Test: Version header is correctly formatted.
    #[test]
    fn test_add_version_header() {
        let data = b"test data".to_vec();
        let with_header = add_version_header(data).expect("adding header should succeed");

        // Check magic bytes
        assert_eq!(
            &with_header[0..MAGIC_BYTES.len()],
            MAGIC_BYTES,
            "magic bytes should match"
        );

        // Check version number
        let version_bytes =
            &with_header[MAGIC_BYTES.len()..MAGIC_BYTES.len() + VERSION_HEADER_SIZE];
        let version = u32::from_le_bytes(version_bytes.try_into().expect("slice length"));
        assert_eq!(version, CHECKPOINT_VERSION, "version should match");

        // Check data is preserved
        let data_start = MAGIC_BYTES.len() + VERSION_HEADER_SIZE;
        assert_eq!(
            &with_header[data_start..],
            b"test data",
            "original data should be preserved"
        );
    }

    /// Test: Serialization of simple struct.
    #[test]
    fn test_serialize_simple_struct() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestState {
            value: u64,
            name: String,
        }

        let state = TestState {
            value: 42,
            name: "test".to_string(),
        };

        let serialized = serialize_state_bincode(&state);
        assert!(serialized.is_ok(), "serialization should succeed");
        assert!(
            !serialized.unwrap().is_empty(),
            "serialized data should not be empty"
        );
    }

    /// Test: Compression reduces size for repetitive data.
    #[test]
    fn test_compress_reduces_size() {
        let data = vec![42u8; 10000]; // Highly compressible data
        let compressed = compress_data(data);

        assert!(compressed.is_ok(), "compression should succeed");
        assert!(
            compressed.unwrap().len() < 10000,
            "compressed data should be smaller"
        );
    }

    /// Test: Full serialization pipeline.
    #[test]
    fn test_serialize_state_full_pipeline() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct WorkflowState {
            workflows: Vec<String>,
            current_phase: String,
            completed_phases: Vec<String>,
        }

        let state = WorkflowState {
            workflows: vec!["build".to_string(), "test".to_string()],
            current_phase: "build".to_string(),
            completed_phases: vec![],
        };

        let compressed = serialize_state(&state);

        assert!(compressed.is_ok(), "full pipeline should succeed");
        let compressed_data = compressed.unwrap();
        assert!(
            !compressed_data.is_empty(),
            "compressed data should not be empty"
        );
    }
}
