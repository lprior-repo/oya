//! Reconciler implementation.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use oya_events::{BeadEvent, BeadState, EventBus};
use tracing::{debug, info, warn};

use crate::error::{Error, Result};
use crate::types::{ActualState, DesiredState, ReconcileAction, ReconcileResult};

/// Configuration for the reconciler.
#[derive(Debug, Clone)]
pub struct ReconcilerConfig {
    /// Maximum concurrent running beads.
    pub max_concurrent: usize,
    /// Whether to auto-start scheduled beads.
    pub auto_start: bool,
    /// Whether to auto-retry failed beads.
    pub auto_retry: bool,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Whether to detect dead workers.
    pub detect_dead_workers: bool,
    /// Duration before an unclaimed running bead is treated as dead.
    pub dead_worker_threshold: Duration,
    /// Whether to detect stuck beads.
    pub detect_stuck_beads: bool,
    /// Duration before a running bead is considered stuck.
    pub stuck_bead_threshold: Duration,
}

impl Default for ReconcilerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            auto_start: true,
            auto_retry: true,
            max_retries: 3,
            detect_dead_workers: true,
            dead_worker_threshold: Duration::from_secs(60),
            detect_stuck_beads: true,
            stuck_bead_threshold: Duration::from_secs(300),
        }
    }
}

/// Trait for executing reconcile actions.
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Execute an action.
    async fn execute(&self, action: &ReconcileAction) -> Result<()>;
}

/// Event-based action executor that publishes events.
pub struct EventActionExecutor {
    bus: Arc<EventBus>,
}

impl EventActionExecutor {
    /// Create a new event action executor.
    pub fn new(bus: Arc<EventBus>) -> Self {
        Self { bus }
    }
}

#[async_trait]
impl ActionExecutor for EventActionExecutor {
    async fn execute(&self, action: &ReconcileAction) -> Result<()> {
        match action {
            ReconcileAction::CreateBead { bead_id, spec } => {
                self.bus
                    .publish(BeadEvent::created(*bead_id, spec.clone()))
                    .await
                    .map_err(|e| Error::event_error(e.to_string()))?;
            }
            ReconcileAction::StartBead { bead_id } => {
                self.bus
                    .publish(BeadEvent::state_changed(
                        *bead_id,
                        BeadState::Ready,
                        BeadState::Running,
                    ))
                    .await
                    .map_err(|e| Error::event_error(e.to_string()))?;
            }
            ReconcileAction::StopBead { bead_id, reason } => {
                self.bus
                    .publish(BeadEvent::state_changed_with_reason(
                        *bead_id,
                        BeadState::Running,
                        BeadState::Paused,
                        reason,
                    ))
                    .await
                    .map_err(|e| Error::event_error(e.to_string()))?;
            }
            ReconcileAction::RetryBead { bead_id } => {
                self.bus
                    .publish(BeadEvent::state_changed(
                        *bead_id,
                        BeadState::BackingOff,
                        BeadState::Running,
                    ))
                    .await
                    .map_err(|e| Error::event_error(e.to_string()))?;
            }
            ReconcileAction::MarkComplete { bead_id, result } => {
                self.bus
                    .publish(BeadEvent::completed(*bead_id, result.clone()))
                    .await
                    .map_err(|e| Error::event_error(e.to_string()))?;
            }
            ReconcileAction::ScheduleBead { bead_id } => {
                self.bus
                    .publish(BeadEvent::state_changed(
                        *bead_id,
                        BeadState::Pending,
                        BeadState::Scheduled,
                    ))
                    .await
                    .map_err(|e| Error::event_error(e.to_string()))?;
            }
            ReconcileAction::UpdateDependencies { .. }
            | ReconcileAction::DeleteBead { .. }
            | ReconcileAction::RescheduleBead { .. }
            | ReconcileAction::RespawnBead { .. }
            | ReconcileAction::CancelBead { .. } => {
                // These would require additional event types or storage operations
                debug!(action = ?action, "Action not implemented via events");
            }
        }
        Ok(())
    }
}

/// K8s-style reconciler for bead management.
pub struct Reconciler {
    /// Event bus for coordination.
    bus: Arc<EventBus>,
    /// Action executor.
    executor: Arc<dyn ActionExecutor>,
    /// Configuration.
    config: ReconcilerConfig,
}

impl Reconciler {
    /// Create a new reconciler.
    pub fn new(
        bus: Arc<EventBus>,
        executor: Arc<dyn ActionExecutor>,
        config: ReconcilerConfig,
    ) -> Self {
        Self {
            bus,
            executor,
            config,
        }
    }

