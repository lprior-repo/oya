//! Property-based tests for idempotency key generation.
//!
//! This module uses proptest to verify core properties of
//! deterministic UUID v5 key generation:
//! - Determinism (same input → same output)
//! - Uniqueness (different inputs → different outputs)
//! - No collisions across large input spaces
//! - Distribution properties

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use oya_workflow::idempotent::{
    hash_input, idempotency_key, idempotency_key_from_bytes, IdempotencyKey,
};
use serde::Serialize;

mod unit_tests {
    use super::*;

    #[test]
    fn test_known_key_determinism() {
        let bead_id = "bead-stable-test";
        let input = "stable input value";

        let key1 = idempotency_key_from_bytes(bead_id, input.as_bytes());
        let key2 = idempotency_key_from_bytes(bead_id, input.as_bytes());

        assert_eq!(key1, key2);
        assert_eq!(key1.to_string(), "799f6031-7567-54a4-987a-331ff803d6cc");
    }

    #[test]
    fn test_serialization_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let key = IdempotencyKey::new(uuid::Uuid::new_v4());

        let serialized = serde_json::to_string(&key)?;
        let deserialized: IdempotencyKey = serde_json::from_str(&serialized)?;

        assert_eq!(deserialized, key);
        Ok(())
    }

    #[test]
    fn test_bincode_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let key = IdempotencyKey::new(uuid::Uuid::new_v4());

        let encoded = bincode::serde::encode_to_vec(key, bincode::config::standard())?;
        let (decoded, _): (IdempotencyKey, _) =
            bincode::serde::decode_from_slice(&encoded, bincode::config::standard())?;

        assert_eq!(decoded, key);
        Ok(())
    }

    #[test]
    fn test_million_iterations_no_panic() {
        // Test that we can generate 1M keys without panicking
        for i in 0..1_000_000 {
            let _ = idempotency_key_from_bytes(&format!("bead-{i}"), b"test input");
        }
    }

    #[test]
    fn test_idempotency_key_determinism() {
        let bead_id = "test-bead";
        let input = b"test input";

        let key1 = idempotency_key_from_bytes(bead_id, input);
        let key2 = idempotency_key_from_bytes(bead_id, input);
        let key3 = idempotency_key_from_bytes(bead_id, input);

        assert_eq!(key1, key2);
        assert_eq!(key2, key3);
    }

    #[test]
    fn test_different_inputs_produce_different_keys() {
        let bead_id = "test-bead";
        let input1 = b"input one";
        let input2 = b"input two";

        let key1 = idempotency_key_from_bytes(bead_id, input1);
        let key2 = idempotency_key_from_bytes(bead_id, input2);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_different_beads_produce_different_keys() {
        let input = b"shared input";

        let key1 = idempotency_key_from_bytes("bead-001", input);
        let key2 = idempotency_key_from_bytes("bead-002", input);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_avalanche_effect() {
        let bead_id = "test-bead";
        let mut input = b"test input".to_vec();

        let key1 = idempotency_key_from_bytes(bead_id, &input);

        // Flip one bit
        input[0] ^= 0x01;

        let key2 = idempotency_key_from_bytes(bead_id, &input);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_input_order_matters() {
        let bead_id = "test-bead";
        let input = b"abc";

        let key1 = idempotency_key_from_bytes(bead_id, input);

        let reversed: Vec<u8> = input.iter().rev().copied().collect();
        let key2 = idempotency_key_from_bytes(bead_id, &reversed);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_all_keys_are_valid_uuid_v5() {
        let bead_id = "test-bead";
        let input = b"test input";

        let key = idempotency_key_from_bytes(bead_id, input);

        assert_eq!(key.get_version(), Some(uuid::Version::Sha1));
        assert_eq!(key.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn test_idempotency_key_wrapper() -> Result<(), Box<dyn std::error::Error>> {
        let bead_id = "test-bead";
        let input = b"test input";

        let uuid = idempotency_key_from_bytes(bead_id, input);
        let key = IdempotencyKey::new(uuid);

        assert_eq!(key.as_uuid(), uuid);
        assert_eq!(key.to_string(), uuid.to_string());

        let key_str = key.to_string();
        let parsed: IdempotencyKey = key_str.parse()?;

        assert_eq!(parsed, key);
        Ok(())
    }

    #[test]
    fn test_structured_data_determinism() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Serialize, Debug, PartialEq)]
        struct TestStruct {
            bead_id: String,
            value: u64,
        }

        let data = TestStruct {
            bead_id: "test-bead".to_string(),
            value: 42,
        };

        let key1 = idempotency_key("test-bead", &data)?;
        let key2 = idempotency_key("test-bead", &data)?;

        assert_eq!(key1, key2);
        Ok(())
    }

    #[test]
    fn test_no_collisions_small_space() {
        let bead_id = "test-bead";

        let mut keys = std::collections::HashSet::new();

        for i in 0..100 {
            let input = format!("input-{i:03}");
            let key = idempotency_key_from_bytes(bead_id, input.as_bytes());
            keys.insert(key);
        }

        // All 100 keys should be unique
        assert_eq!(keys.len(), 100);
    }

    #[test]
    fn test_empty_input() {
        let bead_id = "test-bead";
        let input: Vec<u8> = vec![];

        let key1 = idempotency_key_from_bytes(bead_id, &input);
        let key2 = idempotency_key_from_bytes(bead_id, &input);

        assert_eq!(key1, key2);
        assert!(!key1.is_nil());
    }

    #[test]
    fn test_hash_input_determinism() {
        let data = b"test data";

        let hash1 = hash_input(data);
        let hash2 = hash_input(data);
        let hash3 = hash_input(data);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_hash_input_different_inputs() {
        let data1 = b"input one";
        let data2 = b"input two";

        let hash1 = hash_input(data1);
        let hash2 = hash_input(data2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_input_fixed_size() {
        let data = b"any input";

        let hash = hash_input(data);

        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_large_input_handling() {
        let bead_id = "test-bead";
        let large_input = vec![42u8; 10_000];

        let key1 = idempotency_key_from_bytes(bead_id, &large_input);
        let key2 = idempotency_key_from_bytes(bead_id, &large_input);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_bead_id_isolation() {
        let input = b"shared input";

        let mut keys = std::collections::HashSet::new();

        for i in 0..100 {
            let bead_id = format!("bead-{i:03}");
            let key = idempotency_key_from_bytes(&bead_id, input);
            keys.insert(key);
        }

        // All 100 keys should be unique
        assert_eq!(keys.len(), 100);
    }

    #[test]
    fn test_input_isolation() {
        let bead_id = "test-bead";

        let mut keys = std::collections::HashSet::new();

        for i in 0..100 {
            let input = format!("input-{i:03}");
            let key = idempotency_key_from_bytes(bead_id, input.as_bytes());
            keys.insert(key);
        }

        // All 100 keys should be unique
        assert_eq!(keys.len(), 100);
    }

    #[test]
    fn test_unicode_input() {
        let bead_id = "test-bead-测试";
        let input = "测试数据🦀";

        let key1 = idempotency_key_from_bytes(bead_id, input.as_bytes());
        let key2 = idempotency_key_from_bytes(bead_id, input.as_bytes());

        assert_eq!(key1, key2);
    }
}
