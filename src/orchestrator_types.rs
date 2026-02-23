#![allow(dead_code)]

use super::OyaError;
use oya::types::{MergeDecision, ModelId, QueuePosition, TimelineEntry};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Conflict Cascade Types - immutable audit records for merge conflicts
// ---------------------------------------------------------------------------

/// Strategy used to resolve a merge conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    /// Manual resolution required.
    Manual,
    /// Accept incoming changes.
    AcceptIncoming,
    /// Accept current (working tree) changes.
    AcceptCurrent,
    /// Accept both sets of changes.
    AcceptBoth,
    /// Rebase onto parent.
    Rebase,
    /// Skip this merge.
    Skip,
}

impl ConflictResolutionStrategy {
    /// Check if this strategy is compatible with automatic propagation.
    pub fn is_propagatable(&self) -> bool {
        matches!(self, Self::AcceptIncoming | Self::AcceptCurrent | Self::AcceptBoth | Self::Rebase)
    }

    /// Check if this strategy requires manual review.
    pub fn requires_manual_review(&self) -> bool {
        matches!(self, Self::Manual)
    }
}

/// Identity of the resolver (who resolved the conflict).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolverIdentity {
    /// Unique identifier for the resolver (e.g., agent ID, user ID).
    pub resolver_id: String,
    /// Type of resolver (e.g., "agent", "human", "system").
    pub resolver_type: String,
}

impl ResolverIdentity {
    pub fn new(resolver_id: String, resolver_type: String) -> Self {
        Self { resolver_id, resolver_type }
    }

    pub fn agent(agent_id: &str) -> Self {
        Self::new(agent_id.to_string(), "agent".to_string())
    }

    pub fn human(user_id: &str) -> Self {
        Self::new(user_id.to_string(), "human".to_string())
    }

    pub fn system(system_id: &str) -> Self {
        Self::new(system_id.to_string(), "system".to_string())
    }
}

/// Immutable audit record for a conflict decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDecision {
    /// Unique identifier for this conflict decision record.
    pub decision_id: String,
    /// Bead ID that had the conflict.
    pub bead_id: String,
    /// Parent bead ID (if this is a child in the stack).
    pub parent_bead_id: Option<String>,
    /// Child bead IDs that depend on this bead.
    pub child_bead_ids: Vec<String>,
    /// Strategy used to resolve the conflict.
    pub strategy: ConflictResolutionStrategy,
    /// Identity of the resolver.
    pub resolver: ResolverIdentity,
    /// Timestamp when the decision was made (RFC3339).
    pub resolved_at: String,
    /// Whether this decision was propagated from a parent conflict.
    pub is_propagated: bool,
    /// If true, propagation was blocked due to patch intent violation.
    pub propagation_blocked: bool,
    /// Reason for blocking propagation (if applicable).
    pub propagation_block_reason: Option<String>,
}

/// Arguments for creating a new ConflictDecision.
pub struct ConflictDecisionArgs {
    pub decision_id: String,
    pub bead_id: String,
    pub parent_bead_id: Option<String>,
    pub child_bead_ids: Vec<String>,
    pub strategy: ConflictResolutionStrategy,
    pub resolver: ResolverIdentity,
    pub resolved_at: String,
}

impl ConflictDecision {
    pub fn new(args: ConflictDecisionArgs) -> Self {
        Self {
            decision_id: args.decision_id,
            bead_id: args.bead_id,
            parent_bead_id: args.parent_bead_id,
            child_bead_ids: args.child_bead_ids,
            strategy: args.strategy,
            resolver: args.resolver,
            resolved_at: args.resolved_at,
            is_propagated: false,
            propagation_blocked: false,
            propagation_block_reason: None,
        }
    }

    /// Create a propagated decision (from parent resolution).
    pub fn propagated(
        decision_id: String,
        bead_id: String,
        parent_bead_id: String,
        strategy: ConflictResolutionStrategy,
        resolver: ResolverIdentity,
    ) -> Self {
        Self {
            decision_id,
            bead_id,
            parent_bead_id: Some(parent_bead_id),
            child_bead_ids: Vec::new(),
            strategy,
            resolver,
            resolved_at: String::new(),
            is_propagated: true,
            propagation_blocked: false,
            propagation_block_reason: None,
        }
    }

    /// Set the resolved_at timestamp.
    pub fn with_resolved_at(mut self, resolved_at: String) -> Self {
        self.resolved_at = resolved_at;
        self
    }

    /// Create a decision marked as blocked from propagation.
    pub fn with_propagation_blocked(mut self, reason: String) -> Self {
        self.propagation_blocked = true;
        self.propagation_block_reason = Some(reason);
        self
    }
}

