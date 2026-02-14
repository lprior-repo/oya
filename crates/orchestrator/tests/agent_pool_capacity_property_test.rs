#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Property test: Assigned beads never exceed agent count
//!
//! Invariant: ∀ agent pool operations, assigned_beads <= num_agents
//!
//! This test verifies that no matter what sequence of operations is performed
//! (register, unregister, assign, complete, shutdown), the number of assigned
//! beads never exceeds the number of agents in the pool.

use orchestrator::agent_swarm::{AgentHandle, AgentPool, PoolConfig, PoolStats};

fn count_assigned_beads(agents: &[AgentHandle]) -> usize {
    agents.iter().filter(|a| a.current_bead().is_some()).count()
}

async fn verify_invariant(pool: &AgentPool) -> Result<(), String> {
    let stats = pool.stats().await;
    let agents = pool.all_agents().await;
    let assigned_count = count_assigned_beads(&agents);

    if assigned_count > stats.total {
        return Err(format!(
            "INVARIANT VIOLATION: assigned_beads ({}) > total_agents ({})",
            assigned_count, stats.total
        ));
    }

    if stats.working > stats.total {
        return Err(format!(
            "INVARIANT VIOLATION: working ({}) > total ({})",
            stats.working, stats.total
        ));
    }

    Ok(())
}

#[tokio::test]
async fn prop_assigned_beads_le_agent_count_empty_pool() -> Result<(), String> {
    let pool = AgentPool::new(PoolConfig::for_testing());
    verify_invariant(&pool).await
}

#[tokio::test]
async fn prop_assigned_beads_le_agent_count_single_agent() -> Result<(), String> {
    let pool = AgentPool::new(PoolConfig::for_testing());
    pool.register_agent(AgentHandle::new("agent-1"))
        .await
        .map_err(|e| e.to_string())?;

    verify_invariant(&pool).await?;

    pool.assign_bead("bead-1")
        .await
        .map_err(|e| format!("assign failed: {}", e))?;
    verify_invariant(&pool).await?;

    pool.complete_bead("agent-1")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await
}

#[tokio::test]
async fn prop_assigned_beads_le_agent_count_multiple_agents() -> Result<(), String> {
    let pool = AgentPool::new(PoolConfig::for_testing());

    for i in 0..5 {
        pool.register_agent(AgentHandle::new(format!("agent-{}", i)))
            .await
            .map_err(|e| e.to_string())?;
        verify_invariant(&pool).await?;
    }

    for i in 0..3 {
        let bead_id = format!("bead-{}", i);
        pool.assign_bead(&bead_id)
            .await
            .map_err(|e| format!("assign {} failed: {}", bead_id, e))?;
        verify_invariant(&pool).await?;
    }

    for i in 0..3 {
        pool.complete_bead(&format!("agent-{}", i))
            .await
            .map_err(|e| e.to_string())?;
        verify_invariant(&pool).await?;
    }

    Ok(())
}

#[tokio::test]
async fn prop_assigned_beads_le_agent_count_exhaust_agents() -> Result<(), String> {
    let pool = AgentPool::new(PoolConfig::for_testing());

    pool.register_agent(AgentHandle::new("agent-a"))
        .await
        .map_err(|e| e.to_string())?;
    pool.register_agent(AgentHandle::new("agent-b"))
        .await
        .map_err(|e| e.to_string())?;

    verify_invariant(&pool).await?;

    pool.assign_bead("bead-1")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await?;

    pool.assign_bead("bead-2")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await?;

    let result = pool.assign_bead("bead-3").await;
    if result.is_ok() {
        return Err("Expected error when no agents available".to_string());
    }

    verify_invariant(&pool).await
}

#[tokio::test]
async fn prop_assigned_beads_le_agent_count_interleaved() -> Result<(), String> {
    let pool = AgentPool::new(PoolConfig::for_testing());

    pool.register_agent(AgentHandle::new("agent-1"))
        .await
        .map_err(|e| e.to_string())?;
    pool.register_agent(AgentHandle::new("agent-2"))
        .await
        .map_err(|e| e.to_string())?;
    pool.register_agent(AgentHandle::new("agent-3"))
        .await
        .map_err(|e| e.to_string())?;

    pool.assign_bead("bead-a")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await?;

    pool.complete_bead("agent-1")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await?;

    pool.assign_bead("bead-b")
        .await
        .map_err(|e| e.to_string())?;
    pool.assign_bead("bead-c")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await?;

    pool.unregister_agent("agent-2")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await?;

    Ok(())
}

#[tokio::test]
async fn prop_assigned_beads_le_agent_count_with_shutdown() -> Result<(), String> {
    let pool = AgentPool::new(PoolConfig::for_testing());

    pool.register_agent(AgentHandle::new("agent-1"))
        .await
        .map_err(|e| e.to_string())?;
    pool.register_agent(AgentHandle::new("agent-2"))
        .await
        .map_err(|e| e.to_string())?;
    pool.register_agent(AgentHandle::new("agent-3"))
        .await
        .map_err(|e| e.to_string())?;

    pool.assign_bead("bead-1")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await?;

    pool.shutdown_agent("agent-3")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await?;

    pool.assign_bead("bead-2")
        .await
        .map_err(|e| e.to_string())?;
    verify_invariant(&pool).await
}

#[tokio::test]
async fn prop_assigned_beads_le_agent_count_varied_sizes() -> Result<(), String> {
    for agent_count in [1, 2, 5, 10].iter() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        for i in 0..*agent_count {
            pool.register_agent(AgentHandle::new(format!("agent-{}", i)))
                .await
                .map_err(|e| e.to_string())?;
        }

        verify_invariant(&pool).await?;

        for i in 0..*agent_count {
            let bead_id = format!("bead-{}", i);
            pool.assign_bead(&bead_id)
                .await
                .map_err(|e| format!("Failed at agent_count={}, bead={}", agent_count, i))?;
            verify_invariant(&pool).await?;
        }

        let stats = pool.stats().await;
        assert_eq!(stats.working, *agent_count);
        assert_eq!(stats.idle, 0);
    }

    Ok(())
}

#[tokio::test]
async fn prop_stats_consistency() -> Result<(), String> {
    let pool = AgentPool::new(PoolConfig::for_testing());

    pool.register_agent(AgentHandle::new("a"))
        .await
        .map_err(|e| e.to_string())?;
    pool.register_agent(AgentHandle::new("b"))
        .await
        .map_err(|e| e.to_string())?;
    pool.register_agent(AgentHandle::new("c"))
        .await
        .map_err(|e| e.to_string())?;

    let stats = pool.stats().await;
    assert_eq!(stats.total, 3);
    assert_eq!(stats.idle, 3);
    assert_eq!(stats.working, 0);

    pool.assign_bead("bead-1")
        .await
        .map_err(|e| e.to_string())?;

    let stats = pool.stats().await;
    assert_eq!(stats.total, 3);
    assert_eq!(stats.idle, 2);
    assert_eq!(stats.working, 1);

    pool.assign_bead("bead-2")
        .await
        .map_err(|e| e.to_string())?;

    let stats = pool.stats().await;
    assert_eq!(stats.total, 3);
    assert_eq!(stats.idle, 1);
    assert_eq!(stats.working, 2);

    assert!(stats.working <= stats.total, "working <= total invariant");

    Ok(())
}
