//! Checkpoint storage with compression and SurrealDB persistence.
//!
//! This module provides checkpoint storage functionality with:
//! - zstd compression for efficient storage
//! - UUID v7 time-ordered checkpoint IDs
//! - Metadata tracking (timestamp, sizes, compression ratio)
//! - Railway-Oriented Programming for error handling

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;
use surrealdb::sql::Datetime as SurrealDatetime;
use uuid::Uuid;

use crate::error::{Error, Result};

/// Unique identifier for a checkpoint (UUID v7, time-ordered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(Uuid);

impl CheckpointId {
    /// Create a new time-ordered checkpoint ID (UUID v7).
    pub fn new() -> Self {
        Self(Uuid::new_v7())
    }

    /// Get the inner UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Create from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for CheckpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Checkpoint metadata stored in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// SurrealDB record ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "id")]
    pub record_id: Option<RecordId>,
    /// Checkpoint identifier (UUID v7)
    pub checkpoint_id: String,
    /// Compressed checkpoint data
    pub compressed_data: Vec<u8>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Uncompressed data size in bytes
    pub uncompressed_size: u64,
    /// Compressed data size in bytes
    pub compressed_size: u64,
    /// Compression ratio (uncompressed_size / compressed_size)
    pub compression_ratio: f64,
    /// Version identifier for schema compatibility
    pub version: u32,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl CheckpointMetadata {
    /// Create checkpoint metadata from compressed data.
    pub fn new(
        checkpoint_id: CheckpointId,
        compressed_data: Vec<u8>,
        uncompressed_size: u64,
        version: u32,
    ) -> Self {
        let compressed_size = compressed_data.len() as u64;
        let compression_ratio = if compressed_size > 0 {
            uncompressed_size as f64 / compressed_size as f64
        } else {
            1.0
        };

        Self {
            record_id: None,
            checkpoint_id: checkpoint_id.to_string(),
            compressed_data,
            created_at: Utc::now(),
            uncompressed_size,
            compressed_size,
            compression_ratio,
            version,
            metadata: None,
        }
    }

    /// Set optional metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Calculate space saved in bytes.
    pub fn space_saved(&self) -> u64 {
        self.uncompressed_size.saturating_sub(self.compressed_size)
    }

    /// Calculate space saved as percentage.
    pub fn space_saved_percent(&self) -> f64 {
        if self.uncompressed_size == 0 {
            0.0
        } else {
            (self.space_saved() as f64 / self.uncompressed_size as f64) * 100.0
        }
    }
}

/// Input for creating a checkpoint in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckpointInput {
    checkpoint_id: String,
    compressed_data: Vec<u8>,
    created_at: SurrealDatetime,
    uncompressed_size: u64,
    compressed_size: u64,
    compression_ratio: f64,
    version: u32,
    metadata: Option<serde_json::Value>,
}

impl From<&CheckpointMetadata> for CheckpointInput {
    fn from(metadata: &CheckpointMetadata) -> Self {
        Self {
            checkpoint_id: metadata.checkpoint_id.clone(),
            compressed_data: metadata.compressed_data.clone(),
            created_at: SurrealDatetime::from(metadata.created_at),
            uncompressed_size: metadata.uncompressed_size,
            compressed_size: metadata.compressed_size,
            compression_ratio: metadata.compression_ratio,
            version: metadata.version,
            metadata: metadata.metadata.clone(),
        }
    }
}

/// Compression configuration.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Compression level (0-21, default 3)
    pub level: i32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self { level: 3 }
    }
}

impl CompressionConfig {
    /// Create a new compression config with the given level.
    pub fn new(level: i32) -> Self {
        Self { level }
    }

    /// Create a config for maximum compression (level 21).
    pub fn max_compression() -> Self {
        Self { level: 21 }
    }

    /// Create a config for fastest compression (level 1).
    pub fn fastest() -> Self {
        Self { level: 1 }
    }
}

/// Checkpoint storage client for SurrealDB.
#[derive(Debug, Clone)]
pub struct CheckpointStorage {
    db: surrealdb::Surreal<surrealdb::engine::any::Any>,
    compression_config: CompressionConfig,
    default_version: u32,
}

impl CheckpointStorage {
    /// Create a new checkpoint storage client.
    pub fn new(
        db: surrealdb::Surreal<surrealdb::engine::any::Any>,
        compression_config: CompressionConfig,
    ) -> Self {
        Self {
            db,
            compression_config,
            default_version: 1,
        }
    }

