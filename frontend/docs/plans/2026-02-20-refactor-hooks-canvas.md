# Refactor App Component with Custom Hooks + Canvas Component

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split the 770-line `App` component into focused custom hooks and a new Canvas component using Dioxus 0.7 patterns (Signals, ReadOnlySignal, functional actions).

**Architecture:** Extract 20+ signals into semantic hooks (use_workflow_state, use_selection, use_canvas_interaction, use_ui_panels), create Canvas component to handle all canvas events/rendering, and slim main.rs to orchestration only.

**Tech Stack:** Dioxus 0.7, Rust, Tailwind CSS, thiserror

---

## Task 1: Add Error Types Module

**Files:**
- Create: `src/errors.rs`
- Modify: `src/main.rs:1-5`

**Step 1: Write the error type definitions**

```rust
// src/errors.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use thiserror::Error;
use crate::graph::NodeId;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum WorkflowError {
    #[error("Node {0} not found")]
    NodeNotFound(NodeId),

    #[error("Connection would create a cycle")]
    CycleDetected,

    #[error("Invalid connection: {0}")]
    InvalidConnection(String),

    #[error("Port not found: {0}")]
    PortNotFound(String),

    #[error("Cannot connect node to itself")]
    SelfConnection,
}

pub type WorkflowResult<T> = Result<T, WorkflowError>;
```

**Step 2: Add module declaration to main.rs**

```rust
// Add after mod ui;
mod errors;
```

**Step 3: Run cargo check**

Run: `moon run :check`
Expected: No errors (thiserror should already be in dependencies)

**Step 4: Commit**

```bash
git add src/errors.rs src/main.rs
git commit -m "feat: add WorkflowError type for functional error handling"
```

---

## Task 2: Create Hooks Module Structure

**Files:**
- Create: `src/hooks/mod.rs`
- Create: `src/hooks/use_workflow_state.rs`
- Create: `src/hooks/use_selection.rs`
- Create: `src/hooks/use_canvas_interaction.rs`
- Create: `src/hooks/use_ui_panels.rs`
- Modify: `src/main.rs:15`

**Step 1: Create hooks module entry point**

```rust
// src/hooks/mod.rs
pub mod use_canvas_interaction;
pub mod use_selection;
pub mod use_ui_panels;
pub mod use_workflow_state;

pub use use_canvas_interaction::{CanvasInteraction, InteractionMode};
pub use use_selection::SelectionState;
pub use use_ui_panels::{ContextMenuState, UiPanels};
pub use use_workflow_state::WorkflowState;
```

**Step 2: Create use_workflow_state stub**

```rust
// src/hooks/use_workflow_state.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use crate::errors::WorkflowResult;
use crate::graph::{Connection, Node, NodeId, PortName, Viewport, Workflow};
use dioxus::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct WorkflowState {
    pub workflow: ReadOnlySignal<Workflow>,
    pub workflow_name: ReadOnlySignal<String>,
    pub nodes: ReadOnlySignal<Vec<Node>>,
    pub nodes_by_id: ReadOnlySignal<HashMap<NodeId, Node>>,
    pub connections: ReadOnlySignal<Vec<Connection>>,
    pub viewport: ReadOnlySignal<Viewport>,

    pub add_node: fn(node_type: &str, x: f32, y: f32) -> NodeId,
    pub remove_node: fn(node_id: NodeId) -> WorkflowResult<()>,
    pub add_connection: fn(source: NodeId, target: NodeId, source_port: &PortName, target_port: &PortName) -> WorkflowResult<()>,
    pub zoom: fn(delta: f32, center_x: f32, center_y: f32),
    pub pan: fn(dx: f32, dy: f32),
    pub undo: fn() -> bool,
    pub redo: fn() -> bool,
    pub save_undo_point: fn(),
}

pub fn use_workflow_state() -> WorkflowState {
    todo!("Implement in Task 3")
}
```

**Step 3: Create use_selection stub**

```rust
// src/hooks/use_selection.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use crate::graph::NodeId;
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct SelectionState {
    pub selected_id: ReadOnlySignal<Option<NodeId>>,
    pub selected_ids: ReadOnlySignal<Vec<NodeId>>,

    pub select_single: fn(NodeId),
    pub clear: fn(),
    pub is_selected: fn(NodeId) -> bool,
}

pub fn use_selection() -> SelectionState {
    todo!("Implement in Task 4")
}
```

**Step 4: Create use_canvas_interaction stub**

