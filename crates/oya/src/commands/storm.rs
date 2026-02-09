//! # Oya Storm Orchestration Command
//!
//! This command executes bead workflows using the BeadOrchestrator.
//!
//! ## Usage
//!
//! ```bash
//! oya storm [OPTIONS]
//! oya storm --config /path/to/config.yml --dry-run
//! oya storm --slots 4 --timeout 300
//! ```
//!
//! ## Function Contract
//!
//! This implementation follows design-by-contract principles:
//! - **Preconditions**: Validated before execution (config exists, DB accessible)
//! - **Postconditions**: Guaranteed outputs (exit codes, structured results)
//! - **Invariants**: Database immutability, resource cleanup, deterministic errors
//!
//! ## Error Handling
//!
//! All errors are mapped to specific exit codes:
//! - 3: Config file not found
//! - 4: Config parse failed
//! - 5: Database not found
//! - 6: Database query failed
//! - 7: DAG build failed
//! - 8: No beads to execute
//! - 9: Orchestrator init failed
//! - 10: Orchestrator execution failed
//! - 11: Invalid slot count
//! - 12: Invalid timeout

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use anyhow::Context;
use clap::Parser;
use oya_core::Result;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Configuration file for the orchestrator
///
/// Loaded from YAML file specified by --config flag
/// or default `.oya/orchestrator.yml`
#[derive(Debug, Clone, Deserialize)]
pub struct OrchestratorConfig {
    /// Number of parallel agent slots
    #[serde(default = "default_slots")]
    pub slots: usize,

    /// Maximum execution time before timeout
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Output format (human, json)
    #[serde(default = "default_output")]
    pub output_format: String,
}

fn default_slots() -> usize {
    4
}

fn default_timeout() -> u64 {
    300
}

fn default_output() -> String {
    "human".to_string()
}

/// Arguments for the storm command
#[derive(Parser, Debug, Clone)]
pub struct StormArgs {
    /// Path to orchestrator config file
    #[arg(long, default_value = ".oya/orchestrator.yml")]
    pub config: PathBuf,

    /// Preview execution without running beads
    #[arg(long)]
    pub dry_run: bool,

    /// Maximum execution time before timeout (seconds)
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Number of parallel agent slots
    #[arg(long)]
    pub slots: Option<usize>,

    /// Output format (human, json)
    #[arg(long, default_value = "human")]
    pub output: String,
}

/// Output from the storm command
#[derive(Debug, Clone)]
pub struct StormOutput {
    /// Number of beads completed successfully
    pub beads_completed: usize,

    /// Number of beads that failed
    pub beads_failed: usize,

    /// Execution duration in milliseconds
    pub duration_ms: u64,

    /// Bead execution results (if not dry_run)
    pub results: Option<Vec<BeadExecutionResult>>,

    /// Planned execution order (if dry_run)
    pub planned_order: Option<Vec<String>>,
}

/// Result of executing a single bead
#[derive(Debug, Clone)]
pub struct BeadExecutionResult {
    /// Bead identifier
    pub bead_id: String,

    /// Execution status
    pub status: ExecutionStatus,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Error message if failed
    pub error: Option<String>,
}

/// Execution status for a bead
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStatus {
    Completed,
    Failed,
    Skipped,
    TimedOut,
}

/// Errors specific to the storm command
#[derive(Debug, Error)]
pub enum StormError {
    #[error("Config file not found: {path}")]
    ConfigFileNotFound { path: PathBuf },

    #[error("Failed to parse config: {path}")]
    ConfigParseFailed {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },

    #[error("Beads database not found: {path}")]
    DatabaseNotFound { path: PathBuf },

