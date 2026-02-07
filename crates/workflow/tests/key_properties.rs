//! Property-based tests for UUID v5 idempotency keys.
//!
//! Uses proptest to validate:
//! - Determinism holds for all inputs
//! - Serialization order independence
//! - UUID format validity for all inputs
//! - Collision resistance

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use itertools::Itertools;
use oya_workflow::idempotent::keys::idempotency_key;
use proptest::prelude::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PropTestInput {
    a: String,
    b: u32,
    c: Vec<bool>,
}

proptest! {
    /// Property: Same inputs always produce same UUID (determinism)
    #[test]
    fn prop_determinism(
        bead_id in "[a-z0-9-]{10,50}",
        value in "[a-zA-Z0-9 ]{0,100}",
        count in 0u32..1000,
    ) {
        let input = PropTestInput {
            a: value.clone(),
            b: count,
            c: vec![count % 2 == 0],
        };

        let key1 = idempotency_key(&bead_id, &input)
            .map_err(|e| format!("{:?}", e)).unwrap();
        let key2 = idempotency_key(&bead_id, &input)
            .map_err(|e| format!("{:?}", e)).unwrap();

        prop_assert_eq!(key1, key2, "Determinism property violated");
    }

    /// Property: Different bead_ids produce different UUIDs
    #[test]
    fn prop_bead_id_uniqueness(
        bead_id1 in "[a-z0-9-]{10,50}",
        bead_id2 in "[a-z0-9-]{10,50}",
        value in "[a-zA-Z0-9 ]{0,100}",
    ) {
        // Skip if bead_ids are the same
        if bead_id1 == bead_id2 {
            return Ok(());
        }

        let input = PropTestInput {
            a: value.clone(),
            b: 42,
            c: vec![true, false],
        };

        let key1 = idempotency_key(&bead_id1, &input)

            .map_err(|e| format!("{:?}", e)).unwrap();
        let key2 = idempotency_key(&bead_id2, &input)

            .map_err(|e| format!("{:?}", e)).unwrap();

        prop_assert_ne!(key1, key2, "Different bead_ids must produce different UUIDs");
    }

    /// Property: Different inputs produce different UUIDs
    #[test]
    fn prop_input_uniqueness(
        bead_id in "[a-z0-9-]{10,50}",
        value1 in "[a-zA-Z0-9 ]{0,100}",
        value2 in "[a-zA-Z0-9 ]{0,100}",
        count1 in 0u32..100,
        count2 in 0u32..100,
    ) {
        // Skip if inputs are the same
        if value1 == value2 && count1 == count2 {
            return Ok(());
        }

        let input1 = PropTestInput {
            a: value1.clone(),
            b: count1,
            c: vec![true],
        };
        let input2 = PropTestInput {
            a: value2.clone(),
            b: count2,
            c: vec![false],
        };

        let key1 = idempotency_key(&bead_id, &input1)

            .map_err(|e| format!("{:?}", e)).unwrap();
        let key2 = idempotency_key(&bead_id, &input2)

            .map_err(|e| format!("{:?}", e)).unwrap();

        prop_assert_ne!(key1, key2, "Different inputs must produce different UUIDs");
    }

    /// Property: All generated UUIDs are valid v5
    #[test]
    fn prop_uuid_validity(
        bead_id in "[a-z0-9-]{10,50}",
        value in "[a-zA-Z0-9 ]{0,100}",
        count in 0u32..1000,
    ) {
        let input = PropTestInput {
            a: value.clone(),
            b: count,
            c: vec![count % 2 == 0, count % 3 == 0],
        };

        let key = idempotency_key(&bead_id, &input)

            .map_err(|e| format!("{:?}", e)).unwrap();

        // Verify version is v5 (SHA-1 based)
        prop_assert_eq!(
            key.get_version(),
            Some(uuid::Version::Sha1),
            "UUID must be v5"
        );

        // Verify variant is RFC 4122
        prop_assert_eq!(
            key.get_variant(),
            uuid::Variant::RFC4122,
            "UUID must be RFC 4122 variant"
        );

        // Verify not nil
        prop_assert_ne!(key, uuid::Uuid::nil(), "UUID must not be nil");
    }

    /// Property: UUID is deterministic across multiple calls
    #[test]
    fn prop_multi_call_determinism(
        bead_id in "[a-z0-9-]{10,50}",
        value in "[a-zA-Z0-9 ]{0,100}",
    ) {
        let input = PropTestInput {
            a: value.clone(),
            b: 123,
            c: vec![true, false, true],
        };

        // Generate 5 keys
        let keys: Vec<_> = (0..5)
            .map(|_| idempotency_key(&bead_id, &input).map_err(|e| format!("{:?}", e)).unwrap())
            .collect();

        // All must be equal
        prop_assert!(
            keys.windows(2).all(|w| w[0] == w[1]),
            "All calls must produce same UUID"
        );
    }
}

proptest! {
    /// Property: No collisions across large input space
    #[test]
    fn prop_collision_resistance(
        bead_count in 1usize..20,
        value_count in 10usize..200,
    ) {
        use std::collections::HashSet;

        let mut keys = HashSet::new();

        for i in 0..value_count {
            let bead_id = format!("bead-{}", i % bead_count);
            let input = PropTestInput {
                a: format!("value-{}", i),
                b: i as u32,
                c: vec![i % 2 == 0, i % 3 == 0, i % 5 == 0],
            };

            let key = idempotency_key(&bead_id, &input)
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            prop_assert!(!keys.contains(&key), "Collision detected at index {}!", i);
            keys.insert(key);
        }

        // Should have exactly value_count unique keys
        prop_assert_eq!(keys.len(), value_count, "All keys should be unique");
    }
}
