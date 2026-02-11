# Contract Specification

**Bead ID**: bd-3a0a.9
**Feature**: cli: add oya storm orchestration command
**Generated**: 2026-02-09

## Context

### Feature Description
Create a new `oya storm` CLI command that:
1. Parses orchestrator configuration from a config file
2. Loads a workflow DAG from beads database
3. Runs the BeadOrchestrator to execute beads according to the DAG
4. Supports `--dry-run` flag to preview execution without running
5. Returns non-zero exit code on failures

### Domain Terms
- **WorkflowDAG**: Directed acyclic graph representing bead dependencies (from `orchestrator::dag`)
- **BeadOrchestrator**: Scheduler and executor for bead workflows (from `orchestrator::bead_orchestrator`)
- **OrchestratorConfig**: Configuration file containing orchestrator settings
- **Beads database**: SQLite database at `.beads/beads.db` containing bead definitions
- **Dry run**: Execution mode that validates and plans without executing beads
- **Exit code**: Process exit status (0 = success, non-zero = failure)

### Assumptions
1. BeadOrchestrator is implemented in `crates/orchestrator/src/bead_orchestrator.rs` (bd-3a0a.7)
2. IPC bridge for stage updates is implemented (bd-3a0a.8)
3. WorkflowDAG can be constructed from beads database queries
4. OrchestratorConfig follows YAML format similar to other Oya configs
5. Command should run synchronously (blocking until completion or failure)
6. Bead IDs from database match BeadId type (String) used in WorkflowDAG

### Open Questions
1. **Config file location**: Should be configurable via `--config` flag or default to `.oya/orchestrator.yml`?
2. **DAG loading strategy**: Should DAG be built from dependency relationships in beads database?
3. **Output format**: Should command output progress to stdout/stderr or just return exit code?
4. **Concurrency control**: Should config include parallel execution limits?
5. **Error handling granularity**: Should different failure modes have different exit codes?

## Preconditions

### For StormArgs::parse()
- Command-line arguments must be valid UTF-8
- If provided, config file path must be valid filesystem path

### For storm_command()
- Config file must exist and be readable
- Config file must be valid YAML with required fields
- Beads database at `.beads/beads.db` must exist
- Beads database must contain at least one bead with status `open`
- If config specifies slots, must be positive integer (> 0)
- If config specifies timeout, must be positive Duration (> 0s)

### For build_workflow_dag()
- Database connection must be valid
- All bead IDs referenced in dependencies must exist in database
- Database must not contain circular dependencies

### For run_orchestrator()
- WorkflowDAG must have at least one node
- BeadOrchestrator must initialize successfully
- Required system resources (threads, memory) must be available

## Postconditions

### For storm_command() on success
- Returns `Ok(StormOutput)` with completion status
- Orchestrator runs to completion or timeout
- All DAG-valid beads are executed according to dependencies
- Output contains: beads_completed, beads_failed, duration_ms
- Exit code 0 when invoked from main()

### For storm_command() with --dry-run
- Returns `Ok(StormOutput)` with preview data
- No beads are actually executed
- Output contains: planned_execution_order, total_beads, estimated_duration
- DAG validation still occurs

### For storm_command() on failure
- Returns `Err(StormError)` with specific error variant
- No partial execution state persists
- Exit code is non-zero and maps to error type

### For build_workflow_dag()
- Returns `Ok(WorkflowDAG)` with all open beads as nodes
- Dependencies from database are edges in DAG
- DAG is validated to be acyclic
- All beads in DAG have status `open`

### For run_orchestrator()
- If successful, all executable beads completed
- If failed, returns error with partial execution details
- Orchestrator resources are cleaned up (actors stopped)

## Invariants

- **DAG acyclicity**: WorkflowDAG never contains cycles
- **Bead ID consistency**: All bead IDs in DAG exist in database
- **Slot count positive**: If configured, agent slots >= 1
- **Exit code determinism**: Same error condition always produces same exit code
- **Database immutability**: Command never modifies beads database
- **Resource cleanup**: All spawned actors/threads terminate on exit

## Error Taxonomy

### StormError variants

**Error::ConfigFileNotFound**
- When: Config file path does not exist or is not readable
- Exit code: 3
- Hint: Check path or create config with `oya init --template orchestrator`

**Error::ConfigParseFailed**
- When: Config file is not valid YAML or missing required fields
- Exit code: 4
- Hint: Validate YAML syntax and required fields

**Error::DatabaseNotFound**
- When: `.beads/beads.db` does not exist
- Exit code: 5
- Hint: Initialize workspace with `oya init`

**Error::DatabaseQueryFailed**
- When: SQLite query fails (corruption, permissions, etc.)
- Exit code: 6
- Hint: Check database integrity and file permissions

**Error::DagBuildFailed**
- When: WorkflowDAG construction fails (cycles, missing nodes)
- Exit code: 7
- Hint: Review bead dependencies for circular references

**Error::NoBeadsToExecute**
- When: No beads with status `open` found in database
- Exit code: 8
- Hint: Create beads or update status to `open`

**Error::OrchestratorInitFailed**
- When: BeadOrchestrator initialization fails
- Exit code: 9
- Hint: Check system resources and configuration

**Error::OrchestratorExecutionFailed**
- When: BeadOrchestrator execution fails (timeout, crash)
- Exit code: 10
- Hint: Check logs for bead-specific failures

**Error::InvalidSlotCount**
- When: Configured slot count is not positive
- Exit code: 11
- Hint: Set slots to >= 1 in config

**Error::InvalidTimeout**
- When: Configured timeout is not positive
- Exit code: 12
- Hint: Set timeout to >= 1s in config

## Contract Signatures

```rust
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

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
    pub timeout: Option<Duration>,

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
    // Implementation in functional-rust-generator stage
    todo!()
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
fn build_workflow_dag(
    db: &Database,
) -> Result<orchestrator::dag::WorkflowDAG> {
    // Implementation in functional-rust-generator stage
    todo!()
}

/// Run BeadOrchestrator with given DAG and config
///
/// # Preconditions
/// - DAG has at least one node
/// - Orchestrator initializes successfully
///
/// # Postconditions
/// - Returns Ok(ExecutionResults) on completion
/// - Returns Err(OrchestratorExecutionFailed) on failure
/// - All resources are cleaned up
async fn run_orchestrator(
    dag: orchestrator::dag::WorkflowDAG,
    config: OrchestratorConfig,
) -> Result<Vec<BeadExecutionResult>, StormError> {
    // Implementation in functional-rust-generator stage
    todo!()
}
```

## Non-goals

- **Bead execution logic**: BeadOrchestrator handles bead execution (bd-3a0a.7)
- **IPC bridge setup**: Stage update wiring handled by bd-3a0a.8
- **Workflow DAG algorithms**: DAG implementation exists in `orchestrator::dag`
- **Database migrations**: Assume beads database schema is stable
- **Interactive mode**: Command is non-interactive (use --dry-run for preview)
- **Progress UI**: Progress updates via IPC (bd-3a0a.8), not CLI output
- **Bead creation/modification**: Command only executes existing beads
- **Workspace initialization**: Handled by `oya init` command
- **Configuration file generation**: User creates config manually or via template