/// Immutable audit log of all conflict decisions (append-only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConflictLog {
    /// Sequential list of all conflict decisions.
    pub decisions: Vec<ConflictDecision>,
    /// Sequence number for ordering.
    pub sequence: u64,
}

impl ConflictLog {
    pub fn new() -> Self {
        Self { decisions: Vec::new(), sequence: 0 }
    }

    /// Append a new decision to the log (immutable append).
    pub fn append(&mut self, decision: ConflictDecision) {
        self.sequence = self.sequence.saturating_add(1);
        self.decisions.push(decision);
    }

    /// Get the latest decision for a specific bead.
    pub fn latest_for(&self, bead_id: &str) -> Option<&ConflictDecision> {
        self.decisions.iter().rev().find(|d| d.bead_id == bead_id)
    }

    /// Get all decisions for beads that depend on the given parent.
    pub fn children_of(&self, parent_bead_id: &str) -> Vec<&ConflictDecision> {
        self.decisions
            .iter()
            .filter(|d| d.parent_bead_id.as_deref() == Some(parent_bead_id))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Conflict Cascade Evaluation - pure functions
// ---------------------------------------------------------------------------

/// Input for evaluating conflict propagation.
#[derive(Debug, Clone)]
pub struct ConflictPropagationInput<'a> {
    /// The parent's resolved conflict decision.
    pub parent_decision: &'a ConflictDecision,
    /// Child bead IDs to evaluate.
    pub child_bead_ids: &'a [String],
    /// Child patch intents (what changes the child makes).
    pub child_patch_intents: &'a ChildPatchIntents,
}

/// Child patch intent - describes what changes a child makes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChildPatchIntents {
    /// Map of bead_id to its patch intent.
    pub intents: std::collections::HashMap<String, PatchIntent>,
}

impl ChildPatchIntents {
    pub fn new() -> Self {
        Self { intents: std::collections::HashMap::new() }
    }

    pub fn with_intent(mut self, bead_id: String, intent: PatchIntent) -> Self {
        self.intents.insert(bead_id, intent);
        self
    }

    pub fn get(&self, bead_id: &str) -> Option<&PatchIntent> {
        self.intents.get(bead_id)
    }
}

/// Describes the intent of a child's patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchIntent {
    /// Child modifies the same files as parent.
    OverlappingFiles(Vec<String>),
    /// Child modifies different files than parent.
    NonOverlappingFiles(Vec<String>),
    /// Child intent is unknown (needs manual review).
    Unknown,
}

impl PatchIntent {
    /// Check if this intent would be violated by the parent's strategy.
    pub fn would_be_violated_by(&self, strategy: &ConflictResolutionStrategy) -> bool {
        match (self, strategy) {
            // OverlappingFiles: AcceptCurrent may violate child's changes
            (PatchIntent::OverlappingFiles(_), ConflictResolutionStrategy::AcceptCurrent) => true,
            // OverlappingFiles: AcceptIncoming is safe (parent changes take precedence)
            (PatchIntent::OverlappingFiles(_), ConflictResolutionStrategy::AcceptIncoming) => false,
            // OverlappingFiles: Manual/Skip/AcceptBoth/Rebase need case-by-case review
            (PatchIntent::OverlappingFiles(_), _) => true,
            // NonOverlappingFiles: never violated by any strategy
            (PatchIntent::NonOverlappingFiles(_), _) => false,
            (PatchIntent::Unknown, _) => true,
        }
    }
}

/// Result of conflict propagation evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictPropagationResult {
    /// Children can receive the propagated strategy.
    Propagateable(Vec<ConflictPropagationItem>),
    /// Some or all children need manual review.
    NeedsManualReview(Vec<ConflictPropagationItem>),
}

/// Individual child's propagation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictPropagationItem {
    pub bead_id: String,
    pub strategy: ConflictResolutionStrategy,
    pub can_propagate: bool,
    pub block_reason: Option<String>,
}

