#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(unsafe_code)]

//! Restart strategies for supervisor actor management.
//!
//! This module defines a pluggable strategy pattern for how supervisors
//! handle child actor failures. Each strategy determines which actors to restart
//! when a child crashes.
//!
//! # Strategy Types
//!
//! - **`OneForOne`**: When a child crashes, restart only that child (siblings unaffected)
//! - **`OneForAll`**: When a child crashes, restart all children
//! - **`RestForOne`**: When a child crashes, restart it and other children that depend on it
//! - **`OneForOnePermanently`**: Like `OneForOne`, but restarts permanently (no `max_restarts`)
//!
//! # Example
//!
//! ```ignore
//! use orchestrator::actors::supervisor::strategy::{
//!     RestartStrategy, RestartContext, OneForOne,
//! };
//!
//! let strategy = OneForOne::new();
//! let context = RestartContext::new()
//!     .with_child_name("scheduler-1")
//!     .with_failure_reason("Actor panicked");
//!
//! let decision = strategy.on_child_failure(&context);
//! match decision {
//!     RestartDecision::Restart { child_names } => {
//!         println!("Restarting: {:?}", child_names);
//!     }
//!     RestartDecision::Stop => {
//!         println!("Stopping supervisor");
//!     }
//! }
//! ```

use std::collections::HashSet;
use ractor::Actor;
use thiserror::Error;

use super::{SupervisorActorState, SupervisorConfig};

/// Context provided to restart strategies for decision-making.
#[derive(Debug, Clone)]
pub struct RestartContext<'a, A: Actor> {
    /// Name of the child that failed.
    pub child_name: String,
    /// Reason for failure.
    pub failure_reason: String,
    /// Reference to supervisor state (immutable).
    pub state: &'a SupervisorActorState<A>,
}

impl<'a, A: Actor> RestartContext<'a, A> {
    /// Create a new restart context.
    #[must_use]
    pub fn new(
        child_name: impl Into<String>,
        failure_reason: impl Into<String>,
        state: &'a SupervisorActorState<A>,
    ) -> Self {
        Self {
            child_name: child_name.into(),
            failure_reason: failure_reason.into(),
            state,
        }
    }

    /// Get restart count for the failed child.
    #[must_use]
    pub fn restart_count(&self) -> u32 {
        self.state
            .children
            .get(&self.child_name)
            .map_or(0, |c| c.restart_count)
    }

    /// Get all child names.
    #[must_use]
    pub fn all_children(&self) -> Vec<String> {
        self.state.children.keys().cloned().collect()
    }

    /// Check if child has reached max restarts.
    #[must_use]
    pub fn is_max_restarts_exceeded(&self) -> bool {
        self.restart_count() >= self.state.config.max_restarts
    }

    /// Get time since last restart for the failed child.
    #[must_use]
    pub fn time_since_last_restart(&self) -> Option<std::time::Duration> {
        self.state
            .children
            .get(&self.child_name)
            .and_then(|c| c.last_restart)
            .map(|t: std::time::Instant| t.elapsed())
    }
}

/// Decision returned by restart strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartDecision {
    /// Restart specified children.
    Restart {
        /// Names of children to restart.
        child_names: Vec<String>,
    },
    /// Stop the supervisor (do not restart).
    Stop,
}

/// Trait for restart strategies.
///
/// Strategies determine which children to restart when a child crashes.
pub trait RestartStrategy<A: Actor>: Send + Sync {
    /// Get strategy name.
    fn name(&self) -> &'static str;

    /// Determine what to do when a child fails.
    ///
    /// Returns a decision indicating which children to restart or whether to stop.
    fn on_child_failure(&self, ctx: &RestartContext<'_, A>) -> RestartDecision;

    /// Validate strategy configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the strategy configuration is invalid.
    fn validate(&self) -> Result<(), StrategyError> {
        Ok(())
    }
}

/// Error type for strategy operations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StrategyError {
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// One-for-one restart strategy.
///
/// When a child crashes, restart only that child. Siblings are unaffected.
#[derive(Debug, Clone, Default)]
pub struct OneForOne;

impl OneForOne {
    /// Create a new one-for-one strategy.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<A: Actor> RestartStrategy<A> for OneForOne {
    fn name(&self) -> &'static str {
        "one_for_one"
    }

