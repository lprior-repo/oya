use super::types::{
    AgentId, AgentStatus, ApproverMode, ArtifactType, BeadId, DomainError, FailureCategory, RunId,
    StageName,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

// =============================================================================
//  Run Aggregate
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
                next.state = RunState::Running { current_stage: StageName::Contract }; // Initial stage
                next.updated_at = Utc::now();
                Ok(next)
            }
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
                let mut next = self.clone();
                // Transition logic: determine next stage or finish
                let next_stage = stage.next();

                if let Some(ns) = next_stage {
                    next.state = RunState::Running { current_stage: ns };
                } else {
                    next.state = RunState::Shipped { completed_at: Utc::now() };
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
        next.state = RunState::Failed { reason, failed_at: Utc::now() };
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateResult {
    pub run_id: String,
    pub gate_name: String,
    pub passed: bool,
    pub exit_code: i32,
    pub log_ref: Option<String>,
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
