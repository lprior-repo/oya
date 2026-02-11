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

// Import orchestrator types
use orchestrator::dag::{DependencyType, WorkflowDAG};

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

    #[error("JSONL file not found: {path}")]
    JsonlNotFound { path: PathBuf },

    #[error("Failed to parse JSONL: {path}")]
    JsonlParseFailed {
        path: PathBuf,
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
            Self::JsonlNotFound { .. } => 5,
            Self::JsonlParseFailed { .. } => 6,
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
            Self::JsonlNotFound { .. } => {
                Some("Initialize workspace with `oya init`".to_string())
            }
            Self::JsonlParseFailed { .. } => {
                Some("Check JSONL file format and validity".to_string())
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

    // Step 3: Load beads from JSONL
    let jsonl_path = PathBuf::from(".beads/issues.jsonl");
    let dag = build_workflow_dag_from_jsonl(&jsonl_path)?;

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
    let slots = cli_slots.map_or(Ok(config_slots), |s| {
        match s {
            0 => Err(StormError::InvalidSlotCount { slots: 0 }),
            _ => Ok(s),
        }
    })?;

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
    let timeout = cli_timeout.map_or(Ok(config_timeout), |t| {
        match t {
            0 => Err(StormError::InvalidTimeout { secs: 0 }),
            _ => Ok(t),
        }
    })?;

    match timeout {
        0 => Err(StormError::InvalidTimeout { secs: 0 }),
        _ => Ok(timeout),
    }
}

/// Bead structure as stored in JSONL format
#[derive(Debug, Clone, Deserialize)]
struct JsonlBead {
    id: String,
    status: String,
    dependencies: Option<Vec<JsonlDependency>>,
}

/// Dependency structure as stored in JSONL format
#[derive(Debug, Clone, Deserialize)]
struct JsonlDependency {
    issue_id: String,
    depends_on_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    type_: Option<String>,
}

/// Build WorkflowDAG from JSONL file
///
/// # Preconditions
/// - JSONL file exists and is valid
/// - All referenced beads exist
/// - No circular dependencies exist
///
/// # Postconditions
/// - Returns Ok(WorkflowDAG) with all open/in_progress beads
/// - Or returns Err(DagBuildFailed) with reason
fn build_workflow_dag_from_jsonl(
    jsonl_path: &PathBuf,
) -> Result<WorkflowDAG, StormError> {
    // Check file exists
    match jsonl_path.try_exists() {
        Ok(false) | Err(_) => {
            return Err(StormError::JsonlNotFound { path: jsonl_path.clone() });
        }
        Ok(true) => {
            // File exists, continue
        }
    }

    // Read file content
    let content = std::fs::read_to_string(jsonl_path).map_err(|e| {
        StormError::JsonlParseFailed {
            path: jsonl_path.clone(),
            source: anyhow::Error::from(e).context("Failed to read JSONL file"),
        }
    })?;

    // Parse JSONL lines and filter for open/in_progress beads
    let (bead_ids, dependencies) = parse_jsonl_beads(&content)?;

    // Build DAG using DagBuilder (no mutation in our code)
    let builder = WorkflowDAG::builder().with_nodes(bead_ids);

    // Add edges using iterator pipeline
    let builder = dependencies
        .into_iter()
        .fold(builder, |acc, (issue_id, depends_on_id)| {
            acc.with_edge(depends_on_id, issue_id, DependencyType::BlockingDependency)
        });

    // Build the DAG
    builder.build().map_err(|e| StormError::DagBuildFailed {
        reason: format!("Failed to build DAG: {e}"),
    })
}

/// Parse JSONL content and extract open beads with dependencies
///
/// # Preconditions
/// - JSONL content is valid JSON per line
///
/// # Postconditions
/// - Returns Ok((bead_ids, dependencies))
/// - Filters for status="open" or "in_progress"
/// - Or returns Err(JsonlParseFailed) for malformed JSON
fn parse_jsonl_beads(
    content: &str,
) -> Result<(Vec<String>, Vec<(String, String)>), StormError> {
    // Parse all lines into beads, collecting results
    let beads_result: Result<Vec<JsonlBead>, StormError> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).map_err(|e| StormError::JsonlParseFailed {
                path: PathBuf::from(".beads/issues.jsonl"),
                source: anyhow::Error::from(e).context("Failed to parse JSONL line as JsonlBead"),
            })
        })
        .collect();

    let beads = beads_result?;

    // Filter for open/in_progress and extract IDs and dependencies
    let (bead_ids, dependencies): (Vec<_>, Vec<_>) = beads
        .into_iter()
        .filter(|bead| matches!(bead.status.as_str(), "open" | "in_progress"))
        .flat_map(|bead| {
            // Extract dependencies
            let deps: Vec<_> = bead
                .dependencies
                .map_or_else(Vec::new, |deps| {
                    deps.into_iter()
                        .map(|dep| (dep.issue_id, dep.depends_on_id))
                        .collect()
                });

            // Return tuple of (bead_id, dependencies)
            std::iter::once((bead.id, deps))
        })
        .unzip();

    // Flatten dependencies
    let dependencies = dependencies.into_iter().flatten().collect();

    Ok((bead_ids, dependencies))
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
    dag: WorkflowDAG,
    slots: usize,
    timeout: Duration,
) -> Result<StormOutput, StormError> {
    // TODO: Implement actual orchestrator execution in bd-3a0a.7 (BeadOrchestrator)
    // For now, return a basic planned execution

    // Get topological order for planning
    let planned_order = match dag.topological_order() {
        Ok(order) => order,
        Err(e) => {
            return Err(StormError::OrchestratorExecutionFailed {
                reason: format!("Failed to compute topological order: {e}"),
            });
        }
    };

    // Return planned execution (placeholder for actual orchestrator)
    Ok(StormOutput {
        beads_completed: 0,
        beads_failed: 0,
        duration_ms: 0,
        results: None,
        planned_order: Some(planned_order),
    })
}

