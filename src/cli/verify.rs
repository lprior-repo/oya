#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{ExitStatus, Output};
use tokio::process::Command;

use super::args::VerifyArgs;
use crate::lifecycle::state::StateDb;
use crate::lifecycle::types::{
    BeadId, EvidenceChecksum, EvidenceEnvelope, EvidenceEnvelopeParts, EvidenceKind,
    EvidenceMetadata, EvidenceRecordId, GateFailureCategory, GateId, GateModel,
    RepairMutationScope, RunId,
};

const GATE_OUTPUT_LIMIT_BYTES: usize = 4096;
const REPAIR_PROMPT_LIMIT_BYTES: usize = 1200;
const REPAIR_PROMPT_OUTPUT_LIMIT_BYTES: usize = 480;
const REDACTED_OUTPUT_LINE: &str = "[redacted]";
const REPAIR_BUDGET_EXHAUSTED: &str = "RepairBudgetExhausted";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyOutput {
    pub run_id: String,
    pub bead_id: String,
    pub gate: String,
    pub moon_task: String,
    pub status: String,
    pub failure_category: Option<String>,
    pub finding_id: Option<String>,
    pub exit_code: Option<i32>,
    pub blocks_on_failure: bool,
    pub stdout: GateOutputSummary,
    pub stderr: GateOutputSummary,
    pub evidence_records: usize,
    pub last_record_id: String,
    pub last_checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyRepairOutput {
    pub run_id: String,
    pub bead_id: String,
    pub gate: String,
    pub status: String,
    pub repair_request_id: String,
    pub repair_attempt_id: String,
    pub repair_blocked_id: Option<String>,
    pub block_reason: Option<String>,
    pub finding_id: String,
    pub scope: String,
    pub mutation_scope: String,
    pub mutation_policy: String,
    pub retry_count: String,
    pub repair_prompt: String,
    pub reverification_status: String,
    pub required_gates: Vec<String>,
    pub reverification_gates: Vec<ReverificationGateOutput>,
    pub evidence_records: usize,
    pub last_checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReverificationGateOutput {
    pub gate: String,
    pub moon_task: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub gate_run_started_id: String,
    pub gate_run_finished_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunVerificationResult {
    pub(crate) gate: String,
    pub(crate) moon_task: String,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) gate_run_started_id: String,
    pub(crate) gate_run_finished_id: String,
    pub(crate) finding_id: Option<String>,
    pub(crate) last_record_id: String,
    pub(crate) last_checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOutputSummary {
    pub original_bytes: usize,
    pub stored_bytes: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateCommandResult {
    status: ExitStatus,
    stdout: BoundedGateOutput,
    stderr: BoundedGateOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundedGateOutput {
    preview: String,
    original_bytes: usize,
    stored_bytes: usize,
    truncated: bool,
}

#[derive(Debug, Clone, Copy)]
struct VerifyEvidence<'a> {
    last_record: &'a EvidenceEnvelope,
    evidence_records: usize,
    finding: Option<&'a EvidenceEnvelope>,
}

#[derive(Debug, Clone)]
struct ReverificationGateEvidence {
    model: GateModel,
    result: GateCommandResult,
    started: EvidenceEnvelope,
    finished: EvidenceEnvelope,
    finding: Option<EvidenceEnvelope>,
}

pub async fn verify_command(args: VerifyArgs) -> anyhow::Result<()> {
    let db = StateDb::open(data_dir())?;
    let bead_id = BeadId::parse(&args.bead_id)?;
    let gate_id = GateId::parse(&args.gate)?;
    let model = gate_id.model();
    if args.repair {
        return repair_command(&db, &bead_id, &model, Utc::now()).await;
    }
    let started = persist_gate_started(&db, &bead_id, &model, Utc::now())?;
    let result = run_moon_gate(&model).await?;
    let finished = persist_gate_finished(&db, &model, &started, &result, Utc::now())?;
    let finding = persist_finding_if_failed(&db, &model, &finished, &result, Utc::now())?;
    let evidence_records = db.load_evidence(&RunId::from_bead_id(&bead_id))?.len();
    let last_record = finding.as_ref().map_or(&finished, |record| record);
    let evidence = VerifyEvidence { last_record, evidence_records, finding: finding.as_ref() };
    let output = VerifyOutput::from_evidence(&bead_id, &model, &result, evidence);
    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");
    if result.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(gate_failure_message(&output)))
    }
}

pub(crate) async fn persist_run_verification_gate(
    db: &StateDb,
    bead_id: &BeadId,
    model: &GateModel,
) -> anyhow::Result<RunVerificationResult> {
    let result = run_moon_gate(model).await?;
    let evidence = persist_reverification_gate_result(db, bead_id, *model, result, Utc::now())?;
    Ok(RunVerificationResult::from_evidence(&evidence))
}

#[cfg(test)]
pub(crate) fn persist_synthetic_run_verification_gate(
    db: &StateDb,
    bead_id: &BeadId,
    model: &GateModel,
    status: ExitStatus,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<RunVerificationResult> {
    let result = GateCommandResult {
        status,
        stdout: BoundedGateOutput::from_bytes(b""),
        stderr: BoundedGateOutput::from_bytes(b""),
    };
    let evidence = persist_reverification_gate_result(db, bead_id, *model, result, timestamp)?;
    Ok(RunVerificationResult::from_evidence(&evidence))
}

async fn repair_command(
    db: &StateDb,
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<()> {
    if let Some(blocked) = persist_blocked_if_budget_exhausted(db, bead_id, model, timestamp)? {
        let evidence_records = db.load_evidence(&RunId::from_bead_id(bead_id))?.len();
        let output = VerifyRepairOutput::from_blocked(&blocked, evidence_records)?;
        let json = serde_json::to_string_pretty(&output)?;
        println!("{json}");
        return Err(anyhow::anyhow!(repair_blocked_message(&output)));
    }
    let request = persist_repair_request(db, bead_id, model, timestamp)?;
    let evidence_before_attempt = db.load_evidence(&RunId::from_bead_id(bead_id))?;
    let required_gates = required_reverification_gates(evidence_before_attempt.as_slice(), model)?;
    let attempt = persist_repair_attempt(db, &request, required_gates.as_slice(), Utc::now())?;
    let reruns = persist_reverification_gates(db, bead_id, required_gates.as_slice()).await?;
    let evidence_records = db.load_evidence(&RunId::from_bead_id(bead_id))?.len();
    let output =
        VerifyRepairOutput::from_repair(&request, &attempt, reruns.as_slice(), evidence_records)?;
    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");
    if output.reverification_status == "passed" {
        Ok(())
    } else {
        Err(anyhow::anyhow!("repair reverification failed"))
    }
}

fn persist_repair_request(
    db: &StateDb,
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceEnvelope> {
    let run_id = RunId::from_bead_id(bead_id);
    let evidence = db.load_evidence(&run_id)?;
    let Some(finding) = latest_finding_for_gate(evidence.as_slice(), model) else {
        return Err(anyhow::anyhow!(
            "not_found: finding for gate '{}' does not exist",
            model.id.as_str()
        ));
    };
    let Some(previous) = evidence.last() else {
        return Err(anyhow::anyhow!("not_found: run '{}' has no evidence records", run_id));
    };
    let envelope = repair_request_envelope(
        model,
        next_timestamp_after(Some(previous.timestamp), timestamp)?,
        finding,
        previous,
    )?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

fn persist_repair_attempt(
    db: &StateDb,
    request: &EvidenceEnvelope,
    required_gates: &[GateModel],
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceEnvelope> {
    let envelope = repair_attempt_envelope(
        next_timestamp_after(Some(request.timestamp), timestamp)?,
        request,
        required_gates,
    )?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

fn persist_blocked_if_budget_exhausted(
    db: &StateDb,
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<Option<EvidenceEnvelope>> {
    let run_id = RunId::from_bead_id(bead_id);
    let evidence = db.load_evidence(&run_id)?;
    let Some(attempt) = exhausted_repair_attempt(evidence.as_slice(), model) else {
        return Ok(None);
    };
    let Some(previous) = evidence.last() else {
        return Err(anyhow::anyhow!("not_found: run '{}' has no evidence records", run_id));
    };
    let envelope = repair_blocked_envelope(
        model,
        next_timestamp_after(Some(previous.timestamp), timestamp)?,
        attempt,
        previous,
    )?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(Some(envelope))
}

fn persist_gate_started(
    db: &StateDb,
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceEnvelope> {
    let run_id = RunId::from_bead_id(bead_id);
    let existing = db.load_evidence(&run_id)?;
    let previous = existing.last();
    let envelope = gate_started_envelope(
        bead_id,
        &run_id,
        model,
        next_timestamp_after(previous.map(|record| record.timestamp), timestamp)?,
        previous.map(|record| record.checksum.clone()),
    )?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

fn persist_gate_finished(
    db: &StateDb,
    model: &GateModel,
    started: &EvidenceEnvelope,
    result: &GateCommandResult,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceEnvelope> {
    let envelope = gate_finished_envelope(
        model,
        result,
        next_timestamp_after(Some(started.timestamp), timestamp)?,
        started,
    )?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

fn persist_finding_if_failed(
    db: &StateDb,
    model: &GateModel,
    finished: &EvidenceEnvelope,
    result: &GateCommandResult,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<Option<EvidenceEnvelope>> {
    if result.status.success() {
        return Ok(None);
    }
    let envelope = finding_envelope(
        model,
        result,
        next_timestamp_after(Some(finished.timestamp), timestamp)?,
        finished,
    )?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(Some(envelope))
}

async fn persist_reverification_gates(
    db: &StateDb,
    bead_id: &BeadId,
    required_gates: &[GateModel],
) -> anyhow::Result<Vec<ReverificationGateEvidence>> {
    let mut records = Vec::with_capacity(required_gates.len());
    for model in required_gates {
        let result = run_moon_gate(model).await?;
        records.push(persist_reverification_gate_result(db, bead_id, *model, result, Utc::now())?);
    }
    Ok(records)
}

fn persist_reverification_gate_result(
    db: &StateDb,
    bead_id: &BeadId,
    model: GateModel,
    result: GateCommandResult,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<ReverificationGateEvidence> {
    let started = persist_gate_started(db, bead_id, &model, timestamp)?;
    let finished = persist_gate_finished(db, &model, &started, &result, Utc::now())?;
    let finding = persist_finding_if_failed(db, &model, &finished, &result, Utc::now())?;
    Ok(ReverificationGateEvidence { model, result, started, finished, finding })
}

fn required_reverification_gates(
    evidence: &[EvidenceEnvelope],
    failed_model: &GateModel,
) -> anyhow::Result<Vec<GateModel>> {
    let passed_gates = evidence
        .iter()
        .filter(|record| record.kind == EvidenceKind::GateRunFinished)
        .filter(|record| metadata_is(record, "status", "passed"))
        .filter(|record| metadata_is(record, "blocks_on_failure", "true"))
        .map(passed_gate_model)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut gates = vec![*failed_model];
    passed_gates.into_iter().for_each(|model| push_unique_gate(&mut gates, model));
    Ok(gates)
}

fn passed_gate_model(record: &EvidenceEnvelope) -> anyhow::Result<GateModel> {
    let gate = metadata_field(record, "gate")?;
    GateId::parse(&gate).map(|gate_id| gate_id.model()).map_err(Into::into)
}

fn metadata_is(record: &EvidenceEnvelope, key: &str, expected: &str) -> bool {
    record.metadata.get(key).map(|value| value.as_str()) == Some(expected)
}

fn push_unique_gate(gates: &mut Vec<GateModel>, model: GateModel) {
    if gates.iter().all(|existing| existing.id != model.id) {
        gates.push(model);
    }
}

async fn run_moon_gate(model: &GateModel) -> anyhow::Result<GateCommandResult> {
    Command::new("moon")
        .arg("run")
        .arg(model.moon_task)
        .output()
        .await
        .map(GateCommandResult::from_output)
        .map_err(|error| anyhow::anyhow!("failed to run moon gate '{}': {error}", model.moon_task))
}

impl GateCommandResult {
    fn from_output(output: Output) -> Self {
        Self {
            status: output.status,
            stdout: BoundedGateOutput::from_bytes(&output.stdout),
            stderr: BoundedGateOutput::from_bytes(&output.stderr),
        }
    }
}

impl BoundedGateOutput {
    fn from_bytes(bytes: &[u8]) -> Self {
        let preview = bounded_redacted_preview(bytes);
        Self {
            preview: preview.clone(),
            original_bytes: bytes.len(),
            stored_bytes: preview.len(),
            truncated: bytes.len() > GATE_OUTPUT_LIMIT_BYTES,
        }
    }

    fn summary(&self) -> GateOutputSummary {
        GateOutputSummary {
            original_bytes: self.original_bytes,
            stored_bytes: self.stored_bytes,
            truncated: self.truncated,
        }
    }
}

impl VerifyOutput {
    fn from_evidence(
        bead_id: &BeadId,
        model: &GateModel,
        result: &GateCommandResult,
        evidence: VerifyEvidence<'_>,
    ) -> Self {
        Self {
            run_id: evidence.last_record.run_id.as_str().to_owned(),
            bead_id: bead_id.as_str().to_owned(),
            gate: model.id.as_str().to_owned(),
            moon_task: model.moon_task.to_owned(),
            status: gate_status(&result.status).to_owned(),
            failure_category: output_failure_category(model, &result.status),
            finding_id: evidence.finding.map(|record| record.record_id.as_str().to_owned()),
            exit_code: result.status.code(),
            blocks_on_failure: model.blocks_on_failure,
            stdout: result.stdout.summary(),
            stderr: result.stderr.summary(),
            evidence_records: evidence.evidence_records,
            last_record_id: evidence.last_record.record_id.as_str().to_owned(),
            last_checksum: evidence.last_record.checksum.as_str().to_owned(),
        }
    }
}

impl VerifyRepairOutput {
    fn from_repair(
        request: &EvidenceEnvelope,
        attempt: &EvidenceEnvelope,
        reruns: &[ReverificationGateEvidence],
        evidence_records: usize,
    ) -> anyhow::Result<Self> {
        let required_gates = required_gate_names_from_attempt(attempt);
        let last_record = last_reverification_record(attempt, reruns);
        Ok(Self {
            run_id: request.run_id.as_str().to_owned(),
            bead_id: request.bead_id.as_str().to_owned(),
            gate: metadata_field(request, "gate")?,
            status: metadata_field(attempt, "status")?,
            repair_request_id: request.record_id.as_str().to_owned(),
            repair_attempt_id: attempt.record_id.as_str().to_owned(),
            repair_blocked_id: None,
            block_reason: None,
            finding_id: metadata_field(request, "finding_record_id")?,
            scope: metadata_field(request, "scope")?,
            mutation_scope: metadata_field(request, "mutation_scope")?,
            mutation_policy: metadata_field(request, "mutation_policy")?,
            retry_count: metadata_field(request, "retry_count")?,
            repair_prompt: metadata_field(request, "repair_prompt")?,
            reverification_status: reverification_status(reruns).to_owned(),
            required_gates,
            reverification_gates: reruns
                .iter()
                .map(ReverificationGateOutput::from_evidence)
                .collect(),
            evidence_records,
            last_checksum: last_record.checksum.as_str().to_owned(),
        })
    }

    fn from_blocked(blocked: &EvidenceEnvelope, evidence_records: usize) -> anyhow::Result<Self> {
        Ok(Self {
            run_id: blocked.run_id.as_str().to_owned(),
            bead_id: blocked.bead_id.as_str().to_owned(),
            gate: metadata_field(blocked, "gate")?,
            status: metadata_field(blocked, "status")?,
            repair_request_id: metadata_field(blocked, "repair_request_id")?,
            repair_attempt_id: metadata_field(blocked, "repair_attempt_id")?,
            repair_blocked_id: Some(blocked.record_id.as_str().to_owned()),
            block_reason: Some(metadata_field(blocked, "block_reason")?),
            finding_id: metadata_field(blocked, "finding_record_id")?,
            scope: metadata_field(blocked, "scope")?,
            mutation_scope: metadata_field(blocked, "mutation_scope")?,
            mutation_policy: metadata_field(blocked, "mutation_policy")?,
            retry_count: metadata_field(blocked, "retry_count")?,
            repair_prompt: metadata_field(blocked, "repair_prompt")?,
            reverification_status: "blocked".to_owned(),
            required_gates: required_gate_names_from_attempt(blocked),
            reverification_gates: Vec::new(),
            evidence_records,
            last_checksum: blocked.checksum.as_str().to_owned(),
        })
    }
}

impl ReverificationGateOutput {
    fn from_evidence(evidence: &ReverificationGateEvidence) -> Self {
        Self {
            gate: evidence.model.id.as_str().to_owned(),
            moon_task: evidence.model.moon_task.to_owned(),
            status: gate_status(&evidence.result.status).to_owned(),
            exit_code: evidence.result.status.code(),
            gate_run_started_id: evidence.started.record_id.as_str().to_owned(),
            gate_run_finished_id: evidence.finished.record_id.as_str().to_owned(),
        }
    }
}

impl RunVerificationResult {
    fn from_evidence(evidence: &ReverificationGateEvidence) -> Self {
        let last_record = evidence.last_record();
        Self {
            gate: evidence.model.id.as_str().to_owned(),
            moon_task: evidence.model.moon_task.to_owned(),
            status: gate_status(&evidence.result.status).to_owned(),
            exit_code: evidence.result.status.code(),
            gate_run_started_id: evidence.started.record_id.as_str().to_owned(),
            gate_run_finished_id: evidence.finished.record_id.as_str().to_owned(),
            finding_id: evidence
                .finding
                .as_ref()
                .map(|record| record.record_id.as_str().to_owned()),
            last_record_id: last_record.record_id.as_str().to_owned(),
            last_checksum: last_record.checksum.as_str().to_owned(),
        }
    }
}

impl ReverificationGateEvidence {
    fn last_record(&self) -> &EvidenceEnvelope {
        match &self.finding {
            Some(finding) => finding,
            None => &self.finished,
        }
    }
}

fn reverification_status(reruns: &[ReverificationGateEvidence]) -> &'static str {
    if reruns.iter().all(|rerun| rerun.result.status.success()) {
        "passed"
    } else {
        "failed"
    }
}

fn last_reverification_record<'a>(
    attempt: &'a EvidenceEnvelope,
    reruns: &'a [ReverificationGateEvidence],
) -> &'a EvidenceEnvelope {
    match reruns.last() {
        Some(rerun) => rerun.last_record(),
        None => attempt,
    }
}

fn required_gate_names_from_attempt(attempt: &EvidenceEnvelope) -> Vec<String> {
    match attempt.metadata.get("required_gates") {
        Some(value) => {
            value.split(',').filter(|gate| !gate.is_empty()).map(ToOwned::to_owned).collect()
        }
        None => Vec::new(),
    }
}

fn metadata_field(envelope: &EvidenceEnvelope, key: &str) -> anyhow::Result<String> {
    envelope
        .metadata
        .get(key)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("repair request metadata missing '{key}'"))
}

fn gate_started_envelope(
    bead_id: &BeadId,
    run_id: &RunId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
    previous_checksum: Option<EvidenceChecksum>,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: gate_started_record_id(bead_id, model, timestamp)?,
        run_id: run_id.clone(),
        bead_id: bead_id.clone(),
        timestamp,
        kind: EvidenceKind::GateRunStarted,
        metadata: gate_started_metadata(model),
        previous_checksum,
    })
    .map_err(Into::into)
}

fn gate_finished_envelope(
    model: &GateModel,
    result: &GateCommandResult,
    timestamp: DateTime<Utc>,
    started: &EvidenceEnvelope,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: gate_finished_record_id(&started.bead_id, model, timestamp)?,
        run_id: started.run_id.clone(),
        bead_id: started.bead_id.clone(),
        timestamp,
        kind: EvidenceKind::GateRunFinished,
        metadata: gate_finished_metadata(model, result),
        previous_checksum: Some(started.checksum.clone()),
    })
    .map_err(Into::into)
}

fn finding_envelope(
    model: &GateModel,
    result: &GateCommandResult,
    timestamp: DateTime<Utc>,
    finished: &EvidenceEnvelope,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: finding_record_id(&finished.bead_id, model, timestamp)?,
        run_id: finished.run_id.clone(),
        bead_id: finished.bead_id.clone(),
        timestamp,
        kind: EvidenceKind::Finding,
        metadata: finding_metadata(model, result, finished),
        previous_checksum: Some(finished.checksum.clone()),
    })
    .map_err(Into::into)
}

fn repair_request_envelope(
    model: &GateModel,
    timestamp: DateTime<Utc>,
    finding: &EvidenceEnvelope,
    previous: &EvidenceEnvelope,
) -> anyhow::Result<EvidenceEnvelope> {
    let record_id = repair_request_record_id(&finding.bead_id, model, timestamp)?;
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        metadata: repair_request_metadata(model, finding, record_id.as_str()),
        record_id,
        run_id: finding.run_id.clone(),
        bead_id: finding.bead_id.clone(),
        timestamp,
        kind: EvidenceKind::RepairRequest,
        previous_checksum: Some(previous.checksum.clone()),
    })
    .map_err(Into::into)
}

fn repair_attempt_envelope(
    timestamp: DateTime<Utc>,
    request: &EvidenceEnvelope,
    required_gates: &[GateModel],
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: repair_attempt_record_id(&request.bead_id, request, timestamp)?,
        run_id: request.run_id.clone(),
        bead_id: request.bead_id.clone(),
        timestamp,
        kind: EvidenceKind::RepairAttempt,
        metadata: repair_attempt_metadata(request, required_gates),
        previous_checksum: Some(request.checksum.clone()),
    })
    .map_err(Into::into)
}

fn repair_blocked_envelope(
    model: &GateModel,
    timestamp: DateTime<Utc>,
    attempt: &EvidenceEnvelope,
    previous: &EvidenceEnvelope,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: repair_blocked_record_id(&attempt.bead_id, model, timestamp)?,
        run_id: attempt.run_id.clone(),
        bead_id: attempt.bead_id.clone(),
        timestamp,
        kind: EvidenceKind::RepairBlocked,
        metadata: repair_blocked_metadata(model, attempt),
        previous_checksum: Some(previous.checksum.clone()),
    })
    .map_err(Into::into)
}