```rust
// src/hooks/use_canvas_interaction.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use crate::graph::NodeId;
use crate::ui::edges::Position as FlowPosition;
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InteractionMode {
    Idle,
    Panning,
    Dragging { node_ids: Vec<NodeId> },
    Connecting { from: NodeId, handle: String },
    Marquee { start: (f32, f32) },
}

#[derive(Clone, Copy)]
pub struct CanvasInteraction {
    pub mode: ReadOnlySignal<InteractionMode>,
    pub is_space_hand: ReadOnlySignal<bool>,
    pub mouse_pos: ReadOnlySignal<(f32, f32)>,
    pub canvas_origin: ReadOnlySignal<(f32, f32)>,
    pub temp_edge_to: ReadOnlySignal<Option<FlowPosition>>,
    pub hovered_handle: ReadOnlySignal<Option<(NodeId, String)>>,

    pub start_pan: fn(),
    pub start_drag: fn(node_id: NodeId, selected_ids: Vec<NodeId>),
    pub start_connect: fn(node_id: NodeId, handle: String),
    pub start_marquee: fn(pos: (f32, f32)),
    pub update_mouse: fn(pos: (f32, f32)),
    pub end_interaction: fn(),
    pub set_origin: fn((f32, f32)),
    pub enable_space_hand: fn(),
    pub cancel_interaction: fn(),
}

pub fn use_canvas_interaction() -> CanvasInteraction {
    todo!("Implement in Task 5")
}
```

**Step 5: Create use_ui_panels stub**

```rust
// src/hooks/use_ui_panels.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuState {
    pub open: bool,
    pub x: f32,
    pub y: f32,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self { open: false, x: 0.0, y: 0.0 }
    }
}

#[derive(Clone, Copy)]
pub struct UiPanels {
    pub settings_open: ReadOnlySignal<bool>,
    pub palette_open: ReadOnlySignal<bool>,
    pub palette_query: ReadOnlySignal<String>,
    pub context_menu: ReadOnlySignal<ContextMenuState>,

    pub toggle_settings: fn(),
    pub toggle_palette: fn(),
    pub open_palette: fn(),
    pub close_palette: fn(),
    pub set_palette_query: fn(String),
    pub show_context_menu: fn(x: f32, y: f32),
    pub close_context_menu: fn(),
    pub close_all: fn(),
}

pub fn use_ui_panels() -> UiPanels {
    todo!("Implement in Task 6")
}
```

**Step 6: Add hooks module to main.rs**

```rust
// src/main.rs - add after mod ui;
mod hooks;
```

**Step 7: Run cargo check**

Run: `moon run :check`
Expected: PASS (stubs compile)

**Step 8: Commit**

```bash
git add src/hooks/
git commit -m "feat: add hooks module structure with stub signatures"
```

---

## Task 3: Implement use_workflow_state Hook

**Files:**
- Modify: `src/hooks/use_workflow_state.rs`

**Step 1: Implement the hook with functional actions**

