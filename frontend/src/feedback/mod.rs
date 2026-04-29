use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeedbackLevel(u8);

impl FeedbackLevel {
    pub const MINIMAL: Self = Self(1);
    pub const CATEGORICAL: Self = Self(2);
    pub const GUIDED: Self = Self(3);
    pub const DIAGNOSTIC: Self = Self(4);
    pub const TRANSPARENT: Self = Self(5);

    /// Creates a new feedback configuration.
    ///
    /// # Errors
    ///
    /// Returns `FeedbackConfigError` if the level is invalid.
    pub const fn new(level: u8) -> Result<Self, FeedbackConfigError> {
        match level {
            1 => Ok(Self::MINIMAL),
            2 => Ok(Self::CATEGORICAL),
            3 => Ok(Self::GUIDED),
            4 => Ok(Self::DIAGNOSTIC),
            5 => Ok(Self::TRANSPARENT),
            _ => Err(FeedbackConfigError::InvalidLevel(level)),
        }
    }

    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn is_minimal(&self) -> bool {
        self.0 == Self::MINIMAL.0
    }

    #[must_use]
    pub const fn is_transparent(&self) -> bool {
        self.0 == Self::TRANSPARENT.0
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self.0 {
            1 => "minimal",
            2 => "categorical",
            3 => "guided",
            4 => "diagnostic",
            5 => "transparent",
            _ => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FailureCategoryName(String);

impl FailureCategoryName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn categorize(error_message: &str) -> Self {
        if error_message.contains("404") {
            Self("Resource Not Found".into())
        } else if error_message.contains("500") {
            Self("Server Error".into())
        } else if error_message.contains("timeout") {
            Self("Timeout".into())
        } else {
            Self("Unknown".into())
        }
    }
}

impl std::fmt::Display for FailureCategoryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpecRef(String);

impl SpecRef {
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SpecRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum FeedbackConfigError {
    #[error("Invalid feedback level: {0}. Must be 1-5")]
    InvalidLevel(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct FeedbackConfig {
    pub level: FeedbackLevel,
    pub name: String,
    pub includes_status_codes: bool,
    pub includes_bodies: bool,
    pub includes_timing: bool,
    pub includes_scenario_ids: bool,
    pub includes_step_sequences: bool,
    pub includes_exact_assertions: bool,
}

impl FeedbackConfig {
    #[must_use]
    pub fn from_level(level: u8) -> Self {
        match level {
            1 => Self {
                level: FeedbackLevel::MINIMAL,
                name: "minimal".to_string(),
                includes_status_codes: false,
                includes_bodies: false,
                includes_timing: false,
                includes_scenario_ids: false,
                includes_step_sequences: false,
                includes_exact_assertions: false,
            },
            2 => Self {
                level: FeedbackLevel::CATEGORICAL,
                name: "categorical".to_string(),
                includes_status_codes: false,
                includes_bodies: false,
                includes_timing: false,
                includes_scenario_ids: false,
                includes_step_sequences: false,
                includes_exact_assertions: false,
            },
            3 => Self {
                level: FeedbackLevel::GUIDED,
                name: "guided".to_string(),
                includes_status_codes: false,
                includes_bodies: false,
                includes_timing: false,
                includes_scenario_ids: false,
                includes_step_sequences: false,
                includes_exact_assertions: false,
            },
            4 => Self {
                level: FeedbackLevel::DIAGNOSTIC,
                name: "diagnostic".to_string(),
                includes_status_codes: true,
                includes_bodies: false,
                includes_timing: false,
                includes_scenario_ids: false,
                includes_step_sequences: false,
                includes_exact_assertions: false,
            },
            5 => Self {
                level: FeedbackLevel::TRANSPARENT,
                name: "transparent".to_string(),
                includes_status_codes: true,
                includes_bodies: true,
                includes_timing: true,
                includes_scenario_ids: true,
                includes_step_sequences: true,
                includes_exact_assertions: true,
            },
            _ => Self::from_level(3),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedFailure {
    pub category: FailureCategoryName,
    pub spec_ref: SpecRef,
    pub description: String,
    pub hint: String,
    pub spec_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedFeedback {
    pub iteration: u32,
    pub passed_count: usize,
    pub failed_count: usize,
    pub total_count: usize,
    pub failures: Vec<SanitizedFailure>,
    pub summary: String,
}

pub struct FeedbackSanitizer {
    #[allow(dead_code)]
    config: FeedbackConfig,
}

impl FeedbackSanitizer {
    #[must_use]
    pub fn new(level: u8) -> Self {
        Self { config: FeedbackConfig::from_level(level) }
    }

    #[must_use]
    pub fn sanitize(
        &self,
        raw_results: &[super::scenario_runner::ScenarioResult],
        iteration: u32,
    ) -> SanitizedFeedback {
        let total_count = raw_results.len();
        let passed_count = raw_results.iter().filter(|r| r.passed).count();
        let failed_count = total_count - passed_count;

        let failures: Vec<SanitizedFailure> =
            raw_results.iter().filter(|r| !r.passed).map(Self::sanitize_failure).collect();

        let summary = format!("{failed_count} of {total_count} behavioral tests failed");

        SanitizedFeedback { iteration, passed_count, failed_count, total_count, failures, summary }
    }

    fn sanitize_failure(result: &super::scenario_runner::ScenarioResult) -> SanitizedFailure {
        let category = Self::categorize_failure(result);
        let description = Self::sanitize_description(&category);
        let hint = Self::generate_hint(&category);
        let spec_text = Self::get_spec_text_reference(&category);

        SanitizedFailure {
            category,
            spec_ref: SpecRef::new(result.spec_ref.clone()),
            description,
            hint,
            spec_text,
        }
    }

    fn categorize_failure(result: &super::scenario_runner::ScenarioResult) -> FailureCategoryName {
        let failed_step = result.steps.iter().find(|s| !s.passed);

        if let Some(step) = failed_step {
            if step.error.as_ref().is_some_and(|e| e.contains("404")) {
                return FailureCategoryName::new("Resource Not Found");
            }
            if step.error.as_ref().is_some_and(|e| e.contains("500")) {
                return FailureCategoryName::new("Server Error");
            }
            if step.error.as_ref().is_some_and(|e| e.contains("timeout")) {
                return FailureCategoryName::new("Timeout");
            }
        }

        match result.category.as_str() {
            "security" => FailureCategoryName::new("Security Violation"),
            "error-handling" => FailureCategoryName::new("Error Handling"),
            "happy-path" => FailureCategoryName::new("Happy Path"),
            _ => FailureCategoryName::new("Unknown"),
        }
    }

    fn sanitize_description(category: &FailureCategoryName) -> String {
        match category.as_str() {
            "Security Violation" => "The system does not properly enforce security constraints.".to_string(),
            "Error Handling" => "The system does not gracefully handle error conditions.".to_string(),
            "Happy Path" => "The primary workflow does not produce expected results.".to_string(),
            "Resource Not Found" => "The system returns incorrect HTTP status codes for missing resources.".to_string(),
            "Server Error" => "The system returns internal server errors instead of handling the request properly.".to_string(),
            "Timeout" => "The system does not complete operations within expected time limits.".to_string(),
            _ => "A behavioral requirement is not satisfied.".to_string(),
        }
    }

    fn generate_hint(category: &FailureCategoryName) -> String {
        match category.as_str() {
            "Security Violation" => {
                "Review the spec's security requirements and ensure all invariants are enforced."
                    .to_string()
            }
            "Error Handling" => {
                "Review edge cases in the spec and ensure proper error responses are returned."
                    .to_string()
            }
            "Happy Path" => "Review the spec's acceptance criteria for this behavior.".to_string(),
            "Resource Not Found" => {
                "Ensure API endpoints return correct HTTP status codes per the spec.".to_string()
            }
            "Server Error" => {
                "Check that all error conditions are handled before reaching internal logic."
                    .to_string()
            }
            "Timeout" => {
                "Review performance requirements in the spec's constraints section.".to_string()
            }
            _ => "Review the spec for the relevant behavior.".to_string(),
        }
    }

    fn get_spec_text_reference(category: &FailureCategoryName) -> String {
        match category.as_str() {
            "Security Violation" => {
                "Review context.invariants for security constraints.".to_string()
            }
            "Error Handling" => {
                "Review behaviors[].edge_cases for required error handling.".to_string()
            }
            "Happy Path" => "Review acceptance_criteria for the expected behavior.".to_string(),
            _ => "Review the relevant behavior in the spec.".to_string(),
        }
    }
}

#[must_use]
pub fn sanitize_results(
    raw_results: &[super::scenario_runner::ScenarioResult],
    iteration: u32,
    level: u8,
) -> SanitizedFeedback {
    let sanitizer = FeedbackSanitizer::new(level);
    sanitizer.sanitize(raw_results, iteration)
}
