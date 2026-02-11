//! Projections for materialized views from events.

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::Result;
use crate::event::BeadEvent;
use crate::replay::ReplayTracker;
use crate::store::EventStore;
use crate::types::{BeadId, BeadSpec, BeadState, PhaseId, StateTransition};

/// Trait for projections (materialized views).
#[async_trait]
pub trait Projection: Send + Sync {
    /// The state type this projection produces.
    type State: Send + Sync + Clone;

    /// Apply an event to the state.
    fn apply(&self, state: &mut Self::State, event: &BeadEvent);

    /// Get the initial state.
    fn initial_state(&self) -> Self::State;

    /// Rebuild the state from a store.
    async fn rebuild(&self, store: &dyn EventStore) -> Result<Self::State> {
        self.rebuild_with_progress(store, None).await
    }

    /// Rebuild the state from a store with optional progress tracking.
    async fn rebuild_with_progress(
        &self,
        store: &dyn EventStore,
        tracker: Option<&ReplayTracker>,
    ) -> Result<Self::State> {
        let events = store.read(None).await?;

        // Use functional fold for immutable state application
        events
            .iter()
            .try_fold(self.initial_state(), |mut state, event| {
                self.apply(&mut state, event);
                if let Some(t) = tracker {
                    t.increment()?;
                }
                Ok(state)
            })
    }
}

/// Projected state for a single bead.
#[derive(Debug, Clone)]
pub struct BeadProjection {
    /// Bead ID.
    pub bead_id: BeadId,
    /// Current state.
    pub current_state: BeadState,
    /// Current phase ID (if any).
    pub current_phase: Option<PhaseId>,
    /// Bead specification.
    pub spec: Option<BeadSpec>,
    /// Dependencies.
    pub dependencies: Vec<BeadId>,
    /// Beads that block this one.
    pub blocked_by: Vec<BeadId>,
    /// State transition history.
    pub history: Vec<StateTransition>,
    /// Claiming agent.
    pub claimed_by: Option<String>,
}

impl BeadProjection {
    /// Create a new bead projection.
    pub fn new(bead_id: BeadId) -> Self {
        Self {
            bead_id,
            current_state: BeadState::Pending,
            current_phase: None,
            spec: None,
            dependencies: Vec::new(),
            blocked_by: Vec::new(),
            history: Vec::new(),
            claimed_by: None,
        }
    }

    /// Check if the bead is blocked.
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by.is_empty()
    }

    /// Check if the bead is ready to run.
    pub fn is_ready(&self) -> bool {
        self.current_state == BeadState::Ready && !self.is_blocked()
    }
}

/// State for all beads.
#[derive(Debug, Clone, Default)]
pub struct AllBeadsState {
    /// Projections by bead ID.
    pub beads: HashMap<BeadId, BeadProjection>,
    /// Count by state.
    pub state_counts: HashMap<BeadState, usize>,
}

impl AllBeadsState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a bead projection.
    pub fn get(&self, bead_id: &BeadId) -> Option<&BeadProjection> {
        self.beads.get(bead_id)
    }

    /// Get beads in a given state.
    pub fn in_state(&self, state: BeadState) -> Vec<&BeadProjection> {
        self.beads
            .values()
            .filter(|b| b.current_state == state)
            .collect()
    }

    /// Get ready beads (not blocked).
    pub fn ready(&self) -> Vec<&BeadProjection> {
        self.beads.values().filter(|b| b.is_ready()).collect()
    }

    /// Get blocked beads.
    pub fn blocked(&self) -> Vec<&BeadProjection> {
        self.beads.values().filter(|b| b.is_blocked()).collect()
    }

    /// Get beads claimed by a specific agent.
    pub fn claimed_by(&self, agent_id: &str) -> Vec<&BeadProjection> {
        self.beads
            .values()
            .filter(|b| b.claimed_by.as_deref() == Some(agent_id))
            .collect()
    }

    /// Get unclaimed beads (not currently claimed by any agent).
    pub fn unclaimed(&self) -> Vec<&BeadProjection> {
        self.beads
            .values()
            .filter(|b| b.claimed_by.is_none())
            .collect()
    }

    /// Get beads in a specific phase.
    pub fn in_phase(&self, phase_id: PhaseId) -> Vec<&BeadProjection> {
        self.beads
            .values()
            .filter(|b| b.current_phase == Some(phase_id))
            .collect()
    }

    /// Get beads that depend on a specific bead.
    pub fn dependents_of(&self, bead_id: BeadId) -> Vec<&BeadProjection> {
        self.beads
            .values()
            .filter(|b| b.dependencies.contains(&bead_id))
            .collect()
    }

    /// Get count of beads in a specific state.
    pub fn count_in_state(&self, state: BeadState) -> usize {
        self.state_counts.get(&state).copied().map_or(0, |c| c)
    }

    /// Get all beads sorted by creation order (based on ULID timestamp).
    pub fn all_sorted(&self) -> Vec<&BeadProjection> {
        let mut beads: Vec<_> = self.beads.values().collect();
        beads.sort_by_key(|b| b.bead_id.as_ulid());
        beads
    }
}

