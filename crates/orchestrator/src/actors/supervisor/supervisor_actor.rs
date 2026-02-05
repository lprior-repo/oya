//! Supervisor module for scheduler actor management.
//!
//! This module provides supervisor patterns for scheduler actors,
//! including spawn helpers, supervision strategies, restart with
//! exponential backoff, and meltdown detection.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::shutdown::{ShutdownCoordinator, ShutdownSignal};

use super::super::errors::ActorError;
use super::super::messages::SchedulerMessage;
use super::super::scheduler::{SchedulerActorDef, SchedulerArguments};
use super::strategy::{OneForOne, RestartContext, RestartDecision, RestartStrategy};

/// Spawn a scheduler actor with a unique generated name.
pub async fn spawn_scheduler(
    args: SchedulerArguments,
) -> Result<ActorRef<SchedulerMessage>, ActorError> {
    let (actor_ref, _handle) = Actor::spawn(None, SchedulerActorDef, args)
        .await
        .map_err(|e| ActorError::SpawnFailed(format!("Failed to spawn scheduler: {}", e)))?;
    Ok(actor_ref)
}

/// Spawn a scheduler actor with a specific name.
pub async fn spawn_scheduler_with_name(
    args: SchedulerArguments,
    name: &str,
) -> Result<ActorRef<SchedulerMessage>, ActorError> {
    let (actor_ref, _handle) = Actor::spawn(Some(name.to_string()), SchedulerActorDef, args)
        .await
        .map_err(|e| {
            ActorError::SpawnFailed(format!("Failed to spawn scheduler '{}': {}", name, e))
        })?;
    Ok(actor_ref)
}

/// Error type for spawn operations.
#[derive(Debug, Clone)]
pub struct SpawnError(pub String);

impl std::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Spawn error: {}", self.0)
    }
}

impl std::error::Error for SpawnError {}

/// Status indicating supervisor meltdown conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeltdownStatus {
    /// Normal operation.
    Normal,
    /// High failure rate detected.
    Warning,
    /// Critical failure rate - meltdown triggered.
    Meltdown,
}

/// Configuration for scheduler supervision.
#[derive(Debug, Clone)]
pub struct SchedulerSupervisorConfig {
    /// Maximum restart attempts before giving up.
    pub max_restarts: u32,
    /// Time window for restart counting (seconds).
    pub restart_window_secs: u64,
    /// Base backoff duration in milliseconds.
    pub base_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds.
    pub max_backoff_ms: u64,
    /// Failure rate threshold for warning (failures per second).
    pub warning_threshold: f64,
    /// Failure rate threshold for meltdown (failures per second).
    pub meltdown_threshold: f64,
}

impl Default for SchedulerSupervisorConfig {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            restart_window_secs: 60,
            base_backoff_ms: 100,
            max_backoff_ms: 3200,
            warning_threshold: 0.5,
            meltdown_threshold: 1.0,
        }
    }
}

impl SchedulerSupervisorConfig {
    /// Create a config for testing with shorter timeouts.
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            max_restarts: 3,
            restart_window_secs: 5,
            base_backoff_ms: 10,
            max_backoff_ms: 320,
            warning_threshold: 1.0,
            meltdown_threshold: 2.0,
        }
    }
}

/// Messages for supervisor communication.
pub enum SupervisorMessage {
    /// Child actor exited.
    ChildExited {
        /// Name of the child that exited
        name: String,
        /// Reason for exit
        reason: String,
    },
    /// Request supervisor status.
    GetStatus {
        /// Reply channel
        reply: tokio::sync::oneshot::Sender<SupervisorStatus>,
    },
    /// Shutdown the supervisor.
    Shutdown,
    /// Spawn a new child scheduler.
    SpawnChild {
        /// Name for the child
        name: String,
        /// Arguments for the scheduler
        args: SchedulerArguments,
        /// Reply channel for result
        reply: tokio::sync::oneshot::Sender<Result<(), ActorError>>,
    },
    /// Stop a specific child.
    StopChild {
        /// Name of the child to stop
        name: String,
    },
}

