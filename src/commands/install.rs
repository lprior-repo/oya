//! Install command - installs Zellij WASM plugin

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Install Zellij WASM plugin
pub fn install_command(force: bool) -> Result<()> {
    let home = std::env::var("HOME").map_err(|e| anyhow::anyhow!("No HOME directory: {e}"))?;

    let zellij_dir = PathBuf::from(home).join(".config/zellij/plugins");

    fs::create_dir_all(&zellij_dir).with_context(|| {
        format!(
            "Failed to create plugins directory: {}",
            zellij_dir.display()
        )
    })?;

    let wasm_path = zellij_dir.join("zellij_frontend.wasm");
    let yaml_path = zellij_dir.join("oya.yaml");

    // Check if already installed
    if wasm_path.exists() && !force {
        println!(
            "✅ Zellij plugin already installed at {}",
            wasm_path.display()
        );
        println!("   Use --force to reinstall");
        return Ok(());
    }

    // Get WASM bytes
    let wasm_bytes = get_wasm_bytes()?;

    // Write WASM file
    fs::write(&wasm_path, wasm_bytes)
        .with_context(|| format!("Failed to write WASM file: {}", wasm_path.display()))?;

    // Write plugin YAML
    let plugin_yaml = r#"# OYA Zellij Plugin
layout {
    default_tab_name = "OYA"
    pane size=1 borderless=true {
        plugin location="file:///path/to/zellij_frontend.wasm" {
            _plugin_version = "0.91.0"
        }
    }
}
"#;

    fs::write(&yaml_path, plugin_yaml)
        .with_context(|| format!("Failed to write plugin YAML: {}", yaml_path.display()))?;

    println!("✅ Installed Zellij plugin to {}", zellij_dir.display());
    println!("   WASM: {}", wasm_path.display());
    println!("   Config: {}", yaml_path.display());

    Ok(())
}

fn get_wasm_bytes() -> Result<Vec<u8>> {
    // Try to find built WASM in target directory
    let possible_paths = [
        "../target/wasm32-wasip1/release/zellij_frontend.wasm",
        "./target/wasm32-wasip1/release/zellij_frontend.wasm",
        "target/wasm32-wasip1/release/zellij_frontend.wasm",
    ];

    for path in possible_paths {
        if let Ok(bytes) = fs::read(path) {
            return Ok(bytes);
        }
    }

    // If not found, provide helpful error
    Err(anyhow::anyhow!(
        "WASM plugin not found. Build it first with:\n  moon run :build-zellij\n\nSearched paths:\n{}",
        possible_paths.join("\n")
    ))
}
