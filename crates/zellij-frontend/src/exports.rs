//! Zellij plugin exports - C ABI entry points for Zellij to call
//!
//! This module provides the exported functions that Zellij expects from WASM plugins.
//! These functions use `#[unsafe(no_mangle)]` and `extern "C"` to ensure they're exported
//! with the correct names and calling convention.

use crate::plugin::{OyaPlugin, PluginEvent, PluginInfo, Size};
use serde_json::json;

/// Global plugin instance (using unsafe static for WASM compatibility)
static mut PLUGIN: Option<OyaPlugin> = None;

/// Global buffer for rendered output
static mut RENDER_OUTPUT: Option<String> = None;

/// Initialize the plugin with serialized configuration
///
/// Zellij calls this when the plugin is first loaded.
/// The config is a JSON string with plugin configuration including size.
///
/// # Safety
/// - `config` must be a valid pointer to a null-terminated UTF-8 string
/// - `config_len` must match the length of the string at `config`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load(config: *const u8, config_len: usize) {
    // Parse configuration JSON
    let config_slice = unsafe { std::slice::from_raw_parts(config, config_len) };
    let config_str = String::from_utf8_lossy(config_slice);

    // Parse plugin info (size, etc.)
    let config_value: serde_json::Value = match serde_json::from_str(&config_str) {
        Ok(v) => v,
        Err(_) => {
            // Use defaults if parsing fails
            json!({"size": {"rows": 24, "cols": 80}})
        }
    };

    // Extract size from config
    let size = config_value
        .get("size")
        .and_then(|v| {
            Some(Size {
                rows: v.get("rows")?.as_u64()? as usize,
                cols: v.get("cols")?.as_u64()? as usize,
            })
        })
        .unwrap_or(Size { rows: 24, cols: 80 });

    // Create plugin info
    let info = PluginInfo {
        size,
        config: config_value,
    };

    // Create plugin instance
    match OyaPlugin::new() {
        Ok(mut plugin) => {
            // Initialize plugin with info
            let _ = plugin.start(info);
            unsafe {
                PLUGIN = Some(plugin);
            }
        }
        Err(e) => {
            // Log error but continue with minimal state
            eprintln!("Plugin initialization error: {}", e);
        }
    }
}

/// Update the plugin state with a user input event
///
/// Zellij calls this when there's user input (keyboard, mouse, etc.).
///
/// # Safety
/// - `input` must be a valid pointer to a null-terminated UTF-8 string
/// - `input_len` must match the length of the string at `input`
/// - `buffer` must be a valid pointer writable for at least `buffer_len` bytes
/// - `buffer_len` must be the size of the buffer
///
/// Returns: number of bytes written to the buffer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update(
    input: *const u8,
    input_len: usize,
    buffer: *mut u8,
    buffer_len: usize,
) -> usize {
    let plugin = &raw mut PLUGIN;

    // Check if plugin exists using raw pointer
    if unsafe { (*plugin).is_none() } {
        return 0;
    }

    // Parse input as JSON event
    let input_slice = unsafe { std::slice::from_raw_parts(input, input_len) };
    let input_str = String::from_utf8_lossy(input_slice);

    // Try to parse as plugin event
    if let Ok(event_value) = serde_json::from_str::<serde_json::Value>(&input_str) {
        // Convert JSON to PluginEvent
        if let Some(p) = unsafe { &mut *plugin } {
            if let Ok(event) = json_to_plugin_event(event_value) {
                let _ = p.handle_event(event);
            }
        }
    }

    // Get rendered output
    let output = unsafe { get_render_output() };

    // Copy to buffer if space allows
    let output_bytes = output.as_bytes();
    if !buffer.is_null() && buffer_len > 0 {
        let copy_len = output_bytes.len().min(buffer_len);
        unsafe {
            std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), buffer, copy_len);
        }
        copy_len
    } else {
        output_bytes.len()
    }
}

