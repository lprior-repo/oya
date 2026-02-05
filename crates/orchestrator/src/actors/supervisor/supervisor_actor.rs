//! Generic Supervisor module for managing child actors.
//!
//! This module provides supervisor patterns for child actors,
//! including spawn helpers, supervision strategies, restart with
//! exponential backoff, and meltdown detection.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::shutdown::{ShutdownCoordinator, ShutdownSignal};

use super::super::errors::ActorError;
use super::strategy::{OneForOne, RestartContext, RestartDecision, RestartStrategy};

/// Trait for actors that can be supervised by the GenericSupervisor.
pub trait GenericSupervisableActor: Actor + Clone
where
    Self::Arguments: Clone + Send + Sync,
    Self::Msg: Clone + Send,
{
    /// Get the default arguments for this actor.
    fn default_args() -> Self::Arguments;
}

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

/// Configuration for supervision.
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
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

impl Default for SupervisorConfig {
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

impl SupervisorConfig {
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
pub enum SupervisorMessage<A: Actor> {
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
    /// Spawn a new child.
    SpawnChild {
        /// Name for the child
        name: String,
        /// Arguments for the actor
        args: A::Arguments,
        /// Reply channel for result
        reply: tokio::sync::oneshot::Sender<Result<(), ActorError>>,
    },
    /// Stop a specific child.
    StopChild {
        /// Name of the child to stop
        name: String,
    },
}

impl<A: Actor> Debug for SupervisorMessage<A> {
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
#[derive(Clone)]
pub struct ChildInfo<A: GenericSupervisableActor>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    /// Child name.
    pub name: String,
    /// Actor reference.
    pub actor_ref: ActorRef<A::Msg>,
    /// Number of restarts.
    pub restart_count: u32,
    /// Time of last restart.
    pub last_restart: Option<Instant>,
    /// Arguments used to spawn this child.
    pub args: A::Arguments,
}

impl<A: GenericSupervisableActor> Debug for ChildInfo<A>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildInfo")
            .field("name", &self.name)
            .field("restart_count", &self.restart_count)
            .field("last_restart", &self.last_restart)
            .finish_non_exhaustive()
    }
}

/// Definition for the generic supervisor actor.
#[derive(Clone)]
pub struct SupervisorActorDef<A: GenericSupervisableActor>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    _actor: std::marker::PhantomData<A>,
    child_def: A,
}

impl<A: GenericSupervisableActor> SupervisorActorDef<A>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    /// Create a new supervisor definition for a specific actor type.
    pub fn new(child_def: A) -> Self {
        Self {
            _actor: std::marker::PhantomData,
            child_def,
        }
    }
}

/// State for the supervisor actor.
pub struct SupervisorActorState<A: GenericSupervisableActor>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    /// Configuration.
    pub config: SupervisorConfig,
    /// Current state.
    pub state: SupervisorState,
    /// Supervised children.
    pub children: HashMap<String, ChildInfo<A>>,
    /// Failure timestamps for meltdown detection.
    pub failure_times: Vec<Instant>,
    /// Total restarts performed.
    pub total_restarts: u32,
    /// Counter for generating unique child actor names.
    pub child_id_counter: u64,
    /// Shutdown coordinator reference.
    pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
    /// Shutdown signal receiver.
    pub _shutdown_rx: Option<broadcast::Receiver<ShutdownSignal>>,
    /// Restart strategy.
    pub restart_strategy: Box<dyn RestartStrategy<A>>,
}

impl<A: GenericSupervisableActor> Debug for SupervisorActorState<A>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
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
pub struct SupervisorArguments {
    /// Configuration.
    pub config: SupervisorConfig,
    /// Optional shutdown coordinator.
    pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
}

impl Default for SupervisorArguments {
    fn default() -> Self {
        Self {
            config: SupervisorConfig::default(),
            shutdown_coordinator: None,
        }
    }
}

impl SupervisorArguments {
    /// Create new arguments with default config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration.
    #[must_use]
    pub fn with_config(mut self, config: SupervisorConfig) -> Self {
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

impl<A: GenericSupervisableActor> Actor for SupervisorActorDef<A>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    type Msg = SupervisorMessage<A>;
    type State = SupervisorActorState<A>;
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
                self.handle_child_exited(myself, state, &name, &reason)
                    .await;
            }

            SupervisorMessage::GetStatus { reply } => {
                let status = self.build_status(state);
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
                let result = self.spawn_child(myself.clone(), state, name, args).await;
                let _ = reply.send(result);
            }

