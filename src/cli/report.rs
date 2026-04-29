#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;

use super::args::ReportArgs;
use crate::lifecycle::state::StateDb;
use crate::lifecycle::types::{BeadId, EvidenceEnvelope, EvidenceKind, RunId, RunPhase, RunState};

const REDACTED_REPORT_VALUE: &str = "[redacted]";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum ReportError {
    #[error("not_found: run '{run_id}' has no evidence records")]
    EmptyRun { run_id: String },

    #[error("invalid_chain: first evidence record '{record_id}' has previous checksum")]
    FirstRecordHasPreviousChecksum { record_id: String },

    #[error("invalid_chain: record '{record_id}' belongs to run '{actual}' not '{expected}'")]
    RecordRunIdMismatch { record_id: String, actual: String, expected: String },

    #[error("invalid_chain: record '{record_id}' expected previous checksum '{expected}' but found '{actual}'")]
    PreviousChecksumMismatch { record_id: String, expected: String, actual: String },

    #[error("invalid_chain: record '{record_id}' is missing previous checksum")]
    MissingPreviousChecksum { record_id: String },

    #[error("invalid_state: {message}")]
    InvalidState { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RunReport {
    run_id: String,
    bead_id: String,
    phase: String,
    verdict: String,
    status: String,
    evidence_records: usize,
    prompt: Option<PromptReport>,
    agent_result: Option<AgentResultReport>,
    gates: Vec<GateReport>,
    findings: Vec<FindingReport>,
    last_record_id: String,
    last_checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PromptReport {
    record_id: String,
    prompt_bytes: String,
    prompt_chars: String,
    redacted: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct AgentResultReport {
    agent_request_id: Option<String>,
    agent_run_id: String,
    status: String,
    mode: String,
    model: String,
    failure_category: String,
    exit_code: String,
    sanitized_message: String,
    stdout: ReportOutput,
    stderr: ReportOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct GateReport {
    gate: String,
    moon_task: String,
    status: String,
    exit_code: String,
    gate_run_started_id: Option<String>,
    gate_run_finished_id: String,
    finding_id: Option<String>,
    stdout: ReportOutput,
    stderr: ReportOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FindingReport {
    finding_id: String,
    category: String,
    gate: String,
    status: String,
    next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ReportOutput {
    preview: String,
    original_bytes: usize,
    stored_bytes: usize,
    truncated: bool,
    limit_bytes: usize,
}

pub fn report_command(args: ReportArgs) -> anyhow::Result<()> {
    let run_id = RunId::parse(&args.run_id)?;
    let db = StateDb::open(data_dir())?;
    let evidence = db.load_evidence(&run_id)?;
    let report = RunReport::from_evidence(&run_id, evidence.as_slice())?;
    let json = serde_json::to_string_pretty(&report)?;
    println!("{json}");
    Ok(())
}

impl RunReport {
    pub(crate) fn from_evidence(
        run_id: &RunId,
        evidence: &[EvidenceEnvelope],
    ) -> Result<Self, ReportError> {
        let Some(first) = evidence.first() else {
            return Err(ReportError::EmptyRun { run_id: run_id.as_str().to_owned() });
        };
        let Some(last) = evidence.last() else {
            return Err(ReportError::EmptyRun { run_id: run_id.as_str().to_owned() });
        };
        validate_report_evidence_chain(run_id, evidence)?;
        let state = replay_run_state(run_id, &first.bead_id, evidence)?;
        Ok(Self::from_state(state.phase(), first, last, evidence))
    }

    #[cfg(test)]
    pub(crate) fn verdict(&self) -> &str {
        &self.verdict
    }

    #[cfg(test)]
    pub(crate) fn has_gate(&self, gate: &str) -> bool {
        self.gates.iter().any(|report| report.gate == gate)
    }

    #[cfg(test)]
    pub(crate) fn agent_sanitized_message(&self) -> Option<&str> {
        self.agent_result.as_ref().map(|report| report.sanitized_message.as_str())
    }

    #[cfg(test)]
    pub(crate) fn agent_stdout_preview(&self) -> Option<&str> {
        self.agent_result.as_ref().map(|report| report.stdout.preview.as_str())
    }

    fn from_state(
        phase: RunPhase,
        first: &EvidenceEnvelope,
        last: &EvidenceEnvelope,
        evidence: &[EvidenceEnvelope],
    ) -> Self {
        Self {
            run_id: first.run_id.as_str().to_owned(),
            bead_id: first.bead_id.as_str().to_owned(),
            phase: phase.as_str().to_owned(),
            verdict: verdict_from_phase(phase).to_owned(),
            status: status_from_phase(phase).to_owned(),
            evidence_records: evidence.len(),
            prompt: prompt_report(evidence),
            agent_result: agent_result_report(evidence),
            gates: gate_reports(evidence),
            findings: finding_reports(evidence),
            last_record_id: last.record_id.as_str().to_owned(),
            last_checksum: last.checksum.as_str().to_owned(),
        }
    }
}

fn replay_run_state(
    run_id: &RunId,
    bead_id: &BeadId,
    evidence: &[EvidenceEnvelope],
) -> Result<RunState, ReportError> {
    RunState::planned(run_id.clone(), bead_id.clone())
        .apply_evidence_chain(evidence)
        .map_err(|error| ReportError::InvalidState { message: error.to_string() })
}

fn validate_report_evidence_chain(
    run_id: &RunId,
    evidence: &[EvidenceEnvelope],
) -> Result<(), ReportError> {
    validate_report_chain_genesis(evidence)?;
    validate_report_chain_run_id(run_id, evidence)?;
    validate_report_chain_links(evidence)
}

fn validate_report_chain_genesis(evidence: &[EvidenceEnvelope]) -> Result<(), ReportError> {
    let Some(first) = evidence.first() else {
        return Ok(());
    };
    if first.previous_checksum.is_some() {
        return Err(ReportError::FirstRecordHasPreviousChecksum {
            record_id: first.record_id.as_str().to_owned(),
        });
    }
    Ok(())
}

fn validate_report_chain_run_id(
    run_id: &RunId,
    evidence: &[EvidenceEnvelope],
) -> Result<(), ReportError> {
    if let Some(record) = evidence.iter().find(|record| &record.run_id != run_id) {
        return Err(ReportError::RecordRunIdMismatch {
            record_id: record.record_id.as_str().to_owned(),
            actual: record.run_id.as_str().to_owned(),
            expected: run_id.as_str().to_owned(),
        });
    }
    Ok(())
}

fn validate_report_chain_links(evidence: &[EvidenceEnvelope]) -> Result<(), ReportError> {
    if let Some(error) = evidence.windows(2).find_map(report_previous_checksum_error) {
        return Err(error);
    }
    Ok(())
}

fn report_previous_checksum_error(window: &[EvidenceEnvelope]) -> Option<ReportError> {
    let [previous, current] = window else {
        return None;
    };
    match &current.previous_checksum {
        Some(checksum) if checksum == &previous.checksum => None,
        Some(checksum) => Some(ReportError::PreviousChecksumMismatch {
            record_id: current.record_id.as_str().to_owned(),
            expected: previous.checksum.as_str().to_owned(),
            actual: checksum.as_str().to_owned(),
        }),
        None => Some(ReportError::MissingPreviousChecksum {
            record_id: current.record_id.as_str().to_owned(),
        }),
    }
}

fn prompt_report(evidence: &[EvidenceEnvelope]) -> Option<PromptReport> {
    evidence.iter().find(|record| record.kind == EvidenceKind::PromptRecord).map(|record| {
        PromptReport {
            record_id: record.record_id.as_str().to_owned(),
            prompt_bytes: metadata_or_unknown(record, "prompt_bytes"),
            prompt_chars: metadata_or_unknown(record, "prompt_chars"),
            redacted: metadata_or_unknown(record, "redacted"),
        }
    })
}

fn agent_result_report(evidence: &[EvidenceEnvelope]) -> Option<AgentResultReport> {
    evidence.iter().rev().find(|record| record.kind == EvidenceKind::AgentRun).map(|record| {
        AgentResultReport {
            agent_request_id: linked_record_id(evidence, record),
            agent_run_id: record.record_id.as_str().to_owned(),
            status: metadata_or_unknown(record, "status"),
            mode: metadata_or_unknown(record, "mode"),
            model: metadata_or_unknown(record, "model"),
            failure_category: metadata_or_unknown(record, "failure_category"),
            exit_code: metadata_or_unknown(record, "exit_code"),
            sanitized_message: metadata_or_unknown(record, "sanitized_message"),
            stdout: ReportOutput::from_metadata(record, "stdout"),
            stderr: ReportOutput::from_metadata(record, "stderr"),
        }
    })
}

fn gate_reports(evidence: &[EvidenceEnvelope]) -> Vec<GateReport> {
    evidence
        .iter()
        .filter(|record| record.kind == EvidenceKind::GateRunFinished)
        .map(|record| GateReport::from_evidence(evidence, record))
        .collect()
}

fn finding_reports(evidence: &[EvidenceEnvelope]) -> Vec<FindingReport> {
    evidence
        .iter()
        .filter(|record| record.kind == EvidenceKind::Finding)
        .map(FindingReport::from)
        .collect()
}

impl GateReport {
    fn from_evidence(evidence: &[EvidenceEnvelope], record: &EvidenceEnvelope) -> Self {
        Self {
            gate: metadata_or_unknown(record, "gate"),
            moon_task: metadata_or_unknown(record, "moon_task"),
            status: metadata_or_unknown(record, "status"),
            exit_code: metadata_or_unknown(record, "exit_code"),
            gate_run_started_id: linked_record_id(evidence, record),
            gate_run_finished_id: record.record_id.as_str().to_owned(),
            finding_id: finding_for_gate(evidence, record),
            stdout: ReportOutput::from_metadata(record, "stdout"),
            stderr: ReportOutput::from_metadata(record, "stderr"),
        }
    }
}

impl FindingReport {
    fn from(record: &EvidenceEnvelope) -> Self {
        Self {
            finding_id: record.record_id.as_str().to_owned(),
            category: metadata_or_unknown(record, "category"),
            gate: metadata_or_unknown(record, "gate"),
            status: metadata_or_unknown(record, "status"),
            next_action: metadata_or_unknown(record, "next_action"),
        }
    }
}

impl ReportOutput {
    fn from_metadata(record: &EvidenceEnvelope, prefix: &str) -> Self {
        Self {
            preview: metadata_or_empty(record, &format!("{prefix}_preview")),
            original_bytes: metadata_usize_or_zero(record, &format!("{prefix}_original_bytes")),
            stored_bytes: metadata_usize_or_zero(record, &format!("{prefix}_stored_bytes")),
            truncated: metadata_bool_or_false(record, &format!("{prefix}_truncated")),
            limit_bytes: metadata_usize_or_zero(record, &format!("{prefix}_limit_bytes")),
        }
    }
}

fn linked_record_id(evidence: &[EvidenceEnvelope], record: &EvidenceEnvelope) -> Option<String> {
    record.previous_checksum.as_ref().and_then(|checksum| {
        evidence
            .iter()
            .find(|candidate| &candidate.checksum == checksum)
            .map(|candidate| candidate.record_id.as_str().to_owned())
    })
}

fn finding_for_gate(
    evidence: &[EvidenceEnvelope],
    gate_record: &EvidenceEnvelope,
) -> Option<String> {
    evidence
        .iter()
        .find(|record| {
            record.kind == EvidenceKind::Finding
                && record.metadata.get("gate_record_id").map(String::as_str)
                    == Some(gate_record.record_id.as_str())
        })
        .map(|record| record.record_id.as_str().to_owned())
}

fn metadata_or_unknown(record: &EvidenceEnvelope, key: &str) -> String {
    match record.metadata.get(key) {
        Some(value) => sanitize_report_text(value),
        None => "unknown".to_owned(),
    }
}

fn metadata_or_empty(record: &EvidenceEnvelope, key: &str) -> String {
    match record.metadata.get(key) {
        Some(value) => sanitize_report_text(value),
        None => String::new(),
    }
}

fn sanitize_report_text(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    input.lines().map(redact_report_line).collect::<Vec<_>>().join("\n")
}

fn redact_report_line(line: &str) -> &str {
    if is_sensitive_report_line(line) {
        REDACTED_REPORT_VALUE
    } else {
        line
    }
}

fn is_sensitive_report_line(line: &str) -> bool {
    let normalized = line.to_ascii_lowercase();
    contains_secret_marker(&normalized) || contains_stack_trace_marker(&normalized)
}

fn contains_secret_marker(normalized: &str) -> bool {
    normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("api_key")
        || normalized.contains("apikey")
        || normalized.contains("authorization")
        || normalized.contains("bearer ")
}

fn contains_stack_trace_marker(normalized: &str) -> bool {
    let trimmed = normalized.trim_start();
    normalized.contains("stack trace")
        || normalized.contains("traceback")
        || trimmed.starts_with("at ")
        || trimmed.starts_with("file ")
}

fn metadata_usize_or_zero(record: &EvidenceEnvelope, key: &str) -> usize {
    record.metadata.get(key).and_then(|value| value.parse::<usize>().ok()).map_or(0, |value| value)
}

fn metadata_bool_or_false(record: &EvidenceEnvelope, key: &str) -> bool {
    record.metadata.get(key).and_then(|value| value.parse::<bool>().ok()).is_some_and(|value| value)
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

fn data_dir() -> PathBuf {
    match std::env::var("OYA_DATA_DIR") {
        Ok(value) => PathBuf::from(value),
        Err(_) => PathBuf::from(".oya-lite"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::lifecycle::types::{
        EvidenceChecksum, EvidenceEnvelopeParts, EvidenceMetadata, EvidenceRecordId,
    };

    #[test]
    fn report_from_evidence_includes_prompt_agent_result_gates_findings_and_verdict() {
        let evidence = failed_run_evidence();
        let report = RunReport::from_evidence(&run_id(), evidence.as_slice()).unwrap();
        let json = serde_json::to_string(&report).unwrap();

        assert_eq!(report.phase, "blocked");
        assert_eq!(report.verdict, "fail");
        assert_eq!(report.prompt.as_ref().map(|prompt| prompt.redacted.as_str()), Some("true"));
        assert_eq!(
            report.agent_result.as_ref().map(|agent| agent.status.as_str()),
            Some("succeeded")
        );
        assert_eq!(report.gates.len(), 1);
        assert_eq!(report.gates[0].finding_id, Some("ev-demo-g-fmt-fn-006".to_owned()));
        assert_eq!(report.gates[0].stderr.preview, "[redacted]\n[redacted]");
        assert_eq!(report.findings[0].category, "format");
        assert!(!json.contains("super-secret-token"));
        assert!(!json.contains("Traceback"));
    }

    #[test]
    fn report_from_evidence_derives_pass_verdict_from_run_state() {
        let evidence = completed_run_evidence();
        let report = RunReport::from_evidence(&run_id(), evidence.as_slice()).unwrap();

        assert_eq!(report.phase, "completed");
        assert_eq!(report.verdict, "pass");
        assert_eq!(report.gates[0].status, "passed");
        assert!(report.findings.is_empty());
    }

    #[test]
    fn report_from_evidence_rejects_broken_checksum_chain() {
        let mut evidence = completed_run_evidence();
        evidence[2].previous_checksum =
            Some(EvidenceChecksum::parse("fnv1a64:0000000000000000").unwrap());

        let error = RunReport::from_evidence(&run_id(), evidence.as_slice()).unwrap_err();

        assert_eq!(
            error,
            ReportError::PreviousChecksumMismatch {
                record_id: "ev-demo-agent-request-003".to_owned(),
                expected: evidence[1].checksum.as_str().to_owned(),
                actual: "fnv1a64:0000000000000000".to_owned(),
            }
        );
    }

    #[test]
    fn report_from_evidence_rejects_missing_run_with_typed_error() {
        let error = RunReport::from_evidence(&run_id(), &[]).unwrap_err();

        assert_eq!(error, ReportError::EmptyRun { run_id: "run-demo".to_owned() });
    }

    fn completed_run_evidence() -> Vec<EvidenceEnvelope> {
        let started = record("ev-demo-run-started-001", 0, EvidenceKind::RunStarted, empty(), None);
        let prompt = record(
            "ev-demo-prompt-record-002",
            1,
            EvidenceKind::PromptRecord,
            prompt_metadata(),
            Some(started.checksum.clone()),
        );
        let request = record(
            "ev-demo-agent-request-003",
            2,
            EvidenceKind::AgentRequest,
            agent_request_metadata(),
            Some(prompt.checksum.clone()),
        );
        let agent = record(
            "ev-demo-agent-run-004",
            3,
            EvidenceKind::AgentRun,
            agent_metadata(),
            Some(request.checksum.clone()),
        );
        let gate_started = record(
            "ev-demo-g-fmt-s-005",
            4,
            EvidenceKind::GateRunStarted,
            gate_started_metadata(),
            Some(agent.checksum.clone()),
        );
        let gate_finished = record(
            "ev-demo-g-fmt-f-006",
            5,
            EvidenceKind::GateRunFinished,
            gate_finished_metadata("passed"),
            Some(gate_started.checksum.clone()),
        );
        vec![started, prompt, request, agent, gate_started, gate_finished]
    }

    fn failed_run_evidence() -> Vec<EvidenceEnvelope> {
        let mut evidence = completed_run_evidence();
        let gate_finished = record(
            "ev-demo-g-fmt-f-005",
            5,
            EvidenceKind::GateRunFinished,
            gate_finished_metadata("failed"),
            Some(evidence[4].checksum.clone()),
        );
        let finding = record(
            "ev-demo-g-fmt-fn-006",
            6,
            EvidenceKind::Finding,
            finding_metadata(&gate_finished),
            Some(gate_finished.checksum.clone()),
        );
        evidence.truncate(5);
        evidence.extend([gate_finished, finding]);
        evidence
    }

    fn record(
        record_id: &str,
        offset_seconds: i64,
        kind: EvidenceKind,
        metadata: EvidenceMetadata,
        previous_checksum: Option<EvidenceChecksum>,
    ) -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse(record_id).unwrap(),
            run_id: run_id(),
            bead_id: BeadId::parse("demo").unwrap(),
            timestamp: Utc.timestamp_opt(1_779_999_600 + offset_seconds, 0).unwrap(),
            kind,
            metadata,
            previous_checksum,
        })
        .unwrap()
    }

    fn prompt_metadata() -> EvidenceMetadata {
        EvidenceMetadata::from([
            ("prompt_bytes".to_owned(), "18".to_owned()),
            ("prompt_chars".to_owned(), "18".to_owned()),
            ("redacted".to_owned(), "true".to_owned()),
        ])
    }

    fn agent_request_metadata() -> EvidenceMetadata {
        EvidenceMetadata::from([("status".to_owned(), "requested".to_owned())])
    }

    fn agent_metadata() -> EvidenceMetadata {
        output_metadata(EvidenceMetadata::from([
            ("status".to_owned(), "succeeded".to_owned()),
            ("mode".to_owned(), "subprocess".to_owned()),
            ("model".to_owned(), "zai-coding-plan/glm-5".to_owned()),
            ("failure_category".to_owned(), "none".to_owned()),
            ("exit_code".to_owned(), "0".to_owned()),
            ("sanitized_message".to_owned(), "opencode completed".to_owned()),
        ]))
    }

    fn gate_started_metadata() -> EvidenceMetadata {
        EvidenceMetadata::from([
            ("gate".to_owned(), "fmt".to_owned()),
            ("moon_task".to_owned(), "oya:fmt".to_owned()),
            ("blocks_on_failure".to_owned(), "true".to_owned()),
        ])
    }

    fn gate_finished_metadata(status: &str) -> EvidenceMetadata {
        output_metadata(EvidenceMetadata::from([
            ("gate".to_owned(), "fmt".to_owned()),
            ("moon_task".to_owned(), "oya:fmt".to_owned()),
            ("status".to_owned(), status.to_owned()),
            ("exit_code".to_owned(), if status == "passed" { "0" } else { "1" }.to_owned()),
        ]))
    }

    fn finding_metadata(gate_finished: &EvidenceEnvelope) -> EvidenceMetadata {
        EvidenceMetadata::from([
            ("category".to_owned(), "format".to_owned()),
            ("gate".to_owned(), "fmt".to_owned()),
            ("gate_record_id".to_owned(), gate_finished.record_id.as_str().to_owned()),
            ("status".to_owned(), "open".to_owned()),
            ("next_action".to_owned(), "run moon run oya:fmt-fix then rerun the gate".to_owned()),
        ])
    }

    fn output_metadata(mut metadata: EvidenceMetadata) -> EvidenceMetadata {
        metadata.extend([
            ("stdout_preview".to_owned(), "".to_owned()),
            ("stdout_original_bytes".to_owned(), "0".to_owned()),
            ("stdout_stored_bytes".to_owned(), "0".to_owned()),
            ("stdout_truncated".to_owned(), "false".to_owned()),
            ("stdout_limit_bytes".to_owned(), "4096".to_owned()),
            (
                "stderr_preview".to_owned(),
                "ProviderModelNotFoundError: token=super-secret-token\nTraceback (most recent call last):"
                    .to_owned(),
            ),
            ("stderr_original_bytes".to_owned(), "18".to_owned()),
            ("stderr_stored_bytes".to_owned(), "10".to_owned()),
            ("stderr_truncated".to_owned(), "false".to_owned()),
            ("stderr_limit_bytes".to_owned(), "4096".to_owned()),
        ]);
        metadata
    }

    fn empty() -> EvidenceMetadata {
        EvidenceMetadata::new()
    }

    fn run_id() -> RunId {
        RunId::parse("run-demo").unwrap()
    }
}