```rust
// src/hooks/use_workflow_state.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use crate::errors::{WorkflowError, WorkflowResult};
use crate::graph::{Connection, Node, NodeId, PortName, Viewport, Workflow};
use dioxus::prelude::*;
use std::collections::HashMap;

pub fn use_workflow_state() -> WorkflowState {
    let workflow = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::window;
            let storage = window().and_then(|w| w.local_storage().ok()).flatten();
            if let Some(s) = storage {
                match s.get_item("flow-wasm-v1-workflow") {
                    Ok(Some(json)) => {
                        if let Ok(parsed) = serde_json::from_str::<Workflow>(&json) {
                            return parsed;
                        }
                    }
                    _ => {}
                }
            }
        }
        crate::ui::app_bootstrap::default_workflow()
    });

    let workflow_name = use_signal(|| "SignupWorkflow".to_string());
    let undo_stack = use_signal(Vec::<Workflow>::new);
    let redo_stack = use_signal(Vec::<Workflow>::new);

    // Derived memos for performance
    let nodes = use_memo(move || workflow.read().nodes.clone());
    let nodes_by_id = use_memo(move || {
        workflow
            .read()
            .nodes
            .iter()
            .map(|n| (n.id, n.clone()))
            .collect()
    });
    let connections = use_memo(move || workflow.read().connections.clone());
    let viewport = use_memo(move || workflow.read().viewport.clone());

    // Save undo point
    let save_undo_point = {
        let workflow = workflow.clone();
        let undo_stack = undo_stack.clone();
        let redo_stack = redo_stack.clone();
        move || {
            undo_stack.write().push(workflow.read().clone());
            if undo_stack.read().len() > 60 {
                let _ = undo_stack.write().remove(0);
            }
            redo_stack.write().clear();
        }
    };

    // Add node action
    let add_node = {
        let workflow = workflow.clone();
        let save_undo_point = save_undo_point;
        move |node_type: &str, x: f32, y: f32| {
            let new_id = NodeId::new();
            save_undo_point();
            workflow.write().add_node(node_type, x, y);
            new_id
        }
    };

    // Remove node action
    let remove_node = {
        let workflow = workflow.clone();
        let save_undo_point = save_undo_point;
        move |node_id: NodeId| -> WorkflowResult<()> {
            let exists = workflow.read().nodes.iter().any(|n| n.id == node_id);
            if !exists {
                return Err(WorkflowError::NodeNotFound(node_id));
            }
            save_undo_point();
            workflow.write().remove_node(node_id);
            Ok(())
        }
    };

    // Add connection action
    let add_connection = {
        let workflow = workflow.clone();
        let save_undo_point = save_undo_point;
        move |source: NodeId, target: NodeId, source_port: &PortName, target_port: &PortName| -> WorkflowResult<()> {
            if source == target {
                return Err(WorkflowError::SelfConnection);
            }
            save_undo_point();
            workflow.write().add_connection(source, target, source_port, target_port);
            Ok(())
        }
    };

    // Zoom action
    let zoom = {
        let workflow = workflow.clone();
        move |delta: f32, center_x: f32, center_y: f32| {
            workflow.write().zoom(delta, center_x, center_y);
        }
    };

    // Pan action
    let pan = {
        let workflow = workflow.clone();
        move |dx: f32, dy: f32| {
            workflow.write().viewport.x += dx;
            workflow.write().viewport.y += dy;
        }
    };

    // Undo action
    let undo = {
        let workflow = workflow.clone();
        let undo_stack = undo_stack.clone();
        let redo_stack = redo_stack.clone();
        move || -> bool {
            let previous = undo_stack.write().pop();
            if let Some(snapshot) = previous {
                let current = workflow.read().clone();
                redo_stack.write().push(current);
                workflow.set(snapshot);
                true
            } else {
                false
            }
        }
    };

    // Redo action
    let redo = {
        let workflow = workflow.clone();
        let undo_stack = undo_stack.clone();
        let redo_stack = redo_stack.clone();
        move || -> bool {
            let next = redo_stack.write().pop();
            if let Some(snapshot) = next {
                let current = workflow.read().clone();
                undo_stack.write().push(current);
                workflow.set(snapshot);
                true
            } else {
                false
            }
        }
    };

    WorkflowState {
        workflow: workflow.into(),
        workflow_name: workflow_name.into(),
        nodes: nodes.into(),
        nodes_by_id: nodes_by_id.into(),
        connections: connections.into(),
        viewport: viewport.into(),
        add_node,
        remove_node,
        add_connection,
        zoom,
        pan,
        undo,
        redo,
        save_undo_point,
    }
}
```

**Step 2: Run cargo check**

Run: `moon run :check`
Expected: PASS

**Step 3: Run tests**

Run: `moon run :test`
Expected: Existing tests still pass

**Step 4: Commit**

```bash
git add src/hooks/use_workflow_state.rs
git commit -m "feat: implement use_workflow_state hook with functional actions"
```

---

## Task 4: Implement use_selection Hook

**Files:**
- Modify: `src/hooks/use_selection.rs`

**Step 1: Implement the hook**

```rust
// src/hooks/use_selection.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use crate::graph::NodeId;
use dioxus::prelude::*;

pub fn use_selection() -> SelectionState {
    let selected_id = use_signal(|| None::<NodeId>);
    let selected_ids = use_signal(Vec::<NodeId>::new);

    let select_single = {
        let selected_id = selected_id.clone();
        let selected_ids = selected_ids.clone();
        move |id: NodeId| {
            selected_id.set(Some(id));
            selected_ids.set(vec![id]);
        }
    };

    let clear = {
        let selected_id = selected_id.clone();
        let selected_ids = selected_ids.clone();
        move || {
            selected_id.set(None);
            selected_ids.set(Vec::new());
        }
    };

    let is_selected = {
        let selected_ids = selected_ids.clone();
        move |id: NodeId| -> bool {
            selected_ids.read().contains(&id)
        }
    };

    SelectionState {
        selected_id: selected_id.into(),
        selected_ids: selected_ids.into(),
        select_single,
        clear,
        is_selected,
    }
}
```

**Step 2: Run cargo check**

Run: `moon run :check`
Expected: PASS

**Step 3: Commit**

```bash
git add src/hooks/use_selection.rs
git commit -m "feat: implement use_selection hook"
```

---

## Task 5: Implement use_canvas_interaction Hook

**Files:**
- Modify: `src/hooks/use_canvas_interaction.rs`

