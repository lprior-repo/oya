//! Core types for the reconciler.

use std::collections::HashMap;

use oya_events::{BeadId, BeadProjection, BeadResult, BeadSpec, BeadState};
use serde::{Deserialize, Serialize};

/// Desired state declaration.
///
/// This represents what the system should look like - beads to create,
/// dependencies to establish, etc.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DesiredState {
    /// Beads that should exist.
    pub beads: HashMap<BeadId, BeadSpec>,
    /// Dependencies between beads.
    pub dependencies: HashMap<BeadId, Vec<BeadId>>,
}

impl DesiredState {
    /// Create a new empty desired state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a bead to the desired state.
    pub fn add_bead(&mut self, id: BeadId, spec: BeadSpec) {
        if !spec.dependencies.is_empty() {
            self.dependencies.insert(id, spec.dependencies.clone());
        }
        self.beads.insert(id, spec);
    }

    /// Remove a bead from the desired state.
    pub fn remove_bead(&mut self, id: &BeadId) {
        self.beads.remove(id);
        self.dependencies.remove(id);
    }

    /// Get a bead spec.
    pub fn get(&self, id: &BeadId) -> Option<&BeadSpec> {
        self.beads.get(id)
    }

    /// Get the dependencies for a bead.
    pub fn dependencies(&self, id: &BeadId) -> Vec<BeadId> {
        self.dependencies.get(id).cloned().unwrap_or_default()
    }

    /// Get the number of beads.
    pub fn len(&self) -> usize {
        self.beads.len()
    }

    /// Check if the desired state is empty.
    pub fn is_empty(&self) -> bool {
        self.beads.is_empty()
    }
}

/// Actual state (computed from events).
///
/// This represents what the system actually looks like right now.
#[derive(Debug, Clone, Default)]
pub struct ActualState {
    /// Projected state of all beads.
    pub beads: HashMap<BeadId, BeadProjection>,
    /// Number of running beads.
    pub running_count: usize,
    /// Number of pending beads.
    pub pending_count: usize,
    /// Number of completed beads.
    pub completed_count: usize,
}

impl ActualState {
    /// Create a new empty actual state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update from a bead projection.
    pub fn update(&mut self, projection: BeadProjection) {
        // Update counts
        if let Some(old) = self.beads.get(&projection.bead_id) {
            self.decrement_count(old.current_state);
        }
        self.increment_count(projection.current_state);

        self.beads.insert(projection.bead_id, projection);
    }

    /// Get a bead projection.
    pub fn get(&self, id: &BeadId) -> Option<&BeadProjection> {
        self.beads.get(id)
    }

    /// Get beads in a given state.
    pub fn in_state(&self, state: BeadState) -> Vec<&BeadProjection> {
        self.beads
            .values()
            .filter(|b| b.current_state == state)
            .collect()
    }

    /// Get beads that are ready to run (scheduled and not blocked).
    pub fn ready_to_run(&self) -> Vec<&BeadProjection> {
        self.beads
            .values()
            .filter(|b| b.current_state == BeadState::Scheduled && b.blocked_by.is_empty())
            .collect()
    }

    /// Get beads that exist in actual state but not desired state.
    pub fn orphaned_beads<'a>(&'a self, desired: &DesiredState) -> Vec<&'a BeadProjection> {
        self.beads
            .values()
            .filter(|b| !desired.beads.contains_key(&b.bead_id))
            .collect()
    }

    /// Increment count for a state.
    fn increment_count(&mut self, state: BeadState) {
        match state {
            BeadState::Running => self.running_count += 1,
            BeadState::Pending => self.pending_count += 1,
            BeadState::Completed => self.completed_count += 1,
            _ => {}
        }
    }

    /// Decrement count for a state.
    fn decrement_count(&mut self, state: BeadState) {
        match state {
            BeadState::Running => self.running_count = self.running_count.saturating_sub(1),
            BeadState::Pending => self.pending_count = self.pending_count.saturating_sub(1),
            BeadState::Completed => self.completed_count = self.completed_count.saturating_sub(1),
            _ => {}
        }
    }
}

