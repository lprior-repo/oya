#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use thiserror::Error;

use super::{BeadId, RunId};

const EVIDENCE_RECORD_ID_PREFIX: &str = "ev-";
const MAX_EVIDENCE_RECORD_ID_LEN: usize = 96;
const CHECKSUM_PREFIX: &str = "fnv1a64:";
const CHECKSUM_HEX_LEN: usize = 16;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvidenceRecordId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvidenceChecksum(String);

pub type EvidenceMetadata = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    RunStarted,
    PromptRecord,
    GateRunStarted,
    GateRunFinished,
    Finding,
    RepairRequest,
    RepairAttempt,
    RepairBlocked,
    AgentRequest,
    AgentRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceEnvelope {
    pub record_id: EvidenceRecordId,
    pub run_id: RunId,
    pub bead_id: BeadId,
    pub timestamp: DateTime<Utc>,
    pub kind: EvidenceKind,
    pub metadata: EvidenceMetadata,
    pub previous_checksum: Option<EvidenceChecksum>,
    pub checksum: EvidenceChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceEnvelopeParts {
    pub record_id: EvidenceRecordId,
    pub run_id: RunId,
    pub bead_id: BeadId,
    pub timestamp: DateTime<Utc>,
    pub kind: EvidenceKind,
    pub metadata: EvidenceMetadata,
    pub previous_checksum: Option<EvidenceChecksum>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EvidenceRecordIdError {
    #[error("evidence record id must not be empty")]
    Empty,
    #[error("evidence record id must start with ev-")]
    MissingPrefix,
    #[error("evidence record id suffix must not be empty")]
    MissingSuffix,
    #[error("evidence record id exceeds max length: {len} > {max}")]
    TooLong { len: usize, max: usize },
    #[error("evidence record id contains invalid chars")]
    InvalidChars,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EvidenceChecksumError {
    #[error("evidence checksum must not be empty")]
    Empty,
    #[error("evidence checksum must start with fnv1a64:")]
    MissingPrefix,
    #[error("evidence checksum must contain exactly 16 lowercase hex chars")]
    InvalidHex,
}

#[derive(Debug, Error)]
pub enum EvidenceEnvelopeError {
    #[error("canonical json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("evidence checksum mismatch")]
    ChecksumMismatch,
}

impl EvidenceRecordId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses an evidence record id.
    ///
    /// # Errors
    /// Returns `EvidenceRecordIdError` for blank, too long, unprefixed, empty
    /// suffix, or non `[a-z0-9-]` suffix values.
    pub fn parse(input: &str) -> Result<Self, EvidenceRecordIdError> {
        let normalized = input.trim();
        validate_prefixed_slug(normalized, EVIDENCE_RECORD_ID_PREFIX, MAX_EVIDENCE_RECORD_ID_LEN)
            .map_err(record_id_error)
            .map(|()| Self(normalized.to_owned()))
    }
}

impl EvidenceChecksum {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses an evidence checksum.
    ///
    /// # Errors
    /// Returns `EvidenceChecksumError` when the value is blank, has the wrong
    /// prefix, or is not a canonical lowercase 64-bit hex checksum.
    pub fn parse(input: &str) -> Result<Self, EvidenceChecksumError> {
        let normalized = input.trim();
        if normalized.is_empty() {
            return Err(EvidenceChecksumError::Empty);
        }
        let Some(hex) = normalized.strip_prefix(CHECKSUM_PREFIX) else {
            return Err(EvidenceChecksumError::MissingPrefix);
        };
        if is_lowercase_hex64(hex) {
            Ok(Self(normalized.to_owned()))
        } else {
            Err(EvidenceChecksumError::InvalidHex)
        }
    }
}

impl EvidenceEnvelope {
    /// Builds an envelope and computes its checksum from canonical JSON.
    ///
    /// # Errors
    /// Returns `EvidenceEnvelopeError` when canonical JSON serialization fails.
    pub fn new(parts: EvidenceEnvelopeParts) -> Result<Self, EvidenceEnvelopeError> {
        let checksum = checksum_for_parts(&parts)?;
        Ok(Self {
            record_id: parts.record_id,
            run_id: parts.run_id,
            bead_id: parts.bead_id,
            timestamp: parts.timestamp,
            kind: parts.kind,
            metadata: parts.metadata,
            previous_checksum: parts.previous_checksum,
            checksum,
        })
    }

    /// Serializes this envelope as canonical JSON.
    ///
    /// # Errors
    /// Returns `serde_json::Error` when serialization fails.
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserializes and verifies an envelope from canonical JSON.
    ///
    /// # Errors
    /// Returns `EvidenceEnvelopeError` when JSON is malformed or checksum does
    /// not match the canonical envelope payload.
    pub fn from_canonical_json(input: &str) -> Result<Self, EvidenceEnvelopeError> {
        let envelope = serde_json::from_str::<Self>(input)?;
        verify_envelope_checksum(envelope)
    }
}

impl Serialize for EvidenceRecordId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EvidenceRecordId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|value| Self::parse(&value).map_err(serde::de::Error::custom))
    }
}