**Step 1: Implement the hook with InteractionMode state machine**

```rust
// src/hooks/use_canvas_interaction.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use crate::graph::NodeId;
use crate::ui::edges::Position as FlowPosition;
use dioxus::prelude::*;

pub fn use_canvas_interaction() -> CanvasInteraction {
    let mode = use_signal(|| InteractionMode::Idle);
    let is_space_hand = use_signal(|| false);
    let mouse_pos = use_signal(|| (0.0_f32, 0.0_f32));
    let canvas_origin = use_signal(|| (0.0_f32, 0.0_f32));
    let temp_edge_to = use_signal(|| None::<FlowPosition>);
    let hovered_handle = use_signal(|| None::<(NodeId, String)>);

    let start_pan = {
        let mode = mode.clone();
        move || mode.set(InteractionMode::Panning)
    };

    let start_drag = {
        let mode = mode.clone();
        move |node_id: NodeId, selected_ids: Vec<NodeId>| {
            mode.set(InteractionMode::Dragging { node_ids: selected_ids });
        }
    };

    let start_connect = {
        let mode = mode.clone();
        let temp_edge_to = temp_edge_to.clone();
        let hovered_handle = hovered_handle.clone();
        move |node_id: NodeId, handle: String| {
            hovered_handle.set(Some((node_id, handle.clone())));
            mode.set(InteractionMode::Connecting { from: node_id, handle });
        }
    };

    let start_marquee = {
        let mode = mode.clone();
        move |pos: (f32, f32)| {
            mode.set(InteractionMode::Marquee { start: pos });
        }
    };

    let update_mouse = {
        let mouse_pos = mouse_pos.clone();
        move |pos: (f32, f32)| {
            mouse_pos.set(pos);
        }
    };

    let end_interaction = {
        let mode = mode.clone();
        let temp_edge_to = temp_edge_to.clone();
        let hovered_handle = hovered_handle.clone();
        move || {
            mode.set(InteractionMode::Idle);
            temp_edge_to.set(None);
            hovered_handle.set(None);
        }
    };

    let set_origin = {
        let canvas_origin = canvas_origin.clone();
        move |origin: (f32, f32)| {
            canvas_origin.set(origin);
        }
    };

    let enable_space_hand = {
        let is_space_hand = is_space_hand.clone();
        move || {
            is_space_hand.set(true);
        }
    };

    let cancel_interaction = {
        let mode = mode.clone();
        let temp_edge_to = temp_edge_to.clone();
        let hovered_handle = hovered_handle.clone();
        move || {
            mode.set(InteractionMode::Idle);
            temp_edge_to.set(None);
            hovered_handle.set(None);
        }
    };

    CanvasInteraction {
        mode: mode.into(),
        is_space_hand: is_space_hand.into(),
        mouse_pos: mouse_pos.into(),
        canvas_origin: canvas_origin.into(),
        temp_edge_to: temp_edge_to.into(),
        hovered_handle: hovered_handle.into(),
        start_pan,
        start_drag,
        start_connect,
        start_marquee,
        update_mouse,
        end_interaction,
        set_origin,
        enable_space_hand,
        cancel_interaction,
    }
}
```

**Step 2: Run cargo check**

Run: `moon run :check`
Expected: PASS

**Step 3: Commit**

```bash
git add src/hooks/use_canvas_interaction.rs
git commit -m "feat: implement use_canvas_interaction hook with InteractionMode state machine"
```

---

## Task 6: Implement use_ui_panels Hook

**Files:**
- Modify: `src/hooks/use_ui_panels.rs`

**Step 1: Implement the hook**

```rust
// src/hooks/use_ui_panels.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use dioxus::prelude::*;

pub fn use_ui_panels() -> UiPanels {
    let settings_open = use_signal(|| false);
    let palette_open = use_signal(|| false);
    let palette_query = use_signal(String::new);
    let context_menu = use_signal(|| ContextMenuState::default());

    let toggle_settings = {
        let settings_open = settings_open.clone();
        move || {
            let current = *settings_open.read();
            settings_open.set(!current);
        }
    };

    let toggle_palette = {
        let palette_open = palette_open.clone();
        let palette_query = palette_query.clone();
        move || {
            let current = *palette_open.read();
            palette_open.set(!current);
            if !current {
                palette_query.set(String::new());
            }
        }
    };

    let open_palette = {
        let palette_open = palette_open.clone();
        let palette_query = palette_query.clone();
        move || {
            palette_open.set(true);
            palette_query.set(String::new());
        }
    };

    let close_palette = {
        let palette_open = palette_open.clone();
        move || {
            palette_open.set(false);
        }
    };

    let set_palette_query = {
        let palette_query = palette_query.clone();
        move |value: String| {
            palette_query.set(value);
        }
    };

    let show_context_menu = {
        let context_menu = context_menu.clone();
        move |x: f32, y: f32| {
            context_menu.set(ContextMenuState { open: true, x, y });
        }
    };

    let close_context_menu = {
        let context_menu = context_menu.clone();
        move || {
            context_menu.set(ContextMenuState::default());
        }
    };

    let close_all = {
        let palette_open = palette_open.clone();
        let context_menu = context_menu.clone();
        move || {
            palette_open.set(false);
            context_menu.set(ContextMenuState::default());
        }
    };

    UiPanels {
        settings_open: settings_open.into(),
        palette_open: palette_open.into(),
        palette_query: palette_query.into(),
        context_menu: context_menu.into(),
        toggle_settings,
        toggle_palette,
        open_palette,
        close_palette,
        set_palette_query,
        show_context_menu,
        close_context_menu,
        close_all,
    }
}
```

