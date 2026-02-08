//! BDD integration tests for agent pool capacity limits.
//!
//! This module tests the behavior described in bead src-cfa3:
//!
//! ## Phase 2 - BDD Tests
//!
//! GIVEN agent pool WHEN capacity limit reached THEN rejects new.

// Integration tests allow unwrap/panic for assertions
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use orchestrator::agent_swarm::{AgentHandle, AgentPool, PoolConfig};

/// BDD Test: Agent pool rejects registration when capacity limit is reached
///
/// **Given** an agent pool with a capacity limit
/// **When** the capacity limit is reached
/// **Then** new agent registrations are rejected with appropriate error
#[tokio::test]
async fn given_agent_pool_when_capacity_limit_reached_then_rejects_new() {
    // Given: An agent pool with a capacity limit of 2 agents
    let mut config = PoolConfig::for_testing();
    config.max_agents = 2;
    let pool = AgentPool::new(config);

    // Verify pool is empty
    let stats = pool.stats().await;
    assert_eq!(stats.total, 0, "Pool should start empty");

    // When: Register agents up to capacity limit
    let agent_1 = AgentHandle::new("agent-1");
    let reg_1 = pool.register_agent(agent_1).await;
    assert!(
        reg_1.is_ok(),
        "First agent registration should succeed: {:?}",
        reg_1
    );

    let agent_2 = AgentHandle::new("agent-2");
    let reg_2 = pool.register_agent(agent_2).await;
    assert!(
        reg_2.is_ok(),
        "Second agent registration should succeed: {:?}",
        reg_2
    );

    // Verify pool is at capacity
    let stats = pool.stats().await;
    assert_eq!(stats.total, 2, "Pool should be at capacity");

    // When: Try to register a third agent beyond capacity
    let agent_3 = AgentHandle::new("agent-3");
    let reg_3 = pool.register_agent(agent_3).await;

    // Then: Registration is rejected
    assert!(
        reg_3.is_err(),
        "Third agent registration should fail when pool is at capacity"
    );

    let error = reg_3.expect_err("should return error");
    let error_msg = error.to_string();

    assert!(
        error_msg.contains("capacity") || error_msg.contains("exceeded") || error_msg.contains("max"),
        "Error should indicate capacity limit exceeded: {}",
        error_msg
    );

    // Verify pool size remains at capacity
    let stats = pool.stats().await;
    assert_eq!(stats.total, 2, "Pool size should remain at capacity");
    assert_eq!(stats.idle, 2, "Both agents should be idle");
}

/// BDD Test: Unregistering agent creates capacity for new registration
///
/// **Given** an agent pool at capacity limit
/// **When** an agent is unregistered
/// **Then** new agent can be registered
#[tokio::test]
async fn given_pool_at_capacity_when_agent_unregistered_then_new_registration_succeeds() {
    // Given: An agent pool at capacity (max 2 agents)
    let mut config = PoolConfig::for_testing();
    config.max_agents = 2;
    let pool = AgentPool::new(config);

    let _ = pool.register_agent(AgentHandle::new("agent-1")).await;
    let _ = pool.register_agent(AgentHandle::new("agent-2")).await;

    // Verify pool is at capacity
    let stats = pool.stats().await;
    assert_eq!(stats.total, 2, "Pool should be at capacity");

    // When: Unregister one agent
    let unreg_result = pool.unregister_agent("agent-1").await;
    assert!(
        unreg_result.is_ok(),
        "Agent unregistration should succeed: {:?}",
        unreg_result
    );

    // Verify capacity is available
    let stats = pool.stats().await;
    assert_eq!(stats.total, 1, "Pool should have 1 agent after unregistration");

    // When: Try to register a new agent
    let agent_3 = AgentHandle::new("agent-3");
    let reg_3 = pool.register_agent(agent_3).await;

    // Then: Registration succeeds
    assert!(
        reg_3.is_ok(),
        "New agent registration should succeed after capacity is freed"
    );

    // Verify pool is back at capacity
    let stats = pool.stats().await;
    assert_eq!(stats.total, 2, "Pool should be at capacity again");
}

