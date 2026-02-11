// Data models for swarm coordination
// Zero panic, zero unwrap, purely functional Rust

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================
// Core Types
// ============================================================

/// Agent identifier (1-12)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(u8);

impl AgentId {
    pub fn new(id: u8) -> Option<Self> {
        if (1..=12).contains(&id) {
            Some(Self(id))
        } else {
            None
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Agent-{}", self.0)
    }
}

/// Bead identifier from .beads/
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BeadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pipeline stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    RustContract,
    Implement,
    QaEnforcer,
    RedQueen,
    Done,
}

impl fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RustContract => write!(f, "rust-contract"),
            Self::Implement => write!(f, "implement"),
            Self::QaEnforcer => write!(f, "qa-enforcer"),
            Self::RedQueen => write!(f, "red-queen"),
            Self::Done => write!(f, "done"),
        }
    }
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    Waiting,
    Error,
    Done,
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Working => write!(f, "working"),
            Self::Waiting => write!(f, "waiting"),
            Self::Error => write!(f, "error"),
            Self::Done => write!(f, "done"),
        }
    }
}

/// Stage execution result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageResult {
    Started,
    Passed,
    Failed(String),
    Error(String),
}

impl fmt::Display for StageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Started => write!(f, "started"),
            Self::Passed => write!(f, "passed"),
            Self::Failed(msg) => write!(f, "failed: {}", msg),
            Self::Error(msg) => write!(f, "error: {}", msg),
        }
    }
}

/// Stage execution history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageHistoryEntry {
    pub agent_id: AgentId,
    pub bead_id: BeadId,
    pub stage: PipelineStage,
    pub attempt_number: u32,
    pub result: StageResult,
    pub feedback: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
}

/// Current agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: AgentId,
    pub bead_id: Option<BeadId>,
    pub current_stage: Option<PipelineStage>,
    pub stage_started_at: Option<DateTime<Utc>>,
    pub status: AgentStatus,
    pub last_update: DateTime<Utc>,
    pub implementation_attempt: u32,
    pub feedback: Option<String>,
}

/// Bead claim record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadClaim {
    pub bead_id: BeadId,
    pub claimed_by: AgentId,
    pub claimed_at: DateTime<Utc>,
    pub status: BeadClaimStatus,
}

/// Bead claim status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeadClaimStatus {
    InProgress,
    Completed,
    Blocked,
}

impl fmt::Display for BeadClaimStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

/// Swarm configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub max_agents: u8,
    pub max_implementation_attempts: u32,
    pub claim_label: String,
    pub swarm_started_at: DateTime<Utc>,
    pub swarm_status: SwarmStatus,
}

/// Swarm status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmStatus {
    Initializing,
    Running,
    Paused,
    Complete,
    Error,
}

impl fmt::Display for SwarmStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initializing => write!(f, "initializing"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Complete => write!(f, "complete"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Swarm progress summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmProgress {
    pub completed: u64,
    pub working: u64,
    pub waiting: u64,
    pub errors: u64,
    pub idle: u64,
    pub total_agents: u64,
}

/// Active agent summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAgentSummary {
    pub agent_id: AgentId,
    pub bead_id: BeadId,
    pub current_stage: PipelineStage,
    pub status: AgentStatus,
    pub implementation_attempt: u32,
    pub claimed_at: DateTime<Utc>,
    pub time_elapsed_ms: u64,
}

/// Feedback requiring attention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRequired {
    pub agent_id: AgentId,
    pub bead_id: BeadId,
    pub stage: PipelineStage,
    pub attempt_number: u32,
    pub feedback: String,
    pub completed_at: DateTime<Utc>,
}

// ============================================================
// Tree Keys (Sled)
// ============================================================

/// Tree name prefixes
pub mod trees {
    pub const AGENT_STATE: &str = "agent_state";
    pub const BEAD_CLAIMS: &str = "bead_claims";
    pub const STAGE_HISTORY: &str = "stage_history";
    pub const CONFIG: &str = "config";
}

/// Config keys
pub mod config_keys {
    pub const MAX_AGENTS: &str = "max_agents";
    pub const MAX_IMPLEMENTATION_ATTEMPTS: &str = "max_implementation_attempts";
    pub const CLAIM_LABEL: &str = "claim_label";
    pub const SWARM_STARTED_AT: &str = "swarm_started_at";
    pub const SWARM_STATUS: &str = "swarm_status";
}
