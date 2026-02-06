#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::time::Duration;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::watch;
use tracing::{debug, info, warn};

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
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub checkpoint_interval: Duration,
    pub retry_policy: WorkerRetryPolicy,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval: Duration::from_secs(60),
            retry_policy: WorkerRetryPolicy::default(),
        }
    }
}

/// Messages handled by the BeadWorker actor.
#[derive(Clone, Debug)]
pub enum WorkerMessage {
    StartBead { bead_id: String },
    FailBead { error: String },
    CheckpointTick,
    Stop { reason: Option<String> },
}

/// State for the BeadWorker actor.
pub struct WorkerState {
    current_bead: Option<String>,
    retry_attempts: u32,
    config: WorkerConfig,
    checkpoint_handle: Option<CheckpointHandle>,
}

impl WorkerState {
    #[must_use]
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            current_bead: None,
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
            WorkerMessage::StartBead { bead_id } => {
                state.current_bead = Some(bead_id);
                state.reset_retries();
            }
            WorkerMessage::FailBead { error } => {
                let bead_id = state.current_bead.clone();
                let delay = state.next_retry_delay();
                match (bead_id, delay) {
                    (Some(id), Some(delay)) => {
                        warn!(bead_id = %id, error = %error, delay_ms = delay.as_millis(), "Retrying bead after failure");
                        let myself_clone = myself.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(delay).await;
                            let _ =
                                myself_clone.send_message(WorkerMessage::StartBead { bead_id: id });
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
