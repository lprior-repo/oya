#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde::Serialize;
use std::path::PathBuf;

use super::args::ExplainArgs;
use crate::lifecycle::state::StateDb;
use crate::lifecycle::types::{BeadId, EvidenceEnvelope, EvidenceKind, EvidenceRecordId, RunId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FindingExplanation {
    finding_id: String,
    run_id: String,
    bead_id: String,
    category: String,
    command: String,
    gate: String,
    status: String,
    exit_code: String,
    stdout: ExplainOutput,
    stderr: ExplainOutput,
    next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ExplainOutput {
    preview: String,
    original_bytes: usize,
    stored_bytes: usize,
    truncated: bool,
    limit_bytes: usize,
}

pub fn explain_command(args: ExplainArgs) -> anyhow::Result<()> {
    let db = StateDb::open(data_dir())?;
    let finding = load_finding(&db, &args.finding_id)?;
    let explanation = FindingExplanation::from_finding(&finding)?;
    let json = serde_json::to_string_pretty(&explanation)?;
    println!("{json}");
    Ok(())
}

fn load_finding(db: &StateDb, input: &str) -> anyhow::Result<EvidenceEnvelope> {
    match EvidenceRecordId::parse(input) {
        Ok(record_id) => load_finding_by_record_id(db, &record_id),
        Err(_) => load_latest_finding_for_bead(db, input),
    }
}

fn load_finding_by_record_id(
    db: &StateDb,
    record_id: &EvidenceRecordId,
) -> anyhow::Result<EvidenceEnvelope> {
    let Some(record) = db.find_evidence_record(record_id)? else {
        return Err(anyhow::anyhow!("not_found: finding '{}' does not exist", record_id.as_str()));
    };
    ensure_finding(record)
}

fn load_latest_finding_for_bead(db: &StateDb, input: &str) -> anyhow::Result<EvidenceEnvelope> {
    let bead_id = BeadId::parse(input)?;
    let run_id = RunId::from_bead_id(&bead_id);
    let evidence = db.load_evidence(&run_id)?;
    evidence
        .into_iter()
        .rev()
        .find(|record| record.kind == EvidenceKind::Finding)
        .ok_or_else(|| anyhow::anyhow!("not_found: finding '{}' does not exist", input))
}

fn ensure_finding(record: EvidenceEnvelope) -> anyhow::Result<EvidenceEnvelope> {
    if record.kind == EvidenceKind::Finding {
        Ok(record)
    } else {
        Err(anyhow::anyhow!(
            "invalid_finding: evidence record '{}' is '{}', not finding",
            record.record_id.as_str(),
            evidence_kind_name(&record.kind)
        ))
    }
}

impl FindingExplanation {
    fn from_finding(finding: &EvidenceEnvelope) -> anyhow::Result<Self> {
        ensure_finding(finding.clone())?;
        let moon_task = metadata_value(finding, "moon_task")?;
        Ok(Self {
            finding_id: finding.record_id.as_str().to_owned(),
            run_id: finding.run_id.as_str().to_owned(),
            bead_id: finding.bead_id.as_str().to_owned(),
            category: metadata_value(finding, "category")?.to_owned(),
            command: format!("moon run {moon_task}"),
            gate: metadata_value(finding, "gate")?.to_owned(),
            status: metadata_value(finding, "status")?.to_owned(),
            exit_code: metadata_value(finding, "exit_code")?.to_owned(),
            stdout: ExplainOutput::from_metadata(finding, "stdout")?,
            stderr: ExplainOutput::from_metadata(finding, "stderr")?,
            next_action: metadata_value(finding, "next_action")?.to_owned(),
        })
    }
}

impl ExplainOutput {
    fn from_metadata(finding: &EvidenceEnvelope, prefix: &str) -> anyhow::Result<Self> {
        Ok(Self {
            preview: metadata_value(finding, &format!("{prefix}_preview"))?.to_owned(),
            original_bytes: metadata_usize(finding, &format!("{prefix}_original_bytes"))?,
            stored_bytes: metadata_usize(finding, &format!("{prefix}_stored_bytes"))?,
            truncated: metadata_bool(finding, &format!("{prefix}_truncated"))?,
            limit_bytes: metadata_usize(finding, &format!("{prefix}_limit_bytes"))?,
        })
    }
}

fn metadata_value<'a>(finding: &'a EvidenceEnvelope, key: &str) -> anyhow::Result<&'a str> {
    finding.metadata.get(key).map(String::as_str).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid_finding: missing metadata '{}' for '{}'",
            key,
            finding.record_id.as_str()
        )
    })
}

