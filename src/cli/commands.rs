#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::args::{
    BeadsArgs, CancelArgs, Command, ImplementArgs, InvokeArgs, LifecycleArgs, ServeArgs, StatusArgs,
};
use super::doctor::{print_doctor_jsonl, run_doctor_checks};
use super::evidence::evidence_command;
use super::explain::explain_command;
use super::init::init_command;
use super::repo::resolve_repo_slug;
use super::report::report_command;
use super::restate::{
    call_restate_json, call_restate_root_json, call_restate_service_json, call_restate_start,
    parse_json_payload, pick_ready_bead, run_capture_command, run_capture_command_in,
    run_simple_command,
};
use super::run::run_command;
use super::verify::verify_command;
use crate::lifecycle::state::StateDb;
use crate::lifecycle::types::{EvidenceEnvelope, EvidenceKind, RunId};
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
        Command::Run(args) => run_command(args).await,
        Command::Evidence(args) => evidence_command(args),
        Command::Verify(args) => verify_command(args).await,
        Command::Explain(args) => explain_command(args),
        Command::Report(args) => report_command(args),
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
    let data_dir = std::env::var("OYA_DATA_DIR").unwrap_or_else(|_| ".oya-lite".to_owned());
    let db = crate::lifecycle::state::StateDb::open(data_dir)?;
    crate::restate_oya::init_state_db(db);
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
    match call_restate_service_json(&args.ingress, "Oya", &workflow_key, "run", request).await {
        Ok(body) => {
            println!("{}", body.output);
            Ok(())
        }
        Err(error) if is_already_invoked_error_text(&error.to_string()) => {
            println!("lifecycle already running for key '{}'", workflow_key);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn is_already_invoked_error_text(message: &str) -> bool {
    message.contains("409 Conflict") && message.contains("already invoked")
}

async fn status_command(args: StatusArgs) -> anyhow::Result<()> {
    if let Some(run_id) = args.run_id {
        return status_by_run_id(&run_id);
    }
    let Some(key) = args.key else {
        return Err(anyhow::anyhow!("status requires --key or --run-id"));
    };
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

fn status_by_run_id(input: &str) -> anyhow::Result<()> {
    let run_id = RunId::parse(input)?;
    let db = StateDb::open(data_dir())?;
    let evidence = db.load_evidence(&run_id)?;
    let snapshot = run_status_snapshot(&run_id, evidence.as_slice())?;
    let formatted = serde_json::to_string_pretty(&snapshot)?;
    println!("{formatted}");
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RunStatusSnapshot {
    run_id: String,
    bead_id: String,
    phase: String,
    status: String,
    evidence_records: usize,
    last_evidence_record: EvidenceRecordSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct EvidenceRecordSummary {
    record_id: String,
    kind: String,
    timestamp: String,
    checksum: String,
    previous_checksum: Option<String>,
}

fn run_status_snapshot(
    run_id: &RunId,
    evidence: &[EvidenceEnvelope],
) -> anyhow::Result<RunStatusSnapshot> {
    let Some(last) = evidence.last() else {
        return Err(anyhow::anyhow!("not_found: run '{}' has no evidence records", run_id));
    };
    Ok(RunStatusSnapshot {
        run_id: run_id.as_str().to_owned(),
        bead_id: last.bead_id.as_str().to_owned(),
        phase: run_phase(&last.kind).to_owned(),
        status: run_status(&last.kind).to_owned(),
        evidence_records: evidence.len(),
        last_evidence_record: EvidenceRecordSummary::from_envelope(last),
    })
}

impl EvidenceRecordSummary {
    fn from_envelope(envelope: &EvidenceEnvelope) -> Self {
        Self {
            record_id: envelope.record_id.as_str().to_owned(),
            kind: evidence_kind_name(&envelope.kind).to_owned(),
            timestamp: envelope.timestamp.to_rfc3339(),
            checksum: envelope.checksum.as_str().to_owned(),
            previous_checksum: envelope
                .previous_checksum
                .as_ref()
                .map(|checksum| checksum.as_str().to_owned()),
        }
    }
}

fn run_phase(kind: &EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::RunStarted => "started",
        EvidenceKind::PromptRecord => "prompt_recorded",
        EvidenceKind::GateRunStarted => "gate_running",
        EvidenceKind::GateRunFinished => "gate_finished",
        EvidenceKind::Finding => "finding_recorded",
        EvidenceKind::RepairRequest => "repair_requested",
        EvidenceKind::RepairAttempt => "repair_attempt_recorded",
        EvidenceKind::RepairBlocked => "repair_blocked",
        EvidenceKind::AgentRequest => "agent_requested",
        EvidenceKind::AgentRun => "agent_ran",
        EvidenceKind::VcsSyncFailed => "blocked",
        EvidenceKind::DiffValidationFailed => "blocked",
        EvidenceKind::PullRequestCreated => "completed",
        EvidenceKind::PullRequestFailed => "blocked",
    }
}

fn run_status(kind: &EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::RunStarted | EvidenceKind::PromptRecord => "blocked",
        EvidenceKind::Finding => "blocked",
        EvidenceKind::RepairBlocked => "blocked",
        EvidenceKind::VcsSyncFailed => "blocked",
        EvidenceKind::DiffValidationFailed => "blocked",
        EvidenceKind::PullRequestFailed => "blocked",
        EvidenceKind::RepairRequest | EvidenceKind::RepairAttempt => "repairing",
        EvidenceKind::GateRunStarted | EvidenceKind::AgentRequest => "running",
        EvidenceKind::GateRunFinished | EvidenceKind::AgentRun => "recorded",
        EvidenceKind::PullRequestCreated => "completed",
    }
}

fn evidence_kind_name(kind: &EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::RunStarted => "run_started",
        EvidenceKind::PromptRecord => "prompt_record",
        EvidenceKind::GateRunStarted => "gate_run_started",
        EvidenceKind::GateRunFinished => "gate_run_finished",
        EvidenceKind::Finding => "finding",
        EvidenceKind::RepairRequest => "repair_request",
        EvidenceKind::RepairAttempt => "repair_attempt",
        EvidenceKind::RepairBlocked => "repair_blocked",
        EvidenceKind::AgentRequest => "agent_request",
        EvidenceKind::AgentRun => "agent_run",
        EvidenceKind::VcsSyncFailed => "vcs_sync_failed",
        EvidenceKind::DiffValidationFailed => "diff_validation_failed",
        EvidenceKind::PullRequestCreated => "pull_request_created",
        EvidenceKind::PullRequestFailed => "pull_request_failed",
    }
}

fn data_dir() -> PathBuf {
    match std::env::var("OYA_DATA_DIR") {
        Ok(value) => PathBuf::from(value),
        Err(_) => PathBuf::from(".oya-lite"),
    }
}

pub(crate) fn is_uninitialized_snapshot(snapshot: &LifecycleStatusSnapshot) -> bool {
    snapshot.bead_id.is_none()
        && snapshot.steps.is_empty()
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
        let beads_root = find_beads_root()?;
        let raw = run_capture_command_in(&["ready", "--json"], Some(beads_root.as_path())).await?;
        decode_bead_entries(parse_json_payload(&raw)?)?
    } else {
        let beads_root = find_beads_root()?;
        let beads_path = beads_root.join(".beads").join("issues.jsonl");
        let content = std::fs::read_to_string(&beads_path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", beads_path.display(), e))?;
        content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
    };
    beads.sort_by_key(|bead| bead.priority);
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::lifecycle::types::{
        BeadId, EvidenceEnvelopeParts, EvidenceMetadata, EvidenceRecordId,
    };
    use chrono::{TimeZone, Utc};

    #[test]
    fn run_status_snapshot_reports_phase_and_last_evidence_record() {
        let first = evidence_envelope("ev-demo-run-started-001", EvidenceKind::RunStarted, None);
        let second = evidence_envelope(
            "ev-demo-prompt-record-002",
            EvidenceKind::PromptRecord,
            Some(first.checksum.clone()),
        );
        let first_checksum = first.checksum.as_str().to_owned();
        let run_id = RunId::parse("run-demo").unwrap();

        let snapshot = run_status_snapshot(&run_id, &[first, second.clone()]).unwrap();

        assert_eq!(snapshot.run_id, "run-demo");
        assert_eq!(snapshot.bead_id, "demo");
        assert_eq!(snapshot.phase, "prompt_recorded");
        assert_eq!(snapshot.status, "blocked");
        assert_eq!(snapshot.evidence_records, 2);
        assert_eq!(snapshot.last_evidence_record.record_id, second.record_id.as_str());
        assert_eq!(snapshot.last_evidence_record.kind, "prompt_record");
        assert_eq!(snapshot.last_evidence_record.previous_checksum, Some(first_checksum));
    }

    #[test]
    fn run_status_snapshot_rejects_missing_evidence() {
        let run_id = RunId::parse("run-missing").unwrap();

        let snapshot = run_status_snapshot(&run_id, &[]);

        assert!(snapshot.is_err());
    }

    fn evidence_envelope(
        record_id: &str,
        kind: EvidenceKind,
        previous_checksum: Option<crate::lifecycle::types::EvidenceChecksum>,
    ) -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse(record_id).unwrap(),
            run_id: RunId::parse("run-demo").unwrap(),
            bead_id: BeadId::parse("demo").unwrap(),
            timestamp: Utc.timestamp_opt(1_779_999_600, 0).unwrap(),
            kind,
            metadata: EvidenceMetadata::new(),
            previous_checksum,
        })
        .unwrap()
    }
}