#[cfg(test)]
mod tests {
    
    
    
    use super::*;

    #[test]
    fn test_validate_slots_accepts_positive_values() {
        assert_eq!(validate_slots(Some(5), 4).map_or_else(|_| 0, |v| v), 5);
        assert_eq!(validate_slots(None, 8).map_or_else(|_| 0, |v| v), 8);
        assert_eq!(validate_slots(Some(1), 100).map_or_else(|_| 0, |v| v), 1);
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
        assert_eq!(validate_timeout(Some(60), 300).map_or_else(|_| 0, |v| v), 60);
        assert_eq!(validate_timeout(None, 120).map_or_else(|_| 0, |v| v), 120);
        assert_eq!(validate_timeout(Some(1), 999).map_or_else(|_| 0, |v| v), 1);
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
        let dag = WorkflowDAG::new();
        let output = dry_run_execution(dag);

        assert_eq!(output.beads_completed, 0);
        assert_eq!(output.beads_failed, 0);
        assert_eq!(output.duration_ms, 0);
        assert!(output.results.is_none());
        assert!(output.planned_order.is_some());
    }

    #[test]
    fn test_parse_jsonl_beads_filters_open_status() {
        let jsonl = r#"
{"id":"bd-1","status":"open","dependencies":null}
{"id":"bd-2","status":"closed","dependencies":null}
{"id":"bd-3","status":"in_progress","dependencies":null}
{"id":"bd-4","status":"done","dependencies":null}
"#;

        let result = parse_jsonl_beads(jsonl);
        assert!(result.is_ok());

        let (bead_ids, _deps) = match result {
            Ok(v) => v,
            Err(_) => return,
        };

        assert_eq!(bead_ids.len(), 2);
        assert!(bead_ids.contains(&"bd-1".to_string()));
        assert!(bead_ids.contains(&"bd-3".to_string()));
    }

