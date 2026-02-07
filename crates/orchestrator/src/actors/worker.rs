#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::watch;
use tracing::{debug, info, warn};

use oya_events::{BeadState, EventBus};
use oya_pipeline::workspace::WorkspaceManager;

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
    pub workspace_manager: Option<Arc<WorkspaceManager>>,
}

impl std::fmt::Debug for WorkerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerConfig")
            .field("checkpoint_interval", &self.checkpoint_interval)
            .field("retry_policy", &self.retry_policy)
            .field("event_bus", &self.event_bus.as_ref().map(|_| "<EventBus>"))
            .field(
                "workspace_manager",
                &self
                    .workspace_manager
                    .as_ref()
                    .map(|_| "<WorkspaceManager>"),
            )
            .finish()
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval: Duration::from_secs(60),
            retry_policy: WorkerRetryPolicy::default(),
            event_bus: None,
            workspace_manager: None,
        }
    }
}

impl WorkerConfig {
    #[must_use]
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    #[must_use]
    pub fn with_workspace_manager(mut self, manager: Arc<WorkspaceManager>) -> Self {
        self.workspace_manager = Some(manager);
        self
    }
}

/// Result of executing a bead in an isolated workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceExecutionResult {
    succeeded: bool,
    error: Option<String>,
    exit_code: Option<i32>,
}

impl WorkspaceExecutionResult {
    /// Create a successful execution result.
    #[must_use]
    pub fn success() -> Self {
        Self {
            succeeded: true,
            error: None,
            exit_code: Some(0),
        }
    }

    /// Create a failed execution result.
    #[must_use]
    pub fn failure(error: String) -> Self {
        Self {
            succeeded: false,
            error: Some(error),
            exit_code: Some(1),
        }
    }

    /// Create a result from an exit code.
    #[must_use]
    pub fn from_exit_code(code: i32) -> Self {
        Self {
            succeeded: code == 0,
            error: if code == 0 {
                None
            } else {
                Some(format!("exit code: {code}"))
            },
            exit_code: Some(code),
        }
    }

    /// Whether the execution succeeded.
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.succeeded
    }

    /// Error message if execution failed.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Exit code from execution.
    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}

/// Execution context for a bead running in an isolated workspace.
#[derive(Debug, Clone)]
pub struct BeadExecutionContext {
    bead_id: String,
    workspace_path: PathBuf,
    execution_result: Option<WorkspaceExecutionResult>,
}

impl BeadExecutionContext {
    /// Create a new execution context.
    #[must_use]
    pub fn new(bead_id: impl Into<String>, workspace_path: PathBuf) -> Self {
        Self {
            bead_id: bead_id.into(),
            workspace_path,
            execution_result: None,
        }
    }

    /// The bead ID being executed.
    #[must_use]
    pub fn bead_id(&self) -> &str {
        &self.bead_id
    }

    /// Path to the isolated workspace.
    #[must_use]
    pub fn workspace_path(&self) -> &PathBuf {
        &self.workspace_path
    }

    /// Whether execution has completed.
    #[must_use]
    pub fn has_completed(&self) -> bool {
        self.execution_result.is_some()
    }

    /// The execution result if available.
    #[must_use]
    pub fn execution_result(&self) -> Option<&WorkspaceExecutionResult> {
        self.execution_result.as_ref()
    }

    /// Mark execution as completed with a result.
    ///
    /// # Errors
    /// Returns an error if the context is already marked as completed.
    pub fn mark_completed(&mut self, result: WorkspaceExecutionResult) -> Result<(), String> {
        if self.execution_result.is_some() {
            return Err(format!(
                "bead '{}' execution already completed",
                self.bead_id
            ));
        }
        self.execution_result = Some(result);
        Ok(())
    }
}

/// Messages handled by the BeadWorker actor.
#[derive(Clone, Debug)]
pub enum WorkerMessage {
    StartBead {
        bead_id: String,
        from_state: Option<BeadState>,
    },
    FailBead {
        error: String,
    },
    CheckpointTick,
    HealthCheckFailed {
        reason: String,
    },
    Stop {
        reason: Option<String>,
    },
}

/// State for the BeadWorker actor.
pub struct WorkerState {
    worker_id: String,
    current_bead: Option<String>,
    current_state: Option<BeadState>,
    retry_attempts: u32,
    config: WorkerConfig,
    checkpoint_handle: Option<CheckpointHandle>,
    execution_context: Option<BeadExecutionContext>,
}

