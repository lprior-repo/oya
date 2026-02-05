//! Tier-1 supervision helpers for the orchestrator.

use ractor::ActorRef;

use crate::actors::ActorError;
use crate::actors::queue::QueueActorDef;
use crate::actors::reconciler::ReconcilerActorDef;
use crate::actors::scheduler::SchedulerActorDef;
use crate::actors::storage::StateManagerActorDef;
use crate::actors::supervisor::{
    SupervisorActorDef, SupervisorArguments, SupervisorConfig, SupervisorMessage,
};
use crate::actors::workflow::WorkflowActorDef;

/// Tier-1 supervisor kinds managed by the UniverseSupervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier1SupervisorKind {
    Storage,
    Workflow,
    Queue,
    Reconciler,
}

impl Tier1SupervisorKind {
    /// Return the stable string identifier for this tier-1 supervisor.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::Workflow => "workflow",
            Self::Queue => "queue",
            Self::Reconciler => "reconciler",
        }
    }
}

/// Reference wrapper for a tier-1 supervisor.
#[derive(Debug, Clone)]
pub struct Tier1SupervisorRef<A: ractor::Actor> {
    pub kind: Tier1SupervisorKind,
    pub name: String,
    pub actor: ActorRef<SupervisorMessage<A>>,
}

/// Grouped tier-1 supervisors.
pub struct Tier1Supervisors {
    pub storage: Tier1SupervisorRef<StateManagerActorDef>,
    pub workflow: Tier1SupervisorRef<WorkflowActorDef>,
    pub queue: Tier1SupervisorRef<QueueActorDef>,
    pub reconciler: Tier1SupervisorRef<ReconcilerActorDef>,
}

impl Tier1Supervisors {
    /// Return all supervisors as an ordered list.
    pub fn stop_all(&self, reason: &str) {
        self.storage.actor.stop(Some(reason.to_string()));
        self.workflow.actor.stop(Some(reason.to_string()));
        self.queue.actor.stop(Some(reason.to_string()));
        self.reconciler.actor.stop(Some(reason.to_string()));
    }
}

/// Spawn all tier-1 supervisors with a shared naming prefix.
pub async fn spawn_tier1_supervisors(
    name_prefix: &str,
    config: SupervisorConfig,
) -> Result<Tier1Supervisors, ActorError> {
    let storage = spawn_tier1_supervisor::<StateManagerActorDef>(
        name_prefix,
        Tier1SupervisorKind::Storage,
        &config,
    )
    .await?;

    let workflow = spawn_tier1_supervisor::<WorkflowActorDef>(
        name_prefix,
        Tier1SupervisorKind::Workflow,
        &config,
    )
    .await?;

    let queue =
        spawn_tier1_supervisor::<QueueActorDef>(name_prefix, Tier1SupervisorKind::Queue, &config)
            .await?;

    let reconciler = spawn_tier1_supervisor::<ReconcilerActorDef>(
        name_prefix,
        Tier1SupervisorKind::Reconciler,
        &config,
    )
    .await?;

    Ok(Tier1Supervisors {
        storage,
        workflow,
        queue,
        reconciler,
    })
}

async fn spawn_tier1_supervisor<A>(
    name_prefix: &str,
    kind: Tier1SupervisorKind,
    config: &SupervisorConfig,
) -> Result<Tier1SupervisorRef<A>, ActorError>
where
    A: crate::actors::GenericSupervisableActor + Clone + Default,
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    let name = format!("{}-{}-supervisor", name_prefix, kind.as_str());
    let args = SupervisorArguments::new().with_config(config.clone());

    let (actor, _handle) = ractor::Actor::spawn(
        Some(name.clone()),
        SupervisorActorDef::new(A::default()),
        args,
    )
    .await
    .map_err(|e| ActorError::SpawnFailed(e.to_string()))?;

    Ok(Tier1SupervisorRef { kind, name, actor })
}
