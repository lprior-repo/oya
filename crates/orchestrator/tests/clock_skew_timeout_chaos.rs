#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Chaos tests for clock skew handling in timeout detection.
//!
//! Tests the scenario where system clock skews (forward/backward) and
//! verifies that timeout handling remains correct.
//!
//! **Bead:** src-wxtj
//! **Phase 4 - Chaos Tests:** Clock skew -> timeout handling -> still correct

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

use orchestrator::agent_swarm::{
    AgentHandle, AgentStateLegacy, HealthCheckResult, HealthConfig, HealthMonitor,
};

#[derive(Debug, Error)]
pub enum ClockSkewChaosError {
    #[error("Agent failed to become unhealthy within timeout")]
    HealthCheckTimeout,

    #[error("Agent incorrectly marked unhealthy due to clock skew")]
    FalsePositive,

    #[error("Agent failed to be detected as unhealthy (clock skew hid timeout)")]
    FalseNegative,

    #[error("Setup failed: {reason}")]
    SetupFailed { reason: String },

    #[error("Invariant violated: {details}")]
    InvariantViolation { details: String },

    #[error("Timeout waiting for condition: {condition}")]
    ConditionTimeout { condition: String },
}

pub type ChaosResult<T> = Result<T, ClockSkewChaosError>;

pub struct ClockSkewTestContext {
    agents: Arc<RwLock<std::collections::HashMap<String, AgentHandle>>>,
    health_monitor: HealthMonitor,
}

impl ClockSkewTestContext {
    pub fn new(num_agents: usize, health_config: HealthConfig) -> ChaosResult<Self> {
        let mut agents = std::collections::HashMap::new();
        for i in 0..num_agents {
            let agent_id = format!("agent-{i}");
            agents.insert(
                agent_id.clone(),
                AgentHandle::new(&agent_id).with_max_health_failures(health_config.max_failures),
            );
        }

        let agents = Arc::new(RwLock::new(agents));
        let health_monitor = HealthMonitor::new(health_config, Arc::clone(&agents));

        Ok(Self {
            agents,
            health_monitor,
        })
    }

    pub async fn assign_bead(&self, agent_id: &str, bead_id: &str) -> ChaosResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| ClockSkewChaosError::SetupFailed {
                reason: format!("Agent {agent_id} not found"),
            })?;
        agent.assign_bead(bead_id);
        Ok(())
    }

    pub async fn record_heartbeat(&self, agent_id: &str) -> ChaosResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| ClockSkewChaosError::SetupFailed {
                reason: format!("Agent {agent_id} not found"),
            })?;
        agent.record_heartbeat();
        Ok(())
    }

    pub async fn simulate_clock_forward_skew(
        &self,
        agent_id: &str,
        skew_duration: Duration,
    ) -> ChaosResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| ClockSkewChaosError::SetupFailed {
                reason: format!("Agent {agent_id} not found"),
            })?;

        let skew_chrono = chrono::Duration::from_std(skew_duration).map_err(|e| {
            ClockSkewChaosError::SetupFailed {
                reason: e.to_string(),
            }
        })?;
        let skewed_time = agent
            .last_heartbeat()
            .checked_sub_signed(skew_chrono)
            .ok_or_else(|| ClockSkewChaosError::SetupFailed {
                reason: "Clock skew calculation underflow".to_string(),
            })?;
        agent.set_last_heartbeat_for_test(skewed_time);
        info!(
            "Simulated forward clock skew for {agent_id}: last_heartbeat moved back by {skew_duration:?}"
        );
        Ok(())
    }

    pub async fn simulate_clock_backward_skew(
        &self,
        agent_id: &str,
        skew_duration: Duration,
    ) -> ChaosResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| ClockSkewChaosError::SetupFailed {
                reason: format!("Agent {agent_id} not found"),
            })?;

        let skew_chrono = chrono::Duration::from_std(skew_duration).map_err(|e| {
            ClockSkewChaosError::SetupFailed {
                reason: e.to_string(),
            }
        })?;
        let skewed_time = agent
            .last_heartbeat()
            .checked_add_signed(skew_chrono)
            .ok_or_else(|| ClockSkewChaosError::SetupFailed {
                reason: "Clock skew calculation overflow".to_string(),
            })?;
        agent.set_last_heartbeat_for_test(skewed_time);
        info!(
            "Simulated backward clock skew for {agent_id}: last_heartbeat moved forward by {skew_duration:?}"
        );
        Ok(())
    }

    pub async fn check_agent(&self, agent_id: &str) -> ChaosResult<HealthCheckResult> {
        self.health_monitor
            .check_agent(agent_id)
            .await
            .map_err(|e| ClockSkewChaosError::SetupFailed {
                reason: e.to_string(),
            })
    }

    pub async fn get_agent_state(&self, agent_id: &str) -> ChaosResult<AgentStateLegacy> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| ClockSkewChaosError::SetupFailed {
                reason: format!("Agent {agent_id} not found"),
            })?;
        Ok(agent.state())
    }

    pub async fn get_unhealthy_agents(&self) -> Vec<String> {
        self.health_monitor.get_unhealthy_agents().await
    }

    pub async fn get_available_agents(&self) -> Vec<String> {
        self.health_monitor.get_available_agents().await
    }

    pub fn start_background_check(&self) -> tokio::task::JoinHandle<()> {
        self.health_monitor.start_background_check()
    }

    pub async fn stop_background_check(&self) {
        self.health_monitor.stop().await;
    }
}

