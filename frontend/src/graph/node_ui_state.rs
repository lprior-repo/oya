//! Node UI state and value objects.
//!
//! Implements Scott Wlaschin DDD principles:
//! - Parse, don't validate
//! - Make illegal states unrepresentable
//! - Types act as documentation

use serde::{Deserialize, Serialize};
use std::fmt;

// ===========================================================================
// Node UI State
// ===========================================================================

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeUiState {
    #[default]
    Default,
    Selected,
    Executing,
    Skipped,
}

impl NodeUiState {
    #[must_use]
    pub const fn is_selected(self) -> bool {
        matches!(self, Self::Selected)
    }

    #[must_use]
    pub const fn is_executing(self) -> bool {
        matches!(self, Self::Executing)
    }

    #[must_use]
    pub const fn is_skipped(self) -> bool {
        matches!(self, Self::Skipped)
    }
}

impl fmt::Display for NodeUiState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Selected => write!(f, "selected"),
            Self::Executing => write!(f, "executing"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

// ===========================================================================
// Value Objects (NewTypes)
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptyStringError;

impl fmt::Display for EmptyStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "string cannot be empty")
    }
}

impl std::error::Error for EmptyStringError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NonEmptyString(String);

impl NonEmptyString {
    #[must_use]
    pub fn new(value: String) -> Option<Self> {
        if value.is_empty() {
            return None;
        }
        Some(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = EmptyStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(EmptyStringError)
    }
}

impl From<NonEmptyString> for String {
    fn from(value: NonEmptyString) -> Self {
        value.0
    }
}

impl std::ops::Deref for NonEmptyString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for NonEmptyString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeLabel(NonEmptyString);

impl NodeLabel {
    #[must_use]
    pub fn new(value: String) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for NodeLabel {
    type Error = EmptyStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        NonEmptyString::try_from(value).map(Self)
    }
}

impl From<NodeLabel> for String {
    fn from(value: NodeLabel) -> Self {
        value.0.into_inner()
    }
}

impl fmt::Display for NodeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ServiceName(NonEmptyString);

impl ServiceName {
    #[must_use]
    pub fn new(value: String) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for ServiceName {
    type Error = EmptyStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        NonEmptyString::try_from(value).map(Self)
    }
}

impl From<ServiceName> for String {
    fn from(value: ServiceName) -> Self {
        value.0.into_inner()
    }
}

impl fmt::Display for ServiceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct StateKey(NonEmptyString);

impl StateKey {
    #[must_use]
    pub fn new(value: String) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for StateKey {
    type Error = EmptyStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        NonEmptyString::try_from(value).map(Self)
    }
}

impl From<StateKey> for String {
    fn from(value: StateKey) -> Self {
        value.0.into_inner()
    }
}

impl fmt::Display for StateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
