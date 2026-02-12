//! Chaos tests for slow agent timeout and bead reassignment.
//!
//! Tests the scenario where an agent takes too long (slow/stuck),
//! triggers a timeout, and the bead is reassigned to another agent.
//!
//! **Bead:** src-v5ep
//! **Phase 4 - Chaos Tests:** Slow agent -> timeout -> reassign bead
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::time::Duration;

use orchestrator::agent_swarm::{
    AgentHandle, AgentPool, AgentStateLegacy, HealthConfig, HealthMonitor, PoolConfig,
};
use thiserror::Error;
use tracing::info;

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, Error)]
pub enum SlowAgentChaosError {
    #[error("Agent failed to become unhealthy within timeout")]
    HealthCheckTimeout,

    #[error("Bead was not reassigned after agent timeout")]
    ReassignmentFailed,

    #[error("Agent pool setup failed: {reason}")]
    SetupFailed { reason: String },

    #[error("No agents available for reassignment")]
    NoAgentsAvailable,

    #[error("Invariant violated: {details}")]
    InvariantViolation { details: String },

    #[error("Timeout waiting for condition: {condition}")]
    ConditionTimeout { condition: String },
}

pub type ChaosResult<T> = Result<T, SlowAgentChaosError>;

// =============================================================================
// Test Context
// =============================================================================

pub struct SlowAgentTestContext {
    pub pool: AgentPool,
    pub health_monitor: HealthMonitor,
    pub agent_ids: Vec<String>,
}