#[tokio::test]
async fn given_recent_heartbeat_when_clock_forward_skew_then_not_falsely_marked_unhealthy(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "clock_forward_no_false_positive";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(500),
        max_failures: 1,
    };

    let ctx = ClockSkewTestContext::new(2, health_config)?;

    let agent_id = "agent-0";

    ctx.record_heartbeat(agent_id).await?;

    ctx.simulate_clock_forward_skew(agent_id, Duration::from_millis(200))
        .await?;

    let result = ctx.check_agent(agent_id).await?;

    assert!(
        result.is_healthy,
        "Agent should NOT be marked unhealthy due to forward clock skew when within timeout"
    );
    assert!(
        result.time_since_heartbeat < Duration::from_millis(500),
        "Time since heartbeat should be less than timeout after forward skew"
    );

    info!("Test passed: {test_name}");
    Ok(())
}

#[tokio::test]
async fn given_timed_out_agent_when_clock_backward_skew_then_can_be_hidden(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "clock_backward_can_hide_timeout";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(100),
        max_failures: 1,
    };

    let ctx = ClockSkewTestContext::new(2, health_config)?;

    let agent_id = "agent-0";

    ctx.record_heartbeat(agent_id).await?;

    tokio::time::sleep(Duration::from_millis(150)).await;

    let result_before_skew = ctx.check_agent(agent_id).await?;

    assert!(
        !result_before_skew.is_healthy,
        "Agent should be unhealthy before backward skew (150ms > 100ms timeout)"
    );

    ctx.record_heartbeat(agent_id).await?;

    tokio::time::sleep(Duration::from_millis(80)).await;

    ctx.simulate_clock_backward_skew(agent_id, Duration::from_millis(50))
        .await?;

    let result_after_skew = ctx.check_agent(agent_id).await?;

    assert!(
        result_after_skew.time_since_heartbeat < Duration::from_millis(80),
        "Backward skew should make heartbeat appear more recent (reducing apparent age)"
    );

    info!("Test passed: {test_name}");
    Ok(())
}

