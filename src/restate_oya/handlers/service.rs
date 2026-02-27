#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::restate_oya::opencode::cancel_invocation_query;
use crate::restate_oya::types::{
    BeadSnapshot, CancelResponse, KeyRequest, LifecycleStatusSnapshot, MemorySnapshot,
};
use restate_sdk::prelude::*;

use super::runtime::{cleanup_targets_for_key, forget_workspace_for_targets, get_runtime_status};
use super::status::{
    fetch_lifecycle_status_raw, parse_lifecycle_status_snapshot, read_workflow_status,
    workflow_key_for_service_key,
};
use super::OyaMemoryClient;

pub struct OyaServiceBridge;

impl super::OyaService for OyaServiceBridge {
    async fn get_state(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<MemorySnapshot>, HandlerError> {
        let key = req.into_inner().key;
        ctx.object_client::<OyaMemoryClient>(&key).get_state().call().await.map_err(Into::into)
    }

    async fn get_bead(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<BeadSnapshot>, HandlerError> {
        let key = req.into_inner().key;
        ctx.object_client::<OyaMemoryClient>(&key).get_bead().call().await.map_err(Into::into)
    }

    async fn get_lifecycle(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<LifecycleStatusSnapshot>, HandlerError> {
        let key = req.into_inner().key;
        if let Some(snapshot) = get_runtime_status(&key) {
            return Ok(snapshot.into());
        }
        let workflow_key = workflow_key_for_service_key(&key);
        if let Some(snapshot) = read_workflow_status(&ctx, &workflow_key).await {
            return Ok(snapshot.into());
        }
        let run_key = workflow_key.clone();
        let raw =
            ctx.run(move || fetch_lifecycle_status_raw(run_key)).name("get_lifecycle").await?;
        let snapshot = parse_lifecycle_status_snapshot(&raw, &key);
        Ok(snapshot.into())
    }

    async fn cancel(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<CancelResponse>, HandlerError> {
        let key = req.into_inner().key;
        let memory_result =
            ctx.object_client::<OyaMemoryClient>(&key).request_cancel().call().await;
        let workflow_query = format!("Oya/{key}/run");
        let workflow_result =
            ctx.run(move || cancel_invocation_query(workflow_query)).name("cancel_workflow").await;
        let cleanup_targets = cleanup_targets_for_key(&key);
        let cleanup_result = ctx
            .run(move || forget_workspace_for_targets(cleanup_targets))
            .name("cleanup_workspace")
            .await;
        Ok(compose_cancel_response(memory_result, workflow_result, cleanup_result).into())
    }
}

fn compose_cancel_response(
    memory_result: Result<Json<CancelResponse>, TerminalError>,
    workflow_result: Result<String, TerminalError>,
    cleanup_result: Result<String, TerminalError>,
) -> CancelResponse {
    let (memory_cancelled, memory_message) = memory_outcome(memory_result);
    let (workflow_cancelled, workflow_message) = workflow_outcome(workflow_result);
    let cleanup_message = cleanup_outcome(cleanup_result);
    CancelResponse {
        cancelled: memory_cancelled || workflow_cancelled,
        message: format!("{}; {}; {}", memory_message, workflow_message, cleanup_message),
    }
}

fn memory_outcome(result: Result<Json<CancelResponse>, TerminalError>) -> (bool, String) {
    match result {
        Ok(memory) => {
            let memory = memory.into_inner();
            (memory.cancelled, memory.message)
        }
        Err(error) => (false, format!("memory cancel error: {:?}", error)),
    }
}

fn workflow_outcome(result: Result<String, TerminalError>) -> (bool, String) {
    match result {
        Ok(message) => (message.starts_with("cancelled"), message),
        Err(error) => (false, format!("workflow cancel error: {:?}", error)),
    }
}

fn cleanup_outcome(result: Result<String, TerminalError>) -> String {
    match result {
        Ok(message) => message,
        Err(error) => format!("cleanup error: {:?}", error),
    }
}
