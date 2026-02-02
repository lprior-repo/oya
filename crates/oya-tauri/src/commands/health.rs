//! Health check commands
//!
//! Provides system health information and diagnostics.

use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

/// Health check response
#[derive(Serialize)]
pub struct HealthStatus {
    /// Whether the system is healthy
    pub healthy: bool,
    /// Application version
    pub version: &'static str,
    /// Number of cached beads
    pub cached_beads: u64,
    /// Number of active streams
    pub active_streams: usize,
    /// Project root (if configured)
    pub project_root: Option<String>,
}

/// Check system health
///
/// Returns basic health information for monitoring.
#[tauri::command]
pub fn health_check(state: State<'_, Arc<AppState>>) -> HealthStatus {
    HealthStatus {
        healthy: true,
        version: env!("CARGO_PKG_VERSION"),
        cached_beads: state.cache.entry_count(),
        active_streams: state.active_stream_count(),
        project_root: state.project_root().and_then(|p| p.to_str().map(String::from)),
    }
}

/// Get system information
#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
    }
}

/// System information
#[derive(Serialize)]
pub struct SystemInfo {
    pub os: &'static str,
    pub arch: &'static str,
    pub rust_version: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info() {
        let info = get_system_info();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }
}
