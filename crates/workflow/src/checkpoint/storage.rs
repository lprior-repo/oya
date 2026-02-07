//! Checkpoint storage to SurrealDB.
//!
//! This module provides persistent storage for compressed checkpoints
//! with metadata tracking (timestamp, version, size, compression ratio).

use itertools::Itertools;

// Removed: use std::collections::HashMap;

/// Checkpoint storage errors.
#[derive(Debug, Clone)]
pub enum StorageError {
    /// Checkpoint not found.
    NotFound { checkpoint_id: String },
    /// Storage operation failed.
    StorageFailed { reason: String },
    /// Serialization/deserialization failed.
    CodecFailed { reason: String },
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { checkpoint_id } => {
                write!(f, "checkpoint '{checkpoint_id}' not found")
            }
            Self::StorageFailed { reason } => {
                write!(f, "storage failed: {reason}")
            }
            Self::CodecFailed { reason } => {
                write!(f, "codec failed: {reason}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

/// Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Unique identifier for a checkpoint.
pub type CheckpointId = super::restore::CheckpointId;

/// Metadata about a stored checkpoint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointMetadata {
    /// Unique checkpoint identifier.
    pub id: CheckpointId,
    /// Timestamp when checkpoint was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Version of the checkpoint format.
    pub version: u32,
    /// Size of uncompressed data in bytes.
    pub uncompressed_size: usize,
    /// Size of compressed data in bytes.
    pub compressed_size: usize,
    /// Compression ratio (uncompressed / compressed).
    pub compression_ratio: f64,
}

/// Compression configuration for checkpoint storage.
#[derive(Debug, Clone, Copy)]
pub struct CompressionConfig {
    /// Zstd compression level (0-21).
    pub level: i32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self { level: 3 }
    }
}

/// Storage statistics.
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total number of checkpoints stored.
    pub total_checkpoints: usize,
    /// Total size of all compressed data in bytes.
    pub total_compressed_size: u64,
    /// Total size of all uncompressed data in bytes.
    pub total_uncompressed_size: u64,
    /// Average compression ratio.
    pub average_compression_ratio: f64,
}

/// Checkpoint storage trait.
pub trait CheckpointStorage: Send + Sync {
    /// Store a checkpoint with metadata.
    fn store_checkpoint(
        &mut self,
        data: Vec<u8>,
        metadata: CheckpointMetadata,
    ) -> StorageResult<CheckpointId>;

    /// Load a checkpoint by ID.
    fn load_checkpoint(&self, id: &CheckpointId) -> StorageResult<(Vec<u8>, CheckpointMetadata)>;

    /// Delete a checkpoint by ID.
    fn delete_checkpoint(&mut self, id: &CheckpointId) -> StorageResult<()>;

    /// List all checkpoint IDs.
    fn list_checkpoints(&self) -> StorageResult<Vec<CheckpointId>>;

    /// Get storage statistics.
    fn get_stats(&self) -> StorageResult<StorageStats>;

    /// Clear all checkpoints.
    fn clear_all(&mut self) -> StorageResult<()>;
}

/// In-memory checkpoint storage for testing.
///
/// This storage uses persistent HashMap from `im` crate for functional updates.
#[derive(Debug, Default)]
pub struct InMemoryCheckpointStorage {
    checkpoints: im::HashMap<CheckpointId, (Vec<u8>, CheckpointMetadata)>,
}

impl InMemoryCheckpointStorage {
    /// Create a new in-memory storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl CheckpointStorage for InMemoryCheckpointStorage {
    fn store_checkpoint(
        &mut self,
        data: Vec<u8>,
        metadata: CheckpointMetadata,
    ) -> StorageResult<CheckpointId> {
        let id = metadata.id;
        // Functional: Update using persistent HashMap (returns new map)
        self.checkpoints = self.checkpoints.update(id, (data, metadata));
        Ok(id)
    }

    fn load_checkpoint(&self, id: &CheckpointId) -> StorageResult<(Vec<u8>, CheckpointMetadata)> {
        // Functional: Use get() from im::HashMap (returns Option<&V>)
        self.checkpoints
            .get(id)
            .map(|(data, meta)| (data.clone(), meta.clone()))
            .ok_or_else(|| StorageError::NotFound {
                checkpoint_id: id.to_string(),
            })
    }

    fn delete_checkpoint(&mut self, id: &CheckpointId) -> StorageResult<()> {
        // Functional: Update using persistent HashMap without mutation
        match self.checkpoints.get(id) {
            Some(_) => {
                self.checkpoints = self.checkpoints.without(id);
                Ok(())
            }
            None => Err(StorageError::NotFound {
                checkpoint_id: id.to_string(),
            }),
        }
    }

    fn list_checkpoints(&self) -> StorageResult<Vec<CheckpointId>> {
        // Functional: Use iterator from im::HashMap
        Ok(self.checkpoints.keys().copied().collect_vec())
    }

