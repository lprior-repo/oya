//! Transport layer errors
//!
//! All errors are recoverable and provide diagnostic context.

use std::io::ErrorKind as IoErrorKind;

/// Transport layer errors.
///
/// All errors are recoverable and provide diagnostic context.
#[derive(Debug, Clone, PartialEq)]
pub enum TransportError {
    /// Message payload exceeds 1MB limit.
    ///
    /// Caused by:
    /// - Attempting to send a message that serializes to >1MB
    /// - Receiving a length prefix > 1MB
    MessageTooLarge {
        /// Actual payload size in bytes
        actual_size: usize,
        /// Maximum allowed size
        max_size: usize,
    },

    /// End of stream reached before complete frame.
    ///
    /// Caused by:
    /// - Remote process terminated
    /// - Pipe/socket closed
    /// - Truncated message
    UnexpectedEof {
        /// Bytes successfully read before EOF
        bytes_read: usize,
        /// Expected bytes (from length prefix)
        expected_bytes: usize,
    },

    /// Length prefix indicates invalid size.
    ///
    /// Caused by:
    /// - Length prefix = 0 (invalid frame)
    /// - Length prefix > 1MB (size limit violation)
    InvalidLength {
        /// Invalid length value
        length: u32,
        /// Reason why length is invalid
        reason: String,
    },

    /// Serialization failed (bincode error).
    ///
    /// Caused by:
    /// - Message contains non-serializable data
    /// - Invalid UTF-8 strings
    /// - Out-of-range numeric values
    SerializationFailed {
        /// Bincode error message
        cause: String,
    },

    /// Deserialization failed (bincode error).
    ///
    /// Caused by:
    /// - Corrupted payload data
    /// - Schema mismatch (version incompatibility)
    /// - Invalid bincode format
    DeserializationFailed {
        /// Bincode error message
        cause: String,
        /// Bytes of payload read
        payload_bytes: usize,
    },

    /// Write operation failed.
    ///
    /// Caused by:
    /// - Broken pipe
    /// - Disk full
    /// - Permission denied
    /// - Stream shutdown
    WriteFailed {
        /// OS error code
        error_code: Option<i32>,
        /// Error kind
        kind: IoErrorKind,
    },

    /// Read operation failed.
    ///
    /// Caused by:
    /// - Read timeout (if configured)
    /// - Stream corruption
    /// - OS-level I/O error
    ReadFailed {
        /// OS error code
        error_code: Option<i32>,
        /// Error kind
        kind: IoErrorKind,
    },
}

impl TransportError {
    /// Create a MessageTooLarge error
    pub fn message_too_large(actual_size: usize, max_size: usize) -> Self {
        Self::MessageTooLarge {
            actual_size,
            max_size,
        }
    }

    /// Create an UnexpectedEof error
    pub fn unexpected_eof(bytes_read: usize, expected_bytes: usize) -> Self {
        Self::UnexpectedEof {
            bytes_read,
            expected_bytes,
        }
    }

    /// Create an InvalidLength error
    pub fn invalid_length(length: u32, reason: impl Into<String>) -> Self {
        Self::InvalidLength {
            length,
            reason: reason.into(),
        }
    }

    /// Create a SerializationFailed error
    pub fn serialization_failed(cause: impl Into<String>) -> Self {
        Self::SerializationFailed {
            cause: cause.into(),
        }
    }

    /// Create a DeserializationFailed error
    pub fn deserialization_failed(cause: impl Into<String>, payload_bytes: usize) -> Self {
        Self::DeserializationFailed {
            cause: cause.into(),
            payload_bytes,
        }
    }

    /// Create a WriteFailed error from std::io::Error
    pub fn write_failed(err: &std::io::Error) -> Self {
        Self::WriteFailed {
            error_code: err.raw_os_error(),
            kind: err.kind(),
        }
    }

    /// Create a ReadFailed error from std::io::Error
    pub fn read_failed(err: &std::io::Error) -> Self {
        Self::ReadFailed {
            error_code: err.raw_os_error(),
            kind: err.kind(),
        }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageTooLarge { actual_size, max_size } => {
                write!(
                    f,
                    "Message too large: {} bytes (max {} bytes)",
                    actual_size, max_size
                )
            }
            Self::UnexpectedEof {
                bytes_read,
                expected_bytes,
            } => {
                write!(
                    f,
                    "Unexpected EOF: {} bytes read, expected {}",
                    bytes_read, expected_bytes
                )
            }
            Self::InvalidLength { length, reason } => {
                write!(f, "Invalid length prefix {}: {}", length, reason)
            }
            Self::SerializationFailed { cause } => {
                write!(f, "Serialization failed: {}", cause)
            }
            Self::DeserializationFailed {
                cause,
                payload_bytes,
            } => {
                write!(
                    f,
                    "Deserialization failed at {} bytes: {}",
                    payload_bytes, cause
                )
            }
            Self::WriteFailed { error_code, kind } => {
                write!(
                    f,
                    "Write failed: {:?} (error code: {:?})",
                    kind, error_code
                )
            }
            Self::ReadFailed { error_code, kind } => {
                write!(
                    f,
                    "Read failed: {:?} (error code: {:?})",
                    kind, error_code
                )
            }
        }
    }
}

impl std::error::Error for TransportError {}

/// Result type for transport operations
pub type TransportResult<T> = Result<T, TransportError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_messages() {
        assert_eq!(
            TransportError::message_too_large(2_000_000, 1_048_576).to_string(),
            "Message too large: 2000000 bytes (max 1048576 bytes)"
        );

        assert_eq!(
            TransportError::unexpected_eof(100, 504).to_string(),
            "Unexpected EOF: 100 bytes read, expected 504"
        );

        assert!(TransportError::invalid_length(0, "zero length")
            .to_string()
            .contains("zero length"));

        assert_eq!(
            TransportError::serialization_failed("test error").to_string(),
            "Serialization failed: test error"
        );

        assert_eq!(
            TransportError::deserialization_failed("test error", 100).to_string(),
            "Deserialization failed at 100 bytes: test error"
        );
    }

    #[test]
    fn test_error_equality() {
        let err1 = TransportError::message_too_large(100, 50);
        let err2 = TransportError::message_too_large(100, 50);
        assert_eq!(err1, err2);

        let err3 = TransportError::message_too_large(200, 50);
        assert_ne!(err1, err3);
    }
}
