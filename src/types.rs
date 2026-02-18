//! Domain types, entities, and policies for Oya pipeline orchestration.

use chrono::{DateTime, Utc};
use im::Vector;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

// =============================================================================
//  Value Objects - IDs (Wlaschin: Types as constraints)
// =============================================================================

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
    Research,
    Plan,
    Contract,
    Tdd15,
    Qa,
    RedQueen,
    GptReview,
    ShipGate,
}

impl StageName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Research => "research",
            Self::Plan => "plan",
            Self::Contract => "contract",
            Self::Tdd15 => "tdd15",
            Self::Qa => "qa",
            Self::RedQueen => "red_queen",
            Self::GptReview => "gpt_review",
            Self::ShipGate => "ship_gate",
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Research => Some(Self::Plan),
            Self::Plan => Some(Self::Contract),
            Self::Contract => Some(Self::Tdd15),
            Self::Tdd15 => Some(Self::Qa),
            Self::Qa => Some(Self::RedQueen),
            Self::RedQueen => Some(Self::GptReview),
            Self::GptReview => Some(Self::ShipGate),
            Self::ShipGate => None,
        }
    }

    pub fn model_for_stage(&self) -> ModelTier {
        match self {
            Self::Research => ModelTier::Fast,
            Self::Plan => ModelTier::Balanced,
            Self::Contract => ModelTier::Fast,
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
            Self::Research => vec![Gate::Compiles],
            Self::Plan => vec![Gate::Compiles],
            Self::Contract => vec![Gate::Compiles],
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
            "research" => Ok(Self::Research),
            "plan" => Ok(Self::Plan),
            "contract" => Ok(Self::Contract),
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApproverMode {
    Auto,
    Human,
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
    #[error("Parse error: {0}")]
    ParseError(String),
}
// =============================================================================
//  Domain Entities
// =============================================================================

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

    pub fn validate_invariants(&self) -> Result<(), ValidationError> {
        match self.status {
            AgentStatus::Working => {
                if self.bead_id.is_none() {
                    return Err(ValidationError::InvalidState(
                        "Agent with Working status must have a bead".to_string(),
                    ));
                }
                if self.current_stage.is_none() {
                    return Err(ValidationError::InvalidState(
                        "Agent with Working status must have a current_stage".to_string(),
                    ));
                }
            }
            AgentStatus::Done => {
                if self.bead_id.is_some() {
                    return Err(ValidationError::InvalidState(
                        "Agent with Done status must not have a bead".to_string(),
                    ));
                }
                if self.current_stage.is_some() {
                    return Err(ValidationError::InvalidState(
                        "Agent with Done status must have no active stage".to_string(),
                    ));
                }
            }
            AgentStatus::Idle | AgentStatus::Waiting | AgentStatus::Error => {
                if self.bead_id.is_some() {
                    return Err(ValidationError::InvalidState(format!(
                        "Agent with {:?} status must not have a bead",
                        self.status
                    )));
                }
            }
        }
        Ok(())
    }
}

// =============================================================================
//  Run Aggregate - Pure Functional State Transitions
// =============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RunState {
    Pending,
    Running { current_stage: StageName },
    Waiting { reason: String },
    Shipped { completed_at: DateTime<Utc> },
    Failed { reason: String, failed_at: DateTime<Utc> },
    Aborted { reason: String, aborted_at: DateTime<Utc> },
}

