//! Port types for Restate workflows.
//!
//! Implements Scott Wlaschin DDD principles:
//! - Parse, don't validate
//! - Make illegal states unrepresentable
//! - Types act as documentation

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ===========================================================================
// Port Type
// ===========================================================================

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortType {
    #[default]
    Any,
    Event,
    State,
    Signal,
    #[serde(rename = "flow-control")]
    FlowControl,
    Json,
}

impl fmt::Display for PortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any => "any",
            Self::Event => "event",
            Self::State => "state",
            Self::Signal => "signal",
            Self::FlowControl => "flow-control",
            Self::Json => "json",
        };
        write!(f, "{s}")
    }
}

impl FromStr for PortType {
    type Err = ParsePortTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "any" => Ok(Self::Any),
            "event" => Ok(Self::Event),
            "state" => Ok(Self::State),
            "signal" => Ok(Self::Signal),
            "flow-control" | "flowcontrol" => Ok(Self::FlowControl),
            "json" => Ok(Self::Json),
            _ => Err(ParsePortTypeError::UnknownVariant {
                input: s.to_string(),
                expected: "any, event, state, signal, flow-control, json",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsePortTypeError {
    UnknownVariant { input: String, expected: &'static str },
}

impl std::fmt::Display for ParsePortTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVariant { input, expected } => {
                write!(f, "Unknown PortType '{input}': expected one of {expected}")
            }
        }
    }
}

impl std::error::Error for ParsePortTypeError {}

// ===========================================================================
// Port Type Compatibility
// ===========================================================================

/// Returns true if source and target port types are compatible.
///
/// Compatibility rules:
/// - `Any` matches everything
/// - `Json` matches everything
/// - Specific types only match themselves
#[must_use]
pub fn types_compatible(source: PortType, target: PortType) -> bool {
    if matches!(source, PortType::Any) || matches!(target, PortType::Any) {
        return true;
    }
    if matches!(source, PortType::Json) || matches!(target, PortType::Json) {
        return true;
    }
    source == target
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn any_source_is_compatible_with_any_target() {
        assert!(types_compatible(PortType::Any, PortType::Event));
    }

    #[test]
    fn any_target_accepts_any_source() {
        assert!(types_compatible(PortType::Event, PortType::Any));
    }

    #[test]
    fn json_source_is_compatible_with_any_target() {
        assert!(types_compatible(PortType::Json, PortType::Event));
    }

    #[test]
    fn same_types_are_compatible() {
        assert!(types_compatible(PortType::Event, PortType::Event));
    }

    #[test]
    fn different_specific_types_are_not_compatible() {
        assert!(!types_compatible(PortType::Event, PortType::State));
    }
}
