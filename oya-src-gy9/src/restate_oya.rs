use crate::lifecycle::effects::TokioCommandExecutor;
use crate::lifecycle::types::Model;
use crate::lifecycle::workflow::{run_lifecycle, LifecycleRunRequest};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use tokio::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub bead_id: Option<String>,
    pub bead_status: Option<String>,
    pub bead_state: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BeadSyncRequest {
    pub bead_id: String,
    pub bead_status: String,
    pub bead_state: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipelineRequest {
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LifecycleRequest {
    pub bead_id: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyRequest {
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BeadSnapshot {
    pub bead_id: Option<String>,
    pub bead_status: Option<String>,
    pub bead_state: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemorySnapshot {
    pub bead: BeadSnapshot,
    pub last_output_summary: Option<Value>,
    pub last_output_trace: Option<Value>,
    pub active_invocation_id: Option<String>,
    pub cancel_requested: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelResponse {
    pub cancelled: bool,
    pub message: String,
}

pub fn pipeline_prompt(bead_id: &str, bead_state: Value) -> Result<Prompt, TerminalError> {
    let state_json = serde_json::to_string_pretty(&bead_state)
        .map_err(|error| TerminalError::new(format!("invalid bead_state json: {error}")))?;
    Prompt::parse(format!(
        "Implement bead {bead_id} using this state from Restate.\n\nBead State:\n{state_json}\n\nSteps: 1) implement requested changes in repo, 2) run moon run :check, 3) summarize files changed and test result."
    ))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartResponse {
    pub output: String,
}

#[restate_sdk::object]
trait OyaMemory {
    async fn start(req: Json<StartRequest>) -> Result<Json<StartResponse>, HandlerError>;
    async fn sync_bead(req: Json<BeadSyncRequest>) -> Result<Json<StartResponse>, HandlerError>;
    async fn run_pipeline(req: Json<PipelineRequest>) -> Result<Json<StartResponse>, HandlerError>;
    #[shared]
    async fn get_state() -> Result<Json<MemorySnapshot>, HandlerError>;
    #[shared]
    async fn get_bead() -> Result<Json<BeadSnapshot>, HandlerError>;
    async fn request_cancel() -> Result<Json<CancelResponse>, HandlerError>;
}

pub struct OyaMemoryBridge;

#[restate_sdk::service]
trait OyaService {
    async fn get_state(req: Json<KeyRequest>) -> Result<Json<MemorySnapshot>, HandlerError>;
    async fn get_bead(req: Json<KeyRequest>) -> Result<Json<BeadSnapshot>, HandlerError>;
    async fn cancel(req: Json<KeyRequest>) -> Result<Json<CancelResponse>, HandlerError>;
}

pub struct OyaServiceBridge;

#[restate_sdk::workflow]
trait Oya {
    async fn run(req: Json<LifecycleRequest>) -> Result<Json<StartResponse>, HandlerError>;
}

pub struct OyaBridge;

impl Oya for OyaBridge {
    async fn run(
        &self,
        _ctx: WorkflowContext<'_>,
        req: Json<LifecycleRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let body = req.into_inner();
        let result = run_lifecycle(
            &TokioCommandExecutor,
            LifecycleRunRequest { bead_id: body.bead_id, model: body.model },
        )
        .await;
        match result {
            Ok(outcome) => serialize_workflow_outcome(&outcome).map(Into::into),
            Err(failure) => {
                let message = serde_json::to_string(&failure).map_err(|error| {
                    HandlerError::from(format!("failed to serialize lifecycle failure: {error}"))
                })?;
                Err(TerminalError::new(message).into())
            }
        }
    }
}

impl OyaMemory for OyaMemoryBridge {
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
        if ctx.get::<bool>("cancel_requested").await?.unwrap_or(false) {
            return Err(TerminalError::new("cancel requested before pipeline run").into());
        }
        let model = model_or_default(req.into_inner().model);
        ctx.set("active_invocation_id", ctx.invocation_id().to_owned());
        ctx.set("cancel_requested", false);
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
            bead_id: ctx.get::<String>("bead_id").await?,
            bead_status: ctx.get::<String>("bead_status").await?,
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
            cancel_requested: ctx.get::<bool>("cancel_requested").await?.unwrap_or(false),
        };
        Ok(snapshot.into())
    }

    async fn get_bead(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BeadSnapshot>, HandlerError> {
        Ok(BeadSnapshot {
            bead_id: ctx.get::<String>("bead_id").await?,
            bead_status: ctx.get::<String>("bead_status").await?,
            bead_state: ctx.get::<Json<Value>>("bead_state").await?.map(Json::into_inner),
        }
        .into())
    }

    async fn request_cancel(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<CancelResponse>, HandlerError> {
        ctx.set("cancel_requested", true);
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

impl OyaService for OyaServiceBridge {
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

    async fn cancel(
        &self,
        ctx: Context<'_>,
        req: Json<KeyRequest>,
    ) -> Result<Json<CancelResponse>, HandlerError> {
        let key = req.into_inner().key;
        ctx.object_client::<OyaMemoryClient>(&key).request_cancel().call().await.map_err(Into::into)
    }
}

pub async fn serve(bind: SocketAddr) -> anyhow::Result<()> {
    let endpoint = Endpoint::builder()
        .bind(OyaMemoryBridge.serve())
        .bind(OyaBridge.serve())
        .bind(OyaServiceBridge.serve())
        .build();
    HttpServer::new(endpoint).listen_and_serve(bind).await;
    Ok(())
}

fn serialize_workflow_outcome(
    outcome: &crate::lifecycle::workflow::LifecycleRunOutcome,
) -> Result<StartResponse, HandlerError> {
    let output = serde_json::to_string(outcome).map_err(|error| {
        HandlerError::from(format!("failed to serialize lifecycle outcome: {error}"))
    })?;
    Ok(StartResponse { output })
}

#[derive(Debug, Clone)]
pub struct Prompt(String);

impl Prompt {
    pub fn parse(raw: String) -> Result<Self, TerminalError> {
        let normalized = raw.trim().to_owned();
        if normalized.is_empty() {
            return Err(TerminalError::new("prompt cannot be empty"));
        }
        Ok(Self(normalized))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

fn model_or_default(value: Option<String>) -> Model {
    value.and_then(|m| Model::parse(&m).ok()).unwrap_or_else(Model::default_model)
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

async fn run_opencode(prompt: Prompt, model: Model) -> Result<String, HandlerError> {
    let output = Command::new("opencode")
        .arg("run")
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg(model.as_str())
        .arg(prompt.into_inner())
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to run opencode: {error}")))?;
    parse_output(output)
}

fn parse_output(output: std::process::Output) -> Result<String, HandlerError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let message = format!("opencode failed: {}", stderr.trim());
        return Err(TerminalError::new(message).into());
    }
    String::from_utf8(output.stdout).map_err(|error| {
        TerminalError::new(format!("opencode output was not UTF-8: {error}")).into()
    })
}

async fn cancel_invocation(invocation_id: String) -> Result<(), HandlerError> {
    let output = Command::new("restate")
        .arg("invocations")
        .arg("cancel")
        .arg(&invocation_id)
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to invoke restate CLI: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(TerminalError::new(format!(
            "restate cancel failed for {invocation_id}: {}",
            stderr.trim()
        ))
        .into())
    }
}

fn parse_jsonl_events(raw: &str) -> Result<Vec<Value>, serde_json::Error> {
    raw.lines().filter(|line| !line.trim().is_empty()).map(serde_json::from_str::<Value>).collect()
}

fn summarize_events(events: &[Value]) -> Value {
    let tool_calls = events
        .iter()
        .filter(|event| event.get("type") == Some(&Value::String("tool_use".to_owned())))
        .count();
    let final_text = events
        .iter()
        .rev()
        .find_map(|event| event.get("part")?.get("text")?.as_str())
        .map(|text| text.chars().take(500).collect::<String>())
        .map_or_else(String::new, std::convert::identity);
    serde_json::json!({
        "event_count": events.len(),
        "tool_calls": tool_calls,
        "final_text": final_text,
    })
}

fn fallback_summary(raw_output: &str) -> Value {
    serde_json::json!({
        "event_count": 0,
        "tool_calls": 0,
        "final_text": truncate_text(raw_output, 500),
        "parse_error": true,
    })
}

fn build_clean_trace(events: &[Value]) -> Value {
    let mut step = 0usize;
    let entries =
        events.iter().filter_map(|event| trace_entry(event, &mut step)).collect::<Vec<_>>();
    Value::Array(entries)
}

fn trace_entry(event: &Value, step: &mut usize) -> Option<Value> {
    match event.get("type")?.as_str()? {
        "step_start" => {
            *step += 1;
            Some(serde_json::json!({
                "step": *step,
                "kind": "step_start",
                "timestamp": event.get("timestamp"),
                "session_id": event.get("sessionID"),
            }))
        }
        "tool_use" => Some(tool_entry(event, *step)),
        "text" => Some(text_entry(event, *step)),
        "step_finish" => Some(finish_entry(event, *step)),
        _ => None,
    }
}

fn tool_entry(event: &Value, step: usize) -> Value {
    let part = event.get("part").and_then(Value::as_object);
    let state = part.and_then(|value| value.get("state")).and_then(Value::as_object);
    let input = state.and_then(|value| value.get("input")).and_then(Value::as_object);
    let tool = part
        .and_then(|value| value.get("tool"))
        .and_then(Value::as_str)
        .map_or("unknown", std::convert::identity);
    serde_json::json!({
        "step": step,
        "kind": "tool_use",
        "tool": tool,
        "description": input.and_then(|value| value.get("description")).and_then(Value::as_str),
        "command": input.and_then(|value| value.get("command")).and_then(Value::as_str),
        "query": input.and_then(|value| value.get("query")).and_then(Value::as_str),
    })
}

fn text_entry(event: &Value, step: usize) -> Value {
    let text = event
        .get("part")
        .and_then(|value| value.get("text"))
        .and_then(Value::as_str)
        .map(|value| truncate_text(value, 500))
        .map_or_else(String::new, std::convert::identity);
    serde_json::json!({
        "step": step,
        "kind": "text",
        "text": text,
    })
}

fn finish_entry(event: &Value, step: usize) -> Value {
    let part = event.get("part").and_then(Value::as_object);
    serde_json::json!({
        "step": step,
        "kind": "step_finish",
        "reason": part.and_then(|value| value.get("reason")).and_then(Value::as_str),
        "tokens": part.and_then(|value| value.get("tokens")).cloned(),
    })
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect::<String>()
}
