#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use zellij_frontend::layout::PaneType;
use zellij_frontend::plugin::{
    InputMode, KeyModifiers, OyaPlugin, PluginEvent, PluginInfo, PluginState, Size, TaskRow,
};
use zellij_frontend::state::{StateSnapshot, STATE_VERSION};

fn start_plugin(plugin: &mut OyaPlugin) -> Result<(), Box<dyn std::error::Error>> {
    let _ = plugin.handle_event(PluginEvent::Start {
        info: PluginInfo {
            size: Size {
                rows: 24,
                cols: 100,
            },
            config: serde_json::json!({}),
        },
    })?;
    Ok(())
}

fn make_mods(shift: bool, ctrl: bool) -> KeyModifiers {
    KeyModifiers {
        shift,
        ctrl,
        alt: false,
    }
}

fn restore_with_tasks(
    plugin: &mut OyaPlugin,
    tasks: Vec<TaskRow>,
    selected_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    restore_with_tasks_and_pane(plugin, tasks, selected_index, PaneType::BeadList)
}

fn restore_with_tasks_and_pane(
    plugin: &mut OyaPlugin,
    tasks: Vec<TaskRow>,
    selected_index: usize,
    focused_pane: PaneType,
) -> Result<(), Box<dyn std::error::Error>> {
    plugin.restore_from_snapshot(StateSnapshot {
        version: STATE_VERSION,
        tasks,
        selected_index,
        focused_pane,
        plugin_state: PluginState::Running,
        status_message: None,
        timestamp: 0,
    })?;
    Ok(())
}

#[test]
fn ctrl_shift_g_switches_to_graph_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: make_mods(true, true),
    });

    assert_eq!(plugin.focused_pane(), PaneType::WorkflowGraph);
    Ok(())
}

#[test]
fn tab_and_shift_tab_cycle_panes() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadDetail);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\t',
        modifiers: make_mods(true, false),
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    Ok(())
}

#[test]
fn vim_j_k_wrap_selection() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("task-1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("task-2", "created", "P1", "Rust", "task/2"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'k',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.selected_index(), 1);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'j',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.selected_index(), 0);

    Ok(())
}

#[test]
fn command_mode_filter_and_clear() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("alpha", "created", "P1", "Rust", "task/alpha"),
        TaskRow::new("beta", "created", "P1", "Rust", "task/beta"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: ':',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.input_mode(), InputMode::Command);

    for key in "filter alpha".chars() {
        let _ = plugin.handle_event(PluginEvent::Key {
            key,
            modifiers: make_mods(false, false),
        });
    }
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\n',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.tasks_ref().len(), 1);
    assert_eq!(plugin.tasks_ref()[0].slug, "alpha");
    assert_eq!(plugin.input_mode(), InputMode::Normal);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: ':',
        modifiers: make_mods(false, false),
    });
    for key in "clear".chars() {
        let _ = plugin.handle_event(PluginEvent::Key {
            key,
            modifiers: make_mods(false, false),
        });
    }
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\n',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.tasks_ref().len(), 2);
    Ok(())
}

#[test]
fn slash_search_and_repeat_navigation() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("alpha-1", "created", "P1", "Rust", "task/alpha-1"),
        TaskRow::new("beta", "created", "P1", "Rust", "task/beta"),
        TaskRow::new("alpha-2", "created", "P1", "Rust", "task/alpha-2"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '/',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.input_mode(), InputMode::Search);

    for key in "alpha".chars() {
        let _ = plugin.handle_event(PluginEvent::Key {
            key,
            modifiers: make_mods(false, false),
        });
    }
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\n',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.selected_index(), 2);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'N',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.selected_index(), 0);

    Ok(())
}

#[test]
fn ctrl_shift_l_switches_to_list_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: make_mods(true, true),
    });
    assert_eq!(plugin.focused_pane(), PaneType::WorkflowGraph);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'l',
        modifiers: make_mods(true, true),
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    Ok(())
}

