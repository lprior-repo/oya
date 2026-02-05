//! ReconcilerActor - Manages the reconciliation loop.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::info;

use crate::actors::supervisor::SupervisableActor;

pub struct ReconcilerActorDef;

pub struct ReconcilerState {
    // Reconciliation state
}

pub enum ReconcilerMessage {
    Tick,
}

impl Actor for ReconcilerActorDef {
    type Msg = ReconcilerMessage;
    type State = ReconcilerState;
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("ReconcilerActor starting");

        // Spawn the tick loop
        let myself_clone = myself.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                if myself_clone.send_message(ReconcilerMessage::Tick).is_err() {
                    break;
                }
            }
        });

        Ok(ReconcilerState {})
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        _message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        Ok(())
    }
}

impl SupervisableActor for ReconcilerActorDef {
    fn default_args() -> Self::Arguments {
        ()
    }
}

impl Clone for ReconcilerActorDef {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for ReconcilerActorDef {
    fn default() -> Self {
        Self
    }
}
