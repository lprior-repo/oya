// =============================================================================
//  Value Objects - IDs (Wlaschin: Types as constraints)
// =============================================================================
//! Core identity types for Oya pipeline orchestration.
//! See docs/UBIQUITOUS_LANGUAGE.md for domain terminology.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RunId(pub String);

impl RunId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BeadId(pub String);

impl BeadId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StageName {
    Contract,
    DesignDag,
    Implement,
    Tdd15,
    Qa,
    RedQueen,
    GptReview,
    ShipGate,
}

impl StageName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::DesignDag => "design_dag",
            Self::Implement => "implement",
            Self::Tdd15 => "tdd15",
            Self::Qa => "qa",
            Self::RedQueen => "red_queen",
            Self::GptReview => "gpt_review",
            Self::ShipGate => "ship_gate",
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Contract => Some(Self::DesignDag),
            Self::DesignDag => Some(Self::Implement),
            Self::Implement => Some(Self::Tdd15),
            Self::Tdd15 => Some(Self::Qa),
            Self::Qa => Some(Self::RedQueen),
            Self::RedQueen => Some(Self::GptReview),
            Self::GptReview => Some(Self::ShipGate),
            Self::ShipGate => None,
        }
    }

    pub fn model_for_stage(&self) -> ModelTier {
        match self {
            Self::Contract => ModelTier::Fast,
            Self::DesignDag => ModelTier::Balanced,
            Self::Implement => ModelTier::Balanced,
            Self::Tdd15 => ModelTier::Balanced,
            Self::Qa => ModelTier::Balanced,
            Self::RedQueen => ModelTier::Capable,
            Self::GptReview => ModelTier::Capable,
            Self::ShipGate => ModelTier::Best,
        }
    }

    pub fn max_attempts(&self) -> u32 {
        3
    }

    pub fn gates(&self) -> Vec<Gate> {
        match self {
            Self::Contract => vec![Gate::Compiles],
            Self::DesignDag => vec![Gate::Compiles],
            Self::Implement => vec![Gate::Compiles],
            Self::Tdd15 => vec![Gate::Compiles, Gate::TestsPass],
            Self::Qa => vec![Gate::TestsPass, Gate::EdgeCases],
            Self::RedQueen => vec![Gate::NoVulnerabilities],
            Self::GptReview => vec![Gate::ClippyClean, Gate::Security],
            Self::ShipGate => vec![Gate::MoonCi, Gate::ZjjMergeQueue],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelTier {
    Fast,
    Balanced,
    Capable,
    Best,
}

impl ModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Capable => "capable",
            Self::Best => "best",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gate {
    Compiles,
    TestsPass,
    EdgeCases,
    NoVulnerabilities,
    ClippyClean,
    Security,
    MoonCi,
    ZjjMergeQueue,
}

impl Gate {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compiles => "compiles",
            Self::TestsPass => "tests_pass",
            Self::EdgeCases => "edge_cases",
            Self::NoVulnerabilities => "no_vulnerabilities",
            Self::ClippyClean => "clippy_clean",
            Self::Security => "security",
            Self::MoonCi => "moon_ci",
            Self::ZjjMergeQueue => "zjj_merge_queue",
        }
    }
}

impl TryFrom<&str> for StageName {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "contract" => Ok(Self::Contract),
            "design_dag" => Ok(Self::DesignDag),
            "implement" => Ok(Self::Implement),
            "tdd15" => Ok(Self::Tdd15),
            "qa" => Ok(Self::Qa),
            "red_queen" => Ok(Self::RedQueen),
            "gpt_review" => Ok(Self::GptReview),
            "ship_gate" => Ok(Self::ShipGate),
            _ => Err(format!("Unknown stage: {s}")),
        }
    }
}

impl TryFrom<&str> for ModelTier {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "capable" => Ok(Self::Capable),
            "best" => Ok(Self::Best),
            _ => Err(format!("Unknown model tier: {s}")),
        }
    }
}

