//! Error types for Intent operations using Railway-Oriented Programming.
//!
//! All errors are explicit, typed, and recoverable - no panics allowed.

use std::path::PathBuf;

use thiserror::Error;

/// Result type alias for Intent operations.
pub type IntentResult<T> = std::result::Result<T, IntentError>;

/// Unified error type for all Intent operations.
#[derive(Debug, Error)]
pub enum IntentError {
    // Validation errors
    #[error("validation error in {field}: {message}")]
    Validation { field: String, message: String },

    // Configuration errors
    #[error("configuration error: {reason}")]
    Config { reason: String },

    #[error("failed to read config file '{path}': {reason}")]
    ConfigRead { path: PathBuf, reason: String },

    #[error("failed to parse config file '{path}': {reason}")]
    ConfigParse { path: PathBuf, reason: String },

    // Spec errors
    #[error("spec not found: {name}")]
    SpecNotFound { name: String },

    #[error("invalid spec: {reason}")]
    InvalidSpec { reason: String },

    // Core error wrapper
    #[error(transparent)]
    Core(#[from] oya_core::Error),

    // Generic I/O error wrapper
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl IntentError {
    /// Create a validation error.
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a config error.
    pub fn config(reason: impl Into<String>) -> Self {
        Self::Config {
            reason: reason.into(),
        }
    }

    /// Create a config error with key context.
    pub fn config_for_key(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Config {
            reason: format!("{}: {}", key.into(), message.into()),
        }
    }

    /// Create a config read error.
    pub fn config_read(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::ConfigRead {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a config parse error.
    pub fn config_parse(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::ConfigParse {
            path: path.into(),
            reason: reason.into(),
        }
    }
}
