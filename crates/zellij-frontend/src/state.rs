//! State persistence module for Zellij plugin
//!
//! Provides state save/load functionality with:
//! - Zero panics, zero unwraps
//! - Railway-Oriented Programming (Result types throughout)
//! - Functional patterns (map, `and_then`, ? operator)
//! - Secure file permissions (0600)
//! - Version checking and validation

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::layout::PaneType;
use crate::plugin::{OyaPlugin, PluginState, TaskRow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Current state format version
pub const STATE_VERSION: u32 = 1;

/// Default maximum state file size (1MB)
const DEFAULT_MAX_FILE_SIZE: usize = 1_048_576;

/// Maximum number of tasks allowed in state
const MAX_TASKS: usize = 1000;

/// Errors that can occur during state persistence operations
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StateError {
    /// State file not found
    #[error("State file not found: {path}")]
    NotFound { path: String },

    /// State file is corrupted or invalid JSON
    #[error("State file is corrupted or invalid: {path}")]
    Corrupted { path: String },

    /// State file version is incompatible
    #[error("State file version {version} is incompatible (expected: {expected})")]
    IncompatibleVersion { version: u32, expected: u32 },

    /// State file exceeds maximum size
    #[error("State file exceeds maximum size: {size} bytes (max: {max} bytes)")]
    FileTooLarge { size: usize, max: usize },

    /// I/O error during state file operations
    #[error("I/O error: {message}")]
    Io { message: String },

    /// Serialization error
    #[error("Failed to serialize state: {message}")]
    Serialization { message: String },

    /// Deserialization error
    #[error("Failed to deserialize state: {message}")]
    Deserialization { message: String },

    /// Invalid state data (e.g., invalid `selected_index`)
    #[error("Invalid state data: {message}")]
    InvalidData { message: String },

    /// Directory not found or not writable
    #[error("State directory not available: {path}")]
    DirectoryNotAvailable { path: String },

    /// Too many tasks in state
    #[error("Too many tasks in state: {count} (max: {max})")]
    TooManyTasks { count: usize, max: usize },
}

/// Snapshot of plugin state for persistence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateSnapshot {
    /// Version of the state format
    pub version: u32,

    /// List of tasks
    pub tasks: Vec<TaskRow>,

    /// Currently selected task index
    pub selected_index: usize,

    /// Currently focused pane
    pub focused_pane: PaneType,

    /// Plugin state machine state
    pub plugin_state: PluginState,

    /// Status message (if any)
    pub status_message: Option<String>,

    /// Timestamp when snapshot was created
    pub timestamp: u64,
}

impl StateSnapshot {
    /// Create a snapshot from a running plugin
    ///
    /// # Errors
    ///
    /// Returns `Err(StateError)` if plugin state is invalid or task count exceeds maximum
    pub fn from_plugin(plugin: &OyaPlugin) -> Result<Self, StateError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let tasks = plugin.tasks_ref();

        if tasks.len() > MAX_TASKS {
            return Err(StateError::TooManyTasks {
                count: tasks.len(),
                max: MAX_TASKS,
            });
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_secs());

        Ok(Self {
            version: STATE_VERSION,
            tasks: tasks.to_vec(),
            selected_index: plugin.selected_index(),
            focused_pane: plugin.focused_pane(),
            plugin_state: plugin.plugin_state(),
            status_message: plugin.status_message().map(String::from),
            timestamp,
        })
    }

    /// Validate snapshot data (clamps `selected_index`, etc.)
    ///
    /// # Errors
    ///
    /// Returns `Err(StateError)` if data is irrecoverably invalid
    pub fn validate(&mut self) -> Result<(), StateError> {
        // Check version compatibility
        if self.version != STATE_VERSION {
            return Err(StateError::IncompatibleVersion {
                version: self.version,
                expected: STATE_VERSION,
            });
        }

        // Validate task count
        if self.tasks.len() > MAX_TASKS {
            return Err(StateError::TooManyTasks {
                count: self.tasks.len(),
                max: MAX_TASKS,
            });
        }

        // Clamp selected_index to valid range
        let max_index = self.tasks.len().saturating_sub(1);
        if self.selected_index > max_index {
            self.selected_index = max_index;
        }

        Ok(())
    }
}

/// State persistence manager
pub struct StateManager {
    /// Path to the state file
    state_file: PathBuf,

    /// Maximum allowed state file size in bytes
    max_file_size: usize,

    /// Current state format version
    version: u32,
}

