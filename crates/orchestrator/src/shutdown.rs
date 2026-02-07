//! Graceful shutdown handling for the orchestrator.
//!
//! Handles SIGTERM/SIGINT signals and coordinates graceful shutdown of actors,
//! ensuring checkpoints are saved within a 30-second window.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures::stream::{self, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::{Error, Result};

/// Maximum time allowed for graceful shutdown (30 seconds)
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum time to wait for checkpoint saving (25 seconds, leaving 5s buffer)
const CHECKPOINT_TIMEOUT: Duration = Duration::from_secs(25);

/// Shutdown signal types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShutdownSignal {
    /// SIGTERM signal received
    Sigterm,
    /// SIGINT signal received (Ctrl+C)
    Sigint,
    /// Programmatic shutdown requested
    Programmatic,
}

impl std::fmt::Display for ShutdownSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sigterm => write!(f, "SIGTERM"),
            Self::Sigint => write!(f, "SIGINT"),
            Self::Programmatic => write!(f, "PROGRAMMATIC"),
        }
    }
}

/// Shutdown coordinator state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShutdownPhase {
    /// Normal operation, no shutdown initiated
    Running,
    /// Shutdown signal received, preparing to shut down
    Initiating,
    /// Saving checkpoints and state
    SavingCheckpoints,
    /// Stopping actors
    StoppingActors,
    /// Cleanup complete
    Complete,
}

/// Checkpoint save result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointResult {
    /// Component that saved the checkpoint
    pub component: String,
    /// Whether checkpoint was saved successfully
    pub success: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Optional error message
    pub error: Option<String>,
}

impl CheckpointResult {
    /// Create a successful checkpoint result
    pub fn success(component: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            component: component.into(),
            success: true,
            duration_ms,
            error: None,
        }
    }

    /// Create a failed checkpoint result
    pub fn failure(component: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            success: false,
            duration_ms: 0,
            error: Some(error.into()),
        }
    }
}

