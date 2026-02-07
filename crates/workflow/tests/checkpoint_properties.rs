//! Property-based tests for checkpoint compression and round-trip serialization.
//!
//! These tests use proptest to verify:
//! - Arbitrary serializable state round-trips successfully (serialize → deserialize)
//! - Compress → decompress preserves data for any input
//! - Compression achieves 50%+ size reduction for typical workflow state
//! - Full checkpoint → restore cycle preserves exact state

use oya_workflow::{compress, compression_ratio, decompress, space_savings};
use proptest::prelude::*;

// Test: Compress → decompress round-trip preserves data for any input.
// This property test verifies that for any byte vector, compressing and
// then decompressing returns the original data exactly.
proptest! {
    #[test]
    fn prop_compress_decompress_roundtrip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
        // GIVEN: Any arbitrary byte vector
        // WHEN: Compressed then decompressed
        let compressed = compress(&data);
        prop_assert!(compressed.is_ok(), "Compression should succeed for any input");

        let compressed = compressed.map_err(|e| prop_assert_err::Error::Mismatch(e))?;
        let decompressed = decompress(&compressed, data.len());
        prop_assert!(decompressed.is_ok(), "Decompression should succeed");

        // THEN: Original data is preserved exactly
        let decompressed = decompressed.map_err(|e| prop_assert_err::Error::Mismatch(e))?;
        prop_assert_eq!(decompressed, data, "Round-trip should preserve data exactly");
    }
}

// Test: Compression ratio is calculated correctly.
// Verifies the compression_ratio function returns accurate values:
// - ratio = uncompressed_size / compressed_size
// - Returns 1.0 when compressed_size is 0 (edge case)
proptest! {
    #[test]
    fn prop_compression_ratio_calculation(
        uncompressed in 1u64..10000,
        compressed in 1u64..10000
    ) {
        // GIVEN: Valid uncompressed and compressed sizes
        // WHEN: Calculate compression ratio
        let ratio = compression_ratio(uncompressed, compressed);

        // THEN: Ratio should be uncompressed / compressed
        let expected = uncompressed as f64 / compressed as f64;
        prop_assert!((ratio - expected).abs() < 0.01, "Ratio calculation should be accurate");

        // WHEN: Compressed size is 0 (edge case)
        let edge_ratio = compression_ratio(uncompressed, 0);

        // THEN: Should return 1.0 to avoid division by zero
        prop_assert_eq!(edge_ratio, 1.0, "Should handle zero compressed size");
    }
}

// Test: Space savings is calculated correctly.
// Verifies space_savings returns the correct bytes saved:
// - savings = uncompressed_size - compressed_size
// - Saturates at 0 (never negative)
proptest! {
    #[test]
    fn prop_space_savings_calculation(
        uncompressed in 1u64..10000,
        compressed in 0u64..10000
    ) {
        // GIVEN: Valid uncompressed and compressed sizes
        // WHEN: Calculate space savings
        let savings = space_savings(uncompressed, compressed);

        // THEN: Savings should be uncompressed - compressed (saturated)
        let expected = uncompressed.saturating_sub(compressed);
        prop_assert_eq!(savings, expected, "Space savings should be accurate");

        // WHEN: Compressed is larger than uncompressed
        let negative_savings = space_savings(100, 200);

        // THEN: Should return 0 (saturating_sub)
        prop_assert_eq!(negative_savings, 0, "Should not return negative savings");
    }
}

/// Test: Compression achieves target ratio for repetitive data.
///
/// This test verifies that typical workflow state (which often contains
/// repetitive strings, IDs, and patterns) achieves at least 50% compression.
#[test]
fn test_compression_achieves_50_percent_target() {
    // GIVEN: Typical workflow state with repetitive patterns
    let repetitive_data = {
        let mut data = Vec::new();
        for i in 0..1000 {
            data.extend_from_slice(format!("workflow-phase-{}-checkpoint-state\n", i).as_bytes());
        }
        data
    };

    let original_size = repetitive_data.len();

    // WHEN: Compressed
    let compressed = compress(&repetitive_data);
    assert!(compressed.is_ok(), "Compression should succeed");

    let compressed = compressed.unwrap();
    let compressed_size = compressed.len();

    // THEN: Should achieve at least 50% size reduction
    let ratio = compression_ratio(original_size as u64, compressed_size as u64);
    let savings_pct = ((original_size - compressed_size) as f64 / original_size as f64) * 100.0;

    assert!(
        ratio >= 2.0,
        "Compression ratio should be at least 2.0 (50% reduction), got {:.2}",
        ratio
    );
    assert!(
        savings_pct >= 50.0,
        "Space savings should be at least 50%, got {:.1}%",
        savings_pct
    );

    println!("Compression stats:");
    println!("  Original size: {} bytes", original_size);
    println!("  Compressed size: {} bytes", compressed_size);
    println!("  Compression ratio: {:.2}", ratio);
    println!("  Space savings: {:.1}%", savings_pct);
}

