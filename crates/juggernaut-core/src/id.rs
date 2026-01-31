//! Strongly-typed identifiers for Juggernaut entities.
//!
//! Uses ULID for sortable, unique IDs.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use ulid::Ulid;

/// Bead identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(Ulid);

/// Workflow identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(Ulid);

/// Phase identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhaseId(Ulid);

/// Event identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(Ulid);

macro_rules! impl_id {
    ($name:ident) => {
        impl $name {
            /// Create a new random ID.
            #[must_use]
            pub fn new() -> Self {
                Self(Ulid::new())
            }

            /// Create from an existing ULID.
            #[must_use]
            pub const fn from_ulid(ulid: Ulid) -> Self {
                Self(ulid)
            }

            /// Get the underlying ULID.
            #[must_use]
            pub const fn as_ulid(&self) -> Ulid {
                self.0
            }

            /// Convert to string representation.
            #[must_use]
            pub fn to_string(&self) -> String {
                self.0.to_string()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = ulid::DecodeError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ulid::from_str(s).map(Self)
            }
        }
    };
}

impl_id!(BeadId);
impl_id!(WorkflowId);
impl_id!(PhaseId);
impl_id!(EventId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_id_creation() {
        let id1 = BeadId::new();
        let id2 = BeadId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_bead_id_display() {
        let id = BeadId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 26); // ULID is 26 characters
    }

    #[test]
    fn test_bead_id_from_str() {
        let id = BeadId::new();
        let s = id.to_string();
        let parsed: Result<BeadId, _> = s.parse();
        assert!(parsed.is_ok());
        if let Ok(parsed_id) = parsed {
            assert_eq!(id, parsed_id);
        }
    }

    #[test]
    fn test_id_serialization() {
        let id = BeadId::new();
        let json = serde_json::to_string(&id);
        assert!(json.is_ok());
        if let Ok(json_str) = json {
            let deserialized: Result<BeadId, _> = serde_json::from_str(&json_str);
            assert!(deserialized.is_ok());
        }
    }

    #[test]
    fn test_workflow_id() {
        let id = WorkflowId::new();
        assert_eq!(id.to_string().len(), 26);
    }

    #[test]
    fn test_phase_id() {
        let id = PhaseId::new();
        assert_eq!(id.to_string().len(), 26);
    }

    #[test]
    fn test_event_id() {
        let id = EventId::new();
        assert_eq!(id.to_string().len(), 26);
    }
}
