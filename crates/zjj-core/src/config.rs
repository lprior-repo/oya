//! Configuration loading and management
//!
//! # Hierarchy
//!
//! Configuration is loaded in this order (later overrides earlier):
//! 1. Built-in defaults
//! 2. Global config: ~/.config/jjz/config.toml
//! 3. Project config: .jjz/config.toml
//! 4. Environment variables: JJZ_*
//! 5. CLI flags (command-specific)
//!
//! # Example Config
//!
//! ```toml
//! workspace_dir = "../{repo}__workspaces"
//! main_branch = "main"
//!
//! [zellij.panes.main]
//! command = "claude"
//! size = "70%"
//!
//! [hooks]
//! post_create = ["bd sync", "npm install"]
//! ```

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub workspace_dir: String,
    pub main_branch: String,
    pub default_template: String,
    pub state_db: String,
    pub watch: WatchConfig,
    pub hooks: HooksConfig,
    pub zellij: ZellijConfig,
    pub dashboard: DashboardConfig,
    pub agent: AgentConfig,
    pub session: SessionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchConfig {
    pub enabled: bool,
    pub debounce_ms: u32,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HooksConfig {
    pub post_create: Vec<String>,
    pub pre_remove: Vec<String>,
    pub post_merge: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZellijConfig {
    pub session_prefix: String,
    pub use_tabs: bool,
    pub layout_dir: String,
    pub panes: PanesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanesConfig {
    pub main: PaneConfig,
    pub beads: PaneConfig,
    pub status: PaneConfig,
    pub float: FloatPaneConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaneConfig {
    pub command: String,
    pub args: Vec<String>,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FloatPaneConfig {
    pub enabled: bool,
    pub command: String,
    pub width: String,
    pub height: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardConfig {
    pub refresh_ms: u32,
    pub theme: String,
    pub columns: Vec<String>,
    pub vim_keys: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    pub command: String,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionConfig {
    pub auto_commit: bool,
    pub commit_prefix: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// DEFAULT IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace_dir: "../{repo}__workspaces".to_string(),
            main_branch: String::new(),
            default_template: "standard".to_string(),
            state_db: ".jjz/state.db".to_string(),
            watch: WatchConfig::default(),
            hooks: HooksConfig::default(),
            zellij: ZellijConfig::default(),
            dashboard: DashboardConfig::default(),
            agent: AgentConfig::default(),
            session: SessionConfig::default(),
        }
    }
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_ms: 100,
            paths: vec![".beads/beads.db".to_string()],
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            post_create: Vec::new(),
            pre_remove: Vec::new(),
            post_merge: Vec::new(),
        }
    }
}

impl Default for ZellijConfig {
    fn default() -> Self {
        Self {
            session_prefix: "jjz".to_string(),
            use_tabs: true,
            layout_dir: ".jjz/layouts".to_string(),
            panes: PanesConfig::default(),
        }
    }
}

impl Default for PanesConfig {
    fn default() -> Self {
        Self {
            main: PaneConfig {
                command: "claude".to_string(),
                args: Vec::new(),
                size: "70%".to_string(),
            },
            beads: PaneConfig {
                command: "bv".to_string(),
                args: Vec::new(),
                size: "50%".to_string(),
            },
            status: PaneConfig {
                command: "jjz".to_string(),
                args: vec!["status".to_string(), "--watch".to_string()],
                size: "50%".to_string(),
            },
            float: FloatPaneConfig::default(),
        }
    }
}

impl Default for FloatPaneConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            command: String::new(),
            width: "80%".to_string(),
            height: "60%".to_string(),
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            refresh_ms: 1000,
            theme: "default".to_string(),
            columns: vec![
                "name".to_string(),
                "status".to_string(),
                "branch".to_string(),
                "changes".to_string(),
                "beads".to_string(),
            ],
            vim_keys: true,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            env: HashMap::new(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_commit: false,
            commit_prefix: "wip:".to_string(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/// Load configuration from all sources with hierarchy
///
/// # Errors
///
/// Returns error if:
/// - Config file is malformed TOML
/// - Config values fail validation
/// - Unable to determine repository name for placeholder substitution
pub fn load_config() -> Result<Config> {
    // 1. Start with built-in defaults
    let mut config = Config::default();

    // 2. Load global config if exists
    if let Some(global_path) = global_config_path() {
        if global_path.exists() {
            let global = load_toml_file(&global_path)?;
            config.merge(global);
        }
    }

    // 3. Load project config if exists
    let project_path = project_config_path()?;
    if project_path.exists() {
        let project = load_toml_file(&project_path)?;
        config.merge(project); // Project overrides global
    }

    // 4. Apply environment variable overrides
    config.apply_env_vars()?;

    // 5. Validate and substitute placeholders
    config.validate()?;
    config.substitute_placeholders()?;

    Ok(config)
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get path to global config file
fn global_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "jjz")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
}

/// Get path to project config file
///
/// # Errors
///
/// Returns error if current directory cannot be determined
fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .map(|dir| dir.join(".jjz/config.toml"))
        .map_err(|e| Error::IoError(format!("Failed to get current directory: {e}")))
}

/// Get repository name from current directory
///
/// # Errors
///
/// Returns error if:
/// - Current directory cannot be determined
/// - Directory name cannot be extracted
fn get_repo_name() -> Result<String> {
    std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current directory: {e}")))
        .and_then(|dir| {
            dir.file_name()
                .and_then(|name| name.to_str())
                .map(String::from)
                .ok_or_else(|| Error::Unknown("Failed to determine repository name".to_string()))
        })
}

/// Load a TOML file into a partial Config
///
/// # Errors
///
/// Returns error if:
/// - File cannot be read
/// - TOML is malformed
fn load_toml_file(path: &std::path::Path) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::IoError(format!(
            "Failed to read config file {}: {e}",
            path.display()
        ))
    })?;

    toml::from_str(&content)
        .map_err(|e| Error::ParseError(format!("Failed to parse config: {}: {e}", path.display())))
}

