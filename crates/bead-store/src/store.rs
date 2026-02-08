#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! BeadStore implementation with functional core and async shell.
//!
//! # Architecture
//!
//! - **BeadStoreCore**: Pure, synchronous, immutable operations (functional core)
//! - **BeadStore**: Async I/O, persistence, and concurrency (imperative shell)
//!
//! # Example
//!
//! ```no_run
//! use bead_store::{BeadStore, BeadRecord, BeadStatus};
//! use std::path::PathBuf;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let store = BeadStore::new(PathBuf::from(".oya/beads.jsonl")).await?;
//! let beads = store.list_beads().await?;
//! # Ok(())
//! # }
//! ```

use crate::error::StoreError;
use crate::types::{BeadId, BeadRecord, BeadStatus};
use im::{HashMap, HashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Functional core: In-memory bead store with pure operations.
///
/// All operations are pure functions that return new state instead of
/// mutating existing state. This enables easy testing and reasoning.
#[derive(Clone, Debug, Default)]
pub struct BeadStoreCore {
    /// Map of bead ID to bead record.
    beads: HashMap<BeadId, BeadRecord>,
    /// Index of beads by status.
    by_status: HashMap<BeadStatus, HashSet<BeadId>>,
    /// Index of beads by label.
    by_label: HashMap<String, HashSet<BeadId>>,
}

impl BeadStoreCore {
    /// Create a new empty BeadStoreCore.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a bead to the store, returning updated state.
    ///
    /// If a bead with the same ID exists, it will be replaced.
    #[must_use]
    pub fn with_bead(mut self, bead: BeadRecord) -> Self {
        // Remove old version if exists
        if let Some(old_bead) = self.beads.get(&bead.id) {
            self = self.without_bead(&bead.id);
        }

        // Add to main index
        self.beads = self.beads.update(bead.id.clone(), bead.clone());

        // Add to status index
        let status_set = self
            .by_status
            .get(&bead.status)
            .cloned()
            .unwrap_or_default();
        self.by_status = self
            .by_status
            .update(bead.status.clone(), status_set.update(bead.id.clone()));

        // Add to label indices
        for label in &bead.labels {
            let label_set = self.by_label.get(label).cloned().unwrap_or_default();
            self.by_label = self
                .by_label
                .update(label.clone(), label_set.update(bead.id.clone()));
        }

        self
    }

    /// Remove a bead from the store, returning updated state.
    #[must_use]
    pub fn without_bead(mut self, id: &BeadId) -> Self {
        if let Some(bead) = self.beads.get(id) {
            // Remove from status index
            if let Some(status_set) = self.by_status.get(&bead.status) {
                self.by_status = self
                    .by_status
                    .update(bead.status.clone(), status_set.without(id));
            }

            // Remove from label indices
            for label in &bead.labels {
                if let Some(label_set) = self.by_label.get(label) {
                    self.by_label = self.by_label.update(label.clone(), label_set.without(id));
                }
            }

            // Remove from main index
            self.beads = self.beads.without(id);
        }

        self
    }

    /// Get a bead by ID.
    #[must_use]
    pub fn get_bead(&self, id: &BeadId) -> Option<&BeadRecord> {
        self.beads.get(id)
    }

    /// List all beads.
    #[must_use]
    pub fn list_beads(&self) -> Vec<&BeadRecord> {
        self.beads.values().collect()
    }

    /// Filter beads by status.
    #[must_use]
    pub fn filter_by_status(&self, status: BeadStatus) -> Vec<&BeadRecord> {
        self.by_status
            .get(&status)
            .map(|set| {
                set.iter()
                    .filter_map(|id| self.beads.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter beads by labels (AND logic: must have all specified labels).
    #[must_use]
    pub fn filter_by_labels(&self, labels: &[String]) -> Vec<&BeadRecord> {
        if labels.is_empty() {
            return self.list_beads();
        }

        // Find beads that have ALL the specified labels
        self.by_label
            .get(&labels[0])
            .map(|first_set| {
                first_set
                    .iter()
                    .filter(|id| {
                        // Check if bead has all remaining labels
                        labels.iter().all(|label| {
                            self.beads
                                .get(id)
                                .map(|bead| bead.has_label(label))
                                .unwrap_or(false)
                        })
                    })
                    .filter_map(|id| self.beads.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the number of beads in the store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.beads.len()
    }

    /// Check if the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.beads.is_empty()
    }
}

/// BeadStore with async I/O and persistence.
///
/// This is the "imperative shell" that handles I/O, concurrency, and
/// delegates business logic to the pure `BeadStoreCore`.
#[derive(Clone, Debug)]
pub struct BeadStore {
    /// In-memory core state (protected by RwLock).
    core: Arc<RwLock<BeadStoreCore>>,
    /// Path to the storage file.
    storage_path: PathBuf,
}

impl BeadStore {
    /// Create a new BeadStore, loading existing data if present.
    ///
    /// # Errors
    ///
    /// Returns `StoreError` if the storage file cannot be read or parsed.
    pub async fn new(storage_path: PathBuf) -> Result<Self, StoreError> {
        info!("Creating BeadStore with path: {:?}", storage_path);

        let store = Self {
            core: Arc::new(RwLock::new(BeadStoreCore::new())),
            storage_path,
        };

        // Load existing data if file exists
        if store.storage_path.exists() {
            store.load().await?;
        }

        Ok(store)
    }

    /// Load beads from the storage file.
    ///
    /// # Errors
    ///
    /// Returns `StoreError` if the file cannot be read or parsed.
    pub async fn load(&self) -> Result<(), StoreError> {
        debug!("Loading beads from: {:?}", self.storage_path);

        let content = tokio::fs::read_to_string(&self.storage_path).await?;
        let mut core = self.core.write().await;

        // Parse JSONL format (one JSON object per line)
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<BeadRecord>(line) {
                Ok(bead) => {
                    let current = std::mem::replace(&mut *core, BeadStoreCore::new());
                    *core = current.with_bead(bead);
                }
                Err(e) => {
                    warn!("Failed to parse bead line: {} - Error: {}", line, e);
                    // Continue parsing other lines instead of failing completely
                }
            }
        }

        info!("Loaded {} beads from disk", core.len());
        Ok(())
    }

    /// Save beads to the storage file atomically.
    ///
    /// # Errors
    ///
    /// Returns `StoreError` if the file cannot be written.
    pub async fn save(&self) -> Result<(), StoreError> {
        debug!("Saving {} beads to: {:?}", self.core.read().await.len(), self.storage_path);

        let core = self.core.read().await;
        let beads = core.list_beads();

        // Build content in memory
        let mut content = String::new();
        for bead in beads {
            let json = serde_json::to_string(bead)?;
            content.push_str(&json);
            content.push('\n');
        }

        // Write to temporary file first
        let temp_path = self.temp_path();
        let mut temp_file = tokio::fs::File::create(&temp_path).await?;
        temp_file.write_all(content.as_bytes()).await?;
        temp_file.flush().await?;
        drop(temp_file);

        // Atomically rename temp file to actual path
        tokio::fs::rename(&temp_path, &self.storage_path).await?;

        info!("Saved {} beads to disk", beads.len());
        Ok(())
    }

    /// Get a bead by ID.
    ///
    /// # Errors
    ///
    /// Never returns an error (returns Ok(None) if not found).
    pub async fn get_bead(&self, id: &BeadId) -> Result<Option<BeadRecord>, StoreError> {
        let core = self.core.read().await;
        Ok(core.get_bead(id).cloned())
    }

    /// List all beads.
    ///
    /// # Errors
    ///
    /// Never returns an error (returns empty vec if store is empty).
    pub async fn list_beads(&self) -> Result<Vec<BeadRecord>, StoreError> {
        let core = self.core.read().await;
        Ok(core.list_beads().into_iter().cloned().collect())
    }

    /// Filter beads by status.
    ///
    /// # Errors
    ///
    /// Never returns an error (returns empty vec if no matches).
    pub async fn filter_by_status(
        &self,
        status: BeadStatus,
    ) -> Result<Vec<BeadRecord>, StoreError> {
        let core = self.core.read().await;
        Ok(core.filter_by_status(status).into_iter().cloned().collect())
    }

    /// Filter beads by labels (AND logic).
    ///
    /// # Errors
    ///
    /// Never returns an error (returns empty vec if no matches).
    pub async fn filter_by_labels(&self, labels: &[String]) -> Result<Vec<BeadRecord>, StoreError> {
        let core = self.core.read().await;
        Ok(core.filter_by_labels(labels).into_iter().cloned().collect())
    }

    /// Update or insert a bead.
    ///
    /// # Errors
    ///
    /// Returns `StoreError` if the bead cannot be validated.
    pub async fn update_bead(&self, bead: BeadRecord) -> Result<(), StoreError> {
        let mut core = self.core.write().await;
        let current = std::mem::replace(&mut *core, BeadStoreCore::new());
        *core = current.with_bead(bead);
        Ok(())
    }

    /// Insert a new bead.
    ///
    /// # Errors
    ///
    /// Returns `StoreError` if a bead with the same ID already exists.
    pub async fn insert_bead(&self, bead: BeadRecord) -> Result<(), StoreError> {
        let mut core = self.core.write().await;

        // Check for duplicate
        if core.get_bead(&bead.id).is_some() {
            return Err(StoreError::InvalidData(format!(
                "Bead with ID {} already exists",
                bead.id
            )));
        }

        let current = std::mem::replace(&mut *core, BeadStoreCore::new());
        *core = current.with_bead(bead);
        Ok(())
    }

    /// Get the number of beads in the store.
    ///
    /// # Errors
    ///
    /// Never returns an error.
    pub async fn len(&self) -> Result<usize, StoreError> {
        let core = self.core.read().await;
        Ok(core.len())
    }

    /// Check if the store is empty.
    ///
    /// # Errors
    ///
    /// Never returns an error.
    pub async fn is_empty(&self) -> Result<bool, StoreError> {
        let core = self.core.read().await;
        Ok(core.is_empty())
    }

    /// Generate temp file path for atomic writes.
    fn temp_path(&self) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        self.storage_path.hash(&mut hasher);
        let hash = hasher.finish();
        self.storage_path
            .with_extension(format!("jsonl.tmp.{hash:x}"))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_core_new_empty() {
        let core = BeadStoreCore::new();
        assert!(core.is_empty());
        assert_eq!(core.len(), 0);
    }

    #[test]
    fn test_core_with_bead() {
        let core = BeadStoreCore::new();
        let bead = BeadRecord::test_fixture();
        let core = core.with_bead(bead.clone());

        assert_eq!(core.len(), 1);
        assert_eq!(core.get_bead(&bead.id), Some(&bead));
    }

    #[test]
    fn test_core_without_bead() {
        let core = BeadStoreCore::new();
        let bead = BeadRecord::test_fixture();
        let core = core.with_bead(bead.clone());
        assert_eq!(core.len(), 1);

        let core = core.without_bead(&bead.id);
        assert_eq!(core.len(), 0);
        assert_eq!(core.get_bead(&bead.id), None);
    }

    #[test]
    fn test_core_list_beads() {
        let core = BeadStoreCore::new();
        let bead1 = BeadRecord::test_fixture();
        let bead2 = BeadRecord::new(
            "bead-2",
            "Second",
            "Desc",
            BeadStatus::InProgress,
            1,
        );

        let core = core.with_bead(bead1.clone()).with_bead(bead2.clone());
        let beads = core.list_beads();

        assert_eq!(beads.len(), 2);
        assert!(beads.contains(&bead1));
        assert!(beads.contains(&bead2));
    }

    #[test]
    fn test_core_filter_by_status() {
        let core = BeadStoreCore::new();
        let bead1 = BeadRecord::new("b1", "B1", "D1", BeadStatus::Open, 0);
        let bead2 = BeadRecord::new("b2", "B2", "D2", BeadStatus::InProgress, 0);
        let bead3 = BeadRecord::new("b3", "B3", "D3", BeadStatus::Open, 0);

        let core = core.with_bead(bead1).with_bead(bead2).with_bead(bead3);
        let open_beads = core.filter_by_status(BeadStatus::Open);

        assert_eq!(open_beads.len(), 2);
    }

    #[test]
    fn test_core_filter_by_labels() {
        let core = BeadStoreCore::new();
        let bead1 = BeadRecord::with_labels(
            "b1",
            "B1",
            "D1",
            BeadStatus::Open,
            0,
            vec!["test".to_string(), "priority".to_string()],
        );
        let bead2 = BeadRecord::with_labels(
            "b2",
            "B2",
            "D2",
            BeadStatus::Open,
            0,
            vec!["test".to_string()],
        );

        let core = core.with_bead(bead1.clone()).with_bead(bead2);
        let filtered = core.filter_by_labels(&["test".to_string(), "priority".to_string()]);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, bead1.id);
    }

    #[tokio::test]
    async fn test_store_new_empty() {
        let temp = tempdir().expect("failed to create tempdir");
        let path = temp.path().join("beads.jsonl");

        let store = BeadStore::new(path.clone())
            .await
            .expect("failed to create store");

        assert!(store.is_empty().await.expect("failed to check empty"));
        assert_eq!(store.len().await.expect("failed to get len"), 0);
    }

    #[tokio::test]
    async fn test_store_insert_and_get() {
        let temp = tempdir().expect("failed to create tempdir");
        let path = temp.path().join("beads.jsonl");

        let store = BeadStore::new(path)
            .await
            .expect("failed to create store");

        let bead = BeadRecord::test_fixture();
        store
            .insert_bead(bead.clone())
            .await
            .expect("failed to insert bead");

        let retrieved = store
            .get_bead(&bead.id)
            .await
            .expect("failed to get bead");

        assert_eq!(retrieved, Some(bead));
    }

    #[tokio::test]
    async fn test_store_duplicate_insert() {
        let temp = tempdir().expect("failed to create tempdir");
        let path = temp.path().join("beads.jsonl");

        let store = BeadStore::new(path)
            .await
            .expect("failed to create store");

        let bead = BeadRecord::test_fixture();
        store
            .insert_bead(bead.clone())
            .await
            .expect("failed to insert first bead");

        let result = store.insert_bead(bead).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_save_and_load() {
        let temp = tempdir().expect("failed to create tempdir");
        let path = temp.path().join("beads.jsonl");

        let store = BeadStore::new(path.clone())
            .await
            .expect("failed to create store");

        // Insert beads
        let bead1 = BeadRecord::test_fixture();
        let bead2 = BeadRecord::new(
            "bead-2",
            "Second",
            "Description",
            BeadStatus::InProgress,
            1,
        );

        store
            .insert_bead(bead1.clone())
            .await
            .expect("failed to insert bead1");
        store
            .insert_bead(bead2.clone())
            .await
            .expect("failed to insert bead2");

        // Save to disk
        store.save().await.expect("failed to save store");

        // Load into new store instance
        let store2 = BeadStore::new(path)
            .await
            .expect("failed to create store2");

        assert_eq!(store2.len().await.expect("failed to get len"), 2);
        assert_eq!(
            store2.get_bead(&bead1.id).await.expect("failed to get bead1"),
            Some(bead1)
        );
        assert_eq!(
            store2.get_bead(&bead2.id).await.expect("failed to get bead2"),
            Some(bead2)
        );
    }

    #[tokio::test]
    async fn test_store_filter_by_status() {
        let temp = tempdir().expect("failed to create tempdir");
        let path = temp.path().join("beads.jsonl");

        let store = BeadStore::new(path)
            .await
            .expect("failed to create store");

        let bead1 = BeadRecord::new("b1", "B1", "D1", BeadStatus::Open, 0);
        let bead2 = BeadRecord::new("b2", "B2", "D2", BeadStatus::InProgress, 0);
        let bead3 = BeadRecord::new("b3", "B3", "D3", BeadStatus::Open, 0);

        store.insert_bead(bead1.clone()).await.expect("failed to insert b1");
        store.insert_bead(bead2.clone()).await.expect("failed to insert b2");
        store.insert_bead(bead3.clone()).await.expect("failed to insert b3");

        let open_beads = store
            .filter_by_status(BeadStatus::Open)
            .await
            .expect("failed to filter by status");

        assert_eq!(open_beads.len(), 2);
        assert!(open_beads.iter().any(|b| b.id == bead1.id));
        assert!(open_beads.iter().any(|b| b.id == bead3.id));
    }

    #[tokio::test]
    async fn test_store_update_bead() {
        let temp = tempdir().expect("failed to create tempdir");
        let path = temp.path().join("beads.jsonl");

        let store = BeadStore::new(path)
            .await
            .expect("failed to create store");

        let bead = BeadRecord::new("b1", "B1", "D1", BeadStatus::Open, 0);
        store.insert_bead(bead.clone()).await.expect("failed to insert");

        // Update bead status
        let updated = bead.with_status(BeadStatus::InProgress);
        store
            .update_bead(updated.clone())
            .await
            .expect("failed to update");

        let retrieved = store
            .get_bead(&bead.id)
            .await
            .expect("failed to get bead")
            .expect("bead not found");

        assert_eq!(retrieved.status, BeadStatus::InProgress);
    }
}
