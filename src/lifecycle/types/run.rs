#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use super::{BeadId, MAX_RUN_ID_LEN};

const RUN_ID_PREFIX: &str = "run-";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunId(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RunIdError {
    #[error("run id must not be empty")]
    Empty,
    #[error("run id must start with run-")]
    MissingPrefix,
    #[error("run id suffix must not be empty")]
    MissingSuffix,
    #[error("run id exceeds max length: {len} > {max}")]
    TooLong { len: usize, max: usize },
    #[error("run id contains invalid chars")]
    InvalidChars,
}

impl RunId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn from_bead_id(bead_id: &BeadId) -> Self {
        Self(format!("{RUN_ID_PREFIX}{}", bead_id.as_str()))
    }

    /// Parses a run id from text.
    ///
    /// # Errors
    /// Returns `RunIdError` when input is empty, too long, missing the `run-`
    /// prefix, has no suffix, or contains chars outside `[a-z0-9-]`.
    pub fn parse(input: &str) -> Result<Self, RunIdError> {
        let normalized = input.trim();
        if normalized.is_empty() {
            return Err(RunIdError::Empty);
        }
        if normalized.len() > MAX_RUN_ID_LEN {
            return Err(RunIdError::TooLong { len: normalized.len(), max: MAX_RUN_ID_LEN });
        }
        let suffix = normalized.strip_prefix(RUN_ID_PREFIX).ok_or(RunIdError::MissingPrefix)?;
        if suffix.is_empty() {
            return Err(RunIdError::MissingSuffix);
        }
        if suffix_has_only_run_id_chars(suffix) {
            Ok(Self(normalized.to_owned()))
        } else {
            Err(RunIdError::InvalidChars)
        }
    }
}

impl Display for RunId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RunId {
    type Err = RunIdError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

impl Serialize for RunId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RunId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|value| Self::parse(&value).map_err(serde::de::Error::custom))
    }
}

fn suffix_has_only_run_id_chars(input: &str) -> bool {
    input.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_parse_accepts_prefixed_lowercase_slug() {
        let id = parse_valid("run-oya-ayt-001");

        assert_eq!(id.as_str(), "run-oya-ayt-001");
    }

    #[test]
    fn run_id_display_returns_canonical_value() {
        let id = parse_valid("run-display-proof");

        assert_eq!(id.to_string(), "run-display-proof");
    }

    #[test]
    fn run_id_from_bead_id_uses_run_prefix() {
        let Ok(bead_id) = BeadId::parse("oya-ayt") else {
            assert!(false, "bead id fixture should parse");
            return;
        };
        let run_id = RunId::from_bead_id(&bead_id);

        assert_eq!(run_id.as_str(), "run-oya-ayt");
    }

    #[test]
    fn run_id_serde_round_trip_uses_string_value() {
        let id = parse_valid("run-serde-proof");
        let Ok(json) = serde_json::to_string(&id) else {
            assert!(false, "run id serialization should succeed");
            return;
        };
        let Ok(decoded) = serde_json::from_str::<RunId>(&json) else {
            assert!(false, "run id deserialization should succeed");
            return;
        };

        assert_eq!(json, "\"run-serde-proof\"");
        assert_eq!(decoded, id);
    }

    #[test]
    fn run_id_deserialize_rejects_malformed_values() {
        let decoded = serde_json::from_str::<RunId>("\"RUN-invalid\"");

        assert!(decoded.is_err());
    }

    #[test]
    fn run_id_parse_rejects_empty() {
        assert!(matches!(RunId::parse(""), Err(RunIdError::Empty)));
        assert!(matches!(RunId::parse("   "), Err(RunIdError::Empty)));
    }

    #[test]
    fn run_id_parse_rejects_missing_prefix() {
        assert!(matches!(RunId::parse("oya-ayt"), Err(RunIdError::MissingPrefix)));
    }

    #[test]
    fn run_id_parse_rejects_missing_suffix() {
        assert!(matches!(RunId::parse("run-"), Err(RunIdError::MissingSuffix)));
    }

    #[test]
    fn run_id_parse_rejects_invalid_chars() {
        assert!(matches!(RunId::parse("run-abc_def"), Err(RunIdError::InvalidChars)));
        assert!(matches!(RunId::parse("run-abc/def"), Err(RunIdError::InvalidChars)));
    }

    #[test]
    fn run_id_parse_rejects_too_long() {
        let input = format!("run-{}", "a".repeat(MAX_RUN_ID_LEN));

        assert!(matches!(RunId::parse(&input), Err(RunIdError::TooLong { .. })));
    }

    fn parse_valid(input: &str) -> RunId {
        match RunId::parse(input) {
            Ok(id) => id,
            Err(error) => {
                assert!(false, "expected valid run id, got {error}");
                RunId("run-invalid-test-fallback".to_owned())
            }
        }
    }
}