/// Projection for all beads.
#[derive(Default)]
pub struct AllBeadsProjection;

impl AllBeadsProjection {
    /// Create a new projection.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Projection for AllBeadsProjection {
    type State = AllBeadsState;

    fn apply(&self, state: &mut Self::State, event: &BeadEvent) {
        match event {
            BeadEvent::Created { bead_id, spec, .. } => {
                let mut projection = BeadProjection::new(*bead_id);
                projection.spec = Some(spec.clone());
                projection.dependencies = spec.dependencies.clone();

                // Update state counts
                *state.state_counts.entry(BeadState::Pending).or_insert(0) += 1;

                state.beads.insert(*bead_id, projection);
            }
            BeadEvent::StateChanged {
                bead_id, from, to, ..
            } => {
                if let Some(bead) = state.beads.get_mut(bead_id) {
                    bead.current_state = *to;
                    bead.history.push(StateTransition::new(*from, *to));

                    // Update state counts
                    if let Some(count) = state.state_counts.get_mut(from) {
                        *count = count.saturating_sub(1);
                    }
                    *state.state_counts.entry(*to).or_insert(0) += 1;
                }
            }
            BeadEvent::PhaseCompleted {
                bead_id, phase_id, ..
            } => {
                if let Some(bead) = state.beads.get_mut(bead_id) {
                    bead.current_phase = Some(*phase_id);
                }
            }
            BeadEvent::DependencyResolved {
                bead_id,
                dependency_id,
                ..
            } => {
                if let Some(bead) = state.beads.get_mut(bead_id) {
                    bead.blocked_by.retain(|id| id != dependency_id);
                }
            }
            BeadEvent::Claimed {
                bead_id, agent_id, ..
            } => {
                if let Some(bead) = state.beads.get_mut(bead_id) {
                    bead.claimed_by = Some(agent_id.clone());
                }
            }
            BeadEvent::Unclaimed { bead_id, .. } => {
                if let Some(bead) = state.beads.get_mut(bead_id) {
                    bead.claimed_by = None;
                }
            }
            _ => {}
        }
    }

    fn initial_state(&self) -> Self::State {
        AllBeadsState::new()
    }
}

/// A managed projection with automatic updates.
pub struct ManagedProjection<P: Projection> {
    projection: P,
    state: RwLock<P::State>,
}

impl<P: Projection> ManagedProjection<P> {
    /// Create a new managed projection.
    pub fn new(projection: P) -> Self {
        let state = projection.initial_state();
        Self {
            projection,
            state: RwLock::new(state),
        }
    }

    /// Create from an existing state.
    pub fn with_state(projection: P, state: P::State) -> Self {
        Self {
            projection,
            state: RwLock::new(state),
        }
    }

    /// Apply an event.
    pub async fn apply(&self, event: &BeadEvent) {
        let mut state = self.state.write().await;
        self.projection.apply(&mut state, event);
    }

    /// Get the current state.
    pub async fn state(&self) -> P::State {
        self.state.read().await.clone()
    }

    /// Rebuild from a store.
    pub async fn rebuild(&self, store: &dyn EventStore) -> Result<()> {
        self.rebuild_with_progress(store, None).await
    }

    /// Rebuild from a store with optional progress tracking.
    pub async fn rebuild_with_progress(
        &self,
        store: &dyn EventStore,
        tracker: Option<&ReplayTracker>,
    ) -> Result<()> {
        let new_state = self
            .projection
            .rebuild_with_progress(store, tracker)
            .await?;
        let mut state = self.state.write().await;
        *state = new_state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryEventStore;
    use crate::types::Complexity;
    use std::sync::Arc;

    // ==========================================================================
    // BeadProjection BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn test_bead_projection_new() {
        let bead_id = BeadId::new();
        let proj = BeadProjection::new(bead_id);

        assert_eq!(proj.bead_id, bead_id);
        assert_eq!(proj.current_state, BeadState::Pending);
        assert!(!proj.is_blocked());
    }