    fn on_child_failure(&self, ctx: &RestartContext<'_, A>) -> RestartDecision {
        // One-for-one: restart only the crashed child
        if ctx.is_max_restarts_exceeded() {
            RestartDecision::Stop
        } else {
            RestartDecision::Restart {
                child_names: vec![ctx.child_name.clone()],
            }
        }
    }
}

/// One-for-all restart strategy.
///
/// When a child crashes, restart all children.
#[derive(Debug, Clone, Default)]
pub struct OneForAll;

impl OneForAll {
    /// Create a new one-for-all strategy.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<A: Actor> RestartStrategy<A> for OneForAll {
    fn name(&self) -> &'static str {
        "one_for_all"
    }

    fn on_child_failure(&self, ctx: &RestartContext<'_, A>) -> RestartDecision {
        // One-for-all: restart all children
        if ctx.is_max_restarts_exceeded() {
            RestartDecision::Stop
        } else {
            RestartDecision::Restart {
                child_names: ctx.all_children(),
            }
        }
    }
}

/// Rest-for-one restart strategy.
///
/// When a child crashes, restart it and other children that depend on it.
#[derive(Debug, Clone, Default)]
pub struct RestForOne {
    /// Names of children that depend on each child.
    dependencies: HashSet<String>,
}

impl RestForOne {
    /// Create a new rest-for-one strategy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dependencies: HashSet::new(),
        }
    }

    /// Add a dependency: when `parent` crashes, restart `dependent`.
    #[must_use]
    pub fn with_dependency(
        mut self,
        parent: impl Into<String>,
        dependent: impl Into<String>,
    ) -> Self {
        self.dependencies
            .insert(format!("{}:{}", parent.into(), dependent.into()));
        self
    }
}

impl<A: Actor> RestartStrategy<A> for RestForOne {
    fn name(&self) -> &'static str {
        "rest_for_one"
    }

    fn on_child_failure(&self, ctx: &RestartContext<'_, A>) -> RestartDecision {
        // Check if max restarts exceeded for this child
        if ctx.is_max_restarts_exceeded() {
            return RestartDecision::Stop;
        }

        // Find all children that depend on the crashed one
        let prefix = format!("{}:", ctx.child_name);
        let dependents: Vec<String> = self
            .dependencies
            .iter()
            .filter(|dep| dep.starts_with(&prefix))
            .filter_map(|dep| dep.split(':').nth(1))
            .map(String::from)
            .collect();

        // Restart crashed child and its dependents
        let mut children_to_restart = vec![ctx.child_name.clone()];
        children_to_restart.extend(dependents);

        RestartDecision::Restart {
            child_names: children_to_restart,
        }
    }
}

// ============================================================================
// RESTART STRATEGY TESTS
// ============================================================================

