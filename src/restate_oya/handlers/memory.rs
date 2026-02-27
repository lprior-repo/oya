#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::{BeadId, BeadStatus, CancelState};
use restate_sdk::prelude::*;
use serde_json::Value;

use crate::restate_oya::opencode::{
    cancel_invocation, model_or_default, pipeline_prompt, run_opencode, Prompt,
};
use crate::restate_oya::trace::{
    build_clean_trace, fallback_summary, parse_jsonl_events, summarize_events,
};
use crate::restate_oya::types::{
    BeadSnapshot, BeadSyncRequest, CancelResponse, MemorySnapshot, PipelineRequest, StartRequest,
    StartResponse,
};

pub struct OyaMemoryBridge;

impl super::OyaMemory for OyaMemoryBridge {
    async fn start(
        &self,
        ctx: ObjectContext<'_>,
        req: Json<StartRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let body = req.into_inner();
        persist_bead_state(&ctx, &body);
        let prompt = Prompt::parse(body.prompt).map_err(HandlerError::from)?;
        let model = model_or_default(body.model);
        let output = ctx.run(move || run_opencode(prompt, model)).name("opencode_run").await?;
        store_output(&ctx, &output);
        Ok(StartResponse { output }.into())
    }

    async fn sync_bead(
        &self,
        ctx: ObjectContext<'_>,
        req: Json<BeadSyncRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let bead = req.into_inner();
        ctx.set("bead_id", bead.bead_id.clone());
        ctx.set("bead_status", bead.bead_status.clone());
        ctx.set("bead_state", Json::from(bead.bead_state));
        let output = format!("synced bead {}", bead.bead_id);
        Ok(StartResponse { output }.into())
    }

    async fn run_pipeline(
        &self,
        ctx: ObjectContext<'_>,
        req: Json<PipelineRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let cancel_state = ctx
            .get::<String>("cancel_state")
            .await?
            .and_then(parse_cancel_state)
            .unwrap_or_default();
        if cancel_state.is_cancel_requested() {
            return Err(TerminalError::new("cancel requested before pipeline run").into());
        }
        let model = model_or_default(req.into_inner().model);
        ctx.set("active_invocation_id", ctx.invocation_id().to_owned());
        ctx.set("cancel_state", "active".to_owned());
        let bead_id = require_state_string(&ctx, "bead_id").await?;
        let bead_state = require_state_json(&ctx, "bead_state").await?;
        let prompt = pipeline_prompt(&bead_id, bead_state)?;
        let output = ctx.run(move || run_opencode(prompt, model)).name("opencode_pipeline").await?;
        store_output(&ctx, &output);
        ctx.clear("active_invocation_id");
        Ok(StartResponse { output }.into())
    }

    async fn get_state(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<MemorySnapshot>, HandlerError> {
        let bead = BeadSnapshot {
            bead_id: ctx
                .get::<String>("bead_id")
                .await?
                .and_then(|value| BeadId::parse(&value).ok()),
            bead_status: ctx
                .get::<String>("bead_status")
                .await?
                .and_then(|value| BeadStatus::parse(&value).ok()),
            bead_state: ctx.get::<Json<Value>>("bead_state").await?.map(Json::into_inner),
        };
        let snapshot = MemorySnapshot {
            bead,
            last_output_summary: ctx
                .get::<Json<Value>>("last_output_summary")
                .await?
                .map(Json::into_inner),
            last_output_trace: ctx
                .get::<Json<Value>>("last_output_trace")
                .await?
                .map(Json::into_inner),
            active_invocation_id: ctx.get::<String>("active_invocation_id").await?,
            cancel_state: ctx
                .get::<String>("cancel_state")
                .await?
                .and_then(parse_cancel_state)
                .unwrap_or_default(),
        };
        Ok(snapshot.into())
    }

    async fn get_bead(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BeadSnapshot>, HandlerError> {
        Ok(BeadSnapshot {
            bead_id: ctx
                .get::<String>("bead_id")
                .await?
                .and_then(|value| BeadId::parse(&value).ok()),
            bead_status: ctx
                .get::<String>("bead_status")
                .await?
                .and_then(|value| BeadStatus::parse(&value).ok()),
            bead_state: ctx.get::<Json<Value>>("bead_state").await?.map(Json::into_inner),
        }
        .into())
    }

    async fn request_cancel(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<CancelResponse>, HandlerError> {
        ctx.set("cancel_state", "cancel_requested".to_owned());
        let active_invocation_id = ctx.get::<String>("active_invocation_id").await?;
        match active_invocation_id {
            Some(invocation_id) => {
                let cancel_id = invocation_id.clone();
                let cancel_result =
                    ctx.run(move || cancel_invocation(cancel_id)).name("cancel_invocation").await;
                match cancel_result {
                    Ok(()) => Ok(CancelResponse {
                        cancelled: true,
                        message: format!("cancel requested for invocation {invocation_id}"),
                    }
                    .into()),
                    Err(error) => Ok(CancelResponse {
                        cancelled: false,
                        message: format!("failed to cancel invocation {invocation_id}: {error}"),
                    }
                    .into()),
                }
            }
            None => Ok(CancelResponse {
                cancelled: false,
                message: "no active invocation to cancel".to_owned(),
            }
            .into()),
        }
    }
}

fn parse_cancel_state(value: String) -> Option<CancelState> {
    match value.trim().to_lowercase().as_str() {
        "active" => Some(CancelState::Active),
        "cancel_requested" => Some(CancelState::CancelRequested),
        _ => None,
    }
}

fn persist_bead_state(ctx: &ObjectContext<'_>, request: &StartRequest) {
    if let Some(bead_id) = &request.bead_id {
        ctx.set("bead_id", bead_id.clone());
    }
    if let Some(bead_status) = &request.bead_status {
        ctx.set("bead_status", bead_status.clone());
    }
    if let Some(bead_state) = &request.bead_state {
        ctx.set("bead_state", Json::from(bead_state.clone()));
    }
}

fn store_output(ctx: &ObjectContext<'_>, output: &str) {
    ctx.clear("last_output");
    ctx.clear("last_output_events");
    if let Ok(events) = parse_jsonl_events(output) {
        ctx.set("last_output_summary", Json::from(summarize_events(&events)));
        ctx.set("last_output_trace", Json::from(build_clean_trace(&events)));
    } else {
        ctx.set("last_output_summary", Json::from(fallback_summary(output)));
        ctx.set("last_output_trace", Json::from(Vec::<Value>::new()));
    }
}

async fn require_state_string(ctx: &ObjectContext<'_>, key: &str) -> Result<String, HandlerError> {
    ctx.get::<String>(key)
        .await?
        .ok_or_else(|| TerminalError::new(format!("missing state key: {key}")).into())
}

async fn require_state_json(ctx: &ObjectContext<'_>, key: &str) -> Result<Value, HandlerError> {
    ctx.get::<Json<Value>>(key)
        .await?
        .map(Json::into_inner)
        .ok_or_else(|| TerminalError::new(format!("missing state key: {key}")).into())
}
