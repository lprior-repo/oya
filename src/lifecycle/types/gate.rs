#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

const GATE_FMT: &str = "fmt";
const GATE_LINT: &str = "lint";
const GATE_CLIPPY: &str = "clippy";
const GATE_CHECK: &str = "check";
const GATE_TEST: &str = "test";
const GATE_BUILD: &str = "build";
const GATE_AUDIT: &str = "audit";
const GATE_CI: &str = "ci";

const CATEGORY_FORMAT: &str = "format";
const CATEGORY_LINT: &str = "lint";
const CATEGORY_CHECK: &str = "check";
const CATEGORY_TEST: &str = "test";
const CATEGORY_BUILD: &str = "build";
const CATEGORY_AUDIT: &str = "audit";
const CATEGORY_CI: &str = "ci";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateId {
    Fmt,
    Lint,
    Clippy,
    Check,
    Test,
    Build,
    Audit,
    Ci,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateFailureCategory {
    Format,
    Lint,
    Check,
    Test,
    Build,
    Audit,
    Ci,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateModel {
    pub id: GateId,
    pub moon_task: &'static str,
    pub blocks_on_failure: bool,
    pub failure_category: GateFailureCategory,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GateIdError {
    #[error("gate id must not be empty")]
    Empty,
    #[error("unknown gate id: {0}")]
    Unknown(String),
}

impl GateId {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fmt => GATE_FMT,
            Self::Lint => GATE_LINT,
            Self::Clippy => GATE_CLIPPY,
            Self::Check => GATE_CHECK,
            Self::Test => GATE_TEST,
            Self::Build => GATE_BUILD,
            Self::Audit => GATE_AUDIT,
            Self::Ci => GATE_CI,
        }
    }

    #[must_use]
    pub fn moon_task(&self) -> &'static str {
        match self {
            Self::Fmt => "oya:fmt",
            Self::Lint | Self::Clippy => "oya:clippy",
            Self::Check => "oya:check",
            Self::Test => "oya:test",
            Self::Build => "oya:build-oya",
            Self::Audit => "oya:security",
            Self::Ci => "oya:root-ci",
        }
    }

    #[must_use]
    pub fn failure_category(&self) -> GateFailureCategory {
        match self {
            Self::Fmt => GateFailureCategory::Format,
            Self::Lint | Self::Clippy => GateFailureCategory::Lint,
            Self::Check => GateFailureCategory::Check,
            Self::Test => GateFailureCategory::Test,
            Self::Build => GateFailureCategory::Build,
            Self::Audit => GateFailureCategory::Audit,
            Self::Ci => GateFailureCategory::Ci,
        }
    }

    #[must_use]
    pub fn model(&self) -> GateModel {
        GateModel {
            id: *self,
            moon_task: self.moon_task(),
            blocks_on_failure: true,
            failure_category: self.failure_category(),
        }
    }

    /// Parses a known Moon verification gate id.
    ///
    /// # Errors
    /// Returns `GateIdError::Empty` for blank input and `GateIdError::Unknown`
    /// when the value is not a known verification gate.
    pub fn parse(input: &str) -> Result<Self, GateIdError> {
        match input.trim() {
            "" => Err(GateIdError::Empty),
            GATE_FMT => Ok(Self::Fmt),
            GATE_LINT => Ok(Self::Lint),
            GATE_CLIPPY => Ok(Self::Clippy),
            GATE_CHECK => Ok(Self::Check),
            GATE_TEST => Ok(Self::Test),
            GATE_BUILD => Ok(Self::Build),
            GATE_AUDIT => Ok(Self::Audit),
            GATE_CI => Ok(Self::Ci),
            value => Err(GateIdError::Unknown(value.to_owned())),
        }
    }
}

impl GateFailureCategory {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Format => CATEGORY_FORMAT,
            Self::Lint => CATEGORY_LINT,
            Self::Check => CATEGORY_CHECK,
            Self::Test => CATEGORY_TEST,
            Self::Build => CATEGORY_BUILD,
            Self::Audit => CATEGORY_AUDIT,
            Self::Ci => CATEGORY_CI,
        }
    }
}

impl GateModel {
    #[must_use]
    pub fn from_id(id: GateId) -> Self {
        id.model()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_id_parses_and_maps_to_canonical_moon_task_names() {
        assert_gate("fmt", GateId::Fmt, "oya:fmt");
        assert_gate("lint", GateId::Lint, "oya:clippy");
        assert_gate("clippy", GateId::Clippy, "oya:clippy");
        assert_gate("check", GateId::Check, "oya:check");
        assert_gate("test", GateId::Test, "oya:test");
        assert_gate("build", GateId::Build, "oya:build-oya");
        assert_gate("audit", GateId::Audit, "oya:security");
        assert_gate("ci", GateId::Ci, "oya:root-ci");
    }

    #[test]
    fn gate_ids_map_to_distinct_failure_categories() {
        assert_category(GateId::Fmt, GateFailureCategory::Format, "format");
        assert_category(GateId::Lint, GateFailureCategory::Lint, "lint");
        assert_category(GateId::Check, GateFailureCategory::Check, "check");
        assert_category(GateId::Test, GateFailureCategory::Test, "test");
        assert_category(GateId::Build, GateFailureCategory::Build, "build");
        assert_category(GateId::Audit, GateFailureCategory::Audit, "audit");
        assert_category(GateId::Ci, GateFailureCategory::Ci, "ci");
    }

    #[test]
    fn gate_model_marks_verification_gates_as_blocking() {
        let Ok(gate_id) = GateId::parse("test") else {
            assert!(false, "test gate should parse");
            return;
        };
        let model = GateModel::from_id(gate_id);

        assert_eq!(model.id, GateId::Test);
        assert_eq!(model.moon_task, "oya:test");
        assert_eq!(model.failure_category, GateFailureCategory::Test);
        assert!(model.blocks_on_failure);
    }

    #[test]
    fn gate_id_rejects_unknown_values() {
        assert_eq!(GateId::parse(""), Err(GateIdError::Empty));
        assert_eq!(GateId::parse("fmt-fix"), Err(GateIdError::Unknown("fmt-fix".to_owned())));
        assert_eq!(
            GateId::parse("bad/../gate"),
            Err(GateIdError::Unknown("bad/../gate".to_owned()))
        );
    }

    fn assert_gate(input: &str, expected_id: GateId, expected_task: &str) {
        let Ok(gate_id) = GateId::parse(input) else {
            assert!(false, "gate id should parse");
            return;
        };

        assert_eq!(gate_id, expected_id);
        assert_eq!(gate_id.as_str(), input);
        assert_eq!(gate_id.moon_task(), expected_task);
        assert_eq!(gate_id.model().moon_task, expected_task);
        assert!(gate_id.model().blocks_on_failure);
    }

    fn assert_category(
        gate_id: GateId,
        expected_category: GateFailureCategory,
        expected_name: &str,
    ) {
        assert_eq!(gate_id.failure_category(), expected_category);
        assert_eq!(gate_id.model().failure_category, expected_category);
        assert_eq!(expected_category.as_str(), expected_name);
    }
}
