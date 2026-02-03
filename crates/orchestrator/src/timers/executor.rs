//! Timer execution handling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info};

use super::scheduler::{DurableTimer, TimerScheduler};

/// Result of timer execution.
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Timer executed successfully.
    Success,
    /// Timer execution failed.
    Failed {
        /// Error message
        error: String,
    },
    /// Timer execution should be retried.
    Retry {
        /// Delay before retry
        delay_secs: u64,
    },
}

impl ExecutionResult {
    /// Check if execution was successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Check if execution should be retried.
    #[must_use]
    pub fn should_retry(&self) -> bool {
        matches!(self, Self::Retry { .. })
    }
}

/// Callback for timer execution.
#[async_trait]
pub trait TimerCallback: Send + Sync {
    /// Execute the timer.
    ///
    /// Called when the timer fires.
    async fn execute(&self, timer: &DurableTimer) -> ExecutionResult;
}

/// A callback that logs timer execution.
pub struct LoggingCallback;

#[async_trait]
impl TimerCallback for LoggingCallback {
    async fn execute(&self, timer: &DurableTimer) -> ExecutionResult {
        info!(
            timer_id = %timer.id(),
            payload = %timer.payload(),
            "Timer executed"
        );
        ExecutionResult::Success
    }
}

/// A callback that does nothing (for testing).
pub struct NoopCallback;

#[async_trait]
impl TimerCallback for NoopCallback {
    async fn execute(&self, _timer: &DurableTimer) -> ExecutionResult {
        ExecutionResult::Success
    }
}

/// Configuration for the timer executor.
#[derive(Debug, Clone)]
pub struct TimerExecutorConfig {
    /// Tick interval for polling timers.
    pub tick_interval_ms: u64,
    /// Maximum concurrent executions.
    pub max_concurrent: usize,
    /// Maximum retries per timer.
    pub max_retries: u32,
    /// Default retry delay in seconds.
    pub default_retry_delay_secs: u64,
}

impl Default for TimerExecutorConfig {
    fn default() -> Self {
        Self {
            tick_interval_ms: 100,
            max_concurrent: 100,
            max_retries: 3,
            default_retry_delay_secs: 5,
        }
    }
}

/// Executes fired timers.
pub struct TimerExecutor {
    config: TimerExecutorConfig,
    scheduler: Arc<TimerScheduler>,
    callbacks: Arc<RwLock<HashMap<String, Arc<dyn TimerCallback>>>>,
    default_callback: Arc<dyn TimerCallback>,
    running: Arc<RwLock<bool>>,
    in_flight: Arc<RwLock<usize>>,
}

impl TimerExecutor {
    /// Create a new timer executor.
    #[must_use]
    pub fn new(
        config: TimerExecutorConfig,
        scheduler: Arc<TimerScheduler>,
        default_callback: Arc<dyn TimerCallback>,
    ) -> Self {
        Self {
            config,
            scheduler,
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            default_callback,
            running: Arc::new(RwLock::new(false)),
            in_flight: Arc::new(RwLock::new(0)),
        }
    }

    /// Create an executor with logging callback.
    #[must_use]
    pub fn with_logging(config: TimerExecutorConfig, scheduler: Arc<TimerScheduler>) -> Self {
        Self::new(config, scheduler, Arc::new(LoggingCallback))
    }

    /// Create an executor with noop callback (for testing).
    #[must_use]
    pub fn with_noop(config: TimerExecutorConfig, scheduler: Arc<TimerScheduler>) -> Self {
        Self::new(config, scheduler, Arc::new(NoopCallback))
    }

