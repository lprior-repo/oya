//! Planner Agent.
//!
//! Coordinates contract workflow between Test Writers and Implementers.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use ractor::{Actor, ActorProcessingErr};

use crate::swarm::messages::PlannerMessage;

/// Planner Actor - coordinates contract workflow.
#[derive(Debug, Clone, Default)]
pub struct PlannerActor;

/// Planner Actor state.
#[derive(Debug, Clone)]
pub struct PlannerState {
    /// Actor ID.
    pub id: String,
}

/// Planner Actor arguments.
#[derive(Debug, Clone)]
pub struct PlannerArgs {
    /// Actor ID.
    pub id: String,
}

impl Actor for PlannerActor {
    type Msg = PlannerMessage;
    type State = PlannerState;
    type Arguments = PlannerArgs;

    async fn pre_start(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("PlannerActor {} starting", args.id);
        Ok(PlannerState { id: args.id })
    }

    async fn handle(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PlannerMessage::ReviewRequirements { bead_id } => {
                // TODO: Review bead requirements
                // TODO: Use rust-contract to design contracts
                // TODO: Apply Martin Fowler test philosophy
                tracing::info!("Planner {} reviewing requirements for bead {}", state.id, bead_id);
            }
            PlannerMessage::CoordinateContract { bead_id } => {
                // TODO: Ensure contract is complete before implementation
                // TODO: Coordinate between Test Writer and Implementer
                tracing::info!("Planner {} coordinating contract for bead {}", state.id, bead_id);
            }
        }
        Ok(())
    }
}
