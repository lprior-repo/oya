use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spec {
    pub id: String,
    pub title: String,
    pub description: String,
    pub features: Vec<Feature>,
    pub requirements: Vec<Requirement>,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: Priority,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub description: String,
    pub category: RequirementCategory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub description: String,
    pub constraint_type: ConstraintType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequirementCategory {
    Functional,
    NonFunctional,
    Technical,
    Business,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    Technical,
    Business,
    Regulatory,
    Resource,
}

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpecError {
    #[error("Spec validation failed: {reason}")]
    ValidationFailed { reason: String },
    #[error("Feature not found: {id}")]
    FeatureNotFound { id: String },
    #[error("Requirement not found: {id}")]
    RequirementNotFound { id: String },
    #[error("Invalid spec structure: {details}")]
    InvalidStructure { details: String },
}

impl Spec {
    pub fn new(id: String, title: String, description: String) -> Self {
        Self {
            id,
            title,
            description,
            features: Vec::new(),
            requirements: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn add_feature(&mut self, feature: Feature) -> Result<(), SpecError> {
        if self.features.iter().any(|f| f.id == feature.id) {
            return Err(SpecError::InvalidStructure {
                details: format!("Duplicate feature ID: {}", feature.id),
            });
        }
        self.features.push(feature);
        Ok(())
    }

    pub fn add_requirement(&mut self, requirement: Requirement) -> Result<(), SpecError> {
        if self.requirements.iter().any(|r| r.id == requirement.id) {
            return Err(SpecError::InvalidStructure {
                details: format!("Duplicate requirement ID: {}", requirement.id),
            });
        }
        self.requirements.push(requirement);
        Ok(())
    }

    pub fn add_constraint(&mut self, constraint: Constraint) -> Result<(), SpecError> {
        if self.constraints.iter().any(|c| c.id == constraint.id) {
            return Err(SpecError::InvalidStructure {
                details: format!("Duplicate constraint ID: {}", constraint.id),
            });
        }
        self.constraints.push(constraint);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), SpecError> {
        if self.id.is_empty() {
            return Err(SpecError::ValidationFailed {
                reason: "Spec ID cannot be empty".to_string(),
            });
        }
        if self.title.is_empty() {
            return Err(SpecError::ValidationFailed {
                reason: "Spec title cannot be empty".to_string(),
            });
        }
        Ok(())
    }
}
