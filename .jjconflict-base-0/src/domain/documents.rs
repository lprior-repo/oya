use crate::domain::spec::{Constraint, Feature, Priority, Requirement, Spec};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum DocumentError {
    #[error("Document generation failed: {reason}")]
    GenerationFailed { reason: String },
    #[error("Invalid template: {details}")]
    InvalidTemplate { details: String },
    #[error("Spec error: {source}")]
    SpecError { source: crate::domain::spec::SpecError },
}

impl From<crate::domain::spec::SpecError> for DocumentError {
    fn from(source: crate::domain::spec::SpecError) -> Self {
        DocumentError::SpecError { source }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisionDocument {
    pub title: String,
    pub content: String,
    pub generated_at: String,
}

pub fn generate_vision_document(spec: &Spec) -> Result<VisionDocument, DocumentError> {
    spec.validate().map_err(DocumentError::from)?;

    let content = build_vision_markdown(spec)?;

    Ok(VisionDocument {
        title: format!("{} - Vision Document", spec.title),
        content,
        generated_at: chrono_lite_timestamp(),
    })
}

fn build_vision_markdown(spec: &Spec) -> Result<String, DocumentError> {
    let mut sections = Vec::new();

    sections.push(format!("# {}\n", spec.title));
    sections.push(format!("\n{}\n", spec.description));

    if !spec.features.is_empty() {
        sections.push("\n## Features\n".to_string());
        sections.push(build_features_section(&spec.features)?);
    }

    if !spec.requirements.is_empty() {
        sections.push("\n## Requirements\n".to_string());
        sections.push(build_requirements_section(&spec.requirements)?);
    }

    if !spec.constraints.is_empty() {
        sections.push("\n## Constraints\n".to_string());
        sections.push(build_constraints_section(&spec.constraints)?);
    }

    Ok(sections.join(""))
}

fn build_features_section(features: &[Feature]) -> Result<String, DocumentError> {
    let mut sections = Vec::new();

    for feature in features {
        sections.push(format!("\n### {}\n", feature.name));
        sections.push(format!("- **ID**: {}\n", feature.id));
        sections.push(format!("- **Priority**: {}\n", format_priority(&feature.priority)));
        sections.push(format!("\n{}\n", feature.description));
    }

    Ok(sections.join(""))
}

fn build_requirements_section(requirements: &[Requirement]) -> Result<String, DocumentError> {
    let mut sections = Vec::new();

    for requirement in requirements {
        sections.push(format!("\n### {}\n", requirement.id));
        sections.push(format!("- **Category**: {}\n", format_category(&requirement.category)));
        sections.push(format!("\n{}\n", requirement.description));
    }

    Ok(sections.join(""))
}

fn build_constraints_section(constraints: &[Constraint]) -> Result<String, DocumentError> {
    let mut sections = Vec::new();

    for constraint in constraints {
        sections.push(format!("\n### {}\n", constraint.id));
        sections
            .push(format!("- **Type**: {}\n", format_constraint_type(&constraint.constraint_type)));
        sections.push(format!("\n{}\n", constraint.description));
    }

    Ok(sections.join(""))
}

fn format_priority(priority: &Priority) -> String {
    match priority {
        Priority::Critical => "Critical",
        Priority::High => "High",
        Priority::Medium => "Medium",
        Priority::Low => "Low",
    }
    .to_string()
}

fn format_category(category: &crate::domain::spec::RequirementCategory) -> String {
    match category {
        crate::domain::spec::RequirementCategory::Functional => "Functional",
        crate::domain::spec::RequirementCategory::NonFunctional => "Non-Functional",
        crate::domain::spec::RequirementCategory::Technical => "Technical",
        crate::domain::spec::RequirementCategory::Business => "Business",
    }
    .to_string()
}

fn format_constraint_type(constraint_type: &crate::domain::spec::ConstraintType) -> String {
    match constraint_type {
        crate::domain::spec::ConstraintType::Technical => "Technical",
        crate::domain::spec::ConstraintType::Business => "Business",
        crate::domain::spec::ConstraintType::Regulatory => "Regulatory",
        crate::domain::spec::ConstraintType::Resource => "Resource",
    }
    .to_string()
}

fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            format!(
                "{}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
                1970 + secs / 31_536_000,
                (secs % 31_536_000) / 2_592_000 + 1,
                ((secs % 2_592_000) / 86_400) + 1,
                (secs % 86_400) / 3600,
                (secs % 3600) / 60,
                secs % 60
            )
        })
        .unwrap_or_else(|_| "Unknown timestamp".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::spec::{ConstraintType, Priority, RequirementCategory};

    fn create_test_spec() -> Spec {
        let mut spec = Spec::new(
            "spec-1".to_string(),
            "Test Project".to_string(),
            "A test specification".to_string(),
        );
        spec.add_feature(Feature {
            id: "feat-1".to_string(),
            name: "User Authentication".to_string(),
            description: "Secure login system".to_string(),
            priority: Priority::High,
        })
        .ok();
        spec.add_requirement(Requirement {
            id: "req-1".to_string(),
            description: "Users must authenticate with email".to_string(),
            category: RequirementCategory::Functional,
        })
        .ok();
        spec.add_constraint(Constraint {
            id: "const-1".to_string(),
            description: "Must comply with GDPR".to_string(),
            constraint_type: ConstraintType::Regulatory,
        })
        .ok();
        spec
    }

    #[test]
    fn test_generate_vision_document_succeeds_with_valid_spec() {
        let spec = create_test_spec();
        let result = generate_vision_document(&spec);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vision_document_contains_spec_title() {
        let spec = create_test_spec();
        let doc = generate_vision_document(&spec).unwrap();
        assert!(doc.content.contains("# Test Project"));
    }

    #[test]
    fn test_vision_document_contains_features() {
        let spec = create_test_spec();
        let doc = generate_vision_document(&spec).unwrap();
        assert!(doc.content.contains("## Features"));
        assert!(doc.content.contains("User Authentication"));
    }

    #[test]
    fn test_vision_document_contains_requirements() {
        let spec = create_test_spec();
        let doc = generate_vision_document(&spec).unwrap();
        assert!(doc.content.contains("## Requirements"));
        assert!(doc.content.contains("req-1"));
    }

    #[test]
    fn test_vision_document_contains_constraints() {
        let spec = create_test_spec();
        let doc = generate_vision_document(&spec).unwrap();
        assert!(doc.content.contains("## Constraints"));
        assert!(doc.content.contains("GDPR"));
    }

    #[test]
    fn test_generate_vision_document_fails_with_empty_spec_id() {
        let spec = Spec::new("".to_string(), "Title".to_string(), "Description".to_string());
        let result = generate_vision_document(&spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_vision_document_fails_with_empty_spec_title() {
        let spec = Spec::new("id".to_string(), "".to_string(), "Description".to_string());
        let result = generate_vision_document(&spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_vision_document_has_valid_gfm_format() {
        let spec = create_test_spec();
        let doc = generate_vision_document(&spec).unwrap();
        assert!(doc.content.starts_with("# "));
        assert!(doc.content.contains("## "));
    }

    #[test]
    fn test_priority_formatting() {
        assert_eq!(format_priority(&Priority::Critical), "Critical");
        assert_eq!(format_priority(&Priority::High), "High");
        assert_eq!(format_priority(&Priority::Medium), "Medium");
        assert_eq!(format_priority(&Priority::Low), "Low");
    }

    #[test]
    fn test_category_formatting() {
        assert_eq!(format_category(&RequirementCategory::Functional), "Functional");
        assert_eq!(format_category(&RequirementCategory::NonFunctional), "Non-Functional");
        assert_eq!(format_category(&RequirementCategory::Technical), "Technical");
        assert_eq!(format_category(&RequirementCategory::Business), "Business");
    }

    #[test]
    fn test_constraint_type_formatting() {
        assert_eq!(format_constraint_type(&ConstraintType::Technical), "Technical");
        assert_eq!(format_constraint_type(&ConstraintType::Business), "Business");
        assert_eq!(format_constraint_type(&ConstraintType::Regulatory), "Regulatory");
        assert_eq!(format_constraint_type(&ConstraintType::Resource), "Resource");
    }
}
