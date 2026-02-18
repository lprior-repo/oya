//! Orchestrator trait - abstract the workflow execution
//!
//! This trait allows us to test orchestration logic without running real tools.
//!
//! # Testing Strategy
//!
//! 1. **Unit tests** - Test state transitions directly
//! 2. **FakeOrchestrator** - In-memory implementation for integration tests
//! 3. **RealOrchestrator** - Production implementation in main.rs
//!
//! # Example
//!
//! ```rust,no_run
//! use oya::orchestrator::{Orchestrator, StageExecutionResult};
//! use oya::types::{StageName, FailureCategory};
//!
//! async fn test_happy_path<T: Orchestrator>(orch: T) {
//!     let result = orch.run_stage(
//!         StageName::Research,
//!         1,
//!         "bead-001",
//!         "test context",
//!         None::<(FailureCategory, String)>
//!     ).await.unwrap();
//!     assert!(result.passed);
//! }
//! ```

use crate::types::{FailureCategory, StageName};
use serde_json::Value;
use thiserror::Error;

/// Result of executing a stage
#[derive(Debug, Clone)]
pub struct StageExecutionResult {
    pub passed: bool,
    pub output: Value,
    pub failure_category: Option<FailureCategory>,
    pub next_stage: Option<StageName>,
    pub prompt: String,
}

/// Gate execution result
#[derive(Debug, Clone)]
pub struct GateResult {
    pub gate_name: String,
    pub command: String,
    pub passed: bool,
    pub exit_code: i32,
    pub output: String,
}

/// Errors that can occur during orchestration
#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("Stage execution failed: {0}")]
    StageExecution(String),

    #[error("Gate execution failed: {0}")]
    GateExecution(String),

    #[error("Workspace preparation failed: {0}")]
    WorkspacePrep(String),

    #[error("Invalid configuration: {0}")]
    Config(String),
}

/// Abstract orchestrator trait
///
/// Implementations:
/// - `RealOrchestrator` - Production, calls real tools
/// - `FakeOrchestrator` - Testing, configurable behavior
#[async_trait::async_trait]
pub trait Orchestrator: Send + Sync {
    /// Execute a single stage
    async fn run_stage(
        &self,
        stage: StageName,
        attempt: u32,
        bead_id: &str,
        context: &str,
        last_failure: Option<(FailureCategory, String)>,
    ) -> Result<StageExecutionResult, OrchestratorError>;

    /// Execute a quality gate
    fn run_gate(&self, gate: crate::types::Gate) -> Result<GateResult, OrchestratorError>;

    /// Prepare workspace for a stage
    fn prepare_workspace(
        &self,
        run_id: &str,
        bead_id: &str,
        stage: &StageName,
        attempt: u32,
    ) -> Result<Option<()>, OrchestratorError>;

    /// Get the current run ID
    fn run_id(&self) -> &str;

    /// Get the bead ID
    fn bead_id(&self) -> &str;
}

/// Configuration for fake orchestrator
#[derive(Debug, Clone)]
pub struct FakeOrchestratorConfig {
    /// Predefined stage results (stage -> attempt -> result)
    pub stage_results: std::collections::HashMap<(StageName, u32), StageExecutionResult>,

    /// Default result if not in map
    pub default_result: StageExecutionResult,

    /// Gate results (gate_name -> result)
    pub gate_results: std::collections::HashMap<String, GateResult>,

    /// Simulate delays (milliseconds)
    pub delay_ms: u64,

    /// Track calls for assertions
    pub track_calls: bool,
}

impl Default for FakeOrchestratorConfig {
    fn default() -> Self {
        Self {
            stage_results: std::collections::HashMap::new(),
            default_result: StageExecutionResult {
                passed: true,
                output: serde_json::json!({"output": "success"}),
                failure_category: None,
                next_stage: None,
                prompt: "test prompt".to_string(),
            },
            gate_results: std::collections::HashMap::new(),
            delay_ms: 0,
            track_calls: true,
        }
    }
}

/// Fake orchestrator for testing
///
/// Configurable behavior allows testing various scenarios:
/// - Stage failures
/// - Retry exhaustion  
/// - Gate failures
/// - Success paths
pub struct FakeOrchestrator {
    config: FakeOrchestratorConfig,
    run_id: String,
    bead_id: String,
    calls: std::sync::Arc<std::sync::Mutex<Vec<CallRecord>>>,
}

