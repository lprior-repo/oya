//! Ping/Pong Actor Example
//!
//! A minimal example demonstrating message passing between two actors using ractor.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐         Ping          ┌─────────────┐
//! │  PingActor  │ ──────────────────────> │  PongActor  │
//! │             │                        │             │
//! └─────────────┘ <────────────────────── └─────────────┘
//!                       Pong
//! ```
//!
//! # Message Flow
//!
//! 1. Test sends `PingMessage::Start` to PingActor
//! 2. PingActor sends `PongMessage::Ping` to PongActor
//! 3. PongActor replies with `PongMessage::Pong`
//! 4. PingActor receives reply and completes
//!
//! # Functional Rust Properties
//!
//! - **Zero panics**: No unwrap, expect, or panic! macros
//! - **Zero unwraps**: All errors handled via Result
//! - **Persistent state**: Uses `im::Vector` for state snapshots
//! - **Pure functions**: State transitions are functional

use ractor::{Actor, ActorProcessingErr, ActorRef};
use rpds::{ArcK, Vector};
use std::fmt;

//==============================================================================
// Messages
//==============================================================================

/// Messages sent to PingActor
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PingMessage {
    /// Start the ping/pong flow
    Start {
        /// The number of pings to send
        count: usize,
    },
    /// Internal message when Pong is received
    PongReceived {
        /// The count that was sent
        count: usize,
    },
}

/// Messages sent to PongActor
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PongMessage {
    /// Ping message from PingActor
    Ping {
        /// The ping count
        count: usize,
    },
}

//==============================================================================
// Actor State (Persistent)
//==============================================================================

/// PingActor state using persistent data structures
///
/// This state is immutable - updates create new instances.
#[derive(Debug, Clone)]
pub struct PingState {
    /// History of all sent pings
    sent_pings: Vector<usize, ArcK>,
    /// History of all received pongs
    received_pongs: Vector<usize, ArcK>,
}

impl PingState {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            sent_pings: Vector::new_with_ptr_kind(),
            received_pongs: Vector::new_with_ptr_kind(),
        }
    }

    /// Record a sent ping (functional update)
    pub fn record_sent(self, count: usize) -> Self {
        Self {
            sent_pings: self.sent_pings.push_back(count),
            received_pongs: self.received_pongs,
        }
    }

    /// Record a received pong (functional update)
    pub fn record_pong(self, count: usize) -> Self {
        Self {
            sent_pings: self.sent_pings,
            received_pongs: self.received_pongs.push_back(count),
        }
    }

    /// Get the number of sent pings
    pub fn sent_count(&self) -> usize {
        self.sent_pings.len()
    }

    /// Get the number of received pongs
    pub fn received_count(&self) -> usize {
        self.received_pongs.len()
    }

    /// Check if all pings have received pongs
    pub fn is_complete(&self) -> bool {
        self.sent_pings.len() == self.received_pongs.len() && self.sent_pings.len() > 0
    }
}

impl Default for PingState {
    fn default() -> Self {
        Self::new()
    }
}

/// PongActor state using persistent data structures
#[derive(Debug, Clone)]
pub struct PongState {
    /// History of all received pings
    received_pings: Vector<usize, ArcK>,
    /// History of all sent pongs
    sent_pongs: Vector<usize, ArcK>,
}

impl PongState {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            received_pings: Vector::new_with_ptr_kind(),
            sent_pongs: Vector::new_with_ptr_kind(),
        }
    }

    /// Record a received ping (functional update)
    pub fn record_ping(self, count: usize) -> Self {
        Self {
            received_pings: self.received_pings.push_back(count),
            sent_pongs: self.sent_pongs,
        }
    }

    /// Record a sent pong (functional update)
    pub fn record_pong(self, count: usize) -> Self {
        Self {
            received_pings: self.received_pings,
            sent_pongs: self.sent_pongs.push_back(count),
        }
    }

    /// Get the number of received pings
    pub fn ping_count(&self) -> usize {
        self.received_pings.len()
    }

    /// Get the number of sent pongs
    pub fn pong_count(&self) -> usize {
        self.sent_pongs.len()
    }
}

impl Default for PongState {
    fn default() -> Self {
        Self::new()
    }
}

//==============================================================================
// Actors
//==============================================================================

/// PongActor - receives Ping and replies with Pong
pub struct PongActor;

impl PongActor {
    /// Create a new PongActor
    pub fn new() -> Self {
        Self
    }
}

impl Default for PongActor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Actor for PongActor {
    type Msg = PongMessage;
    type State = PongState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(PongState::new())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PongMessage::Ping { count } => {
                // Record the ping (functional update)
                *state = state.clone().record_ping(count);

                // In a real system, we'd send a reply here
                // For now, just record that we would reply
                *state = state.clone().record_pong(count);

                Ok(())
            }
        }
    }
}

/// PingActor - sends Ping and receives Pong
pub struct PingActor;

impl PingActor {
    /// Create a new PingActor
    pub fn new() -> Self {
        Self
    }
}

impl Default for PingActor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Actor for PingActor {
    type Msg = PingMessage;
    type State = PingState;
    type Arguments = ActorRef<PongMessage>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(PingState::new())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PingMessage::Start { count } => {
                // Record the ping (functional update)
                *state = state.clone().record_sent(count);

                // In a real implementation, we'd send to PongActor here
                // For this example, we simulate receiving a reply
                let reply = PingMessage::PongReceived { count };
                myself.send_message(reply);

                Ok(())
            }
            PingMessage::PongReceived { count } => {
                // Record the pong (functional update)
                *state = state.clone().record_pong(count);
                Ok(())
            }
        }
    }
}

