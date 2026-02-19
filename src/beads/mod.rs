//! Quality Gates bead system - highly decomposed workflow components
//!
//! Each bead is a pure function with clear contracts, minimal implementation,
//! and comprehensive tests. Follows the Data → Calculations → Actions hierarchy.
//!
//! # Quality Gates Workflow
//!
//! 1. [`GateSelection`] - Select gates for a stage
//! 2. [`GateExecution`] - Execute a single gate
//! 3. [`GateResultAggregation`] - Aggregate gate results
//! 4. [`QualityGateReport`] - Build quality gate report
//! 5. [`QualityGateDecision`] - Make pass/fail decision
//! 6. [`MoonCommandGeneration`] - Generate moon command from gate
//! 7. [`QualityGatePipeline`] - Orchestrate full quality gate run
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

pub mod gate_aggregation;
pub mod gate_decision;
pub mod gate_execution;
pub mod gate_report;
pub mod gate_selection;
pub mod moon_command;
pub mod quality_gate_pipeline;