impl TryFrom<&str> for Gate {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "compiles" => Ok(Self::Compiles),
            "tests_pass" => Ok(Self::TestsPass),
            "edge_cases" => Ok(Self::EdgeCases),
            "no_vulnerabilities" => Ok(Self::NoVulnerabilities),
            "clippy_clean" => Ok(Self::ClippyClean),
            "security" => Ok(Self::Security),
            "moon_ci" => Ok(Self::MoonCi),
            "zjj_merge_queue" => Ok(Self::ZjjMergeQueue),
            _ => Err(format!("Unknown gate: {s}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    Waiting,
    Error,
    Done,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Waiting => "waiting",
            Self::Error => "error",
            Self::Done => "done",
        }
    }
}

impl TryFrom<&str> for AgentStatus {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, <Self as TryFrom<&str>>::Error> {
        match s {
            "idle" => Ok(Self::Idle),
            "working" => Ok(Self::Working),
            "waiting" => Ok(Self::Waiting),
            "error" => Ok(Self::Error),
            "done" => Ok(Self::Done),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: AgentId,
    pub bead_id: Option<BeadId>,
    pub current_stage: Option<StageName>,
    pub stage_started_at: Option<DateTime<Utc>>,
    pub status: AgentStatus,
    pub last_update: DateTime<Utc>,
    pub implementation_attempt: u32,
    pub feedback: Option<String>,
}

impl AgentState {
    pub fn new(
        agent_id: AgentId,
        bead_id: Option<BeadId>,
        current_stage: Option<StageName>,
        status: AgentStatus,
        implementation_attempt: u32,
    ) -> Self {
        Self {
            agent_id,
            bead_id,
            current_stage,
            stage_started_at: None,
            status,
            last_update: Utc::now(),
            implementation_attempt,
            feedback: None,
        }
    }

