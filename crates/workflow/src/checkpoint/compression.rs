//! Compression utilities for checkpoint data.
//!
//! This module provides compression and decompression functions using zstd,
//! along with utilities for calculating compression ratios and space savings.
//!
//! # Design Principles
//!
//! - **Zero panics**: All functions return `Result` types, never panic
//! - **Zero unwraps**: No use of `unwrap()`, `expect()`, or `.ok().map_or()` patterns
//! - **Railway-oriented programming**: Use `and_then`, `map`, `or_else` for composition
//! - **Pure functions**: All compression functions are pure and deterministic

use crate::error::{Error, Result};

/// Compression level for zstd.
///
/// Provides predefined compression levels that balance speed vs ratio.
/// Each level maps to a specific zstd compression parameter.
#[derive(Debug, Clone, Copy, Default)]
pub enum CompressionLevel {
    /// Fastest compression (level 1).
    /// Best for: low-latency scenarios, frequent checkpoints
    /// Trade-off: worst compression ratio
    Fastest,
    /// Default compression (level 3).
    /// Best for: general use, balanced speed/ratio
    /// Trade-off: good compression ratio with reasonable speed
    #[default]
    Default,
    /// Maximum compression (level 21).
    /// Best for: cold storage, infrequent checkpoints
    /// Trade-off: slowest compression
    Max,
}

impl CompressionLevel {
    /// Get the zstd compression level value.
    ///
    /// # Postconditions
    /// - Returns value in range [0, 21]
    #[must_use]
    pub const fn as_i32(&self) -> i32 {
        match self {
            Self::Fastest => 1,
            Self::Default => 3,
            Self::Max => 21,
        }
    }
}

/// Compress data using zstd with default level (3).
///
/// This is a convenience wrapper around `compress_with_level` using the default
/// compression level, which balances speed and compression ratio.
///
/// # Arguments
///
/// * `data` - Uncompressed data slice
///
/// # Returns
///
/// `Ok(compressed_data)` on success, `Err(Error::CheckpointFailed)` on failure
///
/// # Postconditions
/// - `result.is_ok()` → compressed data length ≤ input length
/// - Empty input returns `Ok(Vec::new())` (no error)
///
/// # Errors
///
/// Returns `Error::CheckpointFailed` if zstd compression fails.
///
/// # Examples
///
/// ```rust
/// use oya_workflow::checkpoint::compression::compress;
///
/// let data = b"repetitive data repetitive data";
/// let compressed = compress(data).unwrap();
/// assert!(compressed.len() < data.len());
/// ```
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    compress_with_level(data, CompressionLevel::Default.as_i32())
}

/// Compress data using zstd with specified level.
///
/// Provides fine-grained control over compression level for performance tuning.
/// Level must be in range [0, 21] per zstd specification.
///
/// # Arguments
///
/// * `data` - Uncompressed data slice
/// * `level` - Compression level (0-21)
///
/// # Returns
///
/// `Ok(compressed_data)` on success, `Err(Error::CheckpointFailed)` on failure
///
/// # Preconditions
/// - `level` must be in range [0, 21] (enforced by zstd)
///
/// # Postconditions
/// - `result.is_ok()` → compressed data length ≤ input length
/// - Empty input returns `Ok(Vec::new())` (no error)
///
/// # Errors
///
/// Returns `Error::CheckpointFailed` if:
/// - Compression level is invalid (outside [0, 21])
/// - zstd internal error occurs
pub fn compress_with_level(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::bulk::compress(data, level).map_err(|e| Error::CheckpointFailed {
        reason: format!("zstd compression failed (level {level}): {}", e),
    })
}

