use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Canonical pipeline stages in execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stage {
    Implement,
    UnitTest,
    Coverage,
    Lint,
    Static,
    Integration,
    Security,
    Review,
    Accept,
}

impl Stage {
    const ORDERED: [Self; 9] = [
        Self::Implement,
        Self::UnitTest,
        Self::Coverage,
        Self::Lint,
        Self::Static,
        Self::Integration,
        Self::Security,
        Self::Review,
        Self::Accept,
    ];

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Implement => "implement",
            Self::UnitTest => "unit-test",
            Self::Coverage => "coverage",
            Self::Lint => "lint",
            Self::Static => "static",
            Self::Integration => "integration",
            Self::Security => "security",
            Self::Review => "review",
            Self::Accept => "accept",
        }
    }

    /// Parse a canonical stage label.
    ///
    /// # Errors
    /// Returns an error when the stage label is unknown.
    pub fn parse(label: impl AsRef<str>) -> Result<Self> {
        let trimmed = label.as_ref().trim();

        if trimmed.eq_ignore_ascii_case("implement") {
            Ok(Self::Implement)
        } else if trimmed.eq_ignore_ascii_case("unit-test")
            || trimmed.eq_ignore_ascii_case("unit_test")
            || trimmed.eq_ignore_ascii_case("unit test")
        {
            Ok(Self::UnitTest)
        } else if trimmed.eq_ignore_ascii_case("coverage") {
            Ok(Self::Coverage)
        } else if trimmed.eq_ignore_ascii_case("lint") {
            Ok(Self::Lint)
        } else if trimmed.eq_ignore_ascii_case("static") {
            Ok(Self::Static)
        } else if trimmed.eq_ignore_ascii_case("integration") {
            Ok(Self::Integration)
        } else if trimmed.eq_ignore_ascii_case("security") {
            Ok(Self::Security)
        } else if trimmed.eq_ignore_ascii_case("review") {
            Ok(Self::Review)
        } else if trimmed.eq_ignore_ascii_case("accept") {
            Ok(Self::Accept)
        } else {
            Err(Error::InvalidStage(format!("unknown stage '{trimmed}'")))
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &Self::ORDERED
    }

    #[must_use]
    pub const fn index(&self) -> usize {
        match self {
            Self::Implement => 0,
            Self::UnitTest => 1,
            Self::Coverage => 2,
            Self::Lint => 3,
            Self::Static => 4,
            Self::Integration => 5,
            Self::Security => 6,
            Self::Review => 7,
            Self::Accept => 8,
        }
    }

    #[must_use]
    pub const fn next(&self) -> Option<Self> {
        match self {
            Self::Implement => Some(Self::UnitTest),
            Self::UnitTest => Some(Self::Coverage),
            Self::Coverage => Some(Self::Lint),
            Self::Lint => Some(Self::Static),
            Self::Static => Some(Self::Integration),
            Self::Integration => Some(Self::Security),
            Self::Security => Some(Self::Review),
            Self::Review => Some(Self::Accept),
            Self::Accept => None,
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Validate that stages are strictly ordered and unique.
///
/// # Errors
/// Returns an error when stages are out of order or duplicated.
pub fn validate_stage_sequence(stages: &[Stage]) -> Result<()> {
    stages
        .iter()
        .map(Stage::index)
        .try_fold(None, |last_index: Option<usize>, current_index| {
            if last_index.is_some_and(|prev| current_index <= prev) {
                Err(Error::InvalidStageSequence(
                    "stages must be in strictly increasing order".to_string(),
                ))
            } else {
                Ok(Some(current_index))
            }
        })
        .map(|_| ())
}

/// Build linear pipeline stage dependencies for a DAG representation.
#[must_use]
pub fn pipeline_stage_edges() -> Vec<(Stage, Stage)> {
    Stage::all()
        .iter()
        .zip(Stage::all().iter().skip(1))
        .map(|(a, b)| (*a, *b))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parse_accepts_known_labels() {
        let result = Stage::parse("unit-test");
        assert_eq!(result.unwrap(), Stage::UnitTest);
    }

    #[test]
    fn parse_rejects_unknown_label() {
        let result = Stage::parse("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn validate_sequence_rejects_out_of_order() {
        let stages = [Stage::Lint, Stage::Implement];
        let result = validate_stage_sequence(&stages);
        assert!(result.is_err());
    }

    #[test]
    fn next_returns_following_stage() {
        let next = Stage::Implement.next();
        assert_eq!(next, Some(Stage::UnitTest));
    }

    #[test]
    fn pipeline_stage_edges_returns_linear_edges() {
        let edges = pipeline_stage_edges();
        let stages = Stage::all();

        assert_eq!(edges.len(), stages.len().saturating_sub(1));
        assert_eq!(edges.first(), Some(&(Stage::Implement, Stage::UnitTest)));
        assert_eq!(edges.last(), Some(&(Stage::Review, Stage::Accept)));
    }
}
