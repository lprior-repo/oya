//! Core error types for OYA operations using Railway-Oriented Programming.
//!
//! All errors are explicit, typed, and recoverable - no panics allowed.

use std::path::PathBuf;

use thiserror::Error;

/// Core error type for OYA operations.
#[derive(Debug, Error)]
pub enum Error {
    // I/O errors
    #[error("failed to read file '{path}': {reason}")]
    FileReadFailed { path: PathBuf, reason: String },

    #[error("failed to write file '{path}': {reason}")]
    FileWriteFailed { path: PathBuf, reason: String },

    #[error("failed to create directory '{path}': {reason}")]
    DirectoryCreationFailed { path: PathBuf, reason: String },

    #[error("directory does not exist: {path}")]
    DirectoryNotFound { path: PathBuf },

    // Parsing errors
    #[error("JSON parse error: {reason}")]
    JsonParseFailed { reason: String },

    #[error("YAML parse error: {reason}")]
    YamlParseFailed { reason: String },

    #[error("TOML parse error: {reason}")]
    TomlParseFailed { reason: String },

    // Generic errors
    #[error("invalid record: {reason}")]
    InvalidRecord { reason: String },

    #[error("unknown error: {0}")]
    Unknown(String),

    // Generic I/O error wrapper
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Create a file read error.
    pub fn file_read_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::FileReadFailed {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a file write error.
    pub fn file_write_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::FileWriteFailed {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a directory creation error.
    pub fn directory_creation_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::DirectoryCreationFailed {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a JSON parse error.
    pub fn json_parse_failed(reason: impl Into<String>) -> Self {
        Self::JsonParseFailed {
            reason: reason.into(),
        }
    }

    /// Create an invalid record error.
    pub fn invalid_record(reason: impl Into<String>) -> Self {
        Self::InvalidRecord {
            reason: reason.into(),
        }
    }
}