fn gate_started_metadata(model: &GateModel) -> EvidenceMetadata {
    EvidenceMetadata::from([
        ("blocks_on_failure".to_owned(), model.blocks_on_failure.to_string()),
        ("gate".to_owned(), model.id.as_str().to_owned()),
        ("moon_task".to_owned(), model.moon_task.to_owned()),
        ("phase".to_owned(), "started".to_owned()),
    ])
}

fn gate_finished_metadata(model: &GateModel, result: &GateCommandResult) -> EvidenceMetadata {
    [
        ("blocks_on_failure".to_owned(), model.blocks_on_failure.to_string()),
        ("exit_code".to_owned(), exit_code_metadata(&result.status)),
        ("failure_category".to_owned(), failure_category_metadata(model, &result.status)),
        ("gate".to_owned(), model.id.as_str().to_owned()),
        ("moon_task".to_owned(), model.moon_task.to_owned()),
        ("phase".to_owned(), "finished".to_owned()),
        ("status".to_owned(), gate_status(&result.status).to_owned()),
    ]
    .into_iter()
    .chain(bounded_output_metadata("stdout", &result.stdout))
    .chain(bounded_output_metadata("stderr", &result.stderr))
    .collect()
}

fn bounded_output_metadata(prefix: &str, output: &BoundedGateOutput) -> [(String, String); 5] {
    [
        (format!("{prefix}_original_bytes"), output.original_bytes.to_string()),
        (format!("{prefix}_stored_bytes"), output.stored_bytes.to_string()),
        (format!("{prefix}_truncated"), output.truncated.to_string()),
        (format!("{prefix}_limit_bytes"), GATE_OUTPUT_LIMIT_BYTES.to_string()),
        (format!("{prefix}_preview"), output.preview.clone()),
    ]
}

