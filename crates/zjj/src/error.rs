//! Error types for ZJJ operations using Railway-Oriented Programming.
//!
//! All errors are explicit, typed, and recoverable - no panics allowed.

use std::path::PathBuf;

use thiserror::Error;

/// Result type alias for ZJJ operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for all ZJJ operations.
#[derive(Debug, Error)]
pub enum Error {
    // Jujutsu errors
    #[error("jj command failed: {reason}")]
    JjFailed { reason: String },

    #[error("workspace not found: {name}")]
    WorkspaceNotFound { name: String },

    #[error("workspace already exists: {name}")]
    WorkspaceExists { name: String },

    #[error("invalid workspace name: {reason}")]
    InvalidWorkspaceName { reason: String },

    // Zellij errors
    #[error("zellij command failed: {reason}")]
    ZellijFailed { reason: String },

    #[error("session not found: {name}")]
    SessionNotFound { name: String },

    #[error("session already exists: {name}")]
    SessionExists { name: String },

    #[error("tab not found: {name}")]
    TabNotFound { name: String },

    // Beads errors
    #[error("beads not initialized in repository")]
    BeadsNotInitialized,

    #[error("bead not found: {id}")]
    BeadNotFound { id: String },

    #[error("invalid bead format: {reason}")]
    InvalidBeadFormat { reason: String },

    // Contract errors
    #[error("invalid contract: {reason}")]
    InvalidContract { reason: String },

    #[error("contract validation failed: {reason}")]
    ContractValidationFailed { reason: String },

    // Hook errors
    #[error("hook execution failed: {name} - {reason}")]
    HookFailed { name: String, reason: String },

    #[error("invalid hook configuration: {reason}")]
    InvalidHookConfig { reason: String },

    // Watcher errors
    #[error("file watcher error: {reason}")]
    WatcherFailed { reason: String },

    #[error("watch path does not exist: {path}")]
    WatchPathNotFound { path: PathBuf },

    // Configuration errors
    #[error("configuration error: {reason}")]
    ConfigError { reason: String },

    // Core error wrapper
    #[error(transparent)]
    Core(#[from] oya_core::Error),

    // Generic I/O error wrapper
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // Unknown error
    #[error("unknown error: {0}")]
    Unknown(String),
}

impl Error {
    /// Create a JJ failed error.
    pub fn jj_failed(reason: impl Into<String>) -> Self {
        Self::JjFailed {
            reason: reason.into(),
        }
    }

    /// Create a Zellij failed error.
    pub fn zellij_failed(reason: impl Into<String>) -> Self {
        Self::ZellijFailed {
            reason: reason.into(),
        }
    }

    /// Create a hook failed error.
    pub fn hook_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::HookFailed {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Create a watcher failed error.
    pub fn watcher_failed(reason: impl Into<String>) -> Self {
        Self::WatcherFailed {
            reason: reason.into(),
        }
    }

    /// Create a config error.
    pub fn config_error(reason: impl Into<String>) -> Self {
        Self::ConfigError {
            reason: reason.into(),
        }
    }
}