/// Evaluate whether conflict resolution can propagate to children (pure function).
pub fn evaluate_conflict_propagation(
    input: &ConflictPropagationInput<'_>,
) -> ConflictPropagationResult {
    let mut items = Vec::new();
    let mut needs_review = false;

    // If parent strategy is not propagatable, all children need manual review
    if !input.parent_decision.strategy.is_propagatable() {
        for child_id in input.child_bead_ids {
            items.push(ConflictPropagationItem {
                bead_id: child_id.clone(),
                strategy: ConflictResolutionStrategy::Manual,
                can_propagate: false,
                block_reason: Some("parent strategy not propagatable".to_string()),
            });
        }
        return ConflictPropagationResult::NeedsManualReview(items);
    }

    for child_id in input.child_bead_ids {
        let intent = input.child_patch_intents.get(child_id);
        let can_propagate = intent
            .map(|i| !i.would_be_violated_by(&input.parent_decision.strategy))
            .unwrap_or(true); // If no intent known, allow propagation

        if !can_propagate {
            needs_review = true;
        }

        items.push(ConflictPropagationItem {
            bead_id: child_id.clone(),
            strategy: input.parent_decision.strategy,
            can_propagate,
            block_reason: if can_propagate {
                None
            } else {
                Some("child patch intent would be violated".to_string())
            },
        });
    }

    if needs_review {
        ConflictPropagationResult::NeedsManualReview(items)
    } else {
        ConflictPropagationResult::Propagateable(items)
    }
}

/// Check if a child depends on a conflicted parent (pure function).
pub fn is_child_of_conflicted(
    child_bead_id: &str,
    child_depends_on: &[String],
    conflicted_beads: &std::collections::BTreeSet<String>,
) -> bool {
    conflicted_beads.contains(child_bead_id)
        || child_depends_on.iter().any(|dep| conflicted_beads.contains(dep))
}

// ---------------------------------------------------------------------------
// Conflict Log Actions - shell boundary operations
// ---------------------------------------------------------------------------

/// Append a new conflict decision to the workflow state.
pub async fn append_conflict_decision(
    ctx: &WorkflowContext<'_>,
    decision: ConflictDecision,
) -> Result<(), OyaError> {
    let existing = ctx
        .get::<String>("conflict_log")
        .await
        .map_err(|error| OyaError(format!("conflict_log read failed: {}", error)))?
        .unwrap_or_default();

    let mut log: ConflictLog = if existing.is_empty() {
        ConflictLog::new()
    } else {
        serde_json::from_str(&existing)
            .map_err(|error| OyaError(format!("conflict_log parse failed: {}", error)))?
    };

    log.append(decision);

    let encoded = serde_json::to_string(&log)
        .map_err(|error| OyaError(format!("conflict_log encode failed: {}", error)))?;
    ctx.set("conflict_log", encoded);

    Ok(())
}

/// Get the conflict log from workflow state.
pub async fn get_conflict_log(ctx: &WorkflowContext<'_>) -> Result<ConflictLog, OyaError> {
    let existing = ctx
        .get::<String>("conflict_log")
        .await
        .map_err(|error| OyaError(format!("conflict_log read failed: {}", error)))?;

    match existing {
        Some(raw) => serde_json::from_str(&raw)
            .map_err(|error| OyaError(format!("conflict_log parse failed: {}", error))),
        None => Ok(ConflictLog::new()),
    }
}

/// Input parameters for conflict propagation.
pub struct ConflictPropagationArgs<'a> {
    pub parent_bead_id: &'a str,
    pub child_bead_ids: &'a [String],
    pub child_patch_intents: &'a ChildPatchIntents,
    pub resolver: ResolverIdentity,
    pub resolved_at: &'a str,
}

/// Propagate conflict resolution to eligible children.
pub async fn propagate_conflict_resolution(
    ctx: &WorkflowContext<'_>,
    args: ConflictPropagationArgs<'_>,
) -> Result<Vec<ConflictDecision>, OyaError> {
    let log = get_conflict_log(ctx).await?;
    let parent_decision = log.latest_for(args.parent_bead_id).ok_or_else(|| {
        OyaError(format!("no conflict decision found for {}", args.parent_bead_id))
    })?;
    let input = ConflictPropagationInput {
        parent_decision,
        child_bead_ids: args.child_bead_ids,
        child_patch_intents: args.child_patch_intents,
    };
    let result = evaluate_conflict_propagation(&input);
    apply_propagation_result(ctx, args.parent_bead_id, result, &args.resolver, args.resolved_at)
        .await
}

async fn apply_propagation_result(
    ctx: &WorkflowContext<'_>,
    parent_bead_id: &str,
    result: ConflictPropagationResult,
    resolver: &ResolverIdentity,
    resolved_at: &str,
) -> Result<Vec<ConflictDecision>, OyaError> {
    let mut decisions = Vec::new();
    match result {
        ConflictPropagationResult::Propagateable(items) => {
            for item in items {
                if item.can_propagate {
                    let d = make_propagated_decision(
                        parent_bead_id,
                        &item,
                        resolver,
                        resolved_at,
                        None,
                    );
                    append_conflict_decision(ctx, d.clone()).await?;
                    decisions.push(d);
                }
            }
        }
        ConflictPropagationResult::NeedsManualReview(items) => {
            for item in items {
                let reason = item
                    .block_reason
                    .clone()
                    .unwrap_or_else(|| "manual review required".to_string());
                let d = make_propagated_decision(
                    parent_bead_id,
                    &item,
                    resolver,
                    resolved_at,
                    Some(reason),
                );
                append_conflict_decision(ctx, d.clone()).await?;
                decisions.push(d);
            }
        }
    }
    Ok(decisions)
}