    #[test]
    fn should_report_blocked_when_blocked_by_is_not_empty() {
        // Given: a bead with blockers
        let mut proj = BeadProjection::new(BeadId::new());
        proj.blocked_by.push(BeadId::new());

        // When/Then: is_blocked returns true
        assert!(
            proj.is_blocked(),
            "Bead with blockers should report as blocked"
        );
    }

    #[test]
    fn should_report_not_blocked_when_blocked_by_is_empty() {
        // Given: a bead with no blockers
        let proj = BeadProjection::new(BeadId::new());

        // When/Then: is_blocked returns false
        assert!(
            !proj.is_blocked(),
            "Bead without blockers should not be blocked"
        );
    }

    #[test]
    fn should_be_ready_only_when_state_is_ready_and_not_blocked() {
        // Given: a bead in Ready state with no blockers
        let mut proj = BeadProjection::new(BeadId::new());
        proj.current_state = BeadState::Ready;

        // When/Then: is_ready returns true
        assert!(
            proj.is_ready(),
            "Ready bead with no blockers should be ready"
        );
    }

    #[test]
    fn should_not_be_ready_when_state_is_not_ready() {
        // Given: a bead in Pending state
        let proj = BeadProjection::new(BeadId::new());
        assert_eq!(proj.current_state, BeadState::Pending);

        // When/Then: is_ready returns false
        assert!(!proj.is_ready(), "Pending bead should not be ready");
    }

    #[test]
    fn should_not_be_ready_when_blocked_even_if_state_is_ready() {
        // Given: a bead in Ready state BUT with blockers
        let mut proj = BeadProjection::new(BeadId::new());
        proj.current_state = BeadState::Ready;
        proj.blocked_by.push(BeadId::new());

        // When/Then: is_ready returns false (blocked overrides ready state)
        assert!(
            !proj.is_ready(),
            "Blocked bead should not be ready even if state is Ready"
        );
    }

    // ==========================================================================
    // AllBeadsState BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_return_bead_when_it_exists() {
        // Given: state with a bead
        let bead_id = BeadId::new();
        let mut state = AllBeadsState::new();
        state.beads.insert(bead_id, BeadProjection::new(bead_id));

        // When: we get the bead
        let result = state.get(&bead_id);