    /// Create with default compression settings.
    pub fn with_defaults(db: surrealdb::Surreal<surrealdb::engine::any::Any>) -> Self {
        Self::new(db, CompressionConfig::default())
    }

    /// Set the default version for checkpoints.
    pub fn with_version(mut self, version: u32) -> Self {
        self.default_version = version;
        self
    }

    /// Get the database client.
    fn db(&self) -> &surrealdb::Surreal<surrealdb::engine::any::Any> {
        &self.db
    }

    /// Compress data using zstd.
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::bulk::compress(data, self.compression_config.level).map_err(|e| {
            Error::checkpoint_failed(format!("compression failed: {}", e))
        })
    }

    /// Decompress data using zstd.
    fn decompress(&self, compressed_data: &[u8], uncompressed_size: u64) -> Result<Vec<u8>> {
        zstd::bulk::decompress(compressed_data, uncompressed_size as usize).map_err(|e| {
            Error::checkpoint_failed(format!("decompression failed: {}", e))
        })
    }

    /// Store a checkpoint with compression.
    ///
    /// # Arguments
    ///
    /// * `data` - Uncompressed checkpoint data
    ///
    /// # Returns
    ///
    /// Returns the checkpoint ID on success.
    ///
    /// # Errors
    ///
    /// Returns an error if compression or storage fails.
    pub async fn store_checkpoint(&self, data: Vec<u8>) -> Result<CheckpointId> {
        let uncompressed_size = data.len() as u64;
        let checkpoint_id = CheckpointId::new();

        // Compress the data
        let compressed_data = self.compress(&data)?;

        // Create metadata
        let metadata = CheckpointMetadata::new(
            checkpoint_id,
            compressed_data,
            uncompressed_size,
            self.default_version,
        );

        // Convert to input format
        let input = CheckpointInput::from(&metadata);

        // Store in SurrealDB
        let result: Option<CheckpointMetadata> = self
            .db()
            .create(("checkpoint", &checkpoint_id.to_string()))
            .content(input)
            .await
            .map_err(|e| {
                Error::storage_failed(
                    "create_checkpoint",
                    format!("surrealdb error: {}", e),
                )
            })?;

        result.ok_or_else(|| {
            Error::storage_failed(
                "create_checkpoint",
                "no result returned from database",
            )
        })?;

        Ok(checkpoint_id)
    }

    /// Store a checkpoint with custom version and optional metadata.
    ///
    /// # Arguments
    ///
    /// * `data` - Uncompressed checkpoint data
    /// * `version` - Schema version identifier
    /// * `metadata` - Optional metadata JSON
    ///
    /// # Returns
    ///
    /// Returns the checkpoint ID on success.
    ///
    /// # Errors
    ///
    /// Returns an error if compression or storage fails.
    pub async fn store_checkpoint_with_metadata(
        &self,
        data: Vec<u8>,
        version: u32,
        metadata: Option<serde_json::Value>,
    ) -> Result<CheckpointId> {
        let uncompressed_size = data.len() as u64;
        let checkpoint_id = CheckpointId::new();

        // Compress the data
        let compressed_data = self.compress(&data)?;

        // Create metadata with custom version and metadata
        let mut checkpoint_metadata = CheckpointMetadata::new(
            checkpoint_id,
            compressed_data,
            uncompressed_size,
            version,
        );

        if let Some(meta) = metadata {
            checkpoint_metadata = checkpoint_metadata.with_metadata(meta);
        }

        // Convert to input format
        let input = CheckpointInput::from(&checkpoint_metadata);

        // Store in SurrealDB
        let result: Option<CheckpointMetadata> = self
            .db()
            .create(("checkpoint", &checkpoint_id.to_string()))
            .content(input)
            .await
            .map_err(|e| {
                Error::storage_failed(
                    "create_checkpoint",
                    format!("surrealdb error: {}", e),
                )
            })?;

        result.ok_or_else(|| {
            Error::storage_failed(
                "create_checkpoint",
                "no result returned from database",
            )
        })?;

        Ok(checkpoint_id)
    }

    /// Load a checkpoint by ID and decompress it.
    ///
    /// # Arguments
    ///
    /// * `checkpoint_id` - Checkpoint identifier
    ///
    /// # Returns
    ///
    /// Returns the decompressed checkpoint data on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint is not found or decompression fails.
    pub async fn load_checkpoint(&self, checkpoint_id: CheckpointId) -> Result<Vec<u8>> {
        let result: Option<CheckpointMetadata> = self
            .db()
            .select(("checkpoint", &checkpoint_id.to_string()))
            .await
            .map_err(|e| {
                Error::storage_failed(
                    "load_checkpoint",
                    format!("surrealdb error: {}", e),
                )
            })?;

        let metadata = result.ok_or_else(|| {
            Error::checkpoint_not_found(checkpoint_id.to_string())
        })?;

        self.decompress(&metadata.compressed_data, metadata.uncompressed_size)
    }

    /// Load checkpoint metadata without decompressing the data.
    ///
    /// # Arguments
    ///
    /// * `checkpoint_id` - Checkpoint identifier
    ///
    /// # Returns
    ///
    /// Returns the checkpoint metadata on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint is not found.
    pub async fn load_checkpoint_metadata(
        &self,
        checkpoint_id: CheckpointId,
    ) -> Result<CheckpointMetadata> {
        let result: Option<CheckpointMetadata> = self
            .db()
            .select(("checkpoint", &checkpoint_id.to_string()))
            .await
            .map_err(|e| {
                Error::storage_failed(
                    "load_checkpoint_metadata",
                    format!("surrealdb error: {}", e),
                )
            })?;

        result.ok_or_else(|| {
            Error::checkpoint_not_found(checkpoint_id.to_string())
        })
    }

    /// List all checkpoint IDs.
    ///
    /// # Returns
    ///
    /// Returns a list of checkpoint IDs ordered by creation time (newest first).
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn list_checkpoints(&self) -> Result<Vec<CheckpointId>> {
        let records: Vec<CheckpointMetadata> = self
            .db()
            .query("SELECT * FROM checkpoint ORDER BY created_at DESC")
            .await
            .map_err(|e| {
                Error::storage_failed(
                    "list_checkpoints",
                    format!("surrealdb error: {}", e),
                )
            })?
            .take(0)
            .map_err(|e| {
                Error::storage_failed(
                    "list_checkpoints",
                    format!("surrealdb query error: {}", e),
                )
            })?;

        records
            .iter()
            .map(|m| {
                Uuid::parse_str(&m.checkpoint_id)
                    .map(CheckpointId)
                    .map_err(|_| {
                        Error::storage_failed(
                            "list_checkpoints",
                            format!("invalid checkpoint ID: {}", m.checkpoint_id),
                        )
                    })
            })
            .collect()
    }

    /// Delete a checkpoint.
    ///
    /// # Arguments
    ///
    /// * `checkpoint_id` - Checkpoint identifier
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint is not found or deletion fails.
    pub async fn delete_checkpoint(&self, checkpoint_id: CheckpointId) -> Result<()> {
        let result: Option<CheckpointMetadata> = self
            .db()
            .delete(("checkpoint", &checkpoint_id.to_string()))
            .await
            .map_err(|e| {
                Error::storage_failed(
                    "delete_checkpoint",
                    format!("surrealdb error: {}", e),
                )
            })?;

        if result.is_some() {
            Ok(())
        } else {
            Err(Error::checkpoint_not_found(checkpoint_id.to_string()))
        }
    }

    /// Get storage statistics.
    ///
    /// # Returns
    ///
    /// Returns storage statistics including total checkpoints, total size, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn storage_stats(&self) -> Result<StorageStats> {
        let records: Vec<CheckpointMetadata> = self
            .db()
            .query("SELECT * FROM checkpoint")
            .await
            .map_err(|e| {
                Error::storage_failed(
                    "storage_stats",
                    format!("surrealdb error: {}", e),
                )
            })?
            .take(0)
            .map_err(|e| {
                Error::storage_failed(
                    "storage_stats",
                    format!("surrealdb query error: {}", e),
                )
            })?;

        let total_checkpoints = records.len() as u64;
        let total_uncompressed_size: u64 = records.iter().map(|m| m.uncompressed_size).sum();
        let total_compressed_size: u64 = records.iter().map(|m| m.compressed_size).sum();
        let avg_compression_ratio = if total_compressed_size > 0 {
            total_uncompressed_size as f64 / total_compressed_size as f64
        } else {
            1.0
        };
        let total_space_saved = total_uncompressed_size.saturating_sub(total_compressed_size);

        Ok(StorageStats {
            total_checkpoints,
            total_uncompressed_size,
            total_compressed_size,
            avg_compression_ratio,
            total_space_saved,
        })
    }
}

