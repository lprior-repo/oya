//! Health, circuit-breaker, and behavioral-fingerprint types.

use super::domain::DomainError;
use super::ids::{ModelId, Tier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CircuitState {
    #[default]
    Closed,
    Open,
    HalfOpen,
}

impl CircuitState {
    #[must_use]
    pub const fn allows_operations(&self) -> bool {
        matches!(self, Self::Closed | Self::HalfOpen)
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::HalfOpen => "half_open",
        }
    }
}

impl TryFrom<&str> for CircuitState {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, DomainError> {
        match value {
            "closed" => Ok(Self::Closed),
            "open" => Ok(Self::Open),
            "half_open" | "half-open" => Ok(Self::HalfOpen),
            _ => Err(DomainError::ParseError(format!("Unknown circuit state: {value}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub reset_timeout_ms: u64,
}

impl CircuitConfig {
    #[must_use]
    pub const fn new(
        failure_threshold: u32,
        success_threshold: u32,
        reset_timeout_ms: u64,
    ) -> Self {
        Self { failure_threshold, success_threshold, reset_timeout_ms }
    }
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self::new(5, 3, 60_000)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub scope: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub opened_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub config: CircuitConfig,
}

impl CircuitBreaker {
    pub fn new(scope: impl Into<String>, config: CircuitConfig) -> Self {
        Self {
            scope: scope.into(),
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            opened_at: None,
            updated_at: Utc::now(),
            config,
        }
    }

    #[must_use]
    pub const fn should_open(&self) -> bool {
        matches!(self.state, CircuitState::Closed)
            && self.failure_count >= self.config.failure_threshold
    }

    #[must_use]
    pub const fn should_close(&self) -> bool {
        matches!(self.state, CircuitState::HalfOpen)
            && self.success_count >= self.config.success_threshold
    }

    pub fn record_failure(mut self) -> Self {
        self.failure_count += 1;
        self.success_count = 0;
        self.updated_at = Utc::now();
        if matches!(self.state, CircuitState::HalfOpen) || self.should_open() {
            self.state = CircuitState::Open;
            self.opened_at = Some(Utc::now());
        }
        self
    }

    pub fn record_success(mut self) -> Self {
        self.failure_count = 0;
        self.success_count += 1;
        self.updated_at = Utc::now();
        if self.should_close() {
            self.state = CircuitState::Closed;
            self.opened_at = None;
        }
        self
    }

    pub fn try_half_open(mut self) -> Self {
        if self.state == CircuitState::Open {
            if let Some(opened_at) = self.opened_at {
                let elapsed = (Utc::now() - opened_at).num_milliseconds() as u64;
                if elapsed >= self.config.reset_timeout_ms {
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                    self.updated_at = Utc::now();
                }
            }
        }
        self
    }
}

// ---------------------------------------------------------------------------
// Health metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct HealthMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub in_progress: u64,
}

impl HealthMetrics {
    #[must_use]
    pub const fn new(total: u64, success: u64, failed: u64, in_progress: u64) -> Self {
        Self {
            total_operations: total,
            successful_operations: success,
            failed_operations: failed,
            in_progress,
        }
    }

    #[must_use]
    pub fn success_rate(&self) -> u8 {
        if self.total_operations == 0 {
            return 100;
        }
        let rate = (self.successful_operations as f64 / self.total_operations as f64) * 100.0;
        rate.clamp(0.0, 100.0) as u8
    }

    #[must_use]
    pub const fn record_success(&self) -> Self {
        Self::new(
            self.total_operations + 1,
            self.successful_operations + 1,
            self.failed_operations,
            self.in_progress.saturating_sub(1),
        )
    }

    #[must_use]
    pub const fn record_failure(&self) -> Self {
        Self::new(
            self.total_operations + 1,
            self.successful_operations,
            self.failed_operations + 1,
            self.in_progress.saturating_sub(1),
        )
    }

    #[must_use]
    pub const fn start_operation(&self) -> Self {
        Self::new(
            self.total_operations,
            self.successful_operations,
            self.failed_operations,
            self.in_progress + 1,
        )
    }

    #[must_use]
    pub fn is_critical(&self, threshold: u8) -> bool {
        self.total_operations >= 10 && self.success_rate() < threshold
    }
}

// ---------------------------------------------------------------------------
// Agent health + behavioral fingerprint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentHealthStatus {
    Healthy,
    Degraded,
    Stuck,
    RetryLoop,
}

impl AgentHealthStatus {
    #[must_use]
    pub const fn needs_intervention(&self) -> bool {
        matches!(self, Self::Stuck | Self::RetryLoop)
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Stuck => "stuck",
            Self::RetryLoop => "retry_loop",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralFingerprint {
    pub agent_id: String,
    pub current_bead_id: Option<String>,
    pub current_stage: String,
    pub consecutive_failures: u32,
    pub secs_since_progress: u64,
    pub retry_count: u32,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralContext {
    pub current_bead_id: Option<String>,
    pub current_stage: String,
}

impl BehavioralContext {
    #[must_use]
    pub fn new(current_bead_id: Option<String>, current_stage: impl Into<String>) -> Self {
        Self { current_bead_id, current_stage: current_stage.into() }
    }
}

impl BehavioralFingerprint {
    pub fn new(
        agent_id: impl Into<String>,
        context: BehavioralContext,
        consecutive_failures: u32,
        secs_since_progress: u64,
        retry_count: u32,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            current_bead_id: context.current_bead_id,
            current_stage: context.current_stage,
            consecutive_failures,
            secs_since_progress,
            retry_count,
            computed_at: Utc::now(),
        }
    }

    #[must_use]
    pub const fn is_stuck(&self, max_idle_secs: u64, max_failures: u32) -> bool {
        self.secs_since_progress > max_idle_secs || self.consecutive_failures > max_failures
    }

    #[must_use]
    pub const fn is_retry_loop(&self, max_retries: u32) -> bool {
        self.retry_count > max_retries
    }

    #[must_use]
    pub const fn health_status(&self) -> AgentHealthStatus {
        if self.is_stuck(300, 5) {
            AgentHealthStatus::Stuck
        } else if self.is_retry_loop(10) {
            AgentHealthStatus::RetryLoop
        } else if self.consecutive_failures > 0 {
            AgentHealthStatus::Degraded
        } else {
            AgentHealthStatus::Healthy
        }
    }
}

// ---------------------------------------------------------------------------
// Usage tracking types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelHealth {
    pub model_id: ModelId,
    pub is_rate_limited: bool,
    pub consecutive_failures: u32,
    pub cooldown_until: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageStatus {
    pub active_models: std::collections::HashMap<Tier, ModelId>,
    pub model_health: std::collections::HashMap<ModelId, ModelHealth>,
    pub circuit_state: CircuitState,
    pub last_updated: DateTime<Utc>,
}
