//! UUID v5 idempotency key generation from bead ID + input data.
//!
//! This module provides the complete key generation pipeline that combines:
//! 1. Namespace generation (from `bead_id`)
//! 2. Input hashing (SHA-256)
//! 3. Final UUID v5 key (namespace + hash)
//!
//! # Determinism Guarantee
//!
//! The same `(bead_id, input)` pair always produces the same UUID:
//!
//! ```ignore
//! let key1 = idempotency_key("bead-123", &input);
//! let key2 = idempotency_key("bead-123", &input);
//! assert_eq!(key1, key2); // Always true
//! ```
//!
//! # Algorithm
//!
//! 1. Generate namespace: `UUID v5(DNS_NAMESPACE, bead_id)`
//! 2. Hash input: `SHA-256(serde_json::to_string(input))`
//! 3. Generate key: `UUID v5(namespace, hash)`

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use crate::idempotent::{hash::hash_serializable, namespace::namespace_from_bead};
use serde::Serialize;
use uuid::Uuid;

/// Generates a deterministic idempotency key from bead ID and input.
///
/// This is the main entry point for idempotency key generation. It combines:
/// - A namespace derived from the bead ID (ensures keys are scoped to beads)
/// - A hash of the input data (ensures unique keys for different inputs)
///
/// # Type Parameters
///
/// - `T`: Any type that implements `Serialize` (most types via `#[derive(Serialize)]`)
///
/// # Returns
///
/// - `Result<Uuid, bincode::error::EncodeError>`: The generated idempotency key
///   or an error if serialization fails
///
/// # Examples
///
/// ```
/// use oya_workflow::idempotent::idempotency_key;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct WorkflowInput {
///     task: String,
///     priority: u32,
/// }
///
/// let input = WorkflowInput {
///     task: "build".to_string(),
///     priority: 1,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let key = idempotency_key("bead-123", &input)?;
///
/// // Same input produces same key
/// let key2 = idempotency_key("bead-123", &input)?;
/// assert_eq!(key, key2);
/// # Ok(())
/// # }
/// ```
///
/// # Determinism Properties
///
/// - **Pure function**: No side effects, no randomness
/// - **Deterministic**: Same inputs → same output
/// - **Collision-resistant**: Different inputs → different keys (SHA-256)
/// - **Bead-scoped**: Keys are unique per bead ID
///
/// # Implementation Details
///
/// ```text
/// namespace = UUID v5(DNS_NAMESPACE, bead_id.as_bytes())
/// input_hash = SHA-256(bincode::serialize(input))
/// key = UUID v5(namespace, input_hash)
/// ```
///
/// # Error Handling
///
/// Returns `bincode::error::EncodeError` if input cannot be serialized.
/// This is a pure function - no other failure modes exist.
///
/// # Errors
///
/// Returns `bincode::error::EncodeError` if the input data cannot be serialized.
pub fn idempotency_key<T: Serialize>(
    bead_id: &str,
    input: &T,
) -> Result<Uuid, bincode::error::EncodeError> {
    let namespace = namespace_from_bead(bead_id);
    let input_hash = hash_serializable(input)?;
    Ok(Uuid::new_v5(&namespace, &input_hash))
}

/// Generates a deterministic idempotency key from bead ID and raw bytes.
///
/// This is a lower-level API for cases where you already have raw byte data
/// and want to avoid serialization overhead.
///
/// # Examples
///
/// ```
/// use oya_workflow::idempotency_key_from_bytes;
///
/// let data = b"raw workflow input data";
/// let key = idempotency_key_from_bytes("bead-456", data);
///
/// // Same data produces same key
/// let key2 = idempotency_key_from_bytes("bead-456", data);
/// assert_eq!(key, key2);
/// ```
///
/// # When to Use
///
/// - When input is already bytes (no serialization needed)
/// - For performance-critical paths (avoids serialization overhead)
/// - When working with binary protocols
#[must_use]
pub fn idempotency_key_from_bytes(bead_id: &str, input: &[u8]) -> Uuid {
    let namespace = namespace_from_bead(bead_id);
    let input_hash = crate::idempotent::hash::hash_input(input);
    Uuid::new_v5(&namespace, &input_hash)
}

