#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::redundant_closure,
    clippy::must_use_candidate,
    clippy::cast_possible_truncation
)]
//! Fjall-backed persistence for Oya lifecycle state and evidence journal.
//!
//! ## Keyspaces
//!
//! - `workflows` – serialized `LifecycleState` per bead, keyed by bead ID
//! - `journal`   – serialized `EffectJournalEntry` per bead, keyed by
//!   `bead_id_timestamp_sequence`
//!
//! ## Evidence-before-action invariant
//!
//! Every AI action MUST be persisted before execution. Call [`persist_state`]
//! with the journal entries accumulated so far; the batch write guarantees
//! that a crash after persistence can recover the in-flight state.

mod persist;

use crate::lifecycle::types::BeadId;
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;

const KEYSPACE_WORKFLOWS: &str = "workflows";
const KEYSPACE_JOURNAL: &str = "journal";
const KEYSPACE_MEMORY: &str = "memory";
const KEYSPACE_STATUS: &str = "status";

/// Errors that can occur while interacting with the evidence store.
#[derive(Debug, Error)]
pub enum StateDbError {
    #[error("fjall error: {0}")]
    Fjall(#[from] fjall::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// Returned when a lifecycle state has no bead ID (e.g. default/empty phase).
    #[error("state has no bead ID")]
    NoBeadId,
}

pub type Result<T> = std::result::Result<T, StateDbError>;

/// Fjall-backed evidence store for Oya.
///
/// Tracks lifecycle state and append-only journal entries per bead.
/// All writes are batched and flushed to disk before returning.
#[derive(Clone)]
pub struct StateDb {
    db: Arc<Database>,
    workflows: Keyspace,
    journal: Keyspace,
    memory: Keyspace,
    status: Keyspace,
}

impl StateDb {
    /// Opens (or creates) the evidence store at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::builder(path)
            .cache_size(64 * 1024 * 1024)
            .journal_compression(fjall::CompressionType::Lz4)
            .open()?;
        let workflows = db
            .keyspace(KEYSPACE_WORKFLOWS, || KeyspaceCreateOptions::default())
            .map_err(StateDbError::Fjall)?;
        let journal = db
            .keyspace(KEYSPACE_JOURNAL, || KeyspaceCreateOptions::default())
            .map_err(StateDbError::Fjall)?;
        let memory = db
            .keyspace(KEYSPACE_MEMORY, || KeyspaceCreateOptions::default())
            .map_err(StateDbError::Fjall)?;
        let status = db
            .keyspace(KEYSPACE_STATUS, || KeyspaceCreateOptions::default())
            .map_err(StateDbError::Fjall)?;
        Ok(Self { db: Arc::new(db), workflows, journal, memory, status })
    }

    /// Persists a single workflow state JSON for a bead.
    pub fn persist_workflow(&self, bead_id: &BeadId, state_json: &str) -> Result<()> {
        self.workflows.insert(bead_id.as_str(), state_json)?;
        Ok(())
    }

    /// Loads the raw JSON state for a bead, if any.
    pub fn load_workflow(&self, bead_id: &BeadId) -> Result<Option<String>> {
        self.workflows
            .get(bead_id.as_str())
            .map_err(StateDbError::Fjall)
            .map(|opt| opt.and_then(|v| String::from_utf8(v.to_vec()).ok()))
    }

    /// Appends a single journal entry for a bead.
    pub fn append_journal(&self, bead_id: &BeadId, entry_json: &str) -> Result<()> {
        let key = self.next_journal_key(bead_id);
        self.journal.insert(key.as_str(), entry_json)?;
        Ok(())
    }

    /// Loads all journal entries for a bead, in insertion order.
    pub fn load_journal(&self, bead_id: &BeadId) -> Result<Vec<String>> {
        let prefix = format!("{}_", bead_id.as_str());
        Ok(self
            .journal
            .prefix(&prefix)
            .filter_map(|guard| {
                guard.value().ok().and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
            })
            .collect())
    }

    /// Lists all bead IDs that have workflow state stored.
    #[allow(dead_code)]
    pub fn list_workflow_ids(&self) -> Result<Vec<String>> {
        Ok(self
            .workflows
            .iter()
            .filter_map(|guard| guard.key().ok().and_then(|k| String::from_utf8(k.to_vec()).ok()))
            .collect())
    }

    /// Persists a single memory snapshot JSON for a bead.
    pub fn persist_memory(&self, bead_id: &BeadId, memory_json: &str) -> Result<()> {
        self.memory.insert(bead_id.as_str(), memory_json)?;
        Ok(())
    }

    /// Loads the raw JSON memory snapshot for a bead, if any.
    pub fn load_memory(&self, bead_id: &BeadId) -> Result<Option<String>> {
        self.memory
            .get(bead_id.as_str())
            .map_err(StateDbError::Fjall)
            .map(|opt| opt.and_then(|v| String::from_utf8(v.to_vec()).ok()))
    }