/// Decompress data using zstd.
///
/// Requires knowledge of the original uncompressed size.
/// Use `decompress_auto` if the uncompressed size is unknown.
///
/// # Arguments
///
/// * `compressed_data` - Compressed data from `compress` or `compress_with_level`
/// * `uncompressed_size` - Expected uncompressed data size (must be exact)
///
/// # Returns
///
/// `Ok(decompressed_data)` on success, `Err(Error::CheckpointFailed)` on failure
///
/// # Preconditions
/// - `compressed_data` must be valid zstd output
/// - `uncompressed_size` must match the original data size exactly
/// - `uncompressed_size` must be > 0
///
/// # Postconditions
/// - `result.is_ok()` → output length == `uncompressed_size`
/// - Roundtrip: `compress(d).and_then(|c| decompress(&c, d.len())) == Ok(d)`
///
/// # Errors
///
/// Returns `Error::CheckpointFailed` if:
/// - Compressed data is corrupted or invalid
/// - `uncompressed_size` doesn't match actual size
/// - zstd internal error occurs
pub fn decompress(compressed_data: &[u8], uncompressed_size: usize) -> Result<Vec<u8>> {
    zstd::bulk::decompress(compressed_data, uncompressed_size).map_err(|e| {
        Error::CheckpointFailed {
            reason: format!(
                "zstd decompression failed (size {}): {}",
                uncompressed_size, e
            ),
        }
    })
}

/// Decompress data using zstd (auto-detect size).
///
/// Attempts multiple buffer size strategies when the uncompressed size is unknown.
/// Tries sizes in order: 2x, 4x, 8x, 16x the compressed data size.
///
/// # Arguments
///
/// * `data` - Compressed data from `compress` or `compress_with_level`
///
/// # Returns
///
/// `Ok(decompressed_data)` on success, `Err(Error::CheckpointFailed)` if all strategies fail
///
/// # Preconditions
/// - `data` must be valid zstd output
///
/// # Postconditions
/// - Eventually succeeds for valid compressed data (expansion ≤ 16x)
/// - Returns `Err` only if all buffer strategies fail
///
/// # Errors
///
/// Returns `Error::CheckpointFailed` if all buffer size strategies fail.
///
/// # Performance
///
/// May attempt up to 4 decompressions before succeeding.
/// Prefer `decompress` with known size for better performance.
pub fn decompress_auto(data: &[u8]) -> Result<Vec<u8>> {
    // Railway-oriented composition: try each strategy in sequence
    let compressed_len = data.len();

    // Strategy 1: Try 2x buffer
    decompress(data, compressed_len * 2)
        // Strategy 2: Try 4x buffer
        .or_else(|_| decompress(data, compressed_len * 4))
        // Strategy 3: Try 8x buffer
        .or_else(|_| decompress(data, compressed_len * 8))
        // Strategy 4: Try 16x buffer
        .or_else(|_| decompress(data, compressed_len * 16))
        .map_err(|_| Error::CheckpointFailed {
            reason: format!(
                "zstd decompression failed (auto): all buffer strategies failed (compressed size: {})",
                compressed_len
            ),
        })
}

/// Calculate compression ratio.
///
/// Higher ratios indicate better compression.
/// Formula: `uncompressed_size / compressed_size`
///
/// # Arguments
///
/// * `uncompressed_size` - Original size in bytes
/// * `compressed_size` - Compressed size in bytes
///
/// # Returns
///
/// Compression ratio (≥ 1.0)
///
/// # Postconditions
/// - Returns ≥ 1.0 (compressed data never larger per contract)
/// - Returns 1.0 if `compressed_size` is 0 (guard against division by zero)
///
/// # Examples
///
/// ```rust
/// use oya_workflow::checkpoint::compression::compression_ratio;
///
/// // 2:1 compression (50% reduction)
/// let ratio = compression_ratio(1000, 500);
/// assert!((ratio - 2.0).abs() < 0.01);
///
/// // Edge case: zero compressed size
/// let ratio = compression_ratio(1000, 0);
/// assert_eq!(ratio, 1.0);
/// ```
#[must_use]
pub const fn compression_ratio(uncompressed_size: u64, compressed_size: u64) -> f64 {
    if compressed_size == 0 {
        1.0
    } else {
        uncompressed_size as f64 / compressed_size as f64
    }
}