    /// Create a reconciler with default event executor.
    pub fn with_event_executor(bus: Arc<EventBus>, config: ReconcilerConfig) -> Self {
        let executor = Arc::new(EventActionExecutor::new(bus.clone()));
        Self::new(bus, executor, config)
    }

    /// Core reconciliation: compare desired vs actual and generate actions.
    pub async fn reconcile(
        &self,
        desired: &DesiredState,
        actual: &ActualState,
    ) -> Result<ReconcileResult> {
        self.log_reconciliation_start(desired, actual);

        let actions = self.diff(desired, actual);
        debug!(actions = actions.len(), "Generated actions");

        let (taken, failed) = self.apply_actions(actions).await;
        let result = ReconcileResult::new(taken, failed, desired.len(), actual.beads.len());

        self.log_reconciliation_complete(&result);
        Ok(result)
    }

    fn log_reconciliation_start(&self, desired: &DesiredState, actual: &ActualState) {
        info!(
            desired = desired.len(),
            actual = actual.beads.len(),
            running = actual.running_count,
            "Starting reconciliation"
        );
    }

    fn log_reconciliation_complete(&self, result: &ReconcileResult) {
        if result.converged {
            info!("System converged");
        } else {
            info!(
                actions_taken = result.actions_taken.len(),
                actions_failed = result.actions_failed.len(),
                "Reconciliation complete"
            );
        }
    }

    /// Compute diff between desired and actual state.
    fn diff(&self, desired: &DesiredState, actual: &ActualState) -> Vec<ReconcileAction> {
        let mut actions = Vec::new();
        let dead_worker_threshold = ChronoDuration::from_std(self.config.dead_worker_threshold)
            .map_err(|_| ())
            .ok();
        let stuck_bead_threshold = ChronoDuration::from_std(self.config.stuck_bead_threshold)
            .map_err(|_| ())
            .ok();

        // 1. Create beads that exist in desired but not actual
        for (bead_id, spec) in &desired.beads {
            if !actual.beads.contains_key(bead_id) {
                actions.push(ReconcileAction::CreateBead {
                    bead_id: *bead_id,
                    spec: spec.clone(),
                });
            }
        }

        // 2. Detect orphaned beads (actual without desired) and delete
        let orphaned = actual.orphaned_beads(desired);
        if !orphaned.is_empty() {
            warn!(count = orphaned.len(), "Detected orphaned beads");
        }
        for proj in orphaned {
            actions.push(ReconcileAction::DeleteBead {
                bead_id: proj.bead_id,
            });
        }

        // 3. Schedule pending beads whose dependencies are met
        for (bead_id, proj) in &actual.beads {
            if proj.current_state == BeadState::Pending && !proj.is_blocked() {
                actions.push(ReconcileAction::ScheduleBead { bead_id: *bead_id });
            }
        }

        // 4. Auto-start scheduled beads (if below concurrency limit)
        if self.config.auto_start && actual.running_count < self.config.max_concurrent {
            let ready = actual.ready_to_run();
            let slots_available = self.config.max_concurrent - actual.running_count;

            for proj in ready.into_iter().take(slots_available) {
                actions.push(ReconcileAction::StartBead {
                    bead_id: proj.bead_id,
                });
            }
        }

        // 5. Retry backed-off beads (if auto-retry enabled)
        if self.config.auto_retry {
            for (bead_id, proj) in &actual.beads {
                if proj.current_state == BeadState::BackingOff {
                    actions.push(ReconcileAction::RetryBead { bead_id: *bead_id });
                }
            }
        }

        // 6. Detect dead workers (running without claim beyond threshold)
        if self.config.detect_dead_workers {
            if let Some(threshold) = dead_worker_threshold {
                for proj in actual.beads.values() {
                    let is_unclaimed_running =
                        proj.current_state == BeadState::Running && proj.claimed_by.is_none();
                    let running_long_enough = self
                        .running_duration(proj)
                        .map(|elapsed| elapsed >= threshold)
                        .unwrap_or_else(|| false);

                    if is_unclaimed_running && running_long_enough {
                        actions.push(ReconcileAction::RespawnBead {
                            bead_id: proj.bead_id,
                            reason: format!("worker missing for {}s", threshold.num_seconds()),
                        });
                    }
                }
            } else {
                warn!("dead worker threshold invalid; skipping detection");
            }
        }

        // 7. Detect stuck beads (running beyond threshold)
        if self.config.detect_stuck_beads {
            if let Some(threshold) = stuck_bead_threshold {
                for proj in actual.beads.values() {
                    let is_running =
                        proj.current_state == BeadState::Running && proj.claimed_by.is_some();
                    let running_long_enough = self
                        .running_duration(proj)
                        .map(|elapsed| elapsed >= threshold)
                        .unwrap_or_else(|| false);

                    if is_running && running_long_enough {
                        actions.push(ReconcileAction::RescheduleBead {
                            bead_id: proj.bead_id,
                            reason: format!("running for {}s", threshold.num_seconds()),
                        });
                    }
                }
            } else {
                warn!("stuck bead threshold invalid; skipping detection");
            }
        }

        actions
    }