impl std::fmt::Debug for SupervisorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChildExited { name, reason } => f
                .debug_struct("ChildExited")
                .field("name", name)
                .field("reason", reason)
                .finish(),
            Self::GetStatus { .. } => f.debug_struct("GetStatus").finish_non_exhaustive(),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::SpawnChild { name, .. } => f
                .debug_struct("SpawnChild")
                .field("name", name)
                .finish_non_exhaustive(),
            Self::StopChild { name } => f.debug_struct("StopChild").field("name", name).finish(),
        }
    }
}

/// Status response from supervisor.
#[derive(Debug, Clone)]
pub struct SupervisorStatus {
    /// Current state.
    pub state: SupervisorState,
    /// Meltdown status.
    pub meltdown_status: MeltdownStatus,
    /// Number of active children.
    pub active_children: usize,
    /// Total restarts performed.
    pub total_restarts: u32,
    /// Failures in current window.
    pub failures_in_window: u32,
}

/// State of the supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorState {
    /// Supervisor is running normally.
    Running,
    /// Supervisor is shutting down.
    ShuttingDown,
    /// Supervisor has stopped.
    Stopped,
}

/// Information about a supervised child.
pub struct ChildInfo {
    /// Child name.
    pub name: String,
    /// Actor reference.
    pub actor_ref: ActorRef<SchedulerMessage>,
    /// Number of restarts.
    pub restart_count: u32,
    /// Time of last restart.
    pub last_restart: Option<Instant>,
    /// Arguments used to spawn this child.
    pub args: SchedulerArguments,
}

impl std::fmt::Debug for ChildInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildInfo")
            .field("name", &self.name)
            .field("restart_count", &self.restart_count)
            .field("last_restart", &self.last_restart)
            .finish_non_exhaustive()
    }
}

/// Definition for the scheduler supervisor actor.
pub struct SchedulerSupervisorDef;

/// State for the supervisor actor.
pub struct SupervisorActorState {
    /// Configuration.
    pub config: SchedulerSupervisorConfig,
    /// Current state.
    pub state: SupervisorState,
    /// Supervised children.
    pub children: HashMap<String, ChildInfo>,
    /// Failure timestamps for meltdown detection.
    pub failure_times: Vec<Instant>,
    /// Total restarts performed.
    pub total_restarts: u32,
    /// Counter for generating unique child actor names.
    pub child_id_counter: u64,
    /// Shutdown coordinator reference.
    #[allow(dead_code)]
    pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
    /// Shutdown signal receiver.
    pub _shutdown_rx: Option<broadcast::Receiver<ShutdownSignal>>,
    /// Restart strategy (boxed to support different strategies at runtime).
    pub restart_strategy: Box<dyn RestartStrategy>,
}

impl SupervisorActorState {
    /// Generate a unique child actor name.
    fn next_child_name(&mut self, prefix: &str) -> String {
        let id = self.child_id_counter;
        self.child_id_counter = self.child_id_counter.saturating_add(1);
        format!("{}-{}", prefix, id)
    }
}

impl std::fmt::Debug for SupervisorActorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SupervisorActorState")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("children", &self.children)
            .field("failure_times", &self.failure_times)
            .field("total_restarts", &self.total_restarts)
            .field("child_id_counter", &self.child_id_counter)
            .field("restart_strategy", &self.restart_strategy.name())
            .finish_non_exhaustive()
    }
}

/// Arguments for the supervisor actor.
#[derive(Default)]
pub struct SupervisorArguments {
    /// Configuration.
    pub config: SchedulerSupervisorConfig,
    /// Optional shutdown coordinator.
    pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
}

