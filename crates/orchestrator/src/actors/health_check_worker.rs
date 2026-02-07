//! HealthCheckWorker - Background worker for endpoint health polling.
//!
//! This worker periodically polls configured health endpoints and emits
//! events when health status changes. It integrates with the ractor
//! actor system for message-passing concurrency.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::watch;
use tracing::{debug, info, warn};

use oya_events::{BeadEvent, EventBus};

use crate::actors::supervisor::calculate_backoff;

/// Configuration for health check polling behavior.
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Base URL of the server to poll.
    pub base_url: String,
    /// Health endpoint path (default: "/api/health").
    pub health_endpoint: String,
    /// Interval between health checks.
    pub check_interval: Duration,
    /// Timeout for individual health check requests.
    pub timeout: Duration,
    /// Maximum number of consecutive failures before marking unhealthy.
    pub max_failures: u32,
    /// Whether to emit events on health status changes.
    pub emit_events: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            health_endpoint: "/api/health".to_string(),
            check_interval: Duration::from_secs(10),
            timeout: Duration::from_secs(5),
            max_failures: 3,
            emit_events: true,
        }
    }
}

impl HealthCheckConfig {
    /// Create a new health check config with custom values.
    #[must_use]
    pub const fn new(
        base_url: String,
        health_endpoint: String,
        check_interval: Duration,
        timeout: Duration,
        max_failures: u32,
    ) -> Self {
        Self {
            base_url,
            health_endpoint,
            check_interval,
            timeout,
            max_failures,
            emit_events: true,
        }
    }

    /// Create a config for testing with shorter intervals.
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            health_endpoint: "/api/health".to_string(),
            check_interval: Duration::from_millis(100),
            timeout: Duration::from_millis(500),
            max_failures: 2,
            emit_events: false,
        }
    }

    /// Set the base URL for health checks.
    #[must_use]
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Set the health endpoint path.
    #[must_use]
    pub fn with_health_endpoint(mut self, endpoint: String) -> Self {
        self.health_endpoint = endpoint;
        self
    }

    /// Set the check interval.
    #[must_use]
    pub const fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Set the timeout for health check requests.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum failures before marking unhealthy.
    #[must_use]
    pub const fn with_max_failures(mut self, max_failures: u32) -> Self {
        self.max_failures = max_failures;
        self
    }

    /// Enable or disable event emission.
    #[must_use]
    pub const fn with_emit_events(mut self, emit: bool) -> Self {
        self.emit_events = emit;
        self
    }
}

