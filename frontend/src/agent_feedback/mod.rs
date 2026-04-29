use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
pub enum FailureCategory {
    #[serde(rename = "spec")]
    Spec,
    #[serde(rename = "validation")]
    Validation,
    #[serde(rename = "security")]
    Security,
    #[serde(rename = "integration")]
    Integration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRequest {
    pub failure_category: FailureCategory,
    pub spec_ref: String,
    pub iteration: u32,
    pub failure_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFeedback {
    pub message: String,
    pub category: String,
    pub priority: String,
    pub hints: Vec<String>,
    pub spec_reference: String,
}

pub struct FeedbackGenerator {
    templates: HashMap<String, FeedbackTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeedbackTemplate {
    pub title: String,
    pub description: String,
    pub hints: Vec<String>,
}

impl Default for FeedbackGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl FeedbackGenerator {
    #[must_use]
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        templates.insert(
            "spec-quality".to_string(),
            FeedbackTemplate {
                title: "Specification Quality Issue".to_string(),
                description: "The specification has quality issues that need to be addressed before proceeding with implementation.".to_string(),
                hints: vec![
                    "Review the spec linter output for specific failures".to_string(),
                    "Ensure all dependencies have error handling".to_string(),
                    "Add observable outcomes to all behaviors".to_string(),
                    "Specify exact error responses".to_string(),
                ],
            },
        );

        templates.insert(
            "validation-failure".to_string(),
            FeedbackTemplate {
                title: "Behavioral Validation Failed".to_string(),
                description:
                    "The implementation does not satisfy the behavioral requirements as specified."
                        .to_string(),
                hints: vec![
                    "Review the spec for the failed behavior".to_string(),
                    "Ensure all acceptance criteria are met".to_string(),
                    "Check edge cases for the behavior".to_string(),
                    "Test with realistic inputs, not edge cases".to_string(),
                ],
            },
        );

        templates.insert(
            "security-issue".to_string(),
            FeedbackTemplate {
                title: "Security Vulnerability Detected".to_string(),
                description: "A security invariant has been violated in the implementation."
                    .to_string(),
                hints: vec![
                    "Review the spec's security section".to_string(),
                    "Ensure input validation prevents injection attacks".to_string(),
                    "Check that enumeration prevention is implemented".to_string(),
                    "Verify authentication is properly enforced".to_string(),
                ],
            },
        );

        templates.insert(
            "integration-failure".to_string(),
            FeedbackTemplate {
                title: "Integration Issue".to_string(),
                description: "The implementation does not properly integrate with external services or twins.".to_string(),
                hints: vec![
                    "Check twin endpoint configurations".to_string(),
                    "Verify all required integrations are implemented".to_string(),
                    "Test with real twin instances if possible".to_string(),
                    "Review API contract compliance".to_string(),
                ],
            },
        );

        Self { templates }
    }

    #[must_use]
    pub fn generate(&self, request: &FeedbackRequest) -> AgentFeedback {
        let key = Self::category_to_key(request.failure_category);
        let template = self.templates.get(&key).map_or_else(
            || FeedbackTemplate {
                title: "Implementation Issue".to_string(),
                description: request.failure_context.clone(),
                hints: vec!["Review the spec for more details".to_string()],
            },
            |value| value.clone(),
        );

        let priority = Self::determine_priority(request.failure_category);

        AgentFeedback {
            message: format!("{}: {}", template.title, template.description),
            category: template.title,
            priority,
            hints: template.hints,
            spec_reference: request.spec_ref.clone(),
        }
    }

    fn category_to_key(category: FailureCategory) -> String {
        match category {
            FailureCategory::Spec => "spec-quality".to_string(),
            FailureCategory::Validation => "validation-failure".to_string(),
            FailureCategory::Security => "security-issue".to_string(),
            FailureCategory::Integration => "integration-failure".to_string(),
        }
    }

    fn determine_priority(category: FailureCategory) -> String {
        match category {
            FailureCategory::Security | FailureCategory::Validation => "high".to_string(),
            FailureCategory::Integration => "medium".to_string(),
            FailureCategory::Spec => "low".to_string(),
        }
    }

    #[must_use]
    pub fn generate_batch(&self, requests: &[FeedbackRequest]) -> Vec<AgentFeedback> {
        requests.iter().map(|r| self.generate(r)).collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn feedback_generator_generates_message_for_validation_failure() {
        let generator = FeedbackGenerator::new();

        let request = FeedbackRequest {
            failure_category: FailureCategory::Validation,
            spec_ref: "spec-001".to_string(),
            iteration: 1,
            failure_context: "Test context".to_string(),
        };

        let feedback = generator.generate(&request);

        assert!(!feedback.message.is_empty());
        assert!(!feedback.hints.is_empty());
    }

    #[test]
    fn feedback_generator_generates_message_for_spec_quality_issue() {
        let generator = FeedbackGenerator::new();

        let request = FeedbackRequest {
            failure_category: FailureCategory::Spec,
            spec_ref: "spec-002".to_string(),
            iteration: 1,
            failure_context: "Spec quality issue".to_string(),
        };

        let feedback = generator.generate(&request);

        assert!(feedback.message.contains("Specification Quality"));
        assert_eq!(feedback.priority, "low");
    }

    #[test]
    fn feedback_generator_generates_message_for_security_issue() {
        let generator = FeedbackGenerator::new();

        let request = FeedbackRequest {
            failure_category: FailureCategory::Security,
            spec_ref: "spec-003".to_string(),
            iteration: 1,
            failure_context: "Security issue".to_string(),
        };

        let feedback = generator.generate(&request);

        assert!(feedback.message.contains("Security"));
        assert_eq!(feedback.priority, "high");
    }

    #[test]
    fn feedback_generator_generates_message_for_integration_failure() {
        let generator = FeedbackGenerator::new();

        let request = FeedbackRequest {
            failure_category: FailureCategory::Integration,
            spec_ref: "spec-004".to_string(),
            iteration: 1,
            failure_context: "Integration issue".to_string(),
        };

        let feedback = generator.generate(&request);

        assert!(feedback.message.contains("Integration"));
        assert_eq!(feedback.priority, "medium");
    }

    #[test]
    fn feedback_generator_generates_batch_feedback_correctly() {
        let generator = FeedbackGenerator::new();

        let requests = vec![
            FeedbackRequest {
                failure_category: FailureCategory::Validation,
                spec_ref: "spec-001".to_string(),
                iteration: 1,
                failure_context: "Context 1".to_string(),
            },
            FeedbackRequest {
                failure_category: FailureCategory::Spec,
                spec_ref: "spec-002".to_string(),
                iteration: 1,
                failure_context: "Context 2".to_string(),
            },
        ];

        let feedbacks = generator.generate_batch(&requests);

        assert_eq!(feedbacks.len(), 2);
        assert!(!feedbacks[0].message.is_empty());
        assert!(!feedbacks[1].message.is_empty());
    }

    #[test]
    fn feedback_generator_handles_unknown_category_with_fallback() {
        // Test that an unknown category falls back to default message
        let generator = FeedbackGenerator::new();

        let request = FeedbackRequest {
            failure_category: FailureCategory::Validation,
            spec_ref: "spec-001".to_string(),
            iteration: 1,
            failure_context: "Test context".to_string(),
        };

        let feedback = generator.generate(&request);

        assert!(!feedback.spec_reference.is_empty());
        assert_eq!(feedback.spec_reference, "spec-001");
    }
}
