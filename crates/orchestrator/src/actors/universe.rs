//! UniverseSupervisor - Root of the 3-tier supervision hierarchy.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;
use tracing::info;

use crate::actors::supervisor::SupervisorConfig;
use crate::shutdown::ShutdownCoordinator;
use crate::supervision::{Tier1Supervisors, spawn_tier1_supervisors};

pub struct UniverseSupervisorDef;

pub struct UniverseState {
    pub tier1: Tier1Supervisors,
}

pub enum UniverseMessage {
    Shutdown,
}

pub struct UniverseArguments {
    pub name_prefix: String,
    pub config: SupervisorConfig,
    pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
}

impl Actor for UniverseSupervisorDef {
    type Msg = UniverseMessage;
    type State = UniverseState;
    type Arguments = UniverseArguments;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("UniverseSupervisor starting");

        let tier1 = spawn_tier1_supervisors(&args.name_prefix, args.config)
            .await
            .map_err(|e| ActorProcessingErr::from(e.to_string()))?;

        // Subscribe to shutdown
        if let Some(coordinator) = args.shutdown_coordinator {
            let myself_clone = myself.clone();
            let mut rx = coordinator.subscribe();
            tokio::spawn(async move {
                if rx.recv().await.is_ok() {
                    let _ = myself_clone.send_message(UniverseMessage::Shutdown);
                }
            });
        }

        Ok(UniverseState { tier1 })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            UniverseMessage::Shutdown => {
                info!("UniverseSupervisor shutting down");
                // Gracefully handle shutdown errors - log but don't fail
                let _ = state.tier1.stop_all("Universe shutdown").await;
                myself.stop(None);
            }
        }
        Ok(())
    }
}
