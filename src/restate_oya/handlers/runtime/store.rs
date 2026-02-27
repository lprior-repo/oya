#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::workflow::LifecycleProgressUpdate;
use crate::restate_oya::types::{LifecycleStatusSnapshot, LifecycleStepSnapshot};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

static RUNTIME_LIFECYCLE_STATUS: LazyLock<RwLock<HashMap<String, LifecycleStatusSnapshot>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_runtime_status(key: &str) -> Option<LifecycleStatusSnapshot> {
    RUNTIME_LIFECYCLE_STATUS.read().ok().and_then(|map| {
        runtime_lookup_keys(key).into_iter().find_map(|candidate| map.get(&candidate).cloned())
    })
}

pub fn seed_runtime_status(
    workflow_key: &str,
    bead_id: Option<String>,
    steps: &[LifecycleStepSnapshot],
) {
    if let Ok(mut map) = RUNTIME_LIFECYCLE_STATUS.write() {
        insert_runtime_status(
            &mut map,
            workflow_key,
            LifecycleStatusSnapshot {
                bead_id,
                steps: steps.to_vec(),
                state: None,
                pr_url: None,
                done: false,
                success: None,
                message: None,
                compensation_diagnostics: Vec::new(),
            },
        );
    }
}

pub fn update_runtime_progress(
    key: &str,
    live_steps: &[LifecycleStepSnapshot],
    update: LifecycleProgressUpdate,
) {
    if let Ok(mut map) = RUNTIME_LIFECYCLE_STATUS.write() {
        let current = runtime_lookup_keys(key)
            .into_iter()
            .find_map(|candidate| map.get(&candidate).cloned())
            .unwrap_or_else(|| LifecycleStatusSnapshot {
                bead_id: Some(key.to_owned()),
                steps: Vec::new(),
                state: None,
                pr_url: None,
                done: false,
                success: None,
                message: None,
                compensation_diagnostics: Vec::new(),
            });
        let next = runtime_status_next(current, live_steps, update);
        insert_runtime_status(&mut map, key, next);
    }
}

pub fn cleanup_targets_for_key(key: &str) -> Vec<String> {
    let mut targets = vec![key.to_owned()];
    if let Some(status) = get_runtime_status(key) {
        if let Some(bead_id) = status.bead_id {
            if bead_id != key {
                targets.push(bead_id);
            }
        }
    }
    targets
}

fn insert_runtime_status(
    map: &mut HashMap<String, LifecycleStatusSnapshot>,
    workflow_key: &str,
    snapshot: LifecycleStatusSnapshot,
) {
    runtime_store_keys(workflow_key, snapshot.bead_id.as_deref()).into_iter().for_each(
        |candidate| {
            map.insert(candidate, snapshot.clone());
        },
    );
}

fn runtime_store_keys(workflow_key: &str, bead_id: Option<&str>) -> Vec<String> {
    let mut keys = runtime_lookup_keys(workflow_key);
    if let Some(id) = bead_id {
        keys = keys.into_iter().chain(runtime_lookup_keys(id)).collect::<Vec<_>>();
    }
    keys.sort();
    keys.dedup();
    keys
}

fn runtime_lookup_keys(key: &str) -> Vec<String> {
    let normalized = key.strip_prefix("Oya/").and_then(|value| value.strip_suffix("/run"));
    match normalized {
        Some(inner) => vec![key.to_owned(), inner.to_owned()],
        None => vec![key.to_owned(), format!("Oya/{key}/run")],
    }
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