fn finding_metadata(
    model: &GateModel,
    result: &GateCommandResult,
    finished: &EvidenceEnvelope,
) -> EvidenceMetadata {
    [
        ("category".to_owned(), model.failure_category.as_str().to_owned()),
        ("exit_code".to_owned(), exit_code_metadata(&result.status)),
        ("gate".to_owned(), model.id.as_str().to_owned()),
        ("gate_checksum".to_owned(), finished.checksum.as_str().to_owned()),
        ("gate_record_id".to_owned(), finished.record_id.as_str().to_owned()),
        ("kind".to_owned(), "gate_failure".to_owned()),
        ("moon_task".to_owned(), model.moon_task.to_owned()),
        ("next_action".to_owned(), finding_next_action(model).to_owned()),
        ("status".to_owned(), "open".to_owned()),
    ]
    .into_iter()
    .chain(bounded_output_metadata("stdout", &result.stdout))
    .chain(bounded_output_metadata("stderr", &result.stderr))
    .collect()
}

fn finding_next_action(model: &GateModel) -> &'static str {
    match model.failure_category {
        GateFailureCategory::Format => "run moon run oya:fmt-fix then rerun the gate",
        GateFailureCategory::Lint => "fix lint findings then rerun the gate",
        GateFailureCategory::Check => "fix check errors then rerun the gate",
        GateFailureCategory::Test => "fix failing tests then rerun the gate",
        GateFailureCategory::Build => "fix build failure then rerun the gate",
        GateFailureCategory::Audit => "review audit finding then rerun the gate",
        GateFailureCategory::Ci => "inspect failing ci task then rerun the gate",
    }
}