        // Then: we get the actual bead, not None
        assert!(result.is_some(), "Should return Some for existing bead");
        assert_eq!(
            result.map(|b| b.bead_id),
            Some(bead_id),
            "Should return correct bead"
        );
    }

    #[test]
    fn should_return_none_for_nonexistent_bead() {
        // Given: empty state
        let state = AllBeadsState::new();

        // When: we query for a nonexistent bead
        let result = state.get(&BeadId::new());

        // Then: we get None
        assert!(result.is_none(), "Should return None for nonexistent bead");
    }

    #[test]
    fn should_filter_beads_by_state_correctly() {
        // Given: state with beads in different states
        let mut state = AllBeadsState::new();

        let pending_id = BeadId::new();
        let mut pending_bead = BeadProjection::new(pending_id);
        pending_bead.current_state = BeadState::Pending;
        state.beads.insert(pending_id, pending_bead);

        let ready_id = BeadId::new();
        let mut ready_bead = BeadProjection::new(ready_id);
        ready_bead.current_state = BeadState::Ready;
        state.beads.insert(ready_id, ready_bead);

        let completed_id = BeadId::new();
        let mut completed_bead = BeadProjection::new(completed_id);
        completed_bead.current_state = BeadState::Completed;
        state.beads.insert(completed_id, completed_bead);

        // When: we filter by Ready state
        let ready_beads = state.in_state(BeadState::Ready);

        // Then: only the Ready bead is returned
        assert_eq!(ready_beads.len(), 1, "Should return exactly one Ready bead");
        assert_eq!(
            ready_beads[0].bead_id, ready_id,
            "Should return the correct Ready bead"
        );

        // When: we filter by Pending state
        let pending_beads = state.in_state(BeadState::Pending);

        // Then: only the Pending bead is returned
        assert_eq!(
            pending_beads.len(),
            1,
            "Should return exactly one Pending bead"
        );
        assert_eq!(
            pending_beads[0].bead_id, pending_id,
            "Should return the correct Pending bead"
        );
    }

    #[test]
    fn should_return_only_ready_unblocked_beads() {
        // Given: state with ready/blocked/pending beads
        let mut state = AllBeadsState::new();

        // Ready and unblocked
        let ready_id = BeadId::new();
        let mut ready_bead = BeadProjection::new(ready_id);
        ready_bead.current_state = BeadState::Ready;
        state.beads.insert(ready_id, ready_bead);

        // Ready but blocked
        let blocked_id = BeadId::new();
        let mut blocked_bead = BeadProjection::new(blocked_id);
        blocked_bead.current_state = BeadState::Ready;
        blocked_bead.blocked_by.push(BeadId::new());
        state.beads.insert(blocked_id, blocked_bead);

        // Pending
        let pending_id = BeadId::new();
        state
            .beads
            .insert(pending_id, BeadProjection::new(pending_id));

        // When: we get ready beads
        let ready_beads = state.ready();

        // Then: only the ready AND unblocked bead is returned
        assert_eq!(ready_beads.len(), 1, "Should return exactly one ready bead");
        assert_eq!(
            ready_beads[0].bead_id, ready_id,
            "Should return the unblocked ready bead"
        );
    }

    #[test]
    fn should_return_only_blocked_beads() {
        // Given: state with blocked and unblocked beads
        let mut state = AllBeadsState::new();

        // Blocked bead
        let blocked_id = BeadId::new();
        let mut blocked_bead = BeadProjection::new(blocked_id);
        blocked_bead.blocked_by.push(BeadId::new());
        state.beads.insert(blocked_id, blocked_bead);

        // Unblocked bead
        let unblocked_id = BeadId::new();
        state
            .beads
            .insert(unblocked_id, BeadProjection::new(unblocked_id));

        // When: we get blocked beads
        let blocked_beads = state.blocked();

        // Then: only the blocked bead is returned
        assert_eq!(
            blocked_beads.len(),
            1,
            "Should return exactly one blocked bead"
        );
        assert_eq!(
            blocked_beads[0].bead_id, blocked_id,
            "Should return the blocked bead"
        );
    }

    // ==========================================================================
    // AllBeadsProjection BEHAVIORAL TESTS - Event Application
    // ==========================================================================

    #[test]
    fn should_update_state_counts_on_bead_creation() {
        // Given: projection with no beads
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        // When: we create a bead
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        proj.apply(&mut state, &BeadEvent::created(bead_id, spec));

        // Then: state counts are updated
        assert_eq!(
            state.state_counts.get(&BeadState::Pending),
            Some(&1),
            "Pending count should be 1 after creation"
        );
    }

    #[test]
    fn should_track_phase_completion() {
        use crate::types::PhaseOutput;

        // Given: a created bead
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        proj.apply(&mut state, &BeadEvent::created(bead_id, spec));

        // When: phase completes
        let phase_id = PhaseId::new();
        proj.apply(
            &mut state,
            &BeadEvent::phase_completed(
                bead_id,
                phase_id,
                "test-phase",
                PhaseOutput::success(vec![]),
            ),
        );

        // Then: current_phase is updated
        assert_eq!(
            state.beads.get(&bead_id).and_then(|b| b.current_phase),
            Some(phase_id),
            "Phase should be tracked after completion"
        );
    }

    #[test]
    fn should_resolve_dependencies() {
        // Given: a bead blocked by another
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();
        let bead_id = BeadId::new();
        let blocker_id = BeadId::new();

        let spec = BeadSpec::new("Test")
            .with_complexity(Complexity::Simple)
            .with_dependency(blocker_id);
        proj.apply(&mut state, &BeadEvent::created(bead_id, spec));

        // Manually set up the blocked_by relationship
        if let Some(bead) = state.beads.get_mut(&bead_id) {
            bead.blocked_by.push(blocker_id);
        }

        // Verify it's blocked
        assert!(
            state
                .beads
                .get(&bead_id)
                .is_some_and(|b| b.is_blocked()),
            "Bead should be blocked before dependency resolution"
        );

        // When: dependency is resolved
        proj.apply(
            &mut state,
            &BeadEvent::dependency_resolved(bead_id, blocker_id),
        );

        // Then: bead is no longer blocked
        assert!(
            state
                .beads
                .get(&bead_id)
                .is_none_or(|b| !b.is_blocked()),
            "Bead should not be blocked after dependency resolved"
        );
    }

    #[test]
    fn should_track_claimed_agent() {
        // Given: a created bead
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        proj.apply(&mut state, &BeadEvent::created(bead_id, spec));

        // When: bead is claimed
        let agent_id = "agent-123".to_string();
        proj.apply(&mut state, &BeadEvent::claimed(bead_id, agent_id.clone()));

        // Then: claimed_by is set
        assert_eq!(
            state.beads.get(&bead_id).and_then(|b| b.claimed_by.clone()),
            Some(agent_id),
            "Claimed agent should be tracked"
        );
    }

    #[test]
    fn should_clear_claimed_agent_on_unclaim() {
        // Given: a claimed bead
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        proj.apply(&mut state, &BeadEvent::created(bead_id, spec));
        proj.apply(
            &mut state,
            &BeadEvent::claimed(bead_id, "agent-123".to_string()),
        );

        // When: bead is unclaimed
        proj.apply(&mut state, &BeadEvent::unclaimed(bead_id, None));

        // Then: claimed_by is cleared
        assert_eq!(
            state.beads.get(&bead_id).and_then(|b| b.claimed_by.clone()),
            None,
            "Claimed agent should be cleared on unclaim"
        );
    }

    #[test]
    fn should_return_fresh_initial_state() {
        // Given: projection
        let proj = AllBeadsProjection::new();

        // When: we get initial state
        let state = proj.initial_state();

        // Then: it's empty (not some default with data)
        assert!(state.beads.is_empty(), "Initial state should have no beads");
        assert!(
            state.state_counts.is_empty(),
            "Initial state should have no counts"
        );
    }

    #[test]
    fn test_all_beads_projection_created() {
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        let event = BeadEvent::created(bead_id, spec);

        proj.apply(&mut state, &event);

        assert!(state.beads.contains_key(&bead_id));
        assert_eq!(
            state.beads.get(&bead_id).map(|b| b.current_state),
            Some(BeadState::Pending)
        );
    }

    #[test]
    fn test_all_beads_projection_state_changed() {
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);

        proj.apply(&mut state, &BeadEvent::created(bead_id, spec));
        proj.apply(
            &mut state,
            &BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled),
        );

        assert_eq!(
            state.beads.get(&bead_id).map(|b| b.current_state),
            Some(BeadState::Scheduled)
        );
        assert_eq!(state.beads.get(&bead_id).map(|b| b.history.len()), Some(1));
    }

    #[tokio::test]
    async fn test_managed_projection() {
        let store = Arc::new(InMemoryEventStore::new());
        let managed = ManagedProjection::new(AllBeadsProjection::new());

        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        let event = BeadEvent::created(bead_id, spec);

        store.append(event.clone()).await.ok();
        managed.apply(&event).await;

        let state = managed.state().await;
        assert!(state.beads.contains_key(&bead_id));
    }

    #[tokio::test]
    async fn test_projection_rebuild() {
        let store = Arc::new(InMemoryEventStore::new());

        let bead_id = BeadId::new();
        store
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await
            .ok();
        store
            .append(BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled,
            ))
            .await
            .ok();

        let proj = AllBeadsProjection::new();
        let state = proj.rebuild(store.as_ref()).await;

        assert!(state.is_ok());
        let state = state.ok();
        assert_eq!(
            state.and_then(|s| s.beads.get(&bead_id).map(|b| b.current_state)),
            Some(BeadState::Scheduled)
        );
    }

    #[tokio::test]
    async fn test_projection_rebuild_with_progress() {
        use crate::replay::ReplayTracker;

        let store = Arc::new(InMemoryEventStore::new());

        // Add multiple events to test progress tracking
        for _ in 0..5 {
            let bead_id = BeadId::new();
            store
                .append(BeadEvent::created(
                    bead_id,
                    BeadSpec::new("Test").with_complexity(Complexity::Simple),
                ))
                .await
                .ok();
        }

        let (tracker, mut rx) = ReplayTracker::new(5, 1);
        let proj = AllBeadsProjection::new();
        let result = proj
            .rebuild_with_progress(store.as_ref(), Some(&tracker))
            .await;

        assert!(result.is_ok());

        // Verify progress was tracked
        rx.changed().await.ok();
        let progress = rx.borrow().clone();
        assert_eq!(progress.events_processed, 5);
        assert_eq!(progress.percent_complete, 100.0);
    }

    #[tokio::test]
    async fn test_managed_projection_rebuild_with_progress() {
        use crate::replay::ReplayTracker;

        let store = Arc::new(InMemoryEventStore::new());

        // Add events
        for _ in 0..10 {
            let bead_id = BeadId::new();
            store
                .append(BeadEvent::created(
                    bead_id,
                    BeadSpec::new("Test").with_complexity(Complexity::Simple),
                ))
                .await
                .ok();
        }

        let managed = ManagedProjection::new(AllBeadsProjection::new());
        let (tracker, mut rx) = ReplayTracker::new(10, 5);

        let result = managed
            .rebuild_with_progress(store.as_ref(), Some(&tracker))
            .await;
        assert!(result.is_ok());

        // Verify progress updates were emitted
        rx.changed().await.ok();
        let progress = rx.borrow().clone();
        assert!(progress.events_processed > 0);
        assert!(progress.percent_complete > 0.0);
    }

    // ==========================================================================
    // State Counts ADDITIVE BEHAVIORAL TESTS (catching += vs *= mutation)
    // ==========================================================================

    #[test]
    fn should_increment_state_count_additively_on_state_change() {
        // Given: projection with multiple beads in different states
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        // Create 3 beads (all start in Pending)
        let bead1 = BeadId::new();
        let bead2 = BeadId::new();
        let bead3 = BeadId::new();

        proj.apply(
            &mut state,
            &BeadEvent::created(
                bead1,
                BeadSpec::new("Bead 1").with_complexity(Complexity::Simple),
            ),
        );
        proj.apply(
            &mut state,
            &BeadEvent::created(
                bead2,
                BeadSpec::new("Bead 2").with_complexity(Complexity::Simple),
            ),
        );
        proj.apply(
            &mut state,
            &BeadEvent::created(
                bead3,
                BeadSpec::new("Bead 3").with_complexity(Complexity::Simple),
            ),
        );

        // Verify: 3 beads in Pending
        assert_eq!(
            state.state_counts.get(&BeadState::Pending),
            Some(&3),
            "Should have 3 Pending beads after creation"
        );

        // When: transition bead1 from Pending to Scheduled
        proj.apply(
            &mut state,
            &BeadEvent::state_changed(bead1, BeadState::Pending, BeadState::Scheduled),
        );

        // Then: Pending count DECREMENTS, Scheduled count INCREMENTS (additively, not multiplicatively)
        assert_eq!(
            state.state_counts.get(&BeadState::Pending),
            Some(&2),
            "Pending count should be 2 after one transition out"
        );
        assert_eq!(
            state.state_counts.get(&BeadState::Scheduled),
            Some(&1),
            "Scheduled count should be 1 after one transition in"
        );

        // When: transition bead2 from Pending to Scheduled
        proj.apply(
            &mut state,
            &BeadEvent::state_changed(bead2, BeadState::Pending, BeadState::Scheduled),
        );

        // Then: counts should be 1 and 2 respectively (ADDITIVE: 1+1=2, not MULTIPLICATIVE: 1*1=1)
        assert_eq!(
            state.state_counts.get(&BeadState::Pending),
            Some(&1),
            "Pending count should be 1 after two transitions out"
        );
        assert_eq!(
            state.state_counts.get(&BeadState::Scheduled),
            Some(&2),
            "Scheduled count should be 2 after two transitions in - tests += vs *="
        );
    }

    #[test]
    fn should_correctly_count_multiple_creations() {
        // This specifically tests the += operator in line 158 (created event)
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        // Create 5 beads
        for i in 0..5 {
            let bead_id = BeadId::new();
            proj.apply(
                &mut state,
                &BeadEvent::created(
                    bead_id,
                    BeadSpec::new(format!("Bead {}", i)).with_complexity(Complexity::Simple),
                ),
            );
        }

        // Count should be 5 (0+1+1+1+1+1 = 5), not 0 or 1 (multiplication would give 0*1*1*1*1*1 = 0)
        assert_eq!(
            state.state_counts.get(&BeadState::Pending),
            Some(&5),
            "Should have 5 Pending beads - tests additive counting"
        );
    }

    // ==========================================================================
    // initial_state BEHAVIORAL TESTS (catching Default::default() mutation)
    // ==========================================================================

    #[test]
    fn should_return_correct_type_from_initial_state() {
        let proj = AllBeadsProjection::new();
        let state: AllBeadsState = proj.initial_state();

        // Verify it's actually an AllBeadsState with empty collections
        // (Default::default() would also be empty, but this verifies the type)
        assert!(state.beads.is_empty());
        assert!(state.state_counts.is_empty());
    }

    #[test]
    fn should_allow_modification_after_initial_state() {
        // This tests that initial_state returns a usable AllBeadsState
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        // Should be able to insert beads (proving it's the right type)
        let bead_id = BeadId::new();
        state.beads.insert(bead_id, BeadProjection::new(bead_id));

        assert_eq!(state.beads.len(), 1);
    }

    // ==========================================================================
    // ManagedProjection::rebuild BEHAVIORAL TESTS (catching Ok(()) mutation)
    // ==========================================================================

    #[tokio::test]
    async fn should_actually_rebuild_state_from_store() {
        // This tests that rebuild() actually rebuilds from the store
        // (not just returning Ok(()) without doing anything)
        let store = Arc::new(InMemoryEventStore::new());

        // Add events to the store
        let bead_id = BeadId::new();
        store
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await
            .ok();
        store
            .append(BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled,
            ))
            .await
            .ok();

        // Create managed projection (starts with empty state)
        let managed = ManagedProjection::new(AllBeadsProjection::new());

        // Verify initial state is empty
        let state_before = managed.state().await;
        assert!(
            state_before.beads.is_empty(),
            "State should be empty before rebuild"
        );

        // When: rebuild from store
        let result = managed.rebuild(store.as_ref()).await;
        assert!(result.is_ok());

        // Then: state should contain the bead from the store
        let state_after = managed.state().await;
        assert!(
            !state_after.beads.is_empty(),
            "State should NOT be empty after rebuild - tests that rebuild actually processes events"
        );
        assert!(
            state_after.beads.contains_key(&bead_id),
            "State should contain the bead from the store"
        );
        assert_eq!(
            state_after.beads.get(&bead_id).map(|b| b.current_state),
            Some(BeadState::Scheduled),
            "Bead should be in Scheduled state after rebuild"
        );
    }

    #[tokio::test]
    async fn should_replace_existing_state_on_rebuild() {
        // This ensures rebuild replaces state, not appends to it
        let store = Arc::new(InMemoryEventStore::new());

        // Add one bead to store
        let store_bead_id = BeadId::new();
        store
            .append(BeadEvent::created(
                store_bead_id,
                BeadSpec::new("Store Bead").with_complexity(Complexity::Simple),
            ))
            .await
            .ok();

        // Create managed projection with an existing bead
        let proj = AllBeadsProjection::new();
        let mut initial_state = proj.initial_state();
        let existing_bead_id = BeadId::new();
        initial_state
            .beads
            .insert(existing_bead_id, BeadProjection::new(existing_bead_id));

        let managed = ManagedProjection::with_state(proj, initial_state);

        // Verify existing bead is present
        let state_before = managed.state().await;
        assert!(state_before.beads.contains_key(&existing_bead_id));
        assert!(!state_before.beads.contains_key(&store_bead_id));

        // When: rebuild
        managed.rebuild(store.as_ref()).await.ok();

        // Then: state should be REPLACED (existing bead gone, store bead present)
        let state_after = managed.state().await;
        assert!(
            !state_after.beads.contains_key(&existing_bead_id),
            "Existing bead should be gone after rebuild"
        );
        assert!(
            state_after.beads.contains_key(&store_bead_id),
            "Store bead should be present after rebuild"
        );
    }

    // ==========================================================================
    // ADDITIONAL QUERY METHODS TESTS
    // ==========================================================================

    #[test]
    fn should_filter_beads_by_claimed_agent() {
        // Given: state with beads claimed by different agents
        let mut state = AllBeadsState::new();

        let agent1_bead = BeadId::new();
        let mut agent1_proj = BeadProjection::new(agent1_bead);
        agent1_proj.claimed_by = Some("agent-1".to_string());
        state.beads.insert(agent1_bead, agent1_proj);

        let agent2_bead = BeadId::new();
        let mut agent2_proj = BeadProjection::new(agent2_bead);
        agent2_proj.claimed_by = Some("agent-2".to_string());
        state.beads.insert(agent2_bead, agent2_proj);

        let unclaimed_bead = BeadId::new();
        state
            .beads
            .insert(unclaimed_bead, BeadProjection::new(unclaimed_bead));

        // When: query for beads claimed by agent-1
        let agent1_beads = state.claimed_by("agent-1");

        // Then: only agent-1's bead is returned
        assert_eq!(agent1_beads.len(), 1);
        assert_eq!(agent1_beads[0].bead_id, agent1_bead);
    }

    #[test]
    fn should_return_unclaimed_beads() {
        // Given: state with claimed and unclaimed beads
        let mut state = AllBeadsState::new();

        let claimed_bead = BeadId::new();
        let mut claimed_proj = BeadProjection::new(claimed_bead);
        claimed_proj.claimed_by = Some("agent-1".to_string());
        state.beads.insert(claimed_bead, claimed_proj);

        let unclaimed_bead = BeadId::new();
        state
            .beads
            .insert(unclaimed_bead, BeadProjection::new(unclaimed_bead));

        // When: query for unclaimed beads
        let unclaimed_beads = state.unclaimed();

        // Then: only the unclaimed bead is returned
        assert_eq!(unclaimed_beads.len(), 1);
        assert_eq!(unclaimed_beads[0].bead_id, unclaimed_bead);
    }

    #[test]
    fn should_filter_beads_by_phase() {
        use crate::types::PhaseOutput;

        // Given: state with beads in different phases
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        let bead1 = BeadId::new();
        let spec1 = BeadSpec::new("Bead 1").with_complexity(Complexity::Simple);
        proj.apply(&mut state, &BeadEvent::created(bead1, spec1));

        let phase1_id = PhaseId::new();
        proj.apply(
            &mut state,
            &BeadEvent::phase_completed(bead1, phase1_id, "phase-1", PhaseOutput::success(vec![])),
        );

        let bead2 = BeadId::new();
        let spec2 = BeadSpec::new("Bead 2").with_complexity(Complexity::Simple);
        proj.apply(&mut state, &BeadEvent::created(bead2, spec2));

        let phase2_id = PhaseId::new();
        proj.apply(
            &mut state,
            &BeadEvent::phase_completed(bead2, phase2_id, "phase-2", PhaseOutput::success(vec![])),
        );

        // When: query for beads in phase1
        let phase1_beads = state.in_phase(phase1_id);

        // Then: only bead1 is returned
        assert_eq!(phase1_beads.len(), 1);
        assert_eq!(phase1_beads[0].bead_id, bead1);
    }

    #[test]
    fn should_find_dependents_of_a_bead() {
        // Given: state with beads having dependencies
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        let parent = BeadId::new();
        let parent_spec = BeadSpec::new("Parent").with_complexity(Complexity::Simple);
        proj.apply(&mut state, &BeadEvent::created(parent, parent_spec));

        let child1 = BeadId::new();
        let child1_spec = BeadSpec::new("Child 1")
            .with_complexity(Complexity::Simple)
            .with_dependency(parent);
        proj.apply(&mut state, &BeadEvent::created(child1, child1_spec));

        let child2 = BeadId::new();
        let child2_spec = BeadSpec::new("Child 2")
            .with_complexity(Complexity::Simple)
            .with_dependency(parent);
        proj.apply(&mut state, &BeadEvent::created(child2, child2_spec));

        let unrelated = BeadId::new();
        let unrelated_spec = BeadSpec::new("Unrelated").with_complexity(Complexity::Simple);
        proj.apply(&mut state, &BeadEvent::created(unrelated, unrelated_spec));

        // When: query for dependents of parent
        let dependents = state.dependents_of(parent);

        // Then: child1 and child2 are returned, not unrelated
        assert_eq!(dependents.len(), 2);
        let dep_ids: Vec<_> = dependents.iter().map(|b| b.bead_id).collect();
        assert!(dep_ids.contains(&child1));
        assert!(dep_ids.contains(&child2));
        assert!(!dep_ids.contains(&unrelated));
    }

    #[test]
    fn should_count_beads_in_state() {
        // Given: state with beads in different states
        let proj = AllBeadsProjection::new();
        let mut state = proj.initial_state();

        // Create 3 Pending beads
        for _ in 0..3 {
            let bead_id = BeadId::new();
            let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
            proj.apply(&mut state, &BeadEvent::created(bead_id, spec));
        }

        // Transition one to Scheduled
        if let Some(first_bead) = state.beads.keys().next().copied() {
            proj.apply(
                &mut state,
                &BeadEvent::state_changed(first_bead, BeadState::Pending, BeadState::Scheduled),
            );
        } else {
            // This should never happen since we just created 3 beads
            return;
        }

        // When: count Pending beads
        let pending_count = state.count_in_state(BeadState::Pending);

        // Then: count is 2 (one was transitioned)
        assert_eq!(pending_count, 2);

        // When: count Scheduled beads
        let scheduled_count = state.count_in_state(BeadState::Scheduled);

        // Then: count is 1
        assert_eq!(scheduled_count, 1);
    }

    #[test]
    fn should_return_zero_for_nonexistent_state_count() {
        // Given: empty state
        let state = AllBeadsState::new();

        // When: count beads in Completed state (which doesn't exist)
        let count = state.count_in_state(BeadState::Completed);

        // Then: count is 0 (not None or panic)
        assert_eq!(count, 0);
    }

    #[test]
    fn should_sort_beads_by_creation_order() {
        // Given: state with beads created at different times
        let mut state = AllBeadsState::new();

        // Create beads with specific timing
        let bead1 = BeadId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let bead2 = BeadId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let bead3 = BeadId::new();

        // Insert in random order
        state.beads.insert(bead3, BeadProjection::new(bead3));
        state.beads.insert(bead1, BeadProjection::new(bead1));
        state.beads.insert(bead2, BeadProjection::new(bead2));

        // When: get all sorted
        let sorted = state.all_sorted();

        // Then: beads are in ULID order (which is time-ordered)
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].bead_id, bead1);
        assert_eq!(sorted[1].bead_id, bead2);
        assert_eq!(sorted[2].bead_id, bead3);
    }
}
