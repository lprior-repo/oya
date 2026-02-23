//! Value-object IDs: AgentId, RunId, BeadId.
//! Domain newtypes: Tier, ModelId.

use crate::types::pipeline::ModelTier;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RunId(pub String);

impl RunId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BeadId(pub String);

impl BeadId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Tier(pub String);

#[derive(Debug, Error)]
pub enum TierError {
    #[error("Invalid tier: {0}")]
    Invalid(String),
}

impl Tier {
    pub fn new(tier: impl Into<String>) -> Result<Self, TierError> {
        let tier = tier.into();
        if tier.is_empty() {
            return Err(TierError::Invalid("tier cannot be empty".to_string()));
        }
        Ok(Self(tier))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_model_tier(&self) -> Result<ModelTier, TierError> {
        ModelTier::try_from(self.0.as_str()).map_err(TierError::Invalid)
    }
}

impl TryFrom<String> for Tier {
    type Error = TierError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModelId(pub String);

#[derive(Debug, Error)]
pub enum ModelIdError {
    #[error("Invalid model ID: {0}")]
    Invalid(String),
}

impl ModelId {
    pub fn new(model_id: impl Into<String>) -> Result<Self, ModelIdError> {
        let model_id = model_id.into();
        if model_id.is_empty() {
            return Err(ModelIdError::Invalid("model ID cannot be empty".to_string()));
        }
        Ok(Self(model_id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ModelId {
    type Error = ModelIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
