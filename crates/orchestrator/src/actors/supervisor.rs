//! Supervisor configuration and spawning for the SchedulerActor.
//!
//! Uses native ractor supervision with manual restart logic and exponential backoff.
//! The supervisor only restarts on abnormal exits (Transient policy).

use std::sync::Arc;
use std::time::{Duration, Instant};

use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use tracing::{error, info, warn};

use oya_events::EventBus;

use crate::shutdown::ShutdownCoordinator;

use super::messages::SchedulerMessage;
use super::scheduler::{SchedulerActorDef, SchedulerArguments};

/// Default maximum restarts before meltdown.
const DEFAULT_MAX_RESTARTS: usize = 3;

/// Default time window for meltdown counting.
const DEFAULT_MAX_WINDOW: Duration = Duration::from_secs(60);

/// Default reset period after which failure count resets.
const DEFAULT_RESET_AFTER: Duration = Duration::from_secs(120);

/// Base delay for exponential backoff.
const BACKOFF_BASE_MS: u64 = 100;

/// Maximum backoff delay.
const BACKOFF_MAX_MS: u64 = 1600;

/// Configuration for the scheduler supervisor.
#[derive(Clone)]
pub struct SchedulerSupervisorConfig {
    /// Maximum restarts before meltdown (default: 3).
    pub max_restarts: usize,
    /// Time window for meltdown counting (default: 60s).
    pub max_window: Duration,
    /// Reset failure count after this duration of stability (default: 120s).
    pub reset_after: Duration,
    /// Optional EventBus for the scheduler.
    pub event_bus: Option<Arc<EventBus>>,
    /// Optional ShutdownCoordinator for the scheduler.
    pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
}

impl Default for SchedulerSupervisorConfig {
    fn default() -> Self {
        Self {
            max_restarts: DEFAULT_MAX_RESTARTS,
            max_window: DEFAULT_MAX_WINDOW,
            reset_after: DEFAULT_RESET_AFTER,
            event_bus: None,
            shutdown_coordinator: None,
        }
    }
}

impl SchedulerSupervisorConfig {
    /// Create a new supervisor configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum restarts before meltdown.
    pub fn with_max_restarts(mut self, max_restarts: usize) -> Self {
        self.max_restarts = max_restarts;
        self
    }

    /// Set the time window for meltdown counting.
    pub fn with_max_window(mut self, window: Duration) -> Self {
        self.max_window = window;
        self
    }

    /// Set the reset period.
    pub fn with_reset_after(mut self, reset: Duration) -> Self {
        self.reset_after = reset;
        self
    }

    /// Set the EventBus.
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Set the ShutdownCoordinator.
    pub fn with_shutdown_coordinator(mut self, coordinator: Arc<ShutdownCoordinator>) -> Self {
        self.shutdown_coordinator = Some(coordinator);
        self
    }
}

/// Calculate exponential backoff delay for a given restart count.
///
/// Delays increase exponentially: 100ms, 200ms, 400ms, 800ms, 1600ms (capped).
///
/// # Arguments
///
/// * `restart_count` - The number of restarts that have occurred
///
/// # Returns
///
/// * `None` for the first restart (immediate)
/// * `Some(Duration)` for subsequent restarts with exponential backoff
pub fn calculate_backoff(restart_count: usize) -> Option<Duration> {
    if restart_count == 0 {
        // Immediate first restart
        None
    } else {
        // Exponential backoff: 100ms * 2^n, capped at 1.6s
        let exponent = restart_count.min(4) as u32;
        let delay_ms = BACKOFF_BASE_MS.saturating_mul(1u64 << exponent);
        let capped_delay = delay_ms.min(BACKOFF_MAX_MS);
        Some(Duration::from_millis(capped_delay))
    }
}

