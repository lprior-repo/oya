//! Application state management
//!
//! Provides thread-safe state management for the Tauri backend.
//! Uses DashMap for concurrent access and moka for caching.

mod cache;

pub use cache::{BeadCache, CacheConfig};

use crate::error::{AppError, AppResult};
use dashmap::DashMap;
use oya_shared::{Bead, PipelineState};
use parking_lot::RwLock;
use std::path::PathBuf;

/// Application state shared across all Tauri commands
pub struct AppState {
    /// Bead cache for fast lookups
    pub cache: BeadCache,
    /// Active streams (stream_id -> stream info)
    streams: DashMap<String, StreamInfo>,
    /// Pipeline states per task (task_id -> pipeline state)
    pipelines: DashMap<String, PipelineState>,
    /// Project root directory
    project_root: RwLock<Option<PathBuf>>,
    /// Beads directory path
    beads_dir: RwLock<Option<PathBuf>>,
}

/// Information about an active stream
#[derive(Debug)]
pub struct StreamInfo {
    /// Stream identifier
    pub id: String,
    /// Associated bead ID (if any)
    pub bead_id: Option<String>,
    /// Stream status
    pub status: StreamStatus,
}

/// Status of a stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStatus {
    Active,
    Paused,
    Ended,
}

impl AppState {
    /// Create new application state
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: BeadCache::new(),
            streams: DashMap::new(),
            pipelines: DashMap::new(),
            project_root: RwLock::new(None),
            beads_dir: RwLock::new(None),
        }
    }

    /// Create with custom cache configuration
    #[must_use]
    pub fn with_cache_config(config: CacheConfig) -> Self {
        Self {
            cache: BeadCache::with_config(config),
            streams: DashMap::new(),
            pipelines: DashMap::new(),
            project_root: RwLock::new(None),
            beads_dir: RwLock::new(None),
        }
    }

    /// Set the project root directory
    pub fn set_project_root(&self, path: PathBuf) {
        *self.project_root.write() = Some(path);
    }

    /// Get the project root directory
    #[must_use]
    pub fn project_root(&self) -> Option<PathBuf> {
        self.project_root.read().clone()
    }

    /// Set the beads directory
    pub fn set_beads_dir(&self, path: PathBuf) {
        *self.beads_dir.write() = Some(path);
    }

    /// Get the beads directory
    #[must_use]
    pub fn beads_dir(&self) -> Option<PathBuf> {
        self.beads_dir.read().clone()
    }

    /// Load a bead from disk (async file I/O)
    pub async fn load_bead_from_disk(&self, id: &str) -> AppResult<Bead> {
        let beads_dir = self
            .beads_dir()
            .ok_or_else(|| AppError::Config("Beads directory not set".to_string()))?;

        let bead_file = beads_dir.join(format!("{id}.json"));

        let content = tokio::fs::read_to_string(&bead_file).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::BeadNotFound(id.to_string())
            } else {
                AppError::FileSystem(e.to_string())
            }
        })?;

        let bead: Bead = serde_json::from_str(&content)?;
        Ok(bead)
    }

    /// List beads from disk with pagination
    pub async fn list_beads_paginated(&self, offset: usize, limit: usize) -> AppResult<Vec<Bead>> {
        let beads_dir = self
            .beads_dir()
            .ok_or_else(|| AppError::Config("Beads directory not set".to_string()))?;

        // Read directory entries
        let mut entries = tokio::fs::read_dir(&beads_dir)
            .await
            .map_err(|e| AppError::FileSystem(e.to_string()))?;

        let mut bead_files = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                bead_files.push(path);
            }
        }

        // Sort by filename for consistent ordering
        bead_files.sort();

        // Apply pagination
        let paginated: Vec<_> = bead_files.into_iter().skip(offset).take(limit).collect();

        // Load beads concurrently
        let mut beads = Vec::with_capacity(paginated.len());
        for path in paginated {
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    if let Ok(bead) = serde_json::from_str::<Bead>(&content) {
                        beads.push(bead);
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(beads)
    }

    // Pipeline state management

    /// Get pipeline state for a task
    pub async fn get_pipeline_state(&self, task_id: &str) -> Option<PipelineState> {
        self.pipelines.get(task_id).map(|r| r.clone())
    }

    /// Set pipeline state for a task
    pub async fn set_pipeline_state(&self, task_id: String, state: PipelineState) {
        self.pipelines.insert(task_id, state);
    }

    /// Remove pipeline state for a task
    pub fn remove_pipeline_state(&self, task_id: &str) {
        self.pipelines.remove(task_id);
    }

    // Stream management

    /// Register a new stream
    pub fn register_stream(&self, id: String, bead_id: Option<String>) {
        self.streams.insert(
            id.clone(),
            StreamInfo {
                id,
                bead_id,
                status: StreamStatus::Active,
            },
        );
    }

    /// Get stream info
    pub fn get_stream(&self, id: &str) -> Option<StreamInfo> {
        self.streams.get(id).map(|r| StreamInfo {
            id: r.id.clone(),
            bead_id: r.bead_id.clone(),
            status: r.status,
        })
    }

    /// Update stream status
    pub fn update_stream_status(&self, id: &str, status: StreamStatus) {
        if let Some(mut stream) = self.streams.get_mut(id) {
            stream.status = status;
        }
    }

    /// Remove a stream
    pub fn remove_stream(&self, id: &str) {
        self.streams.remove(id);
    }

    /// Get count of active streams
    #[must_use]
    pub fn active_stream_count(&self) -> usize {
        self.streams
            .iter()
            .filter(|r| r.status == StreamStatus::Active)
            .count()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();
        assert!(state.project_root().is_none());
        assert!(state.beads_dir().is_none());
    }

    #[test]
    fn test_project_root() {
        let state = AppState::new();
        let path = PathBuf::from("/test/project");
        state.set_project_root(path.clone());
        assert_eq!(state.project_root(), Some(path));
    }

    #[test]
    fn test_stream_registration() {
        let state = AppState::new();

        state.register_stream("stream-1".to_string(), Some("bead-1".to_string()));

        let info = state.get_stream("stream-1");
        assert!(info.is_some());
        assert_eq!(info.as_ref().map(|i| i.id.as_str()), Some("stream-1"));
        assert_eq!(info.map(|i| i.status), Some(StreamStatus::Active));
    }

    #[test]
    fn test_stream_status_update() {
        let state = AppState::new();

        state.register_stream("stream-2".to_string(), None);
        state.update_stream_status("stream-2", StreamStatus::Paused);

        let info = state.get_stream("stream-2");
        assert_eq!(info.map(|i| i.status), Some(StreamStatus::Paused));
    }

    #[test]
    fn test_active_stream_count() {
        let state = AppState::new();

        state.register_stream("s1".to_string(), None);
        state.register_stream("s2".to_string(), None);
        state.register_stream("s3".to_string(), None);

        assert_eq!(state.active_stream_count(), 3);

        state.update_stream_status("s2", StreamStatus::Ended);
        assert_eq!(state.active_stream_count(), 2);
    }

    #[test]
    fn test_stream_removal() {
        let state = AppState::new();

        state.register_stream("stream-x".to_string(), None);
        assert!(state.get_stream("stream-x").is_some());

        state.remove_stream("stream-x");
        assert!(state.get_stream("stream-x").is_none());
    }
}
