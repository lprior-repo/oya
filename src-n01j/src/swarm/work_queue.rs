//! Work queue for managing bead distribution to agents.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, mpsc};
use tracing::{debug, info, warn};

use crate::swarm::error::{BeadWorkState, SwarmError, SwarmResult};

/// Work item for a bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadWorkItem {
    /// Bead identifier.
    pub bead_id: String,

    /// Current state of the bead.
    pub state: BeadWorkState,

    /// Agent currently assigned to this bead.
    pub assigned_agent: Option<String>,

    /// Workspace where work is happening.
    pub workspace: Option<String>,

    /// Investigation results from bv triage.
    pub investigation: Option<serde_json::Value>,

    /// Test results from moon ci.
    pub test_results: Option<serde_json::Value>,

    /// Commit hash when bead landed.
    pub commit_hash: Option<String>,

    /// Number of retries attempted.
    pub retry_count: usize,

    /// Timestamp when bead was added.
    pub created_at: u64,

    /// Timestamp when bead was last updated.
    pub updated_at: u64,
}

impl BeadWorkItem {
    /// Create a new work item.
    #[must_use]
    pub fn new(bead_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            bead_id,
            state: BeadWorkState::Pending,
            assigned_agent: None,
            workspace: None,
            investigation: None,
            test_results: None,
            commit_hash: None,
            retry_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Transition to a new state.
    ///
    /// # Errors
    ///
    /// Returns error if transition is invalid.
    pub fn transition_to(&mut self, new_state: BeadWorkState) -> SwarmResult<()> {
        // Validate state transition
        let from = self.state;

        match (from, new_state) {
            (BeadWorkState::Pending, BeadWorkState::Claimed) => {}
            (BeadWorkState::Claimed, BeadWorkState::ContractReady) => {}
            (BeadWorkState::ContractReady, BeadWorkState::Implementing) => {}
            (BeadWorkState::Implementing, BeadWorkState::ImplementationComplete) => {}
            (BeadWorkState::ImplementationComplete, BeadWorkState::Reviewing) => {}
            (BeadWorkState::Reviewing, BeadWorkState::Landed) => {}
            (BeadWorkState::Failed, BeadWorkState::Claimed) => {
                // Allow retry
                self.retry_count = self.retry_count.saturating_add(1);
            }
            (_, to) => {
                return Err(SwarmError::InvalidStateTransition {
                    bead_id: self.bead_id.clone(),
                    from,
                    to,
                });
            }
        }

        self.state = new_state;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        debug!(
            bead_id = %self.bead_id,
            from = %from,
            to = %new_state,
            "Transitioned bead state"
        );

        Ok(())
    }

    /// Assign to an agent.
    pub fn assign_to(&mut self, agent_id: String) {
        self.assigned_agent = Some(agent_id);
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Set workspace.
    pub fn set_workspace(&mut self, workspace: String) {
        self.workspace = Some(workspace);
    }

    /// Set test results.
    pub fn set_test_results(&mut self, results: serde_json::Value) {
        self.test_results = Some(results);
    }

    /// Set commit hash.
    pub fn set_commit_hash(&mut self, hash: String) {
        self.commit_hash = Some(hash);
    }
}

/// Work queue for managing beads.
#[derive(Clone)]
pub struct WorkQueue {
    /// Inner state protected by RwLock.
    inner: Arc<RwLock<WorkQueueInner>>,

    /// Channel for claiming beads.
    claim_tx: mpsc::Sender<String>,
}

/// Inner state of work queue.
struct WorkQueueInner {
    /// All beads in the queue.
    beads: HashMap<String, BeadWorkItem>,

    /// Semaphore for limiting concurrent claims.
    claim_semaphore: Arc<Semaphore>,
}

impl WorkQueue {
    /// Create a new work queue.
    #[must_use]
    pub fn new() -> Self {
        let (claim_tx, mut claim_rx) = mpsc::channel::<String>(100);

        let inner = Arc::new(RwLock::new(WorkQueueInner {
            beads: HashMap::new(),
            claim_semaphore: Arc::new(Semaphore::new(1)),
        }));

        // Spawn claim notification handler
        let inner_clone = inner.clone();
        tokio::spawn(async move {
            while let Some(bead_id) = claim_rx.recv().await {
                let mut inner = inner_clone.write().await;
                if let Some(bead) = inner.beads.get_mut(&bead_id) {
                    bead.transition_to(BeadWorkState::Claimed).ok();
                }
            }
        });

        Self { inner, claim_tx }
    }

    /// Add a bead to the queue.
    ///
    /// # Errors
    ///
    /// Returns error if bead already exists.
    pub async fn add_bead(&self, bead_id: String) -> SwarmResult<()> {
        let mut inner = self.inner.write().await;

        if inner.beads.contains_key(&bead_id) {
            return Err(SwarmError::BeadNotFound { bead_id });
        }

        let bead = BeadWorkItem::new(bead_id.clone());
        inner.beads.insert(bead_id.clone(), bead);

        info!(bead_id = %bead_id, "Added bead to work queue");

        Ok(())
    }

    /// Claim the next available bead.
    ///
    /// # Errors
    ///
    /// Returns error if no beads available.
    pub async fn claim_next(&self, agent_id: String) -> SwarmResult<BeadWorkItem> {
        // Acquire semaphore permit - we need to scope this properly
        let semaphore = {
            let inner = self.inner.read().await;
            inner.claim_semaphore.clone()
        };

        let _permit = semaphore.acquire().await.map_err(|e| SwarmError::IoError {
            operation: "acquire_semaphore".to_string(),
            reason: e.to_string(),
        })?;

        let mut inner = self.inner.write().await;

        // Find first pending bead
        let bead_id = inner
            .beads
            .iter()
            .find(|(_, bead)| bead.state == BeadWorkState::Pending)
            .map(|(id, _)| id.clone());

        let bead_id = match bead_id {
            Some(id) => id,
            None => {
                // Check for failed beads that can be retried
                let failed_id = inner
                    .beads
                    .iter()
                    .find(|(_, bead)| bead.state == BeadWorkState::Failed && bead.retry_count < 3)
                    .map(|(id, _)| id.clone());

                match failed_id {
                    Some(id) => id,
                    None => {
                        return Err(SwarmError::BeadNotFound {
                            bead_id: "none".to_string(),
                        });
                    }
                }
            }
        };

        let bead = inner
            .beads
            .get(&bead_id)
            .cloned()
            .ok_or_else(|| SwarmError::BeadNotFound {
                bead_id: bead_id.clone(),
            })?;

        // Mark as claimed
        let _ = self.claim_tx.send(bead_id.clone()).await;

        info!(
            bead_id = %bead_id,
            agent_id = %agent_id,
            "Agent claimed bead"
        );

        Ok(bead)
    }

    /// Update bead state.
    ///
    /// # Errors
    ///
    /// Returns error if bead not found or transition invalid.
    pub async fn update_state(&self, bead_id: &str, new_state: BeadWorkState) -> SwarmResult<()> {
        let mut inner = self.inner.write().await;

        let bead = inner
            .beads
            .get_mut(bead_id)
            .ok_or_else(|| SwarmError::BeadNotFound {
                bead_id: bead_id.to_string(),
            })?;

        bead.transition_to(new_state)?;

        info!(
            bead_id = %bead_id,
            state = %new_state,
            "Updated bead state"
        );

        Ok(())
    }

    /// Get bead by ID.
    ///
    /// # Errors
    ///
    /// Returns error if bead not found.
    pub async fn get_bead(&self, bead_id: &str) -> SwarmResult<BeadWorkItem> {
        let inner = self.inner.read().await;

        inner
            .beads
            .get(bead_id)
            .cloned()
            .ok_or_else(|| SwarmError::BeadNotFound {
                bead_id: bead_id.to_string(),
            })
    }

    /// Get statistics about the queue.
    #[must_use]
    pub async fn stats(&self) -> WorkQueueStats {
        let inner = self.inner.read().await;

        let mut stats = WorkQueueStats::default();

        for bead in inner.beads.values() {
            match bead.state {
                BeadWorkState::Pending => {
                    stats.pending = stats.pending.saturating_add(1);
                }
                BeadWorkState::Claimed
                | BeadWorkState::ContractReady
                | BeadWorkState::Implementing
                | BeadWorkState::Reviewing => {
                    stats.in_progress = stats.in_progress.saturating_add(1);
                }
                BeadWorkState::Landed => {
                    stats.completed = stats.completed.saturating_add(1);
                }
                BeadWorkState::ImplementationComplete => {
                    stats.ready_review = stats.ready_review.saturating_add(1);
                }
                BeadWorkState::Failed => {
                    stats.failed = stats.failed.saturating_add(1);
                }
            }
        }

        stats
    }
}

/// Statistics for work queue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkQueueStats {
    /// Beads pending assignment.
    pub pending: usize,

    /// Beads currently in progress.
    pub in_progress: usize,

    /// Beads completed.
    pub completed: usize,

    /// Beads ready for review.
    pub ready_review: usize,

    /// Beads that failed.
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_work_queue_add_and_claim() {
        let queue = WorkQueue::new();

        queue
            .add_bead("test-123".to_string())
            .await
            .expect("Failed to add bead");

        let bead = queue
            .claim_next("agent-1".to_string())
            .await
            .expect("Failed to claim bead");

        assert_eq!(bead.bead_id, "test-123");
        assert_eq!(bead.state, BeadWorkState::Pending);
    }

    #[tokio::test]
    async fn test_work_queue_stats() {
        let queue = WorkQueue::new();

        queue
            .add_bead("test-1".to_string())
            .await
            .expect("Failed to add bead");
        queue
            .add_bead("test-2".to_string())
            .await
            .expect("Failed to add bead");

        let stats = queue.stats().await;
        assert_eq!(stats.pending, 2);
    }

    #[tokio::test]
    async fn test_bead_work_item_transition() {
        let mut bead = BeadWorkItem::new("test-123".to_string());

        bead.transition_to(BeadWorkState::Claimed)
            .expect("Failed to transition");

        assert_eq!(bead.state, BeadWorkState::Claimed);

        let result = bead.transition_to(BeadWorkState::Pending);
        assert!(result.is_err());
    }
}
