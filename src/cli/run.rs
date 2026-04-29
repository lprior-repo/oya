#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

use super::agent::{
    persist_agent_run, run_opencode_server_if_configured, run_opencode_subprocess,
    AgentOutputSummary, AgentRunResult,
};
use super::args::RunArgs;
use super::verify::{persist_run_verification_gate, RunVerificationResult};
use super::workspace::{
    branch_name_from_ids, create_pull_request_after_green_gates, ensure_clean_workspace_for_run,
    sync_workspace_with_main, validate_meaningful_git_diff, DiffValidationError, PullRequestError,
    PullRequestOutcome, VcsSyncError,
};
use crate::lifecycle::state::StateDb;
use crate::lifecycle::types::{
    BeadId, EvidenceEnvelope, EvidenceEnvelopeParts, EvidenceKind, EvidenceMetadata,
    EvidenceRecordId, GateId, Model, RunId, RunPhase, RunState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSkeletonOutput {
    pub run_id: String,
    pub bead_id: String,
    pub phase: String,
    pub verdict: String,
    pub status: String,
    pub evidence_records: usize,
    pub agent_request_id: String,
    pub agent_run_id: Option<String>,
    pub verification: Option<RunVerificationSummary>,
    pub pull_request: Option<PullRequestSummary>,
    pub failure_category: Option<String>,
    pub error: Option<String>,
    pub stdout: Option<AgentOutputSummary>,
    pub stderr: Option<AgentOutputSummary>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunVerificationSummary {
    pub gate: String,
    pub moon_task: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub gate_run_started_id: String,
    pub gate_run_finished_id: String,
    pub finding_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestSummary {
    pub branch: String,
    pub url: String,
    pub record_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum RunExitCodeError {
    #[error("run verification failed with verdict '{verdict}'")]
    VerificationFailed { verdict: String },

    #[error("run verification incomplete for verdict '{verdict}'")]
    VerificationIncomplete { verdict: String },

    #[error("agent run failed with category '{category}' and record '{record}'")]
    AgentFailed { category: String, record: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum RunStartError {
    #[error("BeadAlreadyRunning: bead '{bead_id}' already has run '{run_id}' in phase '{phase}'")]
    BeadAlreadyRunning { bead_id: String, run_id: String, phase: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunSkeletonEvidence {
    agent_request: EvidenceEnvelope,
    evidence_records: usize,
}

struct PullRequestFailure<'a> {
    branch: &'a str,
    error: &'a PullRequestError,
}

pub async fn run_command(args: RunArgs) -> anyhow::Result<()> {
    let bead_id = BeadId::parse(&args.bead_id)?;
    let run_id = RunId::from_bead_id(&bead_id);
    ensure_clean_workspace_for_run(run_id.as_str()).await?;
    let db = StateDb::open(data_dir())?;
    sync_workspace_with_main_or_record_failure(&db, &bead_id, &run_id).await?;
    let skeleton =
        persist_run_skeleton_evidence(&db, bead_id, &args.prompt, &args.model, Utc::now())?;
    let output = run_output(&db, &skeleton, &args.prompt, &args.model).await?;
    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");
    enforce_run_exit_contract(&output)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn persist_run_skeleton(
    db: &StateDb,
    bead_id: BeadId,
    prompt: &str,
    model: &str,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<RunSkeletonOutput> {
    let evidence = persist_run_skeleton_evidence(db, bead_id, prompt, model, timestamp)?;
    Ok(RunSkeletonOutput::from_agent_request(&evidence.agent_request, evidence.evidence_records))
}

fn persist_run_skeleton_evidence(
    db: &StateDb,
    bead_id: BeadId,
    prompt: &str,
    model: &str,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<RunSkeletonEvidence> {
    let run_id = RunId::from_bead_id(&bead_id);
    ensure_run_can_start(db, &run_id, &bead_id)?;
    let run_started = run_started_envelope(&bead_id, &run_id, timestamp)?;
    let prompt_record = prompt_record_envelope(&bead_id, &run_id, prompt, &run_started)?;
    let agent_request = agent_request_envelope(&bead_id, &run_id, model, &prompt_record)?;
    db.append_evidence(&run_started)?;
    db.append_evidence(&prompt_record)?;
    db.append_evidence(&agent_request)?;
    db.flush()?;
    Ok(RunSkeletonEvidence { agent_request, evidence_records: 3 })
}

async fn sync_workspace_with_main_or_record_failure(
    db: &StateDb,
    bead_id: &BeadId,
    run_id: &RunId,
) -> anyhow::Result<()> {
    match sync_workspace_with_main().await {
        Ok(()) => Ok(()),
        Err(error) => {
            persist_vcs_sync_failed(db, bead_id, run_id, Utc::now(), &error)?;
            Err(error.into())
        }
    }
}

fn persist_vcs_sync_failed(
    db: &StateDb,
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
    error: &VcsSyncError,
) -> anyhow::Result<EvidenceEnvelope> {
    let envelope = vcs_sync_failed_envelope(bead_id, run_id, timestamp, error)?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

fn persist_diff_validation_failed(
    db: &StateDb,
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
    error: &DiffValidationError,
) -> anyhow::Result<EvidenceEnvelope> {
    let envelope = diff_validation_failed_envelope(bead_id, run_id, timestamp, error)?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

fn persist_pull_request_created(
    db: &StateDb,
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
    pull_request: &PullRequestOutcome,
) -> anyhow::Result<EvidenceEnvelope> {
    let envelope = pull_request_created_envelope(bead_id, run_id, timestamp, pull_request)?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

fn persist_pull_request_failed(
    db: &StateDb,
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
    failure: PullRequestFailure<'_>,
) -> anyhow::Result<EvidenceEnvelope> {
    let envelope = pull_request_failed_envelope(bead_id, run_id, timestamp, failure)?;
    db.append_evidence(&envelope)?;
    db.flush()?;
    Ok(envelope)
}

async fn run_output(
    db: &StateDb,
    skeleton: &RunSkeletonEvidence,
    prompt: &str,
    model: &str,
) -> anyhow::Result<RunSkeletonOutput> {
    if should_run_full_factory(skeleton) {
        run_full_factory(db, skeleton, prompt, model).await
    } else if let Some(result) = run_opencode_server_if_configured(prompt, model).await {
        persist_agent_result(db, skeleton, model, &result)
    } else if is_default_model(model) {
        Ok(RunSkeletonOutput::from_agent_request(
            &skeleton.agent_request,
            skeleton.evidence_records,
        ))
    } else {
        run_agent_subprocess(db, skeleton, prompt, model).await
    }
}

async fn run_full_factory(
    db: &StateDb,
    skeleton: &RunSkeletonEvidence,
    prompt: &str,
    model: &str,
) -> anyhow::Result<RunSkeletonOutput> {
    let result = run_agent_for_factory(prompt, model).await;
    let agent_run = persist_agent_run(db, model, &skeleton.agent_request, &result, Utc::now())?;
    let mut output = RunSkeletonOutput::from_agent_run(
        &skeleton.agent_request,
        &agent_run,
        &result,
        db.load_evidence(&agent_run.run_id)?.len(),
    );
    if result.succeeded() {
        let verification =
            persist_run_verification_gate(db, &agent_run.bead_id, &GateId::Fmt.model()).await?;
        let diff_validation_required = verification.status == "passed";
        output.verification = Some(RunVerificationSummary::from_result(&verification));
        if diff_validation_required {
            output = validate_verified_run_diff_or_block(db, &agent_run, output).await?;
            if output.error.is_none() {
                output = create_pull_request_or_block(db, &agent_run, output).await?;
            }
        }
    }
    output = output.with_verdict_from_evidence(db)?;
    if verification_failed(&output) {
        output.error = Some("moon verification failed".to_owned());
    }
    Ok(output)
}

async fn create_pull_request_or_block(
    db: &StateDb,
    agent_run: &EvidenceEnvelope,
    mut output: RunSkeletonOutput,
) -> anyhow::Result<RunSkeletonOutput> {
    let branch = branch_name_from_ids(agent_run.bead_id.as_str(), agent_run.run_id.as_str());
    let title = pull_request_title(&agent_run.bead_id);
    let body = pull_request_body(&agent_run.bead_id, &agent_run.run_id);
    match create_pull_request_after_green_gates(&branch, &title, &body).await {
        Ok(pull_request) => {
            let evidence = persist_pull_request_created(
                db,
                &agent_run.bead_id,
                &agent_run.run_id,
                Utc::now(),
                &pull_request,
            )?;
            output.pull_request =
                Some(PullRequestSummary::from_pull_request(&evidence, &pull_request));
            Ok(output)
        }
        Err(error) => {
            let failure = PullRequestFailure { branch: &branch, error: &error };
            persist_pull_request_failed(
                db,
                &agent_run.bead_id,
                &agent_run.run_id,
                Utc::now(),
                failure,
            )?;
            output.error = Some(error.sanitized_message().to_owned());
            Ok(output)
        }
    }
}

async fn validate_verified_run_diff_or_block(
    db: &StateDb,
    agent_run: &EvidenceEnvelope,
    mut output: RunSkeletonOutput,
) -> anyhow::Result<RunSkeletonOutput> {
    match validate_meaningful_git_diff().await {
        Ok(()) => Ok(output),
        Err(error) => {
            persist_diff_validation_failed(
                db,
                &agent_run.bead_id,
                &agent_run.run_id,
                Utc::now(),
                &error,
            )?;
            output.error = Some(error.sanitized_message().to_owned());
            Ok(output)
        }
    }
}

async fn run_agent_for_factory(prompt: &str, model: &str) -> AgentRunResult {
    match run_opencode_server_if_configured(prompt, model).await {
        Some(result) => result,
        None => run_opencode_subprocess(prompt, model).await,
    }
}

async fn run_agent_subprocess(
    db: &StateDb,
    skeleton: &RunSkeletonEvidence,
    prompt: &str,
    model: &str,
) -> anyhow::Result<RunSkeletonOutput> {
    let result = run_opencode_subprocess(prompt, model).await;
    persist_agent_result(db, skeleton, model, &result)
}

fn persist_agent_result(
    db: &StateDb,
    skeleton: &RunSkeletonEvidence,
    model: &str,
    result: &AgentRunResult,
) -> anyhow::Result<RunSkeletonOutput> {
    let agent_run = persist_agent_run(db, model, &skeleton.agent_request, result, Utc::now())?;
    let evidence_records = db.load_evidence(&agent_run.run_id)?.len();
    Ok(RunSkeletonOutput::from_agent_run(
        &skeleton.agent_request,
        &agent_run,
        result,
        evidence_records,
    ))
}

impl RunSkeletonOutput {
    fn from_agent_request(envelope: &EvidenceEnvelope, evidence_records: usize) -> Self {
        Self {
            run_id: envelope.run_id.as_str().to_owned(),
            bead_id: envelope.bead_id.as_str().to_owned(),
            phase: "agent_requested".to_owned(),
            verdict: "inconclusive".to_owned(),
            status: "blocked".to_owned(),
            evidence_records,
            agent_request_id: envelope.record_id.as_str().to_owned(),
            agent_run_id: None,
            verification: None,
            pull_request: None,
            failure_category: None,
            error: None,
            stdout: None,
            stderr: None,
            message:
                "RunStarted, PromptRecord, and AgentRequest persisted; external action intentionally not invoked"
                    .to_owned(),
        }
    }

    fn from_agent_run(
        request: &EvidenceEnvelope,
        agent_run: &EvidenceEnvelope,
        result: &AgentRunResult,
        evidence_records: usize,
    ) -> Self {
        Self {
            run_id: agent_run.run_id.as_str().to_owned(),
            bead_id: agent_run.bead_id.as_str().to_owned(),
            phase: agent_run_phase(result).to_owned(),
            verdict: agent_run_verdict(result).to_owned(),
            status: result.status.as_str().to_owned(),
            evidence_records,
            agent_request_id: request.record_id.as_str().to_owned(),
            agent_run_id: Some(agent_run.record_id.as_str().to_owned()),
            verification: None,
            pull_request: None,
            failure_category: result.failure_category_name().map(str::to_owned),
            error: result.sanitized_error(),
            stdout: Some(result.stdout.summary()),
            stderr: Some(result.stderr.summary()),
            message: agent_run_message(result).to_owned(),
        }
    }

    fn with_verdict_from_evidence(mut self, db: &StateDb) -> anyhow::Result<Self> {
        let run_id = RunId::parse(&self.run_id)?;
        let bead_id = BeadId::parse(&self.bead_id)?;
        let evidence = db.load_evidence(&run_id)?;
        let state = RunState::planned(run_id, bead_id).apply_evidence_chain(evidence.as_slice())?;
        self.phase = state.phase().as_str().to_owned();
        self.verdict = verdict_from_phase(state.phase()).to_owned();
        self.status = status_from_phase(state.phase()).to_owned();
        self.evidence_records = evidence.len();
        Ok(self)
    }
}

impl PullRequestSummary {
    fn from_pull_request(evidence: &EvidenceEnvelope, pull_request: &PullRequestOutcome) -> Self {
        Self {
            branch: pull_request.branch.clone(),
            url: pull_request.url.clone(),
            record_id: evidence.record_id.as_str().to_owned(),
        }
    }
}

impl RunVerificationSummary {
    fn from_result(result: &RunVerificationResult) -> Self {
        Self {
            gate: result.gate.clone(),
            moon_task: result.moon_task.clone(),
            status: result.status.clone(),
            exit_code: result.exit_code,
            gate_run_started_id: result.gate_run_started_id.clone(),
            gate_run_finished_id: result.gate_run_finished_id.clone(),
            finding_id: result.finding_id.clone(),
        }
    }
}

fn should_run_full_factory(skeleton: &RunSkeletonEvidence) -> bool {
    skeleton.agent_request.bead_id.as_str() == "demo-fix"
}

fn ensure_run_can_start(db: &StateDb, run_id: &RunId, bead_id: &BeadId) -> anyhow::Result<()> {
    if let Some(phase) = existing_run_phase(db, run_id, bead_id)? {
        return Err(RunStartError::BeadAlreadyRunning {
            bead_id: bead_id.as_str().to_owned(),
            run_id: run_id.as_str().to_owned(),
            phase,
        }
        .into());
    }
    Ok(())
}

fn existing_run_phase(
    db: &StateDb,
    run_id: &RunId,
    bead_id: &BeadId,
) -> anyhow::Result<Option<String>> {
    let evidence = db.load_evidence(run_id)?;
    if evidence.is_empty() {
        Ok(None)
    } else {
        Ok(Some(phase_from_evidence(run_id, bead_id, evidence.as_slice())))
    }
}

fn phase_from_evidence(run_id: &RunId, bead_id: &BeadId, evidence: &[EvidenceEnvelope]) -> String {
    RunState::planned(run_id.clone(), bead_id.clone())
        .apply_evidence_chain(evidence)
        .map_or_else(|_| "unknown".to_owned(), |state| state.phase().as_str().to_owned())
}

fn verification_failed(output: &RunSkeletonOutput) -> bool {
    output.verification.as_ref().map(|summary| summary.status.as_str()) == Some("failed")
}

fn enforce_run_exit_contract(output: &RunSkeletonOutput) -> Result<(), RunExitCodeError> {
    if output.bead_id == "demo-fix" {
        return enforce_demo_fix_exit_contract(output);
    }
    match &output.error {
        Some(_) => Err(agent_failed_exit(output)),
        None => Ok(()),
    }
}

fn enforce_demo_fix_exit_contract(output: &RunSkeletonOutput) -> Result<(), RunExitCodeError> {
    if output.error.is_some() && output.verification.is_none() {
        return Err(agent_failed_exit(output));
    }
    match output.verification.as_ref().map(|summary| summary.status.as_str()) {
        Some("passed") if output.verdict == "pass" => Ok(()),
        Some(_) => Err(RunExitCodeError::VerificationFailed { verdict: output.verdict.clone() }),
        None => Err(RunExitCodeError::VerificationIncomplete { verdict: output.verdict.clone() }),
    }
}

fn agent_failed_exit(output: &RunSkeletonOutput) -> RunExitCodeError {
    let category = match &output.failure_category {
        Some(category) => category.clone(),
        None => "unknown".to_owned(),
    };
    let record = match &output.agent_run_id {
        Some(record) => record.clone(),
        None => "unknown".to_owned(),
    };
    RunExitCodeError::AgentFailed { category, record }
}

fn verdict_from_phase(phase: RunPhase) -> &'static str {
    match phase {
        RunPhase::Completed => "pass",
        RunPhase::Blocked | RunPhase::RepairBlocked => "fail",
        RunPhase::Planned
        | RunPhase::Started
        | RunPhase::PromptRecorded
        | RunPhase::AgentRequested
        | RunPhase::AgentRan
        | RunPhase::GateRunning
        | RunPhase::Repairing => "inconclusive",
    }
}

fn status_from_phase(phase: RunPhase) -> &'static str {
    match phase {
        RunPhase::Completed => "completed",
        RunPhase::Blocked | RunPhase::RepairBlocked => "blocked",
        RunPhase::Planned
        | RunPhase::Started
        | RunPhase::PromptRecorded
        | RunPhase::AgentRequested
        | RunPhase::AgentRan
        | RunPhase::GateRunning
        | RunPhase::Repairing => "running",
    }
}

fn agent_run_phase(result: &AgentRunResult) -> &'static str {
    if result.succeeded() {
        "agent_ran"
    } else {
        "blocked"
    }
}

fn agent_run_verdict(result: &AgentRunResult) -> &'static str {
    if result.succeeded() {
        "inconclusive"
    } else {
        "fail"
    }
}

fn is_default_model(model: &str) -> bool {
    model == Model::default_model().as_str()
}

fn agent_run_message(result: &AgentRunResult) -> &'static str {
    if result.succeeded() {
        "AgentRun persisted; agent execution completed"
    } else {
        "AgentRun persisted; agent execution failed with typed sanitized failure"
    }
}

fn run_started_envelope(
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: run_started_record_id(bead_id, timestamp)?,
        run_id: run_id.clone(),
        bead_id: bead_id.clone(),
        timestamp,
        kind: EvidenceKind::RunStarted,
        metadata: EvidenceMetadata::new(),
        previous_checksum: None,
    })
    .map_err(Into::into)
}

fn prompt_record_envelope(
    bead_id: &BeadId,
    run_id: &RunId,
    prompt: &str,
    run_started: &EvidenceEnvelope,
) -> anyhow::Result<EvidenceEnvelope> {
    let timestamp = prompt_record_timestamp(run_started.timestamp)?;
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: prompt_record_id(bead_id, timestamp)?,
        run_id: run_id.clone(),
        bead_id: bead_id.clone(),
        timestamp,
        kind: EvidenceKind::PromptRecord,
        metadata: prompt_metadata(prompt),
        previous_checksum: Some(run_started.checksum.clone()),
    })
    .map_err(Into::into)
}

fn prompt_record_timestamp(timestamp: DateTime<Utc>) -> anyhow::Result<DateTime<Utc>> {
    timestamp
        .checked_add_signed(Duration::milliseconds(1))
        .ok_or_else(|| anyhow::anyhow!("prompt record timestamp overflow"))
}

fn agent_request_envelope(
    bead_id: &BeadId,
    run_id: &RunId,
    model: &str,
    prompt_record: &EvidenceEnvelope,
) -> anyhow::Result<EvidenceEnvelope> {
    let timestamp = agent_request_timestamp(prompt_record.timestamp)?;
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: agent_request_id(bead_id, timestamp)?,
        run_id: run_id.clone(),
        bead_id: bead_id.clone(),
        timestamp,
        kind: EvidenceKind::AgentRequest,
        metadata: agent_request_metadata(model, prompt_record),
        previous_checksum: Some(prompt_record.checksum.clone()),
    })
    .map_err(Into::into)
}

fn vcs_sync_failed_envelope(
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
    error: &VcsSyncError,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: vcs_sync_failed_record_id(bead_id, timestamp)?,
        run_id: run_id.clone(),
        bead_id: bead_id.clone(),
        timestamp,
        kind: EvidenceKind::VcsSyncFailed,
        metadata: vcs_sync_failed_metadata(error),
        previous_checksum: None,
    })
    .map_err(Into::into)
}

