#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;

use super::args::{EvidenceArgs, EvidenceCommand};
use crate::lifecycle::state::StateDb;
use crate::lifecycle::types::{EvidenceEnvelope, RunId};

pub fn evidence_command(args: EvidenceArgs) -> anyhow::Result<()> {
    match args.command {
        EvidenceCommand::Check(args) => check_command(&args.run_id),
    }
}

fn check_command(input: &str) -> anyhow::Result<()> {
    let run_id = RunId::parse(input)?;
    let db = StateDb::open(data_dir())?;
    let evidence = db.load_evidence(&run_id)?;
    let report = evidence_check_report(&run_id, evidence.as_slice())?;
    let json = serde_json::to_string_pretty(&report)?;
    println!("{json}");
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct EvidenceCheckReport {
    run_id: String,
    status: String,
    evidence_records: usize,
    last_record_id: String,
    last_checksum: String,
}

#[cfg(test)]
impl EvidenceCheckReport {
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    pub(crate) fn evidence_records(&self) -> usize {
        self.evidence_records
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum EvidenceCheckError {
    #[error("not_found: run '{run_id}' has no evidence records")]
    EmptyRun { run_id: String },
    #[error("invalid_chain: first evidence record '{record_id}' has previous checksum")]
    FirstRecordHasPreviousChecksum { record_id: String },
    #[error(
        "invalid_chain: evidence record '{record_id}' expected previous checksum '{expected}' but found '{actual}'"
    )]
    BrokenPreviousChecksum { record_id: String, expected: String, actual: String },
    #[error(
        "invalid_chain: evidence record '{record_id}' belongs to run '{actual}', not '{expected}'"
    )]
    RunIdMismatch { record_id: String, expected: String, actual: String },
}

pub(crate) fn evidence_check_report(
    run_id: &RunId,
    evidence: &[EvidenceEnvelope],
) -> Result<EvidenceCheckReport, EvidenceCheckError> {
    let Some(last) = evidence.last() else {
        return Err(EvidenceCheckError::EmptyRun { run_id: run_id.as_str().to_owned() });
    };
    validate_first_record(evidence)?;
    validate_run_ids(run_id, evidence)?;
    validate_previous_checksums(evidence)?;
    Ok(EvidenceCheckReport {
        run_id: run_id.as_str().to_owned(),
        status: "valid".to_owned(),
        evidence_records: evidence.len(),
        last_record_id: last.record_id.as_str().to_owned(),
        last_checksum: last.checksum.as_str().to_owned(),
    })
}

fn validate_first_record(evidence: &[EvidenceEnvelope]) -> Result<(), EvidenceCheckError> {
    match evidence.first() {
        Some(first) if first.previous_checksum.is_some() => {
            Err(EvidenceCheckError::FirstRecordHasPreviousChecksum {
                record_id: first.record_id.as_str().to_owned(),
            })
        }
        Some(_) => Ok(()),
        None => Ok(()),
    }
}

fn validate_run_ids(
    run_id: &RunId,
    evidence: &[EvidenceEnvelope],
) -> Result<(), EvidenceCheckError> {
    evidence.iter().find(|record| &record.run_id != run_id).map_or(Ok(()), |record| {
        Err(EvidenceCheckError::RunIdMismatch {
            record_id: record.record_id.as_str().to_owned(),
            expected: run_id.as_str().to_owned(),
            actual: record.run_id.as_str().to_owned(),
        })
    })
}

fn validate_previous_checksums(evidence: &[EvidenceEnvelope]) -> Result<(), EvidenceCheckError> {
    evidence.windows(2).find_map(previous_checksum_error).map_or(Ok(()), Err)
}

fn previous_checksum_error(window: &[EvidenceEnvelope]) -> Option<EvidenceCheckError> {
    match window {
        [previous, current] if current.previous_checksum.as_ref() == Some(&previous.checksum) => {
            None
        }
        [previous, current] => Some(EvidenceCheckError::BrokenPreviousChecksum {
            record_id: current.record_id.as_str().to_owned(),
            expected: previous.checksum.as_str().to_owned(),
            actual: match &current.previous_checksum {
                Some(checksum) => checksum.as_str().to_owned(),
                None => "none".to_owned(),
            },
        }),
        _ => None,
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
    use crate::lifecycle::types::{
        BeadId, EvidenceEnvelopeParts, EvidenceKind, EvidenceMetadata, EvidenceRecordId,
    };
    use chrono::{TimeZone, Utc};

    #[test]
    fn evidence_check_report_accepts_checksum_linked_records() {
        let first = evidence_envelope("ev-demo-run-started-001", 0, EvidenceKind::RunStarted, None);
        let second = evidence_envelope(
            "ev-demo-prompt-record-002",
            1,
            EvidenceKind::PromptRecord,
            Some(first.checksum.clone()),
        );

        let report = evidence_check_report(&run_id(), &[first, second.clone()]).unwrap();

        assert_eq!(report.run_id, "run-demo");
        assert_eq!(report.status, "valid");
        assert_eq!(report.evidence_records, 2);
        assert_eq!(report.last_record_id, second.record_id.as_str());
        assert_eq!(report.last_checksum, second.checksum.as_str());
    }

    #[test]
    fn evidence_check_report_rejects_broken_previous_checksum() {
        let first = evidence_envelope("ev-demo-run-started-001", 0, EvidenceKind::RunStarted, None);
        let second =
            evidence_envelope("ev-demo-prompt-record-002", 1, EvidenceKind::PromptRecord, None);

        let report = evidence_check_report(&run_id(), &[first, second]);

        assert!(matches!(report, Err(EvidenceCheckError::BrokenPreviousChecksum { .. })));
    }

    fn evidence_envelope(
        record_id: &str,
        offset_seconds: i64,
        kind: EvidenceKind,
        previous_checksum: Option<crate::lifecycle::types::EvidenceChecksum>,
    ) -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse(record_id).unwrap(),
            run_id: run_id(),
            bead_id: BeadId::parse("demo").unwrap(),
            timestamp: Utc.timestamp_opt(1_779_999_600 + offset_seconds, 0).unwrap(),
            kind,
            metadata: EvidenceMetadata::new(),
            previous_checksum,
        })
        .unwrap()
    }

    fn run_id() -> RunId {
        RunId::parse("run-demo").unwrap()
    }
}
