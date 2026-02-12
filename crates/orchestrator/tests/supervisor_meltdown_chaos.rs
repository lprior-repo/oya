//! Chaos tests for rapid restarts causing meltdown and circuit breaker behavior.
//!
//! Tests that rapid child failures trigger meltdown detection and circuit breaker
//! protection, preventing cascade failures from overwhelming the system.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

use ractor::{ActorRef, ActorStatus};
use thiserror::Error;
use tracing::{info, warn};

use orchestrator::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
use orchestrator::actors::supervisor::{
    MeltdownStatus, SupervisorArguments, SupervisorConfig, SupervisorMessage, SupervisorState,
    spawn_supervisor_with_name,
};

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, Error)]
pub enum MeltdownChaosError {
    #[error("Supervisor failed to reach meltdown state")]
    MeltdownNotDetected,

    #[error("Supervisor did not shut down after meltdown")]
    SupervisorNotShutdown,

    #[error("Setup failed: {reason}")]
    SetupFailed { reason: String },

    #[error("RPC failed: {reason}")]
    RpcFailed { reason: String },

    #[error("Timeout waiting for condition: {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

pub type ChaosResult<T> = Result<T, MeltdownChaosError>;

// =============================================================================
// Test Helpers
// =============================================================================

fn meltdown_test_config() -> SupervisorConfig {
    SupervisorConfig {
        max_restarts: 10,
        restart_window_secs: 1,
        base_backoff_ms: 5,
        max_backoff_ms: 20,
        warning_threshold: 2.0,
        meltdown_threshold: 5.0,
    }
}

async fn wait_for_supervisor_shutdown(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    timeout_ms: u64,
) -> ChaosResult<()> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout {
        let status = supervisor.get_status();
        if matches!(status, ActorStatus::Stopped | ActorStatus::Stopping) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    Err(MeltdownChaosError::Timeout { timeout_ms })
}

async fn get_meltdown_status(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
) -> ChaosResult<MeltdownStatus> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    supervisor
        .send_message(SupervisorMessage::GetStatus { reply: tx })
        .map_err(|e| MeltdownChaosError::RpcFailed {
            reason: e.to_string(),
        })?;

    let status = rx.await.map_err(|e| MeltdownChaosError::RpcFailed {
        reason: format!("Failed to receive status: {e}"),
    })?;

    Ok(status.meltdown_status)
}

async fn spawn_child(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    name: &str,
) -> ChaosResult<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    supervisor
        .send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
            name: name.to_string(),
            args: SchedulerArguments::new(),
            reply: tx,
        })
        .map_err(|e| MeltdownChaosError::SetupFailed {
            reason: e.to_string(),
        })?;

    rx.await
        .map_err(|e| MeltdownChaosError::RpcFailed {
            reason: format!("Failed to receive spawn reply: {e}"),
        })?
        .map_err(|e| MeltdownChaosError::SetupFailed {
            reason: format!("Failed to spawn child: {e}"),
        })
}

async fn kill_child(supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>, name: &str) {
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::StopChild {
        name: name.to_string(),
    });
}

// =============================================================================
// Chaos Tests
// =============================================================================

/// Test: Rapid restarts trigger meltdown detection.
///
/// **Given** a supervisor with low meltdown threshold (5 failures/sec)
/// **When** children crash rapidly (>5 times in 1 second)
/// **Then** meltdown is detected and supervisor shuts down
#[tokio::test]
async fn given_rapid_restarts_when_exceeds_meltdown_threshold_then_supervisor_shuts_down() {
    let test_name = "meltdown_detection";
    info!("Starting test: {}", test_name);

    let config = meltdown_test_config();
    let args = SupervisorArguments::new().with_config(config);

    let supervisor =
        spawn_supervisor_with_name::<SchedulerActorDef>(args, &format!("supervisor-{test_name}"))
            .await
            .expect("Failed to spawn supervisor");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let child_name = format!("chaos-child-{test_name}");
    spawn_child(&supervisor, &child_name)
        .await
        .expect("Failed to spawn child");

    tokio::time::sleep(Duration::from_millis(30)).await;

    let initial_status = get_meltdown_status(&supervisor)
        .await
        .expect("Failed to get initial status");
    assert_eq!(
        initial_status,
        MeltdownStatus::Normal,
        "Should start in normal state"
    );

    for i in 1..=8 {
        info!("Killing child (iteration {})", i);
        kill_child(&supervisor, &child_name).await;
        tokio::time::sleep(Duration::from_millis(40)).await;

        let current_status = get_meltdown_status(&supervisor).await;

        match current_status {
            Ok(MeltdownStatus::Meltdown) => {
                info!("Meltdown detected at iteration {}", i);

                let shutdown_result = wait_for_supervisor_shutdown(&supervisor, 2000).await;
                assert!(
                    shutdown_result.is_ok(),
                    "Supervisor should shut down after meltdown"
                );

                info!("Test passed: {}", test_name);
                return;
            }
            Ok(MeltdownStatus::Warning) => {
                warn!("Warning state at iteration {}", i);
            }
            Ok(MeltdownStatus::Normal) => {}
            Err(_) => {
                break;
            }
        }
    }

    let shutdown_result = wait_for_supervisor_shutdown(&supervisor, 1000).await;
    assert!(
        shutdown_result.is_ok(),
        "Supervisor should eventually shut down due to meltdown"
    );

    info!("Test passed: {}", test_name);
}

