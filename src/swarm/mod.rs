//! Swarm module for 13-agent continuous assembly line.
//!
//! This module orchestrates parallel agents to complete beads using
//! contract-first development with continuous-deployment principles.
//!
//! # Architecture
//!
//! - **Orchestrator**: Spawns and supervises 13 agents (4 Test Writers, 4 Implementers, 4 Reviewers, 1 Planner)
//! - **Test Writers**: Write test contracts using rust-contract BEFORE implementation
//! - **Planner**: Coordinates contract workflow with Martin Fowler test philosophy
//! - **Implementers**: Implement following contracts using continuous-deployment principles
//! - **Reviewers**: QA test with red-queen and land beads
//!
//! # Continuous-Deployment Principles (Absolute Law)
//!
//! 1. **Velocity** - Fast, small batches through the pipeline
//! 2. **One-Piece Flow** - Single bead at a time per agent
//! 3. **Moon Gates** - All quality gates must pass
//! 4. **Functional Rust** - Zero unwrap/expect/panic enforced
//! 5. **TDD15** - Test-driven development workflow
//! 6. **Shift-Left** - Quality enforced early
//!
//! # File-Based Handoff
//!
//! Agents communicate via files in /tmp/:
//! - `/tmp/bead-contracts-<id>.json` - Test contracts from Test Writers
//! - `/tmp/bead-ready-to-implement-<id>.json` - Ready for implementation
//! - `/tmp/bead-implementation-complete-<id>.json` - Implementation done
//! - `/tmp/bead-ready-review-<id>.json` - Ready for review
//! - `/tmp/bead-complete-<id>.json` - Bead landed successfully

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

mod config;
mod contract_processor;
mod error;
mod handoff;
mod messages;
mod work_queue;

#[cfg(test)]
mod contract_processor_test;

pub use config::SwarmConfig;
pub use contract_processor::{ContractProcessor, ContractProcessorError};
pub use error::{BeadWorkState, SwarmError, SwarmResult};
pub use handoff::{HandoffFile, HandoffState};
pub use messages::{SwarmMessage, SwarmStatus};
pub use work_queue::{BeadWorkItem, WorkQueue};