/// Spawn the SchedulerActor under a supervisor.
///
/// Note: This currently spawns the scheduler without supervision.
/// For full supervision with automatic restarts, use `spawn_scheduler`
/// and implement supervision in the parent actor via `handle_supervisor_evt`.
///
/// # Arguments
///
/// * `config` - Supervisor configuration
///
/// # Returns
///
/// * `Ok(ActorRef)` - Reference to the spawned scheduler actor
/// * `Err(...)` - If spawning fails
pub async fn spawn_supervised_scheduler(
    config: SchedulerSupervisorConfig,
) -> Result<ActorRef<SchedulerMessage>, SpawnError> {
    info!(
        max_restarts = config.max_restarts,
        max_window_secs = config.max_window.as_secs(),
        reset_after_secs = config.reset_after.as_secs(),
        "Spawning supervised scheduler actor"
    );

    // Create scheduler arguments
    let mut args = SchedulerArguments::new();
    if let Some(bus) = config.event_bus {
        args = args.with_event_bus(bus);
    }
    if let Some(coord) = config.shutdown_coordinator {
        args = args.with_shutdown_coordinator(coord);
    }

    // Spawn the scheduler actor
    let (actor_ref, _handle) = Actor::spawn(Some("scheduler".to_string()), SchedulerActorDef, args)
        .await
        .map_err(|e| SpawnError::ActorFailed(e.to_string()))?;

    info!("Scheduler actor spawned");

    Ok(actor_ref)
}

/// Spawn the SchedulerActor without supervision (for testing).
///
/// # Arguments
///
/// * `args` - Scheduler initialization arguments
///
/// # Returns
///
/// * `Ok(ActorRef)` - Reference to the spawned scheduler actor
/// * `Err(...)` - If spawning fails
pub async fn spawn_scheduler(
    args: SchedulerArguments,
) -> Result<ActorRef<SchedulerMessage>, SpawnError> {
    spawn_scheduler_with_name(args, "scheduler").await
}

/// Spawn the SchedulerActor with a custom name (for parallel testing).
///
/// # Arguments
///
/// * `args` - Scheduler initialization arguments
/// * `name` - Unique name for the actor (must be unique across concurrent tests)
///
/// # Returns
///
/// * `Ok(ActorRef)` - Reference to the spawned scheduler actor
/// * `Err(...)` - If spawning fails
pub async fn spawn_scheduler_with_name(
    args: SchedulerArguments,
    name: &str,
) -> Result<ActorRef<SchedulerMessage>, SpawnError> {
    info!(name = name, "Spawning scheduler actor (unsupervised)");

    let (actor_ref, _handle) = Actor::spawn(Some(name.to_string()), SchedulerActorDef, args)
        .await
        .map_err(|e| SpawnError::ActorFailed(e.to_string()))?;

    Ok(actor_ref)
}

/// Errors that can occur when spawning actors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SpawnError {
    /// Failed to spawn the supervisor.
    #[error("Failed to spawn supervisor: {0}")]
    SupervisorFailed(String),

    /// Failed to spawn the actor.
    #[error("Failed to spawn actor: {0}")]
    ActorFailed(String),
}

// ═══════════════════════════════════════════════════════════════════════════
// SUPERVISOR ACTOR
// ═══════════════════════════════════════════════════════════════════════════

/// Messages for the scheduler supervisor.
#[derive(Debug)]
pub enum SupervisorMessage {
    /// Get the current scheduler actor reference.
    GetScheduler {
        reply: ractor::RpcReplyPort<Option<ActorRef<SchedulerMessage>>>,
    },
    /// Get the supervisor's meltdown status.
    GetMeltdownStatus {
        reply: ractor::RpcReplyPort<MeltdownStatus>,
    },
}

/// Status of the supervisor's meltdown detection.
#[derive(Debug, Clone)]
pub struct MeltdownStatus {
    /// Number of restarts in the current window.
    pub restart_count: usize,
    /// Whether meltdown has been triggered.
    pub is_meltdown: bool,
    /// Time until window resets (if not in meltdown).
    pub window_remaining: Option<Duration>,
}