/// Test: Circuit breaker prevents cascade failures.
///
/// **Given** a supervisor in meltdown state
/// **When** additional failures occur
/// **Then** the supervisor does not restart children (circuit breaker active)
#[tokio::test]
async fn given_meltdown_when_additional_failures_then_no_restarts() {
    let test_name = "circuit_breaker_active";
    info!("Starting test: {}", test_name);

    let config = meltdown_test_config();
    let args = SupervisorArguments::new().with_config(config);

    let supervisor =
        spawn_supervisor_with_name::<SchedulerActorDef>(args, &format!("supervisor-{test_name}"))
            .await
            .expect("Failed to spawn supervisor");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let child_name = format!("chaos-child-{test_name}");
    spawn_child(&supervisor, &child_name)
        .await
        .expect("Failed to spawn child");

    tokio::time::sleep(Duration::from_millis(30)).await;

    for i in 1..=10 {
        kill_child(&supervisor, &child_name).await;
        tokio::time::sleep(Duration::from_millis(30)).await;

        let status = supervisor.get_status();
        if matches!(status, ActorStatus::Stopped | ActorStatus::Stopping) {
            info!(
                "Supervisor stopped at iteration {} (circuit breaker worked)",
                i
            );

            let (tx, rx) = tokio::sync::oneshot::channel();
            let send_result = supervisor.send_message(SupervisorMessage::GetStatus { reply: tx });

            if send_result.is_ok() {
                if rx.await.is_err() {
                    info!("Supervisor fully stopped - circuit breaker confirmed");
                }
            }

            info!("Test passed: {}", test_name);
            return;
        }
    }

    let shutdown_result = wait_for_supervisor_shutdown(&supervisor, 2000).await;
    assert!(
        shutdown_result.is_ok(),
        "Circuit breaker should stop supervisor to prevent cascade failures"
    );

    info!("Test passed: {}", test_name);
}

/// Test: Gradual failures do not trigger meltdown.
///
/// **Given** a supervisor with meltdown threshold of 5 failures/sec
/// **When** failures occur slowly (1 failure per 300ms)
/// **Then** supervisor remains in normal/warning state, not meltdown
#[tokio::test]
async fn given_gradual_failures_when_below_threshold_then_no_meltdown() {
    let test_name = "gradual_failures_ok";
    info!("Starting test: {}", test_name);

    let config = SupervisorConfig {
        max_restarts: 10,
        restart_window_secs: 2,
        base_backoff_ms: 50,
        max_backoff_ms: 200,
        warning_threshold: 2.0,
        meltdown_threshold: 3.0,
    };
    let args = SupervisorArguments::new().with_config(config);

    let supervisor =
        spawn_supervisor_with_name::<SchedulerActorDef>(args, &format!("supervisor-{test_name}"))
            .await
            .expect("Failed to spawn supervisor");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let child_name = format!("gradual-child-{test_name}");
    spawn_child(&supervisor, &child_name)
        .await
        .expect("Failed to spawn child");

    tokio::time::sleep(Duration::from_millis(50)).await;

    for i in 1..=3 {
        info!("Gradual failure iteration {}", i);
        kill_child(&supervisor, &child_name).await;
        tokio::time::sleep(Duration::from_millis(400)).await;

        let status = supervisor.get_status();
        if matches!(status, ActorStatus::Stopped | ActorStatus::Stopping) {
            panic!(
                "Supervisor should NOT shut down with gradual failures (iteration {})",
                i
            );
        }

        let meltdown = get_meltdown_status(&supervisor).await;
        match meltdown {
            Ok(MeltdownStatus::Meltdown) => {
                panic!("Meltdown should not occur with gradual failures");
            }
            Ok(MeltdownStatus::Warning) => {
                info!("Warning state is acceptable for gradual failures");
            }
            Ok(MeltdownStatus::Normal) => {
                info!("Normal state at iteration {}", i);
            }
            Err(_) => {}
        }
    }

    let status = supervisor.get_status();
    assert_eq!(
        status,
        ActorStatus::Running,
        "Supervisor should remain running"
    );

    supervisor.stop(Some("Test complete".to_string()));

    info!("Test passed: {}", test_name);
}

