//! WorkflowActor - Manages a single workflow DAG.

use im::Vector;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::info;

use crate::actors::supervisor::GenericSupervisableActor;
use crate::dag::BeadId;
use crate::scheduler::{WorkflowId, WorkflowState};

#[derive(Clone, Default)]
pub struct WorkflowActorDef;

pub struct WorkflowStateActor {
    pub workflow_id: WorkflowId,
    pub state: WorkflowState,
}

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

impl Clone for WorkflowMessage {
    fn clone(&self) -> Self {
        match self {
            Self::AddBead { bead_id } => Self::AddBead {
                bead_id: bead_id.clone(),
            },
            Self::MarkCompleted { bead_id } => Self::MarkCompleted {
                bead_id: bead_id.clone(),
            },
            Self::Rehydrate { events } => Self::Rehydrate {
                events: events.clone(),
            },
            Self::GetReadyBeads { .. } => panic!("WorkflowMessage::GetReadyBeads cannot be cloned"),
        }
    }
}

/// Effects produced by the functional core of the WorkflowActor.
pub enum WorkflowEffect {
    /// Send a reply to an RPC caller.
    ReplyReadyBeads {
        reply: RpcReplyPort<Vec<BeadId>>,
        beads: Vec<BeadId>,
    },
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
        let (next_state, effects) = core::handle(state.state.clone(), message);
        state.state = next_state;

        for effect in effects {
            match effect {
                WorkflowEffect::ReplyReadyBeads { reply, beads } => {
                    let _ = reply.send(beads);
                }
            }
        }

        Ok(())
    }
}

impl GenericSupervisableActor for WorkflowActorDef {
    fn default_args() -> Self::Arguments {
        "default-workflow".to_string()
    }
}

/// Functional core for WorkflowActor.
mod core {
    use super::*;

    /// Pure function to handle messages and return next state + effects.
    pub fn handle(
        state: WorkflowState,
        msg: WorkflowMessage,
    ) -> (WorkflowState, Vector<WorkflowEffect>) {
        let mut next_state = state;
        let mut effects = Vector::new();

        match msg {
            WorkflowMessage::AddBead { bead_id } => {
                let _ = next_state.add_bead(bead_id);
            }
            WorkflowMessage::MarkCompleted { bead_id } => {
                next_state.mark_completed(&bead_id);
            }
            WorkflowMessage::GetReadyBeads { reply } => {
                let ready = next_state.get_ready_beads();
                effects.push_back(WorkflowEffect::ReplyReadyBeads {
                    reply,
                    beads: ready,
                });
            }
            WorkflowMessage::Rehydrate { events } => {
                for event in events {
                    match event {
                        oya_events::BeadEvent::Created { bead_id, .. } => {
                            let _ = next_state.add_bead(bead_id.to_string());
                        }
                        oya_events::BeadEvent::Completed { bead_id, .. } => {
                            next_state.mark_completed(&bead_id.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        (next_state, effects)
    }
}
