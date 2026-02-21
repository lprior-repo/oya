//! Quality Gates bead system - highly decomposed workflow components
//!
//! Each bead is a pure function with clear contracts, minimal implementation,
//! and comprehensive tests. Follows the Data → Calculations → Actions hierarchy.
//!
//! # Quality Gates Workflow
//!
//! 1. [`gate_selection::select_gates`] - Select gates for a stage
//! 2. [`gate_execution::execute_gate`] - Execute a single gate
//! 3. [`gate_aggregation::aggregate_gate_results`] - Aggregate gate results
//! 4. [`gate_report::build_gate_report`] - Build quality gate report
//! 5. [`gate_decision::make_gate_decision`] - Make pass/fail decision
//! 6. [`moon_command::generate_moon_command`] - Generate moon command from gate
//! 7. [`quality_gate_pipeline::run_quality_gate_pipeline`] - Orchestrate full quality gate run
//!
//! # Design Contract
//!
//! Each bead follows the functional Rust pattern:
//! - Pure functions (no I/O in core)
//! - `thiserror` for domain errors
//! - `im::Vector` for collections
//! - Zero `unwrap`, `expect`, `panic`
//! - Clear contracts with input/output types

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

pub mod cue_artifact;
pub mod gate_aggregation;
pub mod gate_decision;
pub mod gate_execution;
pub mod gate_report;
pub mod gate_selection;
pub mod moon_command;
pub mod quality_gate_pipeline;