    fn running_duration(&self, proj: &oya_events::BeadProjection) -> Option<ChronoDuration> {
        let running_transition = proj
            .history
            .iter()
            .rev()
            .find(|transition| transition.to == BeadState::Running);

        running_transition.map(|transition| Utc::now().signed_duration_since(transition.timestamp))
    }

    /// Apply a list of actions.
    async fn apply_actions(
        &self,
        actions: Vec<ReconcileAction>,
    ) -> (Vec<ReconcileAction>, Vec<(ReconcileAction, String)>) {
        let mut taken = Vec::new();
        let mut failed = Vec::new();

        for action in actions {
            debug!(action = ?action, "Applying action");

            match self.executor.execute(&action).await {
                Ok(()) => {
                    taken.push(action);
                }
                Err(e) => {
                    warn!(action = ?action, error = %e, "Action failed");
                    failed.push((action, e.to_string()));
                }
            }
        }

        (taken, failed)
    }

    /// Get the event bus.
    pub fn bus(&self) -> &Arc<EventBus> {
        &self.bus
    }

    /// Get the configuration.
    pub fn config(&self) -> &ReconcilerConfig {
        &self.config
    }
}

/// Builder for Reconciler.
pub struct ReconcilerBuilder {
    bus: Option<Arc<EventBus>>,
    executor: Option<Arc<dyn ActionExecutor>>,
    config: ReconcilerConfig,
}

impl ReconcilerBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            bus: None,
            executor: None,
            config: ReconcilerConfig::default(),
        }
    }

    /// Set the event bus.
    pub fn with_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.bus = Some(bus);
        self
    }

    /// Set a custom action executor.
    pub fn with_executor(mut self, executor: Arc<dyn ActionExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Set the configuration.
    pub fn with_config(mut self, config: ReconcilerConfig) -> Self {
        self.config = config;
        self
    }

    /// Set max concurrent beads.
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.config.max_concurrent = max;
        self
    }

    /// Enable/disable auto-start.
    pub fn auto_start(mut self, enabled: bool) -> Self {
        self.config.auto_start = enabled;
        self
    }

    /// Build the reconciler.
    pub fn build(self) -> Result<Reconciler> {
        let bus = self
            .bus
            .ok_or_else(|| Error::invalid_config("Event bus is required"))?;

        let executor = self
            .executor
            .unwrap_or_else(|| Arc::new(EventActionExecutor::new(bus.clone())));

        Ok(Reconciler::new(bus, executor, self.config))
    }
}