/// Custom serde module for im::Vector serialization
mod im_vector_serde {
    use im::Vector;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, T>(vec: &Vector<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize + Clone,
    {
        vec.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Vector<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de> + Clone,
    {
        Vec::<T>::deserialize(deserializer).map(Vector::from)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    pub bead_id: BeadId,
    pub state: RunState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// History of stage executions - persistent vector for structural sharing
    #[serde(with = "im_vector_serde")]
    pub history: Vector<StageAttempt>,
}

impl Run {
    /// Create a new Run in Pending state with empty history
    pub fn new(bead_id: BeadId) -> Self {
        let now = Utc::now();
        Self {
            id: RunId::new(),
            bead_id,
            state: RunState::Pending,
            created_at: now,
            updated_at: now,
            history: Vector::new(),
        }
    }

    /// Transition from Pending to Running (Research stage)
    /// Pure functional: returns new state, does not mutate
    pub fn start(&self) -> Result<Self, DomainError> {
        match &self.state {
            RunState::Pending => Ok(Self {
                state: RunState::Running { current_stage: StageName::Research },
                updated_at: Utc::now(),
                ..self.clone()
            }),
            s => {
                Err(DomainError::InvalidStateTransition(format!("{:?}", s), "Running".to_string()))
            }
        }
    }

    /// Complete a stage and transition to next stage or Shipped
    /// Pure functional: returns new state with updated history
    pub fn complete_stage(
        &self,
        stage: StageName,
        _result: StageResult,
    ) -> Result<Self, DomainError> {
        match &self.state {
            RunState::Running { current_stage } if *current_stage == stage => {
                let next_state = stage.next().map_or_else(
                    || RunState::Shipped { completed_at: Utc::now() },
                    |ns| RunState::Running { current_stage: ns },
                );

                Ok(Self { state: next_state, updated_at: Utc::now(), ..self.clone() })
            }
            s => Err(DomainError::InvalidStateTransition(
                format!("{:?}", s),
                "NextStage".to_string(),
            )),
        }
    }

    /// Transition to Failed state
    /// Pure functional: returns new state
    pub fn fail(&self, reason: String) -> Self {
        Self {
            state: RunState::Failed { reason, failed_at: Utc::now() },
            updated_at: Utc::now(),
            ..self.clone()
        }
    }

    /// Append a stage attempt to history
    /// Pure functional: returns new state with appended history
    pub fn with_attempt(&self, attempt: StageAttempt) -> Self {
        let mut history = self.history.clone();
        history.push_back(attempt);
        Self { history, updated_at: Utc::now(), ..self.clone() }
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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Placeholder value in {0}: {1}")]
    PlaceholderValue(String, String),
    #[error("Invalid exit code: {0}")]
    InvalidExitCode(i32),
    #[error("Inconsistent evidence: {0}")]
    InconsistentEvidence(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateResult {
    pub run_id: String,
    pub gate_name: String,
    pub command: Option<String>,
    pub passed: bool,
    pub exit_code: i32,
    pub log_ref: Option<String>,
}

const PLACEHOLDERS: &[&str] = &["todo", "placeholder", "not implemented", "tbd", "tbc"];

fn contains_placeholder(value: &str) -> bool {
    PLACEHOLDERS.iter().any(|p| value.to_lowercase().contains(p))
}

fn validate_field_no_placeholder(field_name: &str, value: &str) -> Result<(), ValidationError> {
    if contains_placeholder(value) {
        return Err(ValidationError::PlaceholderValue(field_name.to_string(), value.to_string()));
    }
    Ok(())
}

impl GateResult {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check non-empty required fields
        if self.run_id.is_empty() {
            return Err(ValidationError::MissingField("run_id".to_string()));
        }
        if self.gate_name.is_empty() {
            return Err(ValidationError::MissingField("gate_name".to_string()));
        }

        // Validate command field (required)
        match &self.command {
            None => return Err(ValidationError::MissingField("command".to_string())),
            Some(command) => {
                if command.is_empty() {
                    return Err(ValidationError::MissingField("command".to_string()));
                }
                validate_field_no_placeholder("command", command)?;
            }
        }

        // Validate gate_name has no placeholders
        validate_field_no_placeholder("gate_name", &self.gate_name)?;

        // Validate log_ref has no placeholders (if present)
        if let Some(ref log) = self.log_ref {
            validate_field_no_placeholder("log_ref", log)?;
        }

        // Validate exit code range (0-255)
        if self.exit_code < 0 || self.exit_code > 255 {
            return Err(ValidationError::InvalidExitCode(self.exit_code));
        }

        // Check consistency between passed and exit_code
        let exit_matches_passed = (self.exit_code == 0) == self.passed;
        if !exit_matches_passed {
            let description = if self.passed {
                "passed=true but exit_code≠0".to_string()
            } else {
                "passed=false but exit_code=0".to_string()
            };
            return Err(ValidationError::InconsistentEvidence(description));
        }

        Ok(())
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
//  Domain Policies
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
        StageName::Research => TransitionDecision::new(
            StageTransition::Advance(StageName::Plan),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Plan => TransitionDecision::new(
            StageTransition::Advance(StageName::Contract),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Contract => TransitionDecision::new(
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
            StageTransition::Complete,
            TransitionReason::RedQueenPassedComplete,
        ),
    }
}

// =============================================================================
//  Timeline Types (Wlaschin: Make illegal states unrepresentable)
// =============================================================================

/// Newtype for workspace names - prevents stringly typed confusion
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WorkspaceName(pub String);

impl WorkspaceName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Newtype for duration in milliseconds
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DurationMs(pub u64);

impl DurationMs {
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs * 1000)
    }

    pub const fn as_secs(self) -> u64 {
        self.0 / 1000
    }
}

impl std::fmt::Display for DurationMs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 1000 {
            write!(f, "{}ms", self.0)
        } else {
            write!(f, "{:.1}s", self.0 as f64 / 1000.0)
        }
    }
}

/// Outcome of a stage - mutually exclusive states
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StageOutcome {
    Passed { gates: Vec<GateResult> },
    Failed { category: FailureCategory, message: String },
    RetryScheduled { next_attempt: u32, reason: String },
}

impl StageOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }
}

/// Gate result with minimal data
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateSummary {
    pub gate: String,
    pub passed: bool,
}

/// Rich stage started event - combines old stage_start + workspace_ready
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageStartedEvent {
    pub stage: StageName,
    pub attempt: u32,
    pub workspace: Option<WorkspaceName>,
    pub started_at: DateTime<Utc>,
}

/// Rich stage completed event - combines old stage_pass with gate results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageCompletedEvent {
    pub stage: StageName,
    pub attempt: u32,
    pub workspace: Option<WorkspaceName>,
    pub duration: DurationMs,
    pub gates: Vec<GateSummary>,
    pub completed_at: DateTime<Utc>,
}

