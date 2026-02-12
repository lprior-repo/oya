//! BDD integration tests for priority strategy.
//!
//! This module tests the behavior described in bead src-8a7n:
//!
//! ## Phase 2 - BDD Tests
//!
//! GIVEN priority strategy WHEN mixed priorities THEN high selected first.

use orchestrator::distribution::{
    AgentMetadata, BeadMetadata, DistributionContext, DistributionStrategy, PriorityStrategy,
};

/// BDD Test: Priority strategy selects highest priority first
///
/// **Given** a priority strategy with beads of mixed priorities
/// **When** selecting a bead from the ready set
/// **Then** the bead with the highest priority is selected first
#[test]
fn given_priority_strategy_when_mixed_priorities_then_high_selected_first() {
    let strategy = PriorityStrategy::new();
    let ctx = DistributionContext::new()
        .with_bead(BeadMetadata::new("low-priority").with_priority(1))
        .with_bead(BeadMetadata::new("high-priority").with_priority(100))
        .with_bead(BeadMetadata::new("medium-priority").with_priority(50));

    let ready_beads = vec![
        "low-priority".to_string(),
        "high-priority".to_string(),
        "medium-priority".to_string(),
    ];

    let result = strategy.select_bead(&ready_beads, &ctx);

    assert_eq!(
        result,
        Some("high-priority".to_string()),
        "Priority strategy should select bead with highest priority (100)"
    );
}

/// BDD Test: Priority strategy handles equal priorities
///
/// **Given** a priority strategy with beads of equal priorities
/// **When** selecting a bead multiple times
/// **Then** selection is stable (same bead returned)
#[test]
fn given_priority_strategy_when_equal_priorities_then_stable_selection() {
    let strategy = PriorityStrategy::new();
    let ctx = DistributionContext::new()
        .with_bead(BeadMetadata::new("bead-a").with_priority(50))
        .with_bead(BeadMetadata::new("bead-b").with_priority(50))
        .with_bead(BeadMetadata::new("bead-c").with_priority(50));

    let ready_beads = vec![
        "bead-a".to_string(),
        "bead-b".to_string(),
        "bead-c".to_string(),
    ];

    let result1 = strategy.select_bead(&ready_beads, &ctx);
    let result2 = strategy.select_bead(&ready_beads, &ctx);

    assert_eq!(
        result1, result2,
        "Priority strategy should provide stable selection for equal priorities"
    );
}

/// BDD Test: Priority strategy selects agent with lowest load
///
/// **Given** a priority strategy with agents of mixed loads
/// **When** selecting an agent for a bead
/// **Then** the agent with the lowest load is selected
#[test]
fn given_priority_strategy_when_mixed_agent_loads_then_lowest_load_selected() {
    let strategy = PriorityStrategy::new();
    let ctx = DistributionContext::new()
        .with_agent(AgentMetadata::new("busy-agent").with_load(0.9))
        .with_agent(AgentMetadata::new("idle-agent").with_load(0.1))
        .with_agent(AgentMetadata::new("medium-agent").with_load(0.5));

    let agents = vec![
        "busy-agent".to_string(),
        "idle-agent".to_string(),
        "medium-agent".to_string(),
    ];

    let result = strategy.select_agent("bead-1", &agents, &ctx);

    assert_eq!(
        result,
        Some("idle-agent".to_string()),
        "Priority strategy should select agent with lowest load (0.1)"
    );
}

/// BDD Test: Priority strategy respects capability requirements
///
/// **Given** a priority strategy with capability matching enabled
/// **When** selecting an agent for a bead with requirements
/// **Then** only agents with matching capabilities are considered
#[test]
fn given_priority_strategy_when_capability_required_then_matching_agent_selected() {
    let strategy = PriorityStrategy::new().with_capability_matching(true);
    let ctx = DistributionContext::new()
        .with_bead(BeadMetadata::new("rust-bead").with_capability("rust"))
        .with_agent(
            AgentMetadata::new("python-agent")
                .with_capability("python")
                .with_load(0.1),
        )
        .with_agent(
            AgentMetadata::new("rust-agent")
                .with_capability("rust")
                .with_load(0.5),
        );

    let agents = vec!["python-agent".to_string(), "rust-agent".to_string()];

    let result = strategy.select_agent("rust-bead", &agents, &ctx);

    assert_eq!(
        result,
        Some("rust-agent".to_string()),
        "Priority strategy should select agent with matching capability"
    );
}

/// BDD Test: Priority strategy handles negative priorities
///
/// **Given** a priority strategy with negative priority values
/// **When** selecting a bead
/// **Then** the least negative (highest) priority is selected
#[test]
fn given_priority_strategy_when_negative_priorities_then_highest_value_selected() {
    let strategy = PriorityStrategy::new();
    let ctx = DistributionContext::new()
        .with_bead(BeadMetadata::new("very-low").with_priority(-100))
        .with_bead(BeadMetadata::new("low").with_priority(-10))
        .with_bead(BeadMetadata::new("less-low").with_priority(-1));

    let ready_beads = vec![
        "very-low".to_string(),
        "low".to_string(),
        "less-low".to_string(),
    ];

    let result = strategy.select_bead(&ready_beads, &ctx);

    assert_eq!(
        result,
        Some("less-low".to_string()),
        "Priority strategy should select bead with highest priority value (-1 > -10 > -100)"
    );
}