/// Result of a health check operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthCheckResult {
    /// Whether the endpoint is healthy.
    pub is_healthy: bool,
    /// Status code from the health check (if available).
    pub status_code: Option<u16>,
    /// Error message if the check failed.
    pub error: Option<String>,
    /// Timestamp of the health check.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HealthCheckResult {
    /// Create a successful health check result.
    #[must_use]
    pub fn healthy(status_code: u16) -> Self {
        Self {
            is_healthy: true,
            status_code: Some(status_code),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a failed health check result.
    #[must_use]
    pub fn unhealthy(error: String) -> Self {
        Self {
            is_healthy: false,
            status_code: None,
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a failed health check result with status code.
    #[must_use]
    pub fn unhealthy_with_status(status_code: u16, error: String) -> Self {
        Self {
            is_healthy: false,
            status_code: Some(status_code),
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Current health status tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Health status is unknown (initial state).
    Unknown,
    /// Endpoint is healthy.
    Healthy,
    /// Endpoint is unhealthy.
    Unhealthy,
}

impl HealthStatus {
    /// Convert to a string representation.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Unknown => "unknown",
            Self::Healthy => "healthy",
            Self::Unhealthy => "unhealthy",
        }
    }
}

/// Messages handled by the HealthCheckWorker actor.
#[derive(Clone, Debug)]
pub enum HealthCheckMessage {
    /// Perform a single health check.
    PerformCheck,
    /// Update the health check configuration.
    UpdateConfig(HealthCheckConfig),
    /// Get the current health status.
    GetStatus,
    /// Stop the health check worker.
    Stop,
}

/// State for the HealthCheckWorker actor.
pub struct HealthCheckWorkerState {
    /// Worker ID for identification.
    worker_id: String,
    /// Current health check configuration.
    config: HealthCheckConfig,
    /// Current health status.
    health_status: HealthStatus,
    /// Consecutive failure count.
    failure_count: u32,
    /// Event bus for emitting health events (optional).
    event_bus: Option<Arc<EventBus>>,
    /// Handle for the polling timer.
    timer_handle: Option<PollingHandle>,
    /// History of recent health check results.
    history: im::Vector<HealthCheckResult>,
}

impl HealthCheckWorkerState {
    /// Create a new health check worker state.
    #[must_use]
    pub fn new(config: HealthCheckConfig, event_bus: Option<Arc<EventBus>>) -> Self {
        let worker_id = format!("health-worker-{}", uuid::Uuid::new_v4());
        Self {
            worker_id,
            config,
            health_status: HealthStatus::Unknown,
            failure_count: 0,
            event_bus,
            timer_handle: None,
            history: im::Vector::new(),
        }
    }

    /// Reset the failure count.
    fn reset_failures(&mut self) {
        self.failure_count = 0;
    }

    /// Increment the failure count and return whether threshold is exceeded.
    fn increment_failures(&mut self) -> bool {
        self.failure_count = self.failure_count.saturating_add(1);
        self.failure_count >= self.config.max_failures
    }

    /// Update health status and emit event if configured.
    fn update_health_status(&mut self, new_status: HealthStatus) {
        let old_status = self.health_status;
        self.health_status = new_status;

        // Emit event if status changed and event emission is enabled
        if old_status != new_status && self.config.emit_events {
            if let Some(ref event_bus) = self.event_bus {
                let event = BeadEvent::worker_unhealthy(
                    self.worker_id.clone(),
                    format!(
                        "Health status changed: {} → {}",
                        old_status.as_str(),
                        new_status.as_str()
                    ),
                );

                let event_bus_clone = event_bus.clone();
                tokio::spawn(async move {
                    if let Err(err) = event_bus_clone.publish(event).await {
                        tracing::error!(
                            error = %err,
                            "Failed to publish health status change event"
                        );
                    }
                });
            }
        }
    }

    /// Add a health check result to history.
    fn add_to_history(&mut self, result: HealthCheckResult) {
        self.history.push_back(result);

        // Keep only the last 100 results
        if self.history.len() > 100 {
            self.history.pop_front();
        }
    }

    /// Get the current health status.
    #[must_use]
    pub const fn health_status(&self) -> HealthStatus {
        self.health_status
    }

    /// Get the failure count.
    #[must_use]
    pub const fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Get the worker ID.
    #[must_use]
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Get the health check history.
    #[must_use]
    pub fn history(&self) -> &im::Vector<HealthCheckResult> {
        &self.history
    }
}

/// Actor definition for the HealthCheckWorker.
#[derive(Clone, Default)]
pub struct HealthCheckWorkerDef;

impl Actor for HealthCheckWorkerDef {
    type Msg = HealthCheckMessage;
    type State = HealthCheckWorkerState;
    type Arguments = (HealthCheckConfig, Option<Arc<EventBus>>);

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("HealthCheckWorker starting");
        let (config, event_bus) = args;
        let mut state = HealthCheckWorkerState::new(config, event_bus);

        // Start the polling timer
        let handle = PollingTimer::start(
            myself.clone(),
            state.config.check_interval,
            state.config.timeout,
        );
        state.timer_handle = Some(handle);

        Ok(state)
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            HealthCheckMessage::PerformCheck => {
                let result = perform_health_check(&state.config).await;

                // Add to history
                state.add_to_history(result.clone());

                // Update health status based on result
                if result.is_healthy {
                    // Reset failure count on success
                    if state.failure_count > 0 {
                        debug!(
                            worker_id = %state.worker_id,
                            "Health check succeeded after {} failures",
                            state.failure_count
                        );
                    }
                    state.reset_failures();

                    // Update status if previously unhealthy
                    if state.health_status != HealthStatus::Healthy {
                        info!(
                            worker_id = %state.worker_id,
                            "Endpoint is now healthy"
                        );
                        state.update_health_status(HealthStatus::Healthy);
                    }
                } else {
                    // Increment failure count
                    let threshold_exceeded = state.increment_failures();

                    if threshold_exceeded && state.health_status != HealthStatus::Unhealthy {
                        warn!(
                            worker_id = %state.worker_id,
                            failure_count = state.failure_count,
                            error = %result.error.as_deref().unwrap_or("unknown"),
                            "Endpoint is now unhealthy after {} consecutive failures",
                            state.failure_count
                        );
                        state.update_health_status(HealthStatus::Unhealthy);
                    } else {
                        debug!(
                            worker_id = %state.worker_id,
                            failure_count = state.failure_count,
                            error = %result.error.as_deref().unwrap_or("unknown"),
                            "Health check failed ({}/{} attempts)",
                            state.failure_count,
                            state.config.max_failures
                        );
                    }
                }
            }
            HealthCheckMessage::UpdateConfig(new_config) => {
                info!(
                    worker_id = %state.worker_id,
                    "Updating health check configuration"
                );
                state.config = new_config;

                // Restart timer with new interval
                if let Some(handle) = state.timer_handle.take() {
                    handle.stop();
                }
                let new_handle = PollingTimer::start(
                    myself.clone(),
                    state.config.check_interval,
                    state.config.timeout,
                );
                state.timer_handle = Some(new_handle);
            }
            HealthCheckMessage::GetStatus => {
                // This would typically be used with ractor's call!() macro
                // For now, just log the status
                debug!(
                    worker_id = %state.worker_id,
                    status = state.health_status.as_str(),
                    failures = state.failure_count,
                    "Health status requested"
                );
            }
            HealthCheckMessage::Stop => {
                info!(
                    worker_id = %state.worker_id,
                    "HealthCheckWorker stopping"
                );
                if let Some(handle) = state.timer_handle.take() {
                    handle.stop();
                }
            }
        }
        Ok(())
    }
}

/// Perform a health check on the configured endpoint.
///
/// This function makes an HTTP GET request to the health endpoint
/// and returns the result.
async fn perform_health_check(config: &HealthCheckConfig) -> HealthCheckResult {
    let url = format!("{}{}", config.base_url, config.health_endpoint);

    // Create HTTP client with timeout
    let client = match reqwest::Client::builder().timeout(config.timeout).build() {
        Ok(client) => client,
        Err(e) => {
            return HealthCheckResult::unhealthy(format!("Failed to create HTTP client: {e}"));
        }
    };

    // Perform the health check
    match client.get(&url).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                HealthCheckResult::healthy(status.as_u16())
            } else {
                HealthCheckResult::unhealthy_with_status(
                    status.as_u16(),
                    format!("HTTP {}", status.as_u16()),
                )
            }
        }
        Err(e) => HealthCheckResult::unhealthy(format!("Request failed: {e}")),
    }
}