#[test]
fn ctrl_shift_a_triggers_approve_action_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![TaskRow::new(
        "approve-me",
        "created",
        "P1",
        "Rust",
        "task/approve",
    )];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'a',
        modifiers: make_mods(true, true),
    });

    assert!(plugin.status_message().is_some_and(|msg| {
        msg.contains("Approve task")
            || msg.contains("Task error")
            || msg.contains("IPC")
            || msg.contains("updated")
    }));
    Ok(())
}

#[test]
fn h_and_l_cycle_panes_like_vim_window_nav() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'l',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadDetail);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'h',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.focused_pane(), PaneType::BeadList);

    Ok(())
}

#[test]
fn gg_and_g_move_to_top_and_bottom() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("task-1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("task-2", "created", "P1", "Rust", "task/2"),
        TaskRow::new("task-3", "created", "P1", "Rust", "task/3"),
    ];
    restore_with_tasks(&mut plugin, tasks, 1)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'G',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.selected_index(), 2);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: make_mods(false, false),
    });
    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'g',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.selected_index(), 0);

    Ok(())
}

#[test]
fn esc_exits_command_mode_without_executing() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("keep-1", "created", "P1", "Rust", "task/keep-1"),
        TaskRow::new("keep-2", "created", "P1", "Rust", "task/keep-2"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: ':',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.input_mode(), InputMode::Command);

    for key in "filter alpha".chars() {
        let _ = plugin.handle_event(PluginEvent::Key {
            key,
            modifiers: make_mods(false, false),
        });
    }

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.input_mode(), InputMode::Normal);
    assert_eq!(plugin.tasks_ref().len(), 2);
    Ok(())
}

#[test]
fn esc_exits_search_mode() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '/',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.input_mode(), InputMode::Search);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.input_mode(), InputMode::Normal);

    Ok(())
}

#[test]
fn visual_mode_entry_navigation_and_exit() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("task-1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("task-2", "created", "P1", "Rust", "task/2"),
        TaskRow::new("task-3", "created", "P1", "Rust", "task/3"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'v',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.input_mode(), InputMode::Visual);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'j',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.selected_index(), 1);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'v',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.input_mode(), InputMode::Normal);

    Ok(())
}

#[test]
fn j_does_not_move_selection_when_not_in_bead_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("task-1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("task-2", "created", "P1", "Rust", "task/2"),
    ];
    restore_with_tasks_and_pane(&mut plugin, tasks, 0, PaneType::PipelineView)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'j',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.selected_index(), 0);
    Ok(())
}

#[test]
fn command_mode_unknown_command_reports_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: ':',
        modifiers: make_mods(false, false),
    });
    for key in "bogus".chars() {
        let _ = plugin.handle_event(PluginEvent::Key {
            key,
            modifiers: make_mods(false, false),
        });
    }
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\n',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.input_mode(), InputMode::Normal);
    assert!(plugin
        .status_message()
        .is_some_and(|msg| msg.contains("Command error")));
    Ok(())
}

#[test]
fn search_reports_no_matches() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("alpha", "created", "P1", "Rust", "task/alpha"),
        TaskRow::new("beta", "created", "P1", "Rust", "task/beta"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '/',
        modifiers: make_mods(false, false),
    });
    for key in "zzz".chars() {
        let _ = plugin.handle_event(PluginEvent::Key {
            key,
            modifiers: make_mods(false, false),
        });
    }
    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\n',
        modifiers: make_mods(false, false),
    });

    assert!(plugin
        .status_message()
        .is_some_and(|msg| msg.contains("No matches found")));
    Ok(())
}

#[test]
fn n_without_active_pattern_reports_status() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: 'n',
        modifiers: make_mods(false, false),
    });

    assert!(plugin
        .status_message()
        .is_some_and(|msg| msg.contains("No active search pattern")));
    Ok(())
}