impl SupervisorArguments {
    /// Create new arguments with default config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration.
    #[must_use]
    pub fn with_config(mut self, config: SchedulerSupervisorConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the shutdown coordinator.
    #[must_use]
    pub fn with_shutdown_coordinator(mut self, coordinator: Arc<ShutdownCoordinator>) -> Self {
        self.shutdown_coordinator = Some(coordinator);
        self
    }
}

impl Actor for SchedulerSupervisorDef {
    type Msg = SupervisorMessage;
    type State = SupervisorActorState;
    type Arguments = SupervisorArguments;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("Supervisor starting");

        let mut state = SupervisorActorState {
            config: args.config,
            state: SupervisorState::Running,
            children: HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: args.shutdown_coordinator.clone(),
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        };

        // Subscribe to shutdown signals
        if let Some(coordinator) = &args.shutdown_coordinator {
            let shutdown_rx = coordinator.subscribe();
            state._shutdown_rx = Some(shutdown_rx);

            // Spawn shutdown listener
            let myself_clone = myself.clone();
            let mut rx = coordinator.subscribe();
            tokio::spawn(async move {
                if rx.recv().await.is_ok() {
                    let _ = myself_clone.send_message(SupervisorMessage::Shutdown);
                }
            });

            debug!("Supervisor subscribed to shutdown coordinator");
        }

        Ok(state)
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisorMessage::ChildExited { name, reason } => {
                Self::handle_child_exited(myself, state, &name, &reason).await;
            }

            SupervisorMessage::GetStatus { reply } => {
                let status = Self::build_status(state);
                let _ = reply.send(status);
            }

            SupervisorMessage::Shutdown => {
                info!("Supervisor shutdown requested");
                state.state = SupervisorState::ShuttingDown;

                // Stop all children
                for (name, child) in &state.children {
                    debug!(child = %name, "Stopping child");
                    child
                        .actor_ref
                        .stop(Some("Supervisor shutting down".to_string()));
                }

                myself.stop(None);
            }

            SupervisorMessage::SpawnChild { name, args, reply } => {
                let result = Self::spawn_child(myself.clone(), state, name, args).await;
                let _ = reply.send(result);
            }

            SupervisorMessage::StopChild { name } => {
                Self::stop_child(state, &name);
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!(
            children = state.children.len(),
            total_restarts = state.total_restarts,
            "Supervisor stopped"
        );
        state.state = SupervisorState::Stopped;
        Ok(())
    }
}

impl SchedulerSupervisorDef {
    /// Handle a child exit event.
    async fn handle_child_exited(
        myself: ActorRef<SupervisorMessage>,
        state: &mut SupervisorActorState,
        name: &str,
        reason: &str,
    ) {
        warn!(child = %name, reason = %reason, "Child exited");

        // Record failure time for meltdown detection
        state.failure_times.push(Instant::now());
        Self::cleanup_old_failures(state);

        // Check meltdown status
        let meltdown_status = Self::check_meltdown(state);
        if meltdown_status == MeltdownStatus::Meltdown {
            error!("Meltdown detected! Too many failures in time window");
            state.state = SupervisorState::ShuttingDown;
            myself.stop(Some("Meltdown triggered".to_string()));
            return;
        }

        if meltdown_status == MeltdownStatus::Warning {
            warn!("High failure rate detected");
        }

        // Don't restart if shutting down
        if state.state == SupervisorState::ShuttingDown {
            debug!(child = %name, "Not restarting child - supervisor shutting down");
            state.children.remove(name);
            return;
        }

        // Use restart strategy to decide what to do
        let ctx = RestartContext::new(name, reason, state);
        let decision = state.restart_strategy.on_child_failure(&ctx);

        match decision {
            RestartDecision::Restart { child_names } => {
                for child_name in &child_names {
                    Self::schedule_child_restart(myself.clone(), state, child_name);
                }
            }
            RestartDecision::Stop => {
                warn!(
                    child = %name,
                    restart_count = ctx.restart_count(),
                    "Max restarts exceeded, stopping supervision"
                );
                state.children.remove(name);
            }
        }
    }

