//! Pipeline-specific types: stages, gates, model tiers, results, failure categories,
//! and stage-transition policies.

use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Stage / Gate / Tier enumerations
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StageName {
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
        2
    }

    pub fn gates(&self) -> Vec<Gate> {
        match self {
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

impl TryFrom<&str> for StageName {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
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

// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Failure / result types
// ---------------------------------------------------------------------------

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
pub enum StageState {
    Pending,
    Running,
    Passed,
    Failed,
    WaitingPermission,
    WaitingQuestion,
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

// ---------------------------------------------------------------------------
// Stage transition state machine
// ---------------------------------------------------------------------------

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