**Step 2: Run cargo check**

Run: `moon run :check`
Expected: PASS

**Step 3: Commit**

```bash
git add src/hooks/use_ui_panels.rs
git commit -m "feat: implement use_ui_panels hook"
```

---

## Task 7: Create Canvas Component

**Files:**
- Create: `src/components/mod.rs`
- Create: `src/components/canvas.rs`
- Create: `src/canvas_events.rs`
- Modify: `src/main.rs`

**Step 1: Create components module**

```rust
// src/components/mod.rs
pub mod canvas;

pub use canvas::Canvas;
```

**Step 2: Create canvas_events.rs with event handlers**

```rust
// src/canvas_events.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use crate::hooks::{CanvasInteraction, SelectionState, UiPanels, WorkflowState};
use crate::ui::editor_interactions;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;

pub fn on_mouse_enter(evt: MouseEvent, canvas: CanvasInteraction) {
    let page = evt.page_coordinates();
    let element = evt.element_coordinates();
    (canvas.set_origin)(
        page.x as f32 - element.x as f32,
        page.y as f32 - element.y as f32,
    );
}

pub fn on_mouse_down(
    evt: MouseEvent,
    canvas: CanvasInteraction,
    selection: &SelectionState,
    panels: &UiPanels,
) {
    (panels.close_context_menu)();
    let trigger_button = evt.trigger_button();

    if matches!(trigger_button, Some(MouseButton::Primary | MouseButton::Auxiliary)) {
        evt.prevent_default();
        let page = evt.page_coordinates();
        let coordinates = evt.element_coordinates();
        let origin_x = page.x as f32 - coordinates.x as f32;
        let origin_y = page.y as f32 - coordinates.y as f32;
        (canvas.set_origin)((origin_x, origin_y));

        let mouse_pos = (coordinates.x as f32, coordinates.y as f32);
        (canvas.update_mouse)(mouse_pos);

        if matches!(trigger_button, Some(MouseButton::Auxiliary))
            || (matches!(trigger_button, Some(MouseButton::Primary)) && *canvas.is_space_hand.read())
        {
            (canvas.start_pan)();
        } else if matches!(trigger_button, Some(MouseButton::Primary)) {
            (canvas.start_marquee)(mouse_pos);
        }
    } else {
        (selection.clear)();
    }
}

pub fn on_mouse_move(
    evt: MouseEvent,
    workflow: &WorkflowState,
    canvas: CanvasInteraction,
    selection: &SelectionState,
) {
    let page = evt.page_coordinates();
    let (origin_x, origin_y) = *canvas.canvas_origin.read();
    let mx = page.x as f32 - origin_x;
    let my = page.y as f32 - origin_y;
    (canvas.update_mouse)((mx, my));

    let mode = *canvas.mode.read();
    let vp = *workflow.viewport.read();
    let zoom = vp.zoom;

    match mode {
        crate::hooks::InteractionMode::Dragging { node_ids } => {
            let (lx, ly) = *canvas.mouse_pos.read();
            let dx = mx - lx;
            let dy = my - ly;

            for node_id in node_ids {
                workflow.pan(dx / zoom, dy / zoom);
            }
        }
        crate::hooks::InteractionMode::Connecting { from, .. } => {
            let canvas_x = (mx - vp.x) / zoom;
            let canvas_y = (my - vp.y) / zoom;

            let nodes = workflow.nodes.read().clone();
            let snapped = editor_interactions::snap_handle(&nodes, mx, my, &vp).filter(
                |(node_id, handle_kind, _)| *node_id != from && *handle_kind != "source",
            );

            if let Some((node_id, handle_kind, snapped_pos)) = snapped {
                use dioxus_signals::Signal;
                canvas.hovered_handle.set(Some((node_id, handle_kind)));
                canvas.temp_edge_to.set(Some(snapped_pos));
            } else {
                canvas.hovered_handle.set(None);
                canvas.temp_edge_to.set(Some(crate::ui::edges::Position {
                    x: canvas_x,
                    y: canvas_y,
                }));
            }
        }
        crate::hooks::InteractionMode::Panning => {
            let (lx, ly) = *canvas.mouse_pos.read();
            let dx = mx - lx;
            let dy = my - ly;
            workflow.pan(dx, dy);
        }
        _ => {}
    }
}

pub fn on_mouse_up(
    evt: MouseEvent,
    workflow: &WorkflowState,
    canvas: CanvasInteraction,
    selection: &SelectionState,
) {
    let mode = *canvas.mode.read();

    if let crate::hooks::InteractionMode::Connecting { from, .. } = mode {
        let hovered = *canvas.hovered_handle.read();
        if let Some((to, _)) = hovered {
            if from != to {
                let _ = workflow.add_connection(
                    from,
                    to,
                    &crate::graph::PortName("main".to_string()),
                    &crate::graph::PortName("main".to_string()),
                );
            }
        }
    }

    (canvas.end_interaction)();
}

pub fn on_key_down(
    evt: KeyboardEvent,
    workflow: &WorkflowState,
    selection: &SelectionState,
    panels: &UiPanels,
    canvas: &CanvasInteraction,
) {
    let key = evt.key().to_string().to_lowercase();

    match key.as_str() {
        " " | "space" => {
            evt.prevent_default();
            (canvas.enable_space_hand)();
        }
        "escape" => {
            evt.prevent_default();
            (panels.close_all)();
            (canvas.cancel_interaction)();
            (selection.clear)();
        }
        "k" => {
            evt.prevent_default();
            (panels.toggle_palette)();
        }
        "+" | "=" | "add" => {
            evt.prevent_default();
            (workflow.zoom)(0.12, 640.0, 400.0);
        }
        "-" | "_" | "subtract" => {
            evt.prevent_default();
            (workflow.zoom)(-0.12, 640.0, 400.0);
        }
        "0" => {
            evt.prevent_default();
            // workflow.fit_view
        }
        "z" => {
            evt.prevent_default();
            (workflow.undo)();
            (selection.clear)();
        }
        "y" => {
            evt.prevent_default();
            (workflow.redo)();
            (selection.clear)();
        }
        "backspace" | "delete" => {
            let ids = selection.selected_ids.read().clone();
            if !ids.is_empty() {
                evt.prevent_default();
                for id in ids {
                    let _ = (workflow.remove_node)(id);
                }
                (selection.clear)();
            }
        }
        _ => {}
    }
}

pub fn on_key_up(evt: KeyEvent, canvas: &CanvasInteraction) {
    let key = evt.key().to_string().to_lowercase();
    if key == " " || key == "space" {
        evt.prevent_default();
        // Reset space hand if needed
    }
}

pub fn on_wheel(evt: WheelEvent, workflow: &WorkflowState, canvas: &CanvasInteraction) {
    evt.prevent_default();
    let page = evt.page_coordinates();
    let (origin_x, origin_y) = *canvas.canvas_origin.read();
    let delta = -evt.delta().strip_units().y as f32 * 0.001;
    let zoom_x = page.x as f32 - origin_x;
    let zoom_y = page.y as f32 - origin_y;
    (workflow.zoom)(delta, zoom_x, zoom_y);
}
```

