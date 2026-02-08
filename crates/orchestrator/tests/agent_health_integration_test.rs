//! BDD integration tests for agent health and assignment.
//!
//! This module tests the behavior described in the bead:
//!
//! ## Phase 2 - BDD Integration Tests
//!
//! GIVEN agent unhealthy WHEN heartbeat timeout THEN not assigned beads.
//!
//! ## Test Scenario
//!
//! Given: An agent that has missed heartbeats
//! When: New beads become ready
//! Then: Unhealthy agent is skipped

// Integration tests allow unwrap/panic for assertions
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use orchestrator::agent_swarm::{AgentHandle, AgentPool, PoolConfig, AgentStateLegacy as AgentState};
use std::time::Duration;

/// BDD Test: Unhealthy agent is not assigned beads
///
/// **Given** an agent that has missed heartbeats (become unhealthy)
/// **When** new beads become ready for assignment
/// **Then** the unhealthy agent is skipped and only healthy agents receive assignments
#[tokio::test]
async fn given_unhealthy_agent_when_beads_ready_then_not_assigned() {
    // Given: An agent pool with three agents
    let pool = AgentPool::new(PoolConfig::for_testing());

    // Register three agents
    let agent_1 = AgentHandle::new("agent-1").with_max_health_failures(2);
    let agent_2 = AgentHandle::new("agent-2").with_max_health_failures(2);
    let agent_3 = AgentHandle::new("agent-3").with_max_health_failures(2);

    pool.register_agent(agent_1).await.expect("agent-1 registration");
    pool.register_agent(agent_2).await.expect("agent-2 registration");
    pool.register_agent(agent_3).await.expect("agent-3 registration");

    // Given: agent-2 becomes unhealthy due to missed heartbeats
    {
        let agents = pool.all_agents().await;
        let mut unhealthy_agent = agents
            .into_iter()
            .find(|a| a.id() == "agent-2")
            .expect("agent-2 should exist");

        // Simulate missed heartbeats by recording health failures
        unhealthy_agent.record_health_failure();
        unhealthy_agent.record_health_failure();

        // Update the pool with the unhealthy agent
        pool.unregister_agent("agent-2")
            .await
            .expect("unregister agent-2");
        pool.register_agent(unhealthy_agent)
            .await
            .expect("re-register unhealthy agent-2");
    }

    // Verify agent-2 is now unhealthy
    let agent_2_state = pool.get_agent("agent-2").await;
    assert!(
        agent_2_state.is_some(),
        "agent-2 should still be in the pool"
    );

    let agent_2 = agent_2_state.expect("agent-2 exists");
    assert_eq!(
        agent_2.state(),
        AgentState::Unhealthy,
        "agent-2 should be marked as Unhealthy"
    );

    // When: New beads become ready for assignment
    let bead_1_result = pool.assign_bead("bead-1").await;
    let bead_2_result = pool.assign_bead("bead-2").await;

    // Then: Only healthy agents receive assignments (agent-1 and agent-3)
    assert!(
        bead_1_result.is_ok(),
        "First bead assignment should succeed"
    );

    let assigned_agent_1 = bead_1_result.expect("bead-1 assigned");
    assert!(
        assigned_agent_1 != "agent-2",
        "agent-2 (unhealthy) should not be assigned bead-1"
    );

    assert!(
        bead_2_result.is_ok(),
        "Second bead assignment should succeed"
    );

    let assigned_agent_2 = bead_2_result.expect("bead-2 assigned");
    assert!(
        assigned_agent_2 != "agent-2",
        "agent-2 (unhealthy) should not be assigned bead-2"
    );

    // Verify that both healthy agents got work
    let assigned_agents = vec![assigned_agent_1, assigned_agent_2];
    assert!(
        assigned_agents.contains(&"agent-1".to_string()),
        "agent-1 should have been assigned a bead"
    );
    assert!(
        assigned_agents.contains(&"agent-3".to_string()),
        "agent-3 should have been assigned a bead"
    );
    assert!(
        !assigned_agents.contains(&"agent-2".to_string()),
        "agent-2 should not have been assigned any bead"
    );

    // Verify pool state
    let stats = pool.stats().await;
    assert_eq!(stats.working, 2, "Two agents should be working");
    assert_eq!(stats.unhealthy, 1, "One agent should be unhealthy");
    assert_eq!(stats.idle, 0, "No idle agents should remain");
}

/// BDD Test: All agents unhealthy results in no assignment
///
/// **Given** all agents in the pool are unhealthy
/// **When** a bead becomes ready for assignment
/// **Then** no agent is assigned the bead and an error is returned
#[tokio::test]
async fn given_all_unhealthy_when_bead_ready_then_no_assignment() {
    // Given: An agent pool with two agents
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent_1 = AgentHandle::new("agent-1").with_max_health_failures(1);
    let agent_2 = AgentHandle::new("agent-2").with_max_health_failures(1);

    pool.register_agent(agent_1).await.expect("agent-1 registration");
    pool.register_agent(agent_2).await.expect("agent-2 registration");

    // Given: Both agents become unhealthy
    for agent_id in &["agent-1", "agent-2"] {
        let agents = pool.all_agents().await;
        let mut agent = agents
            .into_iter()
            .find(|a| a.id() == *agent_id)
            .unwrap_or_else(|| panic!("{} should exist", agent_id));

        agent.record_health_failure();

        pool.unregister_agent(agent_id)
            .await
            .unwrap_or_else(|e| panic!("unregister {}: {:?}", agent_id, e));
        pool.register_agent(agent)
            .await
            .unwrap_or_else(|e| panic!("re-register unhealthy {}: {:?}", agent_id, e));
    }

    // Verify both are unhealthy
    let stats = pool.stats().await;
    assert_eq!(stats.unhealthy, 2, "Both agents should be unhealthy");

    // When: A bead becomes ready for assignment
    let result = pool.assign_bead("bead-1").await;

    // Then: No agent is assigned
    assert!(result.is_err(), "Assignment should fail when all agents are unhealthy");
}