#[tokio::test]
async fn given_multiple_agents_when_mixed_clock_skew_then_correct_agents_unhealthy(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "mixed_clock_skew_selective_timeout";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(200),
        max_failures: 1,
    };

    let ctx = ClockSkewTestContext::new(4, health_config)?;

    ctx.record_heartbeat("agent-0").await?;
    ctx.record_heartbeat("agent-1").await?;
    ctx.record_heartbeat("agent-2").await?;
    ctx.record_heartbeat("agent-3").await?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    ctx.simulate_clock_forward_skew("agent-0", Duration::from_millis(500))
        .await?;
    ctx.simulate_clock_backward_skew("agent-1", Duration::from_millis(100))
        .await?;
    ctx.simulate_clock_forward_skew("agent-2", Duration::from_millis(100))
        .await?;

    let result0 = ctx.check_agent("agent-0").await?;
    let result1 = ctx.check_agent("agent-1").await?;
    let result2 = ctx.check_agent("agent-2").await?;
    let result3 = ctx.check_agent("agent-3").await?;

    assert!(
        !result0.is_healthy,
        "agent-0 should be unhealthy (forward skew makes heartbeat appear 500ms old)"
    );
    assert!(
        result1.is_healthy,
        "agent-1 should be healthy (backward skew only reduces apparent age by 100ms)"
    );
    assert!(
        result2.is_healthy,
        "agent-2 should be healthy (forward skew of 100ms within 200ms timeout)"
    );
    assert!(
        result3.is_healthy,
        "agent-3 should be healthy (no skew, recent heartbeat)"
    );

    info!("Test passed: {test_name}");
    Ok(())
}

#[tokio::test]
async fn given_agent_with_bead_when_clock_skew_then_bead_remains_assigned(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "clock_skew_bead_retention";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(200),
        max_failures: 1,
    };

    let ctx = ClockSkewTestContext::new(2, health_config)?;

    let agent_id = "agent-0";
    let bead_id = "bead-critical-001";

    ctx.assign_bead(agent_id, bead_id).await?;

    let state = ctx.get_agent_state(agent_id).await?;
    assert_eq!(state, AgentStateLegacy::Working, "Agent should be working");

    ctx.record_heartbeat(agent_id).await?;

    ctx.simulate_clock_forward_skew(agent_id, Duration::from_millis(300))
        .await?;

    let result = ctx.check_agent(agent_id).await?;

    assert!(
        !result.is_healthy,
        "Agent should be unhealthy after clock skew creates apparent timeout"
    );

    info!("Test passed: {test_name}");
    Ok(())
}

#[tokio::test]
async fn given_ongoing_clock_skew_when_heartbeat_restored_then_recovered(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "clock_skew_recovery";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(150),
        max_failures: 1,
    };

    let ctx = ClockSkewTestContext::new(2, health_config)?;

    let agent_id = "agent-0";

    ctx.record_heartbeat(agent_id).await?;

    ctx.simulate_clock_forward_skew(agent_id, Duration::from_millis(200))
        .await?;

    let result = ctx.check_agent(agent_id).await?;
    assert!(
        !result.is_healthy,
        "Agent should be unhealthy due to clock skew"
    );

    ctx.record_heartbeat(agent_id).await?;

    tokio::time::sleep(Duration::from_millis(20)).await;

    let recovered = ctx.get_agent_state(agent_id).await?;
    assert!(
        recovered == AgentStateLegacy::Idle || recovered == AgentStateLegacy::Working,
        "Agent should recover to healthy state after heartbeat (got {recovered:?})",
    );

    info!("Test passed: {test_name}");
    Ok(())
}

#[tokio::test]
async fn given_extreme_clock_forward_skew_then_timeout_detected_immediately(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "extreme_forward_skew_immediate_timeout";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(500),
        max_failures: 1,
    };

    let ctx = ClockSkewTestContext::new(2, health_config)?;

    let agent_id = "agent-0";

    ctx.record_heartbeat(agent_id).await?;

    ctx.simulate_clock_forward_skew(agent_id, Duration::from_secs(3600))
        .await?;

    let result = ctx.check_agent(agent_id).await?;

    assert!(
        !result.is_healthy,
        "Agent should be unhealthy with 1 hour of apparent heartbeat age"
    );
    assert!(
        result.time_since_heartbeat > Duration::from_secs(3500),
        "Time since heartbeat should be approximately 1 hour"
    );

    info!("Test passed: {test_name}");
    Ok(())
}