impl Serialize for EvidenceChecksum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EvidenceChecksum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|value| Self::parse(&value).map_err(serde::de::Error::custom))
    }
}

#[derive(Serialize)]
struct EvidenceEnvelopeChecksumPayload<'a> {
    record_id: &'a EvidenceRecordId,
    run_id: &'a RunId,
    bead_id: &'a BeadId,
    timestamp: &'a DateTime<Utc>,
    kind: &'a EvidenceKind,
    metadata: &'a EvidenceMetadata,
    previous_checksum: Option<&'a EvidenceChecksum>,
}

fn checksum_for_parts(
    parts: &EvidenceEnvelopeParts,
) -> Result<EvidenceChecksum, serde_json::Error> {
    checksum_for_payload(&EvidenceEnvelopeChecksumPayload {
        record_id: &parts.record_id,
        run_id: &parts.run_id,
        bead_id: &parts.bead_id,
        timestamp: &parts.timestamp,
        kind: &parts.kind,
        metadata: &parts.metadata,
        previous_checksum: parts.previous_checksum.as_ref(),
    })
}

fn checksum_for_envelope(
    envelope: &EvidenceEnvelope,
) -> Result<EvidenceChecksum, serde_json::Error> {
    checksum_for_payload(&EvidenceEnvelopeChecksumPayload {
        record_id: &envelope.record_id,
        run_id: &envelope.run_id,
        bead_id: &envelope.bead_id,
        timestamp: &envelope.timestamp,
        kind: &envelope.kind,
        metadata: &envelope.metadata,
        previous_checksum: envelope.previous_checksum.as_ref(),
    })
}

fn checksum_for_payload(
    payload: &EvidenceEnvelopeChecksumPayload<'_>,
) -> Result<EvidenceChecksum, serde_json::Error> {
    serde_json::to_vec(payload).map(|bytes| EvidenceChecksum(format_checksum(fnv1a64(&bytes))))
}

fn verify_envelope_checksum(
    envelope: EvidenceEnvelope,
) -> Result<EvidenceEnvelope, EvidenceEnvelopeError> {
    if checksum_for_envelope(&envelope)? == envelope.checksum {
        Ok(envelope)
    } else {
        Err(EvidenceEnvelopeError::ChecksumMismatch)
    }
}

fn validate_prefixed_slug(input: &str, prefix: &str, max_len: usize) -> Result<(), SlugError> {
    if input.is_empty() {
        return Err(SlugError::Empty);
    }
    if input.len() > max_len {
        return Err(SlugError::TooLong { len: input.len(), max: max_len });
    }
    let suffix = input.strip_prefix(prefix).ok_or(SlugError::MissingPrefix)?;
    if suffix.is_empty() {
        return Err(SlugError::MissingSuffix);
    }
    if suffix.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
        Ok(())
    } else {
        Err(SlugError::InvalidChars)
    }
}

fn record_id_error(error: SlugError) -> EvidenceRecordIdError {
    match error {
        SlugError::Empty => EvidenceRecordIdError::Empty,
        SlugError::MissingPrefix => EvidenceRecordIdError::MissingPrefix,
        SlugError::MissingSuffix => EvidenceRecordIdError::MissingSuffix,
        SlugError::TooLong { len, max } => EvidenceRecordIdError::TooLong { len, max },
        SlugError::InvalidChars => EvidenceRecordIdError::InvalidChars,
    }
}

fn is_lowercase_hex64(input: &str) -> bool {
    input.len() == CHECKSUM_HEX_LEN
        && input.chars().all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
}

fn fnv1a64(input: &[u8]) -> u64 {
    input.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        let mixed = hash ^ u64::from(*byte);
        mixed.wrapping_mul(FNV_PRIME)
    })
}

