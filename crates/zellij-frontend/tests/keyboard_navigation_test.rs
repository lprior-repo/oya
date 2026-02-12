#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use crate::plugin::{KeyModifiers, OyaPlugin, PluginEvent};

/// Test: Ctrl+Shift+A approves selected task
#[test]
fn test_ctrl_shift_a_approves_selected_task() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;
    plugin.focused_pane = crate::layout::PaneType::BeadList;

    // Add a task to select
    plugin.tasks = vec![crate::plugin::TaskRow::new(
        "test-task-1",
        "created",
        "P1",
        "Rust",
        "task/test-task-1",
    )];
    plugin.selected_index = 0;

    // Simulate Ctrl+Shift+A key press
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'a',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: true,
            alt: false,
        },
    });

    // Verify approve_selected was called (status message should indicate it)
    assert!(
        plugin
            .status_message
            .as_deref()
            .is_some_and(|msg| msg.contains("approve") || msg.contains("Approve"))
    );

    Ok(())
}

/// Test: Ctrl+Shift+G switches to WorkflowGraph view
#[test]
fn test_ctrl_shift_g_switches_to_graph_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;
    plugin.focused_pane = crate::layout::PaneType::BeadList;

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
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::WorkflowGraph);

    Ok(())
}

/// Test: Ctrl+Shift+L switches to BeadList view
#[test]
fn test_ctrl_shift_l_switches_to_list_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;
    plugin.focused_pane = crate::layout::PaneType::WorkflowGraph;

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
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::BeadList);

    Ok(())
}

/// Test: Ctrl+Shift without all modifiers ignored
#[test]
fn test_ctrl_shift_requires_both_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;
    let initial_focus = plugin.focused_pane;
    plugin.focused_pane = crate::layout::PaneType::BeadList;

    // Simulate just Ctrl+G (without Shift)
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: true,
            alt: false,
        },
    });

    // Verify focus did not change (requires both Ctrl and Shift)
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::BeadList);

    // Simulate just Shift+G (without Ctrl)
    plugin.focused_pane = crate::layout::PaneType::BeadList;
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });

    // Verify focus did not change
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::BeadList);

    Ok(())
}

/// Test: Tab cycles through all 4 panes forward
#[test]
fn test_tab_cycles_through_all_panes_forward() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;

    // Start at BeadList
    plugin.focused_pane = crate::layout::PaneType::BeadList;

    // Tab once → BeadDetail
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::BeadDetail);

    // Tab again → PipelineView
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::PipelineView);

    // Tab again → WorkflowGraph
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::WorkflowGraph);

    // Tab again → back to BeadList
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::BeadList);

    Ok(())
}

/// Test: Shift+Tab cycles through all 4 panes backward
#[test]
fn test_shift_tab_cycles_through_all_panes_backward() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;

    // Start at BeadList
    plugin.focused_pane = crate::layout::PaneType::BeadList;

    // Shift+Tab once → WorkflowGraph
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::WorkflowGraph);

    // Shift+Tab again → PipelineView
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::PipelineView);

    // Shift+Tab again → BeadDetail
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::BeadDetail);

    // Shift+Tab again → back to BeadList
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.focused_pane, crate::layout::PaneType::BeadList);

    Ok(())
}

/// Test: Vim j/k navigation moves selection
#[test]
fn test_vim_j_k_navigation() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;
    plugin.focused_pane = crate::layout::PaneType::BeadList;

    // Add multiple tasks
    plugin.tasks = vec![
        crate::plugin::TaskRow::new("task-1", "created", "P1", "Rust", "task/1"),
        crate::plugin::TaskRow::new("task-2", "created", "P1", "Rust", "task/2"),
        crate::plugin::TaskRow::new("task-3", "created", "P1", "Rust", "task/3"),
    ];
    plugin.selected_index = 0;

    // Press j → move down
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'j',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.selected_index, 1);

    // Press j again → move down again
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'j',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.selected_index, 2);

    // Press k → move up
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'k',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.selected_index, 1);

    Ok(())
}

/// Test: j/k navigation wraps around at boundaries
#[test]
fn test_vim_navigation_wraps_at_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;
    plugin.focused_pane = crate::layout::PaneType::BeadList;

    plugin.tasks = vec![
        crate::plugin::TaskRow::new("task-1", "created", "P1", "Rust", "task/1"),
        crate::plugin::TaskRow::new("task-2", "created", "P1", "Rust", "task/2"),
    ];
    plugin.selected_index = 0;

    // Press k from index 0 → wrap to last
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'k',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.selected_index, 1);

    plugin.selected_index = 1;

    // Press j from last index → wrap to first
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'j',
        modifiers: KeyModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        },
    });
    assert_eq!(plugin.selected_index, 0);

    Ok(())
}

/// Test: Q quits the plugin
#[test]
fn test_q_quits_plugin() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;

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
    assert_eq!(plugin.state, crate::plugin::PluginState::ShuttingDown);

    Ok(())
}

/// Test: ? toggles help overlay
#[test]
fn test_question_mark_toggles_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    plugin.state = crate::plugin::PluginState::Running;

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
    assert_eq!(plugin.state, crate::plugin::PluginState::HelpOverlay);
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
    assert_eq!(plugin.state, crate::plugin::PluginState::Running);
    assert!(result.is_ok());

    Ok(())
}