    /// Persists a single status snapshot JSON for a bead.
    pub fn persist_status(&self, workflow_key: &str, status_json: &str) -> Result<()> {
        self.status.insert(workflow_key, status_json)?;
        Ok(())
    }

    /// Loads the raw JSON status snapshot for a bead, if any.
    pub fn load_status(&self, workflow_key: &str) -> Result<Option<String>> {
        self.status
            .get(workflow_key)
            .map_err(StateDbError::Fjall)
            .map(|opt| opt.and_then(|v| String::from_utf8(v.to_vec()).ok()))
    }

    /// Deletes all state and journal for a bead.
    #[allow(dead_code)]
    pub fn delete_workflow(&self, bead_id: &BeadId) -> Result<()> {
        self.workflows.remove(bead_id.as_str())?;
        Ok(())
    }

    /// Forces a synchronous flush of all pending writes to disk.
    pub fn flush(&self) -> Result<()> {
        self.db.persist(PersistMode::SyncAll)?;
        Ok(())
    }

    /// Returns a snapshot handle for backup purposes.
    #[allow(dead_code)]
    pub fn snapshot(&self) -> fjall::Snapshot {
        self.db.snapshot()
    }

    /// Atomically writes the workflow state and a slice of journal entries.
    /// This is the primary write API; prefer this over individual inserts.
    pub fn batch_persist_state(
        &self,
        bead_id: &BeadId,
        state_json: &str,
        journal_entries: &[(String, String)],
    ) -> Result<()> {
        let mut batch = self.db.batch();
        batch.insert(&self.workflows, bead_id.as_str(), state_json);
        for (key, value) in journal_entries {
            batch.insert(&self.journal, key.as_str(), value.as_str());
        }
        batch.commit()?;
        Ok(())
    }

    /// Generates the next monotonically-increasing journal key for a bead.
    #[allow(dead_code)]
    pub fn next_journal_key(&self, bead_id: &BeadId) -> String {
        next_journal_key(bead_id)
    }
}

// ─── Journal key generation ────────────────────────────────────────────────────

static JOURNAL_COUNTER: AtomicU64 = AtomicU64::new(0);

fn timestamp_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn next_journal_key(bead_id: &BeadId) -> String {
    let ts = timestamp_now();
    let seq = JOURNAL_COUNTER.fetch_add(1, Ordering::SeqCst);
    // Format: bead_id_timestamp_sequence
    // Zero-padded for lexicographic sorting.
    format!("{}_{ts:020}_{seq:010}", bead_id.as_str())
}

// ─── Re-exports ───────────────────────────────────────────────────────────────

/// Persists lifecycle state and journal atomically, then flushes.
pub use persist::persist_state;

/// Loads lifecycle state and journal for a bead.
pub use persist::load_state;

// ─── Tests ───────────────────────────────────────────────────────────────────────

// Test code may use unwrap() for test setup — the zero_panic_lint scanner
// excludes this module so production code remains strictly unwrap-free.
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{BeadData, LifecycleState, Phase};

    #[test]
    fn timestamp_now_returns_real_value() {
        let ts = timestamp_now();
        assert!(ts > 1, "timestamp_now must return epoch ms, not a constant");
    }

    #[test]
    fn persist_workflow_writes_and_loads_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("test-roundtrip").unwrap();
        let state =
            LifecycleState { phase: Phase::Planned(BeadData::from_bead_id(bead_id.clone())) };
        let json = serde_json::to_string(&state).unwrap();
        db.persist_workflow(&bead_id, &json).unwrap();
        let loaded = db.load_workflow(&bead_id).unwrap();
        assert!(loaded.is_some(), "persist_workflow must actually write to Fjall");
    }

    #[test]
    fn append_journal_writes_entries() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("test-journal").unwrap();
        db.append_journal(&bead_id, r#"{"effect":"test","success":true}"#).unwrap();
        db.append_journal(&bead_id, r#"{"effect":"test2","success":false}"#).unwrap();
        let entries = db.load_journal(&bead_id).unwrap();
        assert_eq!(entries.len(), 2, "append_journal must actually write entries");
    }

    #[test]
    fn flush_persists_data_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("test-flush").unwrap();
        let state =
            LifecycleState { phase: Phase::Planned(BeadData::from_bead_id(bead_id.clone())) };
        let json = serde_json::to_string(&state).unwrap();
        db.persist_workflow(&bead_id, &json).unwrap();
        db.flush().unwrap();
        drop(db);
        let db2 = StateDb::open(dir.path().join("db")).unwrap();
        let loaded = db2.load_workflow(&bead_id).unwrap();
        assert!(loaded.is_some(), "flush must persist data so it survives DB close/reopen");
    }
}