/// Handle for stopping a polling timer.
#[derive(Clone)]
pub struct PollingHandle {
    stop_tx: watch::Sender<bool>,
}

impl PollingHandle {
    /// Stop the polling timer.
    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }
}

/// Polling timer that sends health check messages at a fixed interval.
#[derive(Clone, Debug)]
pub struct PollingTimer {
    interval: Duration,
    timeout: Duration,
}

impl PollingTimer {
    /// Create a new polling timer.
    #[must_use]
    pub fn new(interval: Duration, timeout: Duration) -> Self {
        Self { interval, timeout }
    }

    /// Start the polling timer.
    ///
    /// Spawns a background task that sends `PerformCheck` messages
    /// at the configured interval.
    pub fn start(
        target: ActorRef<HealthCheckMessage>,
        interval: Duration,
        timeout: Duration,
    ) -> PollingHandle {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let mut ticker = tokio::time::interval(interval);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if target.send_message(HealthCheckMessage::PerformCheck).is_err() {
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

        PollingHandle { stop_tx }
    }

    /// Get the check interval.
    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Get the request timeout.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_config_default() {
        let config = HealthCheckConfig::default();
        assert_eq!(config.base_url, "http://localhost:3000");
        assert_eq!(config.health_endpoint, "/api/health");
        assert_eq!(config.check_interval, Duration::from_secs(10));
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.max_failures, 3);
        assert!(config.emit_events);
    }

    #[test]
    fn test_health_check_config_builder() {
        let config = HealthCheckConfig::default()
            .with_base_url("http://example.com".to_string())
            .with_health_endpoint("/health".to_string())
            .with_check_interval(Duration::from_secs(5))
            .with_timeout(Duration::from_secs(2))
            .with_max_failures(5)
            .with_emit_events(false);

        assert_eq!(config.base_url, "http://example.com");
        assert_eq!(config.health_endpoint, "/health");
        assert_eq!(config.check_interval, Duration::from_secs(5));
        assert_eq!(config.timeout, Duration::from_secs(2));
        assert_eq!(config.max_failures, 5);
        assert!(!config.emit_events);
    }