    /// Schedule a single child for restart with backoff.
    fn schedule_child_restart(
        myself: ActorRef<SupervisorMessage>,
        state: &mut SupervisorActorState,
        name: &str,
    ) {
        if let Some(child) = state.children.get(name) {
            let backoff = calculate_backoff(
                child.restart_count,
                state.config.base_backoff_ms,
                state.config.max_backoff_ms,
            );

            info!(
                child = %name,
                restart_count = child.restart_count,
                backoff_ms = %backoff.as_millis(),
                "Scheduling child restart via {} strategy",
                state.restart_strategy.name()
            );

            // Clone what we need for the async block
            let child_name = name.to_string();
            let child_args = child.args.clone();
            let myself_clone = myself.clone();

            // Schedule restart after backoff
            tokio::spawn(async move {
                tokio::time::sleep(backoff).await;

                // Create reply channel (we don't wait for result)
                let (tx, _rx) = tokio::sync::oneshot::channel();
                let _ = myself_clone.send_message(SupervisorMessage::SpawnChild {
                    name: child_name,
                    args: child_args,
                    reply: tx,
                });
            });

            // Update restart count
            if let Some(child) = state.children.get_mut(name) {
                child.restart_count = child.restart_count.saturating_add(1);
                child.last_restart = Some(Instant::now());
                state.total_restarts = state.total_restarts.saturating_add(1);
            }
        } else {
            debug!(child = %name, "Unknown child, skipping restart");
        }
    }

    /// Spawn a new child scheduler.
    async fn spawn_child(
        myself: ActorRef<SupervisorMessage>,
        state: &mut SupervisorActorState,
        name: String,
        args: SchedulerArguments,
    ) -> Result<(), ActorError> {
        // Check if child already exists and is alive
        if let Some(existing) = state.children.get(&name) {
            let status = existing.actor_ref.get_status();
            if matches!(
                status,
                ractor::ActorStatus::Starting
                    | ractor::ActorStatus::Running
                    | ractor::ActorStatus::Upgrading
            ) {
                return Err(ActorError::SpawnFailed(format!(
                    "Child '{}' already exists and is running",
                    name
                )));
            }
        }

        // Generate unique actor name to avoid collisions with other supervisors/tests
        let actor_name = state.next_child_name(&name);

        let (actor_ref, handle) =
            Actor::spawn(Some(actor_name.clone()), SchedulerActorDef, args.clone())
                .await
                .map_err(|e| {
                    ActorError::SpawnFailed(format!(
                        "Failed to spawn '{}' (actor: {}): {}",
                        name, actor_name, e
                    ))
                })?;

        // Monitor for exit
        let myself_clone = myself.clone();
        let child_name = name.clone();
        tokio::spawn(async move {
            let _ = handle.await;
            let _ = myself_clone.send_message(SupervisorMessage::ChildExited {
                name: child_name,
                reason: "Actor exited".to_string(),
            });
        });

        let restart_count = state
            .children
            .get(&name)
            .map(|c| c.restart_count)
            .unwrap_or(0);

        state.children.insert(
            name.clone(),
            ChildInfo {
                name,
                actor_ref,
                restart_count,
                last_restart: Some(Instant::now()),
                args,
            },
        );

        Ok(())
    }

    /// Stop a specific child.
    fn stop_child(state: &mut SupervisorActorState, name: &str) {
        if let Some(child) = state.children.remove(name) {
            debug!(child = %name, "Stopping child");
            child
                .actor_ref
                .stop(Some("Requested by supervisor".to_string()));
        }
    }

    /// Clean up old failure timestamps.
    fn cleanup_old_failures(state: &mut SupervisorActorState) {
        let window = Duration::from_secs(state.config.restart_window_secs);
        state.failure_times.retain(|t| t.elapsed() < window);
    }