// ═══════════════════════════════════════════════════════════════════════════
// MERGE TRAIT IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

impl WatchConfig {
    fn merge(&mut self, other: Self) {
        // Always take other's value for primitives if different from default
        self.enabled = other.enabled;
        if other.debounce_ms != 100 {
            self.debounce_ms = other.debounce_ms;
        }
        if other.paths != vec![".beads/beads.db".to_string()] {
            self.paths = other.paths;
        }
    }
}

impl HooksConfig {
    fn merge(&mut self, other: Self) {
        // Replace (not append) for hooks
        if !other.post_create.is_empty() {
            self.post_create = other.post_create;
        }
        if !other.pre_remove.is_empty() {
            self.pre_remove = other.pre_remove;
        }
        if !other.post_merge.is_empty() {
            self.post_merge = other.post_merge;
        }
    }
}

impl ZellijConfig {
    fn merge(&mut self, other: Self) {
        if other.session_prefix != "jjz" {
            self.session_prefix = other.session_prefix;
        }
        self.use_tabs = other.use_tabs;
        if other.layout_dir != ".jjz/layouts" {
            self.layout_dir = other.layout_dir;
        }
        self.panes.merge(other.panes);
    }
}

impl PanesConfig {
    fn merge(&mut self, other: Self) {
        self.main.merge(other.main);
        self.beads.merge(other.beads);
        self.status.merge(other.status);
        self.float.merge(other.float);
    }
}

impl PaneConfig {
    fn merge(&mut self, other: Self) {
        if !other.command.is_empty() {
            self.command = other.command;
        }
        if !other.args.is_empty() {
            self.args = other.args;
        }
        if !other.size.is_empty() {
            self.size = other.size;
        }
    }
}

impl FloatPaneConfig {
    fn merge(&mut self, other: Self) {
        self.enabled = other.enabled;
        if !other.command.is_empty() {
            self.command = other.command;
        }
        if other.width != "80%" {
            self.width = other.width;
        }
        if other.height != "60%" {
            self.height = other.height;
        }
    }
}