// These tests verify restart strategy logic using pure function testing.
// No actors are spawned - we test strategy decision-making in isolation.
//
// Design principle: Strategies are pure functions of (context) -> decision.
// The ActorRef is only stored in state, but strategies don't need to use it.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::supervisor::{
        SupervisorActorState, SupervisorConfig, SupervisorState,
    };
    use std::time::Instant;

    // HELPER: Create a minimal SupervisorActorState for testing
    fn create_test_state() -> SupervisorActorState<crate::actors::scheduler::SchedulerActorDef> {
        SupervisorActorState {
            config: SupervisorConfig::default(),
            state: SupervisorState::Running,
            children: std::collections::HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
        }
    }

    // HELPER: Create a minimal ChildInfo for testing
    // Note: We don't need real ActorRef for testing strategy logic
    // The strategies only access restart_count, last_restart, and config
    fn create_child_info(name: &str, restart_count: u32) -> crate::actors::supervisor::ChildInfo<crate::actors::scheduler::SchedulerActorDef> {
        crate::actors::supervisor::ChildInfo {
            name: name.to_string(),
            actor_ref: create_test_actor_ref(name),
            restart_count,
            last_restart: Some(Instant::now()),
            args: crate::actors::scheduler::SchedulerArguments::new(),
        }
    }

    // HELPER: Create a placeholder ActorRef for testing
    // Strategy tests only need the ChildInfo struct, not a functional actor reference
    fn create_test_actor_ref(name: &str) -> ractor::ActorRef<crate::actors::scheduler::SchedulerMessage> {
        use tokio::runtime::Runtime;
        use ractor::{Actor, ActorProcessingErr, ActorRef};
        use crate::actors::scheduler::{SchedulerActorDef, SchedulerArguments};

        // Create a minimal async runtime for spawning test actors
        let rt = Runtime::new().expect("Runtime creation should succeed");

        rt.block_on(async move {
            // Spawn a real scheduler actor to get a valid ActorRef
            // We'll never use it to send messages - just need the reference
            let (ref_, _handle) = Actor::spawn(
                Some(format!("test-dummy-{}", name)),
                SchedulerActorDef,
                SchedulerArguments::default(),
            ).await.expect("Scheduler actor spawn should succeed");

            // Leak the ActorRef to return it from async context
            // This is acceptable for test-only code where we never clean up
            Box::leak(Box::new(ref_)).clone()
        })
    }

    // ========================================================================
    // ONE-FOR-ONE STRATEGY TESTS
    // ========================================================================

    #[test]
    fn given_one_for_one_strategy_when_get_name_then_returns_correct_name() {
        let strategy = OneForOne::new();

        assert_eq!(RestartStrategy::<crate::actors::scheduler::SchedulerActorDef>::name(&strategy), "one_for_one");
    }

    #[test]
    fn given_one_for_one_when_child_fails_then_restart_only_failed_child() {
        let strategy = OneForOne::new();
        let mut state = create_test_state();

        // GIVEN: Three children running
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 0));
        state
            .children
            .insert("child-2".to_string(), create_child_info("child-2", 0));
        state
            .children
            .insert("child-3".to_string(), create_child_info("child-3", 0));

        // WHEN: Child-2 fails
        let ctx = RestartContext::new("child-2", "Test failure", &state);
        let decision = strategy.on_child_failure(&ctx);

        // THEN: Only child-2 should be restarted
        match decision {
            RestartDecision::Restart { child_names } => {
                assert_eq!(child_names, vec!["child-2".to_string()]);
            }
            RestartDecision::Stop => {
                panic!("Expected Restart decision, got Stop");
            }
        }

        // Verify other children unchanged
        assert_eq!(state.children.get("child-1").unwrap().restart_count, 0);
        assert_eq!(state.children.get("child-2").unwrap().restart_count, 0);
        assert_eq!(state.children.get("child-3").unwrap().restart_count, 0);
    }

    #[test]
    fn given_one_for_one_when_child_exceeds_max_restarts_then_stop() {
        let strategy = OneForOne::new();
        let mut state = create_test_state();
        state.config.max_restarts = 3;

        // GIVEN: Child has exceeded max restarts
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 5));

        // WHEN: Child fails again
        let ctx = RestartContext::new("child-1", "Test failure", &state);
        let decision = strategy.on_child_failure(&ctx);

        // THEN: Should stop (no restart)
        assert_eq!(decision, RestartDecision::Stop);
    }

    #[test]
    fn given_one_for_one_when_child_within_max_restarts_then_restart() {
        let strategy = OneForOne::new();
        let mut state = create_test_state();
        state.config.max_restarts = 10;

        // GIVEN: Child has NOT exceeded max restarts
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 5));

        // WHEN: Child fails
        let ctx = RestartContext::new("child-1", "Test failure", &state);
        let decision = strategy.on_child_failure(&ctx);

        // THEN: Should restart
        match decision {
            RestartDecision::Restart { child_names } => {
                assert_eq!(child_names, vec!["child-1".to_string()]);
            }
            RestartDecision::Stop => {
                panic!("Expected Restart decision, got Stop");
            }
        }
    }

    // ========================================================================
    // ONE-FOR-ALL STRATEGY TESTS
    // ========================================================================

    #[test]
    fn given_one_for_all_when_get_name_then_returns_correct_name() {
        let strategy = OneForAll::new();

        assert_eq!(RestartStrategy::<crate::actors::scheduler::SchedulerActorDef>::name(&strategy), "one_for_all");
    }

    #[test]
    fn given_one_for_all_when_child_fails_then_restart_all_children() {
        let strategy = OneForAll::new();
        let mut state = create_test_state();

        // GIVEN: Three children running
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 0));
        state
            .children
            .insert("child-2".to_string(), create_child_info("child-2", 0));
        state
            .children
            .insert("child-3".to_string(), create_child_info("child-3", 0));

        // WHEN: Child-2 fails
        let ctx = RestartContext::new("child-2", "Test failure", &state);
        let decision = strategy.on_child_failure(&ctx);

        // THEN: All children should be restarted
        match decision {
            RestartDecision::Restart { child_names } => {
                let mut expected_names = vec!["child-1".to_string(), "child-2".to_string(), "child-3".to_string()];
                expected_names.sort();
                let mut actual_names = child_names.clone();
                actual_names.sort();
                assert_eq!(actual_names, expected_names);
            }
            RestartDecision::Stop => {
                panic!("Expected Restart decision, got Stop");
            }
        }
    }

    #[test]
    fn given_one_for_all_when_any_child_exceeds_max_restarts_then_stop() {
        let strategy = OneForAll::new();
        let mut state = create_test_state();
        state.config.max_restarts = 3;

        // GIVEN: One child has exceeded max restarts (but others are fine)
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 5));
        state
            .children
            .insert("child-2".to_string(), create_child_info("child-2", 0));
        state
            .children
            .insert("child-3".to_string(), create_child_info("child-3", 0));

        // WHEN: Child-1 fails
        let ctx = RestartContext::new("child-1", "Test failure", &state);
        let decision = strategy.on_child_failure(&ctx);

        // THEN: Should stop (one_for_all stops if ANY child exceeds limit)
        assert_eq!(decision, RestartDecision::Stop);
    }

    // ========================================================================
    // REST-FOR-ONE STRATEGY TESTS
    // ========================================================================

    #[test]
    fn given_rest_for_one_when_get_name_then_returns_correct_name() {
        let strategy = RestForOne::new();

        assert_eq!(RestartStrategy::<crate::actors::scheduler::SchedulerActorDef>::name(&strategy), "rest_for_one");
    }

    #[test]
    fn given_rest_for_one_when_child_fails_without_deps_then_restart_only_child() {
        let strategy = RestForOne::new();
        let mut state = create_test_state();

        // GIVEN: Three children with no dependencies configured
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 0));
        state
            .children
            .insert("child-2".to_string(), create_child_info("child-2", 0));
        state
            .children
            .insert("child-3".to_string(), create_child_info("child-3", 0));

        // WHEN: Child-2 fails
        let ctx = RestartContext::new("child-2", "Test failure", &state);
        let decision = strategy.on_child_failure(&ctx);

        // THEN: Only child-2 should be restarted (no dependents)
        match decision {
            RestartDecision::Restart { child_names } => {
                assert_eq!(child_names, vec!["child-2".to_string()]);
            }
            RestartDecision::Stop => {
                panic!("Expected Restart decision, got Stop");
            }
        }
    }

    #[test]
    fn given_rest_for_one_when_child_fails_with_dependents_then_restart_child_and_dependents() {
        let strategy = RestForOne::new()
            .with_dependency("parent-1", "child-1")
            .with_dependency("parent-1", "child-2")
            .with_dependency("parent-2", "child-3");
        let mut state = create_test_state();

        // GIVEN: Children with dependencies configured
        state
            .children
            .insert("parent-1".to_string(), create_child_info("parent-1", 0));
        state
            .children
            .insert("parent-2".to_string(), create_child_info("parent-2", 0));
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 0));
        state
            .children
            .insert("child-2".to_string(), create_child_info("child-2", 0));
        state
            .children
            .insert("child-3".to_string(), create_child_info("child-3", 0));

        // WHEN: parent-1 fails
        let ctx = RestartContext::new("parent-1", "Test failure", &state);
        let decision = strategy.on_child_failure(&ctx);

        // THEN: parent-1 AND its dependents (child-1, child-2) should restart
        match decision {
            RestartDecision::Restart { child_names } => {
                let mut expected = vec!["parent-1".to_string(), "child-1".to_string(), "child-2".to_string()];
                expected.sort();
                let mut actual = child_names.clone();
                actual.sort();
                assert_eq!(actual, expected);
            }
            RestartDecision::Stop => {
                panic!("Expected Restart decision, got Stop");
            }
        }
    }

    #[test]
    fn given_rest_for_one_when_child_exceeds_max_restarts_then_stop() {
        let strategy = RestForOne::new();
        let mut state = create_test_state();
        state.config.max_restarts = 3;

        // GIVEN: Child has exceeded max restarts
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 5));

        // WHEN: Child fails again
        let ctx = RestartContext::new("child-1", "Test failure", &state);
        let decision = strategy.on_child_failure(&ctx);

        // THEN: Should stop (no restart, even with dependents)
        assert_eq!(decision, RestartDecision::Stop);
    }

    // ========================================================================
    // RESTART CONTEXT TESTS
    // ========================================================================

    #[test]
    fn given_restart_context_when_get_child_name_then_returns_correct_name() {
        let mut state = create_test_state();
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 0));

        let ctx = RestartContext::new("child-1", "Test failure", &state);
        assert_eq!(ctx.child_name, "child-1");
    }

    #[test]
    fn given_restart_context_when_get_reason_then_returns_correct_reason() {
        let mut state = create_test_state();
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 0));

        let ctx = RestartContext::new("child-1", "Test failure", &state);
        assert_eq!(ctx.failure_reason, "Test failure");
    }

    #[test]
    fn given_restart_context_when_get_restart_count_then_returns_child_restart_count() {
        let mut state = create_test_state();
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 5));

        let ctx = RestartContext::new("child-1", "Test failure", &state);
        assert_eq!(ctx.restart_count(), 5);
    }

    #[test]
    fn given_restart_context_when_get_all_children_then_returns_all_child_names() {
        let mut state = create_test_state();
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 0));
        state
            .children
            .insert("child-2".to_string(), create_child_info("child-2", 0));

        let ctx = RestartContext::new("child-1", "Test", &state);
        let all_children = ctx.all_children();

        assert_eq!(all_children.len(), 2);
        assert!(all_children.contains(&"child-1".to_string()));
        assert!(all_children.contains(&"child-2".to_string()));
    }

    #[test]
    fn given_restart_context_when_check_max_restarts_exceeded_then_returns_true_if_exceeded() {
        let mut state = create_test_state();
        state.config.max_restarts = 5;
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 5));

        let ctx = RestartContext::new("child-1", "Test", &state);
        assert!(ctx.is_max_restarts_exceeded());
    }

    #[test]
    fn given_restart_context_when_check_max_restarts_not_exceeded_then_returns_false() {
        let mut state = create_test_state();
        state.config.max_restarts = 10;
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 5));

        let ctx = RestartContext::new("child-1", "Test", &state);
        assert!(!ctx.is_max_restarts_exceeded());
    }

    #[test]
    fn given_restart_context_when_get_time_since_last_restart_then_returns_duration() {
        let mut state = create_test_state();
        let now = Instant::now();
        let child_info = create_child_info("child-1", 0);
        let mut child_info_with_time = child_info;
        child_info_with_time.last_restart = Some(now);

        state
            .children
            .insert("child-1".to_string(), child_info_with_time);

        let ctx = RestartContext::new("child-1", "Test", &state);

        // Allow some time to pass
        std::thread::sleep(std::time::Duration::from_millis(10));

        let elapsed = ctx.time_since_last_restart();
        assert!(elapsed.is_some());
        assert!(elapsed.unwrap().as_millis() >= 10);
    }
}

