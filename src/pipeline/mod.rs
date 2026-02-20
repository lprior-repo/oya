mod executor;
mod outputs;
mod state;
mod timeline;

pub(super) use executor::{execute_and_accumulate_stage, persist_stage_artifact, StageExecutionInput};
pub(super) use outputs::{record_stage_outputs, RecordStageOutputsInput};
pub(super) use state::{
    deterministic_env_bool, deterministic_timestamp, execute_stage_with_tracker,
    init_pipeline_state, mark_stage_running, parse_rfc3339_deterministic, pipeline_input,
    prepare_stage_attempt, PipelineRunInput, PipelineState, StageExecutionResult,
};
pub(super) use timeline::handle_stage_transition;

use std::path::PathBuf;

use oya::types::Gate;
use restate_sdk::prelude::*;

use super::OyaError;

/// Runtime configuration read deterministically from environment at workflow start.
pub(super) struct RuntimeConfig {
    pub(super) workspace_policy: WorkspacePreparationPolicy,
    pub(super) merge_queue_policy: MergeQueuePolicy,
    pub(super) repo_root: PathBuf,
}

#[derive(Clone, Copy)]
pub enum WorkspacePreparationPolicy {
    Prepare,
    Skip,
}

impl WorkspacePreparationPolicy {
    pub fn from_skip_flag(skip: bool) -> Self {
        if skip {
            Self::Skip
        } else {
            Self::Prepare
        }
    }

    pub fn should_skip(self) -> bool {
        matches!(self, Self::Skip)
    }
}

#[derive(Clone, Copy)]
pub(super) enum MergeQueuePolicy {
    Enforce,
    Skip,
}

impl MergeQueuePolicy {
    pub(super) fn from_skip_flag(skip: bool) -> Self {
        if skip {
            Self::Skip
        } else {
            Self::Enforce
        }
    }

    pub(super) fn should_run(self, gate: &Gate) -> bool {
        !(matches!(self, Self::Skip) && *gate == Gate::ZjjMergeQueue)
    }
}

impl RuntimeConfig {
    /// Read all configuration deterministically from environment.
    pub(super) async fn load(ctx: &WorkflowContext<'_>) -> Result<Self, OyaError> {
        let skip_zjj_workspace = deterministic_env_bool(ctx, "OYA_SKIP_ZJJ_WORKSPACE")
            .await
            .map_err(|_e| OyaError("config error: OYA_SKIP_ZJJ_WORKSPACE".to_string()))?;

        let skip_zjj_gate = deterministic_env_bool(ctx, "OYA_SKIP_ZJJ_GATE")
            .await
            .map_err(|_e| OyaError("config error: OYA_SKIP_ZJJ_GATE".to_string()))?;

        let repo_root_str = Self::deterministic_repo_root(ctx)
            .await
            .map_err(|e| OyaError(format!("config error: repo_root: {}", e)))?;

        Ok(Self {
            workspace_policy: WorkspacePreparationPolicy::from_skip_flag(skip_zjj_workspace),
            merge_queue_policy: MergeQueuePolicy::from_skip_flag(skip_zjj_gate),
            repo_root: PathBuf::from(repo_root_str),
        })
    }

    async fn deterministic_repo_root(ctx: &WorkflowContext<'_>) -> Result<String, TerminalError> {
        ctx.run(|| async move {
            if let Ok(configured_root) = std::env::var("OYA_REPO_ROOT") {
                return Ok::<_, HandlerError>(configured_root);
            }
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .map_err(|e| HandlerError::from(format!("Failed to resolve repo root: {}", e)))
        })
        .await
    }
}
