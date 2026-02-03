//! SHA-256 input hashing for deterministic UUID v5 key generation.
//!
//! This module provides cryptographic hashing of arbitrary input data
//! to produce deterministic keys for idempotent execution. The hashes
//! are used as input to UUID v5 generation.

use bincode::config;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Hash arbitrary byte data using SHA-256.
///
/// Returns a 32-byte (256-bit) hash that can be used as deterministic
/// input for UUID v5 generation.
///
/// # Examples
///
/// ```
/// use oya_workflow::idempotent::hash_input;
///
/// let data = b"test input";
/// let hash = hash_input(data);
/// assert_eq!(hash.len(), 32);
///
/// // Same input produces same hash
/// let hash2 = hash_input(data);
/// assert_eq!(hash, hash2);
/// ```
#[inline]
pub fn hash_input(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash a serializable value using SHA-256 via bincode serialization.
///
/// This is a convenience wrapper around [`hash_input`] that first
/// serializes the input using bincode. This allows hashing of
/// structured data while maintaining determinism.
///
/// # Errors
///
/// Returns an error if bincode serialization fails.
///
/// # Examples
///
/// ```
/// use oya_workflow::idempotent::hash_serializable;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct TaskInput {
///     bead_id: String,
///     phase: String,
/// }
///
/// let input = TaskInput {
///     bead_id: "bead-123".to_string(),
///     phase: "build".to_string(),
/// };
///
/// let hash = hash_serializable(&input).expect("serialization failed");
/// assert_eq!(hash.len(), 32);
/// ```
pub fn hash_serializable<T: Serialize>(value: &T) -> Result<[u8; 32], bincode::error::EncodeError> {
    let bytes = bincode::serde::encode_to_vec(value, config::standard())?;
    Ok(hash_input(&bytes))
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Tests can use expect for clarity
mod tests {
    use super::*;
    use serde::Serialize;

    #[test]
    fn test_hash_input_determinism() {
        let data = b"test input data";
        let hash1 = hash_input(data);
        let hash2 = hash_input(data);

        assert_eq!(hash1, hash2, "Same input must produce same hash");
    }

    #[test]
    fn test_hash_input_different_data() {
        let data1 = b"input one";
        let data2 = b"input two";

        let hash1 = hash_input(data1);
        let hash2 = hash_input(data2);

        assert_ne!(
            hash1, hash2,
            "Different inputs must produce different hashes"
        );
    }

    #[test]
    fn test_hash_input_size() {
        let data = b"any input";
        let hash = hash_input(data);

        assert_eq!(hash.len(), 32, "SHA-256 produces 32-byte hashes");
    }

    #[test]
    fn test_hash_input_empty() {
        let data = b"";
        let hash = hash_input(data);

        assert_eq!(hash.len(), 32, "Empty input still produces 32-byte hash");
    }

    #[test]
    fn test_hash_serializable_determinism() {
        #[derive(Serialize)]
        struct TestData {
            id: String,
            value: u64,
        }

        let data = TestData {
            id: "test-123".to_string(),
            value: 42,
        };

        let hash1 =
            hash_serializable(&data).expect("test_hash_serializable_determinism: first hash");
        let hash2 =
            hash_serializable(&data).expect("test_hash_serializable_determinism: second hash");

        assert_eq!(hash1, hash2, "Same struct must produce same hash");
    }

    #[test]
    fn test_hash_serializable_different_values() {
        #[derive(Serialize)]
        struct TestData {
            id: String,
            value: u64,
        }

        let data1 = TestData {
            id: "test-1".to_string(),
            value: 1,
        };
        let data2 = TestData {
            id: "test-2".to_string(),
            value: 2,
        };

        let hash1 =
            hash_serializable(&data1).expect("test_hash_serializable_different_values: first hash");
        let hash2 = hash_serializable(&data2)
            .expect("test_hash_serializable_different_values: second hash");

        assert_ne!(
            hash1, hash2,
            "Different structs must produce different hashes"
        );
    }

    #[test]
    fn test_hash_serializable_field_order_independence() {
        // Bincode serializes in field declaration order, so this tests
        // that the serialization is consistent
        #[derive(Serialize)]
        struct TestData {
            a: u64,
            b: String,
        }

        let data1 = TestData {
            a: 42,
            b: "test".to_string(),
        };
        let data2 = TestData {
            a: 42,
            b: "test".to_string(),
        };

        let hash1 = hash_serializable(&data1)
            .expect("test_hash_serializable_field_order_independence: first hash");
        let hash2 = hash_serializable(&data2)
            .expect("test_hash_serializable_field_order_independence: second hash");

        assert_eq!(hash1, hash2, "Same field values must produce same hash");
    }

    #[test]
    fn test_hash_serializable_primitive_types() {
        let int_hash =
            hash_serializable(&42u64).expect("test_hash_serializable_primitive_types: int");
        let str_hash = hash_serializable(&"test string")
            .expect("test_hash_serializable_primitive_types: string");
        let bool_hash =
            hash_serializable(&true).expect("test_hash_serializable_primitive_types: bool");

        assert_eq!(int_hash.len(), 32);
        assert_eq!(str_hash.len(), 32);
        assert_eq!(bool_hash.len(), 32);

        // Different types produce different hashes
        assert_ne!(int_hash, str_hash);
        assert_ne!(str_hash, bool_hash);
        assert_ne!(int_hash, bool_hash);
    }

    #[test]
    fn test_hash_serializable_nested_structures() {
        #[derive(Serialize)]
        struct Inner {
            value: u64,
        }

        #[derive(Serialize)]
        struct Outer {
            id: String,
            inner: Inner,
        }

        let data = Outer {
            id: "outer".to_string(),
            inner: Inner { value: 123 },
        };

        let hash1 =
            hash_serializable(&data).expect("test_hash_serializable_nested_structures: first hash");
        let hash2 = hash_serializable(&data)
            .expect("test_hash_serializable_nested_structures: second hash");

        assert_eq!(
            hash1, hash2,
            "Nested structures must hash deterministically"
        );
    }

    #[test]
    fn test_hash_serializable_collections() {
        let vec_data = vec!["a", "b", "c"];
        let hash1 =
            hash_serializable(&vec_data).expect("test_hash_serializable_collections: first hash");
        let hash2 =
            hash_serializable(&vec_data).expect("test_hash_serializable_collections: second hash");

        assert_eq!(hash1, hash2, "Collections must hash deterministically");

        // Different order produces different hash
        let vec_data_reordered = vec!["a", "c", "b"];
        let hash3 = hash_serializable(&vec_data_reordered)
            .expect("test_hash_serializable_collections: reordered hash");
        assert_ne!(hash1, hash3, "Different order must produce different hash");
    }
}