    /// Check meltdown status based on failure rate.
    fn check_meltdown(state: &SupervisorActorState) -> MeltdownStatus {
        if state.failure_times.is_empty() {
            return MeltdownStatus::Normal;
        }

        let window_secs = state.config.restart_window_secs as f64;
        let failure_rate = state.failure_times.len() as f64 / window_secs;

        if failure_rate >= state.config.meltdown_threshold {
            MeltdownStatus::Meltdown
        } else if failure_rate >= state.config.warning_threshold {
            MeltdownStatus::Warning
        } else {
            MeltdownStatus::Normal
        }
    }

    /// Build a status response.
    fn build_status(state: &SupervisorActorState) -> SupervisorStatus {
        SupervisorStatus {
            state: state.state,
            meltdown_status: Self::check_meltdown(state),
            active_children: state.children.len(),
            total_restarts: state.total_restarts,
            failures_in_window: state.failure_times.len() as u32,
        }
    }
}

/// Calculate exponential backoff duration.
#[must_use]
pub fn calculate_backoff(attempt: u32, base_ms: u64, max_ms: u64) -> Duration {
    let backoff = base_ms.saturating_mul(2u64.saturating_pow(attempt));
    Duration::from_millis(backoff.min(max_ms))
}

/// Spawn a supervised scheduler actor.
pub async fn spawn_supervised_scheduler(
    args: SchedulerArguments,
    _config: SchedulerSupervisorConfig,
) -> Result<ActorRef<SchedulerMessage>, ActorError> {
    // For now, just spawn without full supervision (individual spawn)
    spawn_scheduler(args).await
}

/// Spawn a supervisor actor for managing schedulers.
pub async fn spawn_supervisor(
    args: SupervisorArguments,
) -> Result<ActorRef<SupervisorMessage>, ActorError> {
    let (actor_ref, _handle) = Actor::spawn(None, SchedulerSupervisorDef, args)
        .await
        .map_err(|e| ActorError::SpawnFailed(format!("Failed to spawn supervisor: {}", e)))?;
    Ok(actor_ref)
}

/// Spawn a supervisor with a specific name.
pub async fn spawn_supervisor_with_name(
    args: SupervisorArguments,
    name: &str,
) -> Result<ActorRef<SupervisorMessage>, ActorError> {
    let (actor_ref, _handle) = Actor::spawn(Some(name.to_string()), SchedulerSupervisorDef, args)
        .await
        .map_err(|e| {
            ActorError::SpawnFailed(format!("Failed to spawn supervisor '{}': {}", name, e))
        })?;
    Ok(actor_ref)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_for_one_strategy_default() {
        let state = SupervisorActorState {
            config: SchedulerSupervisorConfig::default(),
            state: SupervisorState::Running,
            children: std::collections::HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        };

        assert_eq!(state.restart_strategy.name(), "one_for_one");
    }

    #[test]
    fn test_calculate_backoff() {
        // First attempt: 100ms
        let backoff = calculate_backoff(0, 100, 3200);
        assert_eq!(backoff.as_millis(), 100);

        // Second attempt: 200ms
        let backoff = calculate_backoff(1, 100, 3200);
        assert_eq!(backoff.as_millis(), 200);

        // Third attempt: 400ms
        let backoff = calculate_backoff(2, 100, 3200);
        assert_eq!(backoff.as_millis(), 400);

        // Should cap at max
        let backoff = calculate_backoff(10, 100, 3200);
        assert_eq!(backoff.as_millis(), 3200);
    }

    #[test]
    fn test_backoff_never_exceeds_max() {
        // Test invariant: Backoff never exceeds max (3200ms)
        let max_backoff: u128 = 3200;

        for attempt in 0..20 {
            let backoff = calculate_backoff(attempt, 100, max_backoff as u64);
            assert!(
                backoff.as_millis() <= max_backoff,
                "Backoff at attempt {} exceeded max: {}ms",
                attempt,
                backoff.as_millis()
            );
        }
    }

    #[test]
    fn test_meltdown_status_normal() {
        let state = SupervisorActorState {
            config: SchedulerSupervisorConfig::default(),
            state: SupervisorState::Running,
            children: HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        };

        assert_eq!(
            SchedulerSupervisorDef::check_meltdown(&state),
            MeltdownStatus::Normal
        );
    }

    #[test]
    fn test_meltdown_status_warning() {
        let mut state = SupervisorActorState {
            config: SchedulerSupervisorConfig::default(),
            state: SupervisorState::Running,
            children: HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        };

        // Add failures to trigger warning (0.5 per second over 60 seconds = 30 failures)
        for _ in 0..35 {
            state.failure_times.push(Instant::now());
        }

        assert_eq!(
            SchedulerSupervisorDef::check_meltdown(&state),
            MeltdownStatus::Warning
        );
    }

    #[test]
    fn test_meltdown_status_meltdown() {
        let mut state = SupervisorActorState {
            config: SchedulerSupervisorConfig::default(),
            state: SupervisorState::Running,
            children: HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        };

        // Add failures to trigger meltdown (1.0 per second over 60 seconds = 60+ failures)
        for _ in 0..65 {
            state.failure_times.push(Instant::now());
        }

        assert_eq!(
            SchedulerSupervisorDef::check_meltdown(&state),
            MeltdownStatus::Meltdown
        );
    }

    #[test]
    fn test_supervisor_config_default() {
        let config = SchedulerSupervisorConfig::default();
        assert_eq!(config.max_restarts, 3);
        assert_eq!(config.restart_window_secs, 60);
        assert_eq!(config.base_backoff_ms, 100);
        assert_eq!(config.max_backoff_ms, 3200);
    }

    #[test]
    fn test_supervisor_config_for_testing() {
        let config = SchedulerSupervisorConfig::for_testing();
        assert!(config.base_backoff_ms < 100);
        assert!(config.restart_window_secs < 60);
    }

    #[test]
    fn test_supervisor_arguments_builder() {
        let args = SupervisorArguments::new().with_config(SchedulerSupervisorConfig::for_testing());

        assert!(args.shutdown_coordinator.is_none());
        assert_eq!(args.config.max_restarts, 3);
    }

    #[test]
    fn test_supervisor_state_enum() {
        assert_eq!(SupervisorState::Running, SupervisorState::Running);
        assert_ne!(SupervisorState::Running, SupervisorState::ShuttingDown);
        assert_ne!(SupervisorState::Running, SupervisorState::Stopped);
    }

    #[test]
    fn test_meltdown_status_enum() {
        assert_eq!(MeltdownStatus::Normal, MeltdownStatus::Normal);
        assert_ne!(MeltdownStatus::Normal, MeltdownStatus::Warning);
        assert_ne!(MeltdownStatus::Warning, MeltdownStatus::Meltdown);
    }

    #[test]
    fn test_spawn_error_display() {
        let err = SpawnError("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_build_status() {
        let state = SupervisorActorState {
            config: SchedulerSupervisorConfig::default(),
            state: SupervisorState::Running,
            children: HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 5,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        };

        let status = SchedulerSupervisorDef::build_status(&state);
        assert_eq!(status.state, SupervisorState::Running);
        assert_eq!(status.meltdown_status, MeltdownStatus::Normal);
        assert_eq!(status.active_children, 0);
        assert_eq!(status.total_restarts, 5);
        assert_eq!(status.failures_in_window, 0);
    }

    #[test]
    fn test_cleanup_old_failures() {
        let mut state = SupervisorActorState {
            config: SchedulerSupervisorConfig {
                restart_window_secs: 1, // 1 second window for testing
                ..Default::default()
            },
            state: SupervisorState::Running,
            children: HashMap::new(),
            failure_times: vec![
                Instant::now() - Duration::from_secs(10), // Old, should be removed
                Instant::now(),                           // Recent, should stay
            ],
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        };

        SchedulerSupervisorDef::cleanup_old_failures(&mut state);
        assert_eq!(state.failure_times.len(), 1);
    }

    #[tokio::test]
    async fn test_spawn_supervisor() {
        let args = SupervisorArguments::new().with_config(SchedulerSupervisorConfig::for_testing());

        let result = spawn_supervisor(args).await;
        assert!(result.is_ok());

        if let Ok(supervisor) = result {
            supervisor.stop(None);
        }
    }

    #[tokio::test]
    async fn test_spawn_supervisor_with_name() {
        let args = SupervisorArguments::new().with_config(SchedulerSupervisorConfig::for_testing());

        let result = spawn_supervisor_with_name(args, "test-supervisor").await;
        assert!(result.is_ok());

        if let Ok(supervisor) = result {
            supervisor.stop(None);
        }
    }

    #[tokio::test]
    async fn test_supervisor_get_status() {
        let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());

        let supervisor = spawn_supervisor::<SchedulerActorDef>(args).await;
        assert!(supervisor.is_ok());

        if let Ok(sup) = supervisor {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = sup.send_message(SupervisorMessage::<SchedulerActorDef>::GetStatus { reply: tx });

            let status = rx.await;
            assert!(status.is_ok());

            if let Ok(s) = status {
                assert_eq!(s.state, SupervisorState::Running);
                assert_eq!(s.active_children, 0);
            }

            sup.stop(None);
        }
    }

    #[tokio::test]
    async fn test_supervisor_spawn_child() {
        // Use unique name to avoid collision with parallel tests
        let child_name = format!("spawn-child-{}", std::process::id());

        let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());

        let supervisor = spawn_supervisor::<SchedulerActorDef>(args).await;
        assert!(supervisor.is_ok(), "Failed to spawn supervisor");

        if let Ok(sup) = supervisor {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let send_result = sup.send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
                name: child_name.clone(),
                args: SchedulerArguments::new(),
                reply: tx,
            });
            assert!(send_result.is_ok(), "Failed to send SpawnChild message");