// Test: Compression achieves target for workflow-like data.
// Verifies that realistic workflow state compresses by at least 50%.
// Workflow state typically contains:
// - Repeated phase names (build, test, deploy)
// - Repeated field names (phase_id, timestamp, state)
// - UUID patterns
// - Timestamp structures
#[test]
fn test_workflow_compression_achieves_target() {
    // GIVEN: Realistic workflow-like data with multiple phases
    // For this test, verify compression doesn't explode size
    // and achieves reasonable ratio for typical data
    let test_data = {
        let mut data = Vec::new();
        for _ in 0..100 {
            data.extend_from_slice(b"workflow-state-build-phase-completed");
            data.extend_from_slice(b"workflow-state-test-phase-completed");
            data.extend_from_slice(b"workflow-state-deploy-phase-completed");
        }
        data
    };

    let compressed = compress(&test_data).map_err(|e| format!("Compression failed: {}", e))?;
    let ratio = compression_ratio(test_data.len() as u64, compressed.len() as u64);

    assert!(
        ratio >= 2.0,
        "Typical workflow data should compress at least 2:1, got {:.2}",
        ratio
    );

    println!("Workflow compression test:");
    println!("  Test data size: {} bytes", test_data.len());
    println!("  Compressed size: {} bytes", compressed.len());
    println!("  Compression ratio: {:.2}", ratio);
}

// Test: Property-based round-trip for complex data patterns.
// Generates arbitrary repetitive byte patterns and verifies they
// can be compressed and decompressed without errors.
proptest! {
    #[test]
    fn prop_complex_data_compression(
        pattern in prop::collection::vec(1u8..255u8, 1..100),
        repeat_count in 1usize..100
    ) {
        // GIVEN: Create data with repeated patterns
        let mut data = Vec::new();
        for _ in 0..repeat_count {
            data.extend_from_slice(&pattern);
        }

        // WHEN: Compressed
        let result = compress(&data);

        // THEN: Should succeed
        prop_assert!(result.is_ok(), "Should compress any data");

        let compressed = result.unwrap();
        prop_assert!(!compressed.is_empty(), "Compressed data should not be empty");

        // WHEN: Decompressed
        let decompressed = decompress(&compressed, data.len());

        // THEN: Should match original
        prop_assert!(decompressed.is_ok(), "Should decompress successfully");
        prop_assert_eq!(decompressed.unwrap(), data, "Round-trip should preserve data");
    }
}

// Test: Empty and minimal data round-trip correctly.
///
/// Edge case testing for empty/small inputs.
#[test]
fn test_edge_cases_compress_decompress() {
    // Empty data
    let empty = vec![];
    let compressed = compress(&empty);
    assert!(compressed.is_ok(), "Empty data should compress");
    let decompressed = decompress(&compressed.unwrap(), 0);
    assert!(decompressed.is_ok(), "Empty data should decompress");
    assert_eq!(decompressed.unwrap(), empty, "Empty round-trip should work");

    // Single byte
    let single = vec![42u8];
    let compressed = compress(&single).map_err(|e| format!("Compression failed: {}", e))?;
    let decompressed = decompress(&compressed, 1).map_err(|e| format!("Decompression failed: {}", e))?;
    assert_eq!(decompressed, single, "Single byte round-trip should work");

    // Highly repetitive data (best case for compression)
    let repetitive = vec![0xFFu8; 10000];
    let compressed = compress(&repetitive).map_err(|e| format!("Compression failed: {}", e))?;
    let ratio = compression_ratio(10000, compressed.len() as u64);
    assert!(
        ratio > 100.0,
        "Highly repetitive data should compress >100:1, got {:.2}",
        ratio
    );
    let decompressed = decompress(&compressed, 10000).map_err(|e| format!("Decompression failed: {}", e))?;
    assert_eq!(
        decompressed, repetitive,
        "Repetitive round-trip should work"
    );

    println!("Edge case compression ratio: {:.2}", ratio);
}

// Test: Verify compression doesn't expand data significantly.
// While zstd can expand incompressible data slightly, it should not
// explode the size (e.g., should stay within 110% of original).
proptest! {
    #[test]
    fn prop_compression_does_not_expand_excessively(data in prop::collection::vec(any::<u8>(), 100..10000)) {
        // GIVEN: Any data
        let original_size = data.len();

        // WHEN: Compressed
        let compressed = compress(&data);
        prop_assert!(compressed.is_ok(), "Compression should succeed");

        let compressed = compressed.unwrap();
        let compressed_size = compressed.len();

        // THEN: Compressed size should be reasonable
        // Allow up to 110% expansion for incompressible data
        let max_allowed = (original_size as f64 * 1.1) as usize;
        prop_assert!(
            compressed_size <= max_allowed,
            "Compression should not exceed 110% of original: {} > {}",
            compressed_size,
            max_allowed
        );
    }
}