            SupervisorMessage::StopChild { name } => {
                self.stop_child(state, &name);
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

impl<A: GenericSupervisableActor> SupervisorActorDef<A>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    /// Generate a unique child actor name.
    fn next_child_name(child_id_counter: &mut u64, prefix: &str) -> String {
        let id = *child_id_counter;
        *child_id_counter = child_id_counter.saturating_add(1);
        format!("{}-{}", prefix, id)
    }

    /// Handle a child exit event.
    async fn handle_child_exited(
        &self,
        myself: ActorRef<SupervisorMessage<A>>,
        state: &mut SupervisorActorState<A>,
        name: &str,
        reason: &str,
    ) {
        warn!(child = %name, reason = %reason, "Child exited");

        // Record failure time for meltdown detection
        state.failure_times.push(Instant::now());
        self.cleanup_old_failures(state);

        // Check meltdown status
        let meltdown_status = self.check_meltdown(state);
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
                    self.schedule_child_restart(myself.clone(), state, child_name);
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
        &self,
        myself: ActorRef<SupervisorMessage<A>>,
        state: &mut SupervisorActorState<A>,
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

    /// Spawn a new child.
    async fn spawn_child(
        &self,
        myself: ActorRef<SupervisorMessage<A>>,
        state: &mut SupervisorActorState<A>,
        name: String,
        args: A::Arguments,
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

        // Generate unique actor name to avoid collisions
        let actor_name = Self::next_child_name(&mut state.child_id_counter, &name);

        let (actor_ref, handle) = Actor::spawn(
            Some(actor_name.clone()),
            self.child_def.clone(),
            args.clone(),
        )
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
    fn stop_child(&self, state: &mut SupervisorActorState<A>, name: &str) {
        if let Some(child) = state.children.remove(name) {
            debug!(child = %name, "Stopping child");
            child
                .actor_ref
                .stop(Some("Requested by supervisor".to_string()));
        }
    }

    /// Clean up old failure timestamps.
    fn cleanup_old_failures(&self, state: &mut SupervisorActorState<A>) {
        let window = Duration::from_secs(state.config.restart_window_secs);
        state.failure_times.retain(|t| t.elapsed() < window);
    }

    /// Check meltdown status based on failure rate.
    fn check_meltdown(&self, state: &SupervisorActorState<A>) -> MeltdownStatus {
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
    fn build_status(&self, state: &SupervisorActorState<A>) -> SupervisorStatus {
        SupervisorStatus {
            state: state.state,
            meltdown_status: self.check_meltdown(state),
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

/// Spawn a supervisor actor for managing child actors.
pub async fn spawn_supervisor<A>(
    args: SupervisorArguments,
) -> Result<ActorRef<SupervisorMessage<A>>, ActorError>
where
    A: GenericSupervisableActor + Default,
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    spawn_supervisor_with_name::<A>(args, "supervisor").await
}

/// Spawn a supervisor with a specific name.
pub async fn spawn_supervisor_with_name<A>(
    args: SupervisorArguments,
    name: &str,
) -> Result<ActorRef<SupervisorMessage<A>>, ActorError>
where
    A: GenericSupervisableActor + Default,
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    let (actor, _handle) = Actor::spawn(
        Some(name.to_string()),
        SupervisorActorDef::new(A::default()),
        args,
    )
    .await
    .map_err(|e| ActorError::SpawnFailed(e.to_string()))?;

    Ok(actor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::scheduler::{SchedulerActorDef, SchedulerArguments};

    #[test]
    fn test_one_for_one_strategy_default() {
        let state = SupervisorActorState::<SchedulerActorDef> {
            config: SupervisorConfig::default(),
            state: SupervisorState::Running,
            children: HashMap::new(),
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
    fn test_meltdown_status_normal() {
        let state = SupervisorActorState::<SchedulerActorDef> {
            config: SupervisorConfig::default(),
            state: SupervisorState::Running,
            children: HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        };

        let def = SupervisorActorDef::new(SchedulerActorDef);
        assert_eq!(def.check_meltdown(&state), MeltdownStatus::Normal);
    }

    #[test]
    fn test_supervisor_config_default() {
        let config = SupervisorConfig::default();
        assert_eq!(config.max_restarts, 3);
        assert_eq!(config.restart_window_secs, 60);
        assert_eq!(config.base_backoff_ms, 100);
        assert_eq!(config.max_backoff_ms, 3200);
    }

    #[test]
    fn test_supervisor_config_for_testing() {
        let config = SupervisorConfig::for_testing();
        assert_eq!(config.restart_window_secs, 5);
    }

    #[tokio::test]
    async fn test_spawn_supervisor() {
        let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());

        let result =
            spawn_supervisor_with_name::<SchedulerActorDef>(args, "test-spawn-supervisor").await;
        assert!(result.is_ok());

        if let Ok(supervisor) = result {
            supervisor.stop(None);
        }
    }

    #[tokio::test]
    async fn test_supervisor_get_status() {
        let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());

        let supervisor =
            spawn_supervisor_with_name::<SchedulerActorDef>(args, "test-get-status").await;
        assert!(supervisor.is_ok());

        if let Ok(sup) = supervisor {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ =
                sup.send_message(SupervisorMessage::<SchedulerActorDef>::GetStatus { reply: tx });

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
        let child_name = format!("spawn-child-{}", std::process::id());
        let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());

        let supervisor =
            spawn_supervisor_with_name::<SchedulerActorDef>(args, "test-spawn-child").await;
        assert!(supervisor.is_ok());

        if let Ok(sup) = supervisor {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = sup.send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
                name: child_name.clone(),
                args: SchedulerArguments::default(),
                reply: tx,
            });

            let result = rx.await;
            assert!(result.is_ok());

            sup.stop(None);
        }
    }
}