fn format_checksum(value: u64) -> String {
    format!("{CHECKSUM_PREFIX}{value:016x}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlugError {
    Empty,
    MissingPrefix,
    MissingSuffix,
    TooLong { len: usize, max: usize },
    InvalidChars,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn evidence_envelope_round_trips_through_canonical_json() {
        let envelope = fixture_envelope(None);
        let Ok(json) = envelope.to_canonical_json() else {
            assert!(false, "evidence envelope should serialize");
            return;
        };
        let Ok(decoded) = EvidenceEnvelope::from_canonical_json(&json) else {
            assert!(false, "evidence envelope should deserialize and verify");
            return;
        };

        let Ok(decoded_json) = decoded.to_canonical_json() else {
            assert!(false, "decoded evidence envelope should serialize");
            return;
        };

        assert_eq!(decoded, envelope);
        assert_eq!(decoded_json, json);
    }

    #[test]
    fn evidence_envelope_checksum_covers_previous_checksum_chain() {
        let previous = parse_checksum("fnv1a64:0123456789abcdef");
        let envelope = fixture_envelope(Some(previous.clone()));

        assert_eq!(envelope.previous_checksum, Some(previous));
        assert_ne!(fixture_envelope(None).checksum, envelope.checksum);
    }

    #[test]
    fn evidence_envelope_rejects_tampered_canonical_json() {
        let envelope = fixture_envelope(None);
        let Ok(json) = envelope.to_canonical_json() else {
            assert!(false, "evidence envelope should serialize");
            return;
        };
        let tampered = json.replace("run_started", "prompt_record");

        assert!(matches!(
            EvidenceEnvelope::from_canonical_json(&tampered),
            Err(EvidenceEnvelopeError::ChecksumMismatch)
        ));
    }

    #[test]
    fn evidence_record_id_rejects_malformed_values() {
        assert!(matches!(EvidenceRecordId::parse(""), Err(EvidenceRecordIdError::Empty)));
        assert!(matches!(
            EvidenceRecordId::parse("record-1"),
            Err(EvidenceRecordIdError::MissingPrefix)
        ));
        assert!(matches!(
            EvidenceRecordId::parse("ev-"),
            Err(EvidenceRecordIdError::MissingSuffix)
        ));
        assert!(matches!(
            EvidenceRecordId::parse("ev-bad/../id"),
            Err(EvidenceRecordIdError::InvalidChars)
        ));
    }

    #[test]
    fn evidence_checksum_rejects_noncanonical_values() {
        assert!(matches!(EvidenceChecksum::parse(""), Err(EvidenceChecksumError::Empty)));
        assert!(matches!(
            EvidenceChecksum::parse("sha256:abcd"),
            Err(EvidenceChecksumError::MissingPrefix)
        ));
        assert!(matches!(
            EvidenceChecksum::parse("fnv1a64:ABCDEF0123456789"),
            Err(EvidenceChecksumError::InvalidHex)
        ));
    }

    fn fixture_envelope(previous_checksum: Option<EvidenceChecksum>) -> EvidenceEnvelope {
        let parts = EvidenceEnvelopeParts {
            record_id: parse_record_id("ev-oya-a82-001"),
            run_id: parse_run_id("run-oya-a82"),
            bead_id: parse_bead_id("oya-a82"),
            timestamp: fixture_timestamp(),
            kind: EvidenceKind::RunStarted,
            metadata: EvidenceMetadata::new(),
            previous_checksum,
        };
        match EvidenceEnvelope::new(parts) {
            Ok(envelope) => envelope,
            Err(error) => {
                assert!(false, "evidence envelope should build: {error}");
                fallback_envelope()
            }
        }
    }

    fn fallback_envelope() -> EvidenceEnvelope {
        EvidenceEnvelope {
            record_id: EvidenceRecordId("ev-fallback".to_owned()),
            run_id: RunId::from_bead_id(&parse_bead_id("fallback")),
            bead_id: parse_bead_id("fallback"),
            timestamp: fixture_timestamp(),
            kind: EvidenceKind::RunStarted,
            metadata: EvidenceMetadata::new(),
            previous_checksum: None,
            checksum: parse_checksum("fnv1a64:0000000000000000"),
        }
    }

    fn parse_record_id(input: &str) -> EvidenceRecordId {
        match EvidenceRecordId::parse(input) {
            Ok(id) => id,
            Err(error) => {
                assert!(false, "record id fixture should parse: {error}");
                EvidenceRecordId("ev-fallback".to_owned())
            }
        }
    }

    fn parse_run_id(input: &str) -> RunId {
        match RunId::parse(input) {
            Ok(id) => id,
            Err(error) => {
                assert!(false, "run id fixture should parse: {error}");
                RunId::from_bead_id(&parse_bead_id("fallback"))
            }
        }
    }

    fn parse_bead_id(input: &str) -> BeadId {
        match BeadId::parse(input) {
            Ok(id) => id,
            Err(error) => {
                assert!(false, "bead id fixture should parse: {error}");
                let Ok(fallback) = BeadId::parse("fallback") else {
                    assert!(false, "fallback bead id should parse");
                    std::process::abort();
                };
                fallback
            }
        }
    }

    fn parse_checksum(input: &str) -> EvidenceChecksum {
        match EvidenceChecksum::parse(input) {
            Ok(checksum) => checksum,
            Err(error) => {
                assert!(false, "checksum fixture should parse: {error}");
                EvidenceChecksum("fnv1a64:0000000000000000".to_owned())
            }
        }
    }

    fn fixture_timestamp() -> DateTime<Utc> {
        match Utc.with_ymd_and_hms(2026, 4, 29, 14, 30, 0).single() {
            Some(timestamp) => timestamp,
            None => {
                assert!(false, "timestamp fixture should be valid");
                Utc::now()
            }
        }
    }
}
