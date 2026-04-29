//! Audit: use_effect cleanup verification (of-vad)
//!
//! All use_effect calls in src/hooks/ and src/ui/ were reviewed.
//! None subscribe to events, open connections, or allocate resources
//! needing explicit cleanup. They are pure signal→signal computations.
//!
//! The async polling in use_restate_sync uses use_future, which
//! auto-cancels on component unmount (Dioxus 0.7 behavior).
//!
//! | File | Line | Purpose | Cleanup Needed |
//! |------|------|---------|---------------|
//! | app_shell.rs | 33 | Workflow→localStorage sync | No (one-shot write) |
//! | app_shell.rs | 168 | Inspector panel toggle | No (signal read) |
//! | main.rs | 97 | Tailwind CSS refresh | No (one-shot call) |
//! | selected_node_panel.rs | 31 | Extension preview sync | No (signal computation) |
//! | config_panel/mod.rs | 151 | Rich signal→config sync | No (signal comparison) |
//! | workflow_submit.rs | 21 | JSON draft sync | No (signal comparison) |
//! | workflow_call.rs | 21 | JSON draft sync | No (signal comparison) |
//! | service_call.rs | 23 | JSON draft sync | No (signal comparison) |
//! | delayed_message.rs | 24 | JSON draft sync | No (signal comparison) |
//!
//! use_restate_sync.rs uses use_future (not use_effect) for polling,
//! which auto-cancels on unmount. No explicit cleanup needed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

/// Verify no use_effect in hooks/ directory (they use use_future/use_signal instead).
#[test]
fn no_use_effect_in_hooks_directory() {
    let hooks_dir = std::path::Path::new("src/hooks");
    if !hooks_dir.exists() {
        return;
    }

    fn has_use_effect(dir: &std::path::Path) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if has_use_effect(&path) {
                    return true;
                }
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                if content.contains("use_effect") {
                    return true;
                }
            }
        }
        false
    }

    assert!(!has_use_effect(hooks_dir), "hooks/ should use use_future/use_signal, not use_effect");
}

/// Count use_effect calls in src/ to detect additions that may need cleanup review.
#[test]
fn use_effect_count_is_stable() {
    let src_dir = std::path::Path::new("src");
    let mut count = 0usize;
    let mut locations: Vec<String> = Vec::new();

    fn count_effects(dir: &std::path::Path, count: &mut usize, locations: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count_effects(&path, count, locations);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (i, line) in content.lines().enumerate() {
                    if line.contains("use_effect(move ||") {
                        *count += 1;
                        let rel = path.strip_prefix(std::path::Path::new("src")).unwrap_or(&path);
                        locations.push(format!("{}:{}", rel.display(), i + 1));
                    }
                }
            }
        }
    }

    count_effects(src_dir, &mut count, &mut locations);

    // As of 2026-04-19 audit: 9 use_effect calls, all pure signal computations.
    // If this count changes, review the new calls for cleanup needs.
    assert_eq!(
        count, 9,
        "use_effect count changed from 9. New locations: {locations:?}. \
         Review each for cleanup function necessity."
    );
}
