#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use zellij_frontend::layout::PaneType;
use zellij_frontend::plugin::{KeyModifiers, OyaPlugin, PluginEvent, PluginState};

/// Test: Ctrl+Shift+G switches to WorkflowGraph view
#[test]
fn test_ctrl_shift_g_switches_to_graph_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = PluginState::Running;

    // Simulate Ctrl+Shift+G key press
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: true,
            alt: false,
        },
    });

    // Verify focus switched to WorkflowGraph
    assert_eq!(plugin.focused_pane(), PaneType::WorkflowGraph);

    Ok(())
}

/// Test: Ctrl+Shift+L switches to BeadList view
#[test]
fn test_ctrl_shift_l_switches_to_list_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = PluginState::Running;

    // Simulate Ctrl+Shift+L key press
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'l',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: true,
            alt: false,
        },
    });

    // Verify focus switched to BeadList
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    Ok(())
}

/// Test: Ctrl+Shift without all modifiers ignored for view switching
#[test]
fn test_ctrl_shift_requires_both_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = PluginState::Running;

    // Verify initial state
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    // Simulate just Ctrl+G (without Shift) - should trigger refresh_tasks, not change focus
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: true,
            alt: false,
        },
    });

    // Verify focus did NOT change (requires both Ctrl and Shift for view switch)
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    // Simulate just Shift+G (without Ctrl) - should trigger refresh_tasks, not change focus
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });

    // Verify focus did NOT change
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    Ok(())
}

/// Test: Tab cycles through all 4 panes forward
#[test]
fn test_tab_cycles_through_all_panes_forward() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = PluginState::Running;

    // Verify initial state
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    // Tab once → BeadDetail
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadDetail);

    // Tab again → PipelineView
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane(), PaneType::PipelineView);

    // Tab again → WorkflowGraph
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane(), PaneType::WorkflowGraph);

    // Tab again → back to BeadList
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    Ok(())
}

/// Test: Shift+Tab cycles through all 4 panes backward
#[test]
fn test_shift_tab_cycles_through_all_panes_backward() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = PluginState::Running;

    // Verify initial state
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    // Shift+Tab once → WorkflowGraph
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane(), PaneType::WorkflowGraph);

    // Shift+Tab again → PipelineView
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane(), PaneType::PipelineView);

    // Shift+Tab again → BeadDetail
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadDetail);

    // Shift+Tab again → back to BeadList
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    Ok(())
}

/// Test: Q quits the plugin
#[test]
fn test_q_quits_plugin() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = PluginState::Running;

    // Press q
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'q',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });

    // Verify plugin state changed to ShuttingDown
    assert_eq!(plugin.plugin_state(), PluginState::ShuttingDown);

    Ok(())
}

/// Test: ? toggles help overlay
#[test]
fn test_question_mark_toggles_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = PluginState::Running;

    // Press ?
    let result = plugin.handle_event(PluginEvent::Key {
        key: '?',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });

    // Verify plugin state changed to HelpOverlay
    assert_eq!(plugin.plugin_state(), PluginState::HelpOverlay);
    assert!(result.is_ok());

    // Press ? again to close
    let result = plugin.handle_event(PluginEvent::Key {
        key: '?',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });

    // Verify plugin state changed back to Running
    assert_eq!(plugin.plugin_state(), PluginState::Running);
    assert!(result.is_ok());

    Ok(())
}

/// Test: ? toggles help overlay
#[test]
fn test_question_mark_toggles_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = PluginState::Running;

    // Press ?
    let result = plugin.handle_event(PluginEvent::Key {
        key: '?',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });

    // Verify plugin state changed to HelpOverlay
    assert_eq!(
        plugin.plugin_state(),
        zellij_frontend::plugin::PluginState::HelpOverlay
    );
    assert!(result.is_ok());

    // Press ? again to close
    let result = plugin.handle_event(PluginEvent::Key {
        key: '?',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });

    // Verify plugin state changed back to Running
    assert_eq!(plugin.plugin_state(), PluginState::Running);
    assert!(result.is_ok());

    Ok(())
}
