#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::graph::NodeId;
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelState {
    #[default]
    Closed,
    Open,
}

impl PanelState {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    #[must_use]
    pub fn is_open(&self) -> bool {
        matches!(self, PanelState::Open)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    #[must_use]
    pub fn toggle(&self) -> Self {
        match self {
            PanelState::Closed => PanelState::Open,
            PanelState::Open => PanelState::Closed,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuPosition {
    pub x: f32,
    pub y: f32,
}

impl MenuPosition {
    #[must_use]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum ContextMenuState {
    #[default]
    Hidden,
    Visible {
        position: MenuPosition,
    },
}

impl ContextMenuState {
    #[must_use]
    pub fn is_visible(&self) -> bool {
        matches!(self, ContextMenuState::Visible { .. })
    }

    #[must_use]
    pub fn position(&self) -> Option<MenuPosition> {
        match self {
            ContextMenuState::Hidden => None,
            ContextMenuState::Visible { position } => Some(*position),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum InlinePanelState {
    #[default]
    Closed,
    Open {
        node_id: NodeId,
    },
}

impl InlinePanelState {
    #[must_use]
    pub fn is_open(&self) -> bool {
        matches!(self, InlinePanelState::Open { .. })
    }

    #[must_use]
    pub fn is_open_for(&self, node_id: NodeId) -> bool {
        matches!(self, InlinePanelState::Open { node_id: id } if *id == node_id)
    }

    #[must_use]
    pub fn node_id(&self) -> Option<NodeId> {
        match self {
            InlinePanelState::Closed => None,
            InlinePanelState::Open { node_id } => Some(*node_id),
        }
    }

    #[must_use]
    pub fn toggle_for(&self, node_id: NodeId) -> Self {
        match self {
            InlinePanelState::Open { node_id: current } if *current == node_id => {
                InlinePanelState::Closed
            }
            _ => InlinePanelState::Open { node_id },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaletteState {
    pub visibility: PanelState,
    pub query: String,
}

impl PaletteState {
    #[must_use]
    pub fn open() -> Self {
        Self { visibility: PanelState::Open, query: String::new() }
    }

    #[must_use]
    pub fn close() -> Self {
        Self::default()
    }
}

/// Pure state struct for `UiPanels` - does not require Dioxus runtime.
/// All methods are pure state transformations.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiPanelsState {
    pub settings: PanelState,
    pub palette: PaletteState,
    pub context_menu: ContextMenuState,
    pub inline_panel: InlinePanelState,
    pub shortcuts: PanelState,
}

impl UiPanelsState {
    #[must_use]
    pub fn settings_open(&self) -> bool {
        self.settings.is_open()
    }

    #[must_use]
    pub fn palette_open(&self) -> bool {
        self.palette.visibility.is_open()
    }

    #[must_use]
    pub fn palette_query(&self) -> &str {
        &self.palette.query
    }

    #[must_use]
    pub fn toggle_settings(mut self) -> Self {
        self.settings = self.settings.toggle();
        self
    }

    #[must_use]
    pub fn open_settings(mut self) -> Self {
        self.settings = PanelState::Open;
        self
    }

    #[must_use]
    pub fn close_settings(mut self) -> Self {
        self.settings = PanelState::Closed;
        self
    }

    #[must_use]
    pub fn toggle_shortcuts(mut self) -> Self {
        self.shortcuts = self.shortcuts.toggle();
        self
    }

    #[must_use]
    pub fn close_shortcuts(mut self) -> Self {
        self.shortcuts = PanelState::Closed;
        self
    }

    #[must_use]
    pub fn toggle_palette(mut self) -> Self {
        match self.palette.visibility {
            PanelState::Closed => self.palette = PaletteState::open(),
            PanelState::Open => self.palette = PaletteState::close(),
        }
        self
    }

    #[must_use]
    pub fn open_palette(mut self) -> Self {
        self.palette = PaletteState::open();
        self
    }

    #[must_use]
    pub fn close_palette(mut self) -> Self {
        self.palette = PaletteState::close();
        self
    }

    #[must_use]
    pub fn set_palette_query(mut self, query: String) -> Self {
        self.palette.query = query;
        self
    }

    #[must_use]
    pub fn clear_palette_query(mut self) -> Self {
        self.palette.query.clear();
        self
    }

    #[must_use]
    pub fn show_context_menu(mut self, x: f32, y: f32) -> Self {
        self.context_menu = ContextMenuState::Visible { position: MenuPosition::new(x, y) };
        self
    }

    #[must_use]
    pub fn close_context_menu(mut self) -> Self {
        self.context_menu = ContextMenuState::Hidden;
        self
    }

    #[must_use]
    pub fn is_context_menu_visible(&self) -> bool {
        self.context_menu.is_visible()
    }

    #[must_use]
    pub fn close_all(mut self) -> Self {
        self.settings = PanelState::Closed;
        self.palette = PaletteState::close();
        self.context_menu = ContextMenuState::Hidden;
        self.inline_panel = InlinePanelState::Closed;
        self
    }

    #[must_use]
    pub fn any_open(&self) -> bool {
        self.settings.is_open()
            || self.palette.visibility.is_open()
            || self.context_menu.is_visible()
            || self.inline_panel.is_open()
    }

    #[must_use]
    pub fn open_inline_panel(mut self, node_id: NodeId) -> Self {
        self.inline_panel = InlinePanelState::Open { node_id };
        self
    }

    #[must_use]
    pub fn close_inline_panel(mut self) -> Self {
        self.inline_panel = InlinePanelState::Closed;
        self
    }

    #[must_use]
    pub fn toggle_inline_panel(mut self, node_id: NodeId) -> Self {
        self.inline_panel = self.inline_panel.toggle_for(node_id);
        self
    }

    #[must_use]
    pub fn is_inline_panel_open(&self, node_id: NodeId) -> bool {
        self.inline_panel.is_open_for(node_id)
    }

    #[must_use]
    pub fn inline_panel_node_id(&self) -> Option<NodeId> {
        self.inline_panel.node_id()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct UiPanels {
    settings: Signal<PanelState>,
    palette: Signal<PaletteState>,
    context_menu: Signal<ContextMenuState>,
    inline_panel: Signal<InlinePanelState>,
    shortcuts: Signal<PanelState>,
    settings_open_memo: Memo<bool>,
    palette_open_memo: Memo<bool>,
    palette_query_memo: Memo<String>,
}

impl UiPanels {
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self {
            settings: Signal::new(PanelState::Closed),
            palette: Signal::new(PaletteState::default()),
            context_menu: Signal::new(ContextMenuState::Hidden),
            inline_panel: Signal::new(InlinePanelState::Closed),
            shortcuts: Signal::new(PanelState::Closed),
            settings_open_memo: Memo::new(|| false),
            palette_open_memo: Memo::new(|| false),
            palette_query_memo: Memo::new(|| String::new()),
        }
    }

    #[must_use]
    pub fn settings_open(&self) -> ReadSignal<bool> {
        self.settings_open_memo.into()
    }

    #[must_use]
    pub fn palette_open(&self) -> ReadSignal<bool> {
        self.palette_open_memo.into()
    }

    #[must_use]
    pub fn palette_query(&self) -> ReadSignal<String> {
        self.palette_query_memo.into()
    }

    #[must_use]
    pub fn settings(&self) -> ReadSignal<PanelState> {
        self.settings.into()
    }

    #[must_use]
    pub fn palette(&self) -> ReadSignal<PaletteState> {
        self.palette.into()
    }

    #[must_use]
    pub fn context_menu(&self) -> ReadSignal<ContextMenuState> {
        self.context_menu.into()
    }

    #[must_use]
    pub fn inline_panel(&self) -> ReadSignal<InlinePanelState> {
        self.inline_panel.into()
    }

    pub fn toggle_settings(mut self) {
        let current = (*self.settings.read()).toggle();
        self.settings.set(current);
    }

    pub fn open_settings(mut self) {
        self.settings.set(PanelState::Open);
    }

    pub fn close_settings(mut self) {
        self.settings.set(PanelState::Closed);
    }

    #[must_use]
    pub fn shortcuts_open(&self) -> bool {
        self.shortcuts.read().is_open()
    }

    pub fn toggle_shortcuts(mut self) {
        let current = (*self.shortcuts.read()).toggle();
        self.shortcuts.set(current);
    }

    pub fn close_shortcuts(mut self) {
        self.shortcuts.set(PanelState::Closed);
    }

    pub fn toggle_palette(mut self) {
        let current = (*self.palette.read()).clone();
        let new_palette = match current.visibility {
            PanelState::Closed => PaletteState::open(),
            PanelState::Open => PaletteState::close(),
        };
        self.palette.set(new_palette);
    }

    pub fn open_palette(mut self) {
        self.palette.set(PaletteState::open());
    }

    pub fn close_palette(mut self) {
        self.palette.set(PaletteState::close());
    }

    pub fn set_palette_query(mut self, query: String) {
        let current = (*self.palette.read()).clone();
        self.palette.set(PaletteState { visibility: current.visibility, query });
    }

    pub fn clear_palette_query(mut self) {
        let current = (*self.palette.read()).clone();
        self.palette.set(PaletteState { visibility: current.visibility, query: String::new() });
    }

    pub fn show_context_menu(mut self, x: f32, y: f32) {
        self.context_menu.set(ContextMenuState::Visible { position: MenuPosition::new(x, y) });
    }

    pub fn close_context_menu(mut self) {
        self.context_menu.set(ContextMenuState::Hidden);
    }

    #[must_use]
    pub fn is_context_menu_visible(&self) -> bool {
        self.context_menu.read().is_visible()
    }

    pub fn close_all(mut self) {
        self.settings.set(PanelState::Closed);
        self.palette.set(PaletteState::close());
        self.context_menu.set(ContextMenuState::Hidden);
        self.inline_panel.set(InlinePanelState::Closed);
    }

    #[must_use]
    pub fn any_open(&self) -> bool {
        self.settings.read().is_open()
            || self.palette.read().visibility.is_open()
            || self.context_menu.read().is_visible()
            || self.inline_panel.read().is_open()
    }

    pub fn open_inline_panel(mut self, node_id: NodeId) {
        self.inline_panel.set(InlinePanelState::Open { node_id });
    }

    pub fn close_inline_panel(mut self) {
        self.inline_panel.set(InlinePanelState::Closed);
    }

    pub fn toggle_inline_panel(mut self, node_id: NodeId) {
        let current = (*self.inline_panel.read()).clone();
        self.inline_panel.set(current.toggle_for(node_id));
    }

    #[must_use]
    pub fn is_inline_panel_open(&self, node_id: NodeId) -> bool {
        self.inline_panel.read().is_open_for(node_id)
    }

    #[must_use]
    pub fn inline_panel_node_id(&self) -> Option<NodeId> {
        self.inline_panel.read().node_id()
    }
}

pub fn provide_ui_panels_context() -> UiPanels {
    let settings = use_signal(PanelState::default);
    let palette = use_signal(PaletteState::default);
    let context_menu = use_signal(ContextMenuState::default);
    let inline_panel = use_signal(InlinePanelState::default);
    let shortcuts = use_signal(PanelState::default);
    let settings_open_memo = use_memo(move || settings.read().is_open());
    let palette_open_memo = use_memo(move || palette.read().visibility.is_open());
    let palette_query_memo = use_memo(move || palette.read().query.clone());

    let state = UiPanels {
        settings,
        palette,
        context_menu,
        inline_panel,
        shortcuts,
        settings_open_memo,
        palette_open_memo,
        palette_query_memo,
    };
    provide_context(state)
}

#[must_use]
pub fn use_ui_panels() -> UiPanels {
    use_context::<UiPanels>()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{ContextMenuState, InlinePanelState, MenuPosition, PanelState, UiPanelsState};
    use crate::graph::NodeId;

    fn create_test_state() -> UiPanelsState {
        UiPanelsState::default()
    }

    #[test]
    fn given_closed_panel_when_toggle_then_open() {
        assert_eq!(PanelState::Closed.toggle(), PanelState::Open);
    }

    #[test]
    fn given_open_panel_when_toggle_then_closed() {
        assert_eq!(PanelState::Open.toggle(), PanelState::Closed);
    }

    #[test]
    fn given_hidden_context_menu_when_showing_then_visible() {
        let state = ContextMenuState::Visible { position: MenuPosition::new(100.0, 200.0) };
        assert!(state.is_visible());
    }

    #[test]
    fn given_inline_panel_closed_when_toggle_for_then_opens() {
        let id = NodeId::new();
        let state = InlinePanelState::Closed;
        let toggled = state.toggle_for(id);
        assert!(toggled.is_open_for(id));
    }

    #[test]
    fn given_inline_panel_open_for_node_when_toggle_same_node_then_closes() {
        let id = NodeId::new();
        let state = InlinePanelState::Open { node_id: id };
        let toggled = state.toggle_for(id);
        assert!(matches!(toggled, InlinePanelState::Closed));
    }

    #[test]
    fn given_inline_panel_open_for_one_node_when_toggle_different_node_then_switches() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let state = InlinePanelState::Open { node_id: id1 };
        let toggled = state.toggle_for(id2);
        assert!(toggled.is_open_for(id2));
    }

    #[test]
    fn test_toggle_settings_opens_from_closed() {
        let state = create_test_state();

        let state = state.toggle_settings();

        assert!(state.settings_open());
    }

    #[test]
    fn test_toggle_settings_closes_from_open() {
        let state = create_test_state().open_settings();

        let state = state.toggle_settings();

        assert!(!state.settings_open());
    }

    #[test]
    fn test_toggle_palette_opens_and_clears_query() {
        let state = create_test_state();

        let state = state.toggle_palette();

        assert!(state.palette_open());
        assert!(state.palette.query.is_empty());
    }

    #[test]
    fn test_context_menu_show_at_position() {
        let state = create_test_state();

        let state = state.show_context_menu(150.0, 250.0);

        assert!(state.is_context_menu_visible());
        let pos = state.context_menu.position();
        assert_eq!(pos.map(|p| (p.x, p.y)), Some((150.0, 250.0)));
    }

    #[test]
    fn test_close_all_clears_everything() {
        let state = create_test_state()
            .open_settings()
            .open_palette()
            .show_context_menu(100.0, 200.0)
            .open_inline_panel(NodeId::new());

        let state = state.close_all();

        assert!(!state.settings_open());
        assert!(!state.palette_open());
        assert!(!state.is_context_menu_visible());
        assert!(!state.inline_panel.is_open());
    }

    #[test]
    fn test_any_open_returns_false_when_all_closed() {
        let state = create_test_state();
        assert!(!state.any_open());
    }

    #[test]
    fn test_any_open_returns_true_when_panel_open() {
        let state = create_test_state().open_settings();
        assert!(state.any_open());
    }

    #[test]
    fn test_open_palette_clears_query() {
        let state = create_test_state().set_palette_query("previous".to_string());

        let state = state.open_palette();

        assert!(state.palette.query.is_empty());
    }
}
