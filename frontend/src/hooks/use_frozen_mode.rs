#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::graph::{NodeId, RunRecord};
use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum FrozenMode {
    #[default]
    Live,
    Frozen {
        run_id: Uuid,
        results: HashMap<NodeId, serde_json::Value>,
    },
}

impl FrozenMode {
    #[allow(dead_code)]
    #[must_use]
    pub const fn live() -> Self {
        Self::Live
    }

    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn frozen(run_id: Uuid, results: HashMap<NodeId, serde_json::Value>) -> Self {
        Self::Frozen { run_id, results }
    }

    #[must_use]
    pub const fn is_frozen(&self) -> bool {
        matches!(self, Self::Frozen { .. })
    }

    #[must_use]
    pub const fn run_id(&self) -> Option<Uuid> {
        match self {
            Self::Live => None,
            Self::Frozen { run_id, .. } => Some(*run_id),
        }
    }

    #[must_use]
    pub fn result_for_node(&self, node_id: NodeId) -> Option<serde_json::Value> {
        match self {
            Self::Live => None,
            Self::Frozen { results, .. } => results.get(&node_id).cloned(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct FrozenModeState {
    mode: Signal<FrozenMode>,
}

#[allow(dead_code)]
impl FrozenModeState {
    #[must_use]
    pub fn mode(&self) -> ReadSignal<FrozenMode> {
        self.mode.into()
    }

    #[must_use]
    pub fn is_frozen(&self) -> bool {
        self.mode.read().is_frozen()
    }

    #[must_use]
    pub fn active_run_id(&self) -> Option<Uuid> {
        self.mode.read().run_id()
    }

    #[must_use]
    pub fn result_for_node(&self, node_id: NodeId) -> Option<serde_json::Value> {
        self.mode.read().result_for_node(node_id)
    }

    pub fn activate(&mut self, run: &RunRecord) {
        self.mode.set(FrozenMode::frozen(run.id, run.results.clone()));
    }

    pub fn deactivate(&mut self) {
        self.mode.set(FrozenMode::Live);
    }
}

#[allow(dead_code)]
pub fn use_frozen_mode() -> FrozenModeState {
    FrozenModeState { mode: use_signal(FrozenMode::default) }
}

#[must_use]
#[allow(dead_code)]
pub fn find_run_by_id(history: &[RunRecord], id: Uuid) -> Option<&RunRecord> {
    history.iter().find(|r| r.id == id)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{find_run_by_id, FrozenMode};
    use crate::graph::RunRecord;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn make_run(id: Uuid) -> RunRecord {
        RunRecord {
            id,
            timestamp: chrono::Utc::now(),
            results: HashMap::new(),
            success: true,
            restate_invocation_id: None,
        }
    }

    #[test]
    fn given_live_mode_when_is_frozen_then_returns_false() {
        let mode = FrozenMode::live();
        assert!(!mode.is_frozen());
    }

    #[test]
    fn given_frozen_mode_when_is_frozen_then_returns_true() {
        let mode = FrozenMode::frozen(Uuid::new_v4(), HashMap::new());
        assert!(mode.is_frozen());
    }

    #[test]
    fn given_live_mode_when_run_id_then_returns_none() {
        let mode = FrozenMode::live();
        assert!(mode.run_id().is_none());
    }

    #[test]
    fn given_frozen_mode_when_run_id_then_returns_some() {
        let id = Uuid::new_v4();
        let mode = FrozenMode::frozen(id, HashMap::new());
        assert_eq!(mode.run_id(), Some(id));
    }

    #[test]
    fn given_empty_history_when_finding_run_then_returns_none() {
        let result = find_run_by_id(&[], Uuid::new_v4());
        assert!(result.is_none());
    }

    #[test]
    fn given_history_with_run_when_finding_by_id_then_returns_run() {
        let id = Uuid::new_v4();
        let run = make_run(id);
        let history = vec![run];

        let result = find_run_by_id(&history, id);
        assert!(result.is_some());
        assert_eq!(result.map(|r| r.id), Some(id));
    }

    #[test]
    fn given_history_with_multiple_runs_when_finding_by_wrong_id_then_returns_none() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let wrong_id = Uuid::new_v4();
        let history = vec![make_run(id_a), make_run(id_b)];

        let result = find_run_by_id(&history, wrong_id);
        assert!(result.is_none());
    }
}