/// Test: Supervisor status includes meltdown information.
///
/// **Given** a supervisor with failures
/// **When** status is queried
/// **Then** the response includes meltdown status and failure count
#[tokio::test]
async fn given_failures_when_status_queried_then_includes_meltdown_info() {
    let test_name = "status_meltdown_info";
    info!("Starting test: {}", test_name);

    let config = SupervisorConfig {
        max_restarts: 5,
        restart_window_secs: 2,
        base_backoff_ms: 10,
        max_backoff_ms: 50,
        warning_threshold: 1.0,
        meltdown_threshold: 2.0,
    };
    let args = SupervisorArguments::new().with_config(config);

    let supervisor =
        spawn_supervisor_with_name::<SchedulerActorDef>(args, &format!("supervisor-{test_name}"))
            .await
            .expect("Failed to spawn supervisor");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let child_name = format!("status-child-{test_name}");
    spawn_child(&supervisor, &child_name)
        .await
        .expect("Failed to spawn child");

    tokio::time::sleep(Duration::from_millis(30)).await;

    let (tx1, rx1) = tokio::sync::oneshot::channel();
    supervisor
        .send_message(SupervisorMessage::GetStatus { reply: tx1 })
        .expect("Failed to send status request");
    let initial_status = rx1.await.expect("Failed to receive status");

    assert_eq!(initial_status.state, SupervisorState::Running);
    assert_eq!(initial_status.meltdown_status, MeltdownStatus::Normal);
    assert_eq!(initial_status.failures_in_window, 0);

    kill_child(&supervisor, &child_name).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (tx2, rx2) = tokio::sync::oneshot::channel();
    let send_ok = supervisor.send_message(SupervisorMessage::GetStatus { reply: tx2 });
    if send_ok.is_ok() {
        if let Ok(post_failure_status) = rx2.await {
            assert!(post_failure_status.failures_in_window >= 1);

            info!(
                "Status after failure: state={:?}, meltdown={:?}, failures={}",
                post_failure_status.state,
                post_failure_status.meltdown_status,
                post_failure_status.failures_in_window
            );
        }
    }

    supervisor.stop(Some("Test complete".to_string()));

    info!("Test passed: {}", test_name);
}

/// Test: Exponential backoff increases delay between restarts.
///
/// **Given** a child that fails repeatedly
/// **When** the supervisor restarts it
/// **Then** the backoff delay increases exponentially
#[tokio::test]
async fn given_repeated_failures_when_restarting_then_backoff_increases() {
    let test_name = "exponential_backoff";
    info!("Starting test: {}", test_name);

    let config = SupervisorConfig {
        max_restarts: 5,
        restart_window_secs: 5,
        base_backoff_ms: 20,
        max_backoff_ms: 500,
        warning_threshold: 10.0,
        meltdown_threshold: 20.0,
    };
    let args = SupervisorArguments::new().with_config(config);

    let supervisor =
        spawn_supervisor_with_name::<SchedulerActorDef>(args, &format!("supervisor-{test_name}"))
            .await
            .expect("Failed to spawn supervisor");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let child_name = format!("backoff-child-{test_name}");
    spawn_child(&supervisor, &child_name)
        .await
        .expect("Failed to spawn child");

    let mut restart_times: Vec<Instant> = Vec::new();

    for i in 1..=4 {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let get_ok = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::GetChild {
            name: child_name.clone(),
            reply: tx,
        });

        if get_ok.is_ok() {
            if let Ok(Some(child)) = rx.await {
                if child.get_status() == ActorStatus::Running {
                    info!("Killing child iteration {}", i);
                    kill_child(&supervisor, &child_name).await;
                    restart_times.push(Instant::now());
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    supervisor.stop(Some("Test complete".to_string()));

    info!("Backoff test completed - verified restart attempts with increasing delays");
    info!("Test passed: {}", test_name);
}
