#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::TokioCommandExecutor;
use crate::lifecycle::workflow::{run_lifecycle_with_progress, LifecycleRunRequest};
use restate_sdk::prelude::*;

use super::runtime::{
    apply_progress_update, default_step_snapshots, initialize_lifecycle_status,
    seed_runtime_status, store_lifecycle_state, update_runtime_progress,
};
use super::status::serialize_workflow_outcome;
use crate::restate_oya::types::{
    LifecycleRequest, LifecycleStatusSnapshot, LifecycleStepSnapshot, StartResponse,
};

pub struct OyaBridge;

impl super::Oya for OyaBridge {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: Json<LifecycleRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let body = req.into_inner();
        let workflow_key = ctx.key().to_owned();
        let initial_steps = default_step_snapshots();
        let requested_bead_id = body.bead_id.clone();
        initialize_lifecycle_status(&ctx, requested_bead_id.clone(), &initial_steps);
        seed_runtime_status(&workflow_key, requested_bead_id, &initial_steps);
        let mut live_steps: Vec<LifecycleStepSnapshot> = Vec::new();
        let result = run_lifecycle_with_progress(
            &TokioCommandExecutor,
            LifecycleRunRequest { bead_id: body.bead_id, model: body.model, repo: body.repo },
            |update| {
                let update_clone = update.clone();
                apply_progress_update(&ctx, &mut live_steps, update);
                update_runtime_progress(&workflow_key, &live_steps, update_clone);
            },
        )
        .await;
        match result {
            Ok(outcome) => {
                store_lifecycle_state(&ctx, &outcome.state)?;
                serialize_workflow_outcome(&outcome).map(Into::into)
            }
            Err(failure) => {
                if let Some(state) = &failure.state {
                    store_lifecycle_state(&ctx, state)?;
                }
                let message = serde_json::to_string(&failure).map_err(|error| {
                    HandlerError::from(format!("failed to serialize lifecycle failure: {error}"))
                })?;
                Err(TerminalError::new(message).into())
            }
        }
    }

    async fn status(
        &self,
        ctx: SharedWorkflowContext<'_>,
    ) -> Result<Json<LifecycleStatusSnapshot>, HandlerError> {
        super::status::read_lifecycle_status(&ctx).await.map(Into::into)
    }
}
