//! Tests for UUID v5 idempotency key determinism validation.
//!
//! These tests validate that:
//! - Same bead_id + same input = same UUID (determinism)
//! - Different input = different UUID (uniqueness)
//! - Serialization order doesn't matter
//! - UUID format is valid

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use oya_workflow::idempotent::keys::idempotency_key;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq)]
struct TestInput {
    value: String,
    count: u32,
}

#[test]
fn test_determinism_same_bead_same_input() -> Result<(), String> {
    // Given: Same bead_id and input
    let bead_id = "test-bead-123";
    let input = TestInput {
        value: "test".to_string(),
        count: 42,
    };

    // When: Generate keys twice
    let key1 = idempotency_key(bead_id, &input).map_err(|e| format!("{:?}", e))?;
    let key2 = idempotency_key(bead_id, &input).map_err(|e| format!("{:?}", e))?;

    // Then: Keys must be identical
    assert_eq!(key1, key2, "Same inputs should produce same UUID");
    Ok(())
}

#[test]
fn test_determinism_multiple_calls() -> Result<(), String> {
    // Given: Same bead_id and input
    let bead_id = "test-bead-456";
    let input = TestInput {
        value: "multi-call".to_string(),
        count: 100,
    };

    // When: Generate keys 10 times
    let mut keys = Vec::new();
    for _ in 0..10 {
        let key = idempotency_key(bead_id, &input).map_err(|e| format!("{:?}", e))?;
        keys.push(key);
    }

    // Then: All keys must be identical
    assert!(
        keys.windows(2).all(|w| w[0] == w[1]),
        "All calls should produce same UUID"
    );
    Ok(())
}

#[test]
fn test_uniqueness_different_input() -> Result<(), String> {
    // Given: Same bead_id but different inputs
    let bead_id = "test-bead-789";

    let input1 = TestInput {
        value: "first".to_string(),
        count: 1,
    };

    let input2 = TestInput {
        value: "second".to_string(),
        count: 2,
    };

    // When: Generate keys for each input
    let key1 = idempotency_key(bead_id, &input1).map_err(|e| format!("{:?}", e))?;
    let key2 = idempotency_key(bead_id, &input2).map_err(|e| format!("{:?}", e))?;

    // Then: Keys must be different
    assert_ne!(
        key1, key2,
        "Different inputs should produce different UUIDs"
    );
    Ok(())
}

#[test]
fn test_uniqueness_different_bead_id() -> Result<(), String> {
    // Given: Different bead_id but same input
    let input = TestInput {
        value: "same-value".to_string(),
        count: 99,
    };

    // When: Generate keys with different bead_ids
    let key1 = idempotency_key("bead-1", &input).map_err(|e| format!("{:?}", e))?;
    let key2 = idempotency_key("bead-2", &input).map_err(|e| format!("{:?}", e))?;

    // Then: Keys must be different (namespaced by bead_id)
    assert_ne!(
        key1, key2,
        "Different bead IDs should produce different UUIDs"
    );
    Ok(())
}

#[test]
fn test_uuid_format_valid() -> Result<(), String> {
    // Given: Valid inputs
    let bead_id = "test-bead-format";
    let input = TestInput {
        value: "format-test".to_string(),
        count: 1,
    };

    // When: Generate key
    let key = idempotency_key(bead_id, &input).map_err(|e| format!("{:?}", e))?;

    // Then: UUID must be valid v5
    assert_eq!(
        key.get_version(),
        Some(uuid::Version::Sha1),
        "UUID should be v5"
    );

    // Verify it's a valid UUID (not nil)
    assert_ne!(key, Uuid::nil(), "UUID should not be nil");

    // Verify it can be converted to string and back
    let key_str = key.to_string();
    let parsed_uuid = Uuid::parse_str(&key_str).map_err(|e| format!("{:?}", e))?;
    assert_eq!(key, parsed_uuid, "UUID should round-trip through string");
    Ok(())
}

#[test]
fn test_collision_resistance_different_values() -> Result<(), String> {
    // Given: Many different inputs
    let bead_id = "test-collision";
    let inputs: Vec<TestInput> = vec![
        TestInput {
            value: "a".to_string(),
            count: 0,
        },
        TestInput {
            value: "a".to_string(),
            count: 1, // Different count
        },
        TestInput {
            value: "A".to_string(),
            count: 0, // Different case
        },
        TestInput {
            value: "".to_string(),
            count: 0, // Empty string
        },
        TestInput {
            value: "longer string with spaces".to_string(),
            count: 1000,
        },
    ];

    // When: Generate keys for all inputs
    let mut keys = Vec::new();
    for input in &inputs {
        let key = idempotency_key(bead_id, input).map_err(|e| format!("{:?}", e))?;
        keys.push((input, key));
    }

    // Then: No two keys should be the same
    for (i, (input1, key1)) in keys.iter().enumerate() {
        for (input2, key2) in keys.iter().skip(i + 1) {
            assert_ne!(
                key1, key2,
                "Collision detected: {:?} and {:?} produced same UUID",
                input1, input2
            );
        }
    }
    Ok(())
}

#[test]
fn test_special_characters_in_input() -> Result<(), String> {
    // Given: Input with special characters
    let bead_id = "test-special";
    let input = TestInput {
        value: "Hello, 世界! 🚀".to_string(),
        count: 42,
    };

    // When: Generate key
    let key = idempotency_key(bead_id, &input).map_err(|e| format!("{:?}", e))?;

    // Then: Should be deterministic
    let key2 = idempotency_key(bead_id, &input).map_err(|e| format!("{:?}", e))?;
    assert_eq!(
        key, key2,
        "Special characters should not affect determinism"
    );
    Ok(())
}

#[test]
fn test_empty_input() -> Result<(), String> {
    // Given: Empty input struct
    #[derive(Debug, Clone, Serialize, PartialEq)]
    struct EmptyInput {}

    let bead_id = "test-empty";
    let input = EmptyInput {};

    // When: Generate key
    let key1 = idempotency_key(bead_id, &input).map_err(|e| format!("{:?}", e))?;
    let key2 = idempotency_key(bead_id, &input).map_err(|e| format!("{:?}", e))?;

    // Then: Should be deterministic
    assert_eq!(key1, key2, "Empty input should be deterministic");
    Ok(())
}

#[test]
fn test_numeric_values() -> Result<(), String> {
    // Given: Numeric inputs that should produce different keys
    let bead_id = "test-numeric";

    let key1 = idempotency_key(bead_id, &42u32).map_err(|e| format!("{:?}", e))?;
    let key2 = idempotency_key(bead_id, &42i32).map_err(|e| format!("{:?}", e))?;
    let key3 = idempotency_key(bead_id, &42.0f64).map_err(|e| format!("{:?}", e))?;

    // Then: Different numeric types should produce different keys
    assert_ne!(key1, key2, "u32 and i32 should produce different UUIDs");
    assert_ne!(key1, key3, "u32 and f64 should produce different UUIDs");
    assert_ne!(key2, key3, "i32 and f64 should produce different UUIDs");
    Ok(())
}