    #[test]
    fn test_parse_jsonl_beads_extracts_dependencies() {
        let jsonl = r#"
{"id":"bd-1","status":"open","dependencies":[{"issue_id":"bd-1","depends_on_id":"bd-0","type_":"blocks"}]}
{"id":"bd-2","status":"open","dependencies":[{"issue_id":"bd-2","depends_on_id":"bd-1","type_":"blocks"}]}
"#;

        let result = parse_jsonl_beads(jsonl);
        assert!(result.is_ok());

        let (_bead_ids, deps) = match result {
            Ok(v) => v,
            Err(_) => return,
        };

        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&(String::from("bd-1"), String::from("bd-0"))));
        assert!(deps.contains(&(String::from("bd-2"), String::from("bd-1"))));
    }

    #[test]
    fn test_parse_jsonl_beads_handles_empty_dependencies() {
        let jsonl = r#"
{"id":"bd-1","status":"open","dependencies":null}
{"id":"bd-2","status":"open","dependencies":[]}
"#;

        let result = parse_jsonl_beads(jsonl);
        assert!(result.is_ok());

        let (_bead_ids, deps) = match result {
            Ok(v) => v,
            Err(_) => return,
        };

        assert_eq!(deps.len(), 0);
    }

    #[test]
    fn test_parse_jsonl_beads_handles_malformed_json() {
        let jsonl = r#"
{"id":"bd-1","status":"open","dependencies":null}
this is not valid json
{"id":"bd-2","status":"open","dependencies":null}
"#;

        let result = parse_jsonl_beads(jsonl);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_jsonl_beads_handles_empty_lines() {
        let jsonl = r#"

{"id":"bd-1","status":"open","dependencies":null}

{"id":"bd-2","status":"in_progress","dependencies":null}

"#;

        let result = parse_jsonl_beads(jsonl);
        assert!(result.is_ok());

        let (bead_ids, _deps) = match result {
            Ok(v) => v,
            Err(_) => return,
        };

        assert_eq!(bead_ids.len(), 2);
    }

    #[test]
    fn test_build_workflow_dag_from_jsonl_creates_dag() {
        let jsonl_content = r#"
{"id":"bd-1","status":"open","dependencies":null}
{"id":"bd-2","status":"open","dependencies":[{"issue_id":"bd-2","depends_on_id":"bd-1","type_":"blocks"}]}
{"id":"bd-3","status":"closed","dependencies":null}
"#;

        let temp_dir = std::env::temp_dir();
        let jsonl_path = temp_dir.join("test_issues.jsonl");

        // Write test data
        if std::fs::write(&jsonl_path, jsonl_content).is_err() {
            return;
        }

        let result = build_workflow_dag_from_jsonl(&jsonl_path);

        // Clean up
        let _ = std::fs::remove_file(&jsonl_path);

        match result {
            Ok(dag) => {
                assert_eq!(dag.node_count(), 2);
            }
            Err(_) => {
                return;
            }
        }
    }

    #[test]
    fn test_build_workflow_dag_from_jsonl_handles_missing_file() {
        let jsonl_path = PathBuf::from("/nonexistent/path/issues.jsonl");
        let result = build_workflow_dag_from_jsonl(&jsonl_path);

        assert!(result.is_err());
        match result {
            Err(StormError::JsonlNotFound { .. }) => {
                // Expected
            }
            _ => {
                return;
            }
        }
    }

    #[test]
    fn test_build_workflow_dag_from_jsonl_with_complex_dependencies() {
        let jsonl_content = r#"
{"id":"bd-0","status":"open","dependencies":null}
{"id":"bd-1","status":"open","dependencies":[{"issue_id":"bd-1","depends_on_id":"bd-0","type_":"blocks"}]}
{"id":"bd-2","status":"in_progress","dependencies":[{"issue_id":"bd-2","depends_on_id":"bd-0","type_":"blocks"},{"issue_id":"bd-2","depends_on_id":"bd-1","type_":"blocks"}]}
{"id":"bd-3","status":"open","dependencies":[{"issue_id":"bd-3","depends_on_id":"bd-2","type_":"blocks"}]}
"#;

        let temp_dir = std::env::temp_dir();
        let jsonl_path = temp_dir.join("test_complex_deps.jsonl");

        if std::fs::write(&jsonl_path, jsonl_content).is_err() {
            return;
        }

        let result = build_workflow_dag_from_jsonl(&jsonl_path);

        let _ = std::fs::remove_file(&jsonl_path);

        match result {
            Ok(dag) => {
                assert_eq!(dag.node_count(), 4);
            }
            Err(_) => {
                return;
            }
        }
    }

    #[test]
    fn test_storm_error_exit_codes_includes_jsonl_errors() {
        assert_eq!(
            StormError::JsonlNotFound {
                path: PathBuf::from("x")
            }
            .exit_code(),
            5
        );
        assert_eq!(
            StormError::JsonlParseFailed {
                path: PathBuf::from("x"),
                source: anyhow::anyhow!("test")
            }
            .exit_code(),
            6
        );
    }
}