fn diff_validation_failed_envelope(
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
    error: &DiffValidationError,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: diff_validation_failed_record_id(bead_id, timestamp)?,
        run_id: run_id.clone(),
        bead_id: bead_id.clone(),
        timestamp,
        kind: EvidenceKind::DiffValidationFailed,
        metadata: diff_validation_failed_metadata(error),
        previous_checksum: None,
    })
    .map_err(Into::into)
}

fn pull_request_created_envelope(
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
    pull_request: &PullRequestOutcome,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: pull_request_created_record_id(bead_id, timestamp)?,
        run_id: run_id.clone(),
        bead_id: bead_id.clone(),
        timestamp,
        kind: EvidenceKind::PullRequestCreated,
        metadata: pull_request_created_metadata(pull_request),
        previous_checksum: None,
    })
    .map_err(Into::into)
}

fn pull_request_failed_envelope(
    bead_id: &BeadId,
    run_id: &RunId,
    timestamp: DateTime<Utc>,
    failure: PullRequestFailure<'_>,
) -> anyhow::Result<EvidenceEnvelope> {
    EvidenceEnvelope::new(EvidenceEnvelopeParts {
        record_id: pull_request_failed_record_id(bead_id, timestamp)?,
        run_id: run_id.clone(),
        bead_id: bead_id.clone(),
        timestamp,
        kind: EvidenceKind::PullRequestFailed,
        metadata: pull_request_failed_metadata(failure.branch, failure.error),
        previous_checksum: None,
    })
    .map_err(Into::into)
}

