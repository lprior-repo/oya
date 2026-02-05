//! QueueActor - Manages a single queue of ready beads.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::info;

use crate::actors::supervisor::GenericSupervisableActor;
use crate::scheduler::QueueType;

#[derive(Clone, Default)]
pub struct QueueActorDef;

pub struct QueueState {
    pub queue_id: String,
    pub queue_type: QueueType,
    pub ready_beads: Vec<String>,
}

#[derive(Clone)]
pub enum QueueMessage {
    // Add messages as needed
}

impl Actor for QueueActorDef {
    type Msg = QueueMessage;
    type State = QueueState;
    type Arguments = (String, QueueType);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(queue_id = %args.0, "QueueActor starting");
        Ok(QueueState {
            queue_id: args.0,
            queue_type: args.1,
            ready_beads: Vec::new(),
        })
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

impl GenericSupervisableActor for QueueActorDef {

    fn default_args() -> Self::Arguments {

        ("default-queue".to_string(), QueueType::FIFO)

    }

}