    pub fn validate_invariants(&self) -> Result<(), String> {
        match self.status {
            AgentStatus::Working => {
                if self.bead_id.is_none() {
                    return Err("Agent with Working status must have a bead".to_string());
                }
                if self.current_stage.is_none() {
                    return Err("Agent with Working status must have a current_stage".to_string());
                }
            }
            AgentStatus::Done => {
                if self.bead_id.is_some() {
                    return Err("Agent with Done status must not have a bead".to_string());
                }
                // Done status usually implies we are waiting for new work, so no active stage?
                // Or maybe the *last* stage was done.
                // The user snippet said:
                // if self.current_stage.is_some() && self.current_stage != Some(Stage::Done)
                // But StageName doesn't have a "Done" variant in my enum.
                // Let's assume Done status means no active stage for now, or just skip that check if ambiguous.
                // User snippet: "current_stage = Done or None".
                // I'll enforce None for now as I don't have Stage::Done.
                if self.current_stage.is_some() {
                    return Err("Agent with Done status must have no active stage".to_string());
                }
            }
            AgentStatus::Idle | AgentStatus::Waiting | AgentStatus::Error => {
                if self.bead_id.is_some() {
                    return Err(format!(
                        "Agent with {:?} status must not have a bead",
                        self.status
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid state transition: {0} -> {1}")]
    InvalidStateTransition(String, String),
    #[error("Missing required artifact: {0}")]
    MissingArtifact(String),
    #[error("Gate check failed: {0}")]
    GateCheckFailed(String),
    #[error("Stale data: expected version {0}, got {1}")]
    StaleData(u64, u64),
}

// =============================================================================
//  Run Aggregate
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RunState {
    Pending,
    Running {
        current_stage: StageName,
    },
    Waiting {
        reason: String,
    },
    Shipped {
        completed_at: DateTime<Utc>,
    },
    Failed {
        reason: String,
        failed_at: DateTime<Utc>,
    },
    Aborted {
        reason: String,
        aborted_at: DateTime<Utc>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    pub bead_id: BeadId,
    pub state: RunState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // History of stage executions
    pub history: Vec<StageAttempt>,
}

impl Run {
    pub fn new(bead_id: BeadId) -> Self {
        let now = Utc::now();
        Self {
            id: RunId::new(),
            bead_id,
            state: RunState::Pending,
            created_at: now,
            updated_at: now,
            history: Vec::new(),
        }
    }

    pub fn start(&self) -> Result<Self, DomainError> {
        match &self.state {
            RunState::Pending => {
                let mut next = self.clone();
                next.state = RunState::Running {
                    current_stage: StageName::Contract,
                }; // Initial stage
                next.updated_at = Utc::now();
                Ok(next)
            }
            s => Err(DomainError::InvalidStateTransition(
                format!("{:?}", s),
                "Running".to_string(),
            )),
        }
    }

    pub fn complete_stage(
        &self,
        stage: StageName,
        _result: StageResult,
    ) -> Result<Self, DomainError> {
        match &self.state {
            RunState::Running { current_stage } if *current_stage == stage => {
                let mut next = self.clone();
                // Transition logic: determine next stage or finish
                let next_stage = match stage {
                    StageName::Contract => Some(StageName::DesignDag),
                    StageName::DesignDag => Some(StageName::Implement),
                    StageName::Implement => Some(StageName::Tdd15),
                    StageName::Tdd15 => Some(StageName::Qa),
                    StageName::Qa => Some(StageName::RedQueen),
                    StageName::RedQueen => Some(StageName::GptReview),
                    StageName::GptReview => Some(StageName::ShipGate),
                    StageName::ShipGate => None, // Done
                };

                if let Some(ns) = next_stage {
                    next.state = RunState::Running { current_stage: ns };
                } else {
                    next.state = RunState::Shipped {
                        completed_at: Utc::now(),
                    };
                }
                next.updated_at = Utc::now();
                Ok(next)
            }
            s => Err(DomainError::InvalidStateTransition(
                format!("{:?}", s),
                "NextStage".to_string(),
            )),
        }
    }

    pub fn fail(&self, reason: String) -> Self {
        let mut next = self.clone();
        next.state = RunState::Failed {
            reason,
            failed_at: Utc::now(),
        };
        next.updated_at = Utc::now();
        next
    }
}

// =============================================================================
//  Supporting Types
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    Pending,
    Running,
    Passed,
    Failed,
    WaitingPermission,
    WaitingQuestion,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageAttempt {
    pub run_id: String,
    pub stage: StageName,
    pub attempt: u32,
    pub session_id: Option<String>,
    pub state: StageState,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    TestFailed,
    TestInfraFailed,
    CompileFailed,
    LintFailed,
    MergeConflict,
    RateLimited,
    AuthFailed,
    ContextOverflow,
    ProviderUnavailable,
    OutputParseFailure,
    MaxAttemptsExceeded,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageResult {
    pub run_id: String,
    pub stage: StageName,
    pub attempt: u32,
    pub passed: bool,
    pub output: serde_json::Value,
    pub failure_category: Option<FailureCategory>,
    pub next_stage: Option<StageName>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipDecision {
    pub run_id: String,
    pub shipped: bool,
    pub rationale: String,
    pub approver_mode: ApproverMode,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApproverMode {
    Auto,
    Human,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateResult {
    pub run_id: String,
    pub gate_name: String,
    pub passed: bool,
    pub exit_code: i32,
    pub log_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactType {
    ContractDocument,
    Requirements,
    SystemContext,
    Invariants,
    DataFlow,
    ImplementationPlan,
    AcceptanceCriteria,
    ErrorHandling,
    TestScenarios,
    ValidationGates,
    SuccessMetrics,
    ImplementationCode,
    ModifiedFiles,
    ImplementationNotes,
    TestOutput,
    TestResults,
    CoverageReport,
    ValidationReport,
    FailureDetails,
    AdversarialReport,
    RegressionReport,
    QualityGateReport,
    StageLog,
    RetryPacket,
    SkillInvocation,
    ErrorMessage,
    Feedback,
}

impl ArtifactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ContractDocument => "contract_document",
            Self::Requirements => "requirements",
            Self::SystemContext => "system_context",
            Self::Invariants => "invariants",
            Self::DataFlow => "data_flow",
            Self::ImplementationPlan => "implementation_plan",
            Self::AcceptanceCriteria => "acceptance_criteria",
            Self::ErrorHandling => "error_handling",
            Self::TestScenarios => "test_scenarios",
            Self::ValidationGates => "validation_gates",
            Self::SuccessMetrics => "success_metrics",
            Self::ImplementationCode => "implementation_code",
            Self::ModifiedFiles => "modified_files",
            Self::ImplementationNotes => "implementation_notes",
            Self::TestOutput => "test_output",
            Self::TestResults => "test_results",
            Self::CoverageReport => "coverage_report",
            Self::ValidationReport => "validation_report",
            Self::FailureDetails => "failure_details",
            Self::AdversarialReport => "adversarial_report",
            Self::RegressionReport => "regression_report",
            Self::QualityGateReport => "quality_gate_report",
            Self::StageLog => "stage_log",
            Self::RetryPacket => "retry_packet",
            Self::SkillInvocation => "skill_invocation",
            Self::ErrorMessage => "error_message",
            Self::Feedback => "feedback",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub run_id: String,
    pub artifact_type: ArtifactType,
    pub location: String,
    pub checksum: Option<String>,
    pub produced_by_stage: StageName,
}

// =============================================================================
//  Circuit Breaker (North:fault tolerance, Wlaschin:types prevent misuse)
// =============================================================================

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
    type Error = String;

    fn try_from(value: &str) -> Result<Self, String> {
        match value {
            "closed" => Ok(Self::Closed),
            "open" => Ok(Self::Open),
            "half_open" | "half-open" => Ok(Self::HalfOpen),
            _ => Err(format!("Unknown circuit state: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub reset_timeout_secs: u64,
}

impl CircuitConfig {
    #[must_use]
    pub const fn new(
        failure_threshold: u32,
        success_threshold: u32,
        reset_timeout_secs: u64,
    ) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            reset_timeout_secs,
        }
    }
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self::new(5, 3, 60)
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

        if self.should_open() {
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
                let elapsed = (Utc::now() - opened_at).num_seconds() as u64;
                if elapsed >= self.config.reset_timeout_secs {
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                    self.updated_at = Utc::now();
                }
            }
        }
        self
    }
}

// =============================================================================
//  Health & Behavioral Fingerprint (Wlaschin: types prevent invalid states)
// =============================================================================

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

impl BehavioralFingerprint {
    pub fn new(
        agent_id: impl Into<String>,
        current_bead_id: Option<String>,
        current_stage: impl Into<String>,
        consecutive_failures: u32,
        secs_since_progress: u64,
        retry_count: u32,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            current_bead_id,
            current_stage: current_stage.into(),
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

// =============================================================================
//  Execution Events (Fowler: Domain Events)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSchemaVersion {
    V1,
}

impl EventSchemaVersion {
    #[must_use]
    pub const fn as_i32(&self) -> i32 {
        match self {
            Self::V1 => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDiagnostics {
    pub category: String,
    pub retryable: bool,
    pub next_command: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub seq: i64,
    pub schema_version: i32,
    pub event_type: String,
    pub entity_id: String,
    pub bead_id: Option<String>,
    pub agent_id: Option<String>,
    pub stage: Option<String>,
    pub causation_id: Option<String>,
    pub diagnostics: Option<FailureDiagnostics>,
    pub payload: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
//  Stage Transitions (Fowler: State Machine, North: BDD-style transitions)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageTransition {
    Advance(StageName),
    Retry,
    Block,
    Complete,
    NoOp,
}

impl StageTransition {
    #[must_use]
    pub const fn should_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    #[must_use]
    pub const fn should_retry(&self) -> bool {
        matches!(self, Self::Retry)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransitionReason {
    StagePassedAdvance,
    StagePassedNoNextStage,
    RedQueenPassedComplete,
    StageFailedRetry,
    StageFailedMaxAttemptsReached,
}

impl TransitionReason {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::StagePassedAdvance => "stage_passed_advance",
            Self::StagePassedNoNextStage => "stage_passed_no_next_stage",
            Self::RedQueenPassedComplete => "red_queen_passed_complete",
            Self::StageFailedRetry => "stage_failed_retry",
            Self::StageFailedMaxAttemptsReached => "stage_failed_max_attempts_reached",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionDecision {
    pub transition: StageTransition,
    pub reason: TransitionReason,
}

impl TransitionDecision {
    #[must_use]
    pub const fn new(transition: StageTransition, reason: TransitionReason) -> Self {
        Self { transition, reason }
    }

    #[must_use]
    pub fn transition(&self) -> StageTransition {
        self.transition.clone()
    }

    #[must_use]
    pub const fn reason(&self) -> TransitionReason {
        self.reason
    }
}

#[must_use]
pub fn determine_transition(
    stage: StageName,
    is_success: bool,
    retry_exhausted: bool,
) -> TransitionDecision {
    if is_success {
        return passed_stage_transition(stage);
    }

    if retry_exhausted {
        return TransitionDecision::new(
            StageTransition::Block,
            TransitionReason::StageFailedMaxAttemptsReached,
        );
    }

    TransitionDecision::new(StageTransition::Retry, TransitionReason::StageFailedRetry)
}

#[must_use]
pub fn passed_stage_transition(stage: StageName) -> TransitionDecision {
    match stage {
        StageName::Contract => TransitionDecision::new(
            StageTransition::Advance(StageName::DesignDag),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::DesignDag => TransitionDecision::new(
            StageTransition::Advance(StageName::Implement),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Implement => TransitionDecision::new(
            StageTransition::Advance(StageName::Tdd15),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Tdd15 => TransitionDecision::new(
            StageTransition::Advance(StageName::Qa),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Qa => TransitionDecision::new(
            StageTransition::Advance(StageName::RedQueen),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::RedQueen => TransitionDecision::new(
            StageTransition::Advance(StageName::GptReview),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::GptReview => TransitionDecision::new(
            StageTransition::Advance(StageName::ShipGate),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::ShipGate => TransitionDecision::new(
            StageTransition::NoOp,
            TransitionReason::StagePassedNoNextStage,
        ),
    }
}

// =============================================================================
//  BDD Tests (North: given_when_then)
// =============================================================================

#[cfg(test)]
mod circuit_breaker_tests {
    use super::*;

    #[test]
    fn given_closed_circuit_when_failure_threshold_reached_then_circuit_opens() {
        let config = CircuitConfig::new(3, 2, 60);
        let cb = CircuitBreaker::new("test-scope", config);

        let cb = cb.record_failure().record_failure().record_failure();

        assert_eq!(cb.state, CircuitState::Open);
        assert!(cb.opened_at.is_some());
    }

    #[test]
    fn given_open_circuit_when_reset_timeout_elapsed_then_circuit_half_opens() {
        let config = CircuitConfig::new(2, 2, 0);
        let cb = CircuitBreaker::new("test-scope", config)
            .record_failure()
            .record_failure();

        assert_eq!(cb.state, CircuitState::Open);

        let cb = cb.try_half_open();
        assert_eq!(cb.state, CircuitState::HalfOpen);
    }

    #[test]
    fn given_half_open_circuit_when_success_threshold_reached_then_circuit_closes() {
        let config = CircuitConfig::new(2, 2, 0);
        let mut cb = CircuitBreaker::new("test-scope", config)
            .record_failure()
            .record_failure();

        cb.state = CircuitState::HalfOpen;

        let cb = cb.record_success().record_success();
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn circuit_state_allows_operations_when_closed_or_half_open() {
        assert!(CircuitState::Closed.allows_operations());
        assert!(CircuitState::HalfOpen.allows_operations());
        assert!(!CircuitState::Open.allows_operations());
    }

    #[test]
    fn circuit_state_roundtrip_preserves_values() {
        let cases = [
            (CircuitState::Closed, "closed"),
            (CircuitState::Open, "open"),
            (CircuitState::HalfOpen, "half_open"),
        ];

        for (state, expected) in cases {
            assert_eq!(state.as_str(), expected);
            assert_eq!(CircuitState::try_from(expected), Ok(state));
        }
    }
}

#[cfg(test)]
mod health_metrics_tests {
    use super::*;

    #[test]
    fn given_operations_when_calculating_success_rate_then_returns_percentage() {
        let metrics = HealthMetrics::new(100, 80, 20, 0);
        assert_eq!(metrics.success_rate(), 80);
    }

    #[test]
    fn given_no_operations_when_calculating_success_rate_then_returns_100() {
        let metrics = HealthMetrics::default();
        assert_eq!(metrics.success_rate(), 100);
    }

    #[test]
    fn given_low_success_rate_and_sufficient_operations_when_checking_critical_then_returns_true() {
        let metrics = HealthMetrics::new(100, 30, 70, 0);
        assert!(metrics.is_critical(50));
    }

    #[test]
    fn given_few_operations_when_checking_critical_then_returns_false() {
        let metrics = HealthMetrics::new(5, 1, 4, 0);
        assert!(!metrics.is_critical(50));
    }

    #[test]
    fn given_healthmetrics_when_recording_operations_then_returns_immutable_new_state() {
        let metrics = HealthMetrics::new(10, 8, 2, 1);
        let after_success = metrics.record_success();

        assert_eq!(metrics.total_operations, 10);
        assert_eq!(after_success.total_operations, 11);
    }
}

#[cfg(test)]
mod behavioral_fingerprint_tests {
    use super::*;

    #[test]
    fn given_fingerprint_with_high_idle_time_when_checking_stuck_then_returns_true() {
        let fp = BehavioralFingerprint::new(
            "agent-1",
            Some("bead-123".to_string()),
            "implement",
            0,
            600,
            0,
        );
        assert!(fp.is_stuck(300, 5));
    }

    #[test]
    fn given_fingerprint_with_high_consecutive_failures_when_checking_stuck_then_returns_true() {
        let fp = BehavioralFingerprint::new(
            "agent-1",
            Some("bead-123".to_string()),
            "implement",
            10,
            60,
            0,
        );
        assert!(fp.is_stuck(300, 5));
    }

    #[test]
    fn given_fingerprint_with_high_retry_count_when_checking_retry_loop_then_returns_true() {
        let fp = BehavioralFingerprint::new(
            "agent-1",
            Some("bead-123".to_string()),
            "implement",
            0,
            60,
            15,
        );
        assert!(fp.is_retry_loop(10));
    }

    #[test]
    fn given_healthy_fingerprint_when_checking_health_status_then_returns_healthy() {
        let fp = BehavioralFingerprint::new(
            "agent-1",
            Some("bead-123".to_string()),
            "contract",
            0,
            60,
            0,
        );
        assert_eq!(fp.health_status(), AgentHealthStatus::Healthy);
    }

    #[test]
    fn given_fingerprint_with_failures_when_checking_health_status_then_returns_degraded() {
        let fp = BehavioralFingerprint::new(
            "agent-1",
            Some("bead-123".to_string()),
            "implement",
            3,
            60,
            0,
        );
        assert_eq!(fp.health_status(), AgentHealthStatus::Degraded);
    }

    #[test]
    fn agent_health_status_needs_intervention_for_stuck_and_retry_loop() {
        assert!(!AgentHealthStatus::Healthy.needs_intervention());
        assert!(!AgentHealthStatus::Degraded.needs_intervention());
        assert!(AgentHealthStatus::Stuck.needs_intervention());
        assert!(AgentHealthStatus::RetryLoop.needs_intervention());
    }
}

#[cfg(test)]
mod transition_decision_tests {
    use super::*;

    #[test]
    fn given_contract_stage_when_stage_passes_then_advances_to_design_dag() {
        let decision = determine_transition(StageName::Contract, true, false);

        assert_eq!(
            decision.transition(),
            StageTransition::Advance(StageName::DesignDag)
        );
    }

    #[test]
    fn given_design_dag_stage_when_stage_passes_then_advances_to_implement() {
        let decision = determine_transition(StageName::DesignDag, true, false);

        assert_eq!(
            decision.transition(),
            StageTransition::Advance(StageName::Implement)
        );
    }

    #[test]
    fn given_red_queen_stage_when_stage_passes_then_advances_to_gpt_review() {
        let decision = determine_transition(StageName::RedQueen, true, false);

        assert_eq!(
            decision.transition(),
            StageTransition::Advance(StageName::GptReview)
        );
    }

    #[test]
    fn given_any_stage_when_stage_fails_and_retries_available_then_retry() {
        let decision = determine_transition(StageName::Contract, false, false);

        assert_eq!(decision.transition(), StageTransition::Retry);
    }

    #[test]
    fn given_any_stage_when_stage_fails_and_retries_exhausted_then_block() {
        let decision = determine_transition(StageName::Contract, false, true);

        assert_eq!(decision.transition(), StageTransition::Block);
    }

    #[test]
    fn given_ship_gate_when_stage_passes_then_no_op() {
        let decision = determine_transition(StageName::ShipGate, true, false);

        assert_eq!(decision.transition(), StageTransition::NoOp);
    }
}

#[cfg(test)]
mod pipeline_stage_tests {
    use super::*;

    #[test]
    fn given_stage_when_getting_next_stage_then_returns_correct_next() {
        assert_eq!(StageName::Contract.next(), Some(StageName::DesignDag));
        assert_eq!(StageName::DesignDag.next(), Some(StageName::Implement));
        assert_eq!(StageName::Implement.next(), Some(StageName::Tdd15));
        assert_eq!(StageName::Tdd15.next(), Some(StageName::Qa));
        assert_eq!(StageName::Qa.next(), Some(StageName::RedQueen));
        assert_eq!(StageName::RedQueen.next(), Some(StageName::GptReview));
        assert_eq!(StageName::GptReview.next(), Some(StageName::ShipGate));
        assert_eq!(StageName::ShipGate.next(), None);
    }

    #[test]
    fn given_stage_when_getting_model_tier_then_returns_efficient_tier() {
        assert_eq!(StageName::Contract.model_for_stage(), ModelTier::Fast);
        assert_eq!(StageName::DesignDag.model_for_stage(), ModelTier::Balanced);
        assert_eq!(StageName::Implement.model_for_stage(), ModelTier::Balanced);
        assert_eq!(StageName::Tdd15.model_for_stage(), ModelTier::Balanced);
        assert_eq!(StageName::Qa.model_for_stage(), ModelTier::Balanced);
        assert_eq!(StageName::RedQueen.model_for_stage(), ModelTier::Capable);
        assert_eq!(StageName::GptReview.model_for_stage(), ModelTier::Capable);
        assert_eq!(StageName::ShipGate.model_for_stage(), ModelTier::Best);
    }

    #[test]
    fn given_stage_when_getting_max_attempts_then_returns_three() {
        assert_eq!(StageName::Contract.max_attempts(), 3);
        assert_eq!(StageName::DesignDag.max_attempts(), 3);
        assert_eq!(StageName::Implement.max_attempts(), 3);
        assert_eq!(StageName::Tdd15.max_attempts(), 3);
        assert_eq!(StageName::Qa.max_attempts(), 3);
    }

    #[test]
    fn given_stage_when_getting_gates_then_returns_appropriate_gates() {
        assert_eq!(StageName::Contract.gates(), vec![Gate::Compiles]);
        assert_eq!(StageName::DesignDag.gates(), vec![Gate::Compiles]);
        assert_eq!(StageName::Implement.gates(), vec![Gate::Compiles]);
        assert_eq!(
            StageName::Tdd15.gates(),
            vec![Gate::Compiles, Gate::TestsPass]
        );
        assert_eq!(
            StageName::ShipGate.gates(),
            vec![Gate::MoonCi, Gate::ZjjMergeQueue]
        );
    }

    #[test]
    fn given_stage_name_roundtrip_then_preserves_value() {
        let stages = [
            "contract",
            "design_dag",
            "implement",
            "tdd15",
            "qa",
            "red_queen",
            "gpt_review",
            "ship_gate",
        ];
        for s in stages {
            let parsed = StageName::try_from(s);
            assert!(parsed.is_ok());
            assert_eq!(parsed.map(|stage| stage.as_str()), Ok(s));
        }
    }
}

#[cfg(test)]
mod model_tier_tests {
    use super::*;

    #[test]
    fn given_model_tier_roundtrip_then_preserves_value() {
        let tiers = ["fast", "balanced", "capable", "best"];
        for t in tiers {
            let parsed = ModelTier::try_from(t);
            assert!(parsed.is_ok());
            assert_eq!(parsed.map(|tier| tier.as_str()), Ok(t));
        }
    }
}

#[cfg(test)]
mod gate_tests {
    use super::*;

    #[test]
    fn given_gate_roundtrip_then_preserves_value() {
        let gates = [
            "compiles",
            "tests_pass",
            "edge_cases",
            "no_vulnerabilities",
            "clippy_clean",
            "security",
            "moon_ci",
            "zjj_merge_queue",
        ];
        for g in gates {
            let parsed = Gate::try_from(g);
            assert!(parsed.is_ok());
            assert_eq!(parsed.map(|gate| gate.as_str()), Ok(g));
        }
    }
}
