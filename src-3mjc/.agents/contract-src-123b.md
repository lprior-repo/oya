# Contract Specification: Zellij Plugin State Persistence

**Bead ID**: src-123b
**Feature**: zellij: Restore state on load
**Version**: 1.0.0
**Date**: 2026-02-09

---

## Context

### Feature Description
The Zellij OYA plugin currently loses all state when shut down or restarted. This feature adds state persistence to automatically save the plugin state before shutdown and restore it on the next load, preserving:
- Selected task index
- Focused pane
- Task list data
- Status messages
- Plugin state machine state

### Domain Terms
- **Plugin State**: The complete runtime state of `OyaPlugin` including tasks, selection, focus, and state machine
- **State File**: Persistent storage file (JSON format) containing serialized plugin state
- **State Snapshot**: A point-in-time capture of the plugin state
- **Restore**: Loading a previously saved state snapshot into a running plugin
- **Save**: Writing the current plugin state to persistent storage

### Assumptions
1. State file will be stored in a known location (e.g., `$XDG_DATA_HOME/oya/zellij-plugin-state.json` or `/tmp/oya-zellij-state.json`)
2. State file format is JSON for human readability and debuggability
3. Save operations occur on graceful shutdown (via SIGTERM or explicit quit command)
4. Load operations occur on plugin initialization before rendering
5. Partial or corrupted state files should not crash the plugin; they should be ignored with a warning
6. State versioning is included to handle future schema changes

### Open Questions
1. **Q1**: Should state be saved on every state change or only on shutdown?
   - **A1**: Only on shutdown to minimize I/O. Real-time persistence can be added later if needed.

2. **Q2**: What should happen if the state file format is incompatible (version mismatch)?
   - **A2**: Log a warning and start with fresh state. Version field must be checked.

3. **Q3**: Should sensitive data (e.g., IPC connection) be persisted?
   - **A3**: No. Only persist UI state (tasks, selection, focus). IPC connections are re-established on load.

4. **Q4**: What is the maximum file size allowed for state files?
   - **A4**: 1MB limit. Larger files should be rejected with an error.

5. **Q5**: Should state persistence be automatic or manual?
   - **A5**: Automatic on shutdown (via Drop trait or explicit hook). Manual save method also provided.

---

## Preconditions

### For `save_state()`
- Plugin must be in a valid state (not `ShuttingDown` during cleanup)
- State file directory must exist and be writable
- Task list must not exceed maximum length (1000 items)
- Task data must be serializable (all fields valid)

### For `load_state()`
- State file must exist and be readable
- State file format version must be compatible
- State file size must be within limits (≤ 1MB)
- JSON must be well-formed and valid

### For `StateSnapshot::from_plugin()`
- Plugin reference must be valid
- All task data must be cloneable

### For `OyaPlugin::restore_from_snapshot()`
- Plugin must be in `Starting` or `Running` state
- Snapshot data must be valid
- No active IPC connection (or connection must be droppable)

---

## Postconditions

### For `save_state()`
- State file exists at the specified path
- State file contains complete and valid JSON
- State file includes version field
- All plugin state fields are serialized (except transient fields like IPC client)
- Returns `Ok(())` on success, `Err(StateError)` on failure
- File permissions are set to owner read/write only (0600)

### For `load_state()`
- Returns `Ok(StateSnapshot)` if file exists and is valid
- Returns `Err(StateError)` if file is missing, invalid, or incompatible
- Snapshot contains all fields from the file
- No side effects on the plugin (loading is separate from restoration)

### For `OyaPlugin::restore_from_snapshot()`
- Plugin tasks are replaced with snapshot tasks
- Selected index is updated (clamped to valid range)
- Focused pane is updated
- Plugin state machine is updated
- IPC client is preserved or reset to None
- Status message is restored or set to default
- Returns `Ok(())` on success, `Err(PluginError)` on failure

---

## Invariants

1. **State File Integrity**: If a state file exists, it must be valid JSON with matching version
2. **Idempotent Save**: Multiple consecutive saves produce identical state files (timestamp field excluded)
3. **Clamped Selection**: `selected_index` is always in range `[0, tasks.len().saturating_sub(1)]`
4. **Version Compatibility**: State version is always checked before deserialization
5. **No Partial State**: Either all fields are restored or none are (atomic operation)
6. **File Size Limits**: State files never exceed 1MB
7. **Secure Permissions**: State files always have restrictive permissions (0600)
8. **Transient Fields Excluded**: IPC client and other non-serializable fields are never persisted

---

## Error Taxonomy

