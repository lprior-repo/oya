#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::watch;
use tracing::{debug, info, warn};

use oya_events::{BeadState, EventBus, BeadEvent, BeadId};

use crate::actors::supervisor::{GenericSupervisableActor, calculate_backoff};

/// Configuration for worker retry behavior.
#[derive(Debug, Clone)]
pub struct WorkerRetryPolicy {
    pub max_retries: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for WorkerRetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_backoff_ms: 100,
            max_backoff_ms: 3200,
        }
    }
}

impl WorkerRetryPolicy {
    #[must_use]
    pub fn next_delay(&self, attempt: u32) -> Option<Duration> {
        if attempt > self.max_retries {
            None
        } else {
            Some(calculate_backoff(
                attempt.saturating_sub(1),
                self.base_backoff_ms,
                self.max_backoff_ms,
            ))
        }
    }
}

/// Worker configuration.
#[derive(Clone)]
pub struct WorkerConfig {
    pub checkpoint_interval: Duration,
    pub retry_policy: WorkerRetryPolicy,
    pub event_bus: Option<Arc<EventBus>>,
}

impl std::fmt::Debug for WorkerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerConfig")
            .field("checkpoint_interval", &self.checkpoint_interval)
            .field("retry_policy", &self.retry_policy)
            .field("event_bus", &self.event_bus.as_ref().map(|_| "<EventBus>"))
            .finish()
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval: Duration::from_secs(60),
            retry_policy: WorkerRetryPolicy::default(),
            event_bus: None,
        }
    }
}

impl WorkerConfig {
    #[must_use]
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }
}

/// Messages handled by the BeadWorker actor.
#[derive(Clone, Debug)]
pub enum WorkerMessage {
    StartBead { bead_id: String, from_state: Option<BeadState> },
    FailBead { error: String },
    CheckpointTick,
    Stop { reason: Option<String> },
}

/// State for the BeadWorker actor.
pub struct WorkerState {
    current_bead: Option<String>,
    current_state: Option<BeadState>,
    retry_attempts: u32,
    config: WorkerConfig,
    checkpoint_handle: Option<CheckpointHandle>,
}

impl WorkerState {
    #[must_use]
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            current_bead: None,
            current_state: None,
            retry_attempts: 0,
            config,
            checkpoint_handle: None,
        }
    }

    fn reset_retries(&mut self) {
        self.retry_attempts = 0;
    }

    fn next_retry_delay(&mut self) -> Option<Duration> {
        self.retry_attempts = self.retry_attempts.saturating_add(1);
        self.config.retry_policy.next_delay(self.retry_attempts)
    }

    #[must_use]
    pub fn current_state(&self) -> Option<&BeadState> {
        self.current_state.as_ref()
    }
}

#[derive(Clone, Default)]
pub struct WorkerActorDef;