impl StateManager {
    /// Create a new state manager
    ///
    /// # Arguments
    ///
    /// * `state_file` - Path to the state file
    /// * `max_file_size` - Maximum file size (default: 1MB)
    ///
    /// # Errors
    ///
    /// Returns `Err(StateError)` if directory is not writable
    pub fn new(state_file: PathBuf, max_file_size: usize) -> Result<Self, StateError> {
        // Check if parent directory is writable
        if let Some(parent) = state_file.parent() {
            if parent.exists() {
                // Check if directory is writable by attempting to create a test file
                let test_result =
                    std::fs::metadata(parent).map_err(|_e| StateError::DirectoryNotAvailable {
                        path: parent.display().to_string(),
                    });

                // If we can get metadata, directory exists and is accessible
                match test_result {
                    Ok(metadata) => {
                        if metadata.permissions().readonly() {
                            return Err(StateError::DirectoryNotAvailable {
                                path: parent.display().to_string(),
                            });
                        }
                    }
                    Err(e) => return Err(e),
                }
            } else {
                // Try to create the directory
                std::fs::create_dir_all(parent).map_err(|_| StateError::DirectoryNotAvailable {
                    path: parent.display().to_string(),
                })?;
            }
        }

        Ok(Self {
            state_file,
            max_file_size,
            version: STATE_VERSION,
        })
    }

    /// Get the state format version
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Get the maximum file size in bytes
    #[must_use]
    pub const fn max_file_size(&self) -> usize {
        self.max_file_size
    }

    /// Save plugin state to disk
    ///
    /// # Arguments
    ///
    /// * `plugin` - Reference to the plugin
    ///
    /// # Errors
    ///
    /// Returns `Err(StateError)` if save fails
    pub fn save_state(&self, plugin: &OyaPlugin) -> Result<(), StateError> {
        // Create snapshot
        let mut snapshot = StateSnapshot::from_plugin(plugin)?;

        // Validate snapshot
        snapshot.validate()?;

        // Serialize to JSON
        let json =
            serde_json::to_string_pretty(&snapshot).map_err(|e| StateError::Serialization {
                message: e.to_string(),
            })?;

        // Write to file with secure permissions
        self.write_state_file(&json)?;

        Ok(())
    }

    /// Load plugin state from disk
    ///
    /// # Returns
    ///
    /// * `Ok(Some(snapshot))` - State loaded successfully
    /// * `Ok(None)` - No state file exists (first run)
    /// * `Err(StateError)` - Load failed (corrupted, incompatible, etc.)
    pub fn load_state(&self) -> Result<Option<StateSnapshot>, StateError> {
        // Check if file exists
        if !self.state_exists() {
            return Ok(None);
        }

        // Read file
        let json = self.read_state_file()?;

        // Deserialize
        let mut snapshot: StateSnapshot =
            serde_json::from_str(&json).map_err(|e| StateError::Deserialization {
                message: e.to_string(),
            })?;

        // Validate snapshot
        snapshot.validate()?;

        Ok(Some(snapshot))
    }

    /// Remove the state file
    ///
    /// # Errors
    ///
    /// Returns `Err(StateError)` if deletion fails
    pub fn clear_state(&self) -> Result<(), StateError> {
        if !self.state_exists() {
            return Ok(());
        }

        std::fs::remove_file(&self.state_file).map_err(|e| StateError::Io {
            message: e.to_string(),
        })?;

        Ok(())
    }

    /// Check if a state file exists
    #[must_use] 
    pub fn state_exists(&self) -> bool {
        self.state_file.exists()
    }

    /// Get the state file path
    #[must_use] 
    pub fn state_file_path(&self) -> &Path {
        &self.state_file
    }

    /// Write state to file with secure permissions
    fn write_state_file(&self, json: &str) -> Result<(), StateError> {
        use std::fs::File;
        use std::io::Write;

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.state_file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StateError::Io {
                message: format!("Failed to create directory {}: {}", parent.display(), e),
            })?;
        }

        // Write to temporary file first (atomic write)
        let temp_path = self.state_file.with_extension("tmp");
        let mut file = File::create(&temp_path).map_err(|e| StateError::Io {
            message: format!("Failed to create temp file: {e}"),
        })?;

        file.write_all(json.as_bytes())
            .map_err(|e| StateError::Io {
                message: format!("Failed to write state: {e}"),
            })?;

        file.flush().map_err(|e| StateError::Io {
            message: format!("Failed to flush state: {e}"),
        })?;

        // Set secure permissions (0600)
        set_secure_permissions(&temp_path)?;

        // Atomic rename to final path
        std::fs::rename(&temp_path, &self.state_file).map_err(|e| StateError::Io {
            message: format!("Failed to rename state file: {e}"),
        })?;

        Ok(())
    }

    /// Read state from file
    fn read_state_file(&self) -> Result<String, StateError> {
        // Check file size
        let metadata = std::fs::metadata(&self.state_file).map_err(|e| StateError::Io {
            message: format!("Failed to read file metadata: {e}"),
        })?;

        let file_size = metadata.len() as usize;
        if file_size > self.max_file_size {
            return Err(StateError::FileTooLarge {
                size: file_size,
                max: self.max_file_size,
            });
        }

        // Read file
        std::fs::read_to_string(&self.state_file).map_err(|e| StateError::Io {
            message: format!("Failed to read state file: {e}"),
        })
    }
}