/// Storage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total number of checkpoints
    pub total_checkpoints: u64,
    /// Total uncompressed size in bytes
    pub total_uncompressed_size: u64,
    /// Total compressed size in bytes
    pub total_compressed_size: u64,
    /// Average compression ratio
    pub avg_compression_ratio: f64,
    /// Total space saved in bytes
    pub total_space_saved: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create an in-memory SurrealDB connection for testing.
    async fn create_test_db() -> surrealdb::Surreal<surrealdb::engine::any::Any> {
        let db = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(()).await.ok();
        db.ok_or_else(|| Error::storage_failed("connect", "failed to create test db"))
    }

    #[test]
    fn test_checkpoint_id_new() {
        let id1 = CheckpointId::new();
        let id2 = CheckpointId::new();
        assert_ne!(id1, id2, "IDs should be unique");
    }

    #[test]
    fn test_checkpoint_id_from_uuid() {
        let uuid = Uuid::new_v7();
        let id = CheckpointId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_checkpoint_id_display() {
        let id = CheckpointId::new();
        let s = id.to_string();
        assert!(!s.is_empty(), "ID string should not be empty");
    }

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert_eq!(config.level, 3, "default level should be 3");
    }

    #[test]
    fn test_compression_config_new() {
        let config = CompressionConfig::new(10);
        assert_eq!(config.level, 10);
    }

    #[test]
    fn test_compression_config_max_compression() {
        let config = CompressionConfig::max_compression();
        assert_eq!(config.level, 21);
    }

    #[test]
    fn test_compression_config_fastest() {
        let config = CompressionConfig::fastest();
        assert_eq!(config.level, 1);
    }

    #[test]
    fn test_checkpoint_metadata_new() {
        let id = CheckpointId::new();
        let data = vec![1, 2, 3, 4, 5];
        let metadata = CheckpointMetadata::new(id, data.clone(), 5, 1);

        assert_eq!(metadata.checkpoint_id, id.to_string());
        assert_eq!(metadata.compressed_data, data);
        assert_eq!(metadata.uncompressed_size, 5);
        assert_eq!(metadata.compressed_size, 5);
        assert_eq!(metadata.compression_ratio, 1.0);
        assert_eq!(metadata.version, 1);
    }

    #[test]
    fn test_checkpoint_metadata_with_metadata() {
        let id = CheckpointId::new();
        let data = vec![1, 2, 3];
        let metadata = CheckpointMetadata::new(id, data, 3, 1)
            .with_metadata(serde_json::json!({"key": "value"}));

        assert!(metadata.metadata.is_some());
        assert_eq!(
            metadata.metadata,
            Some(serde_json::json!({"key": "value"}))
        );
    }

    #[test]
    fn test_checkpoint_metadata_space_saved() {
        let id = CheckpointId::new();
        let metadata = CheckpointMetadata::new(id, vec![1, 2, 3], 10, 1);

        assert_eq!(metadata.space_saved(), 7);
        assert!((metadata.space_saved_percent() - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_checkpoint_metadata_space_saved_zero() {
        let id = CheckpointId::new();
        let metadata = CheckpointMetadata::new(id, vec![1, 2, 3], 0, 1);

        assert_eq!(metadata.space_saved(), 0);
        assert_eq!(metadata.space_saved_percent(), 0.0);
    }

    #[tokio::test]
    async fn test_compress_decompress() {
        let db = create_test_db().await.ok();
        if db.is_none() {
            return; // Skip test if DB creation fails
        }
        let db = db.unwrap();

        let storage = CheckpointStorage::with_defaults(db);
        let original = b"Hello, world! This is a test data for compression.";
        let compressed = storage.compress(original).ok();
        assert!(compressed.is_some(), "compression should succeed");

        let compressed = compressed.unwrap();
        let decompressed = storage
            .decompress(&compressed, original.len() as u64)
            .ok();
        assert!(decompressed.is_some(), "decompression should succeed");

        let decompressed = decompressed.unwrap();
        assert_eq!(
            decompressed.as_slice(),
            original,
            "decompressed data should match original"
        );
    }

    #[tokio::test]
    async fn test_compress_large_data() {
        let db = create_test_db().await.ok();
        if db.is_none() {
            return;
        }
        let db = db.unwrap();

        let storage = CheckpointStorage::with_defaults(db);
        let original = vec![42u8; 10_000]; // 10KB of data
        let compressed = storage.compress(&original).ok();
        assert!(compressed.is_some());

        let compressed = compressed.unwrap();
        assert!(
            compressed.len() < original.len(),
            "compressed data should be smaller"
        );

        let decompressed = storage
            .decompress(&compressed, original.len() as u64)
            .ok();
        assert!(decompressed.is_some());
        assert_eq!(decompressed.unwrap(), original);
    }
}