impl Actor for WorkerActorDef {
    type Msg = WorkerMessage;
    type State = WorkerState;
    type Arguments = WorkerConfig;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        config: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("BeadWorkerActor starting");
        let mut state = WorkerState::new(config);
        let handle = CheckpointTimer::start(myself.clone(), state.config.checkpoint_interval);
        state.checkpoint_handle = Some(handle);
        Ok(state)
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            WorkerMessage::StartBead { bead_id, from_state } => {
                let old_state = from_state.unwrap_or(BeadState::Ready);
                let new_state = BeadState::Running;

                // Emit state change event (best-effort)
                if let Some(ref bus) = state.config.event_bus {
                    // Parse bead_id as BeadId (ULID)
                    let result: Result<BeadId, _> = bead_id.parse();
                    if let Ok(bid) = result {
                        let event = BeadEvent::state_changed(bid, old_state.clone(), new_state.clone());
                        match bus.publish(event).await {
                            Ok(_) => {
                                debug!(
                                    bead_id = %bead_id,
                                    from = ?old_state,
                                    to = ?new_state,
                                    "Emitted state change event"
                                );
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to publish state change event");
                            }
                        }
                    } else {
                        warn!(bead_id = %bead_id, "Invalid bead ID format, skipping event emission");
                    }
                }

                state.current_bead = Some(bead_id);
                state.current_state = Some(new_state);
                state.reset_retries();
            }
            WorkerMessage::FailBead { error } => {
                let bead_id = state.current_bead.clone();
                let delay = state.next_retry_delay();

                // Emit failed event (best-effort)
                if let (Some(bus), Some(bid)) = (&state.config.event_bus, &bead_id) {
                    let result: Result<BeadId, _> = bid.parse();
                    if let Ok(event_bead_id) = result {
                        let event = BeadEvent::failed(event_bead_id, error.clone());
                        match bus.publish(event).await {
                            Ok(_) => {
                                debug!(bead_id = %bid, error = %error, "Emitted failed event");
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to publish failed event");
                            }
                        }
                    } else {
                        warn!(bead_id = %bid, "Invalid bead ID format, skipping event emission");
                    }
                }

                match (bead_id, delay) {
                    (Some(id), Some(delay)) => {
                        warn!(bead_id = %id, error = %error, delay_ms = delay.as_millis(), "Retrying bead after failure");
                        let myself_clone = myself.clone();
                        let from_state = state.current_state.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(delay).await;
                            let _ =
                                myself_clone.send_message(WorkerMessage::StartBead { bead_id: id, from_state });
                        });
                    }
                    (Some(id), None) => {
                        warn!(bead_id = %id, error = %error, "Max retries exceeded, giving up");
                    }
                    (None, _) => {
                        debug!(error = %error, "Failure received with no active bead");
                    }
                }
            }
            WorkerMessage::CheckpointTick => {
                if let Some(ref bead_id) = state.current_bead {
                    debug!(bead_id = %bead_id, "Checkpoint timer fired");
                }
            }
            WorkerMessage::Stop { reason } => {
                let reason_text = reason.unwrap_or_else(|| "shutdown".to_string());
                info!(reason = %reason_text, "BeadWorkerActor stopping");
                if let Some(handle) = state.checkpoint_handle.take() {
                    handle.stop();
                }
            }
        }
        Ok(())
    }
}

impl GenericSupervisableActor for WorkerActorDef {
    fn default_args() -> Self::Arguments {
        WorkerConfig::default()
    }
}

/// Handle for stopping a checkpoint timer.
#[derive(Clone)]
pub struct CheckpointHandle {
    stop_tx: watch::Sender<bool>,
}

impl CheckpointHandle {
    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }
}

/// Checkpoint timer that sends tick messages at a fixed interval.
#[derive(Clone, Debug)]
pub struct CheckpointTimer {
    interval: Duration,
}

impl CheckpointTimer {
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }

    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn start(target: ActorRef<WorkerMessage>, interval: Duration) -> CheckpointHandle {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let mut ticker = tokio::time::interval(interval);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if target.send_message(WorkerMessage::CheckpointTick).is_err() {
                            break;
                        }
                    }
                    changed = stop_rx.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        });

        CheckpointHandle { stop_tx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_respects_max_retries() {
        let policy = WorkerRetryPolicy {
            max_retries: 2,
            base_backoff_ms: 100,
            max_backoff_ms: 1000,
        };

        assert!(policy.next_delay(1).is_some());
        assert!(policy.next_delay(2).is_some());
        assert!(policy.next_delay(3).is_none());
    }

    #[test]
    fn test_worker_config_defaults() {
        let config = WorkerConfig::default();
        assert_eq!(config.checkpoint_interval, Duration::from_secs(60));
        assert_eq!(config.retry_policy.max_retries, 3);
    }

    #[test]
    fn test_checkpoint_timer_interval() {
        let timer = CheckpointTimer::new(Duration::from_secs(60));
        assert_eq!(timer.interval(), Duration::from_secs(60));
    }
}