/// The scheduler supervisor actor definition.
///
/// This actor supervises the SchedulerActor, implementing:
/// - Automatic restart on panic (Transient policy)
/// - Exponential backoff between restarts
/// - Meltdown detection (stops after N failures in window)
pub struct SchedulerSupervisorDef;

/// State for the scheduler supervisor.
pub struct SupervisorState {
    /// Configuration for supervision.
    config: SchedulerSupervisorConfig,
    /// Reference to the supervised scheduler actor.
    scheduler_ref: Option<ActorRef<SchedulerMessage>>,
    /// Restart timestamps for meltdown detection.
    restart_times: Vec<Instant>,
    /// Number of consecutive restarts.
    restart_count: usize,
    /// Whether meltdown has been triggered.
    is_meltdown: bool,
    /// Time of last stability check.
    last_stability_check: Option<Instant>,
}

impl SupervisorState {
    fn new(config: SchedulerSupervisorConfig) -> Self {
        Self {
            config,
            scheduler_ref: None,
            restart_times: Vec::new(),
            restart_count: 0,
            is_meltdown: false,
            last_stability_check: None,
        }
    }

    /// Check if we're in meltdown state (too many restarts in window).
    fn check_meltdown(&mut self) -> bool {
        let now = Instant::now();

        // Remove restarts outside the window
        let window_start = now - self.config.max_window;
        self.restart_times.retain(|&t| t > window_start);

        // Check if we've exceeded max restarts
        if self.restart_times.len() >= self.config.max_restarts {
            self.is_meltdown = true;
        }

        self.is_meltdown
    }

    /// Record a restart event.
    fn record_restart(&mut self) {
        self.restart_times.push(Instant::now());
        self.restart_count += 1;
    }

    /// Check if we should reset the failure count (stable period).
    fn check_stability_reset(&mut self) {
        if let Some(last_check) = self.last_stability_check {
            if last_check.elapsed() >= self.config.reset_after {
                // Stable for reset_after duration, reset counters
                self.restart_times.clear();
                self.restart_count = 0;
                info!("Supervisor stability period reached, resetting failure count");
            }
        }
        self.last_stability_check = Some(Instant::now());
    }
}

impl Actor for SchedulerSupervisorDef {
    type Msg = SupervisorMessage;
    type State = SupervisorState;
    type Arguments = SchedulerSupervisorConfig;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("SchedulerSupervisor starting");

        let mut state = SupervisorState::new(args);

        // Spawn the initial scheduler actor as a linked child
        match Self::spawn_scheduler_child(&myself, &state.config).await {
            Ok(scheduler_ref) => {
                state.scheduler_ref = Some(scheduler_ref);
                info!("Initial scheduler actor spawned");
            }
            Err(e) => {
                error!(error = %e, "Failed to spawn initial scheduler actor");
                return Err(Box::new(e));
            }
        }