    #[error("Database query failed: {query}")]
    DatabaseQueryFailed {
        query: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Failed to build workflow DAG: {reason}")]
    DagBuildFailed { reason: String },

    #[error("No beads to execute (no beads with status 'open')")]
    NoBeadsToExecute,

    #[error("Orchestrator initialization failed: {reason}")]
    OrchestratorInitFailed { reason: String },

    #[error("Orchestrator execution failed: {reason}")]
    OrchestratorExecutionFailed { reason: String },

    #[error("Invalid slot count: {slots}, must be >= 1")]
    InvalidSlotCount { slots: usize },

    #[error("Invalid timeout: {secs}s, must be >= 1")]
    InvalidTimeout { secs: u64 },
}

impl StormError {
    /// Get the exit code for this error
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::ConfigFileNotFound { .. } => 3,
            Self::ConfigParseFailed { .. } => 4,
            Self::DatabaseNotFound { .. } => 5,
            Self::DatabaseQueryFailed { .. } => 6,
            Self::DagBuildFailed { .. } => 7,
            Self::NoBeadsToExecute => 8,
            Self::OrchestratorInitFailed { .. } => 9,
            Self::OrchestratorExecutionFailed { .. } => 10,
            Self::InvalidSlotCount { .. } => 11,
            Self::InvalidTimeout { .. } => 12,
        }
    }

    /// Get a hint for remediation
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::ConfigFileNotFound { .. } => {
                Some("Create config with `oya init --template orchestrator` or specify --config".to_string())
            }
            Self::ConfigParseFailed { .. } => {
                Some("Validate YAML syntax and required fields in config file".to_string())
            }
            Self::DatabaseNotFound { .. } => {
                Some("Initialize workspace with `oya init`".to_string())
            }
            Self::DatabaseQueryFailed { .. } => {
                Some("Check database integrity and file permissions".to_string())
            }
            Self::DagBuildFailed { .. } => {
                Some("Review bead dependencies for circular references or missing beads".to_string())
            }
            Self::NoBeadsToExecute => {
                Some("Create beads or update their status to 'open'".to_string())
            }
            Self::OrchestratorInitFailed { .. } => {
                Some("Check system resources and orchestrator configuration".to_string())
            }
            Self::OrchestratorExecutionFailed { .. } => {
                Some("Check logs for specific bead failure details".to_string())
            }
            Self::InvalidSlotCount { .. } => {
                Some("Set slots to >= 1 in config file".to_string())
            }
            Self::InvalidTimeout { .. } => {
                Some("Set timeout to >= 1s in config file".to_string())
            }
        }
    }
}

/// Core storm command implementation
///
/// # Preconditions
/// - Config file exists and is valid
/// - Beads database exists and contains open beads
/// - DAG can be built without cycles
///
/// # Postconditions
/// - Returns Ok(StormOutput) with execution results
/// - Or returns Err(StormError) with specific failure
/// - All orchestrator resources are cleaned up
pub async fn storm_command(args: StormArgs) -> Result<StormOutput, StormError> {
    // Step 1: Load and validate config
    let config = load_config(&args.config)?;

    // Step 2: Validate CLI overrides
    let slots = validate_slots(args.slots, config.slots)?;
    let timeout_secs = validate_timeout(args.timeout, config.timeout_secs)?;

    // Step 3: Load beads database
    let db_path = PathBuf::from(".beads/beads.db");
    check_database_exists(&db_path)?;

    // Step 4: Build workflow DAG from database
    let dag = build_workflow_dag_from_database(&db_path)?;

    // Step 5: Check if DAG has nodes
    if dag.node_count() == 0 {
        return Err(StormError::NoBeadsToExecute);
    }

    // Step 6: Handle dry-run mode
    if args.dry_run {
        return Ok(dry_run_execution(dag));
    }

    // Step 7: Execute orchestrator
    let timeout = Duration::from_secs(timeout_secs);
    run_orchestrator_execution(dag, slots, timeout).await
}