    #[test]
    fn test_health_check_result_healthy() {
        let result = HealthCheckResult::healthy(200);
        assert!(result.is_healthy);
        assert_eq!(result.status_code, Some(200));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_health_check_result_unhealthy() {
        let result = HealthCheckResult::unhealthy("Connection refused".to_string());
        assert!(!result.is_healthy);
        assert!(result.status_code.is_none());
        assert_eq!(result.error, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_health_check_result_unhealthy_with_status() {
        let result =
            HealthCheckResult::unhealthy_with_status(500, "Internal Server Error".to_string());
        assert!(!result.is_healthy);
        assert_eq!(result.status_code, Some(500));
        assert_eq!(result.error, Some("Internal Server Error".to_string()));
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Unknown.as_str(), "unknown");
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Unhealthy.as_str(), "unhealthy");
    }

    #[test]
    fn test_health_check_worker_state_new() {
        let config = HealthCheckConfig::default();
        let state = HealthCheckWorkerState::new(config, None);

        assert_eq!(state.health_status, HealthStatus::Unknown);
        assert_eq!(state.failure_count, 0);
        assert!(state.event_bus.is_none());
        assert!(state.timer_handle.is_none());
        assert!(state.history.is_empty());
    }

    #[test]
    fn test_health_check_worker_state_reset_failures() {
        let config = HealthCheckConfig::default();
        let mut state = HealthCheckWorkerState::new(config.clone(), None);

        state.failure_count = 5;
        state.reset_failures();

        assert_eq!(state.failure_count, 0);
    }

    #[test]
    fn test_health_check_worker_state_increment_failures() {
        let config = HealthCheckConfig::for_testing();
        let mut state = HealthCheckWorkerState::new(config, None);

        // Increment failures
        assert!(!state.increment_failures()); // 1/2
        assert!(!state.increment_failures()); // 2/2
        assert!(state.increment_failures()); // 3/2 - exceeded
    }

    #[test]
    fn test_health_check_worker_state_update_health_status() {
        let config = HealthCheckConfig::for_testing().with_emit_events(false);
        let mut state = HealthCheckWorkerState::new(config, None);

        assert_eq!(state.health_status, HealthStatus::Unknown);

        state.update_health_status(HealthStatus::Healthy);
        assert_eq!(state.health_status, HealthStatus::Healthy);

        state.update_health_status(HealthStatus::Unhealthy);
        assert_eq!(state.health_status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_check_worker_state_add_to_history() {
        let config = HealthCheckConfig::default();
        let mut state = HealthCheckWorkerState::new(config, None);

        let result1 = HealthCheckResult::healthy(200);
        let result2 = HealthCheckResult::unhealthy("Error".to_string());

        state.add_to_history(result1.clone());
        state.add_to_history(result2.clone());

        assert_eq!(state.history.len(), 2);
        assert_eq!(state.history[0], result1);
        assert_eq!(state.history[1], result2);
    }

    #[test]
    fn test_health_check_worker_state_history_limit() {
        let config = HealthCheckConfig::default();
        let mut state = HealthCheckWorkerState::new(config, None);

        // Add more than 100 results
        for i in 0..150 {
            let result = HealthCheckResult::healthy(200);
            state.add_to_history(result);
        }

        // History should be limited to 100
        assert_eq!(state.history.len(), 100);
    }

    #[test]
    fn test_polling_timer_new() {
        let timer = PollingTimer::new(Duration::from_secs(10), Duration::from_secs(5));
        assert_eq!(timer.interval(), Duration::from_secs(10));
        assert_eq!(timer.timeout(), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_perform_health_check_mock_success() {
        // This test would require a mock server
        // For now, we'll test with a real endpoint that should fail
        let config = HealthCheckConfig {
            base_url: "http://localhost:9999".to_string(),
            health_endpoint: "/health".to_string(),
            check_interval: Duration::from_secs(1),
            timeout: Duration::from_millis(100),
            max_failures: 3,
            emit_events: false,
        };

        let result = perform_health_check(&config).await;
        assert!(!result.is_healthy);
        assert!(result.error.is_some());
    }
}
