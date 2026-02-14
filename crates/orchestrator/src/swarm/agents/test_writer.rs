//! Test Writer Agent.
//!
//! Writes test contracts BEFORE implementation using rust-contract skill.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use ractor::{Actor, ActorProcessingErr};

use crate::swarm::messages::TestWriterMessage;

/// Test Writer Actor - writes test contracts using rust-contract.
#[derive(Debug, Clone, Default)]
pub struct TestWriterActor;

/// Test Writer Actor state.
#[derive(Debug, Clone)]
pub struct TestWriterState {
    /// Actor ID.
    pub id: String,
}

/// Test Writer Actor arguments.
#[derive(Debug, Clone)]
pub struct TestWriterArgs {
    /// Actor ID.
    pub id: String,
}

impl Actor for TestWriterActor {
    type Msg = TestWriterMessage;
    type State = TestWriterState;
    type Arguments = TestWriterArgs;

    async fn pre_start(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("TestWriterActor {} starting", args.id);
        Ok(TestWriterState { id: args.id })
    }

    async fn handle(
        &self,
        _myself: ractor::ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
       match message {
             TestWriterMessage::GetNextBead { reply } => {
                 let _ = reply.send(None);
                 tracing::debug!("TestWriter {} got GetNextBead", state.id);
             }
             TestWriterMessage::WriteContract { bead_id, contract } => {
                 tracing::info!(
                     "TestWriter {} writing contract for bead {}",
                     state.id,
                     bead_id
                 );
             }
             TestWriterMessage::ReadyForImplementation { bead_id } => {
                 tracing::info!(
                     "TestWriter {} marking bead {} ready for implementation",
                     state.id,
                     bead_id
                 );
             }
         }
        Ok(())
    }
}