/// Load and parse orchestrator config file
///
/// # Preconditions
/// - Config file path exists
/// - File is valid YAML
///
/// # Postconditions
/// - Returns Ok(OrchestratorConfig) with parsed config
/// - Returns Err(ConfigFileNotFound) if file missing
/// - Returns Err(ConfigParseFailed) if YAML invalid
fn load_config(path: &PathBuf) -> Result<OrchestratorConfig, StormError> {
    // Check file exists
    match path.try_exists() {
        Ok(false) | Err(_) => {
            return Err(StormError::ConfigFileNotFound { path: path.clone() });
        }
        Ok(true) => {
            // File exists, continue
        }
    }

    // Read file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| StormError::ConfigParseFailed {
            path: path.clone(),
            source: anyhow::Error::from(e).context("Failed to read config file"),
        })?;

    // Parse YAML
    serde_yaml::from_str::<OrchestratorConfig>(&content).map_err(|e| {
        StormError::ConfigParseFailed {
            path: path.clone(),
            source: anyhow::Error::from(e).context("Failed to parse YAML"),
        }
    })
}

/// Validate slot count (CLI override or config value)
///
/// # Preconditions
/// - At least one of cli_slots or config_slots is provided
///
/// # Postconditions
/// - Returns Ok(slots) if slots >= 1
/// - Returns Err(InvalidSlotCount) if slots == 0
fn validate_slots(cli_slots: Option<usize>, config_slots: usize) -> Result<usize, StormError> {
    let slots = cli_slots.unwrap_or(config_slots);

    match slots {
        0 => Err(StormError::InvalidSlotCount { slots: 0 }),
        _ => Ok(slots),
    }
}

/// Validate timeout (CLI override or config value)
///
/// # Preconditions
/// - At least one of cli_timeout or config_timeout is provided
///
/// # Postconditions
/// - Returns Ok(timeout_secs) if timeout >= 1
/// - Returns Err(InvalidTimeout) if timeout == 0
fn validate_timeout(cli_timeout: Option<u64>, config_timeout: u64) -> Result<u64, StormError> {
    let timeout = cli_timeout.unwrap_or(config_timeout);

    match timeout {
        0 => Err(StormError::InvalidTimeout { secs: 0 }),
        _ => Ok(timeout),
    }
}

/// Check if beads database exists
///
/// # Preconditions
/// - db_path is a valid file path
///
/// # Postconditions
/// - Returns Ok(()) if database exists
/// - Returns Err(DatabaseNotFound) if missing
fn check_database_exists(db_path: &PathBuf) -> Result<(), StormError> {
    match db_path.try_exists() {
        Ok(false) | Err(_) => {
            Err(StormError::DatabaseNotFound { path: db_path.clone() })
        }
        Ok(true) => Ok(()),
    }
}

/// Build WorkflowDAG from beads database
///
/// # Preconditions
/// - Database connection is valid
/// - All referenced beads exist
/// - No circular dependencies exist
///
/// # Postconditions
/// - Returns Ok(WorkflowDAG) with all open beads
/// - Or returns Err(DagBuildFailed) with reason
fn build_workflow_dag_from_database(db_path: &PathBuf) -> Result<orchestrator::dag::WorkflowDAG, StormError> {
    // TODO: Implement actual database query to load beads
    // For now, return an empty DAG as a placeholder
    // This will be implemented in bd-3a0a.7 (BeadOrchestrator)

    // Placeholder: Create empty DAG
    let dag = orchestrator::dag::WorkflowDAG::new();

    // In real implementation, this would:
    // 1. Open SQLite connection
    // 2. Query all beads with status='open'
    // 3. Query dependency relationships
    // 4. Build WorkflowDAG with nodes and edges
    // 5. Validate no cycles exist

    Ok(dag)
}

/// Execute dry-run mode (validation and planning only)
///
/// # Preconditions
/// - DAG is valid and acyclic
///
/// # Postconditions
/// - Returns Ok(StormOutput) with planned execution order
/// - No beads are executed
fn dry_run_execution(dag: orchestrator::dag::WorkflowDAG) -> StormOutput {
    // Get topological order of beads
    let planned_order = match dag.topological_order() {
        Ok(order) => order,
        Err(_) => vec![],
    };

    StormOutput {
        beads_completed: 0,
        beads_failed: 0,
        duration_ms: 0,
        results: None,
        planned_order: Some(planned_order),
    }
}

