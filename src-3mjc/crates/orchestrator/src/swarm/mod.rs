//! Swarm module for 13-agent continuous assembly line.
//!
//! This module orchestrates parallel agents to complete beads using
//! contract-first development with continuous-deployment principles.
//!
//! # Architecture
//!
//! - **SwarmOrchestratorActor**: Top-level coordinator, handles CLI commands
//! - **SwarmSupervisor**: Supervises 13 agents (4 Test Writers, 4 Implementers, 4 Reviewers, 1 Planner)
//! - **TestWriterAgents**: Write test contracts BEFORE implementation using rust-contract
//! - **ImplementerAgents**: Implement following contracts using continuous-deployment
//! - **ReviewerAgents**: QA test with red-queen and land beads
//! - **PlannerAgent**: Coordinates contract workflow
//!
//! # Continuous-Deployment Principles (Absolute Law)
//!
//! 1. **Velocity** - Fast, small batches through the pipeline
//! 2. **One-Piece Flow** - Single bead at a time per agent
//! 3. **Moon Gates** - All quality gates must pass
//! 4. **Functional Rust** - Zero unwrap/expect/panic enforced
//! 5. **TDD15** - Test-driven development workflow
//! 6. **Shift-Left** - Quality enforced early

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod agents;
pub mod config;
pub mod messages;
mod orchestrator_actor;
mod supervisor_actor;

pub use config::SwarmConfig;
pub use messages::{
    BeadPhase, BeadResult, BeadWork, ImplementerMessage, PlannerMessage, ReviewerMessage,
    SwarmAgentType, SwarmCommand, SwarmStatus, SwarmSupervisorMessage, TestWriterMessage,
};
pub use orchestrator_actor::SwarmOrchestratorActor;
pub use supervisor_actor::SwarmSupervisor;
