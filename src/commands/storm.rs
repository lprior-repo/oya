#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Storm command implementation
//!
//! Orchestrates bead execution using the BeadOrchestrator and WorkflowDAG.

use anyhow::Result;
use clap::Parser;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::fs;
use tracing::{info, warn};

/// Arguments for the storm command
#[derive(Parser, Debug, Clone)]
pub struct StormArgs {
    /// Path to orchestrator config file
    #[arg(long, default_value = ".oya/orchestrator.yml")]
    pub config: PathBuf,

    /// Preview execution without running beads
    #[arg(long)]
    pub dry_run: bool,

    /// Maximum execution time before timeout
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StormOutput {
    /// Number of beads completed successfully
    pub beads_completed: usize,

    /// Number of beads that failed
    pub beads_failed: usize,

    /// Execution duration in milliseconds
    pub duration_ms: u64,

    /// Bead execution results (if not dry_run)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<BeadExecutionResult>>,

    /// Planned execution order (if dry_run)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_order: Option<Vec<String>>,
}

/// Result of executing a single bead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadExecutionResult {
    /// Bead identifier
    pub bead_id: String,

    /// Execution status
    pub status: ExecutionStatus,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Execution status for a bead
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// Orchestrator configuration from YAML file
#[derive(Debug, Clone, Deserialize)]
struct OrchestratorConfig {
    /// Number of parallel agent slots
    #[serde(default = "default_slots")]
    slots: usize,

    /// Maximum execution time in seconds
    #[serde(default = "default_timeout")]
    timeout_secs: u64,

    /// Additional configuration options
    #[serde(default)]
    #[allow(dead_code)]
    extra: HashMap<String, serde_yaml::Value>,
}

fn default_slots() -> usize {
    4
}

fn default_timeout() -> u64 {
    600
}

/// Core storm command implementation
///
/// Preconditions:
/// - Config file exists and is valid
/// - Beads database exists and contains open beads
/// - DAG can be built without cycles
///
/// Postconditions:
/// - Returns Ok(StormOutput) with execution results
/// - Or returns Err(StormError) with specific failure
/// - All orchestrator resources are cleaned up
pub async fn storm_command(args: StormArgs) -> Result<StormOutput, StormError> {
    let start = Instant::now();

    // Load and validate config
    let config = load_config(&args.config).await?;

    // Override config with command-line arguments
    let slots = args.slots.unwrap_or(config.slots);
    let timeout = args.timeout.unwrap_or(config.timeout_secs);

    // Validate slots and timeout
    if slots == 0 {
        return Err(StormError::InvalidSlotCount { slots });
    }
    if timeout == 0 {
        return Err(StormError::InvalidTimeout { secs: timeout });
    }

    // Load beads database
    let db_path = PathBuf::from(".beads/beads.db");
    if !db_path.exists() {
        return Err(StormError::DatabaseNotFound { path: db_path });
    }

    // Build workflow DAG from database
    let dag = build_workflow_dag(&db_path).await?;

    // Handle dry-run mode
    if args.dry_run {
        info!("Dry run mode: planning execution without running beads");
        let planned_order = topological_order(&dag)?;
        return Ok(StormOutput {
            beads_completed: 0,
            beads_failed: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            results: None,
            planned_order: Some(planned_order),
        });
    }

    // Run orchestrator
    let results = run_orchestrator(dag, slots, timeout).await?;

    // Count completed and failed beads
    let beads_completed = results
        .iter()
        .filter(|r| r.status == ExecutionStatus::Completed)
        .count();

    let beads_failed = results
        .iter()
        .filter(|r| r.status == ExecutionStatus::Failed)
        .count();

    Ok(StormOutput {
        beads_completed,
        beads_failed,
        duration_ms: start.elapsed().as_millis() as u64,
        results: Some(results),
        planned_order: None,
    })
}

/// Load orchestrator configuration from YAML file
async fn load_config(path: &Path) -> Result<OrchestratorConfig, StormError> {
    if !path.exists() {
        return Err(StormError::ConfigFileNotFound {
            path: path.to_path_buf(),
        });
    }

    let content = fs::read_to_string(path)
        .await
        .map_err(|_| StormError::ConfigFileNotFound {
            path: path.to_path_buf(),
        })?;

    serde_yaml::from_str(&content).map_err(|e| StormError::ConfigParseFailed {
        path: path.to_path_buf(),
        source: anyhow::Error::from(e),
    })
}

/// Build WorkflowDAG from beads database
///
/// Preconditions:
/// - Database connection is valid
/// - All referenced beads exist
/// - No circular dependencies exist
///
/// Postconditions:
/// - Returns Ok(WorkflowDAG) with all open beads
/// - Or returns Err(DagBuildFailed) with reason
async fn build_workflow_dag(db_path: &Path) -> Result<WorkflowDAG, StormError> {
    use orchestrator::dag::{DependencyType, WorkflowDAG};

    // Open database connection
    let conn = rusqlite::Connection::open(db_path).map_err(|e| StormError::DatabaseNotFound {
        path: db_path.to_path_buf(),
    })?;

    // Query all open beads
    let mut stmt = conn
        .prepare("SELECT id FROM issues WHERE status = 'open'")
        .map_err(|e| StormError::DatabaseQueryFailed {
            query: "SELECT id FROM issues WHERE status = 'open'".to_string(),
            source: anyhow::Error::from(e),
        })?;

    let bead_ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| StormError::DatabaseQueryFailed {
            query: "SELECT id FROM issues WHERE status = 'open'".to_string(),
            source: anyhow::Error::from(e),
        })?
        .filter_map(|r| r.ok())
        .collect();

