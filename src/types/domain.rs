//! Domain entities: errors, agent state, run aggregate, gate results, artifacts.

use super::ids::{AgentId, BeadId, RunId};
use super::pipeline::{StageAttempt, StageName, StageResult};
use chrono::{DateTime, Utc};
use im::Vector;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Agent types
// ---------------------------------------------------------------------------

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

    pub fn validate_invariants(&self) -> Result<(), ValidationError> {
        match self.status {
            AgentStatus::Working => validate_working_agent(self),
            AgentStatus::Done => validate_done_agent(self),
            AgentStatus::Idle | AgentStatus::Waiting | AgentStatus::Error => {
                validate_idle_agent(self)
            }
        }
    }
}

fn validate_working_agent(agent: &AgentState) -> Result<(), ValidationError> {
    if agent.bead_id.is_none() {
        return Err(ValidationError::InvalidState(
            "Agent with Working status must have a bead".to_string(),
        ));
    }
    if agent.current_stage.is_none() {
        return Err(ValidationError::InvalidState(
            "Agent with Working status must have a current_stage".to_string(),
        ));
    }
    Ok(())
}

fn validate_done_agent(agent: &AgentState) -> Result<(), ValidationError> {
    if agent.bead_id.is_some() {
        return Err(ValidationError::InvalidState(
            "Agent with Done status must not have a bead".to_string(),
        ));
    }
    if agent.current_stage.is_some() {
        return Err(ValidationError::InvalidState(
            "Agent with Done status must have no active stage".to_string(),
        ));
    }
    Ok(())
}

fn validate_idle_agent(agent: &AgentState) -> Result<(), ValidationError> {
    if agent.bead_id.is_some() {
        return Err(ValidationError::InvalidState(format!(
            "Agent with {:?} status must not have a bead",
            agent.status
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Run aggregate
// ---------------------------------------------------------------------------

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
    #[serde(with = "im_vector_serde")]
    pub history: Vector<StageAttempt>,
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
            history: Vector::new(),
        }
    }

    pub fn start(&self) -> Result<Self, DomainError> {
        match &self.state {
            RunState::Pending => Ok(Self {
                state: RunState::Running { current_stage: StageName::Contract },
                updated_at: Utc::now(),
                ..self.clone()
            }),
            s => {
                Err(DomainError::InvalidStateTransition(format!("{:?}", s), "Running".to_string()))
            }
        }
    }

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

    pub fn fail(&self, reason: String) -> Self {
        Self {
            state: RunState::Failed { reason, failed_at: Utc::now() },
            updated_at: Utc::now(),
            ..self.clone()
        }
    }

    pub fn with_attempt(&self, attempt: StageAttempt) -> Self {
        let mut history = self.history.clone();
        history.push_back(attempt);
        Self { history, updated_at: Utc::now(), ..self.clone() }
    }
}

// ---------------------------------------------------------------------------
// GateResult + validation
// ---------------------------------------------------------------------------

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
        validate_required_gate_fields(self)?;
        validate_command_field(self.command.as_ref())?;
        validate_field_no_placeholder("gate_name", &self.gate_name)?;
        validate_optional_log_ref(self.log_ref.as_deref())?;
        validate_exit_code_range(self.exit_code)?;
        validate_passed_exit_code_consistency(self.passed, self.exit_code)?;
        Ok(())
    }
}

fn validate_required_gate_fields(gate: &GateResult) -> Result<(), ValidationError> {
    if gate.run_id.is_empty() {
        return Err(ValidationError::MissingField("run_id".to_string()));
    }
    if gate.gate_name.is_empty() {
        return Err(ValidationError::MissingField("gate_name".to_string()));
    }
    Ok(())
}

fn validate_command_field(command: Option<&String>) -> Result<(), ValidationError> {
    let command = command.ok_or_else(|| ValidationError::MissingField("command".to_string()))?;
    if command.is_empty() {
        return Err(ValidationError::MissingField("command".to_string()));
    }
    validate_field_no_placeholder("command", command)
}

fn validate_optional_log_ref(log_ref: Option<&str>) -> Result<(), ValidationError> {
    if let Some(log) = log_ref {
        validate_field_no_placeholder("log_ref", log)?;
    }
    Ok(())
}

fn validate_exit_code_range(exit_code: i32) -> Result<(), ValidationError> {
    if !(0..=255).contains(&exit_code) {
        return Err(ValidationError::InvalidExitCode(exit_code));
    }
    Ok(())
}

fn validate_passed_exit_code_consistency(
    passed: bool,
    exit_code: i32,
) -> Result<(), ValidationError> {
    if (exit_code == 0) == passed {
        return Ok(());
    }
    let description = if passed {
        "passed=true but exit_code≠0".to_string()
    } else {
        "passed=false but exit_code=0".to_string()
    };
    Err(ValidationError::InconsistentEvidence(description))
}

// ---------------------------------------------------------------------------
// Misc types
// ---------------------------------------------------------------------------

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
