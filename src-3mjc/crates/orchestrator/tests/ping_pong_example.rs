//! Ping/Pong Actor Example
//!
//! This test demonstrates a simple ping/pong actor system using ractor.
//! The PingActor sends a Ping message and expects a Pong reply.
//! The PongActor receives Ping and replies with Pong.
//!
//! # Functional Rust Requirements
//!
//! - Zero panics: No unwrap, expect, or panic!
//! - Zero unwraps: Use map, and_then, or ? for error handling
//! - Pure functions where possible
//! - Persistent state using im or rpds

// Only compile these tests when the "examples" feature is enabled
#![cfg(feature = "examples")]

use orchestrator::actors::examples::{PingMessage, PongMessage, ping_pong::PingPongExample};
use ractor::{Actor, ActorProcessingErr, call};

/// Test the complete ping/pong flow
///
/// # Behavior
/// 1. Spawn PongActor
/// 2. Spawn PingActor with PongActor reference
/// 3. Send Ping to PingActor
/// 4. PingActor forwards to PongActor
/// 5. PongActor replies with Pong
/// 6. PingActor receives Pong and completes
///
/// # Success Criteria
/// - All actors spawn successfully
/// - Ping message is delivered
/// - Pong reply is received
/// - No panics or unwraps
#[tokio::test]
async fn test_ping_pong_flow() -> Result<(), ActorProcessingErr> {
    // Create the ping/pong example
    let example = PingPongExample::new();

    // Run the ping/pong flow
    let result = example.run().await?;

    // Verify the result
    result
        .verify()
        .map_err(|e| ActorProcessingErr::from(e.to_string()))?;

    Ok(())
}

/// Test that PongActor handles multiple concurrent pings
///
/// # Behavior
/// - Send 10 concurrent Ping messages
/// - Each Ping should receive a Pong reply
/// - All replies should be received in order
#[tokio::test]
async fn test_concurrent_pings() -> Result<(), ActorProcessingErr> {
    let example = PingPongExample::new();
    let result = example.run_concurrent(10).await?;

    result
        .verify()
        .map_err(|e| ActorProcessingErr::from(e.to_string()))?;

    Ok(())
}

/// Test that actors handle errors gracefully
///
/// # Behavior
/// - Send an invalid message
/// - Actor should return an error, not panic
/// - Error should be propagated via Result
#[tokio::test]
async fn test_error_handling() -> Result<(), ActorProcessingErr> {
    let example = PingPongExample::new();

    // Attempt to send invalid message
    let result = example.send_invalid().await;

    // Verify error is returned, not panic
    assert!(result.is_err(), "Expected error for invalid message");

    Ok(())
}

/// Test that supervision works correctly
///
/// # Behavior
/// - Panic the PongActor
/// - Supervisor should restart it
/// - Ping/pong flow should continue working
#[tokio::test]
async fn test_supervision() -> Result<(), ActorProcessingErr> {
    let example = PingPongExample::new_with_supervision();

    // Run normal flow
    let result = example.run().await?;

    // Cause panic in PongActor
    example.crash_pong().await?;

    // Run flow again - should work after restart
    let result_after_crash = example.run().await?;

    result_after_crash
        .verify()
        .map_err(|e| ActorProcessingErr::from(e.to_string()))?;

    Ok(())
}

/// Test persistent state updates
///
/// # Behavior
/// - Actors maintain state using im::Vector
/// - Each message updates state immutably
/// - State can be queried and verified
#[tokio::test]
async fn test_persistent_state() -> Result<(), ActorProcessingErr> {
    let example = PingPongExample::new();

    // Send multiple pings
    for _ in 0..5 {
        example.run().await?;
    }

    // Query state
    let state = example.get_state().await?;

    // Verify state contains all 5 pings
    assert_eq!(state.ping_count(), 5, "Expected 5 pings in state");
    assert_eq!(state.pong_count(), 5, "Expected 5 pongs in state");

    Ok(())
}
