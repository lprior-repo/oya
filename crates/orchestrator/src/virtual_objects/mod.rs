//! Virtual Objects for stateful entity management.
//!
//! This module provides Restate-style Virtual Objects - stateful entities
//! that maintain isolated key-value state and handle messages.
//!
//! # Architecture
//!
//! Virtual Objects provide:
//! 1. Isolated state per entity (K/V store)
//! 2. Message handling with state access
//! 3. State snapshots for recovery
//! 4. Persistence to SurrealDB
//!
//! # Key Types
//!
//! - `VirtualObject`: Trait for implementing stateful objects
//! - `ObjectState`: K/V state store for an object
//! - `ObjectManager`: Manages object lifecycle and routing

// Allow dead_code until this module is fully integrated
#![allow(dead_code)]

mod isolation;
mod object;
mod state;

pub use isolation::{IsolationLevel, ObjectLock, ObjectLockGuard};
pub use object::{ObjectConfig, ObjectHandler, ObjectId, VirtualObject};
pub use state::{ObjectState, StateSnapshot, StateValue};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_id_from_string() {
        let id: ObjectId = "obj-123".into();
        assert_eq!(id.as_str(), "obj-123");
    }

    #[test]
    fn test_state_value_types() {
        let string_val = StateValue::String("hello".to_string());
        assert!(matches!(string_val, StateValue::String(_)));

        let int_val = StateValue::Integer(42);
        assert!(matches!(int_val, StateValue::Integer(42)));
    }

    #[test]
    fn test_isolation_level_default() {
        let level = IsolationLevel::default();
        assert!(matches!(level, IsolationLevel::Serializable));
    }
}
