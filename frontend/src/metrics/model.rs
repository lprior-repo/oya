use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::errors::MetricsError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecId(pub String);

impl SpecId {
    pub fn new(id: impl Into<String>) -> Result<Self, MetricsError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(MetricsError::InvalidSessionId("SpecId cannot be empty".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(s: String) -> Result<Self, MetricsError> {
        if s.trim().is_empty() {
            return Err(MetricsError::InvalidSessionId("SpecId cannot be empty".into()));
        }
        Ok(Self(s))
    }
}

impl std::fmt::Display for SpecId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SpecId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SpecId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecVersion(pub String);

impl SpecVersion {
    pub fn new(version: impl Into<String>) -> Result<Self, MetricsError> {
        let version = version.into();
        if version.trim().is_empty() {
            return Err(MetricsError::InvalidSessionId("SpecVersion cannot be empty".into()));
        }
        Ok(Self(version))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SpecVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SpecVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SpecVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CategoryName(pub String);

impl CategoryName {
    pub const COMPLETENESS: Self = Self(String::new());
    pub const CLARITY: Self = Self(String::new());
    pub const SECURITY: Self = Self(String::new());
    pub const TESTABILITY: Self = Self(String::new());
    pub const DATA_MODEL: Self = Self(String::new());

    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn completeness() -> Self {
        Self("Completeness".to_string())
    }

    pub fn clarity() -> Self {
        Self("Clarity".to_string())
    }

    pub fn security() -> Self {
        Self("Security".to_string())
    }

    pub fn testability() -> Self {
        Self("Testability".to_string())
    }

    pub fn data_model() -> Self {
        Self("Data Model".to_string())
    }
}

impl std::fmt::Display for CategoryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for CategoryName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CategoryName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Result<Self, MetricsError> {
        if id.trim().is_empty() {
            return Err(MetricsError::InvalidSessionId(id));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SessionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_string(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SuggestionKey(pub String);

impl SuggestionKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SuggestionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SuggestionKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SuggestionKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct FeedbackLevel(u8);

impl FeedbackLevel {
    const MIN: u8 = 1;
    const MAX: u8 = 5;

    pub fn new(level: u8) -> Result<Self, MetricsError> {
        if (Self::MIN..=Self::MAX).contains(&level) {
            Ok(Self(level))
        } else {
            Err(MetricsError::InvalidFeedbackLevel(level))
        }
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn is_minimal(self) -> bool {
        self.0 == Self::MIN
    }

    #[must_use]
    pub const fn is_transparent(self) -> bool {
        self.0 == Self::MAX
    }

    pub const fn minimal() -> Self {
        Self(1)
    }

    pub const fn categorical() -> Self {
        Self(2)
    }

    pub const fn guided() -> Self {
        Self(3)
    }

    pub const fn diagnostic() -> Self {
        Self(4)
    }

    pub const fn transparent() -> Self {
        Self(5)
    }
}

impl Default for FeedbackLevel {
    fn default() -> Self {
        Self(3)
    }
}

impl Serialize for FeedbackLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for FeedbackLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let level = u8::deserialize(deserializer)?;
        Self::new(level).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[allow(dead_code)]
pub struct Priority(u8);

#[allow(dead_code)]
impl Priority {
    pub const CRITICAL: Self = Self(0);
    pub const HIGH: Self = Self(1);
    pub const MEDIUM: Self = Self(2);
    pub const LOW: Self = Self(3);
    pub const BACKLOG: Self = Self(4);

    pub const fn new(level: u8) -> Result<Self, MetricsError> {
        if level <= 4 {
            Ok(Self(level))
        } else {
            Err(MetricsError::InvalidFeedbackLevel(level))
        }
    }

    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self(2)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IterationNumber(u32);

impl IterationNumber {
    pub const fn new(n: u32) -> Self {
        Self(n)
    }

    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn increment(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FailureCategoryName(pub String);

impl FailureCategoryName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FailureCategoryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SpecRef(pub String);

#[allow(dead_code)]
impl SpecRef {
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SpecRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecValidationMetrics {
    pub timestamp: DateTime<Utc>,
    pub spec_id: SpecId,
    pub spec_version: SpecVersion,
    pub overall_score: u32,
    pub passed: bool,
    pub category_scores: HashMap<CategoryName, u32>,
    pub errors_count: usize,
    pub warnings_count: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioValidationMetrics {
    pub timestamp: DateTime<Utc>,
    pub spec_id: SpecId,
    pub total_scenarios: usize,
    pub passed_scenarios: usize,
    pub failed_scenarios: usize,
    pub category_breakdown: HashMap<CategoryName, CategoryStats>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionDecision {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuggestionDecisionMetrics {
    pub timestamp: DateTime<Utc>,
    pub suggestion_key: SuggestionKey,
    pub decision: SuggestionDecision,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryStats {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QualityGateIteration {
    pub iteration: IterationNumber,
    pub timestamp: DateTime<Utc>,
    pub spec_passed: bool,
    pub spec_score: u32,
    pub scenarios_passed: bool,
    pub scenarios_total: usize,
    pub scenarios_passed_count: usize,
    pub overall_passed: bool,
    pub failure_category: Option<FailureCategoryName>,
    pub feedback_level: FeedbackLevel,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QualityGateSession {
    pub session_id: SessionId,
    pub spec_id: SpecId,
    pub spec_version: SpecVersion,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub iterations: Vec<QualityGateIteration>,
    pub total_duration_ms: u64,
    pub status: SessionStatus,
    pub escalated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "passed")]
    Passed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "escalated")]
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsSummary {
    pub total_sessions: usize,
    pub passed_sessions: usize,
    pub failed_sessions: usize,
    pub escalated_sessions: usize,
    pub avg_iterations_to_pass: f64,
    pub avg_duration_minutes: f64,
    pub most_common_failure_categories: Vec<(String, usize)>,
    pub avg_spec_score: f64,
}

pub struct MetricsStore {
    pub(crate) base_path: PathBuf,
    pub(crate) data: Arc<RwLock<MetricsData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MetricsData {
    pub(crate) spec_validations: Vec<SpecValidationMetrics>,
    pub(crate) scenario_validations: Vec<ScenarioValidationMetrics>,
    pub(crate) suggestion_decisions: Vec<SuggestionDecisionMetrics>,
    pub(crate) sessions: Vec<QualityGateSession>,
}