```rust
/// Errors that can occur during state persistence operations
#[derive(Debug, Error, Clone, PartialEq)]
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

    /// Invalid state data (e.g., invalid selected_index)
    #[error("Invalid state data: {message}")]
    InvalidData { message: String },

    /// Directory not found or not writable
    #[error("State directory not available: {path}")]
    DirectoryNotAvailable { path: String },
}
```

---

## Contract Signatures

### Core State Persistence API

```rust
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
    /// Returns `Err(StateError)` if plugin state is invalid
    pub fn from_plugin(plugin: &OyaPlugin) -> Result<Self, StateError>;

    /// Validate snapshot data (clamps selected_index, etc.)
    ///
    /// # Errors
    /// Returns `Err(StateError)` if data is irrecoverably invalid
    pub fn validate(&mut self) -> Result<(), StateError>;
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
    /// * `state_file` - Path to the state file
    /// * `max_file_size` - Maximum file size (default: 1MB)
    ///
    /// # Errors
    /// Returns `Err(StateError)` if directory is not writable
    pub fn new(state_file: PathBuf, max_file_size: usize) -> Result<Self, StateError>;

    /// Save plugin state to disk
    ///
    /// # Arguments
    /// * `plugin` - Reference to the plugin
    ///
    /// # Errors
    /// Returns `Err(StateError)` if save fails
    pub fn save_state(&self, plugin: &OyaPlugin) -> Result<(), StateError>;

    /// Load plugin state from disk
    ///
    /// # Returns
    /// * `Ok(Some(snapshot))` - State loaded successfully
    /// * `Ok(None)` - No state file exists (first run)
    /// * `Err(StateError)` - Load failed (corrupted, incompatible, etc.)
    pub fn load_state(&self) -> Result<Option<StateSnapshot>, StateError>;

    /// Remove the state file
    ///
    /// # Errors
    /// Returns `Err(StateError)` if deletion fails
    pub fn clear_state(&self) -> Result<(), StateError>;

    /// Check if a state file exists
    pub fn state_exists(&self) -> bool;
}

impl OyaPlugin {
    /// Restore plugin state from a snapshot
    ///
    /// # Arguments
    /// * `snapshot` - State snapshot to restore
    ///
    /// # Errors
    /// Returns `Err(PluginError)` if restoration fails
    pub fn restore_from_snapshot(&mut self, snapshot: StateSnapshot) -> PluginResult<()>;

    /// Create a snapshot of current plugin state
    ///
    /// # Returns
    /// State snapshot
    pub fn create_snapshot(&self) -> StateSnapshot;

    /// Get the default state file path
    ///
    /// Uses `$XDG_DATA_HOME/oya/zellij-plugin-state.json` or `$HOME/.local/share/oya/...`
    pub fn default_state_file_path() -> PathBuf;
}
```

---

## Non-goals

1. **Real-time Persistence**: Not saving on every state change (too much I/O overhead)
2. **State Compression**: Not compressing state files (JSON is readable and small enough)
3. **Multiple State Files**: Not supporting multiple named states (only one default state)
4. **State Migration**: Not automatic migration from old versions (manual version check only)
5. **Conflict Resolution**: Not merging state from multiple sources (last write wins)
6. **Encryption**: Not encrypting state files (file permissions are sufficient)
7. **Distributed State**: Not syncing state across multiple instances (local only)
8. **Undo/Redo**: Not supporting state history (only current state)
9. **Partial Restoration**: Not supporting selective field restoration (all or nothing)

---

## Implementation Notes

1. **Use `serde` and `serde_json`** for serialization (already in dependencies)
2. **Implement `Drop` for `OyaPlugin`** to auto-save on shutdown
3. **Add `#[serde(skip)]`** to `ipc: Option<IpcClient>` field to exclude from serialization
4. **Clamp `selected_index`** on restoration to prevent out-of-bounds
5. **Use `std::fs::File` with `0o600` permissions** for security
6. **Add version constant**: `const STATE_VERSION: u32 = 1;`
7. **Log warnings** for corrupted/incompatible state files but don't crash
8. **State file location**: Use `dirs::data_local_dir()` crate or fallback to `/tmp`

---

## Verification Checklist

- [ ] State file is created on shutdown
- [ ] State file is valid JSON
- [ ] State file contains version field
- [ ] State file has restrictive permissions (0600)
- [ ] State is restored on plugin startup
- [ ] Invalid state files are rejected with error
- [ ] Missing state files are handled gracefully (first run)
- [ ] Selected index is clamped to valid range
- [ ] IPC client is not persisted (re-established on load)
- [ ] Version mismatch is detected and rejected
- [ ] File size limit is enforced (1MB)
- [ ] Directory creation fails gracefully
- [ ] Concurrent access is handled (atomic writes)
- [ ] Zero panics in all persistence operations
- [ ] All functions return `Result<T, Error>`
- [ ] Error messages are clear and actionable