/// Generates a deterministic idempotency key from bead ID and JSON string.
///
/// This is useful when you already have JSON-formatted input data and
/// want to avoid re-serializing to Rust types.
///
/// # Examples
///
/// ```
/// use oya_workflow::idempotency_key_from_json;
///
/// let json = r#"{"task":"build","priority":1}"#;
/// let key = idempotency_key_from_json("bead-789", json);
///
/// // Same JSON produces same key
/// let key2 = idempotency_key_from_json("bead-789", json);
/// assert_eq!(key, key2);
/// ```
#[must_use]
pub fn idempotency_key_from_json(bead_id: &str, json: &str) -> Uuid {
    idempotency_key_from_bytes(bead_id, json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestInput {
        bead_id: String,
        phase: String,
        data: Vec<u32>,
    }

    #[test]
    fn test_idempotency_key_determinism() -> Result<(), Box<dyn std::error::Error>> {
        let bead_id = "bead-test-123";
        let input = TestInput {
            bead_id: bead_id.to_string(),
            phase: "build".to_string(),
            data: vec![1, 2, 3],
        };

        let key1 = idempotency_key(bead_id, &input)?;
        let key2 = idempotency_key(bead_id, &input)?;

        assert_eq!(key1, key2, "Same input must produce same key");
        Ok(())
    }

    #[test]
    fn test_different_beads_produce_different_keys() -> Result<(), Box<dyn std::error::Error>> {
        let input = TestInput {
            bead_id: "shared".to_string(),
            phase: "test".to_string(),
            data: vec![1, 2, 3],
        };

        let key1 = idempotency_key("bead-001", &input)?;
        let key2 = idempotency_key("bead-002", &input)?;

        assert_ne!(key1, key2, "Different bead IDs must produce different keys");
        Ok(())
    }

    #[test]
    fn test_different_inputs_produce_different_keys() -> Result<(), Box<dyn std::error::Error>> {
        let bead_id = "bead-shared";

        let input1 = TestInput {
            bead_id: bead_id.to_string(),
            phase: "build".to_string(),
            data: vec![1, 2, 3],
        };

        let input2 = TestInput {
            bead_id: bead_id.to_string(),
            phase: "test".to_string(),
            data: vec![4, 5, 6],
        };

        let key1 = idempotency_key(bead_id, &input1)?;
        let key2 = idempotency_key(bead_id, &input2)?;

        assert_ne!(key1, key2, "Different inputs must produce different keys");
        Ok(())
    }

    #[test]
    fn test_key_version() {
        let key = idempotency_key_from_bytes("bead-test", b"test data");

        assert_eq!(
            key.get_version(),
            Some(uuid::Version::Sha1),
            "Generated key must be UUID v5 (SHA-1)"
        );
    }

    #[test]
    fn test_key_variant() {
        let key = idempotency_key_from_bytes("bead-test", b"test data");

        assert_eq!(
            key.get_variant(),
            uuid::Variant::RFC4122,
            "Generated key must use RFC 4122 variant"
        );
    }

    #[test]
    fn test_idempotency_key_from_bytes_determinism() {
        let bead_id = "bead-bytes-test";
        let data = b"raw input data";

        let key1 = idempotency_key_from_bytes(bead_id, data);
        let key2 = idempotency_key_from_bytes(bead_id, data);

        assert_eq!(key1, key2, "Same bytes must produce same key");
    }

    #[test]
    fn test_idempotency_key_from_json_determinism() {
        let bead_id = "bead-json-test";
        let json = r#"{"name":"test","value":42}"#;

        let key1 = idempotency_key_from_json(bead_id, json);
        let key2 = idempotency_key_from_json(bead_id, json);

        assert_eq!(key1, key2, "Same JSON must produce same key");
    }

    #[test]
    fn test_json_whitespace_affects_key() {
        let bead_id = "bead-whitespace-test";

        let json1 = r#"{"name":"test"}"#;
        let json2 = r#"{ "name" : "test" }"#;

        let key1 = idempotency_key_from_json(bead_id, json1);
        let key2 = idempotency_key_from_json(bead_id, json2);

        assert_ne!(
            key1, key2,
            "Different JSON formatting must produce different keys"
        );
    }

    #[test]
    fn test_empty_input() -> Result<(), Box<dyn std::error::Error>> {
        let bead_id = "bead-empty-test";

        let input = TestInput {
            bead_id: bead_id.to_string(),
            phase: String::new(),
            data: vec![],
        };

        let key1 = idempotency_key(bead_id, &input)?;
        let key2 = idempotency_key(bead_id, &input)?;

        assert_eq!(key1, key2, "Empty input should still be deterministic");
        Ok(())
    }

    #[test]
    fn test_large_input() -> Result<(), Box<dyn std::error::Error>> {
        let bead_id = "bead-large-test";

        let large_data: Vec<u32> = (0..1000).collect();
        let input = TestInput {
            bead_id: bead_id.to_string(),
            phase: "large-phase".to_string(),
            data: large_data,
        };

        let key1 = idempotency_key(bead_id, &input)?;
        let key2 = idempotency_key(bead_id, &input)?;

        assert_eq!(key1, key2, "Large input should still be deterministic");
        Ok(())
    }

    #[test]
    fn test_complex_nested_structures() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct NestedInput {
            outer: Vec<Inner>,
            config: Config,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Inner {
            id: String,
            values: HashMap<String, u64>,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Config {
            enabled: bool,
            threshold: f64,
        }

        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), 2);

        let input = NestedInput {
            outer: vec![Inner {
                id: "test".to_string(),
                values: map,
            }],
            config: Config {
                enabled: true,
                threshold: 0.95,
            },
        };

        let key1 = idempotency_key("bead-complex", &input)?;
        let key2 = idempotency_key("bead-complex", &input)?;

        assert_eq!(
            key1, key2,
            "Complex nested structures must be deterministic"
        );
        Ok(())
    }

    #[test]
    fn test_unicode_input() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct UnicodeInput {
            text: String,
            emoji: String,
        }

        let input = UnicodeInput {
            text: "测试文本".to_string(),
            emoji: "🦀🚀✨".to_string(),
        };

        let key1 = idempotency_key("bead-unicode", &input)?;
        let key2 = idempotency_key("bead-unicode", &input)?;

        assert_eq!(key1, key2, "Unicode input must be deterministic");
        Ok(())
    }

    #[test]
    fn test_known_key_value() {
        // Regression test: ensure known input produces known output
        let bead_id = "bead-stable-test";
        let input = "stable input value";

        let key = idempotency_key_from_bytes(bead_id, input.as_bytes());

        // This UUID is deterministic based on:
        // - namespace: UUID v5(DNS, "bead-stable-test")
        // - hash: SHA-256("stable input value")
        // - final: UUID v5(namespace, hash)
        assert_eq!(
            key.to_string(),
            "799f6031-7567-54a4-987a-331ff803d6cc",
            "Known input must produce known output"
        );
    }

    #[test]
    fn test_collision_resistance_basic() -> Result<(), Box<dyn std::error::Error>> {
        // Test that small changes produce different keys
        let bead_id = "bead-collision-test";

        let input1 = vec![1, 2, 3, 4, 5];
        let input2 = vec![1, 2, 3, 4, 6]; // Last element different

        let key1 = idempotency_key(bead_id, &input1)?;
        let key2 = idempotency_key(bead_id, &input2)?;

        assert_ne!(
            key1, key2,
            "Small input differences must produce different keys"
        );
        Ok(())
    }

    #[test]
    fn test_bead_id_isolation() {
        // Test that keys from different beads don't collide
        let input = "shared input";

        let keys: Vec<_> = (0..100)
            .map(|i| idempotency_key_from_bytes(&format!("bead-{i:03}"), input.as_bytes()))
            .collect();

        // All keys must be unique
        let unique_keys: std::collections::HashSet<_> = keys.into_iter().collect();
        assert_eq!(
            unique_keys.len(),
            100,
            "100 different bead IDs must produce 100 unique keys"
        );
    }

    #[test]
    fn test_input_isolation() {
        // Test that different inputs produce different keys
        let bead_id = "bead-isolation-test";

        let keys: Vec<_> = (0..100)
            .map(|i| idempotency_key_from_bytes(bead_id, format!("input-{i:03}").as_bytes()))
            .collect();

        // All keys must be unique
        let unique_keys: std::collections::HashSet<_> = keys.into_iter().collect();
        assert_eq!(
            unique_keys.len(),
            100,
            "100 different inputs must produce 100 unique keys"
        );
    }
}