impl WorkerState {
    #[must_use]
    pub fn new(config: WorkerConfig) -> Self {
        let worker_id = format!("worker-{}", uuid::Uuid::new_v4());
        Self {
            worker_id,
            current_bead: None,
            current_state: None,
            retry_attempts: 0,
            config,
            checkpoint_handle: None,
            execution_context: None,
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

    fn clear_execution_context(&mut self) {
        self.execution_context = None;
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
            WorkerMessage::StartBead {
                bead_id,
                from_state,
            } => {
                let old_state = from_state.unwrap_or(BeadState::Ready);
                let new_state = BeadState::Running;

                // Create workspace for bead execution
                let exec_result = if let Some(ref workspace_manager) =
                    state.config.workspace_manager
                {
                    match workspace_manager.execute_with_workspace(&bead_id, |_workspace_path| {
                        // TODO: Execute actual bead work here
                        // For now, simulate successful execution
                        info!(bead_id = %bead_id, "Executing bead in isolated workspace");
                        Ok::<(), oya_pipeline::error::Error>(())
                    }) {
                        Ok(()) => {
                            info!(bead_id = %bead_id, "Bead execution completed successfully");
                            WorkspaceExecutionResult::success()
                        }
                        Err(e) => {
                            warn!(bead_id = %bead_id, error = %e, "Bead execution failed");
                            WorkspaceExecutionResult::failure(e.to_string())
                        }
                    }
                } else {
                    warn!(bead_id = %bead_id, "No workspace manager configured, skipping execution");
                    WorkspaceExecutionResult::success()
                };

                state.current_bead = Some(bead_id.clone());
                state.current_state = Some(new_state);
                state.reset_retries();

                // Clear any previous execution context
                state.clear_execution_context();
            }
            WorkerMessage::FailBead { error } => {
                let bead_id = state.current_bead.clone();
                let delay = state.next_retry_delay();

                match (bead_id, delay) {
                    (Some(id), Some(delay)) => {
                        warn!(bead_id = %id, error = %error, delay_ms = delay.as_millis(), "Retrying bead after failure");
                        let myself_clone = myself.clone();
                        let from_state = state.current_state.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(delay).await;
                            let _ = myself_clone.send_message(WorkerMessage::StartBead {
                                bead_id: id,
                                from_state,
                            });
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
            WorkerMessage::HealthCheckFailed { reason } => {
                warn!(worker_id = %state.worker_id, reason = %reason, "Health check failed");

                // Emit WorkerUnhealthy event if event bus is available
                if let Some(ref event_bus) = state.config.event_bus {
                    let event = oya_events::BeadEvent::worker_unhealthy(
                        state.worker_id.clone(),
                        reason.clone(),
                    );

                    // Publish the event asynchronously
                    let event_bus_clone = event_bus.clone();
                    tokio::spawn(async move {
                        if let Err(err) = event_bus_clone.publish(event).await {
                            tracing::error!(
                                error = %err,
                                "Failed to publish WorkerUnhealthy event"
                            );
                        }
                    });
                }

                // Fail the current bead if one is active
                if let Some(ref bead_id) = state.current_bead {
                    warn!(bead_id = %bead_id, "Marking bead as unhealthy due to health check failure");
                    let _ = myself.send_message(WorkerMessage::FailBead {
                        error: reason.clone(),
                    });
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

    #[test]
    fn test_workspace_execution_result_success() {
        let result = WorkspaceExecutionResult::success();
        assert!(result.succeeded());
        assert!(result.error().is_none());
        assert_eq!(result.exit_code(), Some(0));
    }

    #[test]
    fn test_workspace_execution_result_failure() {
        let result = WorkspaceExecutionResult::failure("command failed".to_string());
        assert!(!result.succeeded());
        assert_eq!(result.error(), Some("command failed"));
        assert_eq!(result.exit_code(), Some(1));
    }

    #[test]
    fn test_workspace_execution_result_from_exit_code() {
        let success = WorkspaceExecutionResult::from_exit_code(0);
        assert!(success.succeeded());

        let failure = WorkspaceExecutionResult::from_exit_code(1);
        assert!(!failure.succeeded());
    }

    #[test]
    fn test_bead_execution_context_new() {
        let ctx = BeadExecutionContext::new(
            "test-bead-123",
            PathBuf::from("/tmp/workspace/test-bead-123"),
        );

        assert_eq!(ctx.bead_id(), "test-bead-123");
        assert_eq!(
            *ctx.workspace_path(),
            PathBuf::from("/tmp/workspace/test-bead-123")
        );
        assert!(!ctx.has_completed());
    }

    #[test]
    fn test_bead_execution_context_mark_completed() {
        let mut ctx = BeadExecutionContext::new(
            "test-bead-456",
            PathBuf::from("/tmp/workspace/test-bead-456"),
        );

        assert!(!ctx.has_completed());

        let result = WorkspaceExecutionResult::success();
        ctx.mark_completed(result);

        assert!(ctx.has_completed());
        assert!(ctx.execution_result().unwrap().succeeded());
    }

    #[test]
    fn test_bead_execution_context_double_complete_returns_error() {
        let mut ctx = BeadExecutionContext::new(
            "test-bead-789",
            PathBuf::from("/tmp/workspace/test-bead-789"),
        );

        ctx.mark_completed(WorkspaceExecutionResult::success());

        let second_result =
            ctx.mark_completed(WorkspaceExecutionResult::failure("fail".to_string()));
        assert!(second_result.is_err());
    }
}