    /// Register a callback for a specific callback ID.
    pub async fn register_callback(
        &self,
        callback_id: impl Into<String>,
        callback: Arc<dyn TimerCallback>,
    ) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.insert(callback_id.into(), callback);
    }

    /// Unregister a callback.
    pub async fn unregister_callback(&self, callback_id: &str) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.remove(callback_id);
    }

    /// Start the executor loop.
    ///
    /// This runs until stop() is called.
    pub async fn start(&self) {
        {
            let mut running = self.running.write().await;
            if *running {
                return;
            }
            *running = true;
        }

        info!("Timer executor starting");

        let tick_duration = Duration::from_millis(self.config.tick_interval_ms);
        let mut ticker = interval(tick_duration);

        loop {
            ticker.tick().await;

            // Check if stopped
            {
                let running = self.running.read().await;
                if !*running {
                    break;
                }
            }

            // Poll for due timers
            let due_timers = self.scheduler.poll_due().await;

            for timer in due_timers {
                // Check concurrency limit
                {
                    let in_flight = self.in_flight.read().await;
                    if *in_flight >= self.config.max_concurrent {
                        debug!("Concurrency limit reached, deferring execution");
                        break;
                    }
                }

                // Execute timer
                self.execute_timer(timer).await;
            }
        }

        info!("Timer executor stopped");
    }

    /// Stop the executor.
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    /// Check if the executor is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get the number of in-flight executions.
    pub async fn in_flight_count(&self) -> usize {
        *self.in_flight.read().await
    }

    /// Execute a single timer.
    async fn execute_timer(&self, timer: DurableTimer) {
        // Increment in-flight counter
        {
            let mut in_flight = self.in_flight.write().await;
            *in_flight = in_flight.saturating_add(1);
        }

        let timer_id = timer.id().clone();

        // Get callback
        let callback = self.get_callback(&timer).await;

        // Execute
        let result = callback.execute(&timer).await;

        match result {
            ExecutionResult::Success => {
                debug!(timer_id = %timer_id, "Timer execution succeeded");
                self.scheduler.acknowledge(&timer_id).await;
            }
            ExecutionResult::Failed { error } => {
                error!(timer_id = %timer_id, error = %error, "Timer execution failed");
                self.scheduler.acknowledge(&timer_id).await;
            }
            ExecutionResult::Retry { delay_secs } => {
                debug!(
                    timer_id = %timer_id,
                    delay_secs = %delay_secs,
                    "Timer execution requested retry"
                );
                // Acknowledge and reschedule would go here
                self.scheduler.acknowledge(&timer_id).await;
            }
        }

        // Decrement in-flight counter
        {
            let mut in_flight = self.in_flight.write().await;
            *in_flight = in_flight.saturating_sub(1);
        }
    }

    /// Get the callback for a timer.
    async fn get_callback(&self, timer: &DurableTimer) -> Arc<dyn TimerCallback> {
        if let Some(callback_id) = timer.callback_id() {
            let callbacks = self.callbacks.read().await;
            if let Some(callback) = callbacks.get(callback_id) {
                return Arc::clone(callback);
            }
        }

        Arc::clone(&self.default_callback)
    }

    /// Execute a single tick (for testing).
    pub async fn tick(&self) {
        let due_timers = self.scheduler.poll_due().await;

        for timer in due_timers {
            self.execute_timer(timer).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timers::scheduler::TimerSchedulerConfig;

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::Success;
        assert!(result.is_success());
        assert!(!result.should_retry());
    }

    #[test]
    fn test_execution_result_retry() {
        let result = ExecutionResult::Retry { delay_secs: 5 };
        assert!(!result.is_success());
        assert!(result.should_retry());
    }

    #[test]
    fn test_executor_config_default() {
        let config = TimerExecutorConfig::default();
        assert_eq!(config.tick_interval_ms, 100);
        assert_eq!(config.max_concurrent, 100);
        assert_eq!(config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_executor_with_noop() {
        let scheduler = Arc::new(TimerScheduler::new(TimerSchedulerConfig::default()));
        let executor = TimerExecutor::with_noop(TimerExecutorConfig::default(), scheduler);

        assert!(!executor.is_running().await);
    }

    #[tokio::test]
    async fn test_executor_register_callback() {
        let scheduler = Arc::new(TimerScheduler::new(TimerSchedulerConfig::default()));
        let executor = TimerExecutor::with_noop(TimerExecutorConfig::default(), scheduler);

        executor
            .register_callback("test-cb", Arc::new(LoggingCallback))
            .await;

        // Callback is registered (no direct way to check, but should not panic)
    }

    #[tokio::test]
    async fn test_executor_tick() {
        let scheduler = Arc::new(TimerScheduler::new(TimerSchedulerConfig::default()));

        // Schedule a due timer
        let past = chrono::Utc::now() - chrono::Duration::seconds(1);
        let timer = DurableTimer::new(past, serde_json::json!({"test": true}));
        let _ = scheduler.schedule(timer).await;

        let executor = TimerExecutor::with_noop(TimerExecutorConfig::default(), scheduler.clone());

        // Tick should execute the timer
        executor.tick().await;

        // Timer should be acknowledged (removed from fired list)
        assert_eq!(scheduler.fired_count().await, 0);
    }

    #[tokio::test]
    async fn test_executor_with_custom_callback() {
        struct CountingCallback {
            count: Arc<RwLock<u32>>,
        }

        #[async_trait]
        impl TimerCallback for CountingCallback {
            async fn execute(&self, _timer: &DurableTimer) -> ExecutionResult {
                let mut count = self.count.write().await;
                *count += 1;
                ExecutionResult::Success
            }
        }

        let count = Arc::new(RwLock::new(0u32));
        let callback = Arc::new(CountingCallback {
            count: Arc::clone(&count),
        });

        let scheduler = Arc::new(TimerScheduler::new(TimerSchedulerConfig::default()));

        // Schedule a due timer
        let past = chrono::Utc::now() - chrono::Duration::seconds(1);
        let timer = DurableTimer::new(past, serde_json::json!({}));
        let _ = scheduler.schedule(timer).await;

        let executor = TimerExecutor::new(TimerExecutorConfig::default(), scheduler, callback);

        executor.tick().await;

        // Callback should have been executed
        let final_count = *count.read().await;
        assert_eq!(final_count, 1);
    }

    #[tokio::test]
    async fn test_executor_in_flight_tracking() {
        let scheduler = Arc::new(TimerScheduler::new(TimerSchedulerConfig::default()));
        let executor = TimerExecutor::with_noop(TimerExecutorConfig::default(), scheduler);

        assert_eq!(executor.in_flight_count().await, 0);
    }

    #[tokio::test]
    async fn test_executor_callback_routing() {
        struct SpecificCallback {
            name: String,
            executed: Arc<RwLock<Vec<String>>>,
        }

        #[async_trait]
        impl TimerCallback for SpecificCallback {
            async fn execute(&self, _timer: &DurableTimer) -> ExecutionResult {
                let mut executed = self.executed.write().await;
                executed.push(self.name.clone());
                ExecutionResult::Success
            }
        }

        let executed = Arc::new(RwLock::new(Vec::new()));

        let scheduler = Arc::new(TimerScheduler::new(TimerSchedulerConfig::default()));

        // Schedule a timer with specific callback
        let past = chrono::Utc::now() - chrono::Duration::seconds(1);
        let timer = DurableTimer::new(past, serde_json::json!({})).with_callback("specific");
        let _ = scheduler.schedule(timer).await;

        let default_cb = Arc::new(SpecificCallback {
            name: "default".to_string(),
            executed: Arc::clone(&executed),
        });

        let specific_cb = Arc::new(SpecificCallback {
            name: "specific".to_string(),
            executed: Arc::clone(&executed),
        });

        let executor = TimerExecutor::new(TimerExecutorConfig::default(), scheduler, default_cb);

        executor.register_callback("specific", specific_cb).await;

        executor.tick().await;

        let final_executed = executed.read().await;
        assert_eq!(final_executed.len(), 1);
        assert_eq!(final_executed[0], "specific");
    }
}