            let result = rx.await;
            assert!(result.is_ok(), "Channel receive failed");

            if let Ok(spawn_result) = result {
                assert!(
                    spawn_result.is_ok(),
                    "SpawnChild failed: {:?}",
                    spawn_result.err()
                );
            }

            // Check status shows 1 child
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = sup.send_message(SupervisorMessage::GetStatus { reply: tx });

            let status = rx.await;
            if let Ok(s) = status {
                assert_eq!(s.active_children, 1);
            }

            sup.stop(None);
        }
    }

    #[tokio::test]
    async fn test_supervisor_stop_child() {
        let child_name = format!("stop-child-{}", std::process::id());

        let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());

        let supervisor = spawn_supervisor::<SchedulerActorDef>(args).await;
        assert!(supervisor.is_ok());

        if let Ok(sup) = supervisor {
            // Spawn a child
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = sup.send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
                name: child_name.clone(),
                args: SchedulerArguments::new(),
                reply: tx,
            });
            let _ = rx.await;

            // Stop the child
            let _ = sup.send_message(SupervisorMessage::StopChild { name: child_name });

            // Give it time to process
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Check status shows 0 children
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = sup.send_message(SupervisorMessage::GetStatus { reply: tx });

            let status = rx.await;
            if let Ok(s) = status {
                assert_eq!(s.active_children, 0);
            }

            sup.stop(None);
        }
    }

    #[tokio::test]
    async fn test_supervisor_shutdown() {
        let args = SupervisorArguments::new().with_config(SchedulerSupervisorConfig::for_testing());

        let supervisor = spawn_supervisor(args).await;
        assert!(supervisor.is_ok());

        if let Ok(sup) = supervisor {
            // Spawn a child first
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = sup.send_message(SupervisorMessage::SpawnChild {
                name: "test-child".to_string(),
                args: SchedulerArguments::new(),
                reply: tx,
            });
            let _ = rx.await;

            // Send shutdown
            let _ = sup.send_message(SupervisorMessage::Shutdown);

            // Give it time to shutdown
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Actor should be stopped
            assert!(matches!(sup.get_status(), ractor::ActorStatus::Stopped));
        }
    }
}
