//! BDD integration tests for FIFO strategy.
//!
//! This module tests the behavior described in bead src-3m3t:
//!
//! ## Phase 2 - BDD Tests
//!
//! GIVEN fifo strategy WHEN multiple ready THEN first selected.

use orchestrator::distribution::fifo::FifoStrategy;
use orchestrator::distribution::strategy::{DistributionContext, DistributionStrategy};

/// BDD Test: FIFO strategy selects first ready bead
///
/// **Given** a FIFO strategy with multiple ready beads
/// **When** selecting a bead from the ready set
/// **Then** the first bead in the ready list is selected
#[test]
fn given_fifo_strategy_when_multiple_ready_then_first_selected() {
    let strategy = FifoStrategy::new();
    let ctx = DistributionContext::new();

    let ready_beads = vec![
        "bead-first".to_string(),
        "bead-second".to_string(),
        "bead-third".to_string(),
    ];

    let result = strategy.select_bead(&ready_beads, &ctx);

    assert_eq!(
        result,
        Some("bead-first".to_string()),
        "FIFO strategy should select the first ready bead"
    );
}

/// BDD Test: FIFO strategy selection order matches input order
///
/// **Given** a FIFO strategy with beads in specific order
/// **When** the input order changes
/// **Then** selection follows the new first element
#[test]
fn given_fifo_strategy_when_order_changes_then_new_first_selected() {
    let strategy = FifoStrategy::new();
    let ctx = DistributionContext::new();

    let ready_beads_v1 = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let ready_beads_v2 = vec!["c".to_string(), "a".to_string(), "b".to_string()];

    assert_eq!(
        strategy.select_bead(&ready_beads_v1, &ctx),
        Some("a".to_string()),
        "FIFO should select 'a' when it's first"
    );

    assert_eq!(
        strategy.select_bead(&ready_beads_v2, &ctx),
        Some("c".to_string()),
        "FIFO should select 'c' when it's first"
    );
}

/// BDD Test: FIFO strategy handles single ready bead
///
/// **Given** a FIFO strategy with exactly one ready bead
/// **When** selecting a bead
/// **Then** that single bead is selected
#[test]
fn given_fifo_strategy_when_single_ready_then_that_selected() {
    let strategy = FifoStrategy::new();
    let ctx = DistributionContext::new();

    let ready_beads = vec!["only-bead".to_string()];

    let result = strategy.select_bead(&ready_beads, &ctx);

    assert_eq!(
        result,
        Some("only-bead".to_string()),
        "FIFO strategy should select the single ready bead"
    );
}

/// BDD Test: FIFO strategy returns None for empty ready set
///
/// **Given** a FIFO strategy with no ready beads
/// **When** selecting a bead
/// **Then** None is returned
#[test]
fn given_fifo_strategy_when_no_ready_then_none_selected() {
    let strategy = FifoStrategy::new();
    let ctx = DistributionContext::new();

    let ready_beads: Vec<String> = vec![];

    let result = strategy.select_bead(&ready_beads, &ctx);

    assert!(
        result.is_none(),
        "FIFO strategy should return None when no beads are ready"
    );
}
