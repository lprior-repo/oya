//! Storage actors for durable state management.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::info;

use crate::actors::supervisor::SupervisableActor;

pub struct StateManagerActorDef;

pub struct StateManagerState {
    // Durable state management
}

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

impl SupervisableActor for StateManagerActorDef {
    fn default_args() -> Self::Arguments {
        ()
    }
}

impl Clone for StateManagerActorDef {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for StateManagerActorDef {
    fn default() -> Self {
        Self
    }
}

pub struct EventStoreActorDef;

pub struct EventStoreState {
    // Event persistence
}

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

impl SupervisableActor for EventStoreActorDef {
    fn default_args() -> Self::Arguments {
        ()
    }
}

impl Clone for EventStoreActorDef {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for EventStoreActorDef {
    fn default() -> Self {
        Self
    }
}
