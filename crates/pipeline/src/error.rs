use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Slug cannot {0}")]
    InvalidSlug(String),
    #[error("Stage cannot be {0}")]
    InvalidStage(String),
    #[error("Failure reason cannot be {0}")]
    InvalidFailureReason(String),
    #[error("invalid transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
    #[error("invalid stage sequence: {0}")]
    InvalidStageSequence(String),
    #[error("unable to detect language from repository markers")]
    UnknownLanguage,
    #[error("unknown priority: {0}")]
    UnknownPriority(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("failed to read tasks from {path}: {source}")]
    ReadFailure {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write tasks to {path}: {source}")]
    WriteFailure {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse tasks in {path}: {source}")]
    ParseFailure {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("failed to serialize tasks for {path}: {source}")]
    SerializeFailure {
        path: PathBuf,
        source: serde_json::Error,
    },
}
