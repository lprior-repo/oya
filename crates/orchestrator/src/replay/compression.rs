//! Compression utilities for checkpoint data using zstd.
//!
//! Provides configurable compression with level tuning for optimal
//! balance between compression ratio and CPU overhead.

use std::io::Cursor;

pub use error::CompressionError;
pub use types::{CompressionConfig, CompressionLevel, CompressionStats};

mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum CompressionError {
        #[error("Compression failed: {0}")]
        CompressFailed(String),
        #[error("Decompression failed: {0}")]
        DecompressFailed(String),
        #[error("Invalid compression level: {0}. Valid range is 1-22")]
        InvalidLevel(i32),
    }
}

mod types {
    use std::time::Instant;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CompressionLevel(i32);

    impl CompressionLevel {
        pub const MIN: Self = Self(1);
        pub const DEFAULT: Self = Self(3);
        pub const MAX: Self = Self(22);

        pub const fn new(level: i32) -> Result<Self, super::CompressionError> {
            if level >= 1 && level <= 22 {
                Ok(Self(level))
            } else {
                Err(super::CompressionError::InvalidLevel(level))
            }
        }

        pub const fn as_i32(self) -> i32 {
            self.0
        }
    }

    impl Default for CompressionLevel {
        fn default() -> Self {
            Self::DEFAULT
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct CompressionStats {
        pub original_size: usize,
        pub compressed_size: usize,
        pub compression_ratio: f64,
        pub compression_time_ms: u64,
    }

    impl CompressionStats {
        pub fn new(
            original_size: usize,
            compressed_size: usize,
            elapsed: std::time::Duration,
        ) -> Self {
            let ratio = if original_size == 0 {
                1.0
            } else {
                compressed_size as f64 / original_size as f64
            };
            Self {
                original_size,
                compressed_size,
                compression_ratio: ratio,
                compression_time_ms: elapsed.as_millis() as u64,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct CompressionConfig {
        pub level: CompressionLevel,
        pub enable_checksum: bool,
    }

    impl Default for CompressionConfig {
        fn default() -> Self {
            Self {
                level: CompressionLevel::DEFAULT,
                enable_checksum: true,
            }
        }
    }

    impl CompressionConfig {
        pub const fn new(level: CompressionLevel) -> Self {
            Self {
                level,
                enable_checksum: true,
            }
        }

        pub const fn with_checksum(mut self, enable: bool) -> Self {
            self.enable_checksum = enable;
            self
        }
    }
}

pub struct CheckpointCompressor {
    config: CompressionConfig,
}

impl CheckpointCompressor {
    #[must_use]
    pub const fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn with_default_level() -> Self {
        Self::new(CompressionConfig::default())
    }

    #[must_use]
    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }

    pub fn compress(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionStats), CompressionError> {
        let start = Instant::now();
        let level = self.config.level.as_i32();

        let mut encoder = zstd::Encoder::new(Cursor::new(Vec::new()), level)
            .map_err(|e| CompressionError::CompressFailed(e.to_string()))?;

        if self.config.enable_checksum {
            encoder.include_checksum(true).ok();
        }

        use std::io::Write;
        encoder
            .write_all(data)
            .map_err(|e| CompressionError::CompressFailed(e.to_string()))?;

        let compressed = encoder
            .finish()
            .map_err(|e| CompressionError::CompressFailed(e.to_string()))?
            .into_inner();

        let stats = CompressionStats::new(data.len(), compressed.len(), start.elapsed());
        Ok((compressed, stats))
    }

    pub fn decompress(&self, compressed: &[u8]) -> Result<Vec<u8>, CompressionError> {
        let mut decoder = zstd::Decoder::new(Cursor::new(compressed))
            .map_err(|e| CompressionError::DecompressFailed(e.to_string()))?;

        let mut decompressed = Vec::new();
        use std::io::Read;
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| CompressionError::DecompressFailed(e.to_string()))?;

        Ok(decompressed)
    }

    pub fn compress_string(
        &self,
        s: &str,
    ) -> Result<(Vec<u8>, CompressionStats), CompressionError> {
        self.compress(s.as_bytes())
    }

    pub fn decompress_to_string(&self, compressed: &[u8]) -> Result<String, CompressionError> {
        let bytes = self.decompress(compressed)?;
        String::from_utf8(bytes).map_err(|e| CompressionError::DecompressFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

    fn create_test_data(size: usize) -> Vec<u8> {
        let pattern = b"Hello, World! This is a test pattern for compression. ";
        let pattern_len = pattern.len();
        let mut data = Vec::with_capacity(size);
        while data.len() + pattern_len <= size {
            data.extend_from_slice(pattern);
        }
        if data.len() < size {
            let remaining = size - data.len();
            data.extend_from_slice(&pattern[..remaining]);
        }
        data
    }

    #[test]
    fn compression_level_bounds() {
        assert!(CompressionLevel::new(0).is_err());
        assert!(CompressionLevel::new(1).is_ok());
        assert!(CompressionLevel::new(22).is_ok());
        assert!(CompressionLevel::new(23).is_err());
        assert!(CompressionLevel::new(-1).is_err());
    }

    #[test]
    fn default_level_is_three() {
        assert_eq!(CompressionLevel::default().as_i32(), 3);
    }

    #[test]
    fn compress_empty_data() {
        let compressor = CheckpointCompressor::with_default_level();
        let (compressed, stats) = compressor.compress(&[]).unwrap();
        assert!(!compressed.is_empty());
        assert_eq!(stats.original_size, 0);
    }

    #[test]
    fn compress_and_decompress_roundtrip() {
        let compressor = CheckpointCompressor::with_default_level();
        let original = create_test_data(1024);

        let (compressed, _) = compressor.compress(&original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original, decompressed);
    }

    #[test]
    fn compress_string_roundtrip() {
        let compressor = CheckpointCompressor::with_default_level();
        let original = r#"{"workflow_id":"test-123","state":{"beads":["a","b","c"]}}"#;

        let (compressed, _) = compressor.compress_string(original).unwrap();
        let decompressed = compressor.decompress_to_string(&compressed).unwrap();

        assert_eq!(original, decompressed);
    }

    #[test]
    fn compression_ratio_reasonable() {
        let compressor = CheckpointCompressor::with_default_level();
        let original = create_test_data(10000);

        let (_, stats) = compressor.compress(&original).unwrap();

        assert!(
            stats.compression_ratio < 0.5,
            "Should achieve >50% compression"
        );
        assert_eq!(stats.original_size, 10000);
    }

    #[test]
    fn higher_level_better_compression() {
        let data = create_test_data(5000);

        let low_level =
            CheckpointCompressor::new(CompressionConfig::new(CompressionLevel::new(1).unwrap()));
        let high_level =
            CheckpointCompressor::new(CompressionConfig::new(CompressionLevel::new(19).unwrap()));

        let (_, low_stats) = low_level.compress(&data).unwrap();
        let (_, high_stats) = high_level.compress(&data).unwrap();

        assert!(
            high_stats.compressed_size <= low_stats.compressed_size,
            "Higher level should compress better or equal"
        );
    }

    #[test]
    fn decompress_invalid_data_fails() {
        let compressor = CheckpointCompressor::with_default_level();
        let invalid_data = b"not valid zstd compressed data";

        let result = compressor.decompress(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn checksum_validation() {
        let config = CompressionConfig::default().with_checksum(true);
        let compressor = CheckpointCompressor::new(config);
        let original = create_test_data(500);

        let (compressed, _) = compressor.compress(&original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original, decompressed);
    }

    #[test]
    fn no_checksum_compression() {
        let config = CompressionConfig::default().with_checksum(false);
        let compressor = CheckpointCompressor::new(config);
        let original = create_test_data(500);

        let (compressed, _) = compressor.compress(&original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original, decompressed);
    }

    #[test]
    fn stats_time_measured() {
        let compressor = CheckpointCompressor::with_default_level();
        let data = create_test_data(100000);

        let (_, stats) = compressor.compress(&data).unwrap();

        assert!(stats.compression_time_ms < 1000 || stats.original_size > 1_000_000);
    }
}
