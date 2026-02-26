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
use serde::Serialize;
use serde_json::Value;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::process::Command as TokioCommand;
use tokio::time::{sleep, Duration};

const DEFAULT_BIND: &str = "127.0.0.1:9180";
const DEFAULT_INGRESS: &str = "http://127.0.0.1:909";
const DEFAULT_ADMIN: &str = "http://127.0.0.1:9070";
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
    Doctor(DoctorArgs),
    Serve(ServeArgs),
    Invoke(InvokeArgs),
    Implement(ImplementArgs),
    Lifecycle(LifecycleArgs),
    Status(StatusArgs),
    Cancel(CancelArgs),
    Beads(BeadsArgs),
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
struct DoctorArgs {
    #[arg(long, default_value = DEFAULT_INGRESS, value_parser = parse_ingress_url)]
    ingress: String,
    #[arg(long, default_value = DEFAULT_ADMIN, value_parser = parse_admin_url)]
    admin: String,
    #[arg(long, default_value = DEFAULT_SERVICE_URL, value_parser = parse_service_url)]
    service_url: String,
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

#[derive(Debug, clap::Args)]
struct BeadsArgs {
    #[arg(long)]
    ready: bool,
    #[arg(long)]
    json: bool,
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

#[derive(Debug, Serialize)]
struct DoctorCheck {
    id: String,
    pass: bool,
    expected: String,
    actual: String,
    remediation: String,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    ok: bool,
    checks: Vec<DoctorCheck>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init(args) => init_command(args).await,
        Command::Doctor(args) => doctor_command(args).await,
        Command::Serve(args) => serve_command(args).await,
        Command::Invoke(args) => invoke_command(args).await,
        Command::Implement(args) => implement_command(args).await,
        Command::Lifecycle(args) => lifecycle_command(args).await,
        Command::Status(args) => status_command(args).await,
        Command::Cancel(args) => cancel_command(args).await,
        Command::Beads(args) => beads_command(args).await,
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
    verify_oya_service_unit().await?;
    wait_for_health(&args.ingress, 30).await?;
    wait_for_tcp_port("127.0.0.1", 9180, 30).await?;
    register_services(&args.service_url).await?;
    validate_registered_services().await?;
    println!("[oya] Runtime ready (fresh Restate + handlers registered)");
    println!("  Admin:   http://127.0.0.1:9070");
    println!("  Ingress: {}", args.ingress);
    println!("  Service: {}", args.service_url);
    Ok(())
}

async fn doctor_command(args: DoctorArgs) -> anyhow::Result<()> {
    let checks = vec![
        check_http_ok("restate_ingress", &format!("{}/restate/health", args.ingress), "200").await,
        check_tcp_open(
            "restate_admin",
            &args.admin,
            9070,
            "ensure Restate admin is running on configured host/port",
        )
        .await,
        check_tcp_open(
            "oya_service",
            &args.service_url,
            9180,
            "ensure oya.service is running and bound to configured host/port",
        )
        .await,
        check_restate_services().await,
        check_restate_deployments().await,
        check_moon_tasks().await,
        check_repo_detection().await,
    ];
    let ok = checks.iter().all(|item| item.pass);
    let report = DoctorReport { ok, checks };
    print_doctor_jsonl(&report)?;
    if ok {
        Ok(())
    } else {
        Err(anyhow::anyhow!("doctor checks failed"))
    }
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

async fn verify_oya_service_unit() -> anyhow::Result<()> {
    let unit = run_command_capture("systemctl", &["--user", "cat", "oya.service"], None).await?;
    let exec = extract_exec_start(&unit)
        .ok_or_else(|| anyhow::anyhow!("oya.service missing ExecStart"))?;
    let binary_ok =
        extract_exec_binary(exec).map(std::path::Path::new).is_some_and(std::path::Path::exists);
    if !binary_ok {
        return Err(anyhow::anyhow!("oya.service ExecStart binary not found: {exec}"));
    }
    if !is_valid_oya_exec_start(exec) {
        Err(anyhow::anyhow!(
            "oya.service ExecStart must run 'oya serve' on port 9180 (current: {exec})"
        ))
    } else {
        Ok(())
    }
}

fn extract_exec_start(unit_text: &str) -> Option<&str> {
    unit_text.lines().find_map(|line| line.strip_prefix("ExecStart="))
}

fn extract_exec_binary(exec_start: &str) -> Option<&str> {
    exec_start.split_whitespace().next()
}

fn is_valid_oya_exec_start(exec_start: &str) -> bool {
    exec_start.contains("oya") && exec_start.contains("serve") && exec_start.contains(":9180")
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

async fn wait_for_tcp_port(host: &str, port: u16, retries: u8) -> anyhow::Result<()> {
    for _ in 0..retries {
        if tokio::net::TcpStream::connect((host, port)).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow::anyhow!("tcp port check timed out: {host}:{port}"))
}

async fn register_services(service_url: &str) -> anyhow::Result<()> {
    run_command_capture("restate", &["deployments", "register", "--force", "-y", service_url], None)
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

fn parse_admin_url(value: &str) -> Result<String, String> {
    parse_url_with_expected_port(value, 9070, "admin")
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
    let output = run_command_outcome(command, args, workdir).await?;
    if output.success {
        Ok(output.stdout)
    } else {
        Err(anyhow::anyhow!("{command} failed: {}", output.stderr.trim()))
    }
}

struct CommandOutcome {
    success: bool,
    stdout: String,
    stderr: String,
}

async fn run_command_outcome(
    command: &str,
    args: &[&str],
    workdir: Option<&Path>,
) -> anyhow::Result<CommandOutcome> {
    let mut process = TokioCommand::new(command);
    process.args(args);
    if let Some(path) = workdir {
        process.current_dir(path);
    }
    let output = process
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run {command}: {error}"))?;
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("{command} output was not UTF-8: {error}"))?;
    let stderr = String::from_utf8(output.stderr)
        .map_err(|error| anyhow::anyhow!("{command} output was not UTF-8: {error}"))?;
    Ok(CommandOutcome { success: output.status.success(), stdout, stderr })
}

async fn check_http_ok(id: &str, url: &str, expected: &str) -> DoctorCheck {
    let result = Client::new().get(url).send().await;
    match result {
        Ok(response) => DoctorCheck {
            id: id.to_owned(),
            pass: response.status().is_success(),
            expected: expected.to_owned(),
            actual: response.status().to_string(),
            remediation: format!("verify service bound correctly for {url}"),
        },
        Err(error) => DoctorCheck {
            id: id.to_owned(),
            pass: false,
            expected: expected.to_owned(),
            actual: error.to_string(),
            remediation: format!("start runtime with `oya init` and recheck {url}"),
        },
    }
}

async fn check_tcp_open(
    id: &str,
    endpoint_url: &str,
    expected_port: u16,
    remediation: &str,
) -> DoctorCheck {
    match parse_host_port(endpoint_url, expected_port) {
        Ok((host, port)) => {
            let result = tokio::net::TcpStream::connect((host.as_str(), port)).await;
            let pass = result.is_ok();
            DoctorCheck {
                id: id.to_owned(),
                pass,
                expected: format!("tcp:{port} open ({endpoint_url})"),
                actual: if pass { "open".to_owned() } else { "closed".to_owned() },
                remediation: remediation.to_owned(),
            }
        }
        Err(error) => DoctorCheck {
            id: id.to_owned(),
            pass: false,
            expected: format!("valid URL with port {expected_port}"),
            actual: error,
            remediation: remediation.to_owned(),
        },
    }
}

fn parse_host_port(endpoint_url: &str, expected_port: u16) -> Result<(String, u16), String> {
    let parsed = url::Url::parse(endpoint_url).map_err(|error| error.to_string())?;
    let host = parsed.host_str().ok_or_else(|| "URL missing host".to_owned())?.to_owned();
    let port = parsed.port_or_known_default().ok_or_else(|| "URL missing port".to_owned())?;
    if port == expected_port {
        Ok((host, port))
    } else {
        Err(format!("expected port {expected_port}, found {port}"))
    }
}

async fn check_restate_services() -> DoctorCheck {
    let result = run_command_outcome("restate", &["services", "list"], None).await;
    match result {
        Ok(output) => {
            let pass = output.success && has_required_services(&output.stdout);
            DoctorCheck {
                id: "restate_services".to_owned(),
                pass,
                expected: "Oya,OyaMemory,OyaService present".to_owned(),
                actual: output.stdout.trim().to_owned(),
                remediation: "run `oya init` to register handlers".to_owned(),
            }
        }
        Err(error) => DoctorCheck {
            id: "restate_services".to_owned(),
            pass: false,
            expected: "restate services list succeeds".to_owned(),
            actual: error.to_string(),
            remediation: "install/verify `restate` CLI and runtime".to_owned(),
        },
    }
}

async fn check_restate_deployments() -> DoctorCheck {
    let result = run_command_outcome("restate", &["deployments", "list"], None).await;
    match result {
        Ok(output) => {
            let lines = output.stdout;
            let has_expected = lines.contains("http://127.0.0.1:9180/");
            let has_stale = lines.contains("http://oya:9180/")
                || lines.contains("http://127.0.0.1:8080/")
                || lines.contains("http://127.0.0.1:9090/");
            DoctorCheck {
                id: "restate_deployments".to_owned(),
                pass: output.success && has_expected && !has_stale,
                expected: "single active endpoint http://127.0.0.1:9180/".to_owned(),
                actual: lines.lines().take(4).collect::<Vec<_>>().join(" | "),
                remediation:
                    "remove stale endpoints with `restate deployments remove <id> --force -y`"
                        .to_owned(),
            }
        }
        Err(error) => DoctorCheck {
            id: "restate_deployments".to_owned(),
            pass: false,
            expected: "restate deployments list succeeds".to_owned(),
            actual: error.to_string(),
            remediation: "ensure restate admin endpoint is healthy".to_owned(),
        },
    }
}

async fn check_moon_tasks() -> DoctorCheck {
    let result = run_command_outcome("moon", &["query", "tasks"], None).await;
    match result {
        Ok(output) => {
            let pass =
                output.success && ["quick", "ci", "test"].iter().all(|v| output.stdout.contains(v));
            DoctorCheck {
                id: "moon_tasks".to_owned(),
                pass,
                expected: "moon tasks include quick, ci, test".to_owned(),
                actual: output.stdout.lines().take(6).collect::<Vec<_>>().join(" | "),
                remediation: "define required moon tasks in .moon/tasks/all.yml".to_owned(),
            }
        }
        Err(error) => DoctorCheck {
            id: "moon_tasks".to_owned(),
            pass: false,
            expected: "moon query tasks succeeds".to_owned(),
            actual: error.to_string(),
            remediation: "install moon and run from repo root".to_owned(),
        },
    }
}

async fn check_repo_detection() -> DoctorCheck {
    match detect_repo_slug().await {
        Ok(Some(slug)) => DoctorCheck {
            id: "repo_slug".to_owned(),
            pass: true,
            expected: "owner/repo slug".to_owned(),
            actual: slug,
            remediation: "none".to_owned(),
        },
        Ok(None) => DoctorCheck {
            id: "repo_slug".to_owned(),
            pass: false,
            expected: "owner/repo slug via gh".to_owned(),
            actual: "not detected".to_owned(),
            remediation: "authenticate gh or pass --repo explicitly".to_owned(),
        },
        Err(error) => DoctorCheck {
            id: "repo_slug".to_owned(),
            pass: false,
            expected: "owner/repo slug via gh".to_owned(),
            actual: error.to_string(),
            remediation: "fix gh auth/config".to_owned(),
        },
    }
}

fn print_doctor_jsonl(report: &DoctorReport) -> anyhow::Result<()> {
    for check in &report.checks {
        let payload = serde_json::json!({
            "type": "check",
            "id": check.id,
            "pass": check.pass,
            "expected": check.expected,
            "actual": check.actual,
            "remediation": check.remediation,
        });
        println!("{}", serde_json::to_string(&payload)?);
    }
    let failed = report
        .checks
        .iter()
        .filter(|item| !item.pass)
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let summary = serde_json::json!({
        "type": "summary",
        "ok": report.ok,
        "checks": report.checks.len(),
        "failed": failed,
    });
    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeadEntry {
    id: String,
    title: String,
    status: String,
    priority: u8,
    issue_type: String,
}

async fn beads_command(args: BeadsArgs) -> anyhow::Result<()> {
    let repo_root = find_repo_root()?;
    let beads_path = repo_root.join(".beads").join("issues.jsonl");
    let content = std::fs::read_to_string(&beads_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", beads_path.display(), e))?;
    let mut beads: Vec<BeadEntry> =
        content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();
    if args.ready {
        beads.retain(|b| b.status == "ready");
    }
    beads.sort_by(|a, b| b.priority.cmp(&a.priority));
    if args.json {
        let json = serde_json::to_string_pretty(&beads)?;
        println!("{json}");
    } else {
        for bead in &beads {
            println!("{} [{}/{}] {}", bead.id, bead.status, bead.priority, bead.title);
        }
    }
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