impl Default for ReconcilerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_events::{BeadId, BeadSpec, Complexity, InMemoryEventStore, StateTransition};

    fn setup_reconciler() -> (Reconciler, Arc<EventBus>) {
        let store = Arc::new(InMemoryEventStore::new());
        let bus = Arc::new(EventBus::new(store));
        let reconciler = Reconciler::with_event_executor(bus.clone(), ReconcilerConfig::default());
        (reconciler, bus)
    }

    #[tokio::test]
    async fn test_reconcile_empty() {
        let (reconciler, _) = setup_reconciler();
        let desired = DesiredState::new();
        let actual = ActualState::new();

        let result = reconciler.reconcile(&desired, &actual).await;
        assert!(result.is_ok());
        let result = result.ok();
        assert!(result.as_ref().map(|r| r.converged).unwrap_or(false));
    }

    #[tokio::test]
    async fn test_reconcile_creates_beads() {
        let (reconciler, _) = setup_reconciler();
        let mut desired = DesiredState::new();
        let bead_id = BeadId::new();
        desired.add_bead(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );

        let actual = ActualState::new();

        let result = reconciler.reconcile(&desired, &actual).await;
        assert!(result.is_ok());
        let result = result.ok();

        // Should have a CreateBead action
        let has_create = result
            .as_ref()
            .map(|r| {
                r.actions_taken
                    .iter()
                    .any(|a| matches!(a, ReconcileAction::CreateBead { .. }))
            })
            .unwrap_or(false);
        assert!(has_create);
    }

    #[tokio::test]
    async fn test_diff_detects_missing_beads() {
        let (reconciler, _) = setup_reconciler();
        let mut desired = DesiredState::new();
        let bead_id = BeadId::new();
        desired.add_bead(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );

        let actual = ActualState::new();
        let actions = reconciler.diff(&desired, &actual);

        assert!(!actions.is_empty());
        assert!(actions
            .iter()
            .any(|a| matches!(a, ReconcileAction::CreateBead { .. })));
    }

    #[tokio::test]
    async fn test_diff_detects_orphaned_beads() {
        let (reconciler, _) = setup_reconciler();
        let desired = DesiredState::new();
        let mut actual = ActualState::new();

        let mut proj = oya_events::BeadProjection::new(BeadId::new());
        proj.current_state = BeadState::Running;
        actual.update(proj);

        let actions = reconciler.diff(&desired, &actual);

        let has_delete = actions
            .iter()
            .any(|a| matches!(a, ReconcileAction::DeleteBead { .. }));
        assert!(has_delete);
    }

    #[tokio::test]
    async fn test_diff_detects_dead_workers() {
        let config = ReconcilerConfig {
            detect_dead_workers: true,
            dead_worker_threshold: Duration::from_secs(30),
            ..Default::default()
        };

        let store = Arc::new(InMemoryEventStore::new());
        let bus = Arc::new(EventBus::new(store));
        let reconciler = Reconciler::with_event_executor(bus, config);

        let desired = DesiredState::new();
        let mut actual = ActualState::new();
        let bead_id = BeadId::new();
        let mut proj = oya_events::BeadProjection::new(bead_id);
        proj.current_state = BeadState::Running;
        proj.claimed_by = None;
        proj.history.push(StateTransition {
            from: BeadState::Ready,
            to: BeadState::Running,
            timestamp: Utc::now() - ChronoDuration::seconds(120),
            reason: None,
        });
        actual.update(proj);

        let actions = reconciler.diff(&desired, &actual);
        let has_respawn = actions
            .iter()
            .any(|a| matches!(a, ReconcileAction::RespawnBead { .. }));
        assert!(has_respawn);
    }

    #[tokio::test]
    async fn test_diff_detects_stuck_beads() {
        let config = ReconcilerConfig {
            detect_stuck_beads: true,
            stuck_bead_threshold: Duration::from_secs(60),
            ..Default::default()
        };

        let store = Arc::new(InMemoryEventStore::new());
        let bus = Arc::new(EventBus::new(store));
        let reconciler = Reconciler::with_event_executor(bus, config);

        let desired = DesiredState::new();
        let mut actual = ActualState::new();
        let bead_id = BeadId::new();
        let mut proj = oya_events::BeadProjection::new(bead_id);
        proj.current_state = BeadState::Running;
        proj.claimed_by = Some("agent-1".to_string());
        proj.history.push(StateTransition {
            from: BeadState::Ready,
            to: BeadState::Running,
            timestamp: Utc::now() - ChronoDuration::seconds(120),
            reason: None,
        });
        actual.update(proj);

        let actions = reconciler.diff(&desired, &actual);
        let has_reschedule = actions
            .iter()
            .any(|a| matches!(a, ReconcileAction::RescheduleBead { .. }));
        assert!(has_reschedule);
    }

    #[tokio::test]
    async fn test_diff_respects_concurrency_limit() {
        let config = ReconcilerConfig {
            max_concurrent: 1,
            auto_start: true,
            ..Default::default()
        };
        let store = Arc::new(InMemoryEventStore::new());
        let bus = Arc::new(EventBus::new(store));
        let reconciler = Reconciler::with_event_executor(bus, config);

        let desired = DesiredState::new();
        let mut actual = ActualState::new();

        // Add a running bead
        let mut proj1 = oya_events::BeadProjection::new(BeadId::new());
        proj1.current_state = BeadState::Running;
        actual.update(proj1);

        // Add a scheduled bead (should not start due to limit)
        let mut proj2 = oya_events::BeadProjection::new(BeadId::new());
        proj2.current_state = BeadState::Scheduled;
        actual.update(proj2);

        let actions = reconciler.diff(&desired, &actual);

        // Should not have any StartBead actions
        let has_start = actions
            .iter()
            .any(|a| matches!(a, ReconcileAction::StartBead { .. }));
        assert!(!has_start);
    }

    #[test]
    fn test_builder() {
        let store = Arc::new(InMemoryEventStore::new());
        let bus = Arc::new(EventBus::new(store));

        let result = ReconcilerBuilder::new()
            .with_bus(bus)
            .max_concurrent(5)
            .auto_start(false)
            .build();

        assert!(result.is_ok());
        let reconciler = result.ok();
        assert_eq!(
            reconciler.as_ref().map(|r| r.config.max_concurrent),
            Some(5)
        );
        assert_eq!(reconciler.map(|r| r.config.auto_start), Some(false));
    }
}
