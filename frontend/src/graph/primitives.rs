//! Primitives and basic value objects for the graph module.
//!
//! Contains fundamental types used throughout the graph system:
//! - `NodeId`
//! - `PortName`
//! - `NodeCategory`
//! - `Connection`

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ===========================================================================
// Node ID
// ===========================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub Uuid);

impl NodeId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ===========================================================================
// Port Name
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PortName(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptyPortNameError;

impl std::fmt::Display for EmptyPortNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "port name cannot be empty")
    }
}

impl std::error::Error for EmptyPortNameError {}

impl PortName {
    /// Creates a new `PortName` from a string.
    ///
    /// # Errors
    ///
    /// Returns `EmptyPortNameError` if the value is empty or contains only whitespace.
    pub fn new(value: String) -> Result<Self, EmptyPortNameError> {
        if value.trim().is_empty() {
            return Err(EmptyPortNameError);
        }
        Ok(Self(value))
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

impl<S: Into<String>> From<S> for PortName {
    fn from(s: S) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for PortName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ===========================================================================
// Node Category
// ===========================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum NodeCategory {
    Entry,
    Durable,
    State,
    Flow,
    Timing,
    Signal,
}

impl fmt::Display for NodeCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Entry => "entry",
            Self::Durable => "durable",
            Self::State => "state",
            Self::Flow => "flow",
            Self::Timing => "timing",
            Self::Signal => "signal",
        };
        write!(f, "{s}")
    }
}

// ===========================================================================
// Connection
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Connection {
    pub id: Uuid,
    pub source: NodeId,
    pub target: NodeId,
    pub source_port: PortName,
    pub target_port: PortName,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    mod port_name {
        use super::*;

        #[test]
        fn new_valid_port_name() {
            let result = PortName::new("main".to_string());
            assert!(result.is_ok());
            assert_eq!(result.unwrap().0, "main");
        }

        #[test]
        fn new_empty_port_name() {
            let result = PortName::new("".to_string());
            assert!(result.is_err());
        }

        #[test]
        fn new_whitespace_only_port_name() {
            let result = PortName::new("   ".to_string());
            assert!(result.is_err());
        }

        #[test]
        fn new_trims_whitespace() {
            let result = PortName::new("  main  ".to_string());
            assert!(result.is_ok());
            assert_eq!(result.unwrap().0, "  main  ");
        }

        #[test]
        fn from_string_preserves_value() {
            let port = PortName::from("output");
            assert_eq!(port.0, "output");
        }

        #[test]
        fn port_name_display() {
            let port = PortName::from("test-port");
            assert_eq!(format!("{}", port), "test-port");
        }

        #[test]
        fn port_name_as_str() {
            let port = PortName::from("main");
            assert_eq!(port.as_str(), "main");
        }

        #[test]
        fn port_name_into_inner() {
            let port = PortName::from("main");
            assert_eq!(port.into_inner(), "main");
        }
    }
}