    fn get_stats(&self) -> StorageResult<StorageStats> {
        // Functional: Use im::HashMap iterator and fold
        let total_checkpoints = self.checkpoints.len();

        let (total_compressed_size, total_uncompressed_size, total_compression_ratio) =
            self.checkpoints.values().fold(
                (0u64, 0u64, 0.0),
                |(comp, uncomp, ratio), (_data, metadata)| {
                    (
                        comp + metadata.compressed_size as u64,
                        uncomp + metadata.uncompressed_size as u64,
                        ratio + metadata.compression_ratio,
                    )
                },
            );

        let average_compression_ratio = if total_checkpoints > 0 {
            total_compression_ratio / total_checkpoints as f64
        } else {
            1.0
        };

        Ok(StorageStats {
            total_checkpoints,
            total_compressed_size,
            total_uncompressed_size,
            average_compression_ratio,
        })
    }

    fn clear_all(&mut self) -> StorageResult<()> {
        // Functional: Replace with empty persistent HashMap
        self.checkpoints = im::HashMap::new();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_storage_roundtrip() {
        let mut storage = InMemoryCheckpointStorage::new();
        let id = CheckpointId::new();
        let data = vec![1, 2, 3, 4, 5];

        let metadata = CheckpointMetadata {
            id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: 5,
            compressed_size: 3,
            compression_ratio: 1.6667,
        };

        // Store
        let stored_id = storage.store_checkpoint(data.clone(), metadata.clone());
        assert!(stored_id.is_ok());
        assert_eq!(stored_id.map_or(id, |v| v), id);

        // Load
        let loaded = storage.load_checkpoint(&id);
        assert!(loaded.is_ok());
        let (loaded_data, loaded_metadata) = loaded.map_or((vec![], metadata.clone()), |v| v);
        assert_eq!(loaded_data, data);
        assert_eq!(loaded_metadata.id, id);
    }

    #[test]
    fn test_in_memory_storage_not_found() {
        let storage = InMemoryCheckpointStorage::new();
        let id = CheckpointId::new();

        let result = storage.load_checkpoint(&id);
        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::NotFound { .. })));
    }

    #[test]
    fn test_in_memory_storage_delete() {
        let mut storage = InMemoryCheckpointStorage::new();
        let id = CheckpointId::new();
        let data = vec![1, 2, 3];

        let metadata = CheckpointMetadata {
            id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: 3,
            compressed_size: 2,
            compression_ratio: 1.5,
        };

        storage
            .store_checkpoint(data, metadata)
            .map_err(|e| format!("{:?}", e))
            .unwrap();

        // Delete
        let result = storage.delete_checkpoint(&id);
        assert!(result.is_ok());

        // Verify deleted
        let result = storage.load_checkpoint(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_in_memory_storage_list() {
        let mut storage = InMemoryCheckpointStorage::new();
        let id1 = CheckpointId::new();
        let id2 = CheckpointId::new();

        let metadata1 = CheckpointMetadata {
            id: id1,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: 3,
            compressed_size: 2,
            compression_ratio: 1.5,
        };

        let metadata2 = CheckpointMetadata {
            id: id2,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: 5,
            compressed_size: 3,
            compression_ratio: 1.6667,
        };

        storage
            .store_checkpoint(vec![1, 2, 3], metadata1)
            .map_err(|e| format!("{:?}", e))
            .unwrap();
        storage
            .store_checkpoint(vec![4, 5, 6, 7, 8], metadata2)
            .map_err(|e| format!("{:?}", e))
            .unwrap();

        let ids = storage.list_checkpoints().map_or(vec![], |v| v);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn test_in_memory_storage_stats() {
        let mut storage = InMemoryCheckpointStorage::new();
        let id = CheckpointId::new();

        let metadata = CheckpointMetadata {
            id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: 100,
            compressed_size: 50,
            compression_ratio: 2.0,
        };

        storage
            .store_checkpoint(vec![0u8; 100], metadata)
            .map_err(|e| format!("{:?}", e))
            .unwrap();

        let stats = storage.get_stats().map_or(StorageStats::default(), |v| v);
        assert_eq!(stats.total_checkpoints, 1);
        assert_eq!(stats.total_compressed_size, 50);
        assert_eq!(stats.total_uncompressed_size, 100);
        assert!((stats.average_compression_ratio - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_in_memory_storage_clear() {
        let mut storage = InMemoryCheckpointStorage::new();
        let id = CheckpointId::new();

        let metadata = CheckpointMetadata {
            id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: 3,
            compressed_size: 2,
            compression_ratio: 1.5,
        };

        storage
            .store_checkpoint(vec![1, 2, 3], metadata)
            .map_err(|e| format!("{:?}", e))
            .unwrap();

        // Clear
        let result = storage.clear_all();
        assert!(result.is_ok());

        // Verify cleared
        let ids = storage.list_checkpoints().map_or(vec![], |v| v);
        assert_eq!(ids.len(), 0);
    }
}
