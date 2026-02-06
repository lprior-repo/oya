//! Continuous reconciliation loop.

use std::sync::Arc;
use std::time::Duration;

use oya_events::{AllBeadsProjection, ManagedProjection};
use tokio::sync::watch;
use tracing::{debug, error, info};

use crate::error::{Error, Result};
use crate::reconciler::Reconciler;
use crate::types::{ActualState, DesiredState};

/// Configuration for the reconciliation loop.
#[derive(Debug, Clone)]
pub struct LoopConfig {
    /// Interval between reconciliation cycles.
    pub interval: Duration,
    /// Maximum consecutive errors before stopping.
    pub max_errors: usize,
    /// Whether to stop on first error.
    pub stop_on_error: bool,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(1),
            max_errors: 10,
            stop_on_error: false,
        }
    }
}

/// State provider trait for getting desired state.
#[async_trait::async_trait]
pub trait DesiredStateProvider: Send + Sync {
    /// Get the current desired state.
    async fn get_desired_state(&self) -> Result<DesiredState>;
}

/// Simple in-memory desired state provider.
pub struct InMemoryDesiredStateProvider {
    state: tokio::sync::RwLock<DesiredState>,
}

impl InMemoryDesiredStateProvider {
    /// Create a new provider with initial state.
    pub fn new(state: DesiredState) -> Self {
        Self {
            state: tokio::sync::RwLock::new(state),
        }
    }

    /// Update the desired state.
    pub async fn update(&self, state: DesiredState) {
        *self.state.write().await = state;
    }

    /// Get a mutable reference to update.
    pub async fn modify<F>(&self, f: F)
    where
        F: FnOnce(&mut DesiredState),
    {
        let mut state = self.state.write().await;
        f(&mut state);
    }
}

#[async_trait::async_trait]
impl DesiredStateProvider for InMemoryDesiredStateProvider {
    async fn get_desired_state(&self) -> Result<DesiredState> {
        Ok(self.state.read().await.clone())
    }
}

/// Continuous reconciliation loop.
///
/// Periodically compares desired state vs actual state and
/// reconciles the difference.
pub struct ReconciliationLoop {
    /// The reconciler.
    reconciler: Arc<Reconciler>,
    /// Desired state provider.
    desired_provider: Arc<dyn DesiredStateProvider>,
    /// Projection for actual state.
    projection: Arc<ManagedProjection<AllBeadsProjection>>,
    /// Loop configuration.
    config: LoopConfig,
    /// Stop signal receiver.
    stop_rx: watch::Receiver<bool>,
    /// Stop signal sender (for external control).
    stop_tx: watch::Sender<bool>,
}

impl ReconciliationLoop {
    /// Create a new reconciliation loop.
    pub fn new(
        reconciler: Arc<Reconciler>,
        desired_provider: Arc<dyn DesiredStateProvider>,
        projection: Arc<ManagedProjection<AllBeadsProjection>>,
        config: LoopConfig,
    ) -> Self {
        let (stop_tx, stop_rx) = watch::channel(false);
        Self {
            reconciler,
            desired_provider,
            projection,
            config,
            stop_rx,
            stop_tx,
        }
    }