fn repair_request_metadata(
    model: &GateModel,
    finding: &EvidenceEnvelope,
    repair_request_id: &str,
) -> EvidenceMetadata {
    let base = repair_request_metadata_base(model, finding, repair_request_id);
    let prompt = repair_prompt_from_metadata(&base);
    base.into_iter().chain([("repair_prompt".to_owned(), prompt)]).collect()
}

fn repair_request_metadata_base(
    model: &GateModel,
    finding: &EvidenceEnvelope,
    repair_request_id: &str,
) -> EvidenceMetadata {
    let mutation_scope = repair_mutation_scope(finding);
    EvidenceMetadata::from([
        ("budget_remaining".to_owned(), "0".to_owned()),
        ("command".to_owned(), format!("moon run {}", model.moon_task)),
        ("failure_category".to_owned(), finding_metadata_value(finding, "category")),
        ("finding_checksum".to_owned(), finding.checksum.as_str().to_owned()),
        ("finding_record_id".to_owned(), finding.record_id.as_str().to_owned()),
        ("gate".to_owned(), model.id.as_str().to_owned()),
        ("moon_task".to_owned(), model.moon_task.to_owned()),
        ("mutation_policy".to_owned(), mutation_scope.policy_text().to_owned()),
        ("mutation_scope".to_owned(), mutation_scope.as_str().to_owned()),
        ("next_action".to_owned(), finding_metadata_value(finding, "next_action")),
        ("redacted".to_owned(), "true".to_owned()),
        ("repair_request_id".to_owned(), repair_request_id.to_owned()),
        ("retry_count".to_owned(), "1".to_owned()),
        ("scope".to_owned(), "gate".to_owned()),
        ("status".to_owned(), "requested".to_owned()),
        (
            "stderr_preview".to_owned(),
            repair_prompt_output_preview(&finding_metadata_value(finding, "stderr_preview")),
        ),
        (
            "stdout_preview".to_owned(),
            repair_prompt_output_preview(&finding_metadata_value(finding, "stdout_preview")),
        ),
    ])
}

fn repair_attempt_metadata(
    request: &EvidenceEnvelope,
    required_gates: &[GateModel],
) -> EvidenceMetadata {
    EvidenceMetadata::from([
        ("agent".to_owned(), "repair".to_owned()),
        (
            "budget_remaining".to_owned(),
            repair_metadata_value(&request.metadata, "budget_remaining"),
        ),
        (
            "finding_record_id".to_owned(),
            repair_metadata_value(&request.metadata, "finding_record_id"),
        ),
        ("gate".to_owned(), repair_metadata_value(&request.metadata, "gate")),
        ("mutation_policy".to_owned(), repair_metadata_value(&request.metadata, "mutation_policy")),
        ("mutation_scope".to_owned(), repair_metadata_value(&request.metadata, "mutation_scope")),
        ("redacted".to_owned(), "true".to_owned()),
        ("required_gates".to_owned(), required_gate_names(required_gates)),
        ("repair_request_checksum".to_owned(), request.checksum.as_str().to_owned()),
        ("repair_request_id".to_owned(), request.record_id.as_str().to_owned()),
        ("retry_count".to_owned(), repair_metadata_value(&request.metadata, "retry_count")),
        ("scope".to_owned(), repair_metadata_value(&request.metadata, "scope")),
        ("status".to_owned(), "attempt_recorded".to_owned()),
        ("stderr_preview".to_owned(), "".to_owned()),
        ("stdout_preview".to_owned(), "".to_owned()),
    ])
}

