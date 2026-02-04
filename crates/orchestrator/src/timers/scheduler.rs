//! Timer scheduling and management.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::persistence::TimerPersistence;
use crate::persistence::{OrchestratorStore, PersistenceResult};

/// Type alias for the timer priority queue.
type TimerQueue = Arc<RwLock<BinaryHeap<Reverse<(DateTime<Utc>, TimerId)>>>>;

/// Unique identifier for a timer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TimerId(String);

impl TimerId {
    /// Create a new unique timer ID.
    #[must_use]
    pub fn new() -> Self {
        Self(format!("timer-{}", Uuid::new_v4()))
    }

    /// Create a timer ID from an existing string.
    #[must_use]
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TimerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TimerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerStatus {
    /// Timer is waiting to fire.
    Pending,
    /// Timer has fired.
    Fired,
    /// Timer was cancelled.
    Cancelled,
    /// Timer execution failed.
    Failed,
}

/// Optional timer metadata for restoration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimerMetadata {
    /// Associated workflow ID
    pub workflow_id: Option<String>,
    /// Associated bead ID
    pub bead_id: Option<String>,
    /// Callback identifier
    pub callback_id: Option<String>,
}

impl TimerStatus {
    /// Check if the timer is pending.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Check if the timer has fired.
    #[must_use]
    pub fn is_fired(&self) -> bool {
        matches!(self, Self::Fired)
    }

    /// Check if the timer was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    /// Check if the timer is terminal (won't change).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Fired | Self::Cancelled | Self::Failed)
    }
}

/// A durable timer that persists and fires at a scheduled time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurableTimer {
    /// Unique timer identifier
    id: TimerId,
    /// When the timer should fire
    execute_at: DateTime<Utc>,
    /// Payload to deliver when fired
    payload: serde_json::Value,
    /// Current status
    status: TimerStatus,
    /// When the timer was created
    created_at: DateTime<Utc>,
    /// When the timer was last updated
    updated_at: DateTime<Utc>,
    /// Associated workflow ID
    workflow_id: Option<String>,
    /// Associated bead ID
    bead_id: Option<String>,
    /// Callback identifier
    callback_id: Option<String>,
}

impl DurableTimer {
    /// Create a new timer.
    #[must_use]
    pub fn new(execute_at: DateTime<Utc>, payload: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id: TimerId::new(),
            execute_at,
            payload,
            status: TimerStatus::Pending,
            created_at: now,
            updated_at: now,
            workflow_id: None,
            bead_id: None,
            callback_id: None,
        }
    }

    /// Create a timer with a delay from now.
    #[must_use]
    pub fn with_delay(delay_secs: i64, payload: serde_json::Value) -> Self {
        let execute_at = Utc::now() + chrono::Duration::seconds(delay_secs);
        Self::new(execute_at, payload)
    }

    /// Set the workflow ID.
    #[must_use]
    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    /// Set the bead ID.
    #[must_use]
    pub fn with_bead(mut self, bead_id: impl Into<String>) -> Self {
        self.bead_id = Some(bead_id.into());
        self
    }

    /// Set the callback ID.
    #[must_use]
    pub fn with_callback(mut self, callback_id: impl Into<String>) -> Self {
        self.callback_id = Some(callback_id.into());
        self
    }

    /// Get the timer ID.
    #[must_use]
    pub fn id(&self) -> &TimerId {
        &self.id
    }

    /// Restore a timer from persisted state.
    #[must_use]
    pub fn restore(
        id: TimerId,
        execute_at: DateTime<Utc>,
        payload: serde_json::Value,
        status: TimerStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        metadata: TimerMetadata,
    ) -> Self {
        Self {
            id,
            execute_at,
            payload,
            status,
            created_at,
            updated_at,
            workflow_id: metadata.workflow_id,
            bead_id: metadata.bead_id,
            callback_id: metadata.callback_id,
        }
    }

    /// Get when the timer should execute.
    #[must_use]
    pub fn execute_at(&self) -> DateTime<Utc> {
        self.execute_at
    }

    /// Get the payload.
    #[must_use]
    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    /// Get the status.
    #[must_use]
    pub fn status(&self) -> TimerStatus {
        self.status
    }

    /// Get the creation timestamp.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get the updated timestamp.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Get the workflow ID.
    #[must_use]
    pub fn workflow_id(&self) -> Option<&str> {
        self.workflow_id.as_deref()
    }

    /// Get the bead ID.
    #[must_use]
    pub fn bead_id(&self) -> Option<&str> {
        self.bead_id.as_deref()
    }

    /// Get the callback ID.
    #[must_use]
    pub fn callback_id(&self) -> Option<&str> {
        self.callback_id.as_deref()
    }

    /// Check if the timer is due (should fire now).
    #[must_use]
    pub fn is_due(&self) -> bool {
        self.status == TimerStatus::Pending && Utc::now() >= self.execute_at
    }

    /// Get the time until the timer fires (negative if overdue).
    #[must_use]
    pub fn time_until(&self) -> chrono::Duration {
        self.execute_at - Utc::now()
    }

    /// Mark the timer as fired.
    pub fn mark_fired(&mut self) {
        self.status = TimerStatus::Fired;
        self.updated_at = Utc::now();
    }

    /// Mark the timer as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = TimerStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    /// Mark the timer as failed.
    pub fn mark_failed(&mut self) {
        self.status = TimerStatus::Failed;
        self.updated_at = Utc::now();
    }

    /// Reschedule the timer after a delay.
    pub fn reschedule_after(&mut self, delay_secs: u64) {
        let delay_secs = delay_secs.min(i64::MAX as u64) as i64;
        self.status = TimerStatus::Pending;
        self.execute_at = Utc::now() + chrono::Duration::seconds(delay_secs);
        self.updated_at = Utc::now();
    }
}

