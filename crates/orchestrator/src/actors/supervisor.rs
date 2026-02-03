//! Supervisor module for scheduler actor management.
//!
//! This module provides supervisor patterns for scheduler actors,
//! including spawn helpers and supervision strategies.

use ractor::{Actor, ActorRef};

use super::errors::ActorError;
use super::messages::SchedulerMessage;
use super::scheduler::{SchedulerActorDef, SchedulerArguments};

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
    /// Time window for restart counting.
    pub restart_window_secs: u64,
    /// Base backoff duration in milliseconds.
    pub base_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds.
    pub max_backoff_ms: u64,
}

impl Default for SchedulerSupervisorConfig {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            restart_window_secs: 60,
            base_backoff_ms: 100,
            max_backoff_ms: 10000,
        }
    }
}

/// Messages for supervisor communication.
#[derive(Debug, Clone)]
pub enum SupervisorMessage {
    /// Child actor exited.
    ChildExited { name: String, reason: String },
    /// Request supervisor status.
    GetStatus,
    /// Shutdown the supervisor.
    Shutdown,
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

/// Definition for the scheduler supervisor actor.
pub struct SchedulerSupervisorDef;

/// Calculate exponential backoff duration.
pub fn calculate_backoff(attempt: u32, base_ms: u64, max_ms: u64) -> std::time::Duration {
    let backoff = base_ms.saturating_mul(2u64.saturating_pow(attempt));
    std::time::Duration::from_millis(backoff.min(max_ms))
}

/// Spawn a supervised scheduler actor.
pub async fn spawn_supervised_scheduler(
    args: SchedulerArguments,
    _config: SchedulerSupervisorConfig,
) -> Result<ActorRef<SchedulerMessage>, ActorError> {
    // For now, just spawn without supervision
    spawn_scheduler(args).await
}

/// Spawn a supervisor actor for managing schedulers.
pub async fn spawn_supervisor(
    _config: SchedulerSupervisorConfig,
) -> Result<ActorRef<SupervisorMessage>, ActorError> {
    // TODO: Implement supervisor actor
    Err(ActorError::SpawnFailed(
        "Supervisor not yet implemented".to_string(),
    ))
}