fn repair_blocked_metadata(model: &GateModel, attempt: &EvidenceEnvelope) -> EvidenceMetadata {
    EvidenceMetadata::from([
        ("agent".to_owned(), "repair".to_owned()),
        ("block_reason".to_owned(), REPAIR_BUDGET_EXHAUSTED.to_owned()),
        ("budget_remaining".to_owned(), "0".to_owned()),
        (
            "finding_record_id".to_owned(),
            repair_metadata_value(&attempt.metadata, "finding_record_id"),
        ),
        ("gate".to_owned(), model.id.as_str().to_owned()),
        ("moon_task".to_owned(), model.moon_task.to_owned()),
        ("mutation_policy".to_owned(), repair_metadata_value(&attempt.metadata, "mutation_policy")),
        ("mutation_scope".to_owned(), repair_metadata_value(&attempt.metadata, "mutation_scope")),
        ("redacted".to_owned(), "true".to_owned()),
        ("repair_attempt_checksum".to_owned(), attempt.checksum.as_str().to_owned()),
        ("repair_attempt_id".to_owned(), attempt.record_id.as_str().to_owned()),
        (
            "repair_prompt".to_owned(),
            format!("{REPAIR_BUDGET_EXHAUSTED}: no repair agent invocation was made."),
        ),
        (
            "repair_request_id".to_owned(),
            repair_metadata_value(&attempt.metadata, "repair_request_id"),
        ),
        ("required_gates".to_owned(), repair_metadata_value(&attempt.metadata, "required_gates")),
        ("retry_count".to_owned(), repair_metadata_value(&attempt.metadata, "retry_count")),
        ("scope".to_owned(), repair_metadata_value(&attempt.metadata, "scope")),
        ("status".to_owned(), "blocked".to_owned()),
    ])
}

fn repair_prompt_from_metadata(metadata: &EvidenceMetadata) -> String {
    let prompt = format!(
        "Repair the failing Oya gate.\ncategory: {}\ngate: {}\ncommand: {}\nnext_command: {}\nscope: {}\nmutation_scope: {}\nmutation_policy: {}\nretry_count: {}\nfinding_id: {}\nrepair_request_id: {}\nfinding_checksum: {}\nstdout_preview:\n{}\nstderr_preview:\n{}\nRules: obey mutation_scope, keep evidence valid, then run the next command.",
        repair_metadata_value(metadata, "failure_category"),
        repair_metadata_value(metadata, "gate"),
        repair_metadata_value(metadata, "command"),
        repair_metadata_value(metadata, "next_action"),
        repair_metadata_value(metadata, "scope"),
        repair_metadata_value(metadata, "mutation_scope"),
        repair_metadata_value(metadata, "mutation_policy"),
        repair_metadata_value(metadata, "retry_count"),
        repair_metadata_value(metadata, "finding_record_id"),
        repair_metadata_value(metadata, "repair_request_id"),
        repair_metadata_value(metadata, "finding_checksum"),
        repair_metadata_value(metadata, "stdout_preview"),
        repair_metadata_value(metadata, "stderr_preview"),
    );
    limit_text_to_bytes(&prompt, REPAIR_PROMPT_LIMIT_BYTES)
}

fn repair_prompt_output_preview(preview: &str) -> String {
    let redacted = redact_output_preview(preview);
    limit_text_to_bytes(&redacted, REPAIR_PROMPT_OUTPUT_LIMIT_BYTES)
}

fn repair_metadata_value(metadata: &EvidenceMetadata, key: &str) -> String {
    metadata.get(key).map_or_else(|| "unknown".to_owned(), ToOwned::to_owned)
}

fn required_gate_names(required_gates: &[GateModel]) -> String {
    required_gates.iter().map(|model| model.id.as_str()).collect::<Vec<_>>().join(",")
}

fn repair_mutation_scope(finding: &EvidenceEnvelope) -> RepairMutationScope {
    RepairMutationScope::from_failure_category(&finding_metadata_value(finding, "category"))
}

fn finding_metadata_value(finding: &EvidenceEnvelope, key: &str) -> String {
    finding.metadata.get(key).map_or_else(|| "unknown".to_owned(), ToOwned::to_owned)
}

fn latest_finding_for_gate<'a>(
    evidence: &'a [EvidenceEnvelope],
    model: &GateModel,
) -> Option<&'a EvidenceEnvelope> {
    evidence
        .iter()
        .rev()
        .find(|record| record.kind == EvidenceKind::Finding && finding_matches_gate(record, model))
}

fn exhausted_repair_attempt<'a>(
    evidence: &'a [EvidenceEnvelope],
    model: &GateModel,
) -> Option<&'a EvidenceEnvelope> {
    evidence.iter().rev().find(|record| {
        record.kind == EvidenceKind::RepairAttempt
            && metadata_is(record, "gate", model.id.as_str())
            && metadata_is(record, "budget_remaining", "0")
    })
}

fn finding_matches_gate(record: &EvidenceEnvelope, model: &GateModel) -> bool {
    record.metadata.get("gate").map(|gate| gate.as_str()) == Some(model.id.as_str())
}

fn bounded_redacted_preview(bytes: &[u8]) -> String {
    let limit = bytes.len().min(GATE_OUTPUT_LIMIT_BYTES);
    let lossy = String::from_utf8_lossy(&bytes[..limit]);
    limit_text_to_bytes(&redact_output_preview(&lossy), GATE_OUTPUT_LIMIT_BYTES)
}

fn redact_output_preview(input: &str) -> String {
    input.lines().map(redact_output_line).collect::<Vec<_>>().join("\n")
}

fn redact_output_line(line: &str) -> String {
    let normalized = line.to_ascii_lowercase();
    if is_sensitive_output_line(&normalized) {
        REDACTED_OUTPUT_LINE.to_owned()
    } else {
        line.to_owned()
    }
}

fn is_sensitive_output_line(normalized: &str) -> bool {
    ["token", "secret", "password", "api_key", "apikey"]
        .into_iter()
        .any(|needle| normalized.contains(needle))
}

fn limit_text_to_bytes(input: &str, max_bytes: usize) -> String {
    let boundary = byte_boundary(input, max_bytes);
    match input.get(..boundary) {
        Some(value) => value.to_owned(),
        None => String::new(),
    }
}

fn byte_boundary(input: &str, max_bytes: usize) -> usize {
    if input.len() <= max_bytes {
        input.len()
    } else if input.is_char_boundary(max_bytes) {
        max_bytes
    } else {
        input
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index < max_bytes)
            .last()
            .map_or(0, |index| index)
    }
}

fn output_failure_category(model: &GateModel, status: &ExitStatus) -> Option<String> {
    if status.success() {
        None
    } else {
        Some(model.failure_category.as_str().to_owned())
    }
}

fn failure_category_metadata(model: &GateModel, status: &ExitStatus) -> String {
    match output_failure_category(model, status) {
        Some(category) => category,
        None => "none".to_owned(),
    }
}

fn exit_code_metadata(status: &ExitStatus) -> String {
    match status.code() {
        Some(code) => code.to_string(),
        None => "none".to_owned(),
    }
}

