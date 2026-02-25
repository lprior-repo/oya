mod executor;
mod state;

pub(super) use executor::{
    execute_and_accumulate_stage, persist_stage_artifact, StageExecutionInput,
};
pub(super) use state::{
    init_pipeline_state, parse_rfc3339_stable, pipeline_input, stable_env_var, workflow_timestamp,
    workflow_timestamp_or_error, PipelineRunInput, PipelineState,
};

use std::path::PathBuf;

use oya::types::Gate;
use restate_sdk::prelude::*;

use super::OyaError;

/// Runtime configuration read reliably from environment at workflow start.
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
        let _ = self;
        *gate != Gate::ZjjMergeQueue
    }
}

impl RuntimeConfig {
    /// Read all configuration reliably from environment.
    pub(super) async fn load(ctx: &WorkflowContext<'_>) -> Result<Self, OyaError> {
        let disable_zjj = Self::read_flag(ctx, "OYA_DISABLE_ZJJ").await?;
        let (skip_zjj_workspace, skip_zjj_gate) =
            Self::read_zjj_skip_flags(ctx, disable_zjj).await?;

        let repo_root_str = Self::stable_repo_root(ctx).await.map_err(|error| {
            OyaError(format!(
                "config error resolving repo root (OYA_REPO_ROOT or current_dir): {}",
                error
            ))
        })?;

        Ok(Self {
            workspace_policy: WorkspacePreparationPolicy::from_skip_flag(skip_zjj_workspace),
            merge_queue_policy: MergeQueuePolicy::from_skip_flag(skip_zjj_gate),
            repo_root: PathBuf::from(repo_root_str),
        })
    }

    async fn read_flag(ctx: &WorkflowContext<'_>, key: &str) -> Result<bool, OyaError> {
        Self::stable_env_bool_default_true(ctx, key).await.map_err(|error| {
            OyaError(format!(
                "config error reading {} (expected 1/true/0/false or unset): {}",
                key, error
            ))
        })
    }

    async fn read_zjj_skip_flags(
        ctx: &WorkflowContext<'_>,
        disable_zjj: bool,
    ) -> Result<(bool, bool), OyaError> {
        if disable_zjj {
            return Ok((true, true));
        }
        let workspace = Self::stable_env_bool_default_true(ctx, "OYA_SKIP_ZJJ_WORKSPACE")
            .await
            .map_err(|error| {
                OyaError(format!(
                    "config error reading OYA_SKIP_ZJJ_WORKSPACE (expected 1/true/0/false or unset): {}",
                    error
                ))
            })?;
        let gate = Self::stable_env_bool_default_true(ctx, "OYA_SKIP_ZJJ_GATE").await.map_err(
            |error| {
                OyaError(format!(
                    "config error reading OYA_SKIP_ZJJ_GATE (expected 1/true/0/false or unset): {}",
                    error
                ))
            },
        )?;
        Ok((workspace, gate))
    }

    async fn stable_env_bool_default_true(
        ctx: &WorkflowContext<'_>,
        key: &str,
    ) -> Result<bool, TerminalError> {
        let value = stable_env_var(ctx, key).await?;
        Ok(value.is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true")))
    }

    async fn stable_repo_root(ctx: &WorkflowContext<'_>) -> Result<String, TerminalError> {
        ctx.run(|| async move {
            if let Ok(configured_root) = std::env::var("OYA_REPO_ROOT") {
                if configured_root.trim().is_empty() {
                    return Err::<String, HandlerError>(HandlerError::from(
                        "OYA_REPO_ROOT is set but empty".to_string(),
                    ));
                }
                return Ok::<_, HandlerError>(configured_root);
            }
            std::env::current_dir().map(|p| p.to_string_lossy().to_string()).map_err(|error| {
                HandlerError::from(format!(
                    "failed to resolve repo root from current_dir: {}",
                    error
                ))
            })
        })
        .await
    }
}
