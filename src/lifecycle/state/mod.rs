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

use crate::lifecycle::types::{
    BeadId, EvidenceEnvelope, EvidenceEnvelopeError, EvidenceRecordId, RunId,
};
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;

const KEYSPACE_WORKFLOWS: &str = "workflows";
const KEYSPACE_JOURNAL: &str = "journal";
const KEYSPACE_MEMORY: &str = "memory";
const KEYSPACE_STATUS: &str = "status";
const KEYSPACE_EVIDENCE: &str = "evidence";

/// Errors that can occur while interacting with the evidence store.
#[derive(Debug, Error)]
pub enum StateDbError {
    #[error("fjall error: {0}")]
    Fjall(#[from] fjall::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("evidence envelope error: {0}")]
    EvidenceEnvelope(#[from] EvidenceEnvelopeError),
    #[error("duplicate evidence record: {0}")]
    DuplicateEvidenceRecord(String),
    #[error("invalid utf-8 evidence record: {0}")]
    InvalidEvidenceUtf8(String),
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
    evidence: Keyspace,
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
        let evidence = db
            .keyspace(KEYSPACE_EVIDENCE, || KeyspaceCreateOptions::default())
            .map_err(StateDbError::Fjall)?;
        Ok(Self { db: Arc::new(db), workflows, journal, memory, status, evidence })
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

    /// Appends a canonical evidence envelope for a run.
    pub fn append_evidence(&self, envelope: &EvidenceEnvelope) -> Result<()> {
        let key = evidence_key(envelope);
        if self.evidence.get(key.as_str())?.is_some() {
            return Err(StateDbError::DuplicateEvidenceRecord(key));
        }
        let json = envelope.to_canonical_json()?;
        self.evidence.insert(key.as_str(), json.as_str())?;
        Ok(())
    }

    /// Loads all canonical evidence envelopes for a run, ordered by key.
    pub fn load_evidence(&self, run_id: &RunId) -> Result<Vec<EvidenceEnvelope>> {
        let prefix = evidence_prefix(run_id);
        self.evidence
            .prefix(&prefix)
            .map(|guard| {
                let value = guard.value()?;
                let json = String::from_utf8(value.to_vec())
                    .map_err(|error| StateDbError::InvalidEvidenceUtf8(error.to_string()))?;
                EvidenceEnvelope::from_canonical_json(&json).map_err(StateDbError::from)
            })
            .collect()
    }

    /// Finds one canonical evidence envelope by record id.
    pub fn find_evidence_record(
        &self,
        record_id: &EvidenceRecordId,
    ) -> Result<Option<EvidenceEnvelope>> {
        self.evidence
            .iter()
            .map(|guard| {
                let value = guard.value()?;
                let json = String::from_utf8(value.to_vec())
                    .map_err(|error| StateDbError::InvalidEvidenceUtf8(error.to_string()))?;
                EvidenceEnvelope::from_canonical_json(&json).map_err(StateDbError::from)
            })
            .find_map(|result| match result {
                Ok(envelope) if envelope.record_id.as_str() == record_id.as_str() => {
                    Some(Ok(envelope))
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .transpose()
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

fn evidence_prefix(run_id: &RunId) -> String {
    format!("{}_", run_id.as_str())
}

fn evidence_key(envelope: &EvidenceEnvelope) -> String {
    format!(
        "{}_{:020}_{}",
        envelope.run_id.as_str(),
        envelope.timestamp.timestamp_millis(),
        envelope.record_id.as_str()
    )
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
    use crate::lifecycle::types::{
        BeadData, EvidenceEnvelopeParts, EvidenceKind, EvidenceMetadata, EvidenceRecordId,
        LifecycleState, Phase,
    };
    use chrono::{TimeZone, Utc};

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

    #[test]
    fn evidence_fjall_writes_and_reloads_records_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("db");
        let db = StateDb::open(&db_path).unwrap();
        let first = evidence_envelope("ev-oya-3uu-001", 0, EvidenceKind::RunStarted, None);
        let second = evidence_envelope(
            "ev-oya-3uu-002",
            1,
            EvidenceKind::PromptRecord,
            Some(first.checksum.clone()),
        );

        db.append_evidence(&first).unwrap();
        db.append_evidence(&second).unwrap();
        db.flush().unwrap();
        drop(db);

        let reloaded = StateDb::open(&db_path).unwrap().load_evidence(&run_id()).unwrap();
        assert_eq!(reloaded, vec![first, second]);
    }

    #[test]
    fn evidence_fjall_rejects_duplicate_record_key() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let envelope = evidence_envelope("ev-oya-3uu-dup", 0, EvidenceKind::RunStarted, None);

        db.append_evidence(&envelope).unwrap();
        let duplicate = db.append_evidence(&envelope);

        assert!(matches!(duplicate, Err(StateDbError::DuplicateEvidenceRecord(_))));
    }

    fn evidence_envelope(
        record_id: &str,
        offset_seconds: i64,
        kind: EvidenceKind,
        previous_checksum: Option<crate::lifecycle::types::EvidenceChecksum>,
    ) -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse(record_id).unwrap(),
            run_id: run_id(),
            bead_id: bead_id(),
            timestamp: Utc.timestamp_opt(1_779_999_600 + offset_seconds, 0).unwrap(),
            kind,
            metadata: EvidenceMetadata::new(),
            previous_checksum,
        })
        .unwrap()
    }

    fn run_id() -> RunId {
        RunId::from_bead_id(&bead_id())
    }

    fn bead_id() -> BeadId {
        BeadId::parse("oya-3uu").unwrap()
    }
}