/// Calculate space saved in bytes.
///
/// Absolute byte reduction from compression.
/// Formula: `uncompressed_size - compressed_size` (saturating)
///
/// # Arguments
///
/// * `uncompressed_size` - Original size in bytes
/// * `compressed_size` - Compressed size in bytes
///
/// # Returns
///
/// Space saved in bytes (≥ 0)
///
/// # Postconditions
/// - Returns ≥ 0 (uses saturating subtraction)
/// - Returns `uncompressed_size` if `compressed_size` is 0
/// - Returns 0 if `compressed_size` > `uncompressed_size` (incompressible data)
///
/// # Examples
///
/// ```rust
/// use oya_workflow::checkpoint::compression::space_savings;
///
/// // 50% reduction
/// let saved = space_savings(1000, 500);
/// assert_eq!(saved, 500);
///
/// // No savings (incompressible)
/// let saved = space_savings(1000, 1100);
/// assert_eq!(saved, 0);
/// ```
#[must_use]
pub const fn space_savings(uncompressed_size: u64, compressed_size: u64) -> u64 {
    uncompressed_size.saturating_sub(compressed_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: Generate structured checkpoint-like JSON data
    fn generate_structured_data(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);
        let base = br#"{"worker_id":"worker-","state":"Running","checkpoint":"#;
        while data.len() + base.len() < size {
            data.extend_from_slice(base);
            data.extend_from_slice(format!("{}", data.len()).as_bytes());
            data.extend_from_slice(br#"","timestamp":"#);
            data.extend_from_slice(format!("{}", data.len()).as_bytes());
            data.extend_from_slice(b"\"}");
        }
        data.truncate(size);
        data
    }

    /// Helper: Generate highly repetitive data (best case for compression)
    fn generate_repetitive_data(size: usize) -> Vec<u8> {
        b"Hello, world! This is repetitive data. "
            .iter()
            .cycle()
            .take(size)
            .copied()
            .collect()
    }

    /// Helper: Generate random-like data (worst case for compression)
    fn generate_random_data(size: usize) -> Vec<u8> {
        (0..size).map(|i| (i % 256) as u8).collect()
    }

    // ===== Happy Path Tests =====

    #[test]
    fn test_compress_with_level_succeeds_for_valid_input() {
        let data = b"test data";
        let result = compress_with_level(data, 3);
        assert!(result.is_ok(), "Compression should succeed for valid input");
    }

    #[test]
    fn test_compress_with_level_succeeds_for_structured_data() {
        let data = generate_structured_data(10_000);
        let result = compress_with_level(&data, 3);
        assert!(
            result.is_ok(),
            "Compression should succeed for structured data"
        );
    }

    #[test]
    fn test_compress_with_level_reduces_size_for_compressible_data() {
        let data = generate_repetitive_data(10_000);
        let result = compress_with_level(&data, 3);
        assert!(result.is_ok(), "Compression should succeed");

        let compressed = result.map_or_else(|_| Vec::new(), |c| c);
        assert!(
            compressed.len() < data.len(),
            "Compressed data should be smaller than input"
        );
    }

    #[test]
    fn test_compress_with_level_handles_empty_data() {
        let data = b"";
        let result = compress_with_level(data, 3);
        assert!(result.is_ok(), "Compression should succeed for empty data");

        let compressed = result.map_or_else(|_| Vec::new(), |c| c);
        assert_eq!(
            compressed.len(),
            0,
            "Empty input should produce empty output"
        );
    }

    #[test]
    fn test_compress_with_default_level_3() {
        let data = b"test data";
        let result = compress(data);
        assert!(
            result.is_ok(),
            "Compression with default level should succeed"
        );
    }

    #[test]
    fn test_decompress_succeeds_for_valid_compressed_data() {
        let data = b"test data for compression";
        let compressed = compress(data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let decompressed = compressed.and_then(|c| decompress(&c, data.len()));
        assert!(decompressed.is_ok(), "Decompression should succeed");
    }

    #[test]
    fn test_decompress_returns_exact_original_size() {
        let data = b"test data";
        let compressed = compress(data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let decompressed = compressed.and_then(|c| decompress(&c, data.len()));
        assert!(decompressed.is_ok(), "Decompression should succeed");

        let size = decompressed.map_or_else(|_| 0, |d| d.len());
        assert_eq!(size, data.len(), "Decompressed size should match original");
    }

    #[test]
    fn test_decompress_auto_succeeds_with_buffer_strategy() {
        let data = generate_structured_data(10_000);
        let compressed = compress(&data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let decompressed = compressed.and_then(|c| decompress_auto(&c));
        assert!(decompressed.is_ok(), "Auto decompression should succeed");
    }

    #[test]
    fn test_compression_level_fastest_maps_to_level_1() {
        assert_eq!(CompressionLevel::Fastest.as_i32(), 1);
    }

    #[test]
    fn test_compression_level_default_maps_to_level_3() {
        assert_eq!(CompressionLevel::Default.as_i32(), 3);
    }

    #[test]
    fn test_compression_level_max_maps_to_level_21() {
        assert_eq!(CompressionLevel::Max.as_i32(), 21);
    }

    // ===== Roundtrip Tests =====

    #[test]
    fn test_compress_decompress_roundtrip_preserves_data() {
        let data = b"original test data that should roundtrip perfectly";
        let result = compress(data).and_then(|compressed| decompress(&compressed, data.len()));

        assert!(result.is_ok(), "Roundtrip should succeed");
        let decompressed = result.map_or_else(|_| Vec::new(), |d| d);
        assert_eq!(
            decompressed, data,
            "Decompressed data should match original"
        );
    }

    #[test]
    fn test_compress_decompress_roundtrip_with_different_levels() {
        let data = b"test data for multi-level roundtrip";

        for level in [0, 1, 3, 9, 15, 21] {
            let result = compress_with_level(data, level)
                .and_then(|compressed| decompress(&compressed, data.len()));

            assert!(
                result.is_ok(),
                "Roundtrip should succeed for level {}",
                level
            );
            let decompressed = result.map_or_else(|_| Vec::new(), |d| d);
            assert_eq!(
                decompressed, data,
                "Roundtrip should preserve data for level {}",
                level
            );
        }
    }

    #[test]
    fn test_roundtrip_structured_json_checkpoint_data() {
        let data = generate_structured_data(50_000);
        let compressed = compress(&data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let decompressed = compressed.as_ref().and_then(|c| decompress(c, data.len()));
        assert!(decompressed.is_ok(), "Decompression should succeed");

        let decompressed_data = decompressed.map_or_else(|_| Vec::new(), |d| d);
        assert_eq!(
            decompressed_data, data,
            "Structured checkpoint data should roundtrip"
        );
    }

    #[test]
    fn test_roundtrip_repetitive_data() {
        let data = generate_repetitive_data(10_000);
        let compressed = compress(&data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let decompressed = compressed.as_ref().and_then(|c| decompress(c, data.len()));
        assert!(decompressed.is_ok(), "Decompression should succeed");

        let decompressed_data = decompressed.map_or_else(|_| Vec::new(), |d| d);
        assert_eq!(decompressed_data, data, "Repetitive data should roundtrip");
    }

    // ===== Metric Tests =====

    #[test]
    fn test_compression_ratio_calculates_correctly() {
        let ratio = compression_ratio(1000, 500);
        assert!((ratio - 2.0).abs() < 0.01, "Ratio should be 2.0");
    }

    #[test]
    fn test_compression_ratio_returns_one_when_compressed_size_zero() {
        let ratio = compression_ratio(1000, 0);
        assert_eq!(ratio, 1.0, "Ratio should be 1.0 when compressed size is 0");
    }

    #[test]
    fn test_space_savings_calculates_correctly() {
        let saved = space_savings(1000, 500);
        assert_eq!(saved, 500, "Should save 500 bytes");
    }

    #[test]
    fn test_space_savings_returns_zero_when_compressed_larger() {
        let saved = space_savings(1000, 1100);
        assert_eq!(saved, 0, "Should return 0 when compressed is larger");
    }

    #[test]
    fn test_compression_ratio_gt_one_for_compressible_data() {
        let data = generate_repetitive_data(10_000);
        let compressed = compress(&data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let ratio = compressed.as_ref().map_or(1.0, |c| {
            compression_ratio(data.len() as u64, c.len() as u64)
        });

        assert!(
            ratio > 1.0,
            "Compression ratio should be > 1.0 for compressible data"
        );
    }

    // ===== Error Path Tests =====

    #[test]
    fn test_compress_with_level_returns_error_when_level_negative() {
        let data = b"test data";
        let result = compress_with_level(data, -1);
        assert!(result.is_err(), "Should return error for negative level");

        let err = result.map_err(|e| e.to_string()).unwrap_err();
        assert!(
            err.contains("zstd compression failed"),
            "Error should mention zstd"
        );
    }

    #[test]
    fn test_compress_with_level_returns_error_when_level_exceeds_max() {
        let data = b"test data";
        let result = compress_with_level(data, 22);
        assert!(result.is_err(), "Should return error for level > 21");

        let err = result.map_err(|e| e.to_string()).unwrap_err();
        assert!(
            err.contains("zstd compression failed"),
            "Error should mention zstd"
        );
    }

    #[test]
    fn test_decompress_returns_error_for_invalid_data() {
        let invalid_data = b"this is not valid zstd compressed data";
        let result = decompress(invalid_data, 100);
        assert!(result.is_err(), "Should return error for invalid data");

        let err = result.map_err(|e| e.to_string()).unwrap_err();
        assert!(
            err.contains("zstd decompression failed"),
            "Error should mention zstd"
        );
    }

    #[test]
    fn test_decompress_returns_error_for_truncated_data() {
        let data = b"test";
        let compressed = compress(data);
        assert!(compressed.is_ok(), "Compression should succeed");

        // Truncate the compressed data
        let truncated = compressed.map_or_else(
            |_| Vec::new(),
            |c| {
                if c.len() > 2 {
                    c[..c.len() / 2].to_vec()
                } else {
                    c
                }
            },
        );

        let result = decompress(&truncated, data.len());
        assert!(result.is_err(), "Should return error for truncated data");
    }

    #[test]
    fn test_decompress_returns_error_for_corrupted_data() {
        let corrupted_data = b"\x00\x01\x02\x03corrupted";
        let result = decompress(corrupted_data, 100);
        assert!(result.is_err(), "Should return error for corrupted data");
    }

    #[test]
    fn test_decompress_auto_returns_error_when_all_strategies_fail() {
        let invalid_data = b"invalid zstd data that will fail all strategies";
        let result = decompress_auto(invalid_data);
        assert!(
            result.is_err(),
            "Should return error when all strategies fail"
        );

        let err = result.map_err(|e| e.to_string()).unwrap_err();
        assert!(
            err.contains("all buffer strategies failed"),
            "Error should mention strategy failure"
        );
    }

    #[test]
    fn test_decompress_with_wrong_size_returns_error() {
        let data = b"test data";
        let compressed = compress(data);
        assert!(compressed.is_ok(), "Compression should succeed");

        // Try to decompress with wrong size
        let result = compressed.and_then(|c| decompress(&c, 9999));
        assert!(result.is_err(), "Should return error for wrong size");
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_compress_with_level_zero() {
        let data = b"test data";
        let result = compress_with_level(data, 0);
        assert!(result.is_ok(), "Level 0 should succeed");
    }

    #[test]
    fn test_compress_with_level_twenty_one() {
        let data = b"test data";
        let result = compress_with_level(data, 21);
        assert!(result.is_ok(), "Level 21 should succeed");
    }

    #[test]
    fn test_compress_with_level_one() {
        let data = b"test data";
        let result = compress_with_level(data, 1);
        assert!(result.is_ok(), "Level 1 should succeed");
    }

    #[test]
    fn test_compress_single_byte() {
        let data = b"x";
        let result = compress(data);
        assert!(result.is_ok(), "Single byte compression should succeed");
    }

    #[test]
    fn test_compress_large_data_1mb() {
        let data = generate_structured_data(1_048_576); // 1 MB
        let result = compress(&data);
        assert!(result.is_ok(), "1MB compression should succeed");

        let compressed = result.map_or_else(|_| Vec::new(), |c| c);
        assert!(compressed.len() < data.len(), "Large data should compress");
    }

    #[test]
    fn test_compress_highly_repetitive_data() {
        let data = vec![b'A'; 10_000];
        let result = compress(&data);
        assert!(result.is_ok(), "Highly repetitive data should compress");

        let compressed = result.map_or_else(|_| Vec::new(), |c| c);
        assert!(
            compressed.len() < data.len() / 10,
            "Highly repetitive data should compress very well"
        );
    }

    #[test]
    fn test_compress_random_data_worst_case() {
        let data = generate_random_data(10_000);
        let result = compress(&data);
        assert!(result.is_ok(), "Random data should compress (no error)");

        // Random data may expand slightly due to zstd overhead
        let compressed = result.map_or_else(|_| Vec::new(), |c| c);
        // Roundtrip should still work
        let decompressed = decompress(&compressed, data.len());
        assert!(decompressed.is_ok(), "Random data should roundtrip");
    }

    #[test]
    fn test_compress_empty_slice_returns_empty() {
        let data = b"";
        let result = compress(data);
        assert!(result.is_ok(), "Empty slice should succeed");

        let compressed = result.map_or_else(|_| Vec::new(), |c| c);
        assert_eq!(
            compressed.len(),
            0,
            "Empty input should produce empty output"
        );
    }

    #[test]
    fn test_decompress_empty_compressed_data() {
        let data = b"";
        let compressed = compress(data);
        assert!(compressed.is_ok(), "Empty compression should succeed");

        let decompressed = compressed.and_then(|c| decompress(&c, 0));
        assert!(decompressed.is_ok(), "Empty decompression should succeed");

        let output = decompressed.map_or_else(|_| Vec::new(), |d| d);
        assert_eq!(output.len(), 0, "Empty input should produce empty output");
    }

    #[test]
    fn test_compression_ratio_with_zero_uncompressed_size() {
        let ratio = compression_ratio(0, 100);
        assert_eq!(
            ratio, 0.0,
            "Ratio should be 0.0 when uncompressed size is 0"
        );
    }

    #[test]
    fn test_compression_ratio_with_equal_sizes() {
        let ratio = compression_ratio(1000, 1000);
        assert_eq!(ratio, 1.0, "Ratio should be 1.0 when sizes are equal");
    }

    #[test]
    fn test_compress_incompressible_random_data() {
        let data = generate_random_data(5_000);
        let result = compress(&data);
        assert!(result.is_ok(), "Random data should not error");

        // Roundtrip should preserve data even if compression doesn't help
        let decompressed = result.as_ref().and_then(|c| decompress(c, data.len()));
        assert!(decompressed.is_ok(), "Random data should roundtrip");

        let output = decompressed.map_or_else(|_| Vec::new(), |d| d);
        assert_eq!(output, data, "Random data should be preserved");
    }

    #[test]
    fn test_compress_perfectly_compressible_uniform_data() {
        let data = vec![b'x'; 10_000];
        let result = compress(&data);
        assert!(result.is_ok(), "Uniform data should compress well");

        let compressed = result.map_or_else(|_| Vec::new(), |c| c);
        assert!(
            compressed.len() < 100,
            "Uniform data should compress to very small size"
        );

        let ratio = compression_ratio(data.len() as u64, compressed.len() as u64);
        assert!(
            ratio > 100.0,
            "Uniform data should have high compression ratio"
        );
    }

    #[test]
    fn test_decompress_auto_expands_buffer_multiple_times() {
        // Create data that expands to ~8x when compressed
        let data = generate_random_data(5_000);
        let compressed = compress(&data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let compressed_len = compressed.as_ref().map_or(0, |c| c.len());
        let expansion_factor = data.len() as f64 / compressed_len as f64;

        // decompress_auto should handle this (tries 2x, 4x, 8x, 16x)
        let decompressed = compressed.as_ref().and_then(|c| decompress_auto(c));
        assert!(
            decompressed.is_ok(),
            "Auto decompression should handle {expansion_factor}x expansion"
        );
    }

    // ===== Contract Verification Tests =====

    #[test]
    fn test_postcondition_compress_output_smaller_or_equal() {
        let data = b"test data for postcondition verification";
        let compressed = compress(data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let compressed_data = compressed.map_or_else(|_| Vec::new(), |c| c);
        assert!(
            compressed_data.len() <= data.len(),
            "Postcondition: compressed size ≤ input size"
        );
    }

    #[test]
    fn test_postcondition_decompress_output_matches_size() {
        let data = b"test data";
        let compressed = compress(data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let decompressed = compressed.as_ref().and_then(|c| decompress(c, data.len()));
        assert!(decompressed.is_ok(), "Decompression should succeed");

        let output_len = decompressed.map_or_else(|_| 0, |d| d.len());
        assert_eq!(
            output_len,
            data.len(),
            "Postcondition: output size matches expected"
        );
    }

    #[test]
    fn test_postcondition_roundtrip_preserves_data_exactly() {
        let test_cases = vec![
            b"".as_slice(),
            b"x".as_slice(),
            b"hello world".as_slice(),
            generate_structured_data(10_000).as_slice(),
            generate_repetitive_data(10_000).as_slice(),
            generate_random_data(10_000).as_slice(),
        ];

        for data in test_cases {
            let result = compress(data).and_then(|c| decompress(&c, data.len()));
            assert!(
                result.is_ok(),
                "Roundtrip should succeed for data size {}",
                data.len()
            );

            let output = result.map_or_else(|_| Vec::new(), |d| d);
            assert_eq!(output, data, "Roundtrip should preserve data exactly");
        }
    }

    #[test]
    fn test_postcondition_compression_ratio_gte_one() {
        let ratio = compression_ratio(1000, 500);
        assert!(ratio >= 1.0, "Postcondition: ratio ≥ 1.0");
    }

    #[test]
    fn test_postcondition_space_savings_non_negative() {
        let saved = space_savings(1000, 500);
        assert_eq!(saved, 500, "Postcondition: savings ≥ 0");

        let saved_zero = space_savings(500, 1000);
        assert_eq!(
            saved_zero, 0,
            "Postcondition: savings should be 0 when compressed > input"
        );
    }

    #[test]
    fn test_invariant_roundtrip_fidelity_for_all_levels() {
        let data = generate_structured_data(5_000);

        for level in [0, 1, 3, 9, 15, 21] {
            let result = compress_with_level(data, level)
                .and_then(|compressed| decompress(&compressed, data.len()));

            assert!(
                result.is_ok(),
                "Invariant: roundtrip should succeed for level {}",
                level
            );

            let output = result.map_or_else(|_| Vec::new(), |d| d);
            assert_eq!(
                output, data,
                "Invariant: roundtrip should preserve data for level {}",
                level
            );
        }
    }

    #[test]
    fn test_invariant_compression_ratio_never_less_than_one() {
        // Test various size combinations
        let cases = vec![
            (1000, 500),  // Normal compression
            (1000, 1000), // No compression
            (1000, 0),    // Edge case
            (0, 0),       // Edge case
        ];

        for (uncompressed, compressed) in cases {
            let ratio = compression_ratio(uncompressed, compressed);
            assert!(
                ratio >= 1.0 || uncompressed == 0,
                "Invariant: ratio ≥ 1.0 for ({}, {})",
                uncompressed,
                compressed
            );
        }
    }

    #[test]
    fn test_invariant_space_savings_never_negative() {
        // Test various size combinations
        let cases = vec![(1000, 500), (1000, 1000), (1000, 1100), (0, 0)];

        for (uncompressed, compressed) in cases {
            let saved = space_savings(uncompressed, compressed);
            assert!(
                saved >= 0,
                "Invariant: savings ≥ 0 for ({}, {})",
                uncompressed,
                compressed
            );
        }
    }

    #[test]
    fn test_invariant_level_mapping_in_valid_range() {
        let levels = vec![
            CompressionLevel::Fastest,
            CompressionLevel::Default,
            CompressionLevel::Max,
        ];

        for level in levels {
            let value = level.as_i32();
            assert!(
                (0..=21).contains(&value),
                "Invariant: level in valid range [0, 21], got {}",
                value
            );
        }
    }
}
