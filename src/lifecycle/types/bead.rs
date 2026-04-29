#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use super::MAX_BEAD_ID_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadStatus {
    Planned,
    InProgress,
    Blocked,
    Completed,
}

impl BeadStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
        }
    }

    /// Parses a bead status from text.
    ///
    /// # Errors
    /// Returns `BeadStatusError::Invalid` when input is not one of
    /// "planned", "`in_progress`", "blocked", or "completed".
    pub fn parse(input: &str) -> Result<Self, BeadStatusError> {
        match input.trim().to_lowercase().as_str() {
            "planned" => Ok(Self::Planned),
            "in_progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "completed" => Ok(Self::Completed),
            _ => Err(BeadStatusError::Invalid(input.to_owned())),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BeadStatusError {
    #[error("invalid bead status: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BeadId(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BeadIdError {
    #[error("bead id must not be empty")]
    Empty,
    #[error("bead id exceeds max length: {len} > {max}")]
    TooLong { len: usize, max: usize },
    #[error("bead id contains invalid chars")]
    InvalidChars,
}

impl Serialize for BeadId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BeadId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

impl BeadId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses a bead id from text.
    ///
    /// # Errors
    /// Returns `BeadIdError::Empty` for blank input,
    /// `BeadIdError::TooLong` for IDs over 64 chars, and
    /// `BeadIdError::InvalidChars` when characters are outside `[a-z0-9-]`.
    pub fn parse(input: &str) -> Result<Self, BeadIdError> {
        let normalized = input.trim();
        if normalized.is_empty() {
            Err(BeadIdError::Empty)
        } else if normalized.len() > MAX_BEAD_ID_LEN {
            Err(BeadIdError::TooLong { len: normalized.len(), max: MAX_BEAD_ID_LEN })
        } else if has_only_bead_chars(normalized) {
            Ok(Self(normalized.to_owned()))
        } else {
            Err(BeadIdError::InvalidChars)
        }
    }
}

fn has_only_bead_chars(input: &str) -> bool {
    input.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::{BeadId, BeadIdError};

    #[test]
    fn bead_id_parse_accepts_lowercase_slug() {
        let Ok(bead_id) = BeadId::parse(" oya-8y3 ") else {
            assert!(false, "bead id should parse");
            return;
        };

        assert_eq!(bead_id.as_str(), "oya-8y3");
    }

    #[test]
    fn bead_id_parse_rejects_path_like_values() {
        assert_eq!(BeadId::parse("bad/../id"), Err(BeadIdError::InvalidChars));
    }

    #[test]
    fn bead_id_deserialize_rejects_malformed_values() {
        let result = serde_json::from_str::<BeadId>(r#""bad/../id""#);

        assert!(result.is_err());
    }

    #[test]
    fn bead_id_serde_round_trip_uses_string_value() {
        let Ok(bead_id) = BeadId::parse("oya-8y3") else {
            assert!(false, "bead id should parse");
            return;
        };
        let Ok(json) = serde_json::to_string(&bead_id) else {
            assert!(false, "bead id should serialize");
            return;
        };
        let Ok(decoded) = serde_json::from_str::<BeadId>(&json) else {
            assert!(false, "bead id should deserialize");
            return;
        };

        assert_eq!(json, r#""oya-8y3""#);
        assert_eq!(decoded, bead_id);
    }
}