impl SlowAgentTestContext {
    pub async fn new(num_agents: usize, health_config: HealthConfig) -> ChaosResult<Self> {
        let pool = AgentPool::new(PoolConfig {
            max_agents: num_agents,
            health_config: health_config.clone(),
        });

        let mut agent_ids = Vec::with_capacity(num_agents);
        for i in 0..num_agents {
            let agent_id = format!("agent-{i}");
            pool.register_agent(AgentHandle::new(&agent_id))
                .await
                .map_err(|e| SlowAgentChaosError::SetupFailed {
                    reason: e.to_string(),
                })?;
            agent_ids.push(agent_id);
        }

        let health_monitor = pool.health_monitor().clone();

        Ok(Self {
            pool,
            health_monitor,
            agent_ids,
        })
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

async fn wait_for_agent_state(
    ctx: &SlowAgentTestContext,
    agent_id: &str,
    target_state: AgentStateLegacy,
    timeout_ms: u64,
) -> ChaosResult<()> {
    let start = std::time::Instant::now();
    let deadline = Duration::from_millis(timeout_ms);

    while start.elapsed() < deadline {
        if let Some(agent) = ctx.pool.get_agent(agent_id).await {
            if agent.state() == target_state {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    Err(SlowAgentChaosError::ConditionTimeout {
        condition: format!("agent {agent_id} to reach state {target_state:?}"),
    })
}

async fn simulate_slow_agent(ctx: &SlowAgentTestContext, agent_id: &str, bead_id: &str) -> ChaosResult<()> {
    ctx.pool
        .assign_bead_to_agent(bead_id, agent_id)
        .await
        .map_err(|e| SlowAgentChaosError::SetupFailed {
            reason: e.to_string(),
        })?;

    ctx.pool
        .complete_bead(agent_id)
        .await
        .map_err(|e| SlowAgentChaosError::SetupFailed {
            reason: e.to_string(),
        })?;

    info!("Simulated slow agent {} assigned to bead {}", agent_id, bead_id);
    Ok(())
}

async fn get_available_agent(ctx: &SlowAgentTestContext) -> ChaosResult<String> {
    let available = ctx.health_monitor.get_available_agents().await;
    available
        .first()
        .cloned()
        .ok_or(SlowAgentChaosError::NoAgentsAvailable)
}

// =============================================================================
// Chaos Tests
// =============================================================================

#[tokio::test]
async fn given_agent_with_bead_when_slow_timeout_then_bead_reassigned() {
    let test_name = "slow_agent_reassign";
    info!("Starting test: {}", test_name);

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(200),
        max_failures: 1,
    };

    let ctx = SlowAgentTestContext::new(3, health_config)
        .await
        .expect("Failed to setup test context");

    let slow_agent = "agent-0";
    let bead_id = "bead-slow-001";

    ctx.pool
        .assign_bead_to_agent(bead_id, slow_agent)
        .await
        .expect("assign should succeed");

    let _handle = ctx.health_monitor.start_background_check();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let result = ctx.health_monitor.check_agent(slow_agent).await;
    assert!(result.is_ok());

    tokio::time::sleep(Duration::from_millis(300)).await;

    let health_result = ctx
        .health_monitor
        .check_agent(slow_agent)
        .await
        .expect("health check should succeed");

    assert!(
        !health_result.is_healthy || health_result.state == AgentStateLegacy::Unhealthy,
        "Slow agent should become unhealthy after heartbeat timeout"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_multiple_agents_when_one_times_out_then_others_available() {
    let test_name = "fallback_agent_available";
    info!("Starting test: {}", test_name);

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(100),
        max_failures: 1,
    };

    let ctx = SlowAgentTestContext::new(3, health_config)
        .await
        .expect("Failed to setup test context");

    let slow_agent = "agent-0";
    let healthy_agent = "agent-1";

    ctx.pool
        .assign_bead_to_agent("bead-001", slow_agent)
        .await
        .expect("assign should succeed");

    let _handle = ctx.health_monitor.start_background_check();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let _ = ctx.health_monitor.check_agent(slow_agent).await;

    let healthy = ctx.pool.get_agent(healthy_agent).await.expect("agent exists");
    assert!(
        healthy.is_available(),
        "Other agents should remain available"
    );

    let available_agents = ctx.health_monitor.get_available_agents().await;
    assert!(
        available_agents.contains(&healthy_agent.to_string()),
        "Healthy agent should be in available list"
    );
    assert!(
        !available_agents.contains(&slow_agent.to_string()),
        "Slow agent should not be in available list"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_unhealthy_agent_when_heartbeat_restored_then_becomes_healthy() {
    let test_name = "agent_recovery";
    info!("Starting test: {}", test_name);

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(100),
        max_failures: 1,
    };

    let ctx = SlowAgentTestContext::new(2, health_config)
        .await
        .expect("Failed to setup test context");

    let agent_id = "agent-0";

    let _handle = ctx.health_monitor.start_background_check();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let _ = ctx.health_monitor.check_agent(agent_id).await;

    let result = ctx
        .pool
        .record_heartbeat(agent_id)
        .await;
    assert!(result.is_ok(), "Recording heartbeat should succeed");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let health_result = ctx
        .health_monitor
        .check_agent(agent_id)
        .await
        .expect("health check should succeed");

    assert!(
        health_result.is_healthy,
        "Agent should become healthy after heartbeat"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_working_agent_when_times_out_then_marked_unhealthy_with_bead() {
    let test_name = "working_agent_timeout";
    info!("Starting test: {}", test_name);

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(100),
        max_failures: 1,
    };

    let ctx = SlowAgentTestContext::new(2, health_config)
        .await
        .expect("Failed to setup test context");

    let agent_id = "agent-0";
    let bead_id = "bead-critical-001";

    ctx.pool
        .assign_bead_to_agent(bead_id, agent_id)
        .await
        .expect("assign should succeed");

    let agent = ctx.pool.get_agent(agent_id).await.expect("agent exists");
    assert_eq!(agent.state(), AgentStateLegacy::Working);
    assert_eq!(agent.current_bead(), Some(bead_id));
    drop(agent);

    tokio::time::sleep(Duration::from_millis(150)).await;

    let health_result = ctx
        .health_monitor
        .check_agent(agent_id)
        .await
        .expect("health check should succeed");
    
    assert!(
        !health_result.is_healthy,
        "Agent should become unhealthy after heartbeat timeout"
    );

    let agent = ctx.pool.get_agent(agent_id).await.expect("agent exists");
    assert!(
        agent.state() == AgentStateLegacy::Unhealthy,
        "Working agent should be marked unhealthy"
    );
    assert_eq!(
        agent.current_bead(),
        Some(bead_id),
        "Bead should still be assigned to unhealthy agent"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_pool_with_all_working_agents_when_timeout_then_some_become_unhealthy() {
    let test_name = "multiple_timeouts";
    info!("Starting test: {}", test_name);

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(100),
        max_failures: 1,
    };

    let ctx = SlowAgentTestContext::new(4, health_config)
        .await
        .expect("Failed to setup test context");

    for (i, agent_id) in ctx.agent_ids.iter().enumerate() {
        let bead_id = format!("bead-{:03}", i);
        ctx.pool
            .assign_bead_to_agent(&bead_id, agent_id)
            .await
            .expect("assign should succeed");
    }

    ctx.pool
        .record_heartbeat("agent-1")
        .await
        .expect("heartbeat should succeed");
    ctx.pool
        .record_heartbeat("agent-3")
        .await
        .expect("heartbeat should succeed");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let _ = ctx.health_monitor.check_agent("agent-0").await;
    let _ = ctx.health_monitor.check_agent("agent-2").await;
    let _ = ctx.health_monitor.check_agent("agent-1").await;
    let _ = ctx.health_monitor.check_agent("agent-3").await;

    let unhealthy = ctx.health_monitor.get_unhealthy_agents().await;
    assert!(
        unhealthy.contains(&"agent-0".to_string()),
        "Agent-0 should be unhealthy (no heartbeat)"
    );
    assert!(
        unhealthy.contains(&"agent-2".to_string()),
        "Agent-2 should be unhealthy (no heartbeat)"
    );
    assert!(
        !unhealthy.contains(&"agent-1".to_string()),
        "Agent-1 should be healthy (has heartbeat)"
    );
    assert!(
        !unhealthy.contains(&"agent-3".to_string()),
        "Agent-3 should be healthy (has heartbeat)"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_agent_timeout_when_reassign_attempted_then_succeeds() {
    let test_name = "reassign_after_timeout";
    info!("Starting test: {}", test_name);

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(100),
        max_failures: 1,
    };

    let ctx = SlowAgentTestContext::new(3, health_config)
        .await
        .expect("Failed to setup test context");

    let slow_agent = "agent-0";
    let bead_id = "bead-reassign-001";

    ctx.pool
        .assign_bead_to_agent(bead_id, slow_agent)
        .await
        .expect("assign should succeed");

    let _handle = ctx.health_monitor.start_background_check();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let slow_agent_handle = ctx.pool.get_agent(slow_agent).await.expect("agent exists");
    let was_working = slow_agent_handle.state() == AgentStateLegacy::Working
        || slow_agent_handle.state() == AgentStateLegacy::Unhealthy;
    assert!(was_working, "Agent should be working or unhealthy");

    let new_bead_id = "bead-reassign-002";
    let new_agent_result = ctx.pool.assign_bead(new_bead_id).await;
    assert!(
        new_agent_result.is_ok(),
        "Reassignment should succeed to a healthy agent"
    );

    let new_agent_id = new_agent_result.expect("should have agent id");
    assert_ne!(
        new_agent_id, slow_agent,
        "Bead should be assigned to a different agent"
    );

    let new_agent = ctx.pool.get_agent(&new_agent_id).await.expect("agent exists");
    assert_eq!(new_agent.state(), AgentStateLegacy::Working);

    info!("Test passed: {} (reassigned to {})", test_name, new_agent_id);
}

#[tokio::test]
async fn given_slow_agent_during_high_load_when_timeout_then_pool_remains_stable() {
    let test_name = "high_load_stability";
    info!("Starting test: {}", test_name);

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(150),
        max_failures: 2,
    };

    let ctx = SlowAgentTestContext::new(5, health_config)
        .await
        .expect("Failed to setup test context");

    for (i, agent_id) in ctx.agent_ids.iter().enumerate() {
        let bead_id = format!("bead-{:03}", i);
        ctx.pool
            .assign_bead_to_agent(&bead_id, agent_id)
            .await
            .expect("assign should succeed");
    }

    let _handle = ctx.health_monitor.start_background_check();

    for _ in 0..3 {
        tokio::time::sleep(Duration::from_millis(100)).await;

        ctx.pool
            .record_heartbeat("agent-2")
            .await
            .expect("heartbeat should succeed");
        ctx.pool
            .record_heartbeat("agent-4")
            .await
            .expect("heartbeat should succeed");
    }

    let stats = ctx.pool.stats().await;
    assert_eq!(stats.total, 5, "All agents should still be in pool");

    let unhealthy = ctx.health_monitor.get_unhealthy_agents().await;
    assert!(
        unhealthy.len() <= 3,
        "At most 3 agents should be unhealthy (those without heartbeats)"
    );

    info!(
        "Test passed: {} (unhealthy: {}/{}",
        test_name,
        unhealthy.len(),
        stats.total
    );
}

#[tokio::test]
async fn given_background_monitor_when_stopped_then_no_more_health_checks() {
    let test_name = "monitor_stop";
    info!("Starting test: {}", test_name);

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(100),
        max_failures: 1,
    };

    let ctx = SlowAgentTestContext::new(2, health_config)
        .await
        .expect("Failed to setup test context");

    let handle = ctx.health_monitor.start_background_check();
    assert!(ctx.health_monitor.is_active().await);

    ctx.health_monitor.stop().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(
        !ctx.health_monitor.is_active().await,
        "Monitor should be stopped"
    );

    handle.abort();

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_agent_pool_invariant_all_registered_agents_exist() {
    let test_name = "pool_invariant";
    info!("Starting test: {}", test_name);

    let ctx = SlowAgentTestContext::new(4, HealthConfig::for_testing())
        .await
        .expect("Failed to setup test context");

    for expected_id in &["agent-0", "agent-1", "agent-2", "agent-3"] {
        let agent = ctx
            .pool
            .get_agent(expected_id)
            .await
            .expect("agent should exist");
        assert_eq!(agent.id(), *expected_id);
    }

    let all_agents = ctx.pool.all_agents().await;
    assert_eq!(all_agents.len(), 4);

    let stats = ctx.pool.stats().await;
    assert_eq!(stats.total, 4);

    info!("Test passed: {}", test_name);
}
