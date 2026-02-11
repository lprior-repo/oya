//! Storage traits for workflow persistence.

use async_trait::async_trait;
use itertools::Itertools;

use crate::error::Result;
use crate::types::{Checkpoint, Journal, JournalEntry, PhaseId, Workflow, WorkflowId};

/// Trait for workflow storage backends.
#[async_trait]
pub trait WorkflowStorage: Send + Sync {
    /// Save a workflow.
    async fn save_workflow(&self, workflow: &Workflow) -> Result<()>;

    /// Load a workflow by ID.
    async fn load_workflow(&self, id: WorkflowId) -> Result<Option<Workflow>>;

    /// Delete a workflow.
    async fn delete_workflow(&self, id: WorkflowId) -> Result<()>;

    /// List all workflows.
    async fn list_workflows(&self) -> Result<Vec<Workflow>>;

    /// Save a checkpoint.
    async fn save_checkpoint(&self, workflow_id: WorkflowId, checkpoint: &Checkpoint)
        -> Result<()>;

    /// Load checkpoints for a workflow.
    async fn load_checkpoints(&self, workflow_id: WorkflowId) -> Result<Vec<Checkpoint>>;

    /// Load a specific checkpoint by phase ID.
    async fn load_checkpoint(
        &self,
        workflow_id: WorkflowId,
        phase_id: PhaseId,
    ) -> Result<Option<Checkpoint>>;

    /// Append a journal entry.
    async fn append_journal(&self, workflow_id: WorkflowId, entry: JournalEntry) -> Result<()>;

    /// Load the journal for a workflow.
    async fn load_journal(&self, workflow_id: WorkflowId) -> Result<Journal>;

    /// Clear checkpoints after a phase (for rewind).
    async fn clear_checkpoints_after(
        &self,
        workflow_id: WorkflowId,
        phase_id: PhaseId,
    ) -> Result<()>;
}

/// In-memory storage implementation for testing.
#[derive(Default)]
pub struct InMemoryStorage {
    workflows: tokio::sync::RwLock<std::collections::HashMap<WorkflowId, Workflow>>,
    checkpoints: tokio::sync::RwLock<std::collections::HashMap<WorkflowId, Vec<Checkpoint>>>,
    journals: tokio::sync::RwLock<std::collections::HashMap<WorkflowId, Journal>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl WorkflowStorage for InMemoryStorage {
    async fn save_workflow(&self, workflow: &Workflow) -> Result<()> {
        self.workflows
            .write()
            .await
            .insert(workflow.id, workflow.clone());
        Ok(())
    }

    async fn load_workflow(&self, id: WorkflowId) -> Result<Option<Workflow>> {
        Ok(self.workflows.read().await.get(&id).cloned())
    }

    async fn delete_workflow(&self, id: WorkflowId) -> Result<()> {
        self.workflows.write().await.remove(&id);
        self.checkpoints.write().await.remove(&id);
        self.journals.write().await.remove(&id);
        Ok(())
    }

    async fn list_workflows(&self) -> Result<Vec<Workflow>> {
        Ok(self.workflows.read().await.values().cloned().collect_vec())
    }

    async fn save_checkpoint(
        &self,
        workflow_id: WorkflowId,
        checkpoint: &Checkpoint,
    ) -> Result<()> {
        let mut checkpoints = self.checkpoints.write().await;
        let entries = checkpoints.entry(workflow_id).or_default();
        if let Some(pos) = entries
            .iter()
            .position(|c| c.phase_id == checkpoint.phase_id)
        {
            entries[pos] = checkpoint.clone();
        } else {
            entries.push(checkpoint.clone());
        }
        Ok(())
    }

    async fn load_checkpoints(&self, workflow_id: WorkflowId) -> Result<Vec<Checkpoint>> {
        Ok(self
            .checkpoints
            .read()
            .await
            .get(&workflow_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn load_checkpoint(
        &self,
        workflow_id: WorkflowId,
        phase_id: PhaseId,
    ) -> Result<Option<Checkpoint>> {
        Ok(self
            .checkpoints
            .read()
            .await
            .get(&workflow_id)
            .and_then(|cps| cps.iter().rev().find(|c| c.phase_id == phase_id).cloned()))
    }

    async fn append_journal(&self, workflow_id: WorkflowId, entry: JournalEntry) -> Result<()> {
        self.journals
            .write()
            .await
            .entry(workflow_id)
            .or_default()
            .append(entry);
        Ok(())
    }

    async fn load_journal(&self, workflow_id: WorkflowId) -> Result<Journal> {
        Ok(self
            .journals
            .read()
            .await
            .get(&workflow_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn clear_checkpoints_after(
        &self,
        workflow_id: WorkflowId,
        phase_id: PhaseId,
    ) -> Result<()> {
        if let Some(checkpoints) = self.checkpoints.write().await.get_mut(&workflow_id) {
            // Find the index of the checkpoint and remove all after it
            if let Some(pos) = checkpoints.iter().position(|c| c.phase_id == phase_id) {
                checkpoints.truncate(pos + 1);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Phase;

    #[tokio::test]
    async fn test_save_and_load_workflow() {
        let storage = InMemoryStorage::new();
        let workflow = Workflow::new("test").add_phase(Phase::new("build"));

        storage.save_workflow(&workflow).await.ok();

        let loaded = storage.load_workflow(workflow.id).await;
        assert!(loaded.is_ok());
        let loaded = loaded.ok().flatten();
        assert!(loaded.is_some());
        assert_eq!(loaded.map(|w| w.name), Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_save_checkpoint() {
        let storage = InMemoryStorage::new();
        let workflow_id = WorkflowId::new();
        let phase_id = PhaseId::new();
        let checkpoint = Checkpoint::new(phase_id, vec![1, 2, 3], vec![]);

        storage.save_checkpoint(workflow_id, &checkpoint).await.ok();

        let loaded = storage.load_checkpoint(workflow_id, phase_id).await;
        assert!(loaded.is_ok());
        assert!(loaded.ok().flatten().is_some());
    }

    #[tokio::test]
    async fn test_journal_append() {
        let storage = InMemoryStorage::new();
        let workflow_id = WorkflowId::new();
        let phase_id = PhaseId::new();

        storage
            .append_journal(workflow_id, JournalEntry::phase_started(phase_id, "build"))
            .await
            .ok();
        storage
            .append_journal(
                workflow_id,
                JournalEntry::phase_completed(phase_id, "build", vec![]),
            )
            .await
            .ok();

        let journal = storage.load_journal(workflow_id).await;
        assert!(journal.is_ok());
        assert_eq!(journal.map(|j| j.len()).unwrap_or(0), 2);
    }
}