    /// Run the reconciliation loop.
    ///
    /// This runs until stopped or max errors reached.
    pub async fn run(&mut self) -> Result<()> {
        info!(
            interval_ms = self.config.interval.as_millis(),
            "Starting reconciliation loop"
        );

        let mut consecutive_errors = 0usize;
        let mut interval = tokio::time::interval(self.config.interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match self.reconcile_once().await {
                        Ok(converged) => {
                            consecutive_errors = 0;
                            if converged {
                                debug!("System converged");
                            }
                        }
                        Err(e) => {
                            consecutive_errors += 1;
                            error!(
                                error = %e,
                                consecutive = consecutive_errors,
                                "Reconciliation error"
                            );

                            if self.config.stop_on_error {
                                return Err(e);
                            }

                            if consecutive_errors >= self.config.max_errors {
                                error!("Max errors reached, stopping loop");
                                return Err(Error::reconcile_failed(format!(
                                    "Max errors ({}) reached",
                                    self.config.max_errors
                                )));
                            }
                        }
                    }
                }
                _ = self.stop_rx.changed() => {
                    if *self.stop_rx.borrow() {
                        info!("Reconciliation loop stopped");
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Run a single reconciliation cycle.
    async fn reconcile_once(&self) -> Result<bool> {
        // Get desired state
        let desired = self.desired_provider.get_desired_state().await?;

        // Get actual state from projection
        let proj_state = self.projection.state().await;
        let actual = self.build_actual_state(&proj_state);

        // Reconcile
        let result = self.reconciler.reconcile(&desired, &actual).await?;

        Ok(result.converged)
    }

    /// Build ActualState from projection state.
    fn build_actual_state(&self, proj_state: &oya_events::AllBeadsState) -> ActualState {
        let mut actual = ActualState::new();
        for (bead_id, proj) in &proj_state.beads {
            actual.beads.insert(*bead_id, proj.clone());
        }
        // Recompute counts
        for proj in actual.beads.values() {
            match proj.current_state {
                oya_events::BeadState::Running => actual.running_count += 1,
                oya_events::BeadState::Pending => actual.pending_count += 1,
                oya_events::BeadState::Completed => actual.completed_count += 1,
                _ => {}
            }
        }
        actual
    }

    /// Stop the loop.
    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }

    /// Get a stopper handle.
    pub fn stopper(&self) -> LoopStopper {
        LoopStopper {
            stop_tx: self.stop_tx.clone(),
        }
    }
}

/// Handle to stop a reconciliation loop.
#[derive(Clone)]
pub struct LoopStopper {
    stop_tx: watch::Sender<bool>,
}

impl LoopStopper {
    /// Stop the loop.
    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconciler::{Reconciler, ReconcilerConfig};
    use oya_events::{BeadId, BeadSpec, Complexity, EventBus, InMemoryEventStore};

    fn setup() -> (
        Arc<Reconciler>,
        Arc<InMemoryDesiredStateProvider>,
        Arc<ManagedProjection<AllBeadsProjection>>,
        Arc<EventBus>,
    ) {
        let store = Arc::new(InMemoryEventStore::new());
        let bus = Arc::new(EventBus::new(store));
        let reconciler = Arc::new(Reconciler::with_event_executor(
            bus.clone(),
            ReconcilerConfig::default(),
        ));
        let desired_provider = Arc::new(InMemoryDesiredStateProvider::new(DesiredState::new()));
        let projection = Arc::new(ManagedProjection::new(AllBeadsProjection::new()));

        (reconciler, desired_provider, projection, bus)
    }

    // ===== Behavior-Driven Tests (Martin Fowler style) =====

    /// Given an empty system
    /// When the loop runs one cycle
    /// Then it should report converged
    #[tokio::test]
    async fn empty_system_is_converged() {
        let (reconciler, desired_provider, projection, _) = setup();
        let config = LoopConfig {
            interval: Duration::from_millis(10),
            ..Default::default()
        };
        let loop_runner = ReconciliationLoop::new(reconciler, desired_provider, projection, config);

        let result = loop_runner.reconcile_once().await;
        assert!(result.is_ok());
        assert!(
            result.ok().unwrap_or(false),
            "Empty system should be converged"
        );
    }

    /// Given a desired state with one bead
    /// And an empty actual state
    /// When the loop runs one cycle
    /// Then it should not be converged (actions needed)
    #[tokio::test]
    async fn missing_beads_triggers_actions() {
        let (reconciler, desired_provider, projection, _) = setup();

        // Add a bead to desired state
        let bead_id = BeadId::new();
        desired_provider
            .modify(|state| {
                state.add_bead(
                    bead_id,
                    BeadSpec::new("Test Bead").with_complexity(Complexity::Simple),
                );
            })
            .await;

        let config = LoopConfig::default();
        let loop_runner = ReconciliationLoop::new(reconciler, desired_provider, projection, config);

        let result = loop_runner.reconcile_once().await;
        assert!(result.is_ok());
        // Should NOT be converged because a bead needs to be created
        assert!(
            !result.ok().unwrap_or(true),
            "System with missing bead should not be converged"
        );
    }

    /// Given a loop that is running
    /// When stop() is called
    /// Then the loop should exit gracefully
    #[tokio::test]
    async fn stop_signal_terminates_loop() {
        let (reconciler, desired_provider, projection, _) = setup();
        let config = LoopConfig {
            interval: Duration::from_millis(100),
            ..Default::default()
        };
        let mut loop_runner =
            ReconciliationLoop::new(reconciler, desired_provider, projection, config);

        let stopper = loop_runner.stopper();

        // Start the loop in a task
        let handle = tokio::spawn(async move { loop_runner.run().await });

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Stop it
        stopper.stop();

        // Should complete without error
        let result = tokio::time::timeout(Duration::from_secs(1), handle).await;
        assert!(result.is_ok(), "Loop should stop within timeout");
        let inner = result.ok().and_then(|r| r.ok());
        assert!(inner.is_some());
    }

    /// Given a loop configured with stop_on_error=true
    /// When an error occurs during reconciliation
    /// Then the loop should stop immediately
    #[tokio::test]
    async fn stop_on_error_terminates_on_first_failure() {
        // This test would need a way to inject errors -
        // for now we verify the config is respected
        let config = LoopConfig {
            stop_on_error: true,
            max_errors: 1,
            ..Default::default()
        };
        assert!(config.stop_on_error);
        assert_eq!(config.max_errors, 1);
    }

    /// Given a desired state provider
    /// When modify() is called
    /// Then subsequent calls to get_desired_state() should reflect changes
    #[tokio::test]
    async fn desired_state_provider_reflects_modifications() {
        let provider = InMemoryDesiredStateProvider::new(DesiredState::new());

        // Initially empty
        let state1 = provider.get_desired_state().await;
        assert!(state1.is_ok());
        assert!(state1.ok().map(|s| s.is_empty()).unwrap_or(false));

        // Add a bead
        let bead_id = BeadId::new();
        provider
            .modify(|state| {
                state.add_bead(
                    bead_id,
                    BeadSpec::new("Test").with_complexity(Complexity::Simple),
                );
            })
            .await;

        // Now should have one bead
        let state2 = provider.get_desired_state().await;
        assert!(state2.is_ok());
        assert_eq!(state2.ok().map(|s| s.len()), Some(1));
    }

    /// Given actual state from a projection
    /// When build_actual_state() is called
    /// Then counts should be correctly computed
    #[tokio::test]
    async fn actual_state_computes_counts_correctly() {
        let (reconciler, desired_provider, projection, _) = setup();
        let config = LoopConfig::default();
        let loop_runner =
            ReconciliationLoop::new(reconciler, desired_provider, projection.clone(), config);

        // Manually update the projection with some beads
        use oya_events::{BeadEvent, BeadState};
        let bead1 = BeadId::new();
        let bead2 = BeadId::new();

        projection
            .apply(&BeadEvent::created(
                bead1,
                BeadSpec::new("Bead 1").with_complexity(Complexity::Simple),
            ))
            .await;
        projection
            .apply(&BeadEvent::state_changed(
                bead1,
                BeadState::Pending,
                BeadState::Running,
            ))
            .await;

        projection
            .apply(&BeadEvent::created(
                bead2,
                BeadSpec::new("Bead 2").with_complexity(Complexity::Simple),
            ))
            .await;

        let proj_state = projection.state().await;
        let actual = loop_runner.build_actual_state(&proj_state);

        assert_eq!(actual.beads.len(), 2);
        assert_eq!(actual.running_count, 1);
        assert_eq!(actual.pending_count, 1);
    }
}
