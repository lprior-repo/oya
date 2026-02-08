//! Implementer Agent.
//!
//! Implements beads following contracts using continuous-deployment principles.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use ractor::{Actor, ActorProcessingErr};

use crate::swarm::messages::ImplementerMessage;

/// Implementer Actor - implements following continuous-deployment principles.
#[derive(Debug, Clone, Default)]
pub struct ImplementerActor;

/// Implementer Actor state.
#[derive(Debug, Clone)]
pub struct ImplementerState {
    /// Actor ID.
    pub id: String,
}

/// Implementer Actor arguments.
#[derive(Debug, Clone)]
pub struct ImplementerArgs {
    /// Actor ID.
    pub id: String,
}

impl Actor for ImplementerActor {
    type Msg = ImplementerMessage;
    type State = ImplementerState;
    type Arguments = ImplementerArgs;

    async fn pre_start(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("ImplementerActor {} starting", args.id);
        Ok(ImplementerState { id: args.id })
    }

    async fn handle(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ImplementerMessage::GetNextBead { reply } => {
                // TODO: Poll database for ready_for_implementation beads
                // TODO: Return BeadWork or None
                let _ = reply.send(None);
                tracing::debug!("Implementer {} got GetNextBead", state.id);
            }
            ImplementerMessage::ImplementBead { bead_id, workspace } => {
                // TODO: Spawn zjj workspace
                // TODO: Follow continuous-deployment workflow (TDD15, functional-rust)
                // TODO: Run moon ci gates (must pass)
                tracing::info!(
                    "Implementer {} implementing bead {} in workspace {}",
                    state.id,
                    bead_id,
                    workspace
                );
            }
            ImplementerMessage::SubmitForReview { bead_id, test_results } => {
                // TODO: Mark bead as ready_for_review in database
                tracing::info!(
                    "Implementer {} submitting bead {} for review",
                    state.id,
                    bead_id
                );
            }
        }
        Ok(())
    }
}
