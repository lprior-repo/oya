#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! BDD integration tests for agent assignment when beads become ready.
//!
//! This module tests the behavior described in bead src-1k71:
//!
//! ## Phase 2 - BDD Integration Tests
//!
//! GIVEN agent registered WHEN bead ready THEN assigned to agent.
//!
//! ## Test Scenario
//!
//! Given: A registered agent and ready bead
//! When: Distribution runs
//! Then: Bead is assigned to agent

use orchestrator::agent_swarm::{AgentHandle, AgentPool, PoolConfig};

/// BDD Test: Registered agent is assigned a ready bead
///
/// **Given** an agent is registered in the pool
/// **When** a bead becomes ready for assignment
/// **Then** the bead is assigned to the registered agent
#[tokio::test]
async fn given_registered_agent_when_bead_ready_then_assigned(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: An agent pool with one registered agent
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent = AgentHandle::new("agent-1");
    pool.register_agent(agent).await?;

    // Verify agent is registered and idle
    let agent_state = pool.get_agent("agent-1").await;
    assert!(
        agent_state.is_some(),
        "agent-1 should be registered in the pool"
    );

    let agent = agent_state.ok_or("agent-1 exists")?;
    assert_eq!(
        agent.state(),
        orchestrator::agent_swarm::AgentStateLegacy::Idle,
        "agent-1 should be in Idle state"
    );

    // When: A bead becomes ready for assignment
    let bead_id = "bead-ready-001";
    let assignment_result = pool.assign_bead(bead_id).await;

    // Then: The bead is assigned to the registered agent
    assert!(
        assignment_result.is_ok(),
        "Bead assignment should succeed when agent is available"
    );

    let assigned_agent = assignment_result.map_err(|e| format!("bead assigned failed: {e:?}"))?;
    assert_eq!(
        assigned_agent, "agent-1",
        "Bead should be assigned to agent-1"
    );

    // Verify the agent is now working
    let agent_state = pool.get_agent("agent-1").await;
    let agent = agent_state.ok_or("agent-1 should still exist")?;
    assert_eq!(
        agent.state(),
        orchestrator::agent_swarm::AgentStateLegacy::Working,
        "agent-1 should be in Working state after assignment"
    );

    // Verify pool statistics
    let stats = pool.stats().await;
    assert_eq!(stats.idle, 0, "No idle agents should remain");
    assert_eq!(stats.working, 1, "One agent should be working");
    assert_eq!(stats.total, 1, "Total agent count should be 1");
    Ok(())
}

/// BDD Test: Multiple agents with round-robin assignment
///
/// **Given** multiple agents are registered
/// **When** multiple beads become ready
/// **Then** beads are distributed among available agents
#[tokio::test]
async fn given_multiple_agents_when_multiple_beads_then_distributed(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: An agent pool with three registered agents
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent_1 = AgentHandle::new("agent-a");
    let agent_2 = AgentHandle::new("agent-b");
    let agent_3 = AgentHandle::new("agent-c");

    pool.register_agent(agent_1).await?;
    pool.register_agent(agent_2).await?;
    pool.register_agent(agent_3).await?;

    // Verify all agents are registered
    let stats = pool.stats().await;
    assert_eq!(stats.total, 3, "Three agents should be registered");
    assert_eq!(stats.idle, 3, "All agents should be idle");

    // When: Three beads become ready for assignment
    let bead_1_result = pool.assign_bead("bead-1").await;
    let bead_2_result = pool.assign_bead("bead-2").await;
    let bead_3_result = pool.assign_bead("bead-3").await;

    // Then: All beads are assigned to different agents
    assert!(
        bead_1_result.is_ok(),
        "First bead assignment should succeed"
    );
    assert!(
        bead_2_result.is_ok(),
        "Second bead assignment should succeed"
    );
    assert!(
        bead_3_result.is_ok(),
        "Third bead assignment should succeed"
    );

    let agent_1 = bead_1_result.map_err(|e| format!("bead-1 assigned failed: {e:?}"))?;
    let agent_2 = bead_2_result.map_err(|e| format!("bead-2 assigned failed: {e:?}"))?;
    let agent_3 = bead_3_result.map_err(|e| format!("bead-3 assigned failed: {e:?}"))?;

    // All three agents should have been assigned
    let assigned_agents = [agent_1, agent_2, agent_3];
    assert!(
        assigned_agents.contains(&"agent-a".to_string()),
        "agent-a should have been assigned a bead"
    );
    assert!(
        assigned_agents.contains(&"agent-b".to_string()),
        "agent-b should have been assigned a bead"
    );
    assert!(
        assigned_agents.contains(&"agent-c".to_string()),
        "agent-c should have been assigned a bead"
    );

    // Verify all agents are now working
    let stats = pool.stats().await;
    assert_eq!(stats.working, 3, "All three agents should be working");
    assert_eq!(stats.idle, 0, "No idle agents should remain");
    Ok(())
}

