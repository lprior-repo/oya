# Rust Contract: BeadStore Implementation

**Bead ID**: src-1p3y
**Title**: Implement BeadStore: Persistent storage for bead tracking
**Created**: 2026-02-08
**Status**: In Progress

## Overview

Implement `BeadStore`, a persistent storage layer for bead tracking that provides centralized bead state management, query interface for IPC worker commands, persistence across orchestrator restarts, and enables distributed agent coordination.

## Problem Statement

Multiple TODO comments reference querying beads from BeadStore, but this storage layer doesn't exist:

- `crates/orchestrator/src/actors/scheduler.rs` - References bead storage for querying
- IPC workers need to query actual bead lists and details
- No persistence mechanism for bead state across restarts
- No atomic operations for concurrent updates

## Value Proposition

- **Centralized Management**: Single source of truth for bead state
- **Query Interface**: IPC workers can query beads by status, labels, ID
- **Persistence**: Survives orchestrator restarts
- **Concurrency**: Safe atomic operations for multi-agent scenarios
- **Performance**: Indexed lookups by ID, status, labels

## Core Requirements

### 1. Storage Backend
- JSON file-based storage (`.oya/beads.jsonl` exists)
- Atomic write operations (write-to-temp + rename)
- Read-only memory cache for queries
- Periodic persistence interval

### 2. Query API
```rust
pub struct BeadStore {
    // Storage backend + in-memory cache
}

impl BeadStore {
    // Query operations
    pub async fn get_bead(&self, id: &BeadId) -> Result<Option<BeadRecord>, StoreError>
    pub async fn list_beads(&self) -> Result<Vec<BeadRecord>, StoreError>
    pub async fn filter_by_status(&self, status: BeadStatus) -> Result<Vec<BeadRecord>, StoreError>
    pub async fn filter_by_labels(&self, labels: &[String]) -> Result<Vec<BeadRecord>, StoreError>

    // Mutation operations
    pub async fn update_bead(&self, bead: BeadRecord) -> Result<(), StoreError>
    pub async fn insert_bead(&self, bead: BeadRecord) -> Result<(), StoreError>

    // Lifecycle
    pub async fn load(&self) -> Result<(), StoreError>
    pub async fn save(&self) -> Result<(), StoreError>
}
```

### 3. Data Structures
```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeadRecord {
    pub id: BeadId,
    pub title: String,
    pub description: String,
    pub status: BeadStatus,
    pub labels: Vec<String>,
    pub priority: u8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BeadStatus {
    Open,
    InProgress,
    Closed,
}
```

### 4. Error Handling (thiserror)
```rust
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("bead not found: {0}")]
    NotFound(BeadId),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("lock poisoned")]
    LockPoisoned,
}
```

## Functional Core Design

### Pure Core (Sync)
```rust
/// In-memory bead store - pure and immutable operations
pub struct BeadStoreCore {
    beads: im::HashMap<BeadId, BeadRecord>,
    by_status: im::HashMap<BeadStatus, im::HashSet<BeadId>>,
    by_label: im::HashMap<String, im::HashSet<BeadId>>,
}

impl BeadStoreCore {
    // Pure query operations (no I/O)
    pub fn get_bead(&self, id: &BeadId) -> Option<&BeadRecord>
    pub fn list_beads(&self) -> Vec<&BeadRecord>
    pub fn filter_by_status(&self, status: BeadStatus) -> Vec<&BeadRecord>
    pub fn filter_by_labels(&self, labels: &[String]) -> Vec<&BeadRecord>

    // Pure state transitions (return new state)
    pub fn with_bead(&self, bead: BeadRecord) -> Self
    pub fn without_bead(&self, id: &BeadId) -> Self
}
```

### Imperative Shell (Async)
```rust
/// BeadStore with async I/O and persistence
pub struct BeadStore {
    core: Arc<RwLock<BeadStoreCore>>,
    storage_path: PathBuf,
}

impl BeadStore {
    pub async fn new(storage_path: PathBuf) -> Result<Self, StoreError> {
        // Load from disk, return initialized store
    }

    pub async fn load(&self) -> Result<(), StoreError> {
        // Read .oya/beads.jsonl, update core
    }

    pub async fn save(&self) -> Result<(), StoreError> {
        // Serialize core to temp file, atomic rename
    }

    // Delegate queries to core
    pub async fn get_bead(&self, id: &BeadId) -> Result<Option<BeadRecord>, StoreError> {
        let core = self.core.read().await;
        Ok(core.get_bead(id).cloned())
    }
}
```

