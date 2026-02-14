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

fn key_mods() -> KeyModifiers {
    KeyModifiers {
        shift: false,
        ctrl: false,
        alt: false,
    }
}

fn send_key(plugin: &mut OyaPlugin, key: char) {
    let _ = plugin.handle_event(PluginEvent::Key {
        key,
        modifiers: key_mods(),
    });
}

fn restore_with_tasks(
    plugin: &mut OyaPlugin,
    tasks: Vec<TaskRow>,
    selected_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    plugin.restore_from_snapshot(StateSnapshot {
        version: STATE_VERSION,
        tasks,
        selected_index,
        focused_pane: PaneType::BeadList,
        plugin_state: PluginState::Running,
        status_message: None,
        timestamp: 0,
    })?;
    Ok(())
}

#[test]
fn command_mode_backspace_allows_fixing_typo() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("alpha", "created", "P1", "Rust", "task/alpha"),
        TaskRow::new("beta", "created", "P1", "Rust", "task/beta"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    send_key(&mut plugin, ':');
    for key in "filter alpha".chars() {
        send_key(&mut plugin, key);
    }
    send_key(&mut plugin, '\n');
    assert_eq!(plugin.tasks_ref().len(), 1);

    send_key(&mut plugin, ':');
    for key in "clearr".chars() {
        send_key(&mut plugin, key);
    }
    send_key(&mut plugin, '\x7f');
    send_key(&mut plugin, '\n');

    assert_eq!(plugin.tasks_ref().len(), 2);
    assert_eq!(plugin.input_mode(), InputMode::Normal);
    Ok(())
}

#[test]
fn search_mode_backspace_updates_effective_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("alpha", "created", "P1", "Rust", "task/alpha"),
        TaskRow::new("beta", "created", "P1", "Rust", "task/beta"),
    ];
    restore_with_tasks(&mut plugin, tasks, 1)?;

    send_key(&mut plugin, '/');
    for key in "alphx".chars() {
        send_key(&mut plugin, key);
    }
    send_key(&mut plugin, '\x08');
    send_key(&mut plugin, 'a');
    send_key(&mut plugin, '\n');

    assert_eq!(plugin.selected_index(), 0);
    assert_eq!(plugin.input_mode(), InputMode::Normal);
    Ok(())
}

#[test]
fn single_g_does_not_move_selection() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("t1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("t2", "created", "P1", "Rust", "task/2"),
        TaskRow::new("t3", "created", "P1", "Rust", "task/3"),
    ];
    restore_with_tasks(&mut plugin, tasks, 2)?;

    send_key(&mut plugin, 'g');
    assert_eq!(plugin.selected_index(), 2);
    Ok(())
}

#[test]
fn g_prefix_survives_l_and_second_g_jumps_top() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("t1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("t2", "created", "P1", "Rust", "task/2"),
        TaskRow::new("t3", "created", "P1", "Rust", "task/3"),
    ];
    restore_with_tasks(&mut plugin, tasks, 1)?;

    send_key(&mut plugin, 'g');
    send_key(&mut plugin, 'l');
    send_key(&mut plugin, 'g');

    assert_eq!(plugin.selected_index(), 0);
    assert_eq!(plugin.focused_pane(), PaneType::BeadDetail);
    Ok(())
}

#[test]
fn repeated_gg_is_idempotent_at_top() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("t1", "created", "P1", "Rust", "task/1"),
        TaskRow::new("t2", "created", "P1", "Rust", "task/2"),
    ];
    restore_with_tasks(&mut plugin, tasks, 1)?;

    send_key(&mut plugin, 'g');
    send_key(&mut plugin, 'g');
    assert_eq!(plugin.selected_index(), 0);

    send_key(&mut plugin, 'g');
    send_key(&mut plugin, 'g');
    assert_eq!(plugin.selected_index(), 0);

    Ok(())
}

#[test]
fn help_overlay_interrupts_and_resumes_command_mode() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("alpha", "created", "P1", "Rust", "task/alpha"),
        TaskRow::new("beta", "created", "P1", "Rust", "task/beta"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    send_key(&mut plugin, ':');
    assert_eq!(plugin.input_mode(), InputMode::Command);

    for key in "filter alpha".chars() {
        send_key(&mut plugin, key);
    }

    send_key(&mut plugin, '?');
    assert_eq!(plugin.plugin_state(), PluginState::HelpOverlay);
    assert_eq!(plugin.input_mode(), InputMode::Command);

    send_key(&mut plugin, '\x1b');
    assert_eq!(plugin.plugin_state(), PluginState::Running);
    assert_eq!(plugin.input_mode(), InputMode::Command);

    send_key(&mut plugin, '\n');
    assert_eq!(plugin.tasks_ref().len(), 1);
    assert_eq!(plugin.tasks_ref()[0].slug, "alpha");
    assert_eq!(plugin.input_mode(), InputMode::Normal);

    Ok(())
}

#[test]
fn help_overlay_interrupts_and_resumes_search_mode() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = OyaPlugin::new()?;
    start_plugin(&mut plugin)?;

    let tasks = vec![
        TaskRow::new("alpha-1", "created", "P1", "Rust", "task/alpha-1"),
        TaskRow::new("beta", "created", "P1", "Rust", "task/beta"),
        TaskRow::new("alpha-2", "created", "P1", "Rust", "task/alpha-2"),
    ];
    restore_with_tasks(&mut plugin, tasks, 0)?;

    send_key(&mut plugin, '/');
    assert_eq!(plugin.input_mode(), InputMode::Search);

    for key in "alpha".chars() {
        send_key(&mut plugin, key);
    }

    send_key(&mut plugin, '?');
    assert_eq!(plugin.plugin_state(), PluginState::HelpOverlay);
    assert_eq!(plugin.input_mode(), InputMode::Search);

    send_key(&mut plugin, '\x1b');
    assert_eq!(plugin.plugin_state(), PluginState::Running);
    assert_eq!(plugin.input_mode(), InputMode::Search);

    send_key(&mut plugin, '\n');
    assert_eq!(plugin.selected_index(), 2);
    assert_eq!(plugin.input_mode(), InputMode::Normal);

    Ok(())
}