/// Actions the reconciler can take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconcileAction {
    /// Create a new bead.
    CreateBead { bead_id: BeadId, spec: BeadSpec },
    /// Start a bead (transition to Running).
    StartBead { bead_id: BeadId },
    /// Stop a bead.
    StopBead { bead_id: BeadId, reason: String },
    /// Retry a failed bead.
    RetryBead { bead_id: BeadId },
    /// Mark a bead as complete.
    MarkComplete { bead_id: BeadId, result: BeadResult },
    /// Update bead dependencies.
    UpdateDependencies {
        bead_id: BeadId,
        dependencies: Vec<BeadId>,
    },
    /// Schedule a bead (transition to Scheduled).
    ScheduleBead { bead_id: BeadId },
    /// Delete a bead.
    DeleteBead { bead_id: BeadId },
}

impl ReconcileAction {
    /// Get the bead ID this action targets.
    pub fn bead_id(&self) -> BeadId {
        match self {
            Self::CreateBead { bead_id, .. }
            | Self::StartBead { bead_id }
            | Self::StopBead { bead_id, .. }
            | Self::RetryBead { bead_id }
            | Self::MarkComplete { bead_id, .. }
            | Self::UpdateDependencies { bead_id, .. }
            | Self::ScheduleBead { bead_id }
            | Self::DeleteBead { bead_id } => *bead_id,
        }
    }

    /// Get a description of the action.
    pub fn description(&self) -> String {
        match self {
            Self::CreateBead { bead_id, spec } => {
                format!("create bead {} ({})", bead_id, spec.title)
            }
            Self::StartBead { bead_id } => {
                format!("start bead {bead_id}")
            }
            Self::StopBead { bead_id, reason } => {
                format!("stop bead {bead_id}: {reason}")
            }
            Self::RetryBead { bead_id } => {
                format!("retry bead {bead_id}")
            }
            Self::MarkComplete { bead_id, result } => {
                let status = if result.success { "success" } else { "failure" };
                format!("complete bead {bead_id} ({status})")
            }
            Self::UpdateDependencies {
                bead_id,
                dependencies,
            } => {
                format!("update deps for {bead_id} ({} deps)", dependencies.len())
            }
            Self::ScheduleBead { bead_id } => {
                format!("schedule bead {bead_id}")
            }
            Self::DeleteBead { bead_id } => {
                format!("delete bead {bead_id}")
            }
        }
    }
}

/// Result of reconciliation.
#[derive(Debug, Clone)]
pub struct ReconcileResult {
    /// Actions that were taken.
    pub actions_taken: Vec<ReconcileAction>,
    /// Actions that failed.
    pub actions_failed: Vec<(ReconcileAction, String)>,
    /// Number of beads in desired state.
    pub desired_count: usize,
    /// Number of beads in actual state.
    pub actual_count: usize,
    /// Whether the system is converged.
    pub converged: bool,
}

impl ReconcileResult {
    /// Create a new reconcile result.
    pub fn new(
        actions_taken: Vec<ReconcileAction>,
        actions_failed: Vec<(ReconcileAction, String)>,
        desired_count: usize,
        actual_count: usize,
    ) -> Self {
        let converged = actions_taken.is_empty() && actions_failed.is_empty();
        Self {
            actions_taken,
            actions_failed,
            desired_count,
            actual_count,
            converged,
        }
    }

    /// Check if all actions succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.actions_failed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_events::Complexity;

    #[test]
    fn test_desired_state() {
        let mut desired = DesiredState::new();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);

        desired.add_bead(bead_id, spec);

        assert_eq!(desired.len(), 1);
        assert!(desired.get(&bead_id).is_some());
    }

    #[test]
    fn test_actual_state_counts() {
        let mut actual = ActualState::new();
        let bead_id = BeadId::new();

        let mut proj = BeadProjection::new(bead_id);
        proj.current_state = BeadState::Running;
        actual.update(proj);

        assert_eq!(actual.running_count, 1);
        assert_eq!(actual.pending_count, 0);
    }

    #[test]
    fn test_reconcile_action_description() {
        let action = ReconcileAction::StartBead {
            bead_id: BeadId::new(),
        };
        assert!(action.description().contains("start"));
    }
}