/// Render the current plugin state without updating
///
/// Zellij calls this to refresh the display without user input.
///
/// # Safety
/// - `buffer` must be a valid pointer writable for at least `buffer_len` bytes
/// - `buffer_len` must be the size of the buffer
///
/// Returns: number of bytes written to the buffer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn render(buffer: *mut u8, buffer_len: usize) -> usize {
    // Get rendered output
    let output = unsafe { get_render_output() };

    // Copy to buffer if space allows
    let output_bytes = output.as_bytes();
    if !buffer.is_null() && buffer_len > 0 {
        let copy_len = output_bytes.len().min(buffer_len);
        unsafe {
            std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), buffer, copy_len);
        }
        copy_len
    } else {
        output_bytes.len()
    }
}

/// Clean up plugin resources
///
/// Zellij calls this when unloading the plugin.
///
/// # Safety
/// This function is unsafe because it modifies global static state.
/// Should only be called once when plugin is being unloaded.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unload() {
    unsafe {
        PLUGIN = None;
        RENDER_OUTPUT = None;
    }
}

/// Get the current rendered output from the plugin
///
/// # Safety
/// This function is unsafe because it reads and modifies global static state.
unsafe fn get_render_output() -> String {
    // Check cached render output using raw pointer
    let output_ptr = &raw mut RENDER_OUTPUT;

    if let Some(output) = unsafe { &*output_ptr } {
        return output.clone();
    }

    // For now, return a simple output since we can't easily access the plugin
    "OYA SDLC Plugin\n\
    \n\
    Press 'q' to quit, '?' for help"
        .to_string()
}

/// Convert JSON value to PluginEvent
fn json_to_plugin_event(
    value: serde_json::Value,
) -> Result<PluginEvent, Box<dyn std::error::Error>> {
    use crate::plugin::{KeyModifiers, MouseButton, MouseEvent};

    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or("Missing 'kind' field")?;

    match kind {
        "start" => {
            let info_value = value.get("info").ok_or("Missing 'info'")?;
            let info: PluginInfo = serde_json::from_value(info_value.clone())?;
            Ok(PluginEvent::Start { info })
        }
        "resize" => {
            let size_value = value.get("size").ok_or("Missing 'size'")?;
            let rows = size_value
                .get("rows")
                .and_then(|r| r.as_u64())
                .unwrap_or(24) as usize;
            let cols = size_value
                .get("cols")
                .and_then(|c| c.as_u64())
                .unwrap_or(80) as usize;
            Ok(PluginEvent::Resize {
                size: Size { rows, cols },
            })
        }
        "key" => {
            let key = value
                .get("key")
                .and_then(|k| k.as_str())
                .ok_or("Missing 'key'")?
                .chars()
                .next()
                .ok_or("Empty key")?;

            let mods_value = value.get("modifiers").ok_or("Missing 'modifiers'")?;
            let shift = mods_value
                .get("shift")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);
            let ctrl = mods_value
                .get("ctrl")
                .and_then(|c| c.as_bool())
                .unwrap_or(false);
            let alt = mods_value
                .get("alt")
                .and_then(|a| a.as_bool())
                .unwrap_or(false);

            Ok(PluginEvent::Key {
                key,
                modifiers: KeyModifiers { shift, ctrl, alt },
            })
        }
        "mouse" => {
            let event_value = value.get("event").ok_or("Missing 'event'")?;
            let row = event_value.get("row").and_then(|r| r.as_u64()).unwrap_or(0) as usize;
            let col = event_value.get("col").and_then(|c| c.as_u64()).unwrap_or(0) as usize;

            let button_str = event_value
                .get("button")
                .and_then(|b| b.as_str())
                .unwrap_or("left");

            let button = match button_str {
                "left" => MouseButton::Left,
                "middle" => MouseButton::Middle,
                "right" => MouseButton::Right,
                "scroll_up" => MouseButton::ScrollUp,
                "scroll_down" => MouseButton::ScrollDown,
                _ => MouseButton::Left,
            };

            Ok(PluginEvent::Mouse {
                event: MouseEvent { row, col, button },
            })
        }
        "timer" => Ok(PluginEvent::Timer),
        _ => Err(format!("Unknown event kind: {}", kind).into()),
    }
}
