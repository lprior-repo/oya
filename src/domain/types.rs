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
}
