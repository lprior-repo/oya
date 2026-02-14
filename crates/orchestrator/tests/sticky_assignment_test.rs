#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Sticky mode assignment tests for AgentPool
//!
//! Tests verify:
//! - Same bead assigned to same worker (sticky hit)
//! - Fallback when previous worker is Claimed/Working
//! - Fallback when previous worker is Dead/Unhealthy

use orchestrator::agent_swarm::{AgentHandle, AgentPool, PoolConfig};

#[tokio::test]
async fn test_sticky_assign_same_bead_to_same_worker() -> Result<(), Box<dyn std::error::Error>> {
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent_a = AgentHandle::new("agent-a");
    let agent_b = AgentHandle::new("agent-b");

    pool.register_agent(agent_a).await?;
    pool.register_agent(agent_b).await?;

    let bead_id = "bead-sticky-1";

    // First assignment - should assign to agent-a (first idle, sorted by ID)
    let first_assignment = pool
        .assign_bead(bead_id)
        .await
        .map_err(|e| format!("First assignment failed: {e:?}"))?;
    assert_eq!(first_assignment, "agent-a", "First assignment to agent-a");

    // Complete the bead (agent becomes idle again)
    pool.complete_bead(&first_assignment).await?;

    // Second assignment of same bead - should assign to agent-a (sticky hit)
    let second_assignment = pool
        .assign_bead(bead_id)
        .await
        .map_err(|e| format!("Second assignment failed: {e:?}"))?;
    assert_eq!(
        second_assignment, "agent-a",
        "Sticky assignment to same worker"
    );
    Ok(())
}

#[tokio::test]
async fn test_sticky_fallback_when_previous_worker_busy() -> Result<(), Box<dyn std::error::Error>>
{
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent_a = AgentHandle::new("agent-a");
    let agent_b = AgentHandle::new("agent-b");

    pool.register_agent(agent_a.clone()).await?;
    pool.register_agent(agent_b).await?;

    let bead_id = "bead-fallback-1";

    // First assignment to agent-a
    let first_assignment = pool
        .assign_bead(bead_id)
        .await
        .map_err(|e| format!("First assignment failed: {e:?}"))?;
    assert_eq!(first_assignment, "agent-a");

    // Agent-a is still working with bead-fallback-1
    // Verify agent-a is working
    let stats = pool.stats().await;
    assert_eq!(stats.working, 1, "Agent-a should be working");

    // Attempting to assign to busy agent should fail
    let busy_result = pool.assign_bead_to_agent("other-bead-1", "agent-a").await;
    assert!(busy_result.is_err(), "Assigning to busy agent should fail");

    // Re-assign same bead - should fallback to agent-b (agent-a is busy)
    let second_assignment = pool
        .assign_bead(bead_id)
        .await
        .map_err(|e| format!("Second assignment failed: {e:?}"))?;
    assert_eq!(
        second_assignment, "agent-b",
        "Fallback to agent-b when agent-a busy"
    );
    Ok(())
}

#[tokio::test]
async fn test_sticky_fallback_when_previous_worker_unhealthy(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent_a = AgentHandle::new("agent-a");
    let agent_b = AgentHandle::new("agent-b");

    pool.register_agent(agent_a.clone()).await?;
    pool.register_agent(agent_b).await?;

    let bead_id = "bead-unhealthy-1";

    // First assignment to agent-a
    let first_assignment = pool
        .assign_bead(bead_id)
        .await
        .map_err(|e| format!("First assignment failed: {e:?}"))?;
    assert_eq!(first_assignment, "agent-a");

    // Complete the bead
    pool.complete_bead("agent-a").await?;

    // Mark agent-a as unhealthy
    pool.shutdown_agent("agent-a").await?;

    // Wait for shutdown to take effect
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Re-assign same bead - should fallback to agent-b (agent-a unhealthy)
    let second_assignment = pool
        .assign_bead(bead_id)
        .await
        .map_err(|e| format!("Second assignment failed: {e:?}"))?;
    assert_eq!(
        second_assignment, "agent-b",
        "Fallback to agent-b when agent-a unhealthy"
    );
    Ok(())
}