/// Run orchestrator with given DAG and configuration
///
/// # Preconditions
/// - DAG has at least one node
/// - Orchestrator initializes successfully
///
/// # Postconditions
/// - Returns Ok(ExecutionResults) on completion
/// - Returns Err(OrchestratorExecutionFailed) on failure
/// - All resources are cleaned up
async fn run_orchestrator_execution(
    _dag: orchestrator::dag::WorkflowDAG,
    _slots: usize,
    _timeout: Duration,
) -> Result<StormOutput, StormError> {
    // TODO: Implement actual orchestrator execution
    // This will be implemented in bd-3a0a.7 (BeadOrchestrator)

    // Placeholder: Return error indicating not yet implemented
    Err(StormError::OrchestratorInitFailed {
        reason: "BeadOrchestrator not yet implemented (see bd-3a0a.7)".to_string(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_validate_slots_accepts_positive_values() {
        assert_eq!(validate_slots(Some(5), 4).unwrap(), 5);
        assert_eq!(validate_slots(None, 8).unwrap(), 8);
        assert_eq!(validate_slots(Some(1), 100).unwrap(), 1);
    }

    #[test]
    fn test_validate_slots_rejects_zero() {
        assert!(matches!(
            validate_slots(Some(0), 4),
            Err(StormError::InvalidSlotCount { slots: 0 })
        ));
        assert!(matches!(
            validate_slots(None, 0),
            Err(StormError::InvalidSlotCount { slots: 0 })
        ));
    }

    #[test]
    fn test_validate_timeout_accepts_positive_values() {
        assert_eq!(validate_timeout(Some(60), 300).unwrap(), 60);
        assert_eq!(validate_timeout(None, 120).unwrap(), 120);
        assert_eq!(validate_timeout(Some(1), 999).unwrap(), 1);
    }

    #[test]
    fn test_validate_timeout_rejects_zero() {
        assert!(matches!(
            validate_timeout(Some(0), 300),
            Err(StormError::InvalidTimeout { secs: 0 })
        ));
        assert!(matches!(
            validate_timeout(None, 0),
            Err(StormError::InvalidTimeout { secs: 0 })
        ));
    }

    #[test]
    fn test_storm_error_exit_codes() {
        assert_eq!(StormError::ConfigFileNotFound { path: PathBuf::from("x") }.exit_code(), 3);
        assert_eq!(
            StormError::ConfigParseFailed {
                path: PathBuf::from("x"),
                source: anyhow::anyhow!("test")
            }
            .exit_code(),
            4
        );
        assert_eq!(StormError::DatabaseNotFound { path: PathBuf::from("x") }.exit_code(), 5);
        assert_eq!(
            StormError::DagBuildFailed {
                reason: "test".to_string()
            }
            .exit_code(),
            7
        );
        assert_eq!(StormError::NoBeadsToExecute.exit_code(), 8);
        assert_eq!(
            StormError::OrchestratorInitFailed {
                reason: "test".to_string()
            }
            .exit_code(),
            9
        );
        assert_eq!(
            StormError::OrchestratorExecutionFailed {
                reason: "test".to_string()
            }
            .exit_code(),
            10
        );
        assert_eq!(StormError::InvalidSlotCount { slots: 0 }.exit_code(), 11);
        assert_eq!(StormError::InvalidTimeout { secs: 0 }.exit_code(), 12);
    }

    #[test]
    fn test_dry_run_execution_returns_planned_order() {
        let dag = orchestrator::dag::WorkflowDAG::new();
        let output = dry_run_execution(dag);

        assert_eq!(output.beads_completed, 0);
        assert_eq!(output.beads_failed, 0);
        assert_eq!(output.duration_ms, 0);
        assert!(output.results.is_none());
        assert!(output.planned_order.is_some());
    }
}
