//! Domain entities: errors, agent state, run aggregate, gate results, artifacts.

use super::ids::{AgentId, BeadId, RunId};
use super::pipeline::{StageAttempt, StageName, StageResult};
use chrono::{DateTime, Utc};
use im::Vector;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
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
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AgentStatusData {
    Idle,
    Working { bead_id: BeadId, current_stage: StageName, stage_started_at: DateTime<Utc> },
    Waiting { bead_id: BeadId, current_stage: StageName },
    Error { bead_id: Option<BeadId>, message: String },
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: AgentId,
    #[serde(flatten)]
    pub status: AgentStatusData,
    pub last_update: DateTime<Utc>,
    pub implementation_attempt: u32,
    pub feedback: Option<String>,
}

impl AgentState {
    pub fn new(agent_id: AgentId, status: AgentStatusData, implementation_attempt: u32) -> Self {
        Self { agent_id, status, last_update: Utc::now(), implementation_attempt, feedback: None }
    }

    pub fn validate_invariants(&self) -> Result<(), ValidationError> {
        // Enforced by type structure.
        Ok(())
    }
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
                state: RunState::Running { current_stage: StageName::JjWorkspace },
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
// Queue / lock / merge contracts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QueuePosition(NonZeroU32);

impl QueuePosition {
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0.get()
    }

    #[must_use]
    pub fn next(self) -> Self {
        let next = self.0.get().saturating_add(1);
        Self(NonZeroU32::new(next).unwrap_or(NonZeroU32::MIN))
    }
}