        Ok(state)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisorMessage::GetScheduler { reply } => {
                let _ = reply.send(state.scheduler_ref.clone());
            }
            SupervisorMessage::GetMeltdownStatus { reply } => {
                let window_remaining = if !state.is_meltdown && !state.restart_times.is_empty() {
                    let oldest = state.restart_times.first().copied();
                    oldest.map(|t| {
                        let elapsed = t.elapsed();
                        if elapsed < state.config.max_window {
                            state.config.max_window - elapsed
                        } else {
                            Duration::ZERO
                        }
                    })
                } else {
                    None
                };

                let status = MeltdownStatus {
                    restart_count: state.restart_times.len(),
                    is_meltdown: state.is_meltdown,
                    window_remaining,
                };
                let _ = reply.send(status);
            }
        }

        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        myself: ActorRef<Self::Msg>,
        event: SupervisionEvent,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match event {
            SupervisionEvent::ActorStarted(actor_cell) => {
                info!(
                    actor_id = %actor_cell.get_id(),
                    "Supervised actor started"
                );
                state.check_stability_reset();
            }

            SupervisionEvent::ActorTerminated(actor_cell, _actor_state, reason) => {
                let is_abnormal = reason.is_some();
                info!(
                    actor_id = %actor_cell.get_id(),
                    abnormal = is_abnormal,
                    reason = ?reason,
                    "Supervised actor terminated"
                );

                // Clear the reference
                state.scheduler_ref = None;

                // Only restart on abnormal termination (Transient policy)
                if is_abnormal {
                    state.record_restart();

                    // Check for meltdown
                    if state.check_meltdown() {
                        error!(
                            restart_count = state.restart_times.len(),
                            max_restarts = state.config.max_restarts,
                            window_secs = state.config.max_window.as_secs(),
                            "MELTDOWN: Too many restarts in window, stopping supervisor"
                        );
                        myself.stop(Some("meltdown".to_string()));
                        return Ok(());
                    }

                    // Calculate backoff
                    let backoff = calculate_backoff(state.restart_count);
                    if let Some(delay) = backoff {
                        warn!(
                            restart_count = state.restart_count,
                            delay_ms = delay.as_millis(),
                            "Restarting scheduler after backoff"
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        info!(restart_count = state.restart_count, "Restarting scheduler immediately");
                    }

                    // Attempt restart
                    match Self::spawn_scheduler_child(&myself, &state.config).await {
                        Ok(scheduler_ref) => {
                            state.scheduler_ref = Some(scheduler_ref);
                            info!("Scheduler actor restarted successfully");
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to restart scheduler actor");
                            // Record as another failure
                            state.record_restart();
                            if state.check_meltdown() {
                                error!("MELTDOWN: Restart failed, stopping supervisor");
                                myself.stop(Some("meltdown".to_string()));
                            }
                        }
                    }
                } else {
                    // Normal termination - don't restart
                    info!("Scheduler stopped normally, not restarting");
                }
            }

            SupervisionEvent::ActorFailed(actor_cell, error) => {
                error!(
                    actor_id = %actor_cell.get_id(),
                    error = %error,
                    "Supervised actor panicked"
                );

                // Clear the reference
                state.scheduler_ref = None;
                state.record_restart();

                // Check for meltdown
                if state.check_meltdown() {
                    error!(
                        restart_count = state.restart_times.len(),
                        max_restarts = state.config.max_restarts,
                        "MELTDOWN: Too many panics in window, stopping supervisor"
                    );
                    myself.stop(Some("meltdown".to_string()));
                    return Ok(());
                }

                // Calculate backoff
                let backoff = calculate_backoff(state.restart_count);
                if let Some(delay) = backoff {
                    warn!(
                        restart_count = state.restart_count,
                        delay_ms = delay.as_millis(),
                        "Restarting scheduler after panic with backoff"
                    );
                    tokio::time::sleep(delay).await;
                }

                // Attempt restart
                match Self::spawn_scheduler_child(&myself, &state.config).await {
                    Ok(scheduler_ref) => {
                        state.scheduler_ref = Some(scheduler_ref);
                        info!("Scheduler actor restarted after panic");
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to restart scheduler after panic");
                        state.record_restart();
                        if state.check_meltdown() {
                            error!("MELTDOWN: Restart failed after panic, stopping supervisor");
                            myself.stop(Some("meltdown".to_string()));
                        }
                    }
                }
            }

            SupervisionEvent::ProcessGroupChanged(_) => {
                // Ignore process group changes
            }
        }

        Ok(())
    }
}

