#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod lifecycle;
pub mod restate_oya;

use clap::{Parser, Subcommand};
use reqwest::Client;
use restate_oya::{
    BeadSyncRequest, CancelResponse, KeyRequest, LifecycleRequest, LifecycleStatusSnapshot,
    PipelineRequest, StartRequest, StartResponse,
};
use serde::Deserialize;
use serde_json::Value;
use std::net::SocketAddr;
use tokio::process::Command as TokioCommand;

const DEFAULT_BIND: &str = "127.0.0.1:9080";
const DEFAULT_INGRESS: &str = "http://127.0.0.1:8080";
const DEFAULT_IMPL_MODEL: &str = "zai-coding-plan/glm-5";

#[derive(Debug, Parser)]
#[command(name = "oya")]
#[command(about = "OIA -> Restate -> OpenCode bridge")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve(ServeArgs),
    Invoke(InvokeArgs),
    Implement(ImplementArgs),
    Lifecycle(LifecycleArgs),
    Status(StatusArgs),
    Cancel(CancelArgs),
}

#[derive(Debug, clap::Args)]
struct ServeArgs {
    #[arg(long, default_value = DEFAULT_BIND)]
    bind: String,
}

#[derive(Debug, clap::Args)]
struct InvokeArgs {
    #[arg(long, default_value = DEFAULT_INGRESS)]
    ingress: String,
    #[arg(long, default_value = "default")]
    id: String,
    #[arg(long)]
    prompt: String,
    #[arg(long)]
    model: Option<String>,
}

#[derive(Debug, clap::Args)]
struct ImplementArgs {
    #[arg(long)]
    bead: Option<String>,
    #[arg(long, default_value = DEFAULT_INGRESS)]
    ingress: String,
    #[arg(long, default_value = DEFAULT_IMPL_MODEL)]
    model: String,
}

#[derive(Debug, clap::Args)]
struct LifecycleArgs {
    #[arg(long)]
    bead: Option<String>,
    #[arg(long, default_value = DEFAULT_INGRESS)]
    ingress: String,
    #[arg(long, default_value = DEFAULT_IMPL_MODEL)]
    model: String,
}

#[derive(Debug, clap::Args)]
struct StatusArgs {
    #[arg(long)]
    key: String,
    #[arg(long, default_value = DEFAULT_INGRESS)]
    ingress: String,
}

#[derive(Debug, clap::Args)]
struct CancelArgs {
    #[arg(long)]
    key: String,
    #[arg(long, default_value = DEFAULT_INGRESS)]
    ingress: String,
}

#[derive(Debug, Deserialize)]
struct ReadyIssue {
    id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve(args) => serve_command(args).await,
        Command::Invoke(args) => invoke_command(args).await,
        Command::Implement(args) => implement_command(args).await,
        Command::Lifecycle(args) => lifecycle_command(args).await,
        Command::Status(args) => status_command(args).await,
        Command::Cancel(args) => cancel_command(args).await,
    }
}

async fn serve_command(args: ServeArgs) -> anyhow::Result<()> {
    let bind = parse_socket_addr(args.bind)?;
    restate_oya::serve(bind).await
}

async fn invoke_command(args: InvokeArgs) -> anyhow::Result<()> {
    let request = StartRequest {
        prompt: args.prompt,
        model: args.model,
        bead_id: None,
        bead_status: None,
        bead_state: None,
    };
    let body = call_restate_start(&args.ingress, &args.id, request).await?;
    println!("{}", body.output);
    Ok(())
}

async fn implement_command(args: ImplementArgs) -> anyhow::Result<()> {
    let bead_id = match args.bead {
        Some(id) => id,
        None => pick_ready_bead().await?,
    };
    run_simple_command("br", &["update", &bead_id, "--status", "in_progress"]).await?;
    let bead_state_raw = run_capture_command("br", &["show", "--json", &bead_id]).await?;
    let bead_state = parse_json_payload(&bead_state_raw)?;
    let sync_request = BeadSyncRequest {
        bead_id: bead_id.clone(),
        bead_status: "in_progress".to_owned(),
        bead_state,
    };
    call_restate_json(&args.ingress, &bead_id, "sync_bead", sync_request).await?;
    let pipeline_request = PipelineRequest { model: Some(args.model) };
    let body = call_restate_json(&args.ingress, &bead_id, "run_pipeline", pipeline_request).await?;
    print!("{}", body.output);
    Ok(())
}