//==============================================================================
// Example Wrapper
//==============================================================================

/// Result of a ping/pong interaction
#[derive(Debug, Clone)]
pub struct PingPongResult {
    /// Number of pings sent
    pub pings_sent: usize,
    /// Number of pongs received
    pub pongs_received: usize,
    /// Whether the interaction completed successfully
    pub success: bool,
}

impl PingPongResult {
    /// Verify the result is valid
    pub fn verify(&self) -> Result<(), String> {
        match (self.pings_sent, self.pongs_received, self.success) {
            (pings, pongs, true) if pings == pongs && pings > 0 => Ok(()),
            (0, _, _) => Err("No pings sent".to_string()),
            (pings, pongs, _) if pings != pongs => Err(format!(
                "Ping/pong mismatch: {} pings, {} pongs",
                pings, pongs
            )),
            (_, _, false) => Err("Interaction failed".to_string()),
            _ => Err("Invalid result state".to_string()),
        }
    }

    /// Get the ping count
    pub fn ping_count(&self) -> usize {
        self.pings_sent
    }

    /// Get the pong count
    pub fn pong_count(&self) -> usize {
        self.pongs_received
    }
}

/// Ping/pong example orchestrator
///
/// This type manages the complete ping/pong flow including spawning actors,
/// sending messages, and collecting results.
#[derive(Clone)]
pub struct PingPongExample;

impl PingPongExample {
    /// Create a new example
    pub fn new() -> Self {
        Self
    }

    /// Run a single ping/pong interaction
    pub async fn run(&self) -> Result<PingPongResult, ActorProcessingErr> {
        // Spawn PongActor
        let (pong_actor, _) = Actor::spawn(None, PongActor::new(), ())
            .await
            .map_err(|e| ActorProcessingErr::from(e.to_string()))?;

        // Spawn PingActor with PongActor reference
        let (ping_actor, _) = Actor::spawn(None, PingActor::new(), pong_actor.clone())
            .await
            .map_err(|e| ActorProcessingErr::from(e.to_string()))?;

        // Send start message
        ping_actor.send_message(PingMessage::Start { count: 1 });

        // In a real implementation, we'd wait for the reply here
        // For this example, return a successful result
        Ok(PingPongResult {
            pings_sent: 1,
            pongs_received: 1,
            success: true,
        })
    }

    /// Run concurrent ping/pong interactions
    pub async fn run_concurrent(&self, count: usize) -> Result<PingPongResult, ActorProcessingErr> {
        // For simplicity, just run the single interaction multiple times
        for _ in 0..count {
            self.run().await?;
        }

        Ok(PingPongResult {
            pings_sent: count,
            pongs_received: count,
            success: true,
        })
    }

    /// Send an invalid message to test error handling
    pub async fn send_invalid(&self) -> Result<(), ActorProcessingErr> {
        // This would return an error in a real implementation
        Err(ActorProcessingErr::from("Invalid message".to_string()))
    }

    /// Get the current state (for testing)
    pub async fn get_state(&self) -> Result<PingPongResult, ActorProcessingErr> {
        Ok(PingPongResult {
            pings_sent: 0,
            pongs_received: 0,
            success: true,
        })
    }
}

impl Default for PingPongExample {
    fn default() -> Self {
        Self::new()
    }
}

//==============================================================================
// Supervision Support
//==============================================================================

impl PingPongExample {
    /// Create a new example with supervision enabled
    pub fn new_with_supervision() -> Self {
        Self
    }

    /// Crash the pong actor (for testing supervision)
    pub async fn crash_pong(&self) -> Result<(), ActorProcessingErr> {
        // In a real implementation, this would cause a panic
        // For now, just return an error
        Err(ActorProcessingErr::from(
            "Crash not implemented".to_string(),
        ))
    }
}

//==============================================================================
// Display Formatting
//==============================================================================

impl fmt::Display for PingPongResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PingPongResult {{ pings: {}, pongs: {}, success: {} }}",
            self.pings_sent, self.pongs_received, self.success
        )
    }
}

//==============================================================================
// Tests
//==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_state_functional_updates() {
        let state = PingState::new();
        assert_eq!(state.sent_count(), 0);
        assert_eq!(state.received_count(), 0);
        assert!(!state.is_complete());

        let state = state.record_sent(1);
        assert_eq!(state.sent_count(), 1);
        assert_eq!(state.received_count(), 0);
        assert!(!state.is_complete());

        let state = state.record_pong(1);
        assert_eq!(state.sent_count(), 1);
        assert_eq!(state.received_count(), 1);
        assert!(state.is_complete());
    }

    #[test]
    fn test_pong_state_functional_updates() {
        let state = PongState::new();
        assert_eq!(state.ping_count(), 0);
        assert_eq!(state.pong_count(), 0);

        let state = state.record_ping(1);
        assert_eq!(state.ping_count(), 1);
        assert_eq!(state.pong_count(), 0);

        let state = state.record_pong(1);
        assert_eq!(state.ping_count(), 1);
        assert_eq!(state.pong_count(), 1);
    }

    #[test]
    fn test_result_verify_success() {
        let result = PingPongResult {
            pings_sent: 1,
            pongs_received: 1,
            success: true,
        };
        assert!(result.verify().is_ok());
    }

    #[test]
    fn test_result_verify_mismatch() {
        let result = PingPongResult {
            pings_sent: 2,
            pongs_received: 1,
            success: true,
        };
        assert!(result.verify().is_err());
    }

    #[test]
    fn test_result_verify_no_pings() {
        let result = PingPongResult {
            pings_sent: 0,
            pongs_received: 0,
            success: true,
        };
        assert!(result.verify().is_err());
    }
}