fn agent_request_timestamp(timestamp: DateTime<Utc>) -> anyhow::Result<DateTime<Utc>> {
    timestamp
        .checked_add_signed(Duration::milliseconds(1))
        .ok_or_else(|| anyhow::anyhow!("agent request timestamp overflow"))
}

fn run_started_record_id(
    bead_id: &BeadId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-run-started-{}",
        bead_id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn prompt_record_id(
    bead_id: &BeadId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-prompt-record-{}",
        bead_id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn agent_request_id(
    bead_id: &BeadId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-agent-request-{}",
        bead_id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn vcs_sync_failed_record_id(
    bead_id: &BeadId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-vcs-failed-{}",
        bead_id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn diff_validation_failed_record_id(
    bead_id: &BeadId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-diff-failed-{}",
        bead_id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn pull_request_created_record_id(
    bead_id: &BeadId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-pr-created-{}",
        bead_id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn pull_request_failed_record_id(
    bead_id: &BeadId,
    timestamp: DateTime<Utc>,
) -> anyhow::Result<EvidenceRecordId> {
    EvidenceRecordId::parse(&format!(
        "ev-{}-pr-failed-{}",
        bead_id.as_str(),
        timestamp.timestamp_millis()
    ))
    .map_err(Into::into)
}

fn prompt_metadata(prompt: &str) -> EvidenceMetadata {
    EvidenceMetadata::from([
        ("prompt_bytes".to_owned(), prompt.len().to_string()),
        ("prompt_chars".to_owned(), prompt.chars().count().to_string()),
        ("redacted".to_owned(), "true".to_owned()),
        ("source".to_owned(), "cli".to_owned()),
    ])
}

fn agent_request_metadata(model: &str, prompt_record: &EvidenceEnvelope) -> EvidenceMetadata {
    EvidenceMetadata::from([
        ("agent".to_owned(), "opencode".to_owned()),
        ("mode".to_owned(), "subprocess".to_owned()),
        ("model".to_owned(), model.to_owned()),
        ("prompt_record_checksum".to_owned(), prompt_record.checksum.as_str().to_owned()),
        ("prompt_record_id".to_owned(), prompt_record.record_id.as_str().to_owned()),
        ("redacted".to_owned(), "true".to_owned()),
        ("status".to_owned(), "requested".to_owned()),
        ("workspace_owner_bead_id".to_owned(), prompt_record.bead_id.as_str().to_owned()),
        ("workspace_owner_run_id".to_owned(), prompt_record.run_id.as_str().to_owned()),
        (
            "workspace_branch_name".to_owned(),
            branch_name_from_ids(prompt_record.bead_id.as_str(), prompt_record.run_id.as_str()),
        ),
        ("workspace_status".to_owned(), "clean_at_agent_request".to_owned()),
    ])
}

fn vcs_sync_failed_metadata(error: &VcsSyncError) -> EvidenceMetadata {
    EvidenceMetadata::from([
        ("command".to_owned(), error.command().to_owned()),
        ("failure_type".to_owned(), "VcsSyncFailed".to_owned()),
        ("redacted".to_owned(), "true".to_owned()),
        ("sanitized_message".to_owned(), error.sanitized_message().to_owned()),
        ("status".to_owned(), "failed".to_owned()),
    ])
}

fn diff_validation_failed_metadata(error: &DiffValidationError) -> EvidenceMetadata {
    let mut metadata = EvidenceMetadata::from([
        ("failure_type".to_owned(), error.failure_type().to_owned()),
        ("redacted".to_owned(), "true".to_owned()),
        ("sanitized_message".to_owned(), error.sanitized_message().to_owned()),
        ("status".to_owned(), "blocked".to_owned()),
    ]);
    if let Some(changed_paths) = error.changed_paths() {
        metadata.insert("changed_paths".to_owned(), changed_paths.to_string());
    }
    metadata
}

fn pull_request_created_metadata(pull_request: &PullRequestOutcome) -> EvidenceMetadata {
    EvidenceMetadata::from([
        ("branch".to_owned(), pull_request.branch.clone()),
        ("redacted".to_owned(), "true".to_owned()),
        ("status".to_owned(), "created".to_owned()),
        ("url".to_owned(), pull_request.url.clone()),
    ])
}

fn pull_request_failed_metadata(branch: &str, error: &PullRequestError) -> EvidenceMetadata {
    let mut metadata = EvidenceMetadata::from([
        ("branch".to_owned(), branch.to_owned()),
        ("failure_type".to_owned(), error.failure_type().to_owned()),
        ("redacted".to_owned(), "true".to_owned()),
        ("sanitized_message".to_owned(), error.sanitized_message().to_owned()),
        ("status".to_owned(), "failed".to_owned()),
    ]);
    if let Some(command) = error.command() {
        metadata.insert("command".to_owned(), command.to_owned());
    }
    metadata
}

fn pull_request_title(bead_id: &BeadId) -> String {
    format!("Oya run for {}", bead_id.as_str())
}

fn pull_request_body(bead_id: &BeadId, run_id: &RunId) -> String {
    format!(
        "Automated Oya PR after green gates for bead `{}` and run `{}`.",
        bead_id.as_str(),
        run_id.as_str()
    )
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
    use chrono::TimeZone;

    use super::*;
    use crate::cli::evidence::{evidence_check_report, EvidenceCheckError};
    use crate::cli::report::RunReport;
    use crate::cli::verify::persist_synthetic_run_verification_gate;
    use crate::cli::workspace::{ensure_workspace_owned_from_status, WorkspaceOwnershipError};

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ReleaseNegativeCase {
        name: &'static str,
        outcome: &'static str,
        failure_type: &'static str,
    }

    impl ReleaseNegativeCase {
        const fn new(
            name: &'static str,
            outcome: &'static str,
            failure_type: &'static str,
        ) -> Self {
            Self { name, outcome, failure_type }
        }
    }

    #[test]
    fn run_started_is_persisted_before_blocked_output() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();

        let output =
            persist_run_skeleton(&db, bead_id, "noop", "zai-coding-plan/glm-5", timestamp).unwrap();
        let run_id = RunId::parse(&output.run_id).unwrap();
        let evidence = db.load_evidence(&run_id).unwrap();

        assert_eq!(output.bead_id, "demo");
        assert_eq!(output.status, "blocked");
        assert_eq!(output.evidence_records, 3);
        assert_eq!(output.agent_run_id, None);
        assert_eq!(output.failure_category, None);
        assert_eq!(evidence.len(), 3);
        assert_eq!(evidence[0].kind, EvidenceKind::RunStarted);
        assert_eq!(evidence[0].previous_checksum, None);
    }

    #[test]
    fn prompt_record_is_persisted_with_sanitized_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let prompt = "super-secret-token";

        let output =
            persist_run_skeleton(&db, bead_id, prompt, "zai-coding-plan/glm-5", timestamp).unwrap();
        let run_id = RunId::parse(&output.run_id).unwrap();
        let evidence = db.load_evidence(&run_id).unwrap();
        let prompt_record_json = evidence[1].to_canonical_json().unwrap();

        assert_eq!(evidence[1].kind, EvidenceKind::PromptRecord);
        assert_eq!(evidence[1].previous_checksum, Some(evidence[0].checksum.clone()));
        assert_eq!(evidence[1].metadata.get("redacted"), Some(&"true".to_owned()));
        assert_eq!(evidence[1].metadata.get("prompt_chars"), Some(&"18".to_owned()));
        assert!(!prompt_record_json.contains(prompt));
    }

    #[test]
    fn agent_request_is_persisted_before_external_action() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();

        let output =
            persist_run_skeleton(&db, bead_id, "super-secret-token", "bad/model", timestamp)
                .unwrap();
        let run_id = RunId::parse(&output.run_id).unwrap();
        let evidence = db.load_evidence(&run_id).unwrap();
        let agent_request_json = evidence[2].to_canonical_json().unwrap();

        assert_eq!(output.agent_request_id, evidence[2].record_id.as_str());
        assert_eq!(evidence[2].kind, EvidenceKind::AgentRequest);
        assert_eq!(evidence[2].previous_checksum, Some(evidence[1].checksum.clone()));
        assert_eq!(evidence[2].metadata.get("agent"), Some(&"opencode".to_owned()));
        assert_eq!(evidence[2].metadata.get("model"), Some(&"bad/model".to_owned()));
        assert_eq!(evidence[2].metadata.get("status"), Some(&"requested".to_owned()));
        assert_eq!(evidence[2].metadata.get("workspace_owner_bead_id"), Some(&"demo".to_owned()));
        assert_eq!(
            evidence[2].metadata.get("workspace_owner_run_id"),
            Some(&"run-demo".to_owned())
        );
        assert_eq!(
            evidence[2].metadata.get("workspace_branch_name"),
            Some(&"oya/demo-run-demo".to_owned())
        );
        assert_eq!(
            evidence[2].metadata.get("workspace_status"),
            Some(&"clean_at_agent_request".to_owned())
        );
        assert!(!agent_request_json.contains("super-secret-token"));
    }

    #[test]
    fn vcs_sync_failure_records_typed_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let run_id = RunId::parse("run-demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let error = crate::cli::workspace::VcsSyncError::VcsSyncFailed {
            command: "git fetch origin",
            message: "[redacted] fatal: authentication failed".to_owned(),
        };

        let failure = persist_vcs_sync_failed(&db, &bead_id, &run_id, timestamp, &error).unwrap();
        let evidence = db.load_evidence(&run_id).unwrap();
        let failure_json = failure.to_canonical_json().unwrap();

        assert_eq!(evidence.len(), 1);
        assert_eq!(failure.kind, EvidenceKind::VcsSyncFailed);
        assert_eq!(failure.metadata.get("failure_type"), Some(&"VcsSyncFailed".to_owned()));
        assert_eq!(failure.metadata.get("status"), Some(&"failed".to_owned()));
        assert_eq!(failure.metadata.get("command"), Some(&"git fetch origin".to_owned()));
        assert_eq!(
            failure.metadata.get("sanitized_message"),
            Some(&"[redacted] fatal: authentication failed".to_owned())
        );
        assert!(!failure_json.contains("server-secret-token"));
    }

    #[test]
    fn diff_validation_records_empty_diff_blocking_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let run_id = RunId::parse("run-demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let error = crate::cli::workspace::DiffValidationError::EmptyDiff { changed_paths: 0 };

        let failure =
            persist_diff_validation_failed(&db, &bead_id, &run_id, timestamp, &error).unwrap();
        let evidence = db.load_evidence(&run_id).unwrap();

        assert_eq!(evidence.len(), 1);
        assert_eq!(failure.kind, EvidenceKind::DiffValidationFailed);
        assert_eq!(failure.metadata.get("failure_type"), Some(&"EmptyDiff".to_owned()));
        assert_eq!(failure.metadata.get("status"), Some(&"blocked".to_owned()));
        assert_eq!(failure.metadata.get("changed_paths"), Some(&"0".to_owned()));
        assert_eq!(
            failure.metadata.get("sanitized_message"),
            Some(&"empty diff blocks PR creation".to_owned())
        );
    }

    #[test]
    fn pr_creation_records_url_and_branch_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let run_id = RunId::parse("run-demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let pull_request = crate::cli::workspace::PullRequestOutcome {
            branch: "oya/demo-run-demo".to_owned(),
            url: "https://github.com/priorlewis43/oya/pull/123".to_owned(),
        };

        let evidence =
            persist_pull_request_created(&db, &bead_id, &run_id, timestamp, &pull_request).unwrap();
        let persisted = db.load_evidence(&run_id).unwrap();

        assert_eq!(persisted.len(), 1);
        assert_eq!(evidence.kind, EvidenceKind::PullRequestCreated);
        assert_eq!(evidence.metadata.get("status"), Some(&"created".to_owned()));
        assert_eq!(evidence.metadata.get("branch"), Some(&"oya/demo-run-demo".to_owned()));
        assert_eq!(
            evidence.metadata.get("url"),
            Some(&"https://github.com/priorlewis43/oya/pull/123".to_owned())
        );
    }

    #[test]
    #[cfg(unix)]
    fn agent_run_result_persists_exit_status_sanitized_message_bounded_output_and_duration() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let stderr = b"ProviderModelNotFoundError: token=super-secret-token";
        let skeleton = persist_run_skeleton_evidence(
            &db,
            bead_id,
            "super-secret-token",
            "bad/model",
            timestamp,
        )
        .unwrap();
        let result =
            AgentRunResult::from_process(std::process::ExitStatus::from_raw(256), b"", stderr);

        let agent_run =
            persist_agent_run(&db, "bad/model", &skeleton.agent_request, &result, timestamp)
                .unwrap();
        let run_id = RunId::parse("run-demo").unwrap();
        let evidence = db.load_evidence(&run_id).unwrap();
        let output = RunSkeletonOutput::from_agent_run(
            &skeleton.agent_request,
            &agent_run,
            &result,
            evidence.len(),
        );
        let agent_run_json = agent_run.to_canonical_json().unwrap();

        assert_eq!(output.status, "failed");
        assert_eq!(output.failure_category, Some("invalid_model".to_owned()));
        assert_eq!(output.agent_run_id, Some(agent_run.record_id.as_str().to_owned()));
        assert_eq!(evidence.len(), 4);
        assert_eq!(evidence[3].kind, EvidenceKind::AgentRun);
        assert_eq!(evidence[3].previous_checksum, Some(evidence[2].checksum.clone()));
        assert_eq!(evidence[3].metadata.get("duration_ms"), Some(&"1".to_owned()));
        assert_eq!(evidence[3].metadata.get("exit_code"), Some(&"1".to_owned()));
        assert_eq!(evidence[3].metadata.get("failure_category"), Some(&"invalid_model".to_owned()));
        assert_eq!(
            evidence[3].metadata.get("sanitized_message"),
            Some(&"opencode invalid model".to_owned())
        );
        assert_eq!(
            evidence[3].metadata.get("stderr_original_bytes"),
            Some(&stderr.len().to_string())
        );
        assert_eq!(evidence[3].metadata.get("stderr_preview"), Some(&"[redacted]".to_owned()));
        assert_eq!(evidence[3].metadata.get("stderr_truncated"), Some(&"false".to_owned()));
        assert!(!agent_run_json.contains("super-secret-token"));
        assert!(!agent_run_json.contains("Command"));
    }

    #[test]
    #[cfg(unix)]
    fn oya_run_final_verdict_passes_only_after_moon_verification_passes() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo-fix").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let skeleton =
            persist_run_skeleton_evidence(&db, bead_id, "noop", "zai-coding-plan/glm-5", timestamp)
                .unwrap();
        let result =
            AgentRunResult::from_process(std::process::ExitStatus::from_raw(0), b"done", b"");
        let agent_run = persist_agent_run(
            &db,
            "zai-coding-plan/glm-5",
            &skeleton.agent_request,
            &result,
            timestamp,
        )
        .unwrap();
        let verification = persist_synthetic_run_verification_gate(
            &db,
            &agent_run.bead_id,
            &GateId::Fmt.model(),
            std::process::ExitStatus::from_raw(0),
            timestamp,
        )
        .unwrap();
        let mut output = RunSkeletonOutput::from_agent_run(
            &skeleton.agent_request,
            &agent_run,
            &result,
            db.load_evidence(&agent_run.run_id).unwrap().len(),
        );
        output.verification = Some(RunVerificationSummary::from_result(&verification));
        let output = output.with_verdict_from_evidence(&db).unwrap();

        assert_eq!(output.status, "completed");
        assert_eq!(output.phase, "completed");
        assert_eq!(output.verdict, "pass");
        assert_eq!(output.evidence_records, 6);
        assert_eq!(
            output.verification.as_ref().map(|summary| summary.status.as_str()),
            Some("passed")
        );
    }

    #[test]
    #[cfg(unix)]
    fn full_loop_fake_agent_proves_run_evidence_and_gate_behavior() {
        let (_dir, db, run_id, output) = fake_agent_full_loop_output_for_prompt("noop");
        let evidence = db.load_evidence(&run_id).unwrap();
        let check = evidence_check_report(&run_id, evidence.as_slice()).unwrap();
        let report = RunReport::from_evidence(&run_id, evidence.as_slice()).unwrap();

        assert_fake_agent_output(&output);
        assert_eq!(check.status(), "valid");
        assert_eq!(check.evidence_records(), 6);
        assert_eq!(report.verdict(), "pass");
        assert!(report.has_gate("fmt"));
        assert_eq!(report.agent_sanitized_message(), Some("opencode completed"));
        assert_eq!(report.agent_stdout_preview(), Some("fake agent completed"));
        assert_eq!(enforce_run_exit_contract(&output), Ok(()));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn real_small_bead_demo_reaches_green_verification_with_real_moon_gate() {
        let (_dir, db, run_id, output) =
            real_moon_full_loop_output_for_prompt("make-failing-test-pass").await;
        let evidence = db.load_evidence(&run_id).unwrap();
        let report = RunReport::from_evidence(&run_id, evidence.as_slice()).unwrap();
        let prompt_record = prompt_record(evidence.as_slice());

        assert_eq!(output.verdict, "pass");
        assert_eq!(output.status, "completed");
        assert_eq!(report.verdict(), "pass");
        assert_eq!(prompt_record.metadata.get("prompt_bytes"), Some(&"22".to_owned()));
        assert_eq!(prompt_record.metadata.get("prompt_chars"), Some(&"22".to_owned()));
    }

    #[test]
    #[cfg(unix)]
    fn real_small_bead_demo_can_reach_typed_blocked_state() {
        let (_dir, db, run_id, output) =
            fake_agent_blocked_output_for_prompt("make-failing-test-pass");
        let evidence = db.load_evidence(&run_id).unwrap();
        let prompt_record = prompt_record(evidence.as_slice());

        assert_eq!(output.phase, "blocked");
        assert_eq!(output.verdict, "fail");
        assert_eq!(output.status, "blocked");
        assert_eq!(prompt_record.metadata.get("prompt_bytes"), Some(&"22".to_owned()));
        assert_eq!(prompt_record.metadata.get("prompt_chars"), Some(&"22".to_owned()));
        assert_eq!(
            enforce_run_exit_contract(&output),
            Err(RunExitCodeError::VerificationFailed { verdict: "fail".to_owned() })
        );
    }

    #[cfg(unix)]
    async fn real_moon_full_loop_output_for_prompt(
        prompt: &str,
    ) -> (tempfile::TempDir, StateDb, RunId, RunSkeletonOutput) {
        let (dir, db, timestamp) = fake_agent_test_context();
        let skeleton = fake_agent_skeleton(&db, timestamp, prompt);
        let (result, agent_run) = fake_successful_agent_run(&db, &skeleton, timestamp);
        let verification =
            persist_run_verification_gate(&db, &agent_run.bead_id, &GateId::Fmt.model())
                .await
                .unwrap();
        let output = fake_agent_verified_output(&db, &skeleton, &agent_run, &result, &verification);
        (dir, db, agent_run.run_id.clone(), output)
    }

    #[cfg(unix)]
    fn fake_agent_blocked_output_for_prompt(
        prompt: &str,
    ) -> (tempfile::TempDir, StateDb, RunId, RunSkeletonOutput) {
        let (dir, db, timestamp) = fake_agent_test_context();
        let skeleton = fake_agent_skeleton(&db, timestamp, prompt);
        let (result, agent_run) = fake_successful_agent_run(&db, &skeleton, timestamp);
        let verification = fake_failed_verification(&db, &agent_run, timestamp);
        let output = fake_agent_verified_output(&db, &skeleton, &agent_run, &result, &verification);
        (dir, db, agent_run.run_id.clone(), output)
    }

    fn prompt_record(evidence: &[EvidenceEnvelope]) -> &EvidenceEnvelope {
        evidence.iter().find(|record| record.kind == EvidenceKind::PromptRecord).unwrap()
    }

    #[cfg(unix)]
    fn fake_agent_full_loop_output_for_prompt(
        prompt: &str,
    ) -> (tempfile::TempDir, StateDb, RunId, RunSkeletonOutput) {
        let (dir, db, timestamp) = fake_agent_test_context();
        let skeleton = fake_agent_skeleton(&db, timestamp, prompt);
        let (result, agent_run) = fake_successful_agent_run(&db, &skeleton, timestamp);
        let verification = fake_successful_verification(&db, &agent_run, timestamp);
        let output = fake_agent_verified_output(&db, &skeleton, &agent_run, &result, &verification);
        (dir, db, agent_run.run_id.clone(), output)
    }

    #[cfg(unix)]
    fn fake_agent_test_context() -> (tempfile::TempDir, StateDb, DateTime<Utc>) {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        (dir, db, timestamp)
    }

    #[cfg(unix)]
    fn fake_agent_skeleton(
        db: &StateDb,
        timestamp: DateTime<Utc>,
        prompt: &str,
    ) -> RunSkeletonEvidence {
        let bead_id = BeadId::parse("demo-fix").unwrap();
        persist_run_skeleton_evidence(db, bead_id, prompt, "zai-coding-plan/glm-5", timestamp)
            .unwrap()
    }

    #[cfg(unix)]
    fn fake_successful_agent_run(
        db: &StateDb,
        skeleton: &RunSkeletonEvidence,
        timestamp: DateTime<Utc>,
    ) -> (AgentRunResult, EvidenceEnvelope) {
        let result = AgentRunResult::from_process(
            std::process::ExitStatus::from_raw(0),
            b"fake agent completed",
            b"",
        );
        let agent_run = persist_agent_run(
            db,
            "zai-coding-plan/glm-5",
            &skeleton.agent_request,
            &result,
            timestamp,
        )
        .unwrap();
        (result, agent_run)
    }

    #[cfg(unix)]
    fn fake_successful_verification(
        db: &StateDb,
        agent_run: &EvidenceEnvelope,
        timestamp: DateTime<Utc>,
    ) -> RunVerificationResult {
        persist_synthetic_run_verification_gate(
            db,
            &agent_run.bead_id,
            &GateId::Fmt.model(),
            std::process::ExitStatus::from_raw(0),
            timestamp,
        )
        .unwrap()
    }

    #[cfg(unix)]
    fn fake_failed_verification(
        db: &StateDb,
        agent_run: &EvidenceEnvelope,
        timestamp: DateTime<Utc>,
    ) -> RunVerificationResult {
        persist_synthetic_run_verification_gate(
            db,
            &agent_run.bead_id,
            &GateId::Fmt.model(),
            std::process::ExitStatus::from_raw(256),
            timestamp,
        )
        .unwrap()
    }

    fn fake_agent_verified_output(
        db: &StateDb,
        skeleton: &RunSkeletonEvidence,
        agent_run: &EvidenceEnvelope,
        result: &AgentRunResult,
        verification: &RunVerificationResult,
    ) -> RunSkeletonOutput {
        let mut output = RunSkeletonOutput::from_agent_run(
            &skeleton.agent_request,
            agent_run,
            result,
            db.load_evidence(&agent_run.run_id).unwrap().len(),
        );
        output.verification = Some(RunVerificationSummary::from_result(verification));
        output.with_verdict_from_evidence(db).unwrap()
    }

    fn assert_fake_agent_output(output: &RunSkeletonOutput) {
        assert_eq!(output.phase, "completed");
        assert_eq!(output.verdict, "pass");
        assert_eq!(output.status, "completed");
        assert_eq!(output.evidence_records, 6);
    }

    #[test]
    #[cfg(unix)]
    fn oya_run_final_verdict_fails_when_moon_verification_fails() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo-fix").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let skeleton =
            persist_run_skeleton_evidence(&db, bead_id, "noop", "zai-coding-plan/glm-5", timestamp)
                .unwrap();
        let result = AgentRunResult::from_process(
            std::process::ExitStatus::from_raw(0),
            b"agent says success",
            b"",
        );
        let agent_run = persist_agent_run(
            &db,
            "zai-coding-plan/glm-5",
            &skeleton.agent_request,
            &result,
            timestamp,
        )
        .unwrap();
        let verification = persist_synthetic_run_verification_gate(
            &db,
            &agent_run.bead_id,
            &GateId::Fmt.model(),
            std::process::ExitStatus::from_raw(256),
            timestamp,
        )
        .unwrap();
        let mut output = RunSkeletonOutput::from_agent_run(
            &skeleton.agent_request,
            &agent_run,
            &result,
            db.load_evidence(&agent_run.run_id).unwrap().len(),
        );
        output.verification = Some(RunVerificationSummary::from_result(&verification));
        let output = output.with_verdict_from_evidence(&db).unwrap();
        let evidence = db.load_evidence(&agent_run.run_id).unwrap();

        assert_eq!(output.status, "blocked");
        assert_eq!(output.phase, "blocked");
        assert_eq!(output.verdict, "fail");
        assert_eq!(output.evidence_records, 7);
        assert_eq!(evidence.last().unwrap().kind, EvidenceKind::Finding);
        assert_eq!(
            output.verification.as_ref().and_then(|summary| summary.finding_id.as_ref()),
            Some(&evidence.last().unwrap().record_id.as_str().to_owned())
        );
    }

    #[test]
    fn exit_code_contract_allows_demo_fix_only_after_verified_pass() {
        let output = exit_code_contract_output(
            "completed",
            "pass",
            "passed",
            None,
            Some("ev-demo-fix-agent-run-1"),
        );

        let result = enforce_run_exit_contract(&output);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn exit_code_contract_rejects_demo_fix_failed_verdict() {
        let output = exit_code_contract_output(
            "blocked",
            "fail",
            "failed",
            Some("moon verification failed"),
            Some("ev-demo-fix-agent-run-1"),
        );

        let result = enforce_run_exit_contract(&output);

        assert_eq!(
            result,
            Err(RunExitCodeError::VerificationFailed { verdict: "fail".to_owned() })
        );
    }

    #[test]
    fn exit_code_contract_rejects_demo_fix_without_verification() {
        let mut output = exit_code_contract_output(
            "agent_ran",
            "inconclusive",
            "passed",
            None,
            Some("ev-demo-fix-agent-run-1"),
        );
        output.verification = None;

        let result = enforce_run_exit_contract(&output);

        assert_eq!(
            result,
            Err(RunExitCodeError::VerificationIncomplete { verdict: "inconclusive".to_owned() })
        );
    }

    #[test]
    fn exit_code_contract_rejects_agent_failure_before_verification() {
        let mut output = exit_code_contract_output(
            "blocked",
            "fail",
            "passed",
            Some("opencode invalid model"),
            Some("ev-demo-fix-agent-run-1"),
        );
        output.verification = None;
        output.failure_category = Some("invalid_model".to_owned());

        let result = enforce_run_exit_contract(&output);

        assert_eq!(
            result,
            Err(RunExitCodeError::AgentFailed {
                category: "invalid_model".to_owned(),
                record: "ev-demo-fix-agent-run-1".to_owned()
            })
        );
    }

    #[test]
    fn bead_concurrency_second_same_bead_run_returns_bead_already_running() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();

        persist_run_skeleton_evidence(
            &db,
            bead_id.clone(),
            "noop",
            "zai-coding-plan/glm-5",
            timestamp,
        )
        .unwrap();
        let result =
            persist_run_skeleton_evidence(&db, bead_id, "noop", "zai-coding-plan/glm-5", timestamp);
        let evidence = db.load_evidence(&RunId::parse("run-demo").unwrap()).unwrap();
        let error = result.unwrap_err();

        assert!(matches!(
            error.downcast_ref::<RunStartError>(),
            Some(RunStartError::BeadAlreadyRunning { bead_id, run_id, phase })
                if bead_id == "demo" && run_id == "run-demo" && phase == "agent_requested"
        ));
        assert_eq!(evidence.len(), 3);
    }

    #[test]
    fn release_negative_e2e_matrix_proves_typed_blocked_and_failed_outcomes() {
        let matrix = release_negative_e2e_matrix();

        assert_eq!(matrix.len(), 4);
        assert!(matrix.contains(&ReleaseNegativeCase::new(
            "invalid_model",
            "failed",
            "invalid_model",
        )));
        assert!(matrix.contains(&ReleaseNegativeCase::new(
            "dirty_workspace",
            "blocked",
            "WorkingTreeInvalid",
        )));
        assert!(matrix.contains(&ReleaseNegativeCase::new(
            "evidence_tamper",
            "blocked",
            "EvidenceIntegrityViolation",
        )));
        assert!(matrix.contains(&ReleaseNegativeCase::new(
            "concurrency",
            "blocked",
            "BeadAlreadyRunning",
        )));
    }

    fn release_negative_e2e_matrix() -> Vec<ReleaseNegativeCase> {
        vec![
            release_negative_invalid_model(),
            release_negative_dirty_workspace(),
            release_negative_evidence_tamper(),
            release_negative_concurrency(),
        ]
    }

    fn release_negative_invalid_model() -> ReleaseNegativeCase {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let skeleton =
            persist_run_skeleton_evidence(&db, bead_id, "noop", "bad/model", timestamp).unwrap();
        let result =
            AgentRunResult::from_server_failure(500, b"Model not found: token=server-secret-token");

        let output = persist_agent_result(&db, &skeleton, "bad/model", &result).unwrap();
        let evidence = db.load_evidence(&RunId::parse("run-demo").unwrap()).unwrap();
        let agent_run_json = evidence[3].to_canonical_json().unwrap();

        assert_eq!(output.phase, "blocked");
        assert_eq!(output.verdict, "fail");
        assert_eq!(output.status, "failed");
        assert_eq!(output.failure_category, Some("invalid_model".to_owned()));
        assert_eq!(output.error, Some("opencode invalid model".to_owned()));
        assert_eq!(evidence[3].kind, EvidenceKind::AgentRun);
        assert!(!agent_run_json.contains("server-secret-token"));
        ReleaseNegativeCase::new("invalid_model", "failed", "invalid_model")
    }

    fn release_negative_dirty_workspace() -> ReleaseNegativeCase {
        let result = ensure_workspace_owned_from_status("run-demo", " M src/secret.rs\n?? .env\n");

        assert!(matches!(
            &result,
            Err(WorkspaceOwnershipError::WorkingTreeInvalid { run_id, pending_changes })
                if run_id == "run-demo" && *pending_changes == 2
        ));
        let message = result.unwrap_err().to_string();
        assert!(message.contains("working_tree_invalid"));
        assert!(!message.contains(".env"));
        ReleaseNegativeCase::new("dirty_workspace", "blocked", "WorkingTreeInvalid")
    }

    fn release_negative_evidence_tamper() -> ReleaseNegativeCase {
        let bead_id = BeadId::parse("demo").unwrap();
        let run_id = RunId::parse("run-demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let first = run_started_envelope(&bead_id, &run_id, timestamp).unwrap();
        let mut second = prompt_record_envelope(&bead_id, &run_id, "noop", &first).unwrap();
        second.metadata.insert("secret".to_owned(), "server-secret-token".to_owned());

        let result = evidence_check_report(&run_id, &[first, second]);

        assert!(matches!(
            &result,
            Err(EvidenceCheckError::EvidenceIntegrityViolation { record_id })
                if record_id.starts_with("ev-demo-prompt-record-")
        ));
        let message = result.unwrap_err().to_string();
        assert!(message.contains("EvidenceIntegrityViolation"));
        assert!(!message.contains("server-secret-token"));
        ReleaseNegativeCase::new("evidence_tamper", "blocked", "EvidenceIntegrityViolation")
    }

    fn release_negative_concurrency() -> ReleaseNegativeCase {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();

        persist_run_skeleton_evidence(
            &db,
            bead_id.clone(),
            "noop",
            "zai-coding-plan/glm-5",
            timestamp,
        )
        .unwrap();
        let result =
            persist_run_skeleton_evidence(&db, bead_id, "noop", "zai-coding-plan/glm-5", timestamp);
        let evidence = db.load_evidence(&RunId::parse("run-demo").unwrap()).unwrap();
        let error = result.unwrap_err();

        assert!(matches!(
            error.downcast_ref::<RunStartError>(),
            Some(RunStartError::BeadAlreadyRunning { bead_id, run_id, phase })
                if bead_id == "demo" && run_id == "run-demo" && phase == "agent_requested"
        ));
        assert_eq!(evidence.len(), 3);
        ReleaseNegativeCase::new("concurrency", "blocked", "BeadAlreadyRunning")
    }

    #[test]
    fn oya_run_demo_fix_rejects_repeat_run_before_duplicate_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo-fix").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();

        persist_run_skeleton_evidence(
            &db,
            bead_id.clone(),
            "noop",
            "zai-coding-plan/glm-5",
            timestamp,
        )
        .unwrap();
        let result =
            persist_run_skeleton_evidence(&db, bead_id, "noop", "zai-coding-plan/glm-5", timestamp);
        let evidence = db.load_evidence(&RunId::parse("run-demo-fix").unwrap()).unwrap();
        let error = result.unwrap_err();

        assert!(matches!(
            error.downcast_ref::<RunStartError>(),
            Some(RunStartError::BeadAlreadyRunning { bead_id, run_id, phase })
                if bead_id == "demo-fix" && run_id == "run-demo-fix" && phase == "agent_requested"
        ));
        assert_eq!(evidence.len(), 3);
    }

    #[test]
    fn server_auth_agent_run_is_persisted_with_sanitized_failure() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let skeleton =
            persist_run_skeleton_evidence(&db, bead_id, "noop", "zai-coding-plan/glm-5", timestamp)
                .unwrap();
        let result =
            AgentRunResult::from_server_failure(401, b"unauthorized: password=server-secret-token");

        let output =
            persist_agent_result(&db, &skeleton, "zai-coding-plan/glm-5", &result).unwrap();
        let evidence = db.load_evidence(&RunId::parse("run-demo").unwrap()).unwrap();
        let agent_run = evidence[3].clone();
        let agent_run_json = agent_run.to_canonical_json().unwrap();

        assert_eq!(output.status, "failed");
        assert_eq!(output.failure_category, Some("server_auth".to_owned()));
        assert_eq!(output.error, Some("opencode server authentication failed".to_owned()));
        assert_eq!(evidence.len(), 4);
        assert_eq!(agent_run.kind, EvidenceKind::AgentRun);
        assert_eq!(agent_run.previous_checksum, Some(evidence[2].checksum.clone()));
        assert_eq!(agent_run.metadata.get("duration_ms"), Some(&"1".to_owned()));
        assert_eq!(
            agent_run.metadata.get("sanitized_message"),
            Some(&"opencode server authentication failed".to_owned())
        );
        assert_eq!(agent_run.metadata.get("mode"), Some(&"server".to_owned()));
        assert_eq!(agent_run.metadata.get("failure_category"), Some(&"server_auth".to_owned()));
        assert_eq!(agent_run.metadata.get("stderr_preview"), Some(&"[redacted]".to_owned()));
        assert!(!agent_run_json.contains("server-secret-token"));
        assert!(!agent_run_json.contains("bad"));
    }

    #[test]
    #[cfg(unix)]
    fn opencode_no_secret_leak_absent_from_agent_run_evidence_and_output() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let bead_id = BeadId::parse("demo").unwrap();
        let timestamp = Utc.timestamp_opt(1_779_999_600, 0).unwrap();
        let stderr = b"ProviderModelNotFoundError: password=server-secret-token\n    at Provider.request (/home/lewis/.cache/opencode/provider.js:42:7)\nTraceback (most recent call last):";
        let skeleton = persist_run_skeleton_evidence(
            &db,
            bead_id,
            "prompt with server-secret-token",
            "bad/model",
            timestamp,
        )
        .unwrap();
        let result =
            AgentRunResult::from_process(std::process::ExitStatus::from_raw(256), b"", stderr);

        let output = persist_agent_result(&db, &skeleton, "bad/model", &result).unwrap();
        let evidence = db.load_evidence(&RunId::parse("run-demo").unwrap()).unwrap();
        let agent_run_json = evidence[3].to_canonical_json().unwrap();
        let output_json = serde_json::to_string(&output).unwrap();

        assert_eq!(output.failure_category, Some("invalid_model".to_owned()));
        assert_eq!(
            evidence[3].metadata.get("stderr_preview"),
            Some(&"[redacted]\n[redacted]\n[redacted]".to_owned())
        );
        assert!(!agent_run_json.contains("server-secret-token"));
        assert!(!agent_run_json.contains("Provider.request"));
        assert!(!agent_run_json.contains("Traceback"));
        assert!(!output_json.contains("server-secret-token"));
        assert!(!output_json.contains("Provider.request"));
    }

    fn exit_code_contract_output(
        phase: &str,
        verdict: &str,
        verification_status: &str,
        error: Option<&str>,
        agent_run_id: Option<&str>,
    ) -> RunSkeletonOutput {
        RunSkeletonOutput {
            run_id: "run-demo-fix".to_owned(),
            bead_id: "demo-fix".to_owned(),
            phase: phase.to_owned(),
            verdict: verdict.to_owned(),
            status: phase.to_owned(),
            evidence_records: 6,
            agent_request_id: "ev-demo-fix-agent-request-1".to_owned(),
            agent_run_id: agent_run_id.map(str::to_owned),
            verification: Some(RunVerificationSummary {
                gate: "fmt".to_owned(),
                moon_task: "oya:fmt".to_owned(),
                status: verification_status.to_owned(),
                exit_code: Some(0),
                gate_run_started_id: "ev-demo-fix-g-fmt-s-1".to_owned(),
                gate_run_finished_id: "ev-demo-fix-g-fmt-f-1".to_owned(),
                finding_id: None,
            }),
            pull_request: None,
            failure_category: None,
            error: error.map(str::to_owned),
            stdout: None,
            stderr: None,
            message: "test output".to_owned(),
        }
    }
}
