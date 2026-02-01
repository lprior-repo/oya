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
        let mut state = self.initial_state();
        for event in events {
            self.apply(&mut state, &event);
            if let Some(t) = tracker {
                t.increment()?;
            }
        }
        Ok(state)
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

    #[test]
    fn test_bead_projection_new() {
        let bead_id = BeadId::new();
        let proj = BeadProjection::new(bead_id);

        assert_eq!(proj.bead_id, bead_id);
        assert_eq!(proj.current_state, BeadState::Pending);
        assert!(!proj.is_blocked());
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
}
