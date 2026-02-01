//! Error types for OYA operations using Railway-Oriented Programming.
//!
//! All errors are explicit, typed, and recoverable - no panics allowed.

use std::path::PathBuf;

use thiserror::Error;

/// Result type alias for OYA operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for all OYA operations.
#[derive(Debug, Error)]
pub enum Error {
    // Domain validation errors
    #[error("invalid slug: {reason}")]
    InvalidSlug { reason: String },

    #[error("invalid git hash: {reason}")]
    InvalidGitHash { reason: String },

    #[error("unsupported language: {lang} (supported: go, gleam, rust, python, javascript)")]
    UnsupportedLanguage { lang: String },

    #[error("invalid priority: {value} (valid: P1, P2, P3)")]
    InvalidPriority { value: String },

    #[error("unknown stage: {name}")]
    UnknownStage { name: String },

    #[error("stage range error: {reason}")]
    StageRangeError { reason: String },

    // Repository errors
    #[error("not in a git repository")]
    NotInRepo,

    #[error(
        "could not detect language from repository files (looked for gleam.toml, go.mod, Cargo.toml, pyproject.toml, package.json)"
    )]
    LanguageDetectionFailed,

    #[error("could not determine base branch")]
    BaseBranchNotFound,

    #[error("repository is not clean: uncommitted changes")]
    RepoNotClean,

    #[error("directory does not exist: {path}")]
    DirectoryNotFound { path: PathBuf },

    // Persistence errors
    #[error("failed to create directory '{path}': {reason}")]
    DirectoryCreationFailed { path: PathBuf, reason: String },

    #[error("failed to write file '{path}': {reason}")]
    FileWriteFailed { path: PathBuf, reason: String },

    #[error("failed to read file '{path}': {reason}")]
    FileReadFailed { path: PathBuf, reason: String },

    #[error("JSON parse error: {reason}")]
    JsonParseFailed { reason: String },

    #[error("invalid record: {reason}")]
    InvalidRecord { reason: String },

    #[error("task with slug '{slug}' already exists")]
    DuplicateTask { slug: String },

    #[error("task not found: {slug}")]
    TaskNotFound { slug: String },

    // Process execution errors
    #[error("command not found in PATH: {cmd}")]
    CommandNotFound { cmd: String },

    #[error("command failed with exit code {code}: {stderr}")]
    CommandFailed { code: i32, stderr: String },

    #[error("command timeout after {timeout_ms}ms")]
    CommandTimeout { timeout_ms: u64 },

    // Worktree errors
    #[error("worktree already exists: {slug}")]
    WorktreeExists { slug: String },

    #[error("worktree not found: {slug}")]
    WorktreeNotFound { slug: String },

    #[error("failed to create worktree: {reason}")]
    WorktreeCreationFailed { reason: String },

    // Stage execution errors
    #[error("{language}: {stage} failed - {reason}")]
    StageFailed {
        language: String,
        stage: String,
        reason: String,
    },

    // Audit errors
    #[error("failed to write audit entry: {reason}")]
    AuditWriteFailed { reason: String },

    // Database errors
    #[error("database error: {reason}")]
    DatabaseError { reason: String },

    // Core error wrapper
    #[error(transparent)]
    Core(#[from] oya_core::Error),

    // Generic I/O error wrapper
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Create a slug validation error.
    pub fn invalid_slug(reason: impl Into<String>) -> Self {
        Self::InvalidSlug {
            reason: reason.into(),
        }
    }

    /// Create a command failed error.
    pub fn command_failed(code: i32, stderr: impl Into<String>) -> Self {
        Self::CommandFailed {
            code,
            stderr: stderr.into(),
        }
    }

    /// Create a stage failed error.
    pub fn stage_failed(
        language: impl Into<String>,
        stage: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::StageFailed {
            language: language.into(),
            stage: stage.into(),
            reason: reason.into(),
        }
    }

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