/// BDD Test: Unhealthy agent recovers and can receive assignments
///
/// **Given** an agent that was unhealthy but has now recovered
/// **When** new beads become ready for assignment
/// **Then** the recovered agent can be assigned beads
#[tokio::test]
async fn given_recovered_agent_when_beads_ready_then_assigned() {
    // Given: An agent pool with one unhealthy and one healthy agent
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent_1 = AgentHandle::new("agent-1").with_max_health_failures(2);
    let agent_2 = AgentHandle::new("agent-2").with_max_health_failures(2);

    pool.register_agent(agent_1).await.expect("agent-1 registration");
    pool.register_agent(agent_2).await.expect("agent-2 registration");

    // Make agent-1 unhealthy
    {
        let agents = pool.all_agents().await;
        let mut unhealthy_agent = agents
            .into_iter()
            .find(|a| a.id() == "agent-1")
            .expect("agent-1 should exist");

        unhealthy_agent.record_health_failure();
        unhealthy_agent.record_health_failure();

        pool.unregister_agent("agent-1")
            .await
            .expect("unregister agent-1");
        pool.register_agent(unhealthy_agent)
            .await
            .expect("re-register unhealthy agent-1");
    }

    // Given: agent-1 recovers (heartbeat received)
    pool.record_heartbeat("agent-1")
        .await
        .expect("record heartbeat for agent-1");

    // Verify agent-1 is now healthy (Idle)
    let agent_1_state = pool.get_agent("agent-1").await;
    let agent_1 = agent_1_state.expect("agent-1 exists");
    assert_eq!(
        agent_1.state(),
        AgentState::Idle,
        "agent-1 should have recovered to Idle state"
    );

    // When: A bead becomes ready for assignment
    let result = pool.assign_bead("bead-1").await;

    // Then: The recovered agent can be assigned
    assert!(result.is_ok(), "Assignment should succeed");

    let assigned_agent = result.expect("bead assigned");
    assert_eq!(
        assigned_agent, "agent-1",
        "Recovered agent-1 should be assigned the bead"
    );
}

/// BDD Test: Agent becomes unhealthy while working on a bead
///
/// **Given** an agent that is working on a bead but becomes unhealthy
/// **When** checking available agents for new assignments
/// **Then** the unhealthy working agent is not considered available
#[tokio::test]
async fn given_working_becomes_unhealthy_when_new_bead_then_not_assigned() {
    // Given: An agent pool with two agents, one working
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent_1 = AgentHandle::new("agent-1").with_max_health_failures(2);
    let agent_2 = AgentHandle::new("agent-2").with_max_health_failures(2);

    pool.register_agent(agent_1).await.expect("agent-1 registration");
    pool.register_agent(agent_2).await.expect("agent-2 registration");

    // Assign bead-1 to agent-1 (making it Working)
    pool.assign_bead_to_agent("bead-1", "agent-1")
        .await
        .expect("assign bead-1 to agent-1");

    // Given: agent-1 becomes unhealthy while working
    {
        let agents = pool.all_agents().await;
        let mut unhealthy_agent = agents
            .into_iter()
            .find(|a| a.id() == "agent-1")
            .expect("agent-1 should exist");

        unhealthy_agent.record_health_failure();
        unhealthy_agent.record_health_failure();

        pool.unregister_agent("agent-1")
            .await
            .expect("unregister agent-1");
        pool.register_agent(unhealthy_agent)
            .await
            .expect("re-register unhealthy agent-1");
    }

    // Verify agent-1 is unhealthy
    let agent_1_state = pool.get_agent("agent-1").await;
    let agent_1 = agent_1_state.expect("agent-1 exists");
    assert_eq!(
        agent_1.state(),
        AgentState::Unhealthy,
        "agent-1 should be Unhealthy even while working"
    );

    // When: A new bead becomes ready
    let result = pool.assign_bead("bead-2").await;

    // Then: agent-2 (the only available agent) gets the assignment
    assert!(result.is_ok(), "Assignment should succeed");

    let assigned_agent = result.expect("bead-2 assigned");
    assert_eq!(
        assigned_agent, "agent-2",
        "Only agent-2 should be assigned (agent-1 is unhealthy)"
    );
}

/// Helper test: Verify heartbeat timeout detection
///
/// This test validates that the heartbeat timeout mechanism works correctly.
#[tokio::test]
async fn test_heartbeat_timeout_detection() {
    let pool = AgentPool::new(PoolConfig::for_testing());

    let agent = AgentHandle::new("agent-timeout-test");
    pool.register_agent(agent).await.expect("registration");

    // Agent should initially be healthy
    let agent_state = pool.get_agent("agent-timeout-test").await;
    let agent = agent_state.expect("agent exists");

    assert_eq!(
        agent.state(),
        AgentState::Idle,
        "New agent should be Idle"
    );

    // Simulate heartbeat timeout
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Check health after timeout
    let health_result = pool
        .health_monitor()
        .check_agent("agent-timeout-test")
        .await;

    assert!(health_result.is_ok(), "Health check should succeed");

    let result = health_result.expect("health check result");
    assert!(
        result.time_since_heartbeat > Duration::from_millis(500),
        "Should detect heartbeat timeout"
    );
}
