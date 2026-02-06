//! Storage actors for durable state management.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::info;

use crate::actors::supervisor::GenericSupervisableActor;

#[derive(Clone, Default)]
pub struct StateManagerActorDef;

pub struct StateManagerState {
    // Durable state management
}

#[derive(Clone)]
pub enum StateManagerMessage {
    // Virtual object messages
}

impl Actor for StateManagerActorDef {
    type Msg = StateManagerMessage;
    type State = StateManagerState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("StateManagerActor starting");
        Ok(StateManagerState {})
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

impl GenericSupervisableActor for StateManagerActorDef {
    fn default_args() -> Self::Arguments {
        Self::Arguments::default()
    }
}

#[derive(Clone, Default)]
pub struct EventStoreActorDef;

pub struct EventStoreState {
    // Event persistence
}

#[derive(Clone)]
pub enum EventStoreMessage {
    // Event storage messages
}

impl Actor for EventStoreActorDef {
    type Msg = EventStoreMessage;
    type State = EventStoreState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("EventStoreActor starting");
        Ok(EventStoreState {})
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

impl GenericSupervisableActor for EventStoreActorDef {
    fn default_args() -> Self::Arguments {
        Self::Arguments::default()
    }
}
