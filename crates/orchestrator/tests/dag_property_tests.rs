//! Property-based tests for bead selection strategies.
//!
//! This module tests the property described in bead src-209z:
//!
//! ## Phase 5 - Property Tests
//!
//! ∀ distribution: selected bead ∈ ready beads

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::string::string_regex;

use orchestrator::distribution::{
    AffinityStrategy, DistributionContext, DistributionStrategy, FifoStrategy, PriorityStrategy,
    RoundRobinStrategy, StickyStrategy, available_strategies, create_strategy,
};

fn bead_id_strategy() -> impl Strategy<Value = String> {
    string_regex("bead-[a-z0-9]{3,8}").unwrap()
}

fn ready_beads_strategy() -> impl Strategy<Value = Vec<String>> {
    vec(bead_id_strategy(), 1..20)
}

fn all_strategies() -> Vec<Box<dyn DistributionStrategy>> {
    available_strategies()
        .iter()
        .filter_map(|name| create_strategy(name))
        .collect()
}

proptest! {
    #[test]
    fn prop_selected_bead_in_ready_beads_fifo(ready_beads in ready_beads_strategy()) {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        let selected = strategy.select_bead(&ready_beads, &ctx);

        if let Some(ref bead) = selected {
            prop_assert!(
                ready_beads.contains(bead),
                "FIFO selected bead '{}' must be in ready beads {:?}",
                bead,
                ready_beads
            );
        } else {
            prop_assert!(ready_beads.is_empty(), "FIFO should only return None for empty input");
        }
    }

    #[test]
    fn prop_selected_bead_in_ready_beads_priority(ready_beads in ready_beads_strategy()) {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new();

        let selected = strategy.select_bead(&ready_beads, &ctx);

        if let Some(ref bead) = selected {
            prop_assert!(
                ready_beads.contains(bead),
                "Priority selected bead '{}' must be in ready beads {:?}",
                bead,
                ready_beads
            );
        } else {
            prop_assert!(ready_beads.is_empty(), "Priority should only return None for empty input");
        }
    }

    #[test]
    fn prop_selected_bead_in_ready_beads_round_robin(ready_beads in ready_beads_strategy()) {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let selected = strategy.select_bead(&ready_beads, &ctx);

        if let Some(ref bead) = selected {
            prop_assert!(
                ready_beads.contains(bead),
                "RoundRobin selected bead '{}' must be in ready beads {:?}",
                bead,
                ready_beads
            );
        } else {
            prop_assert!(ready_beads.is_empty(), "RoundRobin should only return None for empty input");
        }
    }

    #[test]
    fn prop_selected_bead_in_ready_beads_affinity_soft(ready_beads in ready_beads_strategy()) {
        let strategy = AffinityStrategy::soft();
        let ctx = DistributionContext::new();

        let selected = strategy.select_bead(&ready_beads, &ctx);

        if let Some(ref bead) = selected {
            prop_assert!(
                ready_beads.contains(bead),
                "AffinitySoft selected bead '{}' must be in ready beads {:?}",
                bead,
                ready_beads
            );
        } else {
            prop_assert!(ready_beads.is_empty(), "AffinitySoft should only return None for empty input");
        }
    }

    #[test]
    fn prop_selected_bead_in_ready_beads_affinity_hard(ready_beads in ready_beads_strategy()) {
        let strategy = AffinityStrategy::hard();
        let ctx = DistributionContext::new();

        let selected = strategy.select_bead(&ready_beads, &ctx);

        if let Some(ref bead) = selected {
            prop_assert!(
                ready_beads.contains(bead),
                "AffinityHard selected bead '{}' must be in ready beads {:?}",
                bead,
                ready_beads
            );
        } else {
            prop_assert!(ready_beads.is_empty(), "AffinityHard should only return None for empty input");
        }
    }

    #[test]
    fn prop_selected_bead_in_ready_beads_sticky(ready_beads in ready_beads_strategy()) {
        let strategy = StickyStrategy::new();
        let ctx = DistributionContext::new();

        let selected = strategy.select_bead(&ready_beads, &ctx);

        if let Some(ref bead) = selected {
            prop_assert!(
                ready_beads.contains(bead),
                "Sticky selected bead '{}' must be in ready beads {:?}",
                bead,
                ready_beads
            );
        } else {
            prop_assert!(ready_beads.is_empty(), "Sticky should only return None for empty input");
        }
    }

    #[test]
    fn prop_selected_bead_in_ready_beads_all_strategies(ready_beads in ready_beads_strategy()) {
        let strategies = all_strategies();
        let ctx = DistributionContext::new();

        for strategy in &strategies {
            let selected = strategy.select_bead(&ready_beads, &ctx);

            if let Some(ref bead) = selected {
                prop_assert!(
                    ready_beads.contains(bead),
                    "Strategy '{}' selected bead '{}' must be in ready beads {:?}",
                    strategy.name(),
                    bead,
                    ready_beads
                );
            } else {
                prop_assert!(
                    ready_beads.is_empty(),
                    "Strategy '{}' should only return None for empty input",
                    strategy.name()
                );
            }
        }
    }

    #[test]
    fn prop_empty_ready_beads_returns_none_all_strategies(
        strategy_name in proptest::sample::select(available_strategies().to_vec())
    ) {
        let strategy = create_strategy(&strategy_name);
        let ctx = DistributionContext::new();
        let empty_beads: Vec<String> = vec![];

        if let Some(s) = strategy {
            let selected = s.select_bead(&empty_beads, &ctx);
            prop_assert!(
                selected.is_none(),
                "Strategy '{}' should return None for empty ready beads",
                s.name()
            );
        }
    }

    #[test]
    fn prop_single_bead_always_selected(
        strategy_name in proptest::sample::select(available_strategies().to_vec()),
        bead_id in bead_id_strategy()
    ) {
        let strategy = create_strategy(&strategy_name);
        let ctx = DistributionContext::new();
        let single_bead = vec![bead_id.clone()];

        if let Some(s) = strategy {
            let selected = s.select_bead(&single_bead, &ctx);
            prop_assert_eq!(
                selected,
                Some(bead_id),
                "Strategy '{}' should select the single available bead",
                s.name()
            );
        }
    }

    #[test]
    fn prop_deterministic_selection_same_input(
        strategy_name in proptest::sample::select(available_strategies().to_vec()),
        ready_beads in ready_beads_strategy()
    ) {
        let strategy1 = create_strategy(&strategy_name);
        let strategy2 = create_strategy(&strategy_name);
        let ctx = DistributionContext::new();

        if let (Some(s1), Some(s2)) = (strategy1, strategy2) {
            let selected1 = s1.select_bead(&ready_beads, &ctx);
            let selected2 = s2.select_bead(&ready_beads, &ctx);

            prop_assert_eq!(
                selected1.clone(), selected2.clone(),
                "Strategy '{}' should be deterministic: got {:?} then {:?}",
                s1.name(),
                selected1,
                selected2
            );
        }
    }
}
