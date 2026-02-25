//! Data types for tail TUI - domain model with illegal states unrepresentable.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde::Deserialize;

/// Newtype for run_id - prevents primitive obsession at domain boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunId(String);

impl RunId {
    /// Smart constructor that always succeeds - provides safe fallback.
    /// Use this when you need a RunId and can handle a placeholder value.
    pub fn new_or_fallback(s: String) -> Self {
        if s.is_empty() {
            Self("<unknown>".to_string())
        } else {
            Self(s)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Newtype for stage name - prevents stringly-typed stage values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageName(String);

impl StageName {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Max attempts per stage - domain logic encapsulated.
    pub fn max_attempts(&self) -> u32 {
        // Based on StageName enum from orchestrator
        2
    }
}

/// Explicit state machine - makes illegal states unrepresentable.
/// The combination of status + result is now enforced by the type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationState {
    Running,
    CompletedSuccess,
    CompletedFailure,
    Suspended,
    Cancelled,
    Unknown,
}

impl InvocationState {
    /// Parse from untrusted Restate status string (boundary).
    pub fn from_restate_status(status: &str, result: Option<&str>) -> Self {
        match status {
            "running" => Self::Running,
            "completed" => match result {
                Some("success") => Self::CompletedSuccess,
                Some("failure") => Self::CompletedFailure,
                _ => Self::CompletedSuccess, // Default to success for backward compatibility
            },
            "suspended" => Self::Suspended,
            "cancelled" => Self::Cancelled,
            _ => Self::Unknown,
        }
    }

    /// Whether this state represents a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::CompletedSuccess | Self::CompletedFailure | Self::Cancelled)
    }

    /// Whether this state is currently running.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

/// Attempt number with type-safe invariant: 1 <= attempt <= max_attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt(u32);

impl Attempt {
    pub fn new(value: u32, max: u32) -> Option<Self> {
        if value > 0 && value <= max {
            Some(Self(value))
        } else {
            None
        }
    }

    pub fn first() -> Self {
        Self(1)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

/// Enriched invocation data for display - illegal states unrepresentable.
#[derive(Debug, Clone)]
pub struct InvocationView {
    pub run_id: RunId,
    pub state: InvocationState,
    pub stage: Option<StageName>,
    pub attempt: Option<Attempt>,
    pub gates: Vec<GateView>,
    pub last_output_lines: Vec<String>,
    pub age_seconds: u64,
}

/// Newtype for gate name - prevents primitive obsession.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateName(String);

impl GateName {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Explicit gate state - no bool flags, no option confusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    Passed,
    Failed,
    Running,
}

impl GateState {
    /// Icon for UI display - single source of truth.
    pub fn icon(&self) -> char {
        match self {
            Self::Passed => '\u{2705}',  // ✅
            Self::Failed => '\u{274c}',  // ❌
            Self::Running => '\u{231b}', // ⏳
        }
    }
}

/// Gate view with semantic types, not primitives.
#[derive(Debug, Clone)]
pub struct GateView {
    pub name: GateName,
    pub state: GateState,
}

/// Raw Restate sys_invocation row from SQL query - boundary type only.
#[derive(Debug, Clone, Deserialize)]
pub struct RestateInvocationRow {
    #[serde(rename = "target_service_key")]
    pub target_service_key: String,
    pub status: String,
    pub completion_result: Option<String>,
    pub completion_failure: Option<String>,
    pub modified_at: String,
}

/// Response from Restate SQL query endpoint - boundary type only.
#[derive(Debug, Clone, Deserialize)]
pub struct RestateQueryResponse {
    pub rows: Vec<RestateInvocationRow>,
}
