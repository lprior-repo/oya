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

use restate_sdk::prelude::*;

use super::OyaError;

/// Runtime configuration read reliably from environment at workflow start.
pub(super) struct RuntimeConfig {
    pub(super) repo_root: PathBuf,
}

impl RuntimeConfig {
    /// Read all configuration reliably from environment.
    pub(super) async fn load(ctx: &WorkflowContext<'_>) -> Result<Self, OyaError> {
        let repo_root_str = Self::stable_repo_root(ctx).await.map_err(|error| {
            OyaError(format!(
                "config error resolving repo root (OYA_REPO_ROOT or current_dir): {}",
                error
            ))
        })?;

        Ok(Self { repo_root: PathBuf::from(repo_root_str) })
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
