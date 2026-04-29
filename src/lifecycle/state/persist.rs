#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! High-level state persistence helpers.
//!
//! These functions enforce the evidence-before-action invariant: every
//! agent action is recorded in the journal *before* it is dispatched.

use crate::lifecycle::effects::EffectJournalEntry;

use crate::lifecycle::state::StateDb;
use crate::lifecycle::state::StateDbError;
use crate::lifecycle::types::{BeadData, BeadId, LifecycleState, Phase};

// ─── Public API ───────────────────────────────────────────────────────────────

/// Persists the current lifecycle state and journal entries atomically.
///
/// All journal entries accumulated since the last persist are written in a
/// single batch, then flushed to disk. This guarantees crash-recoverable
/// evidence for every agent action.
///
/// # Errors
/// Returns `StateDbError::NoBeadId` if the current phase does not contain
/// a bead ID (only happens for malformed/initial states).
pub fn persist_state(
    db: &StateDb,
    state: &LifecycleState,
    journal: &[EffectJournalEntry],
) -> crate::lifecycle::state::Result<()> {
    let bead_id = state.phase.bead_data().ok_or(StateDbError::NoBeadId)?.bead_id.clone();
    let state_json = serde_json::to_string(state)?;
    let journal_entries: Vec<(String, String)> = journal
        .iter()
        .map(|entry| {
            let key = db.next_journal_key(&bead_id);
            let value = serde_json::to_string(entry)?;
            Ok((key, value))
        })
        .collect::<crate::lifecycle::state::Result<Vec<_>>>()?;
    db.batch_persist_state(&bead_id, &state_json, &journal_entries)?;
    db.flush()?;
    Ok(())
}

/// Loads the persisted lifecycle state and all journal entries for a bead.
///
/// Journal entries that fail to deserialize are logged and skipped rather
/// than propagating an error. This prevents a single corrupted entry from
/// blocking recovery of the full run record.
///
/// # Errors
/// Returns a serialization error if the workflow state itself is corrupt.
pub fn load_state(
    db: &StateDb,
    bead_id: &BeadId,
) -> crate::lifecycle::state::Result<Option<(LifecycleState, Vec<EffectJournalEntry>)>> {
    let state_json: Option<String> = db.load_workflow(bead_id)?;
    let journal_raw: Vec<String> = db.load_journal(bead_id)?;
    match state_json {
        Some(json) => {
            let state: LifecycleState = serde_json::from_str(&json)?;
            let journal: Vec<EffectJournalEntry> = journal_raw
                .iter()
                .filter_map(|j| match serde_json::from_str(j) {
                    Ok(entry) => Some(entry),
                    Err(e) => {
                        tracing::warn!(
                            bead_id = bead_id.as_str(),
                            error = %e,
                            "corrupted journal entry skipped"
                        );
                        None
                    }
                })
                .collect();
            Ok(Some((state, journal)))
        }
        None => Ok(None),
    }
}

// ─── Phase bead_data helper ──────────────────────────────────────────────────

impl Phase {
    /// Returns the `BeadData` inside this phase variant, if present.
    ///
    /// All active and terminal phases contain a `BeadData`. The `Completed`
    /// phase also contains one via `LifecycleResult`. Returns `None` only if
    /// a future variant is added that lacks bead data.
    #[must_use]
    pub fn bead_data(&self) -> Option<BeadData> {
        match self {
            Self::Planned(bead)
            | Self::WorkspaceReady(bead)
            | Self::PrOpen { bead, .. }
            | Self::Failed { bead, .. } => Some(bead.clone()),
            Self::Completed(result) => Some(result.bead.clone()),
        }
    }
}
