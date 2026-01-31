//! Configuration viewing and editing command

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use zjj_core::config::Config;

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct ConfigOptions {
    pub key: Option<String>,
    pub value: Option<String>,
    pub global: bool,
}

/// Execute the config command
///
/// # Errors
///
/// Returns error if:
/// - Config file cannot be read or parsed
/// - Config key is not found
/// - Config value cannot be set
/// - Invalid arguments provided
pub fn run(options: ConfigOptions) -> Result<()> {
    let config = zjj_core::config::load_config()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {e}"))?;

    match (options.key, options.value) {
        // No key, no value: Show all config
        (None, None) => {
            show_all_config(&config, options.global)?;
        }

        // Key, no value: Show specific value
        (Some(key), None) => {
            show_config_value(&config, &key)?;
        }

        // Key + value: Set value
        (Some(key), Some(value)) => {
            let config_path = if options.global {
                global_config_path()?
            } else {
                project_config_path()?
            };
            set_config_value(&config_path, &key, &value)?;
            println!("✓ Set {key} = {value}");
            if options.global {
                println!("  (in global config)");
            } else {
                println!("  (in project config)");
            }
        }

        // Value without key: Invalid
        (None, Some(_)) => {
            anyhow::bail!("Cannot set value without key");
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// VIEW OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Show all configuration
fn show_all_config(config: &Config, global_only: bool) -> Result<()> {
    // Serialize config to TOML
    let toml = toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;

    println!(
        "Current configuration{}:",
        if global_only {
            " (global)"
        } else {
            " (merged)"
        }
    );
    println!();
    println!("{toml}");

    if !global_only {
        println!();
        println!("Config sources:");
        println!("  1. Built-in defaults");
        if let Ok(global_path) = global_config_path() {
            println!("  2. Global: {}", global_path.display());
        }
        if let Ok(project_path) = project_config_path() {
            println!("  3. Project: {}", project_path.display());
        }
        println!("  4. Environment: JJZ_* variables");
    }

    Ok(())
}

/// Show a specific config value
fn show_config_value(config: &Config, key: &str) -> Result<()> {
    let value = get_nested_value(config, key)?;
    println!("{key} = {value}");
    Ok(())
}

/// Get a nested value from config using dot notation
fn get_nested_value(config: &Config, key: &str) -> Result<String> {
    // Convert config to JSON for easy nested access
    let json =
        serde_json::to_value(config).context("Failed to serialize config for value lookup")?;

    let parts: Vec<&str> = key.split('.').collect();

    // Navigate through nested keys using functional fold pattern
    let current = parts.iter().try_fold(&json, |current_value, &part| {
        current_value.get(part).ok_or_else(|| {
            anyhow::anyhow!("Config key '{key}' not found. Use 'jjz config' to see all keys.")
        })
    })?;

    // Format value based on type
    Ok(match current {
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(arr) => {
            // Format as TOML array: ["a", "b"]
            let items: Vec<String> = arr
                .iter()
                .map(|v| format!("\"{}\"", v.as_str().unwrap_or("")))
                .collect();
            format!("[{}]", items.join(", "))
        }
        _ => serde_json::to_string_pretty(current)
            .context("Failed to format complex config value")?,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// SET OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Set a config value in the specified config file
fn set_config_value(config_path: &Path, key: &str, value: &str) -> Result<()> {
    // Load existing config or create new
    let mut doc = if config_path.exists() {
        let content = std::fs::read_to_string(config_path).context(format!(
            "Failed to read config file {}",
            config_path.display()
        ))?;
        content
            .parse::<toml_edit::DocumentMut>()
            .context("Failed to parse config file as TOML")?
    } else {
        // Create parent directory if needed
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context(format!(
                "Failed to create config directory {}",
                parent.display()
            ))?;
        }
        toml_edit::DocumentMut::new()
    };

    // Parse dot notation and set value
    let parts: Vec<&str> = key.split('.').collect();
    set_nested_value(&mut doc, &parts, value)?;

    // Write back to file
    std::fs::write(config_path, doc.to_string()).context(format!(
        "Failed to write config file {}",
        config_path.display()
    ))?;

    Ok(())
}

/// Set a nested value in a TOML document using dot notation
fn set_nested_value(doc: &mut toml_edit::DocumentMut, parts: &[&str], value: &str) -> Result<()> {
    if parts.is_empty() {
        anyhow::bail!("Empty config key");
    }

    // Navigate to parent table and ensure all intermediate tables exist
    // Using fold to navigate through the path while maintaining table references
    let final_table =
        parts[..parts.len() - 1]
            .iter()
            .try_fold(doc.as_table_mut(), |current_table, &part| {
                // Ensure table exists
                if !current_table.contains_key(part) {
                    current_table[part] = toml_edit::table();
                }
                current_table[part]
                    .as_table_mut()
                    .ok_or_else(|| anyhow::anyhow!("{part} is not a table"))
            })?;

    // Set the value
    let key = parts
        .last()
        .ok_or_else(|| anyhow::anyhow!("Invalid key path"))?;
    let toml_value = parse_value(value)?;
    final_table[key] = toml_value;

    Ok(())
}

/// Parse a string value into a TOML value (bool, int, string, or array)
fn parse_value(value: &str) -> Result<toml_edit::Item> {
    // Try parsing as different types
    if value == "true" || value == "false" {
        let bool_val = value
            .parse::<bool>()
            .context("Failed to parse boolean value")?;
        Ok(toml_edit::value(bool_val))
    } else if let Ok(n) = value.parse::<i64>() {
        Ok(toml_edit::value(n))
    } else if value.starts_with('[') && value.ends_with(']') {
        // Parse array: ["a", "b"] or [1, 2]
        let items: Vec<&str> = value[1..value.len() - 1]
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .collect();
        let array = items.iter().map(|s| toml_edit::Value::from(*s)).collect();
        Ok(toml_edit::Item::Value(toml_edit::Value::Array(array)))
    } else {
        // Default to string
        Ok(toml_edit::value(value))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get path to global config file
fn global_config_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "jjz")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
        .ok_or_else(|| anyhow::anyhow!("Failed to determine global config directory"))
}

/// Get path to project config file
fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .context("Failed to get current directory")
        .map(|dir| dir.join(".jjz/config.toml"))
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    fn setup_test_config() -> Config {
        Config::default()
    }

    fn create_temp_config(content: &str) -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.toml");
        let mut file = std::fs::File::create(&config_path)?;
        file.write_all(content.as_bytes())?;
        Ok((temp_dir, config_path))
    }

    #[test]
    fn test_get_nested_value_simple() -> Result<()> {
        let config = setup_test_config();
        let value = get_nested_value(&config, "workspace_dir")?;
        assert_eq!(value, "../{repo}__workspaces");
        Ok(())
    }

    #[test]
    fn test_get_nested_value_nested() -> Result<()> {
        let config = setup_test_config();
        let value = get_nested_value(&config, "zellij.use_tabs")?;
        assert_eq!(value, "true");
        Ok(())
    }

    #[test]
    fn test_get_nested_value_deep() -> Result<()> {
        let config = setup_test_config();
        let value = get_nested_value(&config, "zellij.panes.main.command")?;
        assert_eq!(value, "claude");
        Ok(())
    }

    #[test]
    fn test_get_nested_value_not_found() {
        let config = setup_test_config();
        let result = get_nested_value(&config, "invalid.key");
        assert!(result.is_err(), "Expected an error but got Ok: {result:?}");
        if let Err(e) = result {
            assert!(e.to_string().contains("Config key 'invalid.key' not found"));
        }
    }

    #[test]
    fn test_get_nested_value_array() -> Result<()> {
        let config = setup_test_config();
        let value = get_nested_value(&config, "watch.paths")?;
        assert_eq!(value, r#"[".beads/beads.db"]"#);
        Ok(())
    }

    #[test]
    fn test_parse_value_bool_true() -> Result<()> {
        let item = parse_value("true")?;
        assert_eq!(item.to_string().trim(), "true");
        Ok(())
    }

    #[test]
    fn test_parse_value_bool_false() -> Result<()> {
        let item = parse_value("false")?;
        assert_eq!(item.to_string().trim(), "false");
        Ok(())
    }

    #[test]
    fn test_parse_value_int() -> Result<()> {
        let item = parse_value("42")?;
        assert_eq!(item.to_string().trim(), "42");
        Ok(())
    }

    #[test]
    fn test_parse_value_string() -> Result<()> {
        let item = parse_value("hello")?;
        assert_eq!(item.to_string().trim(), r#""hello""#);
        Ok(())
    }

    #[test]
    fn test_parse_value_array() -> Result<()> {
        let item = parse_value(r#"["a", "b", "c"]"#)?;
        let result = item.to_string();
        assert!(result.contains('a'));
        assert!(result.contains('b'));
        assert!(result.contains('c'));
        Ok(())
    }

    #[test]
    fn test_set_config_value_simple() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "workspace_dir", "../custom")?;

        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("workspace_dir"));
        assert!(content.contains("../custom"));
        Ok(())
    }

    #[test]
    fn test_set_config_value_nested() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "zellij.use_tabs", "false")?;

        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("[zellij]"));
        assert!(content.contains("use_tabs = false"));
        Ok(())
    }

    #[test]
    fn test_set_config_value_deep_nested() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "zellij.panes.main.command", "nvim")?;

        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("[zellij.panes.main]"));
        assert!(content.contains("command"));
        assert!(content.contains("nvim"));
        Ok(())
    }

    #[test]
    fn test_set_config_value_overwrite_existing() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config(r#"workspace_dir = "../old""#)?;
        set_config_value(&config_path, "workspace_dir", "../new")?;

        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("../new"));
        assert!(!content.contains("../old"));
        Ok(())
    }

    #[test]
    fn test_set_config_value_creates_parent_dir() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("subdir").join("config.toml");

        set_config_value(&config_path, "workspace_dir", "../test")?;

        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("workspace_dir"));
        Ok(())
    }

    #[test]
    fn test_set_nested_value_empty_parts() {
        let mut doc = toml_edit::DocumentMut::new();
        let result = set_nested_value(&mut doc, &[], "value");
        let has_error = result
            .as_ref()
            .map(|()| false)
            .unwrap_or_else(|e| e.to_string().contains("Empty config key"));
        assert!(has_error);
    }

    #[test]
    fn test_show_config_value() -> Result<()> {
        let config = setup_test_config();
        // Just test that it doesn't panic
        show_config_value(&config, "workspace_dir")?;
        Ok(())
    }

    #[test]
    fn test_show_all_config() -> Result<()> {
        let config = setup_test_config();
        // Just test that it doesn't panic
        show_all_config(&config, false)?;
        show_all_config(&config, true)?;
        Ok(())
    }

    #[test]
    fn test_project_config_path() -> Result<()> {
        let path = project_config_path()?;
        assert!(path.ends_with("config.toml"));
        assert!(path.to_string_lossy().contains(".jjz"));
        Ok(())
    }
}
