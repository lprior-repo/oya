use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use tokio::process::Command;

const DEFAULT_MODEL: &str = "openai/gpt-5.3-codex";

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
trait Oia {
    async fn start(req: Json<StartRequest>) -> Result<Json<StartResponse>, HandlerError>;
    async fn sync_bead(req: Json<BeadSyncRequest>) -> Result<Json<StartResponse>, HandlerError>;
    async fn run_pipeline(req: Json<PipelineRequest>) -> Result<Json<StartResponse>, HandlerError>;
}

pub struct OiaBridge;

impl Oia for OiaBridge {
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
        let model = model_or_default(req.into_inner().model);
        let bead_id = require_state_string(&ctx, "bead_id").await?;
        let bead_state = require_state_json(&ctx, "bead_state").await?;
        let prompt = pipeline_prompt(&bead_id, bead_state)?;
        let output = ctx.run(move || run_opencode(prompt, model)).name("opencode_pipeline").await?;
        store_output(&ctx, &output);
        Ok(StartResponse { output }.into())
    }
}

pub async fn serve(bind: SocketAddr) -> anyhow::Result<()> {
    let endpoint = Endpoint::builder().bind(OiaBridge.serve()).build();
    HttpServer::new(endpoint).listen_and_serve(bind).await;
    Ok(())
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

fn model_or_default(value: Option<String>) -> String {
    match value {
        Some(model) => model,
        None => DEFAULT_MODEL.to_owned(),
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

async fn run_opencode(prompt: Prompt, model: String) -> Result<String, HandlerError> {
    let output = Command::new("opencode")
        .arg("run")
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg(model)
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
        .unwrap_or_default();
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
    let tool =
        part.and_then(|value| value.get("tool")).and_then(Value::as_str).unwrap_or("unknown");
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
        .unwrap_or_default();
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
