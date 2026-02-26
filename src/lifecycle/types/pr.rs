use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::BookmarkName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrNumber(u64);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PrNumberError {
    #[error("pr number must be greater than zero")]
    Zero,
}

impl PrNumber {
    /// Creates a pull request number.
    ///
    /// # Errors
    /// Returns `PrNumberError::Zero` when `value` is `0`.
    pub fn new(value: u64) -> Result<Self, PrNumberError> {
        if value == 0 {
            Err(PrNumberError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: PrNumber,
    pub bookmark: BookmarkName,
    pub url: String,
}
