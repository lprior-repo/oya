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
use std::path::{Path, PathBuf};
use tokio::process::Command as TokioCommand;
use tokio::time::{sleep, Duration};

const DEFAULT_BIND: &str = "127.0.0.1:9180";
const DEFAULT_INGRESS: &str = "http://127.0.0.1:909";
const DEFAULT_IMPL_MODEL: &str = "zai-coding-plan/glm-5";
const DEFAULT_SERVICE_URL: &str = "http://127.0.0.1:9180/";

#[derive(Debug, Parser)]
#[command(name = "oya")]
#[command(about = "OIA -> Restate -> OpenCode bridge")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init(InitArgs),
    Serve(ServeArgs),
    Invoke(InvokeArgs),
    Implement(ImplementArgs),
    Lifecycle(LifecycleArgs),
    Status(StatusArgs),
    Cancel(CancelArgs),
}

#[derive(Debug, clap::Args)]
struct InitArgs {
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    ingress: String,
    #[arg(long, default_value = DEFAULT_SERVICE_URL, value_parser = parse_service_url)]
    service_url: String,
    #[arg(long)]
    down: bool,
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
    #[arg(long, value_parser = parse_repo_slug)]
    repo: Option<String>,
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

#[derive(Debug, Deserialize)]
struct GhRepoView {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init(args) => init_command(args).await,
        Command::Serve(args) => serve_command(args).await,
        Command::Invoke(args) => invoke_command(args).await,
        Command::Implement(args) => implement_command(args).await,
        Command::Lifecycle(args) => lifecycle_command(args).await,
        Command::Status(args) => status_command(args).await,
        Command::Cancel(args) => cancel_command(args).await,
    }
}

async fn init_command(args: InitArgs) -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    disable_systemd_restate().await?;
    if args.down {
        stop_docker_restate(&repo_root).await?;
        println!("[oya] Docker Restate stopped");
        return Ok(());
    }
    start_fresh_docker_restate(&repo_root).await?;
    restart_oya_service().await?;
    wait_for_health(&args.ingress, 30).await?;
    wait_for_service_discovery(30).await?;
    register_services(&args.service_url).await?;
    validate_registered_services().await?;
    println!("[oya] Runtime ready (fresh Restate + handlers registered)");
    println!("  Admin:   http://127.0.0.1:9070");
    println!("  Ingress: {}", args.ingress);
    println!("  Service: {}", args.service_url);
    Ok(())
}

fn find_repo_root() -> anyhow::Result<PathBuf> {
    let current = std::env::current_dir()?;
    let root = current
        .ancestors()
        .find(|path| path.join("docker-compose.yml").is_file())
        .map(Path::to_path_buf);
    root.ok_or_else(|| anyhow::anyhow!("could not find docker-compose.yml from current directory"))
}

async fn disable_systemd_restate() -> anyhow::Result<()> {
    let _ =
        run_command_capture("systemctl", &["--user", "disable", "--now", "restate.service"], None)
            .await;
    let _ =
        run_command_capture("systemctl", &["--user", "stop", "restate-manual.service"], None).await;
    Ok(())
}

async fn stop_docker_restate(repo_root: &Path) -> anyhow::Result<()> {
    run_command_capture(
        "docker",
        &["compose", "-f", "docker-compose.yml", "stop", "restate"],
        Some(repo_root),
    )
    .await
    .map(|_| ())
}

async fn start_fresh_docker_restate(repo_root: &Path) -> anyhow::Result<()> {
    run_command_capture(
        "docker",
        &["compose", "-f", "docker-compose.yml", "down", "-v", "--remove-orphans"],
        Some(repo_root),
    )
    .await?;
    run_command_capture(
        "docker",
        &["compose", "-f", "docker-compose.yml", "pull", "restate"],
        Some(repo_root),
    )
    .await?;
    run_command_capture(
        "docker",
        &["compose", "-f", "docker-compose.yml", "up", "-d", "restate"],
        Some(repo_root),
    )
    .await
    .map(|_| ())
}

async fn restart_oya_service() -> anyhow::Result<()> {
    run_command_capture("systemctl", &["--user", "restart", "oya.service"], None).await?;
    let status =
        run_command_capture("systemctl", &["--user", "is-active", "oya.service"], None).await?;
    if status.trim() == "active" {
        Ok(())
    } else {
        Err(anyhow::anyhow!("oya.service is not active after restart"))
    }
}

