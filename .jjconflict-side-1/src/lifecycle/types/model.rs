use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::MAX_MODEL_LEN;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Model(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModelError {
    #[error("model must not be empty")]
    Empty,
    #[error("model exceeds max length: {len} > {max}")]
    TooLong { len: usize, max: usize },
}

impl Model {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses a model identifier from text.
    ///
    /// # Errors
    /// Returns `ModelError::Empty` for blank input, or
    /// `ModelError::TooLong` for identifiers over 128 chars.
    pub fn parse(input: &str) -> Result<Self, ModelError> {
        let normalized = input.trim();
        if normalized.is_empty() {
            Err(ModelError::Empty)
        } else if normalized.len() > MAX_MODEL_LEN {
            Err(ModelError::TooLong { len: normalized.len(), max: MAX_MODEL_LEN })
        } else {
            Ok(Self(normalized.to_owned()))
        }
    }

    #[must_use]
    pub fn default_model() -> Self {
        Self("zai-coding-plan/glm-5".to_owned())
    }
}
