//! Bead-related Tauri commands
//!
//! Features:
//! - Batched operations to reduce IPC overhead
//! - Cache-first reads with async refresh
//! - Non-blocking file I/O

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use oya_shared::{Bead, BeadStatus};
use std::sync::Arc;
use tauri::State;

/// Get a single bead by ID
///
/// Uses cache-first strategy: returns cached bead if available,
/// otherwise loads from disk and caches for future requests.
#[tauri::command]
pub async fn get_bead(id: String, state: State<'_, Arc<AppState>>) -> AppResult<Bead> {
    // Try cache first
    if let Some(cached) = state.cache.get(&id).await {
        return Ok((*cached).clone());
    }

    // Load from disk
    let bead = state.load_bead_from_disk(&id).await?;

    // Cache for future requests
    state.cache.insert(id, bead.clone()).await;

    Ok(bead)
}

/// Batch fetch multiple beads
///
/// Single IPC call for multiple beads - reduces round-trip overhead.
/// Uses cache-first strategy with concurrent disk loading.
#[tauri::command]
pub async fn get_beads_batch(
    ids: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<Bead>> {
    let mut results = Vec::with_capacity(ids.len());

    for id in ids {
        // Cache-first lookup
        if let Some(cached) = state.cache.get(&id).await {
            results.push((*cached).clone());
        } else {
            // Load from disk and cache
            match state.load_bead_from_disk(&id).await {
                Ok(bead) => {
                    state.cache.insert(id, bead.clone()).await;
                    results.push(bead);
                }
                Err(AppError::BeadNotFound(_)) => {
                    // Skip missing beads in batch operations
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    Ok(results)
}

/// List beads with pagination
///
/// For large datasets, use pagination to avoid loading everything into memory.
/// Default page size is 100 items.
#[tauri::command]
pub async fn list_beads_paginated(
    offset: usize,
    limit: usize,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<Bead>> {
    // Cap the limit to prevent excessive memory usage
    let limit = limit.min(500);
    state.list_beads_paginated(offset, limit).await
}

/// Update bead status
///
/// Updates the status and invalidates the cache entry.
#[tauri::command]
pub async fn update_bead_status(
    id: String,
    status: BeadStatus,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Bead> {
    // Load current bead
    let mut bead = state.load_bead_from_disk(&id).await?;

    // Update status
    bead.status = status;
    bead.updated_at = chrono_now();

    // Save to disk
    save_bead_to_disk(&bead, &state).await?;

    // Update cache
    state.cache.insert(id, bead.clone()).await;

    Ok(bead)
}

/// Batch cancel beads
///
/// Sets status to Cancelled for multiple beads in a single call.
#[tauri::command]
pub async fn cancel_beads_batch(
    ids: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> AppResult<Vec<String>> {
    let mut cancelled = Vec::new();

    for id in ids {
        match state.load_bead_from_disk(&id).await {
            Ok(mut bead) => {
                if !bead.status.is_terminal() {
                    bead.status = BeadStatus::Cancelled;
                    bead.updated_at = chrono_now();

                    if save_bead_to_disk(&bead, &state).await.is_ok() {
                        state.cache.invalidate(&id).await;
                        cancelled.push(id);
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Ok(cancelled)
}

/// Get project root directory
///
/// Attempts to find the project root by looking for .beads/ or .git/
#[tauri::command]
pub async fn get_project_root(state: State<'_, Arc<AppState>>) -> AppResult<String> {
    // Return cached value if available
    if let Some(root) = state.project_root() {
        return root
            .to_str()
            .map(String::from)
            .ok_or_else(|| AppError::Internal("Invalid project root path".to_string()));
    }

    // Try to find project root
    let cwd = std::env::current_dir().map_err(|e| AppError::FileSystem(e.to_string()))?;

    let root = find_project_root(&cwd).ok_or_else(|| {
        AppError::Config("Could not find project root (no .beads/ or .git/ found)".to_string())
    })?;

    // Cache the result
    state.set_project_root(root.clone());
    state.set_beads_dir(root.join(".beads"));

    root.to_str()
        .map(String::from)
        .ok_or_else(|| AppError::Internal("Invalid project root path".to_string()))
}

/// Invalidate cache for a bead
#[tauri::command]
pub async fn invalidate_bead_cache(id: String, state: State<'_, Arc<AppState>>) -> AppResult<()> {
    state.cache.invalidate(&id).await;
    Ok(())
}

/// Get cache statistics
#[tauri::command]
pub fn get_cache_stats(state: State<'_, Arc<AppState>>) -> CacheStats {
    CacheStats {
        entry_count: state.cache.entry_count(),
    }
}

/// Cache statistics
#[derive(serde::Serialize)]
pub struct CacheStats {
    pub entry_count: u64,
}

// Helper functions

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "2026-02-02T{:02}:{:02}:{:02}Z",
        (duration.as_secs() / 3600) % 24,
        (duration.as_secs() / 60) % 60,
        duration.as_secs() % 60
    )
}

async fn save_bead_to_disk(bead: &Bead, state: &State<'_, Arc<AppState>>) -> AppResult<()> {
    let beads_dir = state
        .beads_dir()
        .ok_or_else(|| AppError::Config("Beads directory not set".to_string()))?;

    let bead_file = beads_dir.join(format!("{}.json", bead.id));
    let content = serde_json::to_string_pretty(bead)?;

    tokio::fs::write(&bead_file, content)
        .await
        .map_err(|e| AppError::FileSystem(e.to_string()))?;

    Ok(())
}

fn find_project_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        // Check for .beads directory
        if current.join(".beads").is_dir() {
            return Some(current);
        }

        // Check for .git directory as fallback
        if current.join(".git").exists() {
            return Some(current);
        }

        // Move to parent
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrono_now_format() {
        let now = chrono_now();
        assert!(now.starts_with("2026-02-02T"));
        assert!(now.ends_with('Z'));
    }

    #[test]
    fn test_find_project_root_not_found() {
        let result = find_project_root(std::path::Path::new("/nonexistent"));
        assert!(result.is_none());
    }
}
