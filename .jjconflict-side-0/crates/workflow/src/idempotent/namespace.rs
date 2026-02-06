//! UUID v5 namespace generation from bead IDs.
//!
//! This module provides deterministic namespace generation for UUID v5 based idempotency keys.
//! All namespaces are derived from the DNS namespace to ensure global uniqueness and
//! determinism across systems.
//!
//! # Examples
//!
//! ```
//! use oya_workflow::idempotent::namespace::namespace_from_bead;
//!
//! let namespace1 = namespace_from_bead("bead-123");
//! let namespace2 = namespace_from_bead("bead-123");
//! assert_eq!(namespace1, namespace2); // Deterministic
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use uuid::Uuid;

/// Generates a deterministic UUID v5 namespace from a bead ID.
///
/// This function creates a unique namespace for each bead ID by hashing the bead ID
/// using UUID v5 with the DNS namespace as the base. The same bead ID will always
/// produce the same namespace UUID, ensuring idempotency across different executions
/// and systems.
///
/// # Examples
///
/// ```
/// use oya_workflow::idempotent::namespace::namespace_from_bead;
///
/// let namespace = namespace_from_bead("bead-abc-123");
/// // Same input always produces same output
/// assert_eq!(
///     namespace_from_bead("bead-abc-123"),
///     namespace
/// );
/// ```
///
/// # Implementation Details
///
/// - Base namespace: `Uuid::NAMESPACE_DNS` (RFC 4122 DNS namespace)
/// - Hash algorithm: SHA-1 (as specified by UUID v5)
/// - Input: `bead_id` as UTF-8 bytes
/// - Output: Deterministic UUID v5
///
/// # Determinism Guarantee
///
/// This function is **pure** and **deterministic**:
/// - Same `bead_id` → Same namespace UUID
/// - No side effects
/// - No randomness
/// - Thread-safe
#[must_use]
pub fn namespace_from_bead(bead_id: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, bead_id.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        let bead_id = "bead-test-123";
        let namespace1 = namespace_from_bead(bead_id);
        let namespace2 = namespace_from_bead(bead_id);

        assert_eq!(
            namespace1, namespace2,
            "Same bead_id must produce same namespace"
        );
    }

    #[test]
    fn test_different_beads_produce_different_namespaces() {
        let namespace1 = namespace_from_bead("bead-001");
        let namespace2 = namespace_from_bead("bead-002");

        assert_ne!(
            namespace1, namespace2,
            "Different bead_ids must produce different namespaces"
        );
    }

    #[test]
    fn test_empty_bead_id() {
        // Empty string is a valid input - should produce a deterministic namespace
        let namespace1 = namespace_from_bead("");
        let namespace2 = namespace_from_bead("");

        assert_eq!(
            namespace1, namespace2,
            "Empty bead_id should still be deterministic"
        );
    }

    #[test]
    fn test_namespace_version() {
        let namespace = namespace_from_bead("test-bead");

        // UUID v5 has version field = 5
        assert_eq!(
            namespace.get_version(),
            Some(uuid::Version::Sha1),
            "Generated UUID must be version 5 (SHA-1)"
        );
    }

    #[test]
    fn test_namespace_variant() {
        let namespace = namespace_from_bead("test-bead");

        // RFC 4122 variant
        assert_eq!(
            namespace.get_variant(),
            uuid::Variant::RFC4122,
            "Generated UUID must use RFC 4122 variant"
        );
    }

    #[test]
    fn test_unicode_bead_id() {
        // Test with Unicode characters
        let bead_id = "bead-测试-🦀";
        let namespace1 = namespace_from_bead(bead_id);
        let namespace2 = namespace_from_bead(bead_id);

        assert_eq!(
            namespace1, namespace2,
            "Unicode bead_ids should be deterministic"
        );
    }

    #[test]
    fn test_known_namespace_value() {
        // This test ensures the implementation doesn't change unexpectedly
        // The UUID should remain stable across implementations
        let namespace = namespace_from_bead("bead-stable-test");

        // Convert to string for stable comparison
        let uuid_str = namespace.to_string();

        // This is a deterministic UUID v5 based on:
        // - Base: NAMESPACE_DNS (6ba7b810-9dad-11d1-80b4-00c04fd430c8)
        // - Name: "bead-stable-test"
        assert_eq!(
            uuid_str, "42be463f-1e63-5cae-b607-5bc4f5955d81",
            "Known input should produce known output (regression test)"
        );
    }

    #[test]
    fn test_long_bead_id() {
        // Test with a very long bead_id
        let long_id = "a".repeat(1000);
        let namespace1 = namespace_from_bead(&long_id);
        let namespace2 = namespace_from_bead(&long_id);

        assert_eq!(
            namespace1, namespace2,
            "Long bead_ids should still be deterministic"
        );
    }

    #[test]
    fn test_special_characters() {
        // Test with special characters
        let special = "bead-!@#$%^&*()_+-=[]{}|;:',.<>?/~`";
        let namespace1 = namespace_from_bead(special);
        let namespace2 = namespace_from_bead(special);

        assert_eq!(
            namespace1, namespace2,
            "Special characters should be handled deterministically"
        );
    }

    #[test]
    fn test_whitespace_matters() {
        // Whitespace should be significant
        let namespace1 = namespace_from_bead("bead test");
        let namespace2 = namespace_from_bead("beadtest");

        assert_ne!(
            namespace1, namespace2,
            "Whitespace should affect the namespace"
        );
    }

    #[test]
    fn test_case_sensitive() {
        // Case should be significant
        let namespace1 = namespace_from_bead("BEAD-123");
        let namespace2 = namespace_from_bead("bead-123");

        assert_ne!(namespace1, namespace2, "Bead IDs should be case-sensitive");
    }
}