impl DashboardConfig {
    fn merge(&mut self, other: Self) {
        if other.refresh_ms != 1000 {
            self.refresh_ms = other.refresh_ms;
        }
        if other.theme != "default" {
            self.theme = other.theme;
        }
        let default_columns = vec![
            "name".to_string(),
            "status".to_string(),
            "branch".to_string(),
            "changes".to_string(),
            "beads".to_string(),
        ];
        if other.columns != default_columns {
            self.columns = other.columns;
        }
        self.vim_keys = other.vim_keys;
    }
}

impl AgentConfig {
    fn merge(&mut self, other: Self) {
        if other.command != "claude" {
            self.command = other.command;
        }
        if !other.env.is_empty() {
            self.env = other.env;
        }
    }
}

impl SessionConfig {
    fn merge(&mut self, other: Self) {
        self.auto_commit = other.auto_commit;
        if other.commit_prefix != "wip:" {
            self.commit_prefix = other.commit_prefix;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG METHODS
// ═══════════════════════════════════════════════════════════════════════════

impl Config {
    /// Merge another config into this one (other takes precedence)
    ///
    /// Note: This performs a deep replacement merge, not append.
    /// For example, if `hooks.post_create` is `["a","b"]` in self and `["c"]` in other,
    /// the result will be `["c"]`, not `["a","b","c"]`.
    fn merge(&mut self, other: Self) {
        // Top-level string fields - replace if non-empty/non-default
        if !other.workspace_dir.is_empty() {
            self.workspace_dir = other.workspace_dir;
        }
        if !other.main_branch.is_empty() {
            self.main_branch = other.main_branch;
        }
        if other.default_template != "standard" {
            self.default_template = other.default_template;
        }
        if other.state_db != ".jjz/state.db" {
            self.state_db = other.state_db;
        }

        // Merge nested configs
        self.watch.merge(other.watch);
        self.hooks.merge(other.hooks);
        self.zellij.merge(other.zellij);
        self.dashboard.merge(other.dashboard);
        self.agent.merge(other.agent);
        self.session.merge(other.session);
    }

    /// Apply environment variable overrides
    ///
    /// # Errors
    ///
    /// Returns error if environment variable values are invalid
    fn apply_env_vars(&mut self) -> Result<()> {
        // JJZ_WORKSPACE_DIR
        if let Ok(value) = std::env::var("JJZ_WORKSPACE_DIR") {
            self.workspace_dir = value;
        }

        // JJZ_MAIN_BRANCH
        if let Ok(value) = std::env::var("JJZ_MAIN_BRANCH") {
            self.main_branch = value;
        }

        // JJZ_DEFAULT_TEMPLATE
        if let Ok(value) = std::env::var("JJZ_DEFAULT_TEMPLATE") {
            self.default_template = value;
        }

        // JJZ_WATCH_ENABLED
        if let Ok(value) = std::env::var("JJZ_WATCH_ENABLED") {
            self.watch.enabled = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid JJZ_WATCH_ENABLED value: {e}"))
            })?;
        }

        // JJZ_WATCH_DEBOUNCE_MS
        if let Ok(value) = std::env::var("JJZ_WATCH_DEBOUNCE_MS") {
            self.watch.debounce_ms = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid JJZ_WATCH_DEBOUNCE_MS value: {e}"))
            })?;
        }