fn metadata_usize(finding: &EvidenceEnvelope, key: &str) -> anyhow::Result<usize> {
    metadata_value(finding, key)?.parse::<usize>().map_err(|error| {
        anyhow::anyhow!(
            "invalid_finding: metadata '{}' for '{}' is not a byte count: {error}",
            key,
            finding.record_id.as_str()
        )
    })
}

fn metadata_bool(finding: &EvidenceEnvelope, key: &str) -> anyhow::Result<bool> {
    metadata_value(finding, key)?.parse::<bool>().map_err(|error| {
        anyhow::anyhow!(
            "invalid_finding: metadata '{}' for '{}' is not a boolean: {error}",
            key,
            finding.record_id.as_str()
        )
    })
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
    use super::*;
    use crate::lifecycle::types::{EvidenceEnvelopeParts, EvidenceMetadata};
    use chrono::{TimeZone, Utc};

    #[test]
    fn finding_explanation_prints_category_command_bounded_output_and_next_action() {
        let finding = finding_envelope();

        let explanation = FindingExplanation::from_finding(&finding).unwrap();
        let json = serde_json::to_string(&explanation).unwrap();

        assert_eq!(explanation.finding_id, finding.record_id.as_str());
        assert_eq!(explanation.category, "format");
        assert_eq!(explanation.command, "moon run oya:fmt");
        assert_eq!(explanation.stdout.preview, "[redacted]");
        assert!(explanation.stdout.truncated);
        assert_eq!(explanation.next_action, "run moon run oya:fmt-fix then rerun the gate");
        assert!(!json.contains("super-secret-token"));
    }

    #[test]
    fn explain_loads_latest_finding_from_demo_alias() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(dir.path().join("db")).unwrap();
        let finding = finding_envelope();
        db.append_evidence(&finding).unwrap();
        db.flush().unwrap();

        let loaded = load_finding(&db, "demo").unwrap();

        assert_eq!(loaded.record_id, finding.record_id);
    }

    #[test]
    fn explain_rejects_non_finding_evidence_record() {
        let record = gate_finished_envelope();

        let explanation = FindingExplanation::from_finding(&record);

        assert!(explanation.is_err());
    }

    fn finding_envelope() -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse("ev-demo-g-fmt-fn-001").unwrap(),
            run_id: RunId::parse("run-demo").unwrap(),
            bead_id: BeadId::parse("demo").unwrap(),
            timestamp: Utc.timestamp_opt(1_779_999_600, 0).unwrap(),
            kind: EvidenceKind::Finding,
            metadata: finding_metadata(),
            previous_checksum: None,
        })
        .unwrap()
    }

    fn gate_finished_envelope() -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse("ev-demo-g-fmt-f-001").unwrap(),
            run_id: RunId::parse("run-demo").unwrap(),
            bead_id: BeadId::parse("demo").unwrap(),
            timestamp: Utc.timestamp_opt(1_779_999_600, 0).unwrap(),
            kind: EvidenceKind::GateRunFinished,
            metadata: EvidenceMetadata::new(),
            previous_checksum: None,
        })
        .unwrap()
    }

    fn finding_metadata() -> EvidenceMetadata {
        EvidenceMetadata::from([
            ("category".to_owned(), "format".to_owned()),
            ("exit_code".to_owned(), "1".to_owned()),
            ("gate".to_owned(), "fmt".to_owned()),
            ("moon_task".to_owned(), "oya:fmt".to_owned()),
            ("next_action".to_owned(), "run moon run oya:fmt-fix then rerun the gate".to_owned()),
            ("status".to_owned(), "open".to_owned()),
            ("stdout_preview".to_owned(), "[redacted]".to_owned()),
            ("stdout_original_bytes".to_owned(), "4103".to_owned()),
            ("stdout_stored_bytes".to_owned(), "4096".to_owned()),
            ("stdout_truncated".to_owned(), "true".to_owned()),
            ("stdout_limit_bytes".to_owned(), "4096".to_owned()),
            ("stderr_preview".to_owned(), "".to_owned()),
            ("stderr_original_bytes".to_owned(), "0".to_owned()),
            ("stderr_stored_bytes".to_owned(), "0".to_owned()),
            ("stderr_truncated".to_owned(), "false".to_owned()),
            ("stderr_limit_bytes".to_owned(), "4096".to_owned()),
        ])
    }
}