async fn lifecycle_command(args: LifecycleArgs) -> anyhow::Result<()> {
    let workflow_key = args.bead.clone().unwrap_or_else(|| "auto".to_owned());
    let request = LifecycleRequest { bead_id: args.bead, model: Some(args.model) };
    let body =
        call_restate_service_json(&args.ingress, "Oya", &workflow_key, "run", request).await?;
    println!("{}", body.output);
    Ok(())
}

async fn status_command(args: StatusArgs) -> anyhow::Result<()> {
    let request = KeyRequest { key: args.key };
    let snapshot: LifecycleStatusSnapshot =
        call_restate_root_json(&args.ingress, "OyaService", "get_lifecycle", request).await?;
    let formatted = serde_json::to_string_pretty(&snapshot)?;
    println!("{formatted}");
    Ok(())
}

async fn cancel_command(args: CancelArgs) -> anyhow::Result<()> {
    let request = KeyRequest { key: args.key };
    let response: CancelResponse =
        call_restate_root_json(&args.ingress, "OyaService", "cancel", request).await?;
    let formatted = serde_json::to_string_pretty(&response)?;
    println!("{formatted}");
    Ok(())
}

async fn call_restate_start(
    ingress: &str,
    id: &str,
    request: StartRequest,
) -> anyhow::Result<StartResponse> {
    call_restate_json(ingress, id, "start", request).await
}

async fn call_restate_json<T: serde::Serialize>(
    ingress: &str,
    id: &str,
    handler: &str,
    request: T,
) -> anyhow::Result<StartResponse> {
    call_restate_service_json(ingress, "OyaMemory", id, handler, request).await
}

async fn call_restate_service_json<T: serde::Serialize>(
    ingress: &str,
    service: &str,
    id: &str,
    handler: &str,
    request: T,
) -> anyhow::Result<StartResponse> {
    let url = format!("{}/{}/{}/{}", ingress, service, id, handler);
    let response = Client::new().post(url).json(&request).send().await?;
    let response = response.error_for_status()?;
    response.json().await.map_err(Into::into)
}

async fn call_restate_root_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(
    ingress: &str,
    service: &str,
    handler: &str,
    request: T,
) -> anyhow::Result<R> {
    let url = format!("{}/{}/{}", ingress, service, handler);
    let response = Client::new().post(url).json(&request).send().await?;
    let response = response.error_for_status()?;
    response.json().await.map_err(Into::into)
}

async fn pick_ready_bead() -> anyhow::Result<String> {
    let raw = run_capture_command("br", &["ready", "--json"]).await?;
    let json = extract_json_array(&raw)?;
    let issues: Vec<ReadyIssue> = serde_json::from_str(json)?;
    match issues.first() {
        Some(issue) => Ok(issue.id.clone()),
        None => Err(anyhow::anyhow!("no ready beads found")),
    }
}

fn extract_json_array(raw: &str) -> anyhow::Result<&str> {
    match raw.find('[') {
        Some(index) => Ok(&raw[index..]),
        None => Err(anyhow::anyhow!("br ready --json returned no JSON payload")),
    }
}

fn parse_json_payload(raw: &str) -> anyhow::Result<Value> {
    let object_idx = raw.find('{');
    let array_idx = raw.find('[');
    let start = match (object_idx, array_idx) {
        (Some(o), Some(a)) => o.min(a),
        (Some(o), None) => o,
        (None, Some(a)) => a,
        (None, None) => {
            return Err(anyhow::anyhow!("command returned no JSON payload to parse"));
        }
    };
    serde_json::from_str(&raw[start..]).map_err(Into::into)
}

async fn run_capture_command(binary: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = TokioCommand::new(binary)
        .args(args)
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run {binary}: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("{binary} failed: {}", stderr.trim()));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("{binary} output was not UTF-8: {error}"))
}

async fn run_simple_command(binary: &str, args: &[&str]) -> anyhow::Result<()> {
    let output = TokioCommand::new(binary)
        .args(args)
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run {binary}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("{binary} failed: {}", stderr.trim()))
    }
}

fn parse_socket_addr(value: String) -> anyhow::Result<SocketAddr> {
    value
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("invalid --bind '{}': {error}", value))
}
