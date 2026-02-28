#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::args::{
    BeadsArgs, CancelArgs, Command, ImplementArgs, InvokeArgs, LifecycleArgs, ServeArgs, StatusArgs,
};
use super::doctor::{print_doctor_jsonl, run_doctor_checks};
use super::init::init_command;
use super::repo::resolve_repo_slug;
use super::restate::{
    call_restate_json, call_restate_root_json, call_restate_service_json, call_restate_start,
    parse_json_payload, pick_ready_bead, run_capture_command, run_simple_command,
};
use crate::restate_oya::{
    BeadSyncRequest, CancelResponse, KeyRequest, LifecycleRequest, LifecycleStatusSnapshot,
    PipelineRequest, StartRequest,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct BeadEntry {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: u8,
    #[serde(alias = "type")]
    pub issue_type: String,
}

pub async fn dispatch_command(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Init(args) => init_command(&args.ingress, &args.service_url, args.down).await,
        Command::Doctor(args) => {
            doctor_command(&args.ingress, &args.admin, &args.service_url).await
        }
        Command::Serve(args) => serve_command(args).await,
        Command::Invoke(args) => invoke_command(args).await,
        Command::Implement(args) => implement_command(args).await,
        Command::Lifecycle(args) => lifecycle_command(args).await,
        Command::Status(args) => status_command(args).await,
        Command::Cancel(args) => cancel_command(args).await,
        Command::Beads(args) => beads_command(args).await,
    }
}

async fn doctor_command(ingress: &str, admin: &str, service_url: &str) -> anyhow::Result<()> {
    let report = run_doctor_checks(ingress, admin, service_url).await;
    print_doctor_jsonl(&report)?;
    if report.ok {
        Ok(())
    } else {
        Err(anyhow::anyhow!("doctor checks failed"))
    }
}

async fn serve_command(args: ServeArgs) -> anyhow::Result<()> {
    let bind = parse_socket_addr(args.bind)?;
    crate::restate_oya::serve(bind).await
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

async fn status_command(args: StatusArgs) -> anyhow::Result<()> {
    let key = args.key;
    let request = KeyRequest { key: key.clone() };
    let snapshot: LifecycleStatusSnapshot =
        call_restate_root_json(&args.ingress, "OyaService", "get_lifecycle", request).await?;
    if is_uninitialized_snapshot(&snapshot) {
        return Err(anyhow::anyhow!("not_found: lifecycle '{}' does not exist", key));
    }
    let formatted = serde_json::to_string_pretty(&snapshot)?;
    println!("{formatted}");
    Ok(())
}

pub(crate) fn is_uninitialized_snapshot(snapshot: &LifecycleStatusSnapshot) -> bool {
    snapshot.bead_id.is_none()
        && snapshot.steps.is_empty()
        && snapshot.gates.is_empty()
        && snapshot.discipline_gates.is_empty()
        && snapshot.state.is_none()
        && snapshot.pr_url.is_none()
        && !snapshot.done
        && snapshot.success.is_none()
        && snapshot.message.is_none()
        && snapshot.compensation_diagnostics.is_empty()
}

async fn cancel_command(args: CancelArgs) -> anyhow::Result<()> {
    let request = KeyRequest { key: args.key };
    let response: CancelResponse =
        call_restate_root_json(&args.ingress, "OyaService", "cancel", request).await?;
    let formatted = serde_json::to_string_pretty(&response)?;
    println!("{formatted}");
    Ok(())
}

async fn beads_command(args: BeadsArgs) -> anyhow::Result<()> {
    let mut beads = if args.ready {
        let raw = run_capture_command(&["ready", "--json"]).await?;
        decode_bead_entries(parse_json_payload(&raw)?)?
    } else {
        let beads_root = find_beads_root()?;
        let beads_path = beads_root.join(".beads").join("issues.jsonl");
        let content = std::fs::read_to_string(&beads_path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", beads_path.display(), e))?;
        content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
    };
    beads.sort_by(|a, b| a.priority.cmp(&b.priority));
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

fn find_beads_root() -> anyhow::Result<PathBuf> {
    let current = std::env::current_dir()?;
    for path in current.ancestors() {
        if path.join(".beads").join("issues.jsonl").is_file() {
            return Ok(Path::to_path_buf(path));
        }
        if path.join(".git").exists() {
            break;
        }
    }
    Err(anyhow::anyhow!("could not find .beads/issues.jsonl from current git repository"))
}

pub fn decode_bead_entries(payload: serde_json::Value) -> anyhow::Result<Vec<BeadEntry>> {
    match payload {
        serde_json::Value::Array(_) => serde_json::from_value(payload).map_err(Into::into),
        serde_json::Value::Object(mut obj) => match obj.remove("items") {
            Some(items) => serde_json::from_value(items).map_err(Into::into),
            None => Err(anyhow::anyhow!("br ready --json returned object payload without `items`")),
        },
        _ => Err(anyhow::anyhow!("br ready --json returned unsupported JSON payload")),
    }
}

fn parse_socket_addr(value: String) -> anyhow::Result<SocketAddr> {
    value
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("invalid --bind '{}': {error}", value))
}