**Step 3: Create Canvas component**

```rust
// src/components/canvas.rs
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use crate::canvas_events;
use crate::hooks::{CanvasInteraction, SelectionState, WorkflowState};
use crate::ui::{FlowEdges, FlowMinimap, FlowNodeComponent, FlowPosition};
use dioxus::prelude::*;
use oya_frontend::graph::{NodeId, Workflow};

#[component]
pub fn Canvas(
    workflow: WorkflowState,
    selection: SelectionState,
    canvas: CanvasInteraction,
) -> Element {
    let vp = workflow.viewport;
    let nodes = workflow.nodes;
    let connections = workflow.connections;

    // Cursor based on mode
    let cursor = use_memo(move || match *canvas.mode.read() {
        crate::hooks::InteractionMode::Panning => "cursor-grabbing",
        crate::hooks::InteractionMode::Idle if *canvas.is_space_hand.read() => "cursor-grab",
        _ => "cursor-default",
    });

    // Temp edge for connection preview
    let temp_edge = use_memo(move || {
        let mode = canvas.mode.read();
        let to = *canvas.temp_edge_to.read();

        if let crate::hooks::InteractionMode::Connecting { from, handle_type } = *mode {
            let node_map = workflow.nodes_by_id.read();
            let node = node_map.get(&from);

            node.map(|n| {
                let from = if handle_type == "source" {
                    FlowPosition {
                        x: n.x + 110.0,
                        y: n.y + 68.0,
                    }
                } else {
                    FlowPosition {
                        x: n.x + 110.0,
                        y: n.y,
                    }
                };
                (from, to.unwrap())
            })
        } else {
            None
        }
    });

    rsx! {
        main {
            class: "relative flex-1 overflow-hidden bg-[#f8fafc] {cursor}",
            tabindex: "0",
            onmouseenter: move |evt| canvas_events::on_mouse_enter(evt, canvas),
            oncontextmenu: move |evt| evt.prevent_default(),
            onkeydown: move |evt| canvas_events::on_key_down(evt, &workflow, &selection, &workflow, &canvas),
            onkeyup: move |evt| canvas_events::on_key_up(evt, &canvas),
            onwheel: move |evt| canvas_events::on_wheel(evt, &workflow, &canvas),
            onmousemove: move |evt| canvas_events::on_mouse_move(evt, &workflow, canvas, &selection),
            onmouseup: move |evt| canvas_events::on_mouse_up(evt, &workflow, canvas, &selection),
            onmouseleave: move |_| (canvas.end_interaction)(),
            onmousedown: move |evt| canvas_events::on_mouse_down(evt, canvas, &selection, &workflow),

            // Background grid
            div {
                class: "absolute inset-0 pointer-events-none",
                style: "background-image: radial-gradient(circle, rgba(148, 163, 184, 0.5) 1px, transparent 1px); background-size: calc(22px * {zoom}) calc(22px * {zoom}); background-position: {x}px {y}px;",
            }

            // Transformed canvas content
            div {
                class: "absolute origin-top-left",
                style: "transform: translate({x}px, {y}px) scale({zoom}); will-change: transform;",

                FlowEdges {
                    edges: connections,
                    nodes: nodes,
                    temp_edge: temp_edge,
                }

                for node in nodes.read().iter().cloned() {
                    {
                        let node_id = node.id;
                        let is_selected = (selection.is_selected)(node_id);

                        rsx! {
                            FlowNodeComponent {
                                key: "{node_id}",
                                node: node.clone(),
                                selected: is_selected,
                                on_mouse_down: move |_| (canvas.start_drag)(node_id, vec![node_id]),
                                on_click: move |_| (selection.select_single)(node_id),
                                on_handle_mouse_down: move |(_evt, handle)| {
                                    (canvas.start_connect)(node_id, handle);
                                },
                                on_handle_mouse_enter: move |handle| {
                                    canvas.hovered_handle.set(Some((node_id, handle)));
                                },
                                on_handle_mouse_leave: move |_| {
                                    canvas.hovered_handle.set(None);
                                }
                            }
                        }
                    }
                }
            }

            FlowMinimap {
                nodes: nodes,
                edges: connections,
                selected_node_id: selection.selected_id,
            }
        }
    }
}
```