#[derive(Debug, Clone)]
pub struct CallRecord {
    pub method: String,
    pub stage: Option<StageName>,
    pub attempt: Option<u32>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl FakeOrchestrator {
    pub fn new(config: FakeOrchestratorConfig, run_id: String, bead_id: String) -> Self {
        Self {
            config,
            run_id,
            bead_id,
            calls: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn calls(&self) -> Vec<CallRecord> {
        self.calls.lock().map(|guard| guard.clone()).unwrap_or_else(|_| Vec::new())
    }

    pub fn stage_calls(&self, stage: StageName) -> Vec<CallRecord> {
        self.calls().into_iter().filter(|c| c.stage == Some(stage.clone())).collect()
    }

    fn record_call(&self, method: &str, stage: Option<StageName>, attempt: Option<u32>) {
        if self.config.track_calls {
            if let Ok(mut guard) = self.calls.lock() {
                guard.push(CallRecord {
                    method: method.to_string(),
                    stage,
                    attempt,
                    timestamp: chrono::Utc::now(),
                });
            }
        }
    }
}

#[async_trait::async_trait]
impl Orchestrator for FakeOrchestrator {
    async fn run_stage(
        &self,
        stage: StageName,
        attempt: u32,
        _bead_id: &str,
        _context: &str,
        _last_failure: Option<(FailureCategory, String)>,
    ) -> Result<StageExecutionResult, OrchestratorError> {
        self.record_call("run_stage", Some(stage.clone()), Some(attempt));

        if self.config.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.config.delay_ms)).await;
        }

        let key = (stage.clone(), attempt);
        let result = self.config.stage_results.get(&key).cloned().unwrap_or_else(|| {
            let mut default = self.config.default_result.clone();
            default.next_stage = stage.next();
            default
        });

        Ok(result)
    }

    fn run_gate(&self, gate: crate::types::Gate) -> Result<GateResult, OrchestratorError> {
        self.record_call("run_gate", None, None);

        let gate_name = gate.as_str().to_string();
        self.config.gate_results.get(&gate_name).cloned().map(Ok).unwrap_or_else(|| {
            Ok(GateResult {
                gate_name: gate_name.clone(),
                command: format!("mock-{}", gate_name),
                passed: true,
                exit_code: 0,
                output: "mock gate passed".to_string(),
            })
        })
    }

    fn prepare_workspace(
        &self,
        _run_id: &str,
        _bead_id: &str,
        stage: &StageName,
        attempt: u32,
    ) -> Result<Option<()>, OrchestratorError> {
        self.record_call("prepare_workspace", Some(stage.clone()), Some(attempt));
        Ok(Some(()))
    }

    fn run_id(&self) -> &str {
        &self.run_id
    }

    fn bead_id(&self) -> &str {
        &self.bead_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_orch() -> FakeOrchestrator {
        FakeOrchestrator::new(
            FakeOrchestratorConfig::default(),
            "test-run-001".to_string(),
            "test-bead-001".to_string(),
        )
    }

    #[tokio::test]
    async fn fake_orch_returns_default_success() {
        let orch = fake_orch();
        let result = orch
            .run_stage(StageName::Research, 1, "test-bead", "test context", None)
            .await
            .unwrap();

        assert!(result.passed);
        assert_eq!(result.next_stage, Some(StageName::Plan));
    }

    #[tokio::test]
    async fn fake_orch_records_calls() {
        let orch = fake_orch();

        orch.run_stage(StageName::Research, 1, "b", "c", None).await.unwrap();
        orch.run_stage(StageName::Research, 2, "b", "c", None).await.unwrap();

        let calls = orch.stage_calls(StageName::Research);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].attempt, Some(1));
        assert_eq!(calls[1].attempt, Some(2));
    }

    #[tokio::test]
    async fn fake_orch_uses_configured_results() {
        let mut config = FakeOrchestratorConfig::default();
        config.stage_results.insert(
            (StageName::Tdd15, 1),
            StageExecutionResult {
                passed: false,
                output: serde_json::json!({"error": "test failed"}),
                failure_category: Some(FailureCategory::TestFailed),
                next_stage: Some(StageName::Tdd15),
                prompt: "fix the tests".to_string(),
            },
        );

        let orch = FakeOrchestrator::new(config, "run-001".to_string(), "bead-001".to_string());

        let result = orch.run_stage(StageName::Tdd15, 1, "b", "c", None).await.unwrap();

        assert!(!result.passed);
        assert_eq!(result.failure_category, Some(FailureCategory::TestFailed));
    }
}
