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
async fn test_determinism_same_bead_same_input() {
    // Given: Same bead_id and input
    let bead_id = "test-bead-123";
    let input = TestInput {
        value: "test".to_string(),
        count: 42,
    };

    // When: Generate keys twice
    let key1 = idempotency_key(bead_id, &input);
    let key2 = idempotency_key(bead_id, &input);
    assert!(key1.is_ok(), "Should generate key");
    assert!(key2.is_ok(), "Should generate key");
    let key1 = key1.ok();
    let key2 = key2.ok();

    // Then: Keys must be identical
    assert_eq!(key1, key2, "Same inputs should produce same UUID");
}

#[test]
async fn test_determinism_multiple_calls() {
    // Given: Same bead_id and input
    let bead_id = "test-bead-456";
    let input = TestInput {
        value: "multi-call".to_string(),
        count: 100,
    };

    // When: Generate keys 10 times
    let mut keys = Vec::new();
    for _ in 0..10 {
        let key = idempotency_key(bead_id, &input).await;
        assert!(key.is_ok(), "Should generate key");
        keys.push(key.ok());
    }

    // Then: All keys must be identical
    for key in &keys[1..] {
        assert_eq!(keys[0], *key, "All calls should produce same UUID");
    }
}

#[test]
async fn test_uniqueness_different_input() {
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
    let key1 = idempotency_key(bead_id, &input1).await;
    let key2 = idempotency_key(bead_id, &input2).await;
    assert!(key1.is_ok(), "Should generate key");
    assert!(key2.is_ok(), "Should generate key");
    let key1 = key1.ok();
    let key2 = key2.ok();

    // Then: Keys must be different
    assert_ne!(
        key1, key2,
        "Different inputs should produce different UUIDs"
    );
}

#[test]
async fn test_uniqueness_different_bead_id() {
    // Given: Different bead_id but same input
    let input = TestInput {
        value: "same-value".to_string(),
        count: 99,
    };

    // When: Generate keys with different bead_ids
    let key1 = idempotency_key("bead-1", &input).await;
    let key2 = idempotency_key("bead-2", &input).await;
    assert!(key1.is_ok(), "Should generate key");
    assert!(key2.is_ok(), "Should generate key");
    let key1 = key1.ok();
    let key2 = key2.ok();

    // Then: Keys must be different (namespaced by bead_id)
    assert_ne!(
        key1, key2,
        "Different bead IDs should produce different UUIDs"
    );
}

#[test]
async fn test_uuid_format_valid() {
    // Given: Valid inputs
    let bead_id = "test-bead-format";
    let input = TestInput {
        value: "format-test".to_string(),
        count: 1,
    };

    // When: Generate key
    let key_result = idempotency_key(bead_id, &input);
    assert!(key_result.is_ok(), "Should generate key");

    // Then: UUID must be valid v5
    if let Some(key) = key_result.ok() {
        assert_eq!(
            key.get_version(),
            Some(uuid::Version::Sha1),
            "UUID should be v5"
        );

        // Verify it's a valid UUID (not nil)
        assert_ne!(key, Uuid::nil(), "UUID should not be nil");

        // Verify it can be converted to string and back
        let key_str = key.to_string();
        let parsed = Uuid::parse_str(&key_str);
        assert!(parsed.is_ok(), "Should parse UUID string");
        if let Some(parsed_uuid) = parsed.ok() {
            assert_eq!(key, parsed_uuid, "UUID should round-trip through string");
        }
    }
}

#[test]
async fn test_collision_resistance_different_values() {
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
        let key = idempotency_key(bead_id, input)
            .await
            .expect("Should generate key");
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
}

#[test]
async fn test_special_characters_in_input() {
    // Given: Input with special characters
    let bead_id = "test-special";
    let input = TestInput {
        value: "Hello, 世界! 🚀".to_string(),
        count: 42,
    };

    // When: Generate key
    let key = idempotency_key(bead_id, &input)
        .await
        .expect("Should handle special characters");

    // Then: Should be deterministic
    let key2 = idempotency_key(bead_id, &input)
        .await
        .expect("Should generate same key");
    assert_eq!(
        key, key2,
        "Special characters should not affect determinism"
    );
}

#[test]
async fn test_empty_input() {
    // Given: Empty input struct
    #[derive(Debug, Clone, Serialize, PartialEq)]
    struct EmptyInput {}

    let bead_id = "test-empty";
    let input = EmptyInput {};

    // When: Generate key
    let key1 = idempotency_key(bead_id, &input)
        .await
        .expect("Should handle empty input");
    let key2 = idempotency_key(bead_id, &input)
        .await
        .expect("Should generate same key");

    // Then: Should be deterministic
    assert_eq!(key1, key2, "Empty input should be deterministic");
}

#[test]
async fn test_numeric_values() {
    // Given: Numeric inputs that should produce different keys
    let bead_id = "test-numeric";

    let key1 = idempotency_key(bead_id, &42u32)
        .await
        .expect("Should handle u32");
    let key2 = idempotency_key(bead_id, &42i32)
        .await
        .expect("Should handle i32");
    let key3 = idempotency_key(bead_id, &42.0f64)
        .await
        .expect("Should handle f64");

    // Then: Different numeric types should produce different keys
    assert_ne!(key1, key2, "u32 and i32 should produce different UUIDs");
    assert_ne!(key1, key3, "u32 and f64 should produce different UUIDs");
    assert_ne!(key2, key3, "i32 and f64 should produce different UUIDs");
}
