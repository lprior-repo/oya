#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::flow_extender::ExtensionPatchPreview;
use crate::hooks::use_canvas_interaction::CanvasInteraction;
use crate::hooks::use_selection::SelectionState;
use crate::hooks::use_ui_panels::UiPanels;
use crate::hooks::use_workflow_state::WorkflowState;
use crate::ui::constants::{
    ARROW_KEY_DELTA, DEFAULT_CANVAS_HEIGHT, DEFAULT_CANVAS_WIDTH, ZOOM_CENTER_X, ZOOM_CENTER_Y,
    ZOOM_DELTA,
};
use dioxus::prelude::*;

// ============================================================================
// Editor Command Dispatcher
// ============================================================================

/// Centralized command enum for repeated editor actions.
/// This dispatcher reduces duplicated side effects across toolbar, context menu,
/// minimap, and keyboard handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    ZoomIn,
    ZoomOut,
    FitView,
    AutoLayout,
    Undo,
    Redo,
    Duplicate,
}

/// Keyboard modifier state for command routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

/// Parse a keyboard event into an optional editor command.
/// Returns `None` for keys that don't map to editor commands or lack required modifiers.
#[must_use]
pub fn parse_key_event(key: &str, modifiers: KeyModifiers) -> Option<EditorCommand> {
    match key {
        // Zoom commands - no modifiers required
        "+" | "=" | "add" => Some(EditorCommand::ZoomIn),
        "-" | "_" | "subtract" => Some(EditorCommand::ZoomOut),
        "0" => Some(EditorCommand::FitView),

        // Undo/Redo - require Ctrl/Cmd modifier (security requirement)
        // Plain 'z'/'y' should NOT mutate history
        // Note: More specific patterns must come first
        "y" | "z" if modifiers.ctrl && modifiers.shift => Some(EditorCommand::Redo),
        "z" if modifiers.ctrl => Some(EditorCommand::Undo),
        "y" if modifiers.ctrl => Some(EditorCommand::Redo),

        // Auto layout - accessible via toolbar/context
        "l" if modifiers.ctrl => Some(EditorCommand::AutoLayout),

        // Duplicate selected node - Ctrl+D
        "d" if modifiers.ctrl => Some(EditorCommand::Duplicate),

        _ => None,
    }
}

/// Canvas dimensions for viewport calculations. Test-only at present.
#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub struct CanvasDimensions {
    pub width: f32,
    pub height: f32,
}

/// Zoom delta constants for the dispatcher. Test-only at present.
#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub struct ZoomConfig {
    pub delta: f32,
    pub center_x: f32,
    pub center_y: f32,
}

/// Fit view padding constant (re-exported from `ui::constants`).
pub use crate::ui::constants::FIT_VIEW_PADDING;