fn next_timestamp_after(
    previous: Option<DateTime<Utc>>,
    requested: DateTime<Utc>,
) -> anyhow::Result<DateTime<Utc>> {
    match previous {
        Some(timestamp) if requested.timestamp_millis() <= timestamp.timestamp_millis() => {
            timestamp
                .checked_add_signed(Duration::milliseconds(1))
                .ok_or_else(|| anyhow::anyhow!("gate evidence timestamp overflow"))
        }
        _ => Ok(requested),
    }
}

fn gate_started_record_id(
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-g-{}-s-{}",
        bead_id.as_str(),
        model.id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn gate_finished_record_id(
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-g-{}-f-{}",
        bead_id.as_str(),
        model.id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn finding_record_id(
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-g-{}-fn-{}",
        bead_id.as_str(),
        model.id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn repair_request_record_id(
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-g-{}-rr-{}",
        bead_id.as_str(),
        model.id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn repair_attempt_record_id(
    bead_id: &BeadId,
    request: &EvidenceEnvelope,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    let gate = repair_metadata_value(&request.metadata, "gate");
    EvidenceRecordId::parse(&format!(
        "ev-{}-g-{}-ra-{}",
        bead_id.as_str(),
        gate,
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn repair_blocked_record_id(
    bead_id: &BeadId,
    model: &GateModel,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-g-{}-rb-{}",
        bead_id.as_str(),
        model.id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn gate_status(status: &ExitStatus) -> &'static str {
    if status.success() {
        "passed"
    } else {
        "failed"
    }
}

fn gate_failure_message(output: &VerifyOutput) -> String {
    let category = match &output.failure_category {
        Some(category) => category.as_str(),
        None => "unknown",
    };
    let finding = match &output.finding_id {
        Some(finding_id) => finding_id.as_str(),
        None => "none",
    };
    match output.exit_code {
        Some(code) => {
            format!(
                "gate '{}' failed with category '{category}', finding '{finding}', and exit code {code}",
                output.gate
            )
        }
        None => {
            format!(
                "gate '{}' failed with category '{category}', finding '{finding}', and no exit code",
                output.gate
            )
        }
    }
}

fn repair_blocked_message(output: &VerifyRepairOutput) -> String {
    let reason = match output.block_reason.as_deref() {
        Some(reason) => reason,
        None => REPAIR_BUDGET_EXHAUSTED,
    };
    format!("{reason}: repair budget exhausted for gate '{}'", output.gate)
}

fn data_dir() -> PathBuf {
    match std::env::var("OYA_DATA_DIR") {
        Ok(value) => PathBuf::from(value),
        Err(_) => PathBuf::from(".oya-lite"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn verify_output_records_successful_gate_status() {
        let (bead_id, model, result, finished) = successful_fixture();

        let evidence =
            VerifyEvidence { last_record: &finished, evidence_records: 2, finding: None };
        let output = VerifyOutput::from_evidence(&bead_id, &model, &result, evidence);

        assert_eq!(output.run_id, "run-demo");
        assert_eq!(output.bead_id, "demo");
        assert_eq!(output.gate, "fmt");
        assert_eq!(output.moon_task, "oya:fmt");
        assert_eq!(output.status, "passed");
        assert_eq!(output.failure_category, None);
        assert_eq!(output.finding_id, None);
        assert_eq!(output.exit_code, Some(0));
        assert!(output.blocks_on_failure);
        assert_eq!(output.stdout.original_bytes, 0);
        assert_eq!(output.stderr.original_bytes, 0);
        assert_eq!(output.evidence_records, 2);
        assert_eq!(output.last_record_id, finished.record_id.as_str());
        assert_eq!(output.last_checksum, finished.checksum.as_str());
    }

    #[test]
    fn verify_output_records_failed_gate_status_without_raw_output() {
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let result = gate_result(ExitStatus::from_raw(256), b"stdout text", b"stderr text");
        let finished = gate_finished_fixture(&bead_id, &model, &result);
        let finding = finding_fixture(&model, &result, &finished);

        let evidence =
            VerifyEvidence { last_record: &finding, evidence_records: 3, finding: Some(&finding) };
        let output = VerifyOutput::from_evidence(&bead_id, &model, &result, evidence);
        let json = serde_json::to_string(&output).unwrap();

        assert_eq!(output.status, "failed");
        assert_eq!(output.failure_category, Some("format".to_owned()));
        assert_eq!(output.finding_id, Some(finding.record_id.as_str().to_owned()));
        assert_eq!(output.exit_code, Some(1));
        assert_eq!(
            gate_failure_message(&output),
            format!(
                "gate 'fmt' failed with category 'format', finding '{}', and exit code 1",
                finding.record_id.as_str()
            )
        );
        assert!(!json.contains("Command"));
        assert!(!json.contains("stdout text"));
        assert!(!json.contains("stderr text"));
    }

    #[test]
    fn gate_evidence_records_successful_gate_status() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let result = gate_result(ExitStatus::from_raw(0), b"", b"");

        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let finished = persist_gate_finished(&db, &model, &started, &result, timestamp(1)).unwrap();
        let evidence = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();

        assert_eq!(evidence.len(), 2);
        assert_eq!(evidence[0].kind, EvidenceKind::GateRunStarted);
        assert_eq!(evidence[0].previous_checksum, None);
        assert_eq!(evidence[1].kind, EvidenceKind::GateRunFinished);
        assert_eq!(evidence[1].previous_checksum, Some(started.checksum.clone()));
        assert_eq!(finished.metadata.get("status"), Some(&"passed".to_owned()));
        assert_eq!(finished.metadata.get("exit_code"), Some(&"0".to_owned()));
        assert_eq!(finished.metadata.get("failure_category"), Some(&"none".to_owned()));
    }

    #[test]
    fn gate_evidence_records_typed_failure_category() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let result = gate_result(ExitStatus::from_raw(256), b"", b"");

        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let finished = persist_gate_finished(&db, &model, &started, &result, timestamp(1)).unwrap();
        let finding = persist_finding_if_failed(&db, &model, &finished, &result, timestamp(2))
            .unwrap()
            .unwrap();
        let evidence = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();
        let json = finished.to_canonical_json().unwrap();

        assert_eq!(evidence.len(), 3);
        assert_eq!(evidence[1].kind, EvidenceKind::GateRunFinished);
        assert_eq!(evidence[2].kind, EvidenceKind::Finding);
        assert_eq!(evidence[2].previous_checksum, Some(finished.checksum.clone()));
        assert_eq!(finished.metadata.get("status"), Some(&"failed".to_owned()));
        assert_eq!(finished.metadata.get("failure_category"), Some(&"format".to_owned()));
        assert_eq!(finding.metadata.get("category"), Some(&"format".to_owned()));
        assert_eq!(
            finding.metadata.get("gate_record_id"),
            Some(&finished.record_id.as_str().to_owned())
        );
        assert_eq!(
            finding.metadata.get("next_action"),
            Some(&"run moon run oya:fmt-fix then rerun the gate".to_owned())
        );
        assert!(!json.contains("Command"));
    }

    #[test]
    fn successful_gate_does_not_create_finding() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let result = gate_result(ExitStatus::from_raw(0), b"", b"");

        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let finished = persist_gate_finished(&db, &model, &started, &result, timestamp(1)).unwrap();
        let finding =
            persist_finding_if_failed(&db, &model, &finished, &result, timestamp(2)).unwrap();
        let evidence = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();

        assert_eq!(finding, None);
        assert_eq!(evidence.len(), 2);
    }

    #[test]
    fn repair_request_points_to_finding_gate_scope_and_retry_count() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let result = gate_result(ExitStatus::from_raw(256), b"", b"");
        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let finished = persist_gate_finished(&db, &model, &started, &result, timestamp(1)).unwrap();
        let finding = persist_finding_if_failed(&db, &model, &finished, &result, timestamp(2))
            .unwrap()
            .unwrap();

        let request = persist_repair_request(&db, &bead_id, &model, timestamp(3)).unwrap();
        let evidence = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();

        assert_eq!(evidence.len(), 4);
        assert_eq!(evidence[3].kind, EvidenceKind::RepairRequest);
        assert_eq!(request.previous_checksum, Some(finding.checksum.clone()));
        assert_eq!(
            request.metadata.get("finding_record_id"),
            Some(&finding.record_id.as_str().to_owned())
        );
        assert_eq!(request.metadata.get("gate"), Some(&"fmt".to_owned()));
        assert_eq!(request.metadata.get("scope"), Some(&"gate".to_owned()));
        assert_eq!(request.metadata.get("retry_count"), Some(&"1".to_owned()));
    }

    #[test]
    fn repair_attempt_is_persisted_before_reverification() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let result = gate_result(ExitStatus::from_raw(256), b"", b"token=super-secret-token");
        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let finished = persist_gate_finished(&db, &model, &started, &result, timestamp(1)).unwrap();
        let finding = persist_finding_if_failed(&db, &model, &finished, &result, timestamp(2))
            .unwrap()
            .unwrap();
        let request = persist_repair_request(&db, &bead_id, &model, timestamp(3)).unwrap();
        let required_gates = [model];

        let attempt = persist_repair_attempt(&db, &request, &required_gates, timestamp(4)).unwrap();
        let evidence = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();
        let output =
            VerifyRepairOutput::from_repair(&request, &attempt, &[], evidence.len()).unwrap();
        let json = attempt.to_canonical_json().unwrap();

        assert_eq!(evidence.len(), 5);
        assert_eq!(evidence[4].kind, EvidenceKind::RepairAttempt);
        assert_eq!(attempt.previous_checksum, Some(request.checksum.clone()));
        assert_eq!(attempt.metadata.get("status"), Some(&"attempt_recorded".to_owned()));
        assert_eq!(
            attempt.metadata.get("repair_request_id"),
            Some(&request.record_id.as_str().to_owned())
        );
        assert_eq!(
            attempt.metadata.get("finding_record_id"),
            Some(&finding.record_id.as_str().to_owned())
        );
        assert_eq!(attempt.metadata.get("budget_remaining"), Some(&"0".to_owned()));
        assert_eq!(attempt.metadata.get("retry_count"), Some(&"1".to_owned()));
        assert_eq!(attempt.metadata.get("required_gates"), Some(&"fmt".to_owned()));
        assert_eq!(output.status, "attempt_recorded");
        assert_eq!(output.repair_request_id, request.record_id.as_str());
        assert_eq!(output.repair_attempt_id, attempt.record_id.as_str());
        assert_eq!(output.finding_id, finding.record_id.as_str());
        assert_eq!(output.required_gates, vec!["fmt".to_owned()]);
        assert!(!json.contains("super-secret-token"));
    }

    #[test]
    fn repair_reverification_reruns_failed_and_previously_passed_blocking_gates() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let passed_model = GateId::Check.model();
        let failed_model = GateId::Fmt.model();
        let passed = gate_result(ExitStatus::from_raw(0), b"", b"");
        let failed = gate_result(ExitStatus::from_raw(256), b"", b"");
        let repaired = gate_result(ExitStatus::from_raw(0), b"", b"");
        let check_started =
            persist_gate_started(&db, &bead_id, &passed_model, timestamp(0)).unwrap();
        let _check_finished =
            persist_gate_finished(&db, &passed_model, &check_started, &passed, timestamp(1))
                .unwrap();
        let fmt_started = persist_gate_started(&db, &bead_id, &failed_model, timestamp(2)).unwrap();
        let fmt_finished =
            persist_gate_finished(&db, &failed_model, &fmt_started, &failed, timestamp(3)).unwrap();
        let _finding =
            persist_finding_if_failed(&db, &failed_model, &fmt_finished, &failed, timestamp(4))
                .unwrap()
                .unwrap();
        let request = persist_repair_request(&db, &bead_id, &failed_model, timestamp(5)).unwrap();
        let evidence_before_attempt = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();
        let required_gates =
            required_reverification_gates(evidence_before_attempt.as_slice(), &failed_model)
                .unwrap();
        let attempt =
            persist_repair_attempt(&db, &request, required_gates.as_slice(), timestamp(6)).unwrap();

        let fmt_rerun = persist_reverification_gate_result(
            &db,
            &bead_id,
            required_gates[0],
            repaired.clone(),
            timestamp(7),
        )
        .unwrap();
        let check_rerun = persist_reverification_gate_result(
            &db,
            &bead_id,
            required_gates[1],
            repaired,
            timestamp(8),
        )
        .unwrap();
        let reruns = vec![fmt_rerun, check_rerun];
        let evidence = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();
        let output =
            VerifyRepairOutput::from_repair(&request, &attempt, reruns.as_slice(), evidence.len())
                .unwrap();

        assert_eq!(required_gate_names(required_gates.as_slice()), "fmt,check");
        assert_eq!(attempt.metadata.get("required_gates"), Some(&"fmt,check".to_owned()));
        assert_eq!(output.reverification_status, "passed");
        assert_eq!(output.required_gates, vec!["fmt".to_owned(), "check".to_owned()]);
        assert_eq!(output.reverification_gates.len(), 2);
        assert_eq!(output.reverification_gates[0].gate, "fmt");
        assert_eq!(output.reverification_gates[1].gate, "check");
        assert_eq!(output.evidence_records, 11);
    }

    #[test]
    fn repair_budget_exhaustion_blocks_without_another_gate_invocation() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let failed = gate_result(ExitStatus::from_raw(256), b"", b"");
        let repaired = gate_result(ExitStatus::from_raw(0), b"", b"");
        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let finished = persist_gate_finished(&db, &model, &started, &failed, timestamp(1)).unwrap();
        let _finding = persist_finding_if_failed(&db, &model, &finished, &failed, timestamp(2))
            .unwrap()
            .unwrap();
        let request = persist_repair_request(&db, &bead_id, &model, timestamp(3)).unwrap();
        let required_gates = [model];
        let attempt = persist_repair_attempt(&db, &request, &required_gates, timestamp(4)).unwrap();
        let _rerun =
            persist_reverification_gate_result(&db, &bead_id, model, repaired, timestamp(5))
                .unwrap();
        let evidence_before = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();
        let previous_checksum = evidence_before.last().unwrap().checksum.clone();

        let blocked = persist_blocked_if_budget_exhausted(&db, &bead_id, &model, timestamp(6))
            .unwrap()
            .unwrap();
        let evidence_after = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();
        let output = VerifyRepairOutput::from_blocked(&blocked, evidence_after.len()).unwrap();

        assert_eq!(evidence_after.len(), evidence_before.len() + 1);
        assert_eq!(evidence_after.last().unwrap().kind, EvidenceKind::RepairBlocked);
        assert_eq!(blocked.previous_checksum, Some(previous_checksum));
        assert_eq!(blocked.metadata.get("block_reason"), Some(&REPAIR_BUDGET_EXHAUSTED.to_owned()));
        assert_eq!(
            blocked.metadata.get("repair_attempt_id"),
            Some(&attempt.record_id.as_str().to_owned())
        );
        assert_eq!(output.status, "blocked");
        assert_eq!(output.block_reason, Some(REPAIR_BUDGET_EXHAUSTED.to_owned()));
        assert_eq!(output.repair_blocked_id, Some(blocked.record_id.as_str().to_owned()));
        assert_eq!(output.repair_attempt_id, attempt.record_id.as_str());
        assert_eq!(output.required_gates, vec!["fmt".to_owned()]);
        assert_eq!(output.reverification_status, "blocked");
        assert!(output.reverification_gates.is_empty());
        assert_eq!(
            repair_blocked_message(&output),
            "RepairBudgetExhausted: repair budget exhausted for gate 'fmt'"
        );
    }

    #[test]
    fn repair_prompt_excludes_raw_log_spam_and_includes_next_command() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let stdout = repair_prompt_spam_output();
        let stderr = b"token=super-secret-token";
        let result = gate_result(ExitStatus::from_raw(256), stdout.as_bytes(), stderr);
        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let finished = persist_gate_finished(&db, &model, &started, &result, timestamp(1)).unwrap();
        let finding = persist_finding_if_failed(&db, &model, &finished, &result, timestamp(2))
            .unwrap()
            .unwrap();

        let request = persist_repair_request(&db, &bead_id, &model, timestamp(3)).unwrap();
        let required_gates = [model];
        let attempt = persist_repair_attempt(&db, &request, &required_gates, timestamp(4)).unwrap();
        let output = VerifyRepairOutput::from_repair(&request, &attempt, &[], 5).unwrap();
        let prompt = output.repair_prompt.as_str();

        assert!(prompt.len() <= REPAIR_PROMPT_LIMIT_BYTES);
        assert!(prompt.contains("run moon run oya:fmt-fix then rerun the gate"));
        assert!(prompt.contains(finding.record_id.as_str()));
        assert!(prompt.contains(request.record_id.as_str()));
        assert!(prompt.contains("scope: gate"));
        assert!(prompt.contains("retry_count: 1"));
        assert!(!prompt.contains("raw-log-spam-tail-marker"));
        assert!(!prompt.contains("super-secret-token"));
        assert_eq!(request.metadata.get("stderr_preview"), Some(&"[redacted]".to_owned()));
    }

    #[test]
    fn repair_request_rejects_missing_finding() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();

        let error = persist_repair_request(&db, &bead_id, &model, timestamp(3)).unwrap_err();

        assert!(error.to_string().contains("not_found: finding for gate 'fmt' does not exist"));
    }

    #[test]
    fn output_limit_bounds_gate_output_and_finding_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let oversized = oversized_gate_output("stdout-tail-marker");
        let result =
            gate_result(ExitStatus::from_raw(256), &oversized, b"token=super-secret-token");

        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let finished = persist_gate_finished(&db, &model, &started, &result, timestamp(1)).unwrap();
        let finding = persist_finding_if_failed(&db, &model, &finished, &result, timestamp(2))
            .unwrap()
            .unwrap();
        let json = finished.to_canonical_json().unwrap();
        let finding_json = finding.to_canonical_json().unwrap();

        assert_eq!(finished.metadata.get("stdout_truncated"), Some(&"true".to_owned()));
        assert_eq!(
            finished.metadata.get("stdout_original_bytes"),
            Some(&oversized.len().to_string())
        );
        assert_eq!(
            finished.metadata.get("stdout_stored_bytes"),
            Some(&GATE_OUTPUT_LIMIT_BYTES.to_string())
        );
        assert_eq!(
            finding.metadata.get("stdout_stored_bytes"),
            Some(&GATE_OUTPUT_LIMIT_BYTES.to_string())
        );
        assert_eq!(finished.metadata.get("stderr_preview"), Some(&"[redacted]".to_owned()));
        assert!(json.len() < GATE_OUTPUT_LIMIT_BYTES * 3);
        assert!(finding_json.len() < GATE_OUTPUT_LIMIT_BYTES * 3);
        assert!(!json.contains("stdout-tail-marker"));
        assert!(!json.contains("super-secret-token"));
        assert!(!finding_json.contains("stdout-tail-marker"));
        assert!(!finding_json.contains("super-secret-token"));
    }

    #[test]
    fn gate_evidence_links_to_existing_run_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let existing = run_started_fixture(&bead_id);
        db.append_evidence(&existing).unwrap();
        db.flush().unwrap();

        let started = persist_gate_started(&db, &bead_id, &model, timestamp(0)).unwrap();
        let evidence = db.load_evidence(&RunId::from_bead_id(&bead_id)).unwrap();

        assert_eq!(evidence.len(), 2);
        assert_eq!(started.previous_checksum, Some(existing.checksum));
        assert!(started.timestamp > existing.timestamp);
    }

    fn successful_fixture() -> (BeadId, GateModel, GateCommandResult, EvidenceEnvelope) {
        let bead_id = bead_id();
        let model = GateId::Fmt.model();
        let result = gate_result(ExitStatus::from_raw(0), b"", b"");
        let finished = gate_finished_fixture(&bead_id, &model, &result);
        (bead_id, model, result, finished)
    }

    fn gate_finished_fixture(
        bead_id: &BeadId,
        model: &GateModel,
        result: &GateCommandResult,
    ) -> EvidenceEnvelope {
        let started = gate_started_envelope(
            bead_id,
            &RunId::from_bead_id(bead_id),
            model,
            timestamp(0),
            None,
        )
        .unwrap();
        gate_finished_envelope(model, result, timestamp(1), &started).unwrap()
    }

    fn finding_fixture(
        model: &GateModel,
        result: &GateCommandResult,
        finished: &EvidenceEnvelope,
    ) -> EvidenceEnvelope {
        finding_envelope(model, result, timestamp(2), finished).unwrap()
    }

    fn gate_result(status: ExitStatus, stdout: &[u8], stderr: &[u8]) -> GateCommandResult {
        GateCommandResult {
            status,
            stdout: BoundedGateOutput::from_bytes(stdout),
            stderr: BoundedGateOutput::from_bytes(stderr),
        }
    }

    fn run_started_fixture(bead_id: &BeadId) -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse("ev-demo-run-started-001").unwrap(),
            run_id: RunId::from_bead_id(bead_id),
            bead_id: bead_id.clone(),
            timestamp: timestamp(0),
            kind: EvidenceKind::RunStarted,
            metadata: EvidenceMetadata::new(),
            previous_checksum: None,
        })
        .unwrap()
    }

    fn bead_id() -> BeadId {
        BeadId::parse("demo").unwrap()
    }

    fn repair_prompt_spam_output() -> String {
        format!("{}raw-log-spam-tail-marker", "raw-log-spam ".repeat(600))
    }

    fn oversized_gate_output(marker: &str) -> Vec<u8> {
        let mut output = vec![b'a'; GATE_OUTPUT_LIMIT_BYTES + 128];
        output.extend(marker.as_bytes());
        output
    }

    fn timestamp(offset_seconds: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_779_999_600 + offset_seconds, 0).unwrap()
    }
}
