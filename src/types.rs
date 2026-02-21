//! Domain types, entities, and policies for Oya pipeline orchestration.
//!
//! Submodule layout:
//! - `ids`       — AgentId, RunId, BeadId
//! - `pipeline`  — stages, gates, model tiers, results, failure categories, transitions
//! - `domain`    — errors, agent state, run aggregate, gate results, artifacts
//! - `health`    — circuit breaker, health metrics, behavioral fingerprint, usage types
//! - `timeline`  — timeline events, stage outcomes, ANSI/text utilities

mod domain;
mod health;
mod ids;
mod pipeline;
mod timeline;

// Re-export everything so all existing `use oya::types::*` paths continue to work.

// --- ids ---
pub use ids::{AgentId, BeadId, RunId};

// --- pipeline ---
pub use pipeline::{
    determine_transition, load_model_tier_config, normalize_attempt, passed_stage_transition,
    ApproverMode, FailureCategory, Gate, ModelTier, ModelTierConfig, ShipDecision, StageAttempt,
    StageFailure, StageName, StageResult, StageState, StageTransition, TransitionDecision,
    TransitionReason,
};

// --- domain ---
pub use domain::{
    AgentState, AgentStatus, Artifact, ArtifactType, DomainError, EventSchemaVersion,
    ExecutionEvent, FailureDiagnostics, GateResult, Run, RunState, ValidationError,
};

// --- health ---
pub use health::{
    AgentHealthStatus, BehavioralContext, BehavioralFingerprint, CircuitBreaker, CircuitConfig,
    CircuitState, HealthMetrics, ModelHealth, UsageStatus,
};

// --- timeline ---
pub use timeline::{
    sanitize_url_for_logging, strip_ansi_codes, truncate_clean, DurationMs, GateSummary,
    StageOutcome, TimelineEntry, WorkspaceName,
};