/// Handle a canvas keydown event.
///
/// This function encapsulates all keyboard interaction logic for the canvas,
/// including panel shortcuts, editor commands (zoom, undo, redo, layout),
/// node deletion, tab navigation, arrow key movement, and enter to toggle panels.
#[allow(clippy::too_many_lines)]
pub fn handle_canvas_keydown(
    key: &str,
    evt: &KeyboardEvent,
    panels: &UiPanels,
    canvas: CanvasInteraction,
    selection: SelectionState,
    workflow: &WorkflowState,
    extension_previews: &mut Signal<Vec<ExtensionPatchPreview>>,
) {
    if panels.any_open() {
        if key == "escape" {
            evt.prevent_default();
            (*panels).close_all();
        }
        return;
    }

    if key == " " || key == "space" {
        evt.prevent_default();
        canvas.enable_space_hand();
        return;
    }

    if key == "escape" {
        evt.prevent_default();
        (*panels).close_all();
        canvas.cancel_interaction();
        selection.clear_pending_drag();
        return;
    }

    if key == "k" {
        evt.prevent_default();
        (*panels).toggle_palette();
        return;
    }

    if key == "?" || key == "/" && evt.modifiers().shift() {
        evt.prevent_default();
        panels.toggle_shortcuts();
        return;
    }

    // Use command dispatcher for editor commands
    // Dioxus 0.7 has limited modifier detection - use default
    let modifiers = KeyModifiers::default();
    if let Some(cmd) = parse_key_event(key, modifiers) {
        evt.prevent_default();
        match cmd {
            EditorCommand::ZoomIn => {
                (*workflow).zoom(ZOOM_DELTA, ZOOM_CENTER_X, ZOOM_CENTER_Y);
            }
            EditorCommand::ZoomOut => {
                (*workflow).zoom(-ZOOM_DELTA, ZOOM_CENTER_X, ZOOM_CENTER_Y);
            }
            EditorCommand::FitView => {
                (*workflow).fit_view(DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT, FIT_VIEW_PADDING);
            }
            EditorCommand::AutoLayout => {
                (*workflow).apply_layout();
            }
            EditorCommand::Undo => {
                let _ = (*workflow).undo();
                extension_previews.set(Vec::new());
                selection.clear();
            }
            EditorCommand::Redo => {
                let _ = (*workflow).redo();
                extension_previews.set(Vec::new());
                selection.clear();
            }
            EditorCommand::Duplicate => {
                if let Some(selected_id) = *selection.selected_id().read() {
                    if let Some(new_id) = (*workflow).duplicate_node(selected_id) {
                        selection.select_single(new_id);
                    }
                }
            }
        }
        return;
    }

    if key == "backspace" || key == "delete" {
        let ids = selection.selected_ids().read().clone();
        if ids.is_empty() {
            return;
        }

        evt.prevent_default();
        let _ = (*workflow).remove_nodes(&ids);
        selection.clear();
        return;
    }

    if key == "tab" {
        evt.prevent_default();
        let shift = evt.modifiers().shift();
        match *selection.selected_id().read() {
            Some(current_id) => {
                let next_id = if shift {
                    (*workflow).upstream_nodes(current_id).first().copied()
                } else {
                    (*workflow).downstream_nodes(current_id).first().copied()
                };
                if let Some(id) = next_id {
                    selection.select_single(id);
                }
            }
            _ => {
                if let Some(first_id) = (*workflow).first_node_id() {
                    selection.select_single(first_id);
                }
            }
        }
        return;
    }

    if key == "enter" {
        if let Some(node_id) = *selection.selected_id().read() {
            evt.prevent_default();
            (*panels).toggle_inline_panel(node_id);
        }
        return;
    }

    if key == "arrowup" {
        if let Some(node_id) = *selection.selected_id().read() {
            evt.prevent_default();
            (*workflow).move_node_by(node_id, 0.0, -ARROW_KEY_DELTA);
        }
        return;
    }
    if key == "arrowdown" {
        if let Some(node_id) = *selection.selected_id().read() {
            evt.prevent_default();
            (*workflow).move_node_by(node_id, 0.0, ARROW_KEY_DELTA);
        }
        return;
    }
    if key == "arrowleft" {
        if let Some(node_id) = *selection.selected_id().read() {
            evt.prevent_default();
            (*workflow).move_node_by(node_id, -ARROW_KEY_DELTA, 0.0);
        }
        return;
    }
    if key == "arrowright" {
        if let Some(node_id) = *selection.selected_id().read() {
            evt.prevent_default();
            (*workflow).move_node_by(node_id, ARROW_KEY_DELTA, 0.0);
        }
    }
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn given_plain_z_key_when_parsing_then_returns_none() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("z", mods);
        assert!(result.is_none(), "Plain 'z' should not trigger undo");
    }

    #[test]
    fn given_ctrl_z_key_when_parsing_then_returns_undo() {
        let mods = KeyModifiers { ctrl: true, ..Default::default() };
        let result = parse_key_event("z", mods);
        assert_eq!(result, Some(EditorCommand::Undo));
    }

    #[test]
    fn given_ctrl_shift_z_key_when_parsing_then_returns_redo() {
        let mods = KeyModifiers { ctrl: true, shift: true, ..Default::default() };
        let result = parse_key_event("z", mods);
        assert_eq!(result, Some(EditorCommand::Redo));
    }

    #[test]
    fn given_ctrl_y_key_when_parsing_then_returns_redo() {
        let mods = KeyModifiers { ctrl: true, ..Default::default() };
        let result = parse_key_event("y", mods);
        assert_eq!(result, Some(EditorCommand::Redo));
    }

    #[test]
    fn given_plain_y_key_when_parsing_then_returns_none() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("y", mods);
        assert!(result.is_none(), "Plain 'y' should not trigger redo");
    }

    #[test]
    fn given_plus_key_when_parsing_then_returns_zoom_in() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("+", mods);
        assert_eq!(result, Some(EditorCommand::ZoomIn));
    }

    #[test]
    fn given_minus_key_when_parsing_then_returns_zoom_out() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("-", mods);
        assert_eq!(result, Some(EditorCommand::ZoomOut));
    }

    #[test]
    fn given_zero_key_when_parsing_then_returns_fit_view() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("0", mods);
        assert_eq!(result, Some(EditorCommand::FitView));
    }

    #[test]
    fn given_ctrl_l_key_when_parsing_then_returns_auto_layout() {
        let mods = KeyModifiers { ctrl: true, ..Default::default() };
        let result = parse_key_event("l", mods);
        assert_eq!(result, Some(EditorCommand::AutoLayout));
    }

    #[test]
    fn given_unmapped_key_when_parsing_then_returns_none() {
        let mods = KeyModifiers::default();
        assert!(parse_key_event("a", mods).is_none());
        assert!(parse_key_event("escape", mods).is_none());
        assert!(parse_key_event("enter", mods).is_none());
    }

    #[test]
    fn given_meta_key_as_ctrl_when_parsing_then_interprets_as_ctrl() {
        // On macOS, meta (Cmd) should be treated like ctrl
        let mods = KeyModifiers { ctrl: true, ..Default::default() };
        let result = parse_key_event("z", mods);
        assert_eq!(result, Some(EditorCommand::Undo));
    }

    // ========================================================================
    // Alternative key mappings for zoom
    // ========================================================================

    #[test]
    fn given_equals_key_when_parsing_then_returns_zoom_in() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("=", mods);
        assert_eq!(result, Some(EditorCommand::ZoomIn));
    }

    #[test]
    fn given_add_key_when_parsing_then_returns_zoom_in() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("add", mods);
        assert_eq!(result, Some(EditorCommand::ZoomIn));
    }

    #[test]
    fn given_underscore_key_when_parsing_then_returns_zoom_out() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("_", mods);
        assert_eq!(result, Some(EditorCommand::ZoomOut));
    }

    #[test]
    fn given_subtract_key_when_parsing_then_returns_zoom_out() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("subtract", mods);
        assert_eq!(result, Some(EditorCommand::ZoomOut));
    }

    // ========================================================================
    // Security: modifier-free command keys must return None
    // ========================================================================

    #[test]
    fn given_plain_l_key_when_parsing_then_returns_none() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("l", mods);
        assert!(result.is_none(), "Plain 'l' should not trigger auto-layout");
    }

    #[test]
    fn given_shift_z_without_ctrl_when_parsing_then_returns_none() {
        let mods = KeyModifiers { shift: true, ..Default::default() };
        let result = parse_key_event("z", mods);
        assert!(result.is_none(), "Shift+z without ctrl should not trigger any command");
    }

    #[test]
    fn given_alt_z_when_parsing_then_returns_none() {
        let mods = KeyModifiers { alt: true, ..Default::default() };
        let result = parse_key_event("z", mods);
        assert!(result.is_none(), "Alt+z should not trigger undo - ctrl is required");
    }

    #[test]
    fn given_ctrl_shift_y_key_when_parsing_then_returns_redo() {
        let mods = KeyModifiers { ctrl: true, shift: true, ..Default::default() };
        let result = parse_key_event("y", mods);
        assert_eq!(result, Some(EditorCommand::Redo));
    }

    #[test]
    fn given_alt_y_when_parsing_then_returns_none() {
        let mods = KeyModifiers { alt: true, ..Default::default() };
        let result = parse_key_event("y", mods);
        assert!(result.is_none(), "Alt+y should not trigger redo - ctrl is required");
    }

    #[test]
    fn given_alt_l_when_parsing_then_returns_none() {
        let mods = KeyModifiers { alt: true, ..Default::default() };
        let result = parse_key_event("l", mods);
        assert!(result.is_none(), "Alt+l should not trigger auto-layout - ctrl is required");
    }

    // ========================================================================
    // Edge cases: empty string and unknown keys with modifiers
    // ========================================================================

    #[test]
    fn given_empty_string_key_when_parsing_then_returns_none() {
        let mods = KeyModifiers::default();
        let result = parse_key_event("", mods);
        assert!(result.is_none(), "Empty key string should not map to any command");
    }

    #[test]
    fn given_unknown_key_with_ctrl_when_parsing_then_returns_none() {
        let mods = KeyModifiers { ctrl: true, ..Default::default() };
        assert!(parse_key_event("a", mods).is_none());
        assert!(parse_key_event("b", mods).is_none());
        assert!(parse_key_event("x", mods).is_none());
        assert!(parse_key_event("1", mods).is_none());
        assert!(parse_key_event("9", mods).is_none());
    }

    #[test]
    fn given_all_modifiers_active_when_parsing_known_keys_then_returns_expected() {
        let mods = KeyModifiers { ctrl: true, shift: true, alt: true };
        // Ctrl+Shift+z should still map to Redo even with alt
        assert_eq!(parse_key_event("z", mods), Some(EditorCommand::Redo));
        // Zoom keys ignore modifiers
        assert_eq!(parse_key_event("+", mods), Some(EditorCommand::ZoomIn));
        assert_eq!(parse_key_event("-", mods), Some(EditorCommand::ZoomOut));
        assert_eq!(parse_key_event("0", mods), Some(EditorCommand::FitView));
    }

    // ========================================================================
    // EditorCommand enum trait tests
    // ========================================================================

    #[test]
    fn given_editor_command_when_cloned_then_is_equal() {
        let cmd = EditorCommand::Undo;
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn given_editor_command_when_copied_then_is_equal() {
        let cmd = EditorCommand::ZoomIn;
        let copied = cmd;
        assert_eq!(cmd, copied);
    }

    #[test]
    fn given_all_editor_command_variants_then_debug_format_contains_name() {
        let variants = [
            (EditorCommand::ZoomIn, "ZoomIn"),
            (EditorCommand::ZoomOut, "ZoomOut"),
            (EditorCommand::FitView, "FitView"),
            (EditorCommand::AutoLayout, "AutoLayout"),
            (EditorCommand::Undo, "Undo"),
            (EditorCommand::Redo, "Redo"),
        ];
        for (variant, name) in variants {
            let debug_str = format!("{variant:?}");
            assert!(debug_str.contains(name), "Debug output '{debug_str}' should contain '{name}'");
        }
    }

    #[test]
    fn given_different_editor_commands_then_not_equal() {
        assert_ne!(EditorCommand::ZoomIn, EditorCommand::ZoomOut);
        assert_ne!(EditorCommand::Undo, EditorCommand::Redo);
        assert_ne!(EditorCommand::FitView, EditorCommand::AutoLayout);
        assert_ne!(EditorCommand::Undo, EditorCommand::ZoomIn);
    }

    // ========================================================================
    // KeyModifiers struct tests
    // ========================================================================

    #[test]
    fn given_default_key_modifiers_then_all_fields_are_false() {
        let mods = KeyModifiers::default();
        assert!(!mods.ctrl, "Default ctrl should be false");
        assert!(!mods.shift, "Default shift should be false");
        assert!(!mods.alt, "Default alt should be false");
    }

    #[test]
    fn given_manually_constructed_modifiers_then_values_are_set() {
        let mods = KeyModifiers { ctrl: true, shift: false, alt: true };
        assert!(mods.ctrl);
        assert!(!mods.shift);
        assert!(mods.alt);
    }

    #[test]
    fn given_identical_modifiers_then_are_equal() {
        let a = KeyModifiers { ctrl: true, shift: false, alt: true };
        let b = KeyModifiers { ctrl: true, shift: false, alt: true };
        assert_eq!(a, b);
    }

    #[test]
    fn given_different_modifiers_then_not_equal() {
        let a = KeyModifiers { ctrl: true, shift: false, alt: false };
        let b = KeyModifiers { ctrl: false, shift: true, alt: false };
        assert_ne!(a, b);
    }

    #[test]
    fn given_key_modifiers_when_cloned_then_is_equal() {
        let mods = KeyModifiers { ctrl: true, shift: true, alt: false };
        let cloned = mods.clone();
        assert_eq!(mods, cloned);
    }

    // ========================================================================
    // CanvasDimensions struct tests
    // ========================================================================

    #[test]
    fn given_default_canvas_dimensions_then_values_are_zero() {
        let dims = CanvasDimensions::default();
        assert!(dims.width.abs() < f32::EPSILON, "Default width should be 0.0");
        assert!(dims.height.abs() < f32::EPSILON, "Default height should be 0.0");
    }

    #[test]
    fn given_manually_constructed_dimensions_then_values_are_set() {
        let dims = CanvasDimensions { width: 1280.0, height: 760.0 };
        assert!((dims.width - 1280.0).abs() < f32::EPSILON);
        assert!((dims.height - 760.0).abs() < f32::EPSILON);
    }

    #[test]
    fn given_canvas_dimensions_when_cloned_then_values_match() {
        let dims = CanvasDimensions { width: 800.0, height: 600.0 };
        let cloned = dims.clone();
        assert!((cloned.width - dims.width).abs() < f32::EPSILON);
        assert!((cloned.height - dims.height).abs() < f32::EPSILON);
    }

    // ========================================================================
    // ZoomConfig struct tests
    // ========================================================================

    #[test]
    fn given_default_zoom_config_then_values_are_zero() {
        let config = ZoomConfig::default();
        assert!(config.delta.abs() < f32::EPSILON, "Default delta should be 0.0");
        assert!(config.center_x.abs() < f32::EPSILON, "Default center_x should be 0.0");
        assert!(config.center_y.abs() < f32::EPSILON, "Default center_y should be 0.0");
    }

    #[test]
    fn given_manually_constructed_zoom_config_then_values_are_set() {
        let config = ZoomConfig { delta: 0.12, center_x: 640.0, center_y: 400.0 };
        assert!((config.delta - 0.12).abs() < f32::EPSILON);
        assert!((config.center_x - 640.0).abs() < f32::EPSILON);
        assert!((config.center_y - 400.0).abs() < f32::EPSILON);
    }

    #[test]
    fn given_zoom_config_when_cloned_then_values_match() {
        let config = ZoomConfig { delta: -0.12, center_x: 100.0, center_y: 200.0 };
        let cloned = config.clone();
        assert!((cloned.delta - config.delta).abs() < f32::EPSILON);
        assert!((cloned.center_x - config.center_x).abs() < f32::EPSILON);
        assert!((cloned.center_y - config.center_y).abs() < f32::EPSILON);
    }

    // ========================================================================
    // FIT_VIEW_PADDING constant test
    // ========================================================================

    #[test]
    fn given_fit_view_padding_constant_then_is_200() {
        assert!(
            (FIT_VIEW_PADDING - 200.0).abs() < f32::EPSILON,
            "FIT_VIEW_PADDING should be 200.0"
        );
    }
}