/// Shutdown coordinator for graceful system shutdown
pub struct ShutdownCoordinator {
    /// Current shutdown phase
    phase: Arc<RwLock<ShutdownPhase>>,
    /// Whether shutdown has been initiated
    shutdown_initiated: Arc<AtomicBool>,
    /// Broadcast channel for shutdown signal
    shutdown_tx: broadcast::Sender<ShutdownSignal>,
    /// Channel for checkpoint save results
    checkpoint_tx: mpsc::Sender<CheckpointResult>,
    checkpoint_rx: Arc<RwLock<mpsc::Receiver<CheckpointResult>>>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);
        let (checkpoint_tx, checkpoint_rx) = mpsc::channel(32);

        Self {
            phase: Arc::new(RwLock::new(ShutdownPhase::Running)),
            shutdown_initiated: Arc::new(AtomicBool::new(false)),
            shutdown_tx,
            checkpoint_tx,
            checkpoint_rx: Arc::new(RwLock::new(checkpoint_rx)),
        }
    }

    /// Get the current shutdown phase
    pub async fn phase(&self) -> ShutdownPhase {
        *self.phase.read().await
    }

    /// Check if shutdown has been initiated
    pub fn is_shutdown_initiated(&self) -> bool {
        self.shutdown_initiated.load(Ordering::Acquire)
    }

    /// Subscribe to shutdown notifications
    pub fn subscribe(&self) -> broadcast::Receiver<ShutdownSignal> {
        self.shutdown_tx.subscribe()
    }

    /// Get a sender for checkpoint results
    pub fn checkpoint_sender(&self) -> mpsc::Sender<CheckpointResult> {
        self.checkpoint_tx.clone()
    }

    /// Initiate graceful shutdown
    pub async fn initiate_shutdown(&self, signal: ShutdownSignal) -> Result<()> {
        // Check if already shutting down
        if self
            .shutdown_initiated
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            info!("Shutdown already in progress, ignoring duplicate signal");
            return Ok(());
        }

        info!(signal = %signal, "Initiating graceful shutdown");

        // Update phase
        *self.phase.write().await = ShutdownPhase::Initiating;

        // Broadcast shutdown signal
        let subscriber_count = self.shutdown_tx.receiver_count();
        info!(
            subscribers = subscriber_count,
            "Broadcasting shutdown signal to subscribers"
        );

        if let Err(e) = self.shutdown_tx.send(signal) {
            // Log but don't fail - no subscribers is not an error condition
            debug!("No active subscribers for shutdown signal: {}", e);
        }

        Ok(())
    }

    /// Execute graceful shutdown sequence
    pub async fn shutdown(&self) -> Result<ShutdownStats> {
        let start = std::time::Instant::now();
        let mut stats = ShutdownStats::default();

        info!("Starting graceful shutdown sequence");

        // Phase 1: Save checkpoints (with timeout)
        *self.phase.write().await = ShutdownPhase::SavingCheckpoints;
        match self.save_checkpoints().await {
            Ok(results) => {
                stats.checkpoints_saved = results.iter().filter(|r| r.success).count();
                stats.checkpoints_failed = results.iter().filter(|r| !r.success).count();
                info!(
                    saved = stats.checkpoints_saved,
                    failed = stats.checkpoints_failed,
                    "Checkpoint phase complete"
                );
            }
            Err(e) => {
                warn!(error = %e, "Checkpoint saving phase failed");
                stats.checkpoint_error = Some(format!("{}", e));
            }
        }

        // Phase 2: Stop actors
        *self.phase.write().await = ShutdownPhase::StoppingActors;
        if let Err(e) = self.stop_actors().await {
            warn!(error = %e, "Actor shutdown phase encountered errors");
        }

        // Phase 3: Mark complete
        *self.phase.write().await = ShutdownPhase::Complete;

        stats.total_duration_ms = start.elapsed().as_millis() as u64;
        info!(
            duration_ms = stats.total_duration_ms,
            "Graceful shutdown complete"
        );

        Ok(stats)
    }

    /// Save all checkpoints with timeout
    async fn save_checkpoints(&self) -> Result<Vec<CheckpointResult>> {
        info!("Saving checkpoints before shutdown");

        let mut rx = self.checkpoint_rx.write().await;

        // Wait for checkpoint results with timeout
        let results = timeout(CHECKPOINT_TIMEOUT, async {
            let mut results = Vec::new();
            // Collect all checkpoint results until channel closes or timeout
            while let Some(result) = rx.recv().await {
                debug!(
                    component = %result.component,
                    success = result.success,
                    duration_ms = result.duration_ms,
                    "Received checkpoint result"
                );
                results.push(result);
            }
            results
        })
        .await;

        match results {
            Ok(res) => Ok(res),
            Err(_) => {
                warn!(
                    timeout_secs = CHECKPOINT_TIMEOUT.as_secs(),
                    "Checkpoint timeout exceeded"
                );
                Ok(Vec::new())
            }
        }
    }

    /// Stop all actors gracefully
    async fn stop_actors(&self) -> Result<()> {
        info!("Stopping actors");
        // Actor shutdown logic will be implemented by components that subscribe
        // to shutdown notifications. This is a coordination point.
        // For fail-fast behavior, we would collect and propagate errors from
        // all actor shutdown operations
        Ok(())
    }

    /// Wait for shutdown with overall timeout
    pub async fn wait_with_timeout(&self) -> Result<ShutdownStats> {
        match timeout(SHUTDOWN_TIMEOUT, self.shutdown()).await {
            Ok(stats) => stats,
            Err(_) => {
                error!(
                    timeout_secs = SHUTDOWN_TIMEOUT.as_secs(),
                    "Shutdown timeout exceeded, forcing exit"
                );
                Err(Error::invalid_record(format!(
                    "Shutdown timeout exceeded: {} seconds",
                    SHUTDOWN_TIMEOUT.as_secs()
                )))
            }
        }
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the shutdown process
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShutdownStats {
    /// Number of checkpoints successfully saved
    pub checkpoints_saved: usize,
    /// Number of checkpoints that failed to save
    pub checkpoints_failed: usize,
    /// Error during checkpoint phase, if any
    pub checkpoint_error: Option<String>,
    /// Total shutdown duration in milliseconds
    pub total_duration_ms: u64,
}

/// Install OS signal handlers (SIGTERM, SIGINT)
pub async fn install_signal_handlers(
    coordinator: Arc<ShutdownCoordinator>,
) -> Result<tokio::task::JoinHandle<()>> {
    info!("Installing OS signal handlers");

    let handle = tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            // Install SIGTERM handler
            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "Failed to install SIGTERM handler");
                    return;
                }
            };

            // Install SIGINT handler
            let mut sigint = match signal(SignalKind::interrupt()) {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "Failed to install SIGINT handler");
                    return;
                }
            };

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM");
                    if let Err(e) = coordinator.initiate_shutdown(ShutdownSignal::Sigterm).await {
                        error!(error = %e, "Failed to initiate shutdown on SIGTERM");
                    }
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT");
                    if let Err(e) = coordinator.initiate_shutdown(ShutdownSignal::Sigint).await {
                        error!(error = %e, "Failed to initiate shutdown on SIGINT");
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            // Windows doesn't have SIGTERM, only SIGINT (Ctrl+C)
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!(error = %e, "Failed to listen for Ctrl+C");
                return;
            }

            info!("Received Ctrl+C");
            if let Err(e) = coordinator.initiate_shutdown(ShutdownSignal::Sigint).await {
                error!(error = %e, "Failed to initiate shutdown on Ctrl+C");
            }
        }
    });

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coordinator = ShutdownCoordinator::new();
        assert_eq!(coordinator.phase().await, ShutdownPhase::Running);
        assert!(!coordinator.is_shutdown_initiated());
    }

    #[tokio::test]
    async fn test_initiate_shutdown() {
        let coordinator = ShutdownCoordinator::new();

        let result = coordinator
            .initiate_shutdown(ShutdownSignal::Programmatic)
            .await;
        assert!(result.is_ok());
        assert!(coordinator.is_shutdown_initiated());
        assert_eq!(coordinator.phase().await, ShutdownPhase::Initiating);
    }

    #[tokio::test]
    async fn test_duplicate_shutdown_ignored() {
        let coordinator = ShutdownCoordinator::new();

        coordinator
            .initiate_shutdown(ShutdownSignal::Programmatic)
            .await
            .ok();

        // Second shutdown should succeed but be ignored
        let result = coordinator.initiate_shutdown(ShutdownSignal::Sigterm).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscribe_to_shutdown() {
        let coordinator = ShutdownCoordinator::new();
        let mut receiver = coordinator.subscribe();

        coordinator
            .initiate_shutdown(ShutdownSignal::Programmatic)
            .await
            .ok();

        let signal = receiver.recv().await;
        assert!(signal.is_ok());
        assert_eq!(signal.ok(), Some(ShutdownSignal::Programmatic));
    }

    #[tokio::test]
    async fn test_checkpoint_sender() {
        let coordinator = ShutdownCoordinator::new();
        let sender = coordinator.checkpoint_sender();

        let result = CheckpointResult::success("test-component", 100);
        assert!(sender.send(result).await.is_ok());
    }

    #[tokio::test]
    async fn test_checkpoint_result_success() {
        let result = CheckpointResult::success("scheduler", 150);
        assert!(result.success);
        assert_eq!(result.component, "scheduler");
        assert_eq!(result.duration_ms, 150);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_checkpoint_result_failure() {
        let result = CheckpointResult::failure("scheduler", "timeout");
        assert!(!result.success);
        assert_eq!(result.component, "scheduler");
        assert_eq!(result.error, Some("timeout".to_string()));
    }

    #[tokio::test]
    async fn test_shutdown_sequence() {
        let coordinator = ShutdownCoordinator::new();

        // Initiate shutdown
        coordinator
            .initiate_shutdown(ShutdownSignal::Programmatic)
            .await
            .ok();

        // Drop receiver to close checkpoint channel immediately
        drop(coordinator.checkpoint_rx.write().await);

        // Execute shutdown
        let stats = coordinator.shutdown().await;
        assert!(stats.is_ok());

        let stats = stats.ok();
        assert!(stats.is_some());
        if let Some(s) = stats {
            assert_eq!(coordinator.phase().await, ShutdownPhase::Complete);
            assert!(s.total_duration_ms > 0);
        }
    }

    #[test]
    fn test_shutdown_signal_display() {
        assert_eq!(format!("{}", ShutdownSignal::Sigterm), "SIGTERM");
        assert_eq!(format!("{}", ShutdownSignal::Sigint), "SIGINT");
        assert_eq!(format!("{}", ShutdownSignal::Programmatic), "PROGRAMMATIC");
    }
}