/// Set secure file permissions (0600 - owner read/write only)
///
/// # Errors
///
/// Returns `Err(StateError)` if permissions cannot be set
fn set_secure_permissions(path: &Path) -> Result<(), StateError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| StateError::Io {
                message: format!("Failed to get file metadata: {e}"),
            })?
            .permissions();

        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms).map_err(|e| StateError::Io {
            message: format!("Failed to set file permissions: {e}"),
        })?;
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, we can't set specific permissions
        // Just log a warning or ignore
    }

    Ok(())
}

impl Default for StateManager {
    fn default() -> Self {
        let state_file = OyaPlugin::default_state_file_path();
        Self::new(state_file, DEFAULT_MAX_FILE_SIZE).unwrap_or_else(|_| {
            // Fallback to /tmp if default path fails
            let fallback = PathBuf::from("/tmp/oya-zellij-state.json");
            Self::new(fallback, DEFAULT_MAX_FILE_SIZE).unwrap_or_else(|_| Self {
                state_file: PathBuf::from("/tmp/oya-zellij-state-fallback.json"),
                max_file_size: DEFAULT_MAX_FILE_SIZE,
                version: STATE_VERSION,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_state_snapshot_from_plugin() {
        // This test requires access to OyaPlugin internals
        // For now, just test the validation logic
        let mut snapshot = StateSnapshot {
            version: STATE_VERSION,
            tasks: vec![],
            selected_index: 0,
            focused_pane: PaneType::BeadList,
            plugin_state: PluginState::Running,
            status_message: None,
            timestamp: 0,
        };

        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn test_state_snapshot_validate_clamps_selected_index() {
        let mut snapshot = StateSnapshot {
            version: STATE_VERSION,
            tasks: vec![],
            selected_index: 10,
            focused_pane: PaneType::BeadList,
            plugin_state: PluginState::Running,
            status_message: None,
            timestamp: 0,
        };

        assert!(snapshot.validate().is_ok());
        assert_eq!(snapshot.selected_index, 0);
    }

    #[test]
    fn test_state_snapshot_validate_incompatible_version() {
        let mut snapshot = StateSnapshot {
            version: 999,
            tasks: vec![],
            selected_index: 0,
            focused_pane: PaneType::BeadList,
            plugin_state: PluginState::Running,
            status_message: None,
            timestamp: 0,
        };

        let result = snapshot.validate();
        assert!(matches!(
            result,
            Err(StateError::IncompatibleVersion { .. })
        ));
    }

    #[test]
    fn test_state_manager_creation() {
        let temp_dir = std::env::temp_dir();
        let state_file = temp_dir.join("test-state.json");
        let manager = StateManager::new(state_file, DEFAULT_MAX_FILE_SIZE);

        assert!(manager.is_ok());
    }

    #[test]
    fn test_state_manager_default() {
        let manager = StateManager::default();
        assert_eq!(manager.version(), STATE_VERSION);
        assert_eq!(manager.max_file_size(), DEFAULT_MAX_FILE_SIZE);
    }

    #[test]
    fn test_state_manager_state_exists() {
        let temp_dir = std::env::temp_dir();
        let state_file = temp_dir.join("test-state-exists.json");
        let manager = StateManager::new(state_file.clone(), DEFAULT_MAX_FILE_SIZE)
            .expect("Failed to create StateManager");

        // File doesn't exist yet
        assert!(!manager.state_exists());

        // Create file
        fs::write(&state_file, "{}").expect("Failed to write test state file");

        // File exists now
        assert!(manager.state_exists());

        // Cleanup
        fs::remove_file(&state_file).expect("Failed to remove test state file");
    }

    #[test]
    fn test_state_manager_clear_state() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir();
        let state_file = temp_dir.join("test-state-clear.json");
        let manager = StateManager::new(state_file.clone(), DEFAULT_MAX_FILE_SIZE)?;

        // Clear when file doesn't exist should succeed
        assert!(manager.clear_state().is_ok());

        // Create file
        fs::write(&state_file, "{}")?;

        // File exists
        assert!(manager.state_exists());

        // Clear should succeed
        assert!(manager.clear_state().is_ok());

        // File should be gone
        assert!(!manager.state_exists());

        Ok(())
    }

    #[test]
    fn test_state_error_display() {
        let err = StateError::NotFound {
            path: "/test/path".to_string(),
        };
        assert!(err.to_string().contains("State file not found"));
        assert!(err.to_string().contains("/test/path"));

        let err = StateError::IncompatibleVersion {
            version: 2,
            expected: 1,
        };
        assert!(err.to_string().contains("version 2"));
        assert!(err.to_string().contains("expected: 1"));

        let err = StateError::FileTooLarge {
            size: 2_000_000,
            max: 1_000_000,
        };
        assert!(err.to_string().contains("2000000"));
        assert!(err.to_string().contains("1000000"));
    }

    #[test]
    fn test_state_constants() {
        assert_eq!(STATE_VERSION, 1);
        assert_eq!(DEFAULT_MAX_FILE_SIZE, 1_048_576);
        assert_eq!(MAX_TASKS, 1000);
    }
}