/// Ordering for timers by execution time.
impl PartialOrd for DurableTimer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DurableTimer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.execute_at.cmp(&other.execute_at)
    }
}

impl PartialEq for DurableTimer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for DurableTimer {}

/// Configuration for the timer scheduler.
#[derive(Debug, Clone)]
pub struct TimerSchedulerConfig {
    /// Maximum number of timers to keep in memory.
    pub max_in_memory: usize,
    /// How far ahead to load timers from persistence.
    pub lookahead_secs: u64,
    /// Tick interval for checking timers.
    pub tick_interval_ms: u64,
}

impl Default for TimerSchedulerConfig {
    fn default() -> Self {
        Self {
            max_in_memory: 10_000,
            lookahead_secs: 300, // 5 minutes
            tick_interval_ms: 100,
        }
    }
}

/// Schedules and manages durable timers.
pub struct TimerScheduler {
    config: TimerSchedulerConfig,
    persistence: Option<TimerPersistence>,

    /// Pending timers indexed by ID
    timers: Arc<RwLock<HashMap<String, DurableTimer>>>,
    /// Priority queue of timer IDs by execution time
    queue: TimerQueue,
    /// Fired timers waiting for callback
    fired: Arc<RwLock<Vec<DurableTimer>>>,
}

impl TimerScheduler {
    /// Create a new in-memory timer scheduler.
    #[must_use]
    pub fn new(config: TimerSchedulerConfig) -> Self {
        Self {
            config,
            persistence: None,
            timers: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            fired: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a scheduler with persistent storage.
    #[must_use]
    pub fn with_store(config: TimerSchedulerConfig, store: OrchestratorStore) -> Self {
        Self {
            config,
            persistence: Some(TimerPersistence::new(store)),
            timers: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            fired: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Schedule a new timer.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn schedule(&self, timer: DurableTimer) -> PersistenceResult<TimerId> {
        let timer_id = timer.id().clone();
        let execute_at = timer.execute_at();

        // Persist if enabled
        if let Some(persistence) = &self.persistence {
            persistence.save(&timer).await?;
        }

        // Add to in-memory structures
        {
            let mut timers = self.timers.write().await;
            timers.insert(timer_id.as_str().to_string(), timer);
        }

        {
            let mut queue = self.queue.write().await;
            queue.push(Reverse((execute_at, timer_id.clone())));
        }

        Ok(timer_id)
    }

    /// Reschedule an existing timer after a delay.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn reschedule(
        &self,
        mut timer: DurableTimer,
        delay_secs: u64,
    ) -> PersistenceResult<()> {
        timer.reschedule_after(delay_secs);
        let timer_id = timer.id().clone();
        let execute_at = timer.execute_at();

        if let Some(persistence) = &self.persistence {
            persistence.reschedule(&timer).await?;
        }

        {
            let mut timers = self.timers.write().await;
            timers.insert(timer_id.as_str().to_string(), timer);
        }

        {
            let mut queue = self.queue.write().await;
            queue.push(Reverse((execute_at, timer_id)));
        }

        Ok(())
    }

    /// Cancel a timer.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn cancel(&self, timer_id: &TimerId) -> PersistenceResult<bool> {
        let mut timers = self.timers.write().await;

        if let Some(timer) = timers.get_mut(timer_id.as_str()) {
            if timer.status().is_terminal() {
                return Ok(false);
            }

            timer.mark_cancelled();

            // Persist if enabled
            if let Some(persistence) = &self.persistence {
                persistence
                    .update_status(timer_id, TimerStatus::Cancelled)
                    .await?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get a timer by ID.
    pub async fn get(&self, timer_id: &TimerId) -> Option<DurableTimer> {
        let timers = self.timers.read().await;
        timers.get(timer_id.as_str()).cloned()
    }

    /// Poll for due timers.
    ///
    /// Returns timers that are ready to fire.
    pub async fn poll_due(&self) -> Vec<DurableTimer> {
        self.poll_due_with_limit(usize::MAX).await
    }

    /// Poll for due timers with a limit.
    pub async fn poll_due_with_limit(&self, limit: usize) -> Vec<DurableTimer> {
        if limit == 0 {
            return Vec::new();
        }

        let now = Utc::now();
        let mut due_timers = Vec::new();

        {
            let mut queue = self.queue.write().await;
            let mut timers = self.timers.write().await;

            while let Some(Reverse((execute_at, timer_id))) = queue.pop() {
                if due_timers.len() >= limit || execute_at > now {
                    queue.push(Reverse((execute_at, timer_id)));
                    break;
                }

                if let Some(timer) = timers.get_mut(timer_id.as_str()) {
                    if timer.status().is_pending()
                        && timer.execute_at() <= now
                        && timer.execute_at() == execute_at
                    {
                        timer.mark_fired();
                        due_timers.push(timer.clone());
                    }
                }
            }
        }

        // Move to fired list
        {
            let mut fired = self.fired.write().await;
            fired.extend(due_timers.iter().cloned());
        }

        // Persist status changes
        if let Some(persistence) = &self.persistence {
            for timer in &due_timers {
                if let Err(err) = persistence
                    .update_status(timer.id(), TimerStatus::Fired)
                    .await
                {
                    tracing::error!(timer_id = %timer.id(), error = %err, "Failed to persist timer fired status");
                }
            }
        }

        due_timers
    }

    /// Get the next timer that will fire.
    pub async fn peek_next(&self) -> Option<DateTime<Utc>> {
        let queue = self.queue.read().await;
        queue.peek().map(|Reverse((execute_at, _))| *execute_at)
    }

    /// Get the number of pending timers.
    pub async fn pending_count(&self) -> usize {
        let timers = self.timers.read().await;
        timers.values().filter(|t| t.status().is_pending()).count()
    }

    /// Get the number of fired timers.
    pub async fn fired_count(&self) -> usize {
        let fired = self.fired.read().await;
        fired.len()
    }

    /// Acknowledge a fired timer (remove from fired list).
    pub async fn acknowledge(&self, timer_id: &TimerId) {
        let mut fired = self.fired.write().await;
        fired.retain(|t| t.id() != timer_id);
    }

    /// Finalize a timer (remove from fired list and timer map).
    pub async fn finalize(&self, timer_id: &TimerId) {
        self.acknowledge(timer_id).await;
        let mut timers = self.timers.write().await;
        timers.remove(timer_id.as_str());
    }

    /// Mark a timer as failed.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn mark_failed(&self, timer_id: &TimerId) -> PersistenceResult<()> {
        if let Some(persistence) = &self.persistence {
            persistence
                .update_status(timer_id, TimerStatus::Failed)
                .await?;
        }

        let mut timers = self.timers.write().await;
        if let Some(timer) = timers.get_mut(timer_id.as_str()) {
            timer.mark_failed();
        }

        Ok(())
    }

    /// Load pending timers from persistence.
    ///
    /// # Errors
    ///
    /// Returns an error if loading fails.
    pub async fn load_pending(&self) -> PersistenceResult<usize> {
        let Some(persistence) = &self.persistence else {
            return Ok(0);
        };

        let until = Utc::now() + chrono::Duration::seconds(self.config.lookahead_secs as i64);
        let records = persistence.load_pending_until(until).await?;
        let count = records.len();

        let mut timers = self.timers.write().await;
        let mut queue = self.queue.write().await;
        let mut loaded = 0usize;

        for record in records {
            if timers.len() >= self.config.max_in_memory {
                break;
            }

            if timers.contains_key(record.timer_id.as_str()) {
                continue;
            }

            let timer = record.into_timer()?;
            queue.push(Reverse((timer.execute_at(), timer.id().clone())));
            timers.insert(timer.id().as_str().to_string(), timer);
            loaded = loaded.saturating_add(1);
        }

        Ok(loaded.min(count))
    }

    /// Clear all timers (for testing).
    pub async fn clear(&self) {
        let mut timers = self.timers.write().await;
        let mut queue = self.queue.write().await;
        let mut fired = self.fired.write().await;

        timers.clear();
        queue.clear();
        fired.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_id_display() {
        let id = TimerId::from_string("timer-123");
        assert_eq!(format!("{}", id), "timer-123");
    }

    #[test]
    fn test_timer_status_terminal() {
        assert!(!TimerStatus::Pending.is_terminal());
        assert!(TimerStatus::Fired.is_terminal());
        assert!(TimerStatus::Cancelled.is_terminal());
        assert!(TimerStatus::Failed.is_terminal());
    }

    #[test]
    fn test_durable_timer_with_delay() {
        let timer = DurableTimer::with_delay(60, serde_json::json!({}));
        let time_until = timer.time_until().num_seconds();
        assert!(time_until > 55 && time_until <= 60);
    }

    #[test]
    fn test_durable_timer_is_due() {
        let past = Utc::now() - chrono::Duration::seconds(10);
        let timer = DurableTimer::new(past, serde_json::json!({}));
        assert!(timer.is_due());

        let future = Utc::now() + chrono::Duration::seconds(10);
        let timer = DurableTimer::new(future, serde_json::json!({}));
        assert!(!timer.is_due());
    }

    #[test]
    fn test_durable_timer_mark_status() {
        let mut timer = DurableTimer::with_delay(60, serde_json::json!({}));
        assert!(timer.status().is_pending());

        timer.mark_fired();
        assert!(timer.status().is_fired());

        let mut timer2 = DurableTimer::with_delay(60, serde_json::json!({}));
        timer2.mark_cancelled();
        assert!(timer2.status().is_cancelled());
    }

    #[test]
    fn test_durable_timer_with_associations() {
        let timer = DurableTimer::with_delay(60, serde_json::json!({}))
            .with_workflow("wf-1")
            .with_bead("bead-1")
            .with_callback("cb-1");

        assert_eq!(timer.workflow_id(), Some("wf-1"));
        assert_eq!(timer.bead_id(), Some("bead-1"));
        assert_eq!(timer.callback_id(), Some("cb-1"));
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = TimerSchedulerConfig::default();
        assert_eq!(config.max_in_memory, 10_000);
        assert_eq!(config.lookahead_secs, 300);
    }

    #[tokio::test]
    async fn test_scheduler_schedule() {
        let scheduler = TimerScheduler::new(TimerSchedulerConfig::default());

        let timer = DurableTimer::with_delay(60, serde_json::json!({"task": "test"}));
        let timer_id = scheduler.schedule(timer).await;

        assert!(timer_id.is_ok());
        assert_eq!(scheduler.pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_scheduler_cancel() {
        let scheduler = TimerScheduler::new(TimerSchedulerConfig::default());

        let timer = DurableTimer::with_delay(60, serde_json::json!({}));
        let timer_id = scheduler
            .schedule(timer)
            .await
            .unwrap_or_else(|_| TimerId::new());

        let cancelled = scheduler.cancel(&timer_id).await;
        assert!(cancelled.is_ok());
        assert!(cancelled.unwrap_or(false));

        let timer = scheduler.get(&timer_id).await;
        assert!(timer.is_some());
        assert!(timer.map(|t| t.status().is_cancelled()).unwrap_or(false));
    }

    #[tokio::test]
    async fn test_scheduler_poll_due() {
        let scheduler = TimerScheduler::new(TimerSchedulerConfig::default());

        // Schedule a timer that's already due
        let past = Utc::now() - chrono::Duration::seconds(1);
        let timer = DurableTimer::new(past, serde_json::json!({"ready": true}));
        let _ = scheduler.schedule(timer).await;

        // Schedule a timer in the future
        let future = Utc::now() + chrono::Duration::seconds(60);
        let timer = DurableTimer::new(future, serde_json::json!({"not_ready": true}));
        let _ = scheduler.schedule(timer).await;

        let due = scheduler.poll_due().await;
        assert_eq!(due.len(), 1);
        assert!(due[0].payload().get("ready").is_some());
    }

    #[tokio::test]
    async fn test_scheduler_peek_next() {
        let scheduler = TimerScheduler::new(TimerSchedulerConfig::default());

        let future = Utc::now() + chrono::Duration::seconds(60);
        let timer = DurableTimer::new(future, serde_json::json!({}));
        let _ = scheduler.schedule(timer).await;

        let next = scheduler.peek_next().await;
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_scheduler_acknowledge() {
        let scheduler = TimerScheduler::new(TimerSchedulerConfig::default());

        let past = Utc::now() - chrono::Duration::seconds(1);
        let timer = DurableTimer::new(past, serde_json::json!({}));
        let timer_id = scheduler
            .schedule(timer)
            .await
            .unwrap_or_else(|_| TimerId::new());

        let _ = scheduler.poll_due().await;
        assert_eq!(scheduler.fired_count().await, 1);

        scheduler.acknowledge(&timer_id).await;
        assert_eq!(scheduler.fired_count().await, 0);
    }

    #[tokio::test]
    async fn test_scheduler_clear() {
        let scheduler = TimerScheduler::new(TimerSchedulerConfig::default());

        let _ = scheduler
            .schedule(DurableTimer::with_delay(60, serde_json::json!({})))
            .await;
        let _ = scheduler
            .schedule(DurableTimer::with_delay(120, serde_json::json!({})))
            .await;

        scheduler.clear().await;
        assert_eq!(scheduler.pending_count().await, 0);
    }
}