## Implementation Plan

### Phase 0-2: Setup
- Create `crates/bead-store/Cargo.toml` with dependencies
  - `im = "15.1"` (persistent data structures)
  - `thiserror = "2.0"`
  - `anyhow = "1.0"`
  - `serde = { version = "1.0", features = ["derive"] }`
  - `serde_json = "1.0"`
  - `tokio = { version = "1", features = ["fs", "io-util", "sync"] }`
  - `chrono = { version = "0.4", features = ["serde"] }`
- Create `src/lib.rs` with file header lints
- Create `src/error.rs` for StoreError
- Create `src/types.rs` for BeadRecord, BeadStatus

### Phase 3-6: Core Data Structures
- Implement `BeadRecord` with Serialize/Deserialize
- Implement `BeadStatus` enum
- Implement `BeadStoreCore` with pure operations
- Add unit tests for core (no unwrap, functional patterns)

### Phase 7-10: Persistence Layer
- Implement `BeadStore::new()` to load existing beads
- Implement `BeadStore::load()` to read `.oya/beads.jsonl`
- Implement `BeadStore::save()` atomic write
- Add error recovery (corrupted file handling)

### Phase 11-13: Query API
- Implement `get_bead()`, `list_beads()`
- Implement `filter_by_status()`, `filter_by_labels()`
- Implement `update_bead()`, `insert_bead()`
- Add comprehensive integration tests

### Phase 14: Integration
- Add periodic auto-save background task
- Expose `BeadStore` as dependency injection target
- Document API with examples

## Quality Gates

- **Zero Panic**: No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`
- **Functional Core**: Pure `BeadStoreCore` with immutable operations
- **Error Handling**: All operations return `Result<T, StoreError>`
- **Test Coverage**: Unit tests for all core functions
- **Documentation**: All public APIs documented with examples

## Testing Strategy

### Unit Tests (Core)
```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_core_get_bead() {
        let core = BeadStoreCore::default();
        let bead = BeadRecord::test_fixture();
        let core = core.with_bead(bead.clone());
        assert_eq!(core.get_bead(&bead.id), Some(&bead));
    }
}
```

### Integration Tests (Persistence)
```rust
#[tokio::test]
async fn test_save_and_load() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("beads.jsonl");
    let store = BeadStore::new(path.clone()).await.unwrap();

    // Insert bead, save
    let bead = BeadRecord::test_fixture();
    store.insert_bead(bead.clone()).await.unwrap();
    store.save().await.unwrap();

    // Load new store instance
    let store2 = BeadStore::new(path).await.unwrap();
    let loaded = store2.get_bead(&bead.id).await.unwrap();
    assert_eq!(loaded, Some(bead));
}
```

## Dependencies

Required dependencies for `Cargo.toml`:
```toml
[dependencies]
im = "15.1"              # Persistent data structures
thiserror = "2.0"        # Domain errors
anyhow = "1.0"           # Boundary errors
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["fs", "io-util", "sync", "rt"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"

[dev-dependencies]
tempfile = "3.0"
proptest = "1.0"
```

## Success Criteria

1. ✅ BeadStore crate created with functional core/imperative shell architecture
2. ✅ Loads existing beads from `.oya/beads.jsonl`
3. ✅ Provides query API: `get_bead()`, `list_beads()`, `filter_by_status()`, `filter_by_labels()`
4. ✅ Supports atomic updates: `update_bead()`, `insert_bead()`
5. ✅ Persists changes to disk with atomic write
6. ✅ All code uses `Result<T, Error>` - zero panic
7. ✅ Unit and integration tests pass
8. ✅ `moon run :ci` passes (fmt, clippy, test, build)

## References

- `/home/lewis/src/oya/crates/orchestrator/src/actors/scheduler.rs` - Needs BeadStore integration
- `/home/lewis/src/oya/.beads/issues.jsonl` - Bead data source
- Project CLAUDE.md - Functional Rust patterns and Core 6 libraries

## Notes

- Use `im` crate for persistent HashMap (not `rpds`) - better for async sharing via Arc
- Core operations are pure and synchronous (no async in core)
- Shell handles all I/O and async operations
- File writes use atomic pattern: write to temp + rename
- Auto-save every 30 seconds (background task)
- Read-heavy workload: keep cache in memory, only write on mutation