impl TryFrom<u32> for QueuePosition {
    type Error = ValidationError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .map(Self)
            .ok_or_else(|| ValidationError::InvalidState("queue_position must be > 0".to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LockToken(String);

impl LockToken {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<&str> for LockToken {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ValidationError::MissingField("lock_token".to_string()));
        }
        if contains_forbidden_control_chars(trimmed) {
            return Err(ValidationError::InvalidState(
                "lock_token contains control characters".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeBlockReason {
    LockUnavailable,
    DependencyPending,
    QueueConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum MergeDecision {
    Merge { queue_position: QueuePosition, lock: LockToken },
    Requeue { reason: MergeBlockReason, queue_position: QueuePosition },
    Block { reason: MergeBlockReason },
}

#[must_use]
pub fn derive_merge_decision(
    queue_position: QueuePosition,
    lock: Option<LockToken>,
    dependencies_ready: bool,
) -> MergeDecision {
    if !dependencies_ready {
        return MergeDecision::Requeue {
            reason: MergeBlockReason::DependencyPending,
            queue_position: queue_position.next(),
        };
    }

    match lock {
        Some(lock) => MergeDecision::Merge { queue_position, lock },
        None => MergeDecision::Requeue {
            reason: MergeBlockReason::LockUnavailable,
            queue_position: queue_position.next(),
        },
    }
}

/// Parse a raw serialized queue record into a validated QueueItem
///
/// # Returns
/// - `Ok(QueueItem)` with all fields wrapped in validated newtypes
/// - `Err(ValidationError)` with field-scoped parse diagnostics
pub fn parse_queue_record(
    id: &str,
    bead_id: &str,
    priority: u8,
    sha: &str,
    freshness_base_rev: &str,
) -> Result<QueueItem, ValidationError> {
    QueueItem::try_new(id, bead_id, priority, sha, freshness_base_rev)
}

/// Select next merge candidate from queue snapshot
///
/// # Returns
/// - `Ok(SelectionDecision)` with exhaustive variant based on queue/state
/// - `Err(ValidationError)` if queue snapshot is invalid
pub fn select_next_merge_candidate(
    queue_snapshot: &[QueueItem],
    current_lock: Option<&SessionLock>,
    now_epoch_seconds: u64,
    main_revision: &FullSha,
) -> Result<SelectionDecision, ValidationError> {
    if queue_snapshot.is_empty() {
        return Ok(SelectionDecision::Blocked {
            reason: BlockReason::LockUnavailable {
                owner: current_lock.map(|l| l.token.as_str().to_string()),
                expires_at: current_lock.map(|l| l.expires_at),
            },
        });
    }

    // Check if there's an active lock (owned by someone else and not expired)
    let lock_active = current_lock.map(|lock| !lock.is_expired(now_epoch_seconds)).unwrap_or(false);

    if lock_active {
        return Ok(SelectionDecision::Blocked {
            reason: BlockReason::LockUnavailable {
                owner: current_lock.map(|l| l.token.as_str().to_string()),
                expires_at: current_lock.map(|l| l.expires_at),
            },
        });
    }

    // Sort by priority (higher first), then by id for determinism
    let mut sorted_items: Vec<&QueueItem> = queue_snapshot.iter().collect();
    sorted_items.sort_by(|a, b| {
        let priority_cmp = b.priority.as_u8().cmp(&a.priority.as_u8());
        if priority_cmp == std::cmp::Ordering::Equal {
            a.id.as_str().cmp(b.id.as_str())
        } else {
            priority_cmp
        }
    });

    // Find first item that's not stale (freshness matches main)
    Ok(sorted_items
        .into_iter()
        .find(|item| item.freshness_base_rev.as_str() == main_revision.as_str())
        .map(|item| SelectionDecision::Ready { queue_item: item.clone() })
        .unwrap_or_else(|| SelectionDecision::Stale { reason: StaleReason::BaseRevisionAdvanced }))
}

fn contains_forbidden_control_chars(value: &str) -> bool {
    value.chars().any(|ch| ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t')
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

// ---------------------------------------------------------------------------
// Queue / lock / merge contracts - NEW TYPED DOMAIN TYPES
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QueueItemId(String);

impl QueueItemId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<&str> for QueueItemId {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ValidationError::MissingField("queue_item_id".to_string()));
        }
        if contains_forbidden_control_chars(trimmed) {
            return Err(ValidationError::InvalidState(
                "queue_item_id contains control characters".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QueueBeadId(String);

impl QueueBeadId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<&str> for QueueBeadId {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ValidationError::MissingField("queue_bead_id".to_string()));
        }
        if contains_forbidden_control_chars(trimmed) {
            return Err(ValidationError::InvalidState(
                "queue_bead_id contains control characters".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NonZeroPriority(u8);

impl NonZeroPriority {
    #[must_use]
    pub const fn as_u8(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for NonZeroPriority {
    type Error = ValidationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 0 {
            return Err(ValidationError::InvalidState("priority must be > 0".to_string()));
        }
        if value > 10 {
            return Err(ValidationError::InvalidState("priority must be <= 10".to_string()));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FullSha(String);

impl FullSha {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<&str> for FullSha {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.len() != 40 {
            return Err(ValidationError::InvalidState("sha must be 40 characters".to_string()));
        }
        if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ValidationError::InvalidState("sha must be hexadecimal".to_string()));
        }
        Ok(Self(trimmed.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: QueueItemId,
    pub bead_id: QueueBeadId,
    pub priority: NonZeroPriority,
    pub sha: FullSha,
    pub freshness_base_rev: FullSha,
}

impl QueueItem {
    pub fn try_new(
        id: &str,
        bead_id: &str,
        priority: u8,
        sha: &str,
        freshness_base_rev: &str,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            id: QueueItemId::try_from(id)?,
            bead_id: QueueBeadId::try_from(bead_id)?,
            priority: NonZeroPriority::try_from(priority)?,
            sha: FullSha::try_from(sha)?,
            freshness_base_rev: FullSha::try_from(freshness_base_rev)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLock {
    pub token: LockToken,
    pub acquired_at: u64,
    pub expires_at: u64,
}

impl SessionLock {
    pub fn try_new(
        token: &str,
        acquired_at: u64,
        ttl_seconds: u64,
    ) -> Result<Self, ValidationError> {
        let lock_token = LockToken::try_from(token)?;
        let expires_at = acquired_at.saturating_add(ttl_seconds);
        if ttl_seconds == 0 {
            return Err(ValidationError::InvalidState("ttl_seconds must be > 0".to_string()));
        }
        if expires_at <= acquired_at {
            return Err(ValidationError::InvalidState(
                "expires_at must be > acquired_at".to_string(),
            ));
        }
        Ok(Self { token: lock_token, acquired_at, expires_at })
    }

    #[must_use]
    pub fn is_expired(&self, now_epoch_seconds: u64) -> bool {
        now_epoch_seconds >= self.expires_at
    }

    #[must_use]
    pub fn is_owned_by(&self, token: &str) -> bool {
        self.token.as_str() == token
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionDecision {
    Ready { queue_item: QueueItem },
    Blocked { reason: BlockReason },
    Stale { reason: StaleReason },
    Conflict { bead_id: QueueBeadId },
    Merged { bead_id: QueueBeadId, queue_position: QueuePosition },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    LockUnavailable { owner: Option<String>, expires_at: Option<u64> },
    DependencyPending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
    BaseRevisionAdvanced,
    ConflictDetected,
}

#[cfg(test)]
mod queue_domain_tests {
    use super::*;

    #[test]
    fn given_valid_inputs_when_creating_queue_item_then_succeeds() {
        let result = QueueItem::try_new(
            "queue-item-1",
            "src-abc123",
            5,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert!(result.is_ok());
        let item = result.unwrap();
        assert_eq!(item.id.as_str(), "queue-item-1");
        assert_eq!(item.bead_id.as_str(), "src-abc123");
        assert_eq!(item.priority.as_u8(), 5);
    }

    #[test]
    fn given_zero_priority_when_creating_queue_item_then_fails() {
        let result = QueueItem::try_new(
            "queue-item-1",
            "src-abc123",
            0,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::InvalidState(_)));
    }

    #[test]
    fn given_priority_gt_10_when_creating_queue_item_then_fails() {
        let result = QueueItem::try_new(
            "queue-item-1",
            "src-abc123",
            11,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert!(result.is_err());
    }

    #[test]
    fn given_invalid_sha_length_when_creating_queue_item_then_fails() {
        let result = QueueItem::try_new(
            "queue-item-1",
            "src-abc123",
            5,
            "aaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert!(result.is_err());
    }

    #[test]
    fn given_non_hex_sha_when_creating_queue_item_then_fails() {
        let result = QueueItem::try_new(
            "queue-item-1",
            "src-abc123",
            5,
            "gggggggggggggggggggggggggggggggggggggggg",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert!(result.is_err());
    }

    #[test]
    fn given_empty_bead_id_when_creating_queue_item_then_fails() {
        let result = QueueItem::try_new(
            "queue-item-1",
            "",
            5,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::MissingField(_)));
    }

    #[test]
    fn given_valid_inputs_when_creating_session_lock_then_succeeds() {
        let result = SessionLock::try_new("worker-1", 1000, 60);
        assert!(result.is_ok());
        let lock = result.unwrap();
        assert_eq!(lock.token.as_str(), "worker-1");
        assert_eq!(lock.acquired_at, 1000);
        assert_eq!(lock.expires_at, 1060);
    }

    #[test]
    fn given_zero_ttl_when_creating_session_lock_then_fails() {
        let result = SessionLock::try_new("worker-1", 1000, 0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::InvalidState(_)));
    }

    #[test]
    fn given_empty_token_when_creating_session_lock_then_fails() {
        let result = SessionLock::try_new("", 1000, 60);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::MissingField(_)));
    }

    #[test]
    fn given_expired_lock_when_checking_expiration_then_true() {
        let lock = SessionLock::try_new("worker-1", 1000, 60).unwrap();
        assert!(lock.is_expired(1060));
        assert!(lock.is_expired(2000));
    }

    #[test]
    fn given_valid_lock_when_checking_expiration_then_false() {
        let lock = SessionLock::try_new("worker-1", 1000, 60).unwrap();
        assert!(!lock.is_expired(1000));
        assert!(!lock.is_expired(1050));
        assert!(!lock.is_expired(1059));
    }

    #[test]
    fn given_valid_queue_item_when_creating_ready_decision_then_succeeds() {
        let item = QueueItem::try_new(
            "queue-item-1",
            "src-abc123",
            5,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap();
        let decision = SelectionDecision::Ready { queue_item: item.clone() };
        assert!(matches!(decision, SelectionDecision::Ready { queue_item: _ }));
    }

    #[test]
    fn given_lock_unavailable_when_creating_blocked_decision_then_succeeds() {
        let decision = SelectionDecision::Blocked {
            reason: BlockReason::LockUnavailable {
                owner: Some("worker-1".to_string()),
                expires_at: Some(1060),
            },
        };
        assert!(matches!(decision, SelectionDecision::Blocked { .. }));
    }

    #[test]
    fn given_stale_data_when_creating_stale_decision_then_succeeds() {
        let decision = SelectionDecision::Stale { reason: StaleReason::BaseRevisionAdvanced };
        assert!(matches!(decision, SelectionDecision::Stale { .. }));
    }

    #[test]
    fn given_conflict_when_creating_conflict_decision_then_succeeds() {
        let bead_id = QueueBeadId::try_from("src-abc123").unwrap();
        let decision = SelectionDecision::Conflict { bead_id };
        assert!(matches!(decision, SelectionDecision::Conflict { .. }));
    }

    #[test]
    fn given_completed_merge_when_creating_merged_decision_then_succeeds() {
        let bead_id = QueueBeadId::try_from("src-abc123").unwrap();
        let queue_position = QueuePosition::try_from(1).unwrap();
        let decision = SelectionDecision::Merged { bead_id, queue_position };
        assert!(matches!(decision, SelectionDecision::Merged { .. }));
    }
}

#[cfg(test)]
mod agent_domain_tests {
    use super::*;
    use anyhow::Result;
    use chrono::Utc;

    #[test]
    fn given_idle_status_when_constructing_agent_then_succeeds() -> Result<()> {
        let agent_id = AgentId("agent-1".to_string());
        let agent = AgentState::new(agent_id, AgentStatusData::Idle, 0);

        assert_eq!(agent.agent_id.0, "agent-1");
        assert!(matches!(agent.status, AgentStatusData::Idle));
        Ok(())
    }

    #[test]
    fn given_working_status_when_constructing_agent_then_has_all_fields() -> Result<()> {
        let agent_id = AgentId("agent-1".to_string());
        let bead_id = BeadId::new("bead-1");
        let now = Utc::now();

        let agent = AgentState::new(
            agent_id,
            AgentStatusData::Working {
                bead_id: bead_id.clone(),
                current_stage: StageName::Implementation,
                stage_started_at: now,
            },
            1,
        );

        if let AgentStatusData::Working { bead_id: b, current_stage: s, stage_started_at: t } =
            agent.status
        {
            assert_eq!(b, bead_id);
            assert_eq!(s, StageName::Implementation);
            assert_eq!(t, now);
        } else {
            anyhow::bail!("Expected Working status");
        }
        assert_eq!(agent.implementation_attempt, 1);
        Ok(())
    }

    #[test]
    fn given_error_status_when_constructing_agent_then_has_message() -> Result<()> {
        let agent_id = AgentId("agent-1".to_string());
        let agent = AgentState::new(
            agent_id,
            AgentStatusData::Error { bead_id: None, message: "Something went wrong".to_string() },
            0,
        );

        if let AgentStatusData::Error { bead_id: b, message: m } = agent.status {
            assert!(b.is_none());
            assert_eq!(m, "Something went wrong");
        } else {
            anyhow::bail!("Expected Error status");
        }
        Ok(())
    }

    #[test]
    fn given_agent_state_when_serializing_then_roundtrips() -> Result<()> {
        let agent_id = AgentId("agent-1".to_string());
        let bead_id = BeadId::new("bead-1");
        let now = Utc::now();

        let agent = AgentState::new(
            agent_id,
            AgentStatusData::Working {
                bead_id,
                current_stage: StageName::Implementation,
                stage_started_at: now,
            },
            2,
        );

        let serialized = serde_json::to_string(&agent)?;
        let deserialized: AgentState = serde_json::from_str(&serialized)?;

        assert_eq!(deserialized.agent_id.0, agent.agent_id.0);
        assert_eq!(deserialized.implementation_attempt, 2);

        if let AgentStatusData::Working { bead_id: b, .. } = deserialized.status {
            assert_eq!(b.0, "bead-1");
        } else {
            anyhow::bail!("Expected Working status after roundtrip");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Metadata for a GitHub Pull Request associated with a bead.
pub struct GitHubPrMetadata {
    pub pr_url: String,
    pub pr_number: u64,
    pub head_branch: String,
    pub base_branch: String,
    pub bead_id: String,
    pub last_updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Dashboard snapshot for multi-agent queue-lock visibility.
pub struct DashboardSnapshot {
    pub generated_at: String,
    pub active_workers: Vec<String>,
    pub queue_depth: usize,
    pub stale_count: usize,
    pub conflict_count: usize,
    pub warning_count: usize,
}