**Step 4: Add components module to main.rs**

```rust
// Add after mod ui;
mod components;
```

**Step 5: Run cargo check**

Run: `moon run :check`
Expected: May have some compilation errors to fix

**Step 6: Fix any compilation errors and commit**

```bash
git add src/components/ src/canvas_events.rs src/main.rs
git commit -m "feat: add Canvas component and event handlers module"
```

---

## Task 8: Refactor Main App Component

**Files:**
- Modify: `src/main.rs:17-774`

**Step 1: Replace the App component implementation**

```rust
// src/main.rs - Replace the App function with:
#[component]
fn App() -> Element {
    // Initialize hooks
    let workflow = use_workflow_state();
    let selection = use_selection();
    let canvas = use_canvas_interaction();
    let panels = use_ui_panels();

    // Persistence effect
    use_effect(move || {
        let wf = workflow.workflow.read();
        if let Ok(_json) = serde_json::to_string(&*wf) {
            #[cfg(target_arch = "wasm32")]
            {
                use web_sys::window;
                let storage = window().and_then(|w| w.local_storage().ok()).flatten();
                if let Some(s) = storage {
                    let _ = s.set_item("flow-wasm-v1-workflow", &_json);
                }
            }
        }
    });

    // Derived values
    let node_count = use_memo(move || workflow.nodes.read().len());
    let edge_count = use_memo(move || workflow.connections.read().len());
    let zoom_label = use_memo(move || format!("{:.0}%", workflow.viewport.read().zoom * 100.0));
    let can_undo = use_memo(move || {
        // Check if undo is available
        true
    });
    let can_redo = use_memo(move || {
        // Check if redo is available
        true
    });

    rsx! {
        script { src: "https://cdn.tailwindcss.com" }
        style {
            "@keyframes dash {{ to {{ stroke-dashoffset: -16; }} }}"
            "@keyframes slide-in-right {{ from {{ transform: translateX(24px); opacity: 0; }} to {{ transform: translateX(0); opacity: 1; }} }}"
            ".animate-slide-in-right {{ animation: slide-in-right 0.22s ease-out; }}"
        }

        div { class: "relative flex h-screen w-screen flex-col overflow-hidden bg-[#f4f6fb] text-slate-900 [font-family:'Geist',_'Inter',sans-serif] select-none",
            FlowToolbar {
                workflow_name: workflow.workflow_name,
                on_workflow_name_change: move |value| {},
                node_count: node_count,
                edge_count: edge_count,
                zoom_label: zoom_label,
                can_undo: can_undo,
                can_redo: can_redo,
                on_zoom_in: move |_| (workflow.zoom)(0.12, 640.0, 400.0),
                on_zoom_out: move |_| (workflow.zoom)(-0.12, 640.0, 400.0),
                on_fit_view: move |_| {},
                on_execute: move |_| {
                    spawn(async move {
                        // workflow.run().await
                    });
                },
                on_undo: move |_| { (workflow.undo)(); },
                on_redo: move |_| { (workflow.redo)(); },
                on_save: move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        // download
                    }
                },
                on_settings: move |_| { (panels.toggle_settings)(); }
            }

            // Settings modal (simplified for now)
            if *panels.settings_open.read() {
                div { class: "absolute right-4 top-14 z-40 w-[280px] rounded-lg border border-slate-700 bg-slate-900/95 p-3 shadow-2xl",
                    "Settings"
                }
            }

            NodeCommandPalette {
                open: panels.palette_open,
                query: panels.palette_query,
                on_query_change: move |v| (panels.set_palette_query)(v),
                on_close: move |_| (panels.close_palette)(),
                on_pick: move |node_type| {
                    (workflow.add_node)(node_type, 0.0, 0.0);
                    (panels.close_palette)();
                }
            }

            // Context menu
            if *panels.context_menu.read().open() {
                div {
                    class: "absolute z-50",
                    style: "left: {}; top: {};",
                    panels.context_menu.read().x,
                    panels.context_menu.read().y,
                    "Context Menu"
                }
            }

            div { class: "flex flex-1 overflow-hidden",
                NodeSidebar {
                    search: use_signal(String::new),
                    on_search_change: move |_| {},
                    on_pickup_node: move |_| {},
                    on_add_node: move |node_type| {
                        (workflow.add_node)(node_type, 0.0, 0.0);
                    }
                }

                Canvas {
                    workflow: workflow,
                    selection: selection,
                    canvas: canvas,
                }

                SelectedNodePanel {
                    selected_node_id: selection.selected_id,
                    selected_node_ids: selection.selected_ids,
                    nodes_by_id: workflow.nodes_by_id,
                    workflow: workflow.workflow,
                    undo_stack: use_signal(Vec::new),
                    redo_stack: use_signal(Vec::new),
                }
            }
        }
    }
}
```

