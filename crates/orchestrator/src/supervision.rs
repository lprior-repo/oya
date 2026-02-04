//! Tier-1 supervision helpers for the orchestrator.

use ractor::ActorRef;

use crate::actors::ActorError;
use crate::actors::supervisor::{
    SchedulerSupervisorConfig, SupervisorArguments, SupervisorMessage, spawn_supervisor_with_name,
};

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
pub struct Tier1SupervisorRef {
    pub kind: Tier1SupervisorKind,
    pub name: String,
    pub actor: ActorRef<SupervisorMessage>,
}

/// Grouped tier-1 supervisors.
#[derive(Debug, Clone)]
pub struct Tier1Supervisors {
    pub storage: Tier1SupervisorRef,
    pub workflow: Tier1SupervisorRef,
    pub queue: Tier1SupervisorRef,
    pub reconciler: Tier1SupervisorRef,
}

impl Tier1Supervisors {
    /// Return all supervisors as an ordered list.
    #[must_use]
    pub fn all(&self) -> Vec<&Tier1SupervisorRef> {
        vec![&self.storage, &self.workflow, &self.queue, &self.reconciler]
    }
}

/// Spawn all tier-1 supervisors with a shared naming prefix.
pub async fn spawn_tier1_supervisors(
    name_prefix: &str,
    config: SchedulerSupervisorConfig,
) -> Result<Tier1Supervisors, ActorError> {
    let mut spawned: Vec<Tier1SupervisorRef> = Vec::new();
    let storage = spawn_tier1_supervisor(name_prefix, Tier1SupervisorKind::Storage, &config).await;
    let storage = match storage {
        Ok(supervisor) => {
            spawned.push(supervisor.clone());
            supervisor
        }
        Err(err) => return Err(err),
    };

    let workflow =
        spawn_tier1_supervisor(name_prefix, Tier1SupervisorKind::Workflow, &config).await;
    let workflow = match workflow {
        Ok(supervisor) => {
            spawned.push(supervisor.clone());
            supervisor
        }
        Err(err) => {
            stop_supervisors(&spawned, "tier-1 workflow spawn failed");
            return Err(err);
        }
    };

    let queue = spawn_tier1_supervisor(name_prefix, Tier1SupervisorKind::Queue, &config).await;
    let queue = match queue {
        Ok(supervisor) => {
            spawned.push(supervisor.clone());
            supervisor
        }
        Err(err) => {
            stop_supervisors(&spawned, "tier-1 queue spawn failed");
            return Err(err);
        }
    };

    let reconciler =
        spawn_tier1_supervisor(name_prefix, Tier1SupervisorKind::Reconciler, &config).await;
    let reconciler = match reconciler {
        Ok(supervisor) => supervisor,
        Err(err) => {
            stop_supervisors(&spawned, "tier-1 reconciler spawn failed");
            return Err(err);
        }
    };

    Ok(Tier1Supervisors {
        storage,
        workflow,
        queue,
        reconciler,
    })
}

async fn spawn_tier1_supervisor(
    name_prefix: &str,
    kind: Tier1SupervisorKind,
    config: &SchedulerSupervisorConfig,
) -> Result<Tier1SupervisorRef, ActorError> {
    let name = format!("{}-{}-supervisor", name_prefix, kind.as_str());
    let args = SupervisorArguments::new().with_config(config.clone());
    let actor = spawn_supervisor_with_name(args, &name).await?;

    Ok(Tier1SupervisorRef { kind, name, actor })
}

fn stop_supervisors(supervisors: &[Tier1SupervisorRef], reason: &str) {
    supervisors.iter().for_each(|supervisor| {
        supervisor
            .actor
            .stop(Some(format!("Startup cleanup: {reason}")));
    });
}