async fn wait_for_health(ingress: &str, retries: u8) -> anyhow::Result<()> {
    let url = format!("{}/restate/health", ingress);
    for _ in 0..retries {
        if let Ok(response) = Client::new().get(&url).send().await {
            if response.status().is_success() {
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow::anyhow!("restate health check timed out: {url}"))
}

async fn wait_for_service_discovery(retries: u8) -> anyhow::Result<()> {
    let url = "http://127.0.0.1:9180/discover";
    for _ in 0..retries {
        if let Ok(response) = Client::new().get(url).send().await {
            if response.status().is_success() {
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow::anyhow!("oya discovery endpoint timed out: {url}"))
}

async fn register_services(service_url: &str) -> anyhow::Result<()> {
    run_command_capture("restate", &["deployments", "register", "--force", service_url], None)
        .await
        .map(|_| ())
}

async fn validate_registered_services() -> anyhow::Result<()> {
    let output = run_command_capture("restate", &["services", "list"], None).await?;
    if has_required_services(&output) {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "missing required services in Restate registry (expected Oya/OyaMemory/OyaService)"
        ))
    }
}

fn has_required_services(output: &str) -> bool {
    let tokens = output.lines().flat_map(|line| line.split_whitespace()).collect::<Vec<_>>();
    ["Oya", "OyaMemory", "OyaService"].iter().all(|name| tokens.iter().any(|token| token == name))
}

fn parse_ingress_url(value: &str) -> Result<String, String> {
    parse_url_with_expected_port(value, 909, "ingress")
}

fn parse_service_url(value: &str) -> Result<String, String> {
    parse_url_with_expected_port(value, 9180, "service")
}

fn parse_url_with_expected_port(
    value: &str,
    expected_port: u16,
    label: &str,
) -> Result<String, String> {
    let parsed = url::Url::parse(value).map_err(|error| format!("invalid {label} URL: {error}"))?;
    let port =
        parsed.port_or_known_default().ok_or_else(|| format!("{label} URL must include port"))?;
    if port == expected_port {
        Ok(value.to_owned())
    } else {
        Err(format!("{label} URL must use port {expected_port} (avoid common 8080/80 ports)"))
    }
}

async fn run_command_capture(
    command: &str,
    args: &[&str],
    workdir: Option<&Path>,
) -> anyhow::Result<String> {
    let mut process = TokioCommand::new(command);
    process.args(args);
    if let Some(path) = workdir {
        process.current_dir(path);
    }
    let output = process
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run {command}: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("{command} failed: {}", stderr.trim()));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("{command} output was not UTF-8: {error}"))
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
    run_simple_command(&["update", &bead_id, "--status", "in_progress"]).await?;
    let bead_state_raw = run_capture_command(&["show", "--json", &bead_id]).await?;
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
    let repo = resolve_repo_slug(args.repo).await?;
    let request = LifecycleRequest { bead_id: args.bead, model: Some(args.model), repo };
    let body =
        call_restate_service_json(&args.ingress, "Oya", &workflow_key, "run", request).await?;
    println!("{}", body.output);
    Ok(())
}

async fn resolve_repo_slug(repo: Option<String>) -> anyhow::Result<Option<String>> {
    match repo {
        Some(explicit) => Ok(Some(explicit)),
        None => detect_repo_slug().await,
    }
}

async fn detect_repo_slug() -> anyhow::Result<Option<String>> {
    let output = TokioCommand::new("gh")
        .args(["repo", "view", "--json", "nameWithOwner"])
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run gh repo view: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("gh output was not UTF-8: {error}"))?;
    extract_repo_slug_from_gh_output(&stdout).map(Some)
}

fn extract_repo_slug_from_gh_output(raw: &str) -> anyhow::Result<String> {
    let payload: GhRepoView = serde_json::from_str(raw)?;
    parse_repo_slug(&payload.name_with_owner).map_err(anyhow::Error::msg)
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
    let raw = run_capture_command(&["ready", "--json"]).await?;
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

async fn run_capture_command(args: &[&str]) -> anyhow::Result<String> {
    let output = TokioCommand::new("br")
        .args(args)
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run br: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("br failed: {}", stderr.trim()));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("br output was not UTF-8: {error}"))
}

async fn run_simple_command(args: &[&str]) -> anyhow::Result<()> {
    let output = TokioCommand::new("br")
        .args(args)
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run br: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("br failed: {}", stderr.trim()))
    }
}

fn parse_socket_addr(value: String) -> anyhow::Result<SocketAddr> {
    value
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("invalid --bind '{}': {error}", value))
}

fn parse_repo_slug(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let (owner, repo) =
        trimmed.split_once('/').ok_or_else(|| "expected OWNER/REPO format".to_owned())?;
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return Err("expected OWNER/REPO format".to_owned());
    }
    if is_valid_repo_part(owner) && is_valid_repo_part(repo) {
        Ok(trimmed.to_owned())
    } else {
        Err("repo may contain only [A-Za-z0-9._-]".to_owned())
    }
}

fn is_valid_repo_part(value: &str) -> bool {
    value.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

#[cfg(test)]
mod main_tests;
