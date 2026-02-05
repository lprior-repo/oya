//! WorkflowActor - Manages a single workflow DAG.

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::{debug, info};

use crate::actors::supervisor::SupervisableActor;
use crate::dag::BeadId;
use crate::scheduler::{WorkflowId, WorkflowState};

pub struct WorkflowActorDef;

pub struct WorkflowStateActor {
    pub workflow_id: WorkflowId,
    pub state: WorkflowState,
}

#[derive(Clone)]
pub enum WorkflowMessage {
    /// Add a bead to the workflow.
    AddBead { bead_id: BeadId },
    /// Mark a bead as completed.
    MarkCompleted { bead_id: BeadId },
    /// Get ready beads.
    GetReadyBeads { reply: RpcReplyPort<Vec<BeadId>> },
    /// Rehydrate state from events.
    Rehydrate { events: Vec<oya_events::BeadEvent> },
}

impl Actor for WorkflowActorDef {
    type Msg = WorkflowMessage;
    type State = WorkflowStateActor;
    type Arguments = WorkflowId;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(workflow_id = %args, "WorkflowActor starting");
        Ok(WorkflowStateActor {
            workflow_id: args.clone(),
            state: WorkflowState::new(args),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            WorkflowMessage::AddBead { bead_id } => {
                let _ = state.state.add_bead(bead_id);
            }
            WorkflowMessage::MarkCompleted { bead_id } => {
                state.state.mark_completed(&bead_id);
            }
            WorkflowMessage::GetReadyBeads { reply } => {
                let ready = state.state.get_ready_beads();
                let _ = reply.send(ready);
            }
            WorkflowMessage::Rehydrate { events } => {
                info!(workflow_id = %state.workflow_id, count = events.len(), "Rehydrating workflow state");
                for event in events {
                    match event {
                        oya_events::BeadEvent::Created { bead_id, .. } => {
                            let _ = state.state.add_bead(bead_id.to_string());
                        }
                        oya_events::BeadEvent::Completed { bead_id, .. } => {
                            state.state.mark_completed(&bead_id.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
}

impl SupervisableActor for WorkflowActorDef {
    fn default_args() -> Self::Arguments {
        "default-workflow".to_string()
    }
}

impl Clone for WorkflowActorDef {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for WorkflowActorDef {
    fn default() -> Self {
        Self
    }
}