/// Create a propagated or review-blocked ConflictDecision.
/// If `block_reason` is Some, the decision is marked as propagation-blocked.
fn make_propagated_decision(
    parent_bead_id: &str,
    item: &ConflictPropagationItem,
    resolver: &ResolverIdentity,
    resolved_at: &str,
    block_reason: Option<String>,
) -> ConflictDecision {
    let suffix = if block_reason.is_some() { "review" } else { "propagated" };
    let decision_id = format!("{}_{suffix}_{}", parent_bead_id, item.bead_id);
    let d = ConflictDecision::propagated(
        decision_id,
        item.bead_id.clone(),
        parent_bead_id.to_string(),
        item.strategy,
        resolver.clone(),
    )
    .with_resolved_at(resolved_at.to_string());
    match block_reason {
        Some(reason) => d.with_propagation_blocked(reason),
        None => d,
    }
}

// ---------------------------------------------------------------------------
// Public request/response types for OyaOpsMonitor handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
/// Request body for polling OpenCode event stream snapshots.
pub struct OpsMonitorEventRequest {
    /// Maximum number of events to return in one poll.
    pub max_events: Option<usize>,
    /// Long-poll timeout in seconds for the event endpoint.
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
/// Aggregated OpenCode status counters at one observation timestamp.
pub struct OpsMonitorPollResponse {
    pub source: String,
    pub observed_at: String,
    pub busy_sessions: Vec<String>,
    pub pending_permissions: usize,
    pub pending_questions: usize,
}

#[derive(Debug, Serialize)]
/// One raw OpenCode SSE event plus optional parsed JSON payload.
pub struct OpsMonitorEventEnvelope {
    pub raw: String,
    pub parsed: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
/// Event polling response with bounded event payloads and timing metadata.
pub struct OpsMonitorEventResponse {
    pub source: String,
    pub observed_at: String,
    pub events: Vec<OpsMonitorEventEnvelope>,
    pub count: usize,
    pub timeout_seconds: u64,
}

// ---------------------------------------------------------------------------
// Internal orchestrator event types persisted into Restate state
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct WorkspaceLifecycleEvent {
    pub workspace: String,
    pub workspace_path: String,
    pub queue_command: String,
    pub queue_passed: bool,
    pub queue_exit_code: i32,
    pub queue_output: String,
    pub add_command: String,
    pub add_passed: bool,
    pub add_exit_code: i32,
    pub add_output: String,
    pub coordination: WorkspaceCoordination,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkspaceCoordination {
    pub queue_position: QueuePosition,
    pub merge_decision: MergeDecision,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct OrchestratorState {
    pub status: String,
    pub stage: String,
    pub attempt: u32,
    pub bead_id: String,
    pub context: String,
    pub model: ModelId,
    pub last_failure: String,
    pub last_output: String,
    pub last_prompt: String,
    pub updated_at: String,
}

/// Consolidated stage artifact containing all data for one stage attempt.
/// Replaces 8+ individual keys with a single rich payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageArtifact {
    pub stage: String,
    pub attempt: u32,
    pub failure_category: Option<String>,
    pub next_stage: Option<String>,
    pub timing: StageTiming,
    pub workspace: Option<WorkspaceLifecycle>,
    pub input: StageInputData,
    pub prompt: String,
    pub output: StageOutputData,
    pub task_tracking: Option<TaskTracking>,
    pub gates: Vec<GateResultData>,
    pub status: StageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageTiming {
    pub started_at: String,
    pub completed_at: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkspaceLifecycle {
    pub name: String,
    pub path: String,
    pub queue_command: String,
    pub queue_passed: bool,
    pub queue_exit_code: i32,
    pub add_command: String,
    pub add_passed: bool,
    pub add_exit_code: i32,
    pub coordination: WorkspaceCoordination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageInputData {
    pub run_id: String,
    pub bead_id: String,
    pub context: String,
    pub model: ModelId,
    pub last_failure: Option<FailureSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageOutputData {
    pub success: bool,
    pub exit_code: i32,
    pub full_log: String,
    pub feedback: String,
    pub contract_document: Option<String>,
    pub implementation_code: Option<String>,
    pub test_results: Option<String>,
    pub adversarial_report: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TaskTracking {
    pub tasks_created: Vec<String>,
    pub tasks_updated: Vec<String>,
    pub tasks_completed: Vec<String>,
    pub task_states: std::collections::HashMap<String, TaskState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TaskState {
    pub subject: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GateResultData {
    pub gate: String,
    pub passed: bool,
    pub exit_code: i32,
    pub command: String,
    pub output: String,
}

/// Stage status - mutually exclusive states making illegal states unrepresentable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum StageStatus {
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct RunRequestEvent {
    pub run_id: String,
    pub bead_id: String,
    pub context: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct FailureSnapshot {
    pub category: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ChangeIdentity {
    pub logical_change_id: String,
    pub vcs_change_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct DurableEvent {
    pub event_type: String,
    pub run_id: String,
    pub bead_id: String,
    pub stage: String,
    pub attempt: u32,
    pub status: String,
    pub reason: String,
    pub at: String,
    pub identity: ChangeIdentity,
}

// ---------------------------------------------------------------------------
// Start-request parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(super) struct StartRequestPayload {
    pub bead_id: Option<String>,
    pub context: Option<String>,
    pub model: Option<ModelId>,
}

pub(super) fn parse_start_request(
    request: serde_json::Value,
) -> Result<StartRequestPayload, TerminalError> {
    match request {
        serde_json::Value::Object(_) => serde_json::from_value(request)
            .map_err(|e| TerminalError::new_with_code(400, format!("Invalid JSON body: {}", e))),
        serde_json::Value::String(raw) => serde_json::from_str::<StartRequestPayload>(&raw)
            .map_err(|e| {
                TerminalError::new_with_code(400, format!("Invalid JSON string body: {}", e))
            }),
        other => Err(TerminalError::new_with_code(
            400,
            format!("Invalid request payload type: expected object or JSON string, got {}", other),
        )),
    }
}

// ---------------------------------------------------------------------------
// Restate state helpers
// ---------------------------------------------------------------------------

pub(super) fn to_json_string<T: Serialize>(value: &T) -> Result<String, OyaError> {
    serde_json::to_string(value).map_err(|error| OyaError(format!("json encode failed: {}", error)))
}

pub(super) fn set_json_state<T: Serialize>(
    ctx: &WorkflowContext<'_>,
    key: &str,
    value: &T,
) -> Result<(), OyaError> {
    let encoded = to_json_string(value)?;
    ctx.set(key, encoded);
    Ok(())
}

pub(super) fn write_orchestrator_state(
    ctx: &WorkflowContext<'_>,
    state: &OrchestratorState,
) -> Result<(), OyaError> {
    set_json_state(ctx, "state", state)
}

pub(super) async fn append_timeline(
    ctx: &WorkflowContext<'_>,
    entry: TimelineEntry,
) -> Result<(), OyaError> {
    let existing = ctx
        .get::<String>("timeline")
        .await
        .map_err(|error| OyaError(format!("timeline read failed: {}", error)))?;
    let existing = existing.unwrap_or_default();

    let event_seq = ctx
        .get::<u32>("event_seq")
        .await
        .map_err(|error| OyaError(format!("event_seq read failed: {}", error)))?
        .map_or(1, |value| value + 1);
    ctx.set("event_seq", event_seq);

    let event_key = format!("event_{:04}", event_seq);
    set_json_state(ctx, &event_key, &entry)?;

    let line = to_json_string(&entry)?;
    let next = if existing.is_empty() { line } else { format!("{}\n{}", existing, line) };

    ctx.set("timeline", next);
    Ok(())
}

/// Persist a single consolidated stage artifact.
pub(super) fn set_stage_artifact(
    ctx: &WorkflowContext<'_>,
    key: &str,
    artifact: &StageArtifact,
) -> Result<(), OyaError> {
    set_json_state(ctx, key, artifact)
}

/// Set lean timeline as a single JSON array instead of incremental appends.
pub(super) fn set_timeline_once(ctx: &WorkflowContext<'_>, timeline: &str) -> Result<(), OyaError> {
    ctx.set("timeline", timeline.to_string());
    Ok(())
}

pub(super) fn resolve_change_identity(
    run_id: &str,
    bead_id: &str,
    workspace_name: Option<&str>,
) -> ChangeIdentity {
    let vcs_change_id = workspace_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("jj:{}", value))
        .unwrap_or_else(|| format!("git:{}", run_id));
    ChangeIdentity { logical_change_id: format!("{}:{}", bead_id, run_id), vcs_change_id }
}

pub(super) async fn append_durable_event(
    ctx: &WorkflowContext<'_>,
    event: DurableEvent,
) -> Result<(), OyaError> {
    enforce_durable_event_sequence(ctx, &event).await?;
    let existing = ctx
        .get::<String>("event_ledger")
        .await
        .map_err(|error| OyaError(format!("event_ledger read failed: {}", error)))?
        .unwrap_or_default();
    let line = to_json_string(&event)?;
    let next = if existing.is_empty() { line } else { format!("{}\n{}", existing, line) };
    ctx.set("event_ledger", next);
    ctx.set("event_ledger_last_at", event.at.clone());
    Ok(())
}

async fn enforce_durable_event_sequence(
    ctx: &WorkflowContext<'_>,
    event: &DurableEvent,
) -> Result<(), OyaError> {
    let last_seen = ctx
        .get::<String>("event_ledger_last_at")
        .await
        .map_err(|error| OyaError(format!("event_ledger_last_at read failed: {}", error)))?;
    validate_event_timestamp_sequence(last_seen.as_deref(), event.at.as_str())
}

fn validate_event_timestamp_sequence(
    last_seen: Option<&str>,
    next_at: &str,
) -> Result<(), OyaError> {
    let Some(last_seen) = last_seen else {
        return Ok(());
    };
    let last = chrono::DateTime::parse_from_rfc3339(last_seen).map_err(|error| {
        OyaError(format!("invalid stored event timestamp '{}': {}", last_seen, error))
    })?;
    let next = chrono::DateTime::parse_from_rfc3339(next_at)
        .map_err(|error| OyaError(format!("invalid event timestamp '{}': {}", next_at, error)))?;
    if next < last {
        return Err(OyaError(format!(
            "event sequence guard rejected out-of-order event: last_at={} next_at={}",
            last_seen, next_at
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // Conflict Cascade Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn conflict_resolution_strategy_is_propagatable() {
        assert!(ConflictResolutionStrategy::AcceptIncoming.is_propagatable());
        assert!(ConflictResolutionStrategy::AcceptCurrent.is_propagatable());
        assert!(ConflictResolutionStrategy::AcceptBoth.is_propagatable());
        assert!(ConflictResolutionStrategy::Rebase.is_propagatable());
        assert!(!ConflictResolutionStrategy::Manual.is_propagatable());
        assert!(!ConflictResolutionStrategy::Skip.is_propagatable());
    }

    #[test]
    fn conflict_resolution_strategy_requires_manual_review() {
        assert!(ConflictResolutionStrategy::Manual.requires_manual_review());
        assert!(!ConflictResolutionStrategy::AcceptIncoming.requires_manual_review());
    }

    #[test]
    fn resolver_identity_factory_methods() {
        let agent = ResolverIdentity::agent("agent-001");
        assert_eq!(agent.resolver_id, "agent-001");
        assert_eq!(agent.resolver_type, "agent");

        let human = ResolverIdentity::human("user-123");
        assert_eq!(human.resolver_type, "human");

        let system = ResolverIdentity::system("system-merge");
        assert_eq!(system.resolver_type, "system");
    }

    #[test]
    fn conflict_decision_creation() {
        let decision = ConflictDecision::new(ConflictDecisionArgs {
            decision_id: "dec-001".to_string(),
            bead_id: "src-abc".to_string(),
            parent_bead_id: Some("src-parent".to_string()),
            child_bead_ids: vec!["src-child1".to_string(), "src-child2".to_string()],
            strategy: ConflictResolutionStrategy::AcceptIncoming,
            resolver: ResolverIdentity::agent("agent-001"),
            resolved_at: "2026-02-22T10:00:00Z".to_string(),
        });

        assert_eq!(decision.decision_id, "dec-001");
        assert_eq!(decision.bead_id, "src-abc");
        assert_eq!(decision.parent_bead_id, Some("src-parent".to_string()));
        assert_eq!(decision.child_bead_ids.len(), 2);
        assert_eq!(decision.strategy, ConflictResolutionStrategy::AcceptIncoming);
        assert!(!decision.is_propagated);
        assert!(!decision.propagation_blocked);
    }

    #[test]
    fn conflict_decision_propagated_factory() {
        let decision = ConflictDecision::propagated(
            "dec-prop-001".to_string(),
            "src-child".to_string(),
            "src-parent".to_string(),
            ConflictResolutionStrategy::AcceptCurrent,
            ResolverIdentity::agent("agent-001"),
        )
        .with_resolved_at("2026-02-22T10:00:00Z".to_string());

        assert!(decision.is_propagated);
        assert_eq!(decision.parent_bead_id, Some("src-parent".to_string()));
    }

    #[test]
    fn conflict_decision_with_propagation_blocked() {
        let decision = ConflictDecision::new(ConflictDecisionArgs {
            decision_id: "dec-001".to_string(),
            bead_id: "src-abc".to_string(),
            parent_bead_id: None,
            child_bead_ids: Vec::new(),
            strategy: ConflictResolutionStrategy::Manual,
            resolver: ResolverIdentity::human("user-1"),
            resolved_at: "2026-02-22T10:00:00Z".to_string(),
        })
        .with_propagation_blocked("child patch intent violation".to_string());

        assert!(decision.propagation_blocked);
        assert_eq!(
            decision.propagation_block_reason,
            Some("child patch intent violation".to_string())
        );
    }

    #[test]
    fn conflict_log_append_and_query() {
        let mut log = ConflictLog::new();

        let decision1 = ConflictDecision::new(ConflictDecisionArgs {
            decision_id: "dec-001".to_string(),
            bead_id: "src-a".to_string(),
            parent_bead_id: None,
            child_bead_ids: vec!["src-b".to_string()],
            strategy: ConflictResolutionStrategy::AcceptIncoming,
            resolver: ResolverIdentity::agent("agent-1"),
            resolved_at: "2026-02-22T10:00:00Z".to_string(),
        });

        let decision2 = ConflictDecision::new(ConflictDecisionArgs {
            decision_id: "dec-002".to_string(),
            bead_id: "src-b".to_string(),
            parent_bead_id: Some("src-a".to_string()),
            child_bead_ids: Vec::new(),
            strategy: ConflictResolutionStrategy::AcceptCurrent,
            resolver: ResolverIdentity::human("user-1"),
            resolved_at: "2026-02-22T10:01:00Z".to_string(),
        });

        log.append(decision1);
        log.append(decision2);

        assert_eq!(log.sequence, 2);
        assert_eq!(log.decisions.len(), 2);

        // Query latest for src-a
        let latest_a = log.latest_for("src-a").unwrap();
        assert_eq!(latest_a.bead_id, "src-a");

        // Query children of src-a
        let children = log.children_of("src-a");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].bead_id, "src-b");
    }

    #[test]
    fn patch_intent_violation_detection() {
        let overlapping = PatchIntent::OverlappingFiles(vec!["src/lib.rs".to_string()]);
        let non_overlapping = PatchIntent::NonOverlappingFiles(vec!["src/main.rs".to_string()]);
        let unknown = PatchIntent::Unknown;

        // Overlapping + AcceptCurrent = violation
        assert!(overlapping.would_be_violated_by(&ConflictResolutionStrategy::AcceptCurrent));
        // Overlapping + AcceptIncoming = OK
        assert!(!overlapping.would_be_violated_by(&ConflictResolutionStrategy::AcceptIncoming));
        // Non-overlapping = always OK
        assert!(!non_overlapping.would_be_violated_by(&ConflictResolutionStrategy::AcceptCurrent));
        assert!(!non_overlapping.would_be_violated_by(&ConflictResolutionStrategy::AcceptIncoming));
        // Unknown = always needs review
        assert!(unknown.would_be_violated_by(&ConflictResolutionStrategy::AcceptIncoming));
    }

    #[test]
    fn evaluate_conflict_propagation_propagatable() {
        let parent_decision = ConflictDecision::new(ConflictDecisionArgs {
            decision_id: "dec-001".to_string(),
            bead_id: "src-parent".to_string(),
            parent_bead_id: None,
            child_bead_ids: vec!["src-child1".to_string(), "src-child2".to_string()],
            strategy: ConflictResolutionStrategy::AcceptIncoming,
            resolver: ResolverIdentity::agent("agent-1"),
            resolved_at: "2026-02-22T10:00:00Z".to_string(),
        });

        let child_patch_intents = ChildPatchIntents::new()
            .with_intent("src-child1".to_string(), PatchIntent::NonOverlappingFiles(vec![]))
            .with_intent("src-child2".to_string(), PatchIntent::NonOverlappingFiles(vec![]));

        let input = ConflictPropagationInput {
            parent_decision: &parent_decision,
            child_bead_ids: &["src-child1".to_string(), "src-child2".to_string()],
            child_patch_intents: &child_patch_intents,
        };

        let result = evaluate_conflict_propagation(&input);

        match result {
            ConflictPropagationResult::Propagateable(items) => {
                assert_eq!(items.len(), 2);
                assert!(items.iter().all(|i| i.can_propagate));
            }
            ConflictPropagationResult::NeedsManualReview(_) => {
                panic!("Expected propagatable result");
            }
        }
    }

    #[test]
    fn evaluate_conflict_propagation_needs_review() {
        let parent_decision = ConflictDecision::new(ConflictDecisionArgs {
            decision_id: "dec-001".to_string(),
            bead_id: "src-parent".to_string(),
            parent_bead_id: None,
            child_bead_ids: vec!["src-child1".to_string()],
            strategy: ConflictResolutionStrategy::AcceptCurrent,
            resolver: ResolverIdentity::agent("agent-1"),
            resolved_at: "2026-02-22T10:00:00Z".to_string(),
        });

        // Child has overlapping files - AcceptCurrent would violate intent
        let child_patch_intents = ChildPatchIntents::new()
            .with_intent("src-child1".to_string(), PatchIntent::OverlappingFiles(vec![]));

        let input = ConflictPropagationInput {
            parent_decision: &parent_decision,
            child_bead_ids: &["src-child1".to_string()],
            child_patch_intents: &child_patch_intents,
        };

        let result = evaluate_conflict_propagation(&input);

        match result {
            ConflictPropagationResult::NeedsManualReview(items) => {
                assert_eq!(items.len(), 1);
                assert!(!items[0].can_propagate);
            }
            ConflictPropagationResult::Propagateable(_) => {
                panic!("Expected needs manual review result");
            }
        }
    }

    #[test]
    fn evaluate_conflict_propagation_non_propagatable_strategy() {
        let parent_decision = ConflictDecision::new(ConflictDecisionArgs {
            decision_id: "dec-001".to_string(),
            bead_id: "src-parent".to_string(),
            parent_bead_id: None,
            child_bead_ids: vec!["src-child1".to_string()],
            strategy: ConflictResolutionStrategy::Manual,
            resolver: ResolverIdentity::human("user-1"),
            resolved_at: "2026-02-22T10:00:00Z".to_string(),
        });

        let input = ConflictPropagationInput {
            parent_decision: &parent_decision,
            child_bead_ids: &["src-child1".to_string()],
            child_patch_intents: &ChildPatchIntents::new(),
        };

        let result = evaluate_conflict_propagation(&input);

        match result {
            ConflictPropagationResult::NeedsManualReview(items) => {
                assert_eq!(items.len(), 1);
                assert!(!items[0].can_propagate);
                assert!(items[0].block_reason.as_ref().unwrap().contains("not propagatable"));
            }
            ConflictPropagationResult::Propagateable(_) => {
                panic!("Expected needs manual review result");
            }
        }
    }

    #[test]
    fn test_is_child_of_conflicted_logic() {
        let conflicted = std::collections::BTreeSet::from(["src-parent".to_string()]);

        // Child directly in conflicted set
        assert!(super::is_child_of_conflicted("src-parent", &[], &conflicted));

        // Child depends on conflicted
        assert!(super::is_child_of_conflicted(
            "src-child",
            &["src-parent".to_string()],
            &conflicted
        ));

        // Child depends on non-conflicted
        assert!(!super::is_child_of_conflicted(
            "src-child",
            &["src-other".to_string()],
            &conflicted
        ));
    }

    #[test]
    fn child_patch_intents_builder() {
        let intents = ChildPatchIntents::new()
            .with_intent("src-a".to_string(), PatchIntent::OverlappingFiles(vec![]))
            .with_intent("src-b".to_string(), PatchIntent::NonOverlappingFiles(vec![]));

        assert!(intents.get("src-a").is_some());
        assert!(intents.get("src-b").is_some());
        assert!(intents.get("src-c").is_none());
    }

    // ---------------------------------------------------------------------------
    // Original tests
    // ---------------------------------------------------------------------------

    #[test]
    fn resolve_change_identity_prefers_jj_workspace_when_present() {
        let identity = resolve_change_identity("run-1", "src-2s0", Some("ws-1"));
        assert_eq!(identity.logical_change_id, "src-2s0:run-1");
        assert_eq!(identity.vcs_change_id, "jj:ws-1");
    }

    #[test]
    fn resolve_change_identity_falls_back_to_git_when_workspace_missing() {
        let identity = resolve_change_identity("run-1", "src-2s0", None);
        assert_eq!(identity.logical_change_id, "src-2s0:run-1");
        assert_eq!(identity.vcs_change_id, "git:run-1");
    }

    #[test]
    fn validate_event_timestamp_sequence_accepts_monotonic_order() {
        let result =
            validate_event_timestamp_sequence(Some("2026-02-22T10:00:00Z"), "2026-02-22T10:00:01Z");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_event_timestamp_sequence_rejects_out_of_order_write() {
        let result =
            validate_event_timestamp_sequence(Some("2026-02-22T10:00:01Z"), "2026-02-22T10:00:00Z");
        assert!(result.is_err());
    }

    #[test]
    fn validate_event_timestamp_sequence_rejects_invalid_next_timestamp() {
        let result = validate_event_timestamp_sequence(Some("2026-02-22T10:00:01Z"), "bad-ts");
        assert!(result.is_err());
    }
}