/// BDD Test: Agent pool capacity limit is enforced across multiple operations
///
/// **Given** an agent pool with capacity limit
/// **When** agents are registered and unregistered in various patterns
/// **Then** capacity limit is always enforced
#[tokio::test]
async fn given_pool_capacity_when_multiple_operations_then_limit_always_enforced() {
    // Given: An agent pool with capacity of 3 agents
    let mut config = PoolConfig::for_testing();
    config.max_agents = 3;
    let pool = AgentPool::new(config);

    // When: Register up to capacity
    let _ = pool.register_agent(AgentHandle::new("agent-a")).await;
    let _ = pool.register_agent(AgentHandle::new("agent-b")).await;
    let _ = pool.register_agent(AgentHandle::new("agent-c")).await;

    // Then: Cannot exceed capacity
    let over_capacity = pool.register_agent(AgentHandle::new("agent-d")).await;
    assert!(
        over_capacity.is_err(),
        "Should not exceed capacity"
    );

    // When: Unregister one agent
    let _ = pool.unregister_agent("agent-b").await;

    // Then: Can register one more
    let new_agent = pool.register_agent(AgentHandle::new("agent-e")).await;
    assert!(
        new_agent.is_ok(),
        "Should be able to register after freeing capacity"
    );

    // Then: Still cannot exceed capacity
    let over_capacity_2 = pool.register_agent(AgentHandle::new("agent-f")).await;
    assert!(
        over_capacity_2.is_err(),
        "Should still enforce capacity limit"
    );

    let stats = pool.stats().await;
    assert_eq!(stats.total, 3, "Pool should remain at capacity");
}

/// BDD Test: Pool capacity configuration is respected
///
/// **Given** different pool capacity configurations
/// **When** agents are registered
/// **Then** each pool enforces its own capacity limit
#[tokio::test]
async fn given_different_capacities_when_agents_registered_then_each_enforces_limit() {
    // Given: Two pools with different capacity limits
    let mut config_small = PoolConfig::for_testing();
    config_small.max_agents = 1;
    let pool_small = AgentPool::new(config_small);

    let mut config_large = PoolConfig::for_testing();
    config_large.max_agents = 5;
    let pool_large = AgentPool::new(config_large);

    // When: Register agents in both pools
    let _ = pool_small.register_agent(AgentHandle::new("small-1")).await;
    let _ = pool_large
        .register_agent(AgentHandle::new("large-1"))
        .await;
    let _ = pool_large
        .register_agent(AgentHandle::new("large-2"))
        .await;

    // Then: Each pool enforces its own limit
    let small_over = pool_small.register_agent(AgentHandle::new("small-2")).await;
    assert!(
        small_over.is_err(),
        "Small pool should enforce capacity of 1"
    );

    let large_ok = pool_large
        .register_agent(AgentHandle::new("large-3"))
        .await;
    assert!(
        large_ok.is_ok(),
        "Large pool should allow up to 5 agents"
    );

    // Verify pool sizes
    assert_eq!(pool_small.len().await, 1, "Small pool at capacity");
    assert_eq!(pool_large.len().await, 3, "Large pool has room");
}

/// BDD Test: Agent pool stats accurately reflect capacity limit
///
/// **Given** an agent pool at capacity
/// **When** statistics are queried
/// **Then** stats show pool is at capacity
#[tokio::test]
async fn given_pool_at_capacity_when_stats_queried_then_show_at_capacity() {
    // Given: An agent pool with capacity of 2
    let mut config = PoolConfig::for_testing();
    config.max_agents = 2;
    let pool = AgentPool::new(config);

    // Register agents to capacity
    let _ = pool.register_agent(AgentHandle::new("agent-1")).await;
    let _ = pool.register_agent(AgentHandle::new("agent-2")).await;

    // Assign one bead to make one agent working
    let _ = pool.assign_bead("bead-1").await;

    // When: Query pool stats
    let stats = pool.stats().await;

    // Then: Stats accurately reflect pool state
    assert_eq!(
        stats.total, 2,
        "Total agents should equal capacity"
    );
    assert_eq!(
        stats.working, 1,
        "One agent should be working"
    );
    assert_eq!(
        stats.idle, 1,
        "One agent should be idle"
    );
    assert_eq!(
        stats.total,
        stats.working + stats.idle,
        "Total should equal sum of working and idle"
    );

    // Verify against config
    assert_eq!(
        stats.total,
        pool.config().max_agents,
        "Pool should be at configured max capacity"
    );
}

/// BDD Test: Zero capacity pool rejects all registrations
///
/// **Given** an agent pool with zero capacity
/// **When** attempting to register any agent
/// **Then** all registrations are rejected
#[tokio::test]
async fn given_zero_capacity_pool_when_registration_then_always_rejects() {
    // Given: An agent pool with zero capacity
    let mut config = PoolConfig::for_testing();
    config.max_agents = 0;
    let pool = AgentPool::new(config);

    // When: Try to register an agent
    let result = pool.register_agent(AgentHandle::new("agent-1")).await;

    // Then: Registration is rejected
    assert!(
        result.is_err(),
        "Zero-capacity pool should reject all registrations"
    );

    if let Err(err) = result {
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("capacity") || err_msg.contains("exceeded"),
            "Error should mention capacity: {}",
            err
        );
    }

    // Verify pool remains empty
    let stats = pool.stats().await;
    assert_eq!(stats.total, 0, "Pool should remain empty");
}
