//! Pipeline-specific types: stages, gates, model tiers, results, failure categories,
//! and stage-transition policies.

use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

// ---------------------------------------------------------------------------
// Stage / Gate / Tier enumerations
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StageName {
    Explore,
    Contract,
    Red,
    Implementation,
    Witness,
    ShipGate,
}

impl StageName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Explore => "explore",
            Self::Contract => "contract",
            Self::Red => "red",
            Self::Implementation => "implementation",
            Self::Witness => "witness",
            Self::ShipGate => "ship_gate",
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Explore => Some(Self::Contract),
            Self::Contract => Some(Self::Red),
            Self::Red => Some(Self::Implementation),
            Self::Implementation => Some(Self::Witness),
            Self::Witness => Some(Self::ShipGate),
            Self::ShipGate => None,
        }
    }

    pub fn model_for_stage(&self) -> ModelTier {
        match self {
            Self::Explore => ModelTier::Fast,
            Self::Contract => ModelTier::Fast,
            Self::Red => ModelTier::Balanced,
            Self::Implementation => ModelTier::Balanced,
            Self::Witness => ModelTier::Capable,
            Self::ShipGate => ModelTier::Best,
        }
    }

    pub fn max_attempts(&self) -> u32 {
        2
    }

    pub fn gates(&self) -> Vec<Gate> {
        match self {
            Self::Explore => vec![],
            Self::Contract => vec![Gate::Compiles],
            Self::Red => vec![Gate::Compiles],
            Self::Implementation => vec![Gate::Compiles, Gate::TestsPass],
            Self::Witness => vec![Gate::HoldoutScenarios],
            Self::ShipGate => vec![Gate::CueArtifactGenerated, Gate::ZjjMergeQueue],
        }
    }
}

impl TryFrom<&str> for StageName {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "explore" => Ok(Self::Explore),
            "contract" => Ok(Self::Contract),
            "red" => Ok(Self::Red),
            "implementation" => Ok(Self::Implementation),
            "witness" => Ok(Self::Witness),
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
            Self::Fast => "d",
            Self::Balanced => "c",
            Self::Capable => "b",
            Self::Best => "a",
        }
    }
}

impl TryFrom<&str> for ModelTier {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "d" => Ok(Self::Fast),
            "c" => Ok(Self::Balanced),
            "b" => Ok(Self::Capable),
            "a" | "s" => Ok(Self::Best),
            _ => Err(format!("Unknown model tier: {s}")),
        }
    }
}

// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gate {
    Compiles,
    TestsPass,
    MoonCi,
    HoldoutScenarios,
    ZjjMergeQueue,
    CueArtifactGenerated,
}

impl Gate {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compiles => "compiles",
            Self::TestsPass => "tests_pass",
            Self::MoonCi => "moon_ci",
            Self::HoldoutScenarios => "holdout_scenarios",
            Self::ZjjMergeQueue => "zjj_merge_queue",
            Self::CueArtifactGenerated => "cue_artifact_generated",
        }
    }
}

impl TryFrom<&str> for Gate {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "compiles" => Ok(Self::Compiles),
            "tests_pass" => Ok(Self::TestsPass),
            "moon_ci" => Ok(Self::MoonCi),
            "holdout_scenarios" => Ok(Self::HoldoutScenarios),
            "zjj_merge_queue" => Ok(Self::ZjjMergeQueue),
            "cue_artifact_generated" => Ok(Self::CueArtifactGenerated),
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
    TestsUnexpectedlyGreen,
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

/// Typed failure details carried between stage attempts.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StageFailure {
    pub category: FailureCategory,
    pub message: String,
    pub retryable: bool,
    pub failed_at: String,
}

impl StageFailure {
    pub fn new(
        category: FailureCategory,
        message: String,
        retryable: bool,
        failed_at: String,
    ) -> Self {
        Self { category, message, retryable, failed_at }
    }