/// Rich stage failed event - combines old stage_fail + retry info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageFailedEvent {
    pub stage: StageName,
    pub attempt: u32,
    pub workspace: Option<WorkspaceName>,
    pub duration: DurationMs,
    pub outcome: StageOutcome,
    pub failed_at: DateTime<Utc>,
}

/// Run-level events - terminal states
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunEvent {
    Started {
        bead_id: BeadId,
        context: String,
        started_at: DateTime<Utc>,
    },
    Shipped {
        completed_at: DateTime<Utc>,
        total_duration: DurationMs,
        stages_passed: u32,
    },
    Failed {
        stage: StageName,
        category: FailureCategory,
        message: String,
        failed_at: DateTime<Utc>,
    },
}

/// Unified timeline event - 3 per stage + run-level events
/// This replaces the verbose 24+ events with ~11 rich events:
/// - 1 RunStarted
/// - 8 stages × (Started + Completed) = 16 (or fewer if early failure)
/// - 1 RunShipped or RunFailed
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TimelineEntry {
    RunStarted {
        bead_id: String,
        context: String,
        at: DateTime<Utc>,
    },
    StageStarted {
        stage: String,
        attempt: u32,
        workspace: Option<String>,
        at: DateTime<Utc>,
    },
    StageCompleted {
        stage: String,
        attempt: u32,
        workspace: Option<String>,
        duration_ms: u64,
        gates: Vec<GateSummary>,
        at: DateTime<Utc>,
    },
    StageFailed {
        stage: String,
        attempt: u32,
        workspace: Option<String>,
        duration_ms: u64,
        category: String,
        message: String,
        retry_scheduled: bool,
        at: DateTime<Utc>,
    },
    RunShipped {
        total_duration_ms: u64,
        stages_passed: u32,
        at: DateTime<Utc>,
    },
    RunFailed {
        stage: String,
        category: String,
        at: DateTime<Utc>,
    },
}

impl TimelineEntry {
    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Self::RunStarted { at, .. } => at,
            Self::StageStarted { at, .. } => at,
            Self::StageCompleted { at, .. } => at,
            Self::StageFailed { at, .. } => at,
            Self::RunShipped { at, .. } => at,
            Self::RunFailed { at, .. } => at,
        }
    }

    pub fn stage(&self) -> Option<&str> {
        match self {
            Self::RunStarted { .. } => None,
            Self::StageStarted { stage, .. } => Some(stage),
            Self::StageCompleted { stage, .. } => Some(stage),
            Self::StageFailed { stage, .. } => Some(stage),
            Self::RunShipped { .. } => None,
            Self::RunFailed { stage, .. } => Some(stage),
        }
    }
}

// =============================================================================
//  Text Utilities - Pure Functions (Calculations)
// =============================================================================

/// Strip ANSI escape codes from a string (pure function)
/// Removes color codes, cursor movements, and other terminal escapes
pub fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    chars.next();
                    while let Some(&peek) = chars.peek() {
                        match peek {
                            '0'..='9' | ';' | '?' => {
                                chars.next();
                            }
                            _ => {
                                chars.next();
                                break;
                            }
                        }
                    }
                    continue;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Truncate text to max chars, appending truncation marker if needed
pub fn truncate_clean(input: &str, max_chars: usize) -> String {
    let stripped = strip_ansi_codes(input);
    let chars: Vec<char> = stripped.chars().take(max_chars).collect();
    if stripped.chars().count() > max_chars {
        format!("{}…[truncated]", chars.into_iter().collect::<String>())
    } else {
        chars.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_color_codes() {
        let input = "\x1b[32mSuccess\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "Success");
    }

    #[test]
    fn strip_ansi_removes_dim_codes() {
        let input = "\x1b[2m2026-02-18\x1b[0m INFO";
        assert_eq!(strip_ansi_codes(input), "2026-02-18 INFO");
    }

    #[test]
    fn strip_ansi_preserves_plain_text() {
        let input = "Hello, world!";
        assert_eq!(strip_ansi_codes(input), "Hello, world!");
    }

    #[test]
    fn truncate_clean_strips_and_truncates() {
        let input = "\x1b[32mThis is a long message\x1b[0m";
        let result = truncate_clean(input, 10);
        assert!(result.contains("This is a"));
        assert!(result.contains("truncated"));
    }

    #[test]
    fn duration_ms_display() {
        assert_eq!(format!("{}", DurationMs(500)), "500ms");
        assert_eq!(format!("{}", DurationMs(1500)), "1.5s");
        assert_eq!(format!("{}", DurationMs(61000)), "61.0s");
    }
}