impl SchedulerSupervisorDef {
    /// Spawn the scheduler as a linked child of the supervisor.
    async fn spawn_scheduler_child(
        supervisor: &ActorRef<SupervisorMessage>,
        config: &SchedulerSupervisorConfig,
    ) -> Result<ActorRef<SchedulerMessage>, SpawnError> {
        let mut args = SchedulerArguments::new();
        if let Some(bus) = &config.event_bus {
            args = args.with_event_bus(bus.clone());
        }
        if let Some(coord) = &config.shutdown_coordinator {
            args = args.with_shutdown_coordinator(coord.clone());
        }

        // Generate unique name for each spawn
        let name = format!("scheduler-{}", uuid_v4_simple());

        // Spawn linked to supervisor so we receive supervision events
        let (actor_ref, _handle) =
            Actor::spawn_linked(Some(name), SchedulerActorDef, args, supervisor.get_cell())
                .await
                .map_err(|e| SpawnError::ActorFailed(e.to_string()))?;

        Ok(actor_ref)
    }
}

/// Generate a simple unique ID (not a real UUID, just for naming).
fn uuid_v4_simple() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{:x}-{:x}", timestamp, id)
}

/// Spawn the SchedulerSupervisor actor.
///
/// # Arguments
///
/// * `config` - Supervisor configuration
/// * `name` - Optional name for the supervisor (defaults to "scheduler-supervisor")
///
/// # Returns
///
/// * `Ok(ActorRef)` - Reference to the spawned supervisor actor
/// * `Err(...)` - If spawning fails
pub async fn spawn_supervisor(
    config: SchedulerSupervisorConfig,
    name: Option<&str>,
) -> Result<ActorRef<SupervisorMessage>, SpawnError> {
    let supervisor_name = name.unwrap_or("scheduler-supervisor").to_string();

    info!(
        name = %supervisor_name,
        max_restarts = config.max_restarts,
        max_window_secs = config.max_window.as_secs(),
        "Spawning scheduler supervisor"
    );

    let (actor_ref, _handle) =
        Actor::spawn(Some(supervisor_name), SchedulerSupervisorDef, config)
            .await
            .map_err(|e| SpawnError::SupervisorFailed(e.to_string()))?;

    Ok(actor_ref)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_default_config() {
        let config = SchedulerSupervisorConfig::default();

        assert_eq!(config.max_restarts, 3);
        assert_eq!(config.max_window, Duration::from_secs(60));
        assert_eq!(config.reset_after, Duration::from_secs(120));
        assert!(config.event_bus.is_none());
        assert!(config.shutdown_coordinator.is_none());
    }

    #[test]
    fn should_configure_max_restarts() {
        let config = SchedulerSupervisorConfig::new().with_max_restarts(5);

        assert_eq!(config.max_restarts, 5);
    }

    #[test]
    fn should_configure_max_window() {
        let config = SchedulerSupervisorConfig::new().with_max_window(Duration::from_secs(30));

        assert_eq!(config.max_window, Duration::from_secs(30));
    }

    #[test]
    fn should_calculate_exponential_backoff() {
        // First restart: no delay
        let delay_0 = calculate_backoff(0);
        assert!(delay_0.is_none());

        // Second restart: 200ms
        let delay_1 = calculate_backoff(1);
        assert_eq!(delay_1, Some(Duration::from_millis(200)));

        // Third restart: 400ms
        let delay_2 = calculate_backoff(2);
        assert_eq!(delay_2, Some(Duration::from_millis(400)));

        // Fourth restart: 800ms
        let delay_3 = calculate_backoff(3);
        assert_eq!(delay_3, Some(Duration::from_millis(800)));

        // Fifth restart: 1600ms (capped)
        let delay_4 = calculate_backoff(4);
        assert_eq!(delay_4, Some(Duration::from_millis(1600)));

        // Beyond cap: still 1600ms
        let delay_5 = calculate_backoff(5);
        assert_eq!(delay_5, Some(Duration::from_millis(1600)));
    }

    #[tokio::test]
    async fn should_spawn_unsupervised_scheduler() {
        let args = SchedulerArguments::new();
        let result = spawn_scheduler(args).await;

        assert!(result.is_ok());

        // Stop the actor
        if let Ok(actor_ref) = result {
            actor_ref.stop(None);
        }
    }
}