    pub fn with_reason(category: FailureCategory, message: String, failed_at: String) -> Self {
        let retryable = crate::is_retryable_failure(&category);
        Self::new(category, message, retryable, failed_at)
    }
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
        StageName::Explore => TransitionDecision::new(
            StageTransition::Advance(StageName::Contract),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Contract => TransitionDecision::new(
            StageTransition::Advance(StageName::Red),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Red => TransitionDecision::new(
            StageTransition::Advance(StageName::Implementation),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Implementation => TransitionDecision::new(
            StageTransition::Advance(StageName::Witness),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::Witness => TransitionDecision::new(
            StageTransition::Advance(StageName::ShipGate),
            TransitionReason::StagePassedAdvance,
        ),
        StageName::ShipGate => TransitionDecision::new(
            StageTransition::Complete,
            TransitionReason::RedQueenPassedComplete,
        ),
    }
}

// ---------------------------------------------------------------------------
// Model Tier Configuration
// ---------------------------------------------------------------------------

/// Configuration for model tiers, loaded from oya.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTierConfig {
    /// Map of tier identifier to list of model names
    pub tiers: HashMap<String, Vec<String>>,
}

impl ModelTierConfig {
    /// Get models for a specific tier
    pub fn get_models_for_tier(&self, tier: &str) -> Vec<String> {
        self.tiers.get(tier).cloned().unwrap_or_default()
    }

    /// Get all available tier identifiers
    pub fn tier_ids(&self) -> Vec<&str> {
        self.tiers.keys().map(|s: &String| s.as_str()).collect()
    }
}

impl FailureCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TestFailed => "test_failed",
            Self::TestsUnexpectedlyGreen => "tests_unexpectedly_green",
            Self::TestInfraFailed => "test_infra_failed",
            Self::CompileFailed => "compile_failed",
            Self::LintFailed => "lint_failed",
            Self::MergeConflict => "merge_conflict",
            Self::RateLimited => "rate_limited",
            Self::AuthFailed => "auth_failed",
            Self::ContextOverflow => "context_overflow",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::OutputParseFailure => "output_parse_failure",
            Self::MaxAttemptsExceeded => "max_attempts_exceeded",
        }
    }
}

/// Load model tier configuration from oya.yaml
/// Returns the config if file exists and parses successfully, error otherwise
pub fn load_model_tier_config() -> Result<ModelTierConfig> {
    let config_path = PathBuf::from("oya.yaml");

    let content = fs::read_to_string(&config_path)
        .map_err(|_| anyhow::anyhow!("Failed to read config file: {}", config_path.display()))?;

    #[derive(Deserialize)]
    struct OyaConfigFile {
        model_tiers: HashMap<String, Vec<String>>,
    }

    let config: OyaConfigFile = serde_yaml::from_str(&content)
        .map_err(|_| anyhow::anyhow!("Failed to parse model_tiers from oya.yaml"))?;

    Ok(ModelTierConfig { tiers: config.model_tiers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_model_tier_config_from_oya_yaml() {
        let config = load_model_tier_config();
        assert!(config.is_ok(), "Should load config from oya.yaml");

        let config = config.unwrap();
        assert!(config.tiers.contains_key("d"));
        assert!(config.tiers.contains_key("c"));
        assert!(config.tiers.contains_key("b"));
        assert!(config.tiers.contains_key("a"));
        assert!(config.tiers.contains_key("s"));
    }

    #[test]
    fn test_get_models_for_tier_returns_correct_models() {
        let config = load_model_tier_config().unwrap();

        let tier_d = config.get_models_for_tier("d");
        assert_eq!(tier_d.len(), 1);
        assert_eq!(tier_d[0], "zai-coding-plan/glm-4.6");

        let tier_c = config.get_models_for_tier("c");
        assert_eq!(tier_c.len(), 4);
        assert!(tier_c.contains(&"opencode/glm-5-free".to_string()));
    }

    #[test]
    fn test_get_models_for_tier_returns_empty_for_unknown() {
        let config = load_model_tier_config().unwrap();
        assert!(config.get_models_for_tier("unknown").is_empty());
        assert!(config.get_models_for_tier("fast").is_empty());
        assert!(config.get_models_for_tier("balanced").is_empty());
        assert!(config.get_models_for_tier("capable").is_empty());
        assert!(config.get_models_for_tier("best").is_empty());
    }

    #[test]
    fn test_tier_ids_returns_all_tiers() {
        let config = load_model_tier_config().unwrap();
        let tiers = config.tier_ids();
        assert!(tiers.contains(&"d"));
        assert!(tiers.contains(&"c"));
        assert!(tiers.contains(&"b"));
        assert!(tiers.contains(&"a"));
        assert!(tiers.contains(&"s"));
    }
}