/// BDD Test: Bead ready when no agents available returns error
///
/// **Given** an empty agent pool (no registered agents)
/// **When** a bead becomes ready for assignment
/// **Then** assignment fails with an appropriate error
#[tokio::test]
async fn given_no_agents_when_bead_ready_then_assignment_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: An empty agent pool
    let pool = AgentPool::new(PoolConfig::for_testing());

    // Verify pool is empty
    let stats = pool.stats().await;
    assert_eq!(stats.total, 0, "Pool should be empty");

    // When: A bead becomes ready for assignment
    let assignment_result = pool.assign_bead("bead-no-agent").await;

    // Then: Assignment fails
    assert!(
        assignment_result.is_err(),
        "Bead assignment should fail when no agents are available"
    );

    let error = assignment_result
        .map_err(|e| format!("should return error: {e:?}"))
        .err()
        .ok_or("Expected Err but got Ok")?;
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("no agents")
            || error_msg.contains("available")
            || error_msg.contains("empty"),
        "Error should indicate no agents available: {error_msg}",
    );
    Ok(())
}

/// BDD Test: Bead assignment when all agents are working
///
/// **Given** all registered agents are already working
/// **When** a new bead becomes ready
/// **Then** assignment fails or queues the bead
#[tokio::test]
async fn given_all_agents_working_when_new_bead_then_no_assignment(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: An agent pool with one agent that's already working
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent = AgentHandle::new("agent-busy");
    pool.register_agent(agent).await?;

    // Manually assign first bead to make agent working
    pool.assign_bead_to_agent("bead-1", "agent-busy").await?;

    // Verify agent is working
    let agent_state = pool.get_agent("agent-busy").await;
    let agent = agent_state.ok_or("agent should exist")?;
    assert_eq!(
        agent.state(),
        orchestrator::agent_swarm::AgentStateLegacy::Working,
        "Agent should be working"
    );

    // When: A new bead becomes ready
    let assignment_result = pool.assign_bead("bead-2").await;

    // Then: No assignment available (all agents busy)
    // This may either return an error or None depending on implementation
    assert!(
        assignment_result.is_err(),
        "Assignment should fail when all agents are working"
    );
    Ok(())
}

/// BDD Test: Agent completes bead and becomes available again
///
/// **Given** an agent that was working on a bead
/// **When** the agent completes the bead
/// **Then** the agent becomes available for new assignments
#[tokio::test]
async fn given_agent_completes_bead_when_bead_ready_then_reassigned(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: An agent pool with one working agent
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent = AgentHandle::new("agent-reuse");
    pool.register_agent(agent).await?;

    // Assign first bead
    let first_assignment = pool.assign_bead("bead-1").await;
    assert!(first_assignment.is_ok(), "First assignment should succeed");

    // When: Agent completes the bead
    pool.complete_bead("agent-reuse").await?;

    // Verify agent is back to idle
    let agent_state = pool.get_agent("agent-reuse").await;
    let agent = agent_state.ok_or("agent should exist")?;
    assert_eq!(
        agent.state(),
        orchestrator::agent_swarm::AgentStateLegacy::Idle,
        "Agent should be idle after completing bead"
    );

    // When: A new bead becomes ready
    let second_assignment = pool.assign_bead("bead-2").await;

    // Then: The same agent can be assigned again
    assert!(
        second_assignment.is_ok(),
        "Second assignment should succeed"
    );

    let assigned_agent = second_assignment.map_err(|e| format!("second bead assigned: {e:?}"))?;
    assert_eq!(
        assigned_agent, "agent-reuse",
        "Same agent should be assigned new bead"
    );
    Ok(())
}