    if bead_ids.is_empty() {
        return Err(StormError::NoBeadsToExecute);
    }

    // Create DAG and add nodes
    let mut dag = WorkflowDAG::new();
    for bead_id in &bead_ids {
        dag.add_node(bead_id.clone()).map_err(|e| StormError::DagBuildFailed {
            reason: format!("Failed to add bead {bead_id}: {e}"),
        })?;
    }

    // Query dependencies and add edges
    let mut dep_stmt = conn
        .prepare(
            "SELECT issue_id, depends_on_id FROM dependencies
             WHERE type = 'blocks' AND issue_id IN (SELECT id FROM issues WHERE status = 'open')
             AND depends_on_id IN (SELECT id FROM issues WHERE status = 'open')",
        )
        .map_err(|e| StormError::DatabaseQueryFailed {
            query: "SELECT issue_id, depends_on_id FROM dependencies".to_string(),
            source: anyhow::Error::from(e),
        })?;

    let deps: Vec<(String, String)> = dep_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| StormError::DatabaseQueryFailed {
            query: "SELECT issue_id, depends_on_id FROM dependencies".to_string(),
            source: anyhow::Error::from(e),
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (issue_id, depends_on_id) in deps {
        dag.add_edge(
            depends_on_id.clone(),
            issue_id.clone(),
            DependencyType::BlockingDependency,
        )
            .map_err(|e| {
                StormError::DagBuildFailed {
                    reason: format!(
                        "Failed to add dependency {depends_on_id} -> {issue_id}: {e}"
                    ),
                }
            })?;
    }

    Ok(dag)
}

/// Get topological order of beads in DAG
fn topological_order(dag: &WorkflowDAG) -> Result<Vec<String>, StormError> {
    use orchestrator::dag::WorkflowDAG;

    // For now, return empty vec as topological sort will be implemented
    // when WorkflowDAG exposes its internal petgraph structure
    // The dry-run is primarily for validation that DAG can be built
    warn!("Topological sort not yet implemented - returning empty order");
    warn!("This will be completed when WorkflowDAG exposes petgraph accessors");

    Ok(Vec::new())
}

/// Run BeadOrchestrator with given DAG and config
///
/// Preconditions:
/// - DAG has at least one node
/// - Orchestrator initializes successfully
///
/// Postconditions:
/// - Returns Ok(ExecutionResults) on completion
/// - Returns Err(OrchestratorExecutionFailed) on failure
/// - All resources are cleaned up
async fn run_orchestrator(
    _dag: WorkflowDAG,
    _slots: usize,
    _timeout_secs: u64,
) -> Result<Vec<BeadExecutionResult>, StormError> {
    // Placeholder implementation
    // This will be implemented in bd-3a0a.7 (BeadOrchestrator)
    // For now, return empty results to satisfy the contract

    info!("Orchestrator execution not yet implemented");
    info!("This will be provided by bd-3a0a.7 (BeadOrchestrator)");

    Ok(Vec::new())
}

// Type alias for WorkflowDAG to avoid import issues
type WorkflowDAG = orchestrator::dag::WorkflowDAG;
