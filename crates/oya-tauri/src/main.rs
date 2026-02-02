//! oya-tauri desktop application
//!
//! High-performance Tauri backend for the oya development dashboard.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use oya_tauri::commands;
use oya_tauri::state::AppState;
use std::sync::Arc;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Initialize application state
            let state = Arc::new(AppState::new());
            app.manage(state);

            // Try to auto-detect project root
            if let Ok(cwd) = std::env::current_dir() {
                let app_state = app.state::<Arc<AppState>>();
                if let Some(root) = find_project_root(&cwd) {
                    app_state.set_project_root(root.clone());
                    app_state.set_beads_dir(root.join(".beads"));
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Bead commands
            commands::get_bead,
            commands::get_beads_batch,
            commands::list_beads_paginated,
            commands::update_bead_status,
            commands::cancel_beads_batch,
            commands::get_project_root,
            commands::invalidate_bead_cache,
            commands::get_cache_stats,
            // Stream commands
            commands::start_process_stream,
            commands::stop_stream,
            commands::get_stream_status,
            commands::list_active_streams,
            // Health commands
            commands::health_check,
            commands::get_system_info,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Error running Tauri application: {e}");
            std::process::exit(1);
        });
}

/// Find project root by looking for .beads/ or .git/
fn find_project_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        if current.join(".beads").is_dir() {
            return Some(current);
        }
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}