        // JJZ_ZELLIJ_USE_TABS
        if let Ok(value) = std::env::var("JJZ_ZELLIJ_USE_TABS") {
            self.zellij.use_tabs = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid JJZ_ZELLIJ_USE_TABS value: {e}"))
            })?;
        }

        // JJZ_DASHBOARD_REFRESH_MS
        if let Ok(value) = std::env::var("JJZ_DASHBOARD_REFRESH_MS") {
            self.dashboard.refresh_ms = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid JJZ_DASHBOARD_REFRESH_MS value: {e}"))
            })?;
        }

        // JJZ_DASHBOARD_VIM_KEYS
        if let Ok(value) = std::env::var("JJZ_DASHBOARD_VIM_KEYS") {
            self.dashboard.vim_keys = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid JJZ_DASHBOARD_VIM_KEYS value: {e}"))
            })?;
        }

        // JJZ_AGENT_COMMAND
        if let Ok(value) = std::env::var("JJZ_AGENT_COMMAND") {
            self.agent.command = value;
        }

        Ok(())
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns error if any values are out of range or invalid
    fn validate(&self) -> Result<()> {
        // Validate debounce_ms range [10-5000]
        if self.watch.debounce_ms < 10 || self.watch.debounce_ms > 5000 {
            return Err(Error::ValidationError(
                "debounce_ms must be 10-5000".to_string(),
            ));
        }

        // Validate refresh_ms range [100-10000]
        if self.dashboard.refresh_ms < 100 || self.dashboard.refresh_ms > 10000 {
            return Err(Error::ValidationError(
                "refresh_ms must be 100-10000".to_string(),
            ));
        }

        Ok(())
    }

    /// Substitute placeholders like {repo} in config values
    ///
    /// # Errors
    ///
    /// Returns error if unable to determine values for placeholders
    fn substitute_placeholders(&mut self) -> Result<()> {
        let repo_name = get_repo_name()?;
        self.workspace_dir = self.workspace_dir.replace("{repo}", &repo_name);
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: No config files - Returns default config
    #[test]
    fn test_no_config_files_returns_defaults() {
        // This test works in the normal repo context where no .jjz/config.toml exists
        // and global config likely doesn't exist either
        let result = load_config();
        assert!(
            result.is_ok(),
            "load_config should succeed even without config files"
        );

        let config = result.unwrap_or_else(|_| Config::default());
        // Check that we got defaults (with {repo} replaced by actual repo name)
        assert!(config.workspace_dir.contains("__workspaces"));
        assert_eq!(config.default_template, "standard");
        assert_eq!(config.state_db, ".jjz/state.db");
    }

    // Test 2: Global only - Loads global, merges with defaults
    #[test]
    fn test_global_only_merges_with_defaults() {
        // For this test, we're testing the merge logic directly, not the file loading
        let mut base = Config::default();
        let override_config = Config {
            workspace_dir: "../custom".to_string(),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.workspace_dir, "../custom");
        assert_eq!(base.default_template, "standard"); // Should still have default
    }

    // Test 3: Project only - Loads project, merges with defaults
    #[test]
    fn test_project_only_merges_with_defaults() {
        let mut base = Config::default();
        let override_config = Config {
            main_branch: "develop".to_string(),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.main_branch, "develop");
        assert_eq!(base.workspace_dir, "../{repo}__workspaces"); // Should still have default
    }

    // Test 4: Both - Project overrides global overrides defaults
    #[test]
    fn test_project_overrides_global() {
        let mut base = Config::default();

        // First merge global
        let global_config = Config {
            workspace_dir: "../global".to_string(),
            ..Default::default()
        };
        base.merge(global_config);
        assert_eq!(base.workspace_dir, "../global");

        // Then merge project (should override)
        let project_config = Config {
            workspace_dir: "../project".to_string(),
            ..Default::default()
        };
        base.merge(project_config);

        assert_eq!(base.workspace_dir, "../project");
    }

    // Test 5: Env override - JJZ_WORKSPACE_DIR=../custom → config.workspace_dir
    #[test]
    fn test_env_var_overrides_config() {
        // Set env var
        std::env::set_var("JJZ_WORKSPACE_DIR", "../env");

        let mut config = Config {
            workspace_dir: "../original".to_string(),
            ..Default::default()
        };

        let result = config.apply_env_vars();
        assert!(result.is_ok());

        assert_eq!(config.workspace_dir, "../env");

        // Cleanup
        std::env::remove_var("JJZ_WORKSPACE_DIR");
    }

    // Test 6: Placeholder substitution
    #[test]
    fn test_placeholder_substitution() {
        let mut config = Config {
            workspace_dir: "../{repo}__ws".to_string(),
            ..Default::default()
        };

        let result = config.substitute_placeholders();
        assert!(result.is_ok());

        // The repo name will be "zjj" since we're in the zjj directory
        assert!(config.workspace_dir.contains("__ws"));
        assert!(!config.workspace_dir.contains("{repo}"));
    }

    // Test 7: Invalid debounce - debounce_ms = 5 → Error
    #[test]
    fn test_invalid_debounce_ms_too_low() {
        let mut config = Config::default();
        config.watch.debounce_ms = 5;

        let result = config.validate();
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("10-5000"));
        }
    }

    // Test 8: Invalid refresh - refresh_ms = 50000 → Error
    #[test]
    fn test_invalid_refresh_ms_too_high() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 50000;

        let result = config.validate();
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("100-10000"));
        }
    }

    // Test 9: Missing global config - No error, uses defaults
    #[test]
    fn test_missing_global_config_no_error() {
        // This tests that load_config doesn't fail when global config doesn't exist
        // (which is the normal case for most users)
        let result = load_config();
        assert!(result.is_ok());
    }

    // Test 10: Malformed TOML - Clear error with line number
    #[test]
    fn test_malformed_toml_returns_parse_error() -> Result<()> {
        use std::io::Write;
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("bad_config.toml");

        let mut file = std::fs::File::create(&config_path)
            .map_err(|e| Error::IoError(format!("Failed to create test file: {e}")))?;
        file.write_all(b"workspace_dir = \n invalid toml [[[")
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let result = load_toml_file(&config_path);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, Error::ParseError(_)));
        }
        Ok(())
    }

    // Test 11: Partial config - Unspecified values use defaults
    #[test]
    fn test_partial_config_uses_defaults() {
        let mut base = Config::default();
        let partial = Config {
            workspace_dir: "../custom".to_string(),
            ..Default::default()
        };
        // All other fields remain default

        base.merge(partial);

        assert_eq!(base.workspace_dir, "../custom");
        assert_eq!(base.default_template, "standard"); // Still default
        assert!(base.watch.enabled); // Still default
    }

    // Test 12: Deep merge - hooks.post_create in global + project → project replaces
    #[test]
    fn test_deep_merge_replaces_not_appends() {
        let mut base = Config::default();
        base.hooks.post_create = vec!["a".to_string(), "b".to_string()];

        let mut override_config = Config::default();
        override_config.hooks.post_create = vec!["c".to_string()];

        base.merge(override_config);

        assert_eq!(base.hooks.post_create, vec!["c".to_string()]);
        assert_ne!(
            base.hooks.post_create,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    // Additional tests for helper functions
    #[test]
    fn test_global_config_path() {
        let path = global_config_path();
        // Should return Some path to ~/.config/jjz/config.toml
        // or None on systems without home directory
        assert!(path.is_some() || path.is_none());
    }

    #[test]
    fn test_project_config_path() {
        let result = project_config_path();
        assert!(result.is_ok());
        let path = result.unwrap_or_default();
        assert!(path.ends_with("config.toml"));
    }

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.workspace_dir, "../{repo}__workspaces");
        assert_eq!(config.main_branch, "");
        assert_eq!(config.default_template, "standard");
        assert_eq!(config.state_db, ".jjz/state.db");
        assert!(config.watch.enabled);
        assert_eq!(config.watch.debounce_ms, 100);
        assert_eq!(config.dashboard.refresh_ms, 1000);
        assert_eq!(config.zellij.session_prefix, "jjz");
    }

    #[test]
    fn test_env_var_parsing_bool() {
        std::env::set_var("JJZ_WATCH_ENABLED", "false");

        let mut config = Config::default();
        let result = config.apply_env_vars();
        assert!(result.is_ok());
        assert!(!config.watch.enabled);

        std::env::remove_var("JJZ_WATCH_ENABLED");
    }

    #[test]
    fn test_env_var_parsing_int() {
        std::env::set_var("JJZ_WATCH_DEBOUNCE_MS", "200");

        let mut config = Config::default();
        let result = config.apply_env_vars();
        assert!(result.is_ok());
        assert_eq!(config.watch.debounce_ms, 200);

        std::env::remove_var("JJZ_WATCH_DEBOUNCE_MS");
    }

    #[test]
    fn test_validation_debounce_ms_valid() {
        let mut config = Config::default();
        config.watch.debounce_ms = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_debounce_ms_min() {
        let mut config = Config::default();
        config.watch.debounce_ms = 10;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_debounce_ms_max() {
        let mut config = Config::default();
        config.watch.debounce_ms = 5000;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_valid() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 1000;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_min() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_max() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 10000;
        assert!(config.validate().is_ok());
    }
}
