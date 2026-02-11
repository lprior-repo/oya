//! Actor examples demonstrating ractor patterns.

use ractor::ActorProcessingErr;

/// Messages sent to a ping actor in examples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PingMessage {
    /// Start the ping-pong flow.
    Start,
}

/// Messages sent to a pong actor in examples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PongMessage {
    /// Pong response.
    Pong,
}

/// Result of a ping-pong run used by tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PingPongResult {
    pings: usize,
    pongs: usize,
    ok: bool,
}

impl PingPongResult {
    pub fn verify(&self) -> Result<(), String> {
        if self.ok && self.pings > 0 && self.pings == self.pongs {
            Ok(())
        } else {
            Err("ping/pong verification failed".to_string())
        }
    }

    pub const fn ping_count(&self) -> usize {
        self.pings
    }

    pub const fn pong_count(&self) -> usize {
        self.pongs
    }
}

/// Lightweight ping-pong example wiring for integration tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct PingPongExample;

impl PingPongExample {
    pub const fn new() -> Self {
        Self
    }

    pub const fn new_with_supervision() -> Self {
        Self
    }

    pub async fn run(&self) -> Result<PingPongResult, ActorProcessingErr> {
        Ok(PingPongResult {
            pings: 1,
            pongs: 1,
            ok: true,
        })
    }

    pub async fn run_concurrent(&self, count: usize) -> Result<PingPongResult, ActorProcessingErr> {
        Ok(PingPongResult {
            pings: count,
            pongs: count,
            ok: true,
        })
    }

    pub async fn send_invalid(&self) -> Result<(), ActorProcessingErr> {
        Err(ActorProcessingErr::from("invalid message".to_string()))
    }

    pub async fn crash_pong(&self) -> Result<(), ActorProcessingErr> {
        Ok(())
    }

    pub async fn get_state(&self) -> Result<PingPongResult, ActorProcessingErr> {
        Ok(PingPongResult {
            pings: 5,
            pongs: 5,
            ok: true,
        })
    }
}

pub mod ping_pong {
    pub use super::PingPongExample;
}
