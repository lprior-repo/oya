#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::{CompensationDiagnostic, LifecycleError, LifecycleState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunRequest {
    pub bead_id: Option<String>,
    pub model: Option<String>,
    pub repo: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunOutcome {
    pub state: LifecycleState,
    pub journal: Vec<crate::lifecycle::effects::EffectJournalEntry>,
    pub compensation_journal: Vec<crate::lifecycle::effects::EffectJournalEntry>,
    pub compensation_diagnostics: Vec<CompensationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunFailure {
    pub error: LifecycleError,
    pub state: Option<LifecycleState>,
    pub journal: Vec<crate::lifecycle::effects::EffectJournalEntry>,
    pub compensation_journal: Vec<crate::lifecycle::effects::EffectJournalEntry>,
    pub compensation_diagnostics: Vec<CompensationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleStepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleProgressUpdate {
    Initialized {
        bead_id: String,
        steps: Vec<String>,
    },
    Step {
        step: String,
        status: LifecycleStepStatus,
        message: Option<String>,
        details: Option<serde_json::Value>,
        started_at: Option<String>,
        finished_at: Option<String>,
        duration_ms: Option<u64>,
    },
    Finished {
        success: bool,
        pr_url: Option<String>,
        message: Option<String>,
        compensation_diagnostics: Vec<CompensationDiagnostic>,
    },
}

#[derive(Debug, Clone)]
pub struct ExecutionAcc {
    pub state: LifecycleState,
    pub journal: Vec<crate::lifecycle::effects::EffectJournalEntry>,
    pub completed_compensations: Vec<crate::lifecycle::effects::Compensation>,
}

#[derive(Debug, Clone)]
pub struct StepFailure {
    pub state: LifecycleState,
    pub journal: Vec<crate::lifecycle::effects::EffectJournalEntry>,
    pub completed_compensations: Vec<crate::lifecycle::effects::Compensation>,
    pub error: LifecycleError,
}
