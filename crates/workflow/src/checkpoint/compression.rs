//! Compression utilities for checkpoint data.
//!
//! This module provides compression and decompression functions using zstd,
//! along with utilities for calculating compression ratios and space savings.

use crate::error::{Error, Result};

/// Compression level for zstd.
#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    /// Fastest compression (level 1).
    Fastest,
    /// Default compression (level 3).
    Default,
    /// Maximum compression (level 21).
    Max,
}

impl CompressionLevel {
    /// Get the zstd compression level value.
    #[must_use]
    pub const fn as_i32(&self) -> i32 {
        match self {
            Self::Fastest => 1,
            Self::Default => 3,
            Self::Max => 21,
        }
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::Default
    }
}

/// Compress data using zstd with default level (3).
///
/// # Arguments
///
/// * `data` - Uncompressed data
///
/// # Returns
///
/// Compressed data.
///
/// # Errors
///
/// Returns an error if compression fails.
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    zstd::bulk::compress(data, 3)
        .map_err(|e| Error::CheckpointFailed {
            reason: format!("zstd compression failed: {}", e),
        })
}

/// Compress data using zstd with specified level.
///
/// # Arguments
///
/// * `data` - Uncompressed data
/// * `level` - Compression level (0-21)
///
/// # Returns
///
/// Compressed data.
///
/// # Errors
///
/// Returns an error if compression fails.
pub fn compress_with_level(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::bulk::compress(data, level).map_err(|e| Error::CheckpointFailed {
        reason: format!("zstd compression failed: {}", e),
    })
}

/// Decompress data using zstd.
///
/// # Arguments
///
/// * `compressed_data` - Compressed data
/// * `uncompressed_size` - Expected uncompressed size
///
/// # Returns
///
/// Decompressed data.
///
/// # Errors
///
/// Returns an error if decompression fails.
pub fn decompress(compressed_data: &[u8], uncompressed_size: usize) -> Result<Vec<u8>> {
    zstd::bulk::decompress(compressed_data, uncompressed_size).map_err(|e| {
        Error::CheckpointFailed {
            reason: format!("zstd decompression failed: {}", e),
        }
    })
}

/// Decompress data using zstd (auto-detect size).
///
/// # Arguments
///
/// * `data` - Compressed data
///
/// # Returns
///
/// Decompressed data.
///
/// # Errors
///
/// Returns an error if decompression fails.
pub fn decompress_auto(data: &[u8]) -> Result<Vec<u8>> {
    // Start with a 2x buffer estimate, fall back to larger sizes if needed
    decompress(data, data.len() * 2)
        .or_else(|_| decompress(data, data.len() * 4))
        .or_else(|_| decompress(data, data.len() * 8))
        .or_else(|_| decompress(data, data.len() * 16))
}

/// Calculate compression ratio.
///
/// # Arguments
///
/// * `uncompressed_size` - Original size in bytes
/// * `compressed_size` - Compressed size in bytes
///
/// # Returns
///
/// Compression ratio (uncompressed_size / compressed_size).
/// Returns 1.0 if compressed_size is 0.
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
/// # Arguments
///
/// * `uncompressed_size` - Original size in bytes
/// * `compressed_size` - Compressed size in bytes
///
/// # Returns
///
/// Space saved in bytes.
#[must_use]
pub const fn space_savings(uncompressed_size: u64, compressed_size: u64) -> u64 {
    uncompressed_size.saturating_sub(compressed_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DATA: &str = "This is a test string that will be compressed. \
        It contains some repetitive data that should compress well. \
        This is a test string that will be compressed. \
        It contains some repetitive data that should compress well.";

    #[test]
    fn test_compress_decompress_roundtrip() {
        let data = TEST_DATA.as_bytes();
        let compressed = compress(data);
        assert!(compressed.is_ok(), "Compression should succeed");

        let compressed = compressed.expect("compressed should be available");
        let decompressed = decompress(&compressed, data.len());
        assert!(decompressed.is_ok(), "Decompression should succeed");

        let decompressed = decompressed.expect("decompressed should be available");
        assert_eq!(
            decompressed, data,
            "Decompressed data should match original"
        );
    }

    #[test]
    fn test_compression_ratio() {
        let ratio = compression_ratio(1000, 500);
        assert!((ratio - 2.0).abs() < 0.01, "Ratio should be 2.0");
    }

    #[test]
    fn test_space_savings() {
        let saved = space_savings(1000, 500);
        assert_eq!(saved, 500, "Should save 500 bytes");
    }
}