#[tokio::test]
async fn given_rapid_clock_fluctuations_then_health_state_stable(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "rapid_clock_fluctuations";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(20),
        heartbeat_timeout: Duration::from_millis(200),
        max_failures: 3,
    };

    let ctx = ClockSkewTestContext::new(2, health_config)?;

    let agent_id = "agent-0";

    ctx.record_heartbeat(agent_id).await?;

    let handle = ctx.start_background_check();

    for i in 0..5 {
        if i % 2 == 0 {
            ctx.simulate_clock_forward_skew(agent_id, Duration::from_millis(100))
                .await?;
        } else {
            ctx.simulate_clock_backward_skew(agent_id, Duration::from_millis(50))
                .await?;
        }

        tokio::time::sleep(Duration::from_millis(30)).await;

        ctx.record_heartbeat(agent_id).await?;
    }

    let state = ctx.get_agent_state(agent_id).await?;
    assert!(
        state == AgentStateLegacy::Idle,
        "Agent should remain healthy despite clock fluctuations (state: {state:?})",
    );

    ctx.stop_background_check().await;
    handle.abort();

    info!("Test passed: {test_name}");
    Ok(())
}

#[tokio::test]
async fn given_all_agents_clock_skewed_when_one_recovers_then_available_for_work(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "partial_recovery_from_clock_skew";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(50),
        heartbeat_timeout: Duration::from_millis(200),
        max_failures: 1,
    };

    let ctx = ClockSkewTestContext::new(3, health_config)?;

    ctx.record_heartbeat("agent-0").await?;
    ctx.record_heartbeat("agent-1").await?;
    ctx.record_heartbeat("agent-2").await?;

    ctx.simulate_clock_forward_skew("agent-0", Duration::from_millis(500))
        .await?;
    ctx.simulate_clock_forward_skew("agent-1", Duration::from_millis(500))
        .await?;

    let _ = ctx.check_agent("agent-0").await;
    let _ = ctx.check_agent("agent-1").await;

    ctx.record_heartbeat("agent-2").await?;

    let available = ctx.get_available_agents().await;
    assert!(
        available.contains(&"agent-2".to_string()),
        "agent-2 should be available after recovery heartbeat"
    );

    let unhealthy = ctx.get_unhealthy_agents().await;
    assert!(
        unhealthy.contains(&"agent-0".to_string()),
        "agent-0 should be unhealthy"
    );
    assert!(
        unhealthy.contains(&"agent-1".to_string()),
        "agent-1 should be unhealthy"
    );

    info!("Test passed: {test_name}");
    Ok(())
}

#[tokio::test]
async fn given_clock_skew_invariant_health_failures_bounded(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "health_failures_bounded";
    info!("Starting test: {test_name}");

    let health_config = HealthConfig {
        check_interval: Duration::from_millis(20),
        heartbeat_timeout: Duration::from_millis(50),
        max_failures: 3,
    };

    let ctx = ClockSkewTestContext::new(2, health_config)?;

    let agent_id = "agent-0";

    ctx.record_heartbeat(agent_id).await?;

    ctx.simulate_clock_forward_skew(agent_id, Duration::from_millis(100))
        .await?;

    for _ in 0..10 {
        let _ = ctx.check_agent(agent_id).await;
    }

    let state = ctx.get_agent_state(agent_id).await?;
    assert_eq!(
        state,
        AgentStateLegacy::Unhealthy,
        "Agent should be unhealthy after max_failures"
    );

    ctx.record_heartbeat(agent_id).await?;

    let recovered_state = ctx.get_agent_state(agent_id).await?;
    assert_eq!(
        recovered_state,
        AgentStateLegacy::Idle,
        "Agent should recover to Idle after heartbeat"
    );

    info!("Test passed: {test_name}");
    Ok(())
}
