#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::workflow::LifecycleProgressUpdate;
use crate::restate_oya::types::{LifecycleStatusSnapshot, LifecycleStepSnapshot};
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

#[derive(Clone)]
struct RuntimeKey(String);

impl RuntimeKey {
    fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn aliases(&self) -> Vec<Self> {
        let normalized = self.0.strip_prefix("Oya/").and_then(|value| value.strip_suffix("/run"));
        match normalized {
            Some(inner) => vec![Self::new(self.0.clone()), Self::new(inner)],
            None => vec![Self::new(self.0.clone()), Self::new(format!("Oya/{}/run", self.0))],
        }
    }
}

static RUNTIME_LIFECYCLE_STATUS: LazyLock<RwLock<HashMap<String, LifecycleStatusSnapshot>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_runtime_status(key: &str) -> Option<LifecycleStatusSnapshot> {
    let runtime_key = RuntimeKey::new(key);
    RUNTIME_LIFECYCLE_STATUS.read().ok().and_then(|map| {
        runtime_key.aliases().into_iter().find_map(|candidate| map.get(candidate.as_str()).cloned())
    })
}

pub fn seed_runtime_status(
    workflow_key: &str,
    bead_id: Option<String>,
    steps: &[LifecycleStepSnapshot],
) {
    if let Ok(mut map) = RUNTIME_LIFECYCLE_STATUS.write() {
        let snapshot = LifecycleStatusSnapshot {
            bead_id,
            steps: steps.to_vec(),
            state: None,
            pr_url: None,
            done: false,
            success: None,
            message: None,
            compensation_diagnostics: Vec::new(),
        };
        insert_runtime_status(&mut map, workflow_key, &snapshot);
    }
}

pub fn update_runtime_progress(
    key: &str,
    live_steps: &[LifecycleStepSnapshot],
    update: LifecycleProgressUpdate,
) {
    if let Ok(mut map) = RUNTIME_LIFECYCLE_STATUS.write() {
        let current = RuntimeKey::new(key)
            .aliases()
            .into_iter()
            .find_map(|candidate| map.get(candidate.as_str()).cloned());
        let current = match current {
            Some(snapshot) => snapshot,
            None => LifecycleStatusSnapshot {
                bead_id: Some(key.to_owned()),
                steps: Vec::new(),
                state: None,
                pr_url: None,
                done: false,
                success: None,
                message: None,
                compensation_diagnostics: Vec::new(),
            },
        };
        let next = runtime_status_next(current, live_steps, update);
        insert_runtime_status(&mut map, key, &next);
    }
}

pub fn cleanup_targets_for_key(key: &str) -> Vec<String> {
    std::iter::once(key.to_owned())
        .chain(get_runtime_status(key).and_then(|status| status.bead_id))
        .unique()
        .collect()
}

fn insert_runtime_status(
    map: &mut HashMap<String, LifecycleStatusSnapshot>,
    workflow_key: &str,
    snapshot: &LifecycleStatusSnapshot,
) {
    for candidate in runtime_store_keys(workflow_key, snapshot.bead_id.as_deref()) {
        map.insert(candidate, snapshot.clone());
    }
}

fn runtime_store_keys(workflow_key: &str, bead_id: Option<&str>) -> Vec<String> {
    RuntimeKey::new(workflow_key)
        .aliases()
        .into_iter()
        .chain(bead_id.map(RuntimeKey::new).map_or_else(Vec::new, |id| id.aliases()))
        .map(|key| key.0)
        .unique()
        .collect()
}

fn runtime_status_next(
    current: LifecycleStatusSnapshot,
    live_steps: &[LifecycleStepSnapshot],
    update: LifecycleProgressUpdate,
) -> LifecycleStatusSnapshot {
    match update {
        LifecycleProgressUpdate::Initialized { bead_id, .. } => LifecycleStatusSnapshot {
            bead_id: Some(bead_id),
            steps: live_steps.to_vec(),
            state: current.state,
            pr_url: current.pr_url,
            done: false,
            success: None,
            message: None,
            compensation_diagnostics: current.compensation_diagnostics,
        },
        LifecycleProgressUpdate::Step { message, .. } => LifecycleStatusSnapshot {
            bead_id: current.bead_id,
            steps: live_steps.to_vec(),
            state: current.state,
            pr_url: current.pr_url,
            done: false,
            success: None,
            message,
            compensation_diagnostics: current.compensation_diagnostics,
        },
        LifecycleProgressUpdate::Finished {
            success,
            pr_url,
            message,
            compensation_diagnostics,
        } => LifecycleStatusSnapshot {
            bead_id: current.bead_id,
            steps: live_steps.to_vec(),
            state: current.state,
            pr_url,
            done: true,
            success: Some(success),
            message,
            compensation_diagnostics,
        },
    }
}