#[test]
fn question_mark_and_esc_toggle_help_overlay() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '?',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.plugin_state(), PluginState::HelpOverlay);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.plugin_state(), PluginState::Running);

    Ok(())
}

#[test]
fn esc_returns_to_parent_pane_from_bead_detail() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    restore_with_tasks_and_pane(
        &mut plugin,
        vec![TaskRow::new("task-1", "created", "P1", "Rust", "task/1")],
        0,
        PaneType::BeadDetail,
    )?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadList);
    assert_eq!(plugin.input_mode(), InputMode::Normal);
    Ok(())
}

#[test]
fn esc_returns_to_parent_pane_from_pipeline_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    restore_with_tasks_and_pane(
        &mut plugin,
        vec![TaskRow::new("task-1", "created", "P1", "Rust", "task/1")],
        0,
        PaneType::PipelineView,
    )?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadList);
    assert_eq!(plugin.input_mode(), InputMode::Normal);
    Ok(())
}

#[test]
fn esc_returns_to_parent_pane_from_workflow_graph() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    restore_with_tasks_and_pane(
        &mut plugin,
        vec![TaskRow::new("task-1", "created", "P1", "Rust", "task/1")],
        0,
        PaneType::WorkflowGraph,
    )?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadList);
    assert_eq!(plugin.input_mode(), InputMode::Normal);
    Ok(())
}

#[test]
fn esc_returns_to_parent_pane_from_agent_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    restore_with_tasks_and_pane(
        &mut plugin,
        vec![TaskRow::new("task-1", "created", "P1", "Rust", "task/1")],
        0,
        PaneType::AgentView,
    )?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadList);
    assert_eq!(plugin.input_mode(), InputMode::Normal);
    Ok(())
}

#[test]
fn esc_stays_on_bead_list_when_already_there() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    restore_with_tasks_and_pane(
        &mut plugin,
        vec![TaskRow::new("task-1", "created", "P1", "Rust", "task/1")],
        0,
        PaneType::BeadList,
    )?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadList);
    assert_eq!(plugin.input_mode(), InputMode::Normal);
    Ok(())
}

#[test]
fn esc_exits_command_mode_and_returns_to_parent_pane() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    restore_with_tasks_and_pane(
        &mut plugin,
        vec![TaskRow::new("task-1", "created", "P1", "Rust", "task/1")],
        0,
        PaneType::BeadDetail,
    )?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: ':',
        modifiers: make_mods(false, false),
    });
    assert_eq!(plugin.input_mode(), InputMode::Command);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\x1b',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadList);
    assert_eq!(plugin.input_mode(), InputMode::Normal);
    Ok(())
}

#[test]
fn enter_drills_down_to_bead_detail() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("task-1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("task-2", "created", "P1", "Rust", "task/2"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    assert_eq!(plugin.focused_pane(), PaneType::BeadList);
    assert_eq!(plugin.selected_index(), 0);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\n',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadDetail);
    assert_eq!(plugin.selected_index(), 0);

    Ok(())
}

#[test]
fn enter_drills_down_preserves_selection() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("task-1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("task-2", "created", "P1", "Rust", "task/2"),
        TaskRow::new("task-3", "created", "P1", "Rust", "task/3"),
    ];
    restore_with_tasks(&mut plugin, tasks, 2)?;

    assert_eq!(plugin.selected_index(), 2);

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\n',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadDetail);
    assert_eq!(plugin.selected_index(), 2);

    Ok(())
}

#[test]
fn enter_does_nothing_in_bead_detail() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    restore_with_tasks_and_pane(&mut plugin, vec![], 0, PaneType::BeadDetail)?;

    let _ = plugin.handle_event(PluginEvent::Key {
        key: '\n',
        modifiers: make_mods(false, false),
    });

    assert_eq!(plugin.focused_pane(), PaneType::BeadDetail);

    Ok(())
}
