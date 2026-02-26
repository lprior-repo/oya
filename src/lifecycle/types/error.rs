use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureCategory {
    Validation,
    Workspace,
    Bookmark,
    PullRequest,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureClass {
    Terminal,
    Transient,
}

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleError {
    #[error("terminal {category:?}: {message}")]
    Terminal { category: FailureCategory, message: String },
    #[error("transient {category:?}: {message}")]
    Transient { category: FailureCategory, message: String },
}

impl LifecycleError {
    #[must_use]
    pub fn terminal(category: FailureCategory, message: impl Into<String>) -> Self {
        Self::Terminal { category, message: message.into() }
    }

    #[must_use]
    pub fn transient(category: FailureCategory, message: impl Into<String>) -> Self {
        Self::Transient { category, message: message.into() }
    }

    #[must_use]
    pub fn class(&self) -> FailureClass {
        match self {
            Self::Terminal { .. } => FailureClass::Terminal,
            Self::Transient { .. } => FailureClass::Transient,
        }
    }

    #[must_use]
    pub fn category(&self) -> FailureCategory {
        match self {
            Self::Terminal { category, .. } | Self::Transient { category, .. } => category.clone(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Terminal { message, .. } | Self::Transient { message, .. } => message,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal { .. })
    }
}
