#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(unsafe_code)]

//! Restart strategies for supervisor actor management.

use im::HashSet;
use thiserror::Error;

use super::supervisor_actor::{GenericSupervisableActor, SupervisorActorState, SupervisorConfig};

/// Context provided to restart strategies for decision-making.
#[derive(Debug, Clone)]
pub struct RestartContext<'a, A: GenericSupervisableActor>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    /// Name of the child that failed.
    pub child_name: String,
    /// Reason for failure.
    pub failure_reason: String,
    /// Reference to supervisor state (immutable).
    pub state: &'a SupervisorActorState<A>,
}

impl<'a, A: GenericSupervisableActor> RestartContext<'a, A>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
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
pub trait RestartStrategy<A: GenericSupervisableActor>: Send + Sync
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    /// Get strategy name.
    fn name(&self) -> &'static str;

    /// Determine what to do when a child fails.
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
#[derive(Debug, Clone, Default)]
pub struct OneForOne;

impl OneForOne {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<A: GenericSupervisableActor> RestartStrategy<A> for OneForOne
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    fn name(&self) -> &'static str {
        "one_for_one"
    }

    fn on_child_failure(&self, ctx: &RestartContext<'_, A>) -> RestartDecision {
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
#[derive(Debug, Clone, Default)]
pub struct OneForAll;

impl OneForAll {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<A: GenericSupervisableActor> RestartStrategy<A> for OneForAll
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    fn name(&self) -> &'static str {
        "one_for_all"
    }

    fn on_child_failure(&self, ctx: &RestartContext<'_, A>) -> RestartDecision {
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
#[derive(Debug, Clone, Default)]
pub struct RestForOne {
    /// Names of children that depend on each child.
    dependencies: HashSet<String>,
}

impl RestForOne {
    #[must_use]
    pub fn new() -> Self {
        Self {
            dependencies: HashSet::new(),
        }
    }

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

impl<A: GenericSupervisableActor> RestartStrategy<A> for RestForOne
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Clone + Send,
{
    fn name(&self) -> &'static str {
        "rest_for_one"
    }

    fn on_child_failure(&self, ctx: &RestartContext<'_, A>) -> RestartDecision {
        if ctx.is_max_restarts_exceeded() {
            return RestartDecision::Stop;
        }

        let prefix = format!("{}:", ctx.child_name);
        let dependents: Vec<String> = self
            .dependencies
            .iter()
            .filter(|dep| dep.starts_with(&prefix))
            .filter_map(|dep| dep.split(':').nth(1))
            .map(String::from)
            .collect();

        let mut children_to_restart = vec![ctx.child_name.clone()];
        children_to_restart.extend(dependents);

        RestartDecision::Restart {
            child_names: children_to_restart,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::messages::SchedulerMessage;
    use crate::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
    use crate::actors::supervisor::{SupervisorConfig, SupervisorState};
    use std::time::Instant;

    fn create_test_state() -> SupervisorActorState<SchedulerActorDef> {
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

    fn create_child_info(
        name: &str,
        restart_count: u32,
    ) -> crate::actors::supervisor::ChildInfo<SchedulerActorDef> {
        crate::actors::supervisor::ChildInfo {
            name: name.to_string(),
            actor_ref: create_test_actor_ref(name),
            restart_count,
            last_restart: Some(Instant::now()),
            args: SchedulerArguments::default(),
        }
    }

    fn create_test_actor_ref(name: &str) -> ractor::ActorRef<SchedulerMessage> {
        use ractor::Actor;
        use tokio::runtime::Runtime;

        let rt = Runtime::new().expect("Runtime creation should succeed");

        rt.block_on(async move {
            let (ref_, _handle) = Actor::spawn(
                Some(format!("test-dummy-{}", name)),
                SchedulerActorDef,
                SchedulerArguments::default(),
            )
            .await
            .expect("Actor spawn should succeed");

            Box::leak(Box::new(ref_)).clone()
        })
    }

    #[test]
    fn test_one_for_one_name() {
        let strategy = OneForOne::new();
        assert_eq!(
            RestartStrategy::<SchedulerActorDef>::name(&strategy),
            "one_for_one"
        );
    }

    #[test]
    fn test_one_for_all_name() {
        let strategy = OneForAll::new();
        assert_eq!(
            RestartStrategy::<SchedulerActorDef>::name(&strategy),
            "one_for_all"
        );
    }

    #[test]
    fn test_rest_for_one_name() {
        let strategy = RestForOne::new();
        assert_eq!(
            RestartStrategy::<SchedulerActorDef>::name(&strategy),
            "rest_for_one"
        );
    }

    #[test]
    fn test_restart_context_fields() {
        let mut state = create_test_state();
        state
            .children
            .insert("child-1".to_string(), create_child_info("child-1", 5));

        let ctx = RestartContext::new("child-1", "Test failure", &state);
        assert_eq!(ctx.child_name, "child-1");
        assert_eq!(ctx.failure_reason, "Test failure");
        assert_eq!(ctx.restart_count(), 5);
    }
}
