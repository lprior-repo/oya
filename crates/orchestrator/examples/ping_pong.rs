//! Ping/Pong Example using ractor Actor trait
//!
//! This example demonstrates:
//! - Basic Actor trait implementation
//! - Message passing between actors
//! - Bidirectional communication
//! - Supervision (actors handle errors gracefully)
//!
//! # Run
//!
//! ```sh
//! cargo run --example ping_pong
//! ```

use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Ping message sent from PingActor to PongActor.
#[derive(Debug, Clone)]
pub struct Ping {
    /// Ping count for tracking.
    pub count: u32,
}

/// Pong message sent from PongActor to PingActor.
#[derive(Debug, Clone)]
pub struct Pong {
    /// Pong count for tracking.
    pub count: u32,
}

/// Messages received by PingActor.
#[derive(Debug)]
pub enum PingMessage {
    /// Receive Pong from PongActor.
    Pong(Pong),
    /// Stop the ping loop.
    Stop,
}

/// Messages received by PongActor.
#[derive(Debug)]
pub enum PongMessage {
    /// Receive Ping from PingActor.
    Ping(Ping, ActorRef<PingMessage>),
}

/// Ping actor definition.
///
/// This actor sends pings and receives pongs.
pub struct PingActorDef;

/// State for PingActor.
#[derive(Debug)]
pub struct PingState {
    /// Current ping count.
    count: u32,
    /// Maximum pings to send before stopping.
    max_pings: u32,
    /// Reference to PongActor for sending pings.
    pong_actor: ActorRef<PongMessage>,
}

impl PingState {
    /// Create new PingState with max pings limit and pong actor reference.
    fn new(max_pings: u32, pong_actor: ActorRef<PongMessage>) -> Self {
        Self {
            count: 0,
            max_pings,
            pong_actor,
        }
    }
}

impl Actor for PingActorDef {
    type Msg = PingMessage;
    type State = PingState;
    type Arguments = (u32, ActorRef<PongMessage>);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        (max_pings, pong_actor): Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("PingActor starting (max pings: {})", max_pings);
        Ok(PingState::new(max_pings, pong_actor))
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PingMessage::Pong(pong) => {
                state.count += 1;
                debug!("Ping received pong #{}", pong.count);

                if state.count < state.max_pings {
                    // Send next ping
                    let next_ping = Ping { count: state.count };
                    let ping_msg = PongMessage::Ping(next_ping, myself.clone());
                    if let Err(e) = state.pong_actor.send_message(ping_msg) {
                        warn!("Failed to send ping: {}", e);
                    }
                } else {
                    info!("Ping sent {} pongs, stopping", state.count);
                    myself.stop(None);
                }
            }
            PingMessage::Stop => {
                info!("PingActor received stop message");
                myself.stop(None);
            }
        }
        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!("PingActor stopped");
        Ok(())
    }
}

/// Pong actor definition.
///
/// This actor receives pings and replies with pongs.
pub struct PongActorDef;

/// State for PongActor.
#[derive(Debug)]
pub struct PongState {
    /// Pongs received count.
    count: u32,
}

impl PongState {
    /// Create new PongState.
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Actor for PongActorDef {
    type Msg = PongMessage;
    type State = PongState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("PongActor starting");
        Ok(PongState::new())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PongMessage::Ping(ping, ping_actor) => {
                state.count += 1;
                debug!("Pong received ping #{}", ping.count);

                // Reply with pong
                let reply = PingMessage::Pong(Pong { count: state.count });
                if let Err(e) = ping_actor.send_message(reply) {
                    warn!("Failed to send pong: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!("PongActor stopped (total pings received: {})", state.count);
        Ok(())
    }
}

/// Spawn ping and pong actors.
///
/// Returns tuple of (ping_actor, pong_actor).
async fn spawn_ping_pong(
    max_pings: u32,
) -> Result<(ActorRef<PingMessage>, ActorRef<PongMessage>), SpawnError> {
    // Spawn PongActor
    let (pong_actor, _) = Actor::spawn(Some("pong_actor".to_string()), PongActorDef, ())
        .await
        .map_err(|e| SpawnError(format!("Failed to spawn pong actor: {}", e)))?;

    // Spawn PingActor
    let (ping_actor, _) = Actor::spawn(
        Some("ping_actor".to_string()),
        PingActorDef,
        (max_pings, pong_actor.clone()),
    )
    .await
    .map_err(|e| SpawnError(format!("Failed to spawn ping actor: {}", e)))?;

    Ok((ping_actor, pong_actor))
}

/// Error for spawn operations.
#[derive(Debug, Clone)]
pub struct SpawnError(pub String);

impl std::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Spawn error: {}", self.0)
    }
}

impl std::error::Error for SpawnError {}

/// Main entry point.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting ping/pong example");

    // Spawn actors
    let (ping_actor, pong_actor) = spawn_ping_pong(5).await?;

    // Send initial ping
    let initial_ping = PongMessage::Ping(Ping { count: 0 }, ping_actor.clone());
    pong_actor.send_message(initial_ping)?;

    // Wait for actors to stop
    tokio::time::sleep(Duration::from_secs(2)).await;

    info!("Ping/pong example complete");

    Ok(())
}