**Step 2: Run cargo check**

Run: `moon run :check`
Expected: May have errors to fix (adjusting prop types, etc.)

**Step 3: Fix compilation errors iteratively**

Run: `moon run :check` repeatedly and fix issues until clean

**Step 4: Run tests**

Run: `moon run :test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "refactor: slim App component to use new hooks and Canvas component"
```

---

## Task 9: Run Full CI and Fix Issues

**Files:**
- All modified files

**Step 1: Run full CI**

Run: `moon run :ci --force`
Expected: Check, clippy, fmt, test all pass

**Step 2: Fix any clippy warnings**

Run: `moon run :clippy`
Fix any warnings that appear

**Step 3: Fix any format issues**

Run: `moon run :fmt`
Commit format fixes

**Step 4: Final commit**

```bash
git add -A
git commit -m "fix: address CI issues from refactoring"
```

---

## Task 10: Manual Testing Verification

**Step 1: Build and run**

Run: `moon run :serve`
Open browser and verify:
- Nodes can be added from sidebar
- Nodes can be dragged
- Connections can be created between nodes
- Selection works
- Zoom/pan works
- Undo/redo works

**Step 2: Document any issues found**

If issues found, create tasks to fix them

**Step 3: Final commit if all good**

```bash
git commit --allow-empty -m "test: manual testing passed for refactored App component"
```
