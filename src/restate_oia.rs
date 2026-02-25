use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::process::Command;

const DEFAULT_MODEL: &str = "openai/gpt-5.3-codex";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartRequest {
    pub prompt: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartResponse {
    pub output: String,
}

#[restate_sdk::object]
trait Oia {
    async fn start(req: Json<StartRequest>) -> Result<Json<StartResponse>, HandlerError>;
}

pub struct OiaBridge;

impl Oia for OiaBridge {
    async fn start(
        &self,
        ctx: ObjectContext<'_>,
        req: Json<StartRequest>,
    ) -> Result<Json<StartResponse>, HandlerError> {
        let body = req.into_inner();
        let prompt = Prompt::parse(body.prompt).map_err(HandlerError::from)?;
        let model = model_or_default(body.model);
        let output = ctx.run(move || run_opencode(prompt, model)).name("opencode_run").await?;
        Ok(StartResponse { output }.into())
    }
}

pub async fn serve(bind: SocketAddr) -> anyhow::Result<()> {
    let endpoint = Endpoint::builder().bind(OiaBridge.serve()).build();
    HttpServer::new(endpoint).listen_and_serve(bind).await;
    Ok(())
}

#[derive(Debug, Clone)]
struct Prompt(String);

impl Prompt {
    fn parse(raw: String) -> Result<Self, TerminalError> {
        let normalized = raw.trim().to_owned();
        if normalized.is_empty() {
            return Err(TerminalError::new("prompt cannot be empty"));
        }
        Ok(Self(normalized))
    }

    fn into_inner(self) -> String {
        self.0
    }
}

fn model_or_default(value: Option<String>) -> String {
    match value {
        Some(model) => model,
        None => DEFAULT_MODEL.to_owned(),
    }
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
