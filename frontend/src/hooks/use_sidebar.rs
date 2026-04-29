#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeType(String);

impl NodeType {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for NodeType {
    fn from(s: &'static str) -> Self {
        Self::new(s)
    }
}

impl From<String> for NodeType {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum DropState {
    #[default]
    Idle,
    Dragging {
        node_type: NodeType,
    },
}

impl DropState {
    #[allow(dead_code)]
    #[must_use]
    pub fn idle() -> Self {
        Self::Idle
    }

    #[must_use]
    pub fn dragging(node_type: NodeType) -> Self {
        Self::Dragging { node_type }
    }

    #[must_use]
    pub fn is_dragging(&self) -> bool {
        matches!(self, DropState::Dragging { .. })
    }

    #[must_use]
    pub fn node_type(&self) -> Option<&NodeType> {
        match self {
            DropState::Idle => None,
            DropState::Dragging { node_type } => Some(node_type),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchQuery(String);

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self(query.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl From<String> for SearchQuery {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SidebarState {
    search: Signal<SearchQuery>,
    drop_state: Signal<DropState>,
}

impl SidebarState {
    #[must_use]
    pub fn search(&self) -> ReadSignal<SearchQuery> {
        self.search.into()
    }

    #[must_use]
    pub fn drop_state(&self) -> ReadSignal<DropState> {
        self.drop_state.into()
    }

    pub fn set_search(mut self, query: String) {
        self.search.set(SearchQuery::new(query));
    }

    pub fn set_drop_state(mut self, state: DropState) {
        self.drop_state.set(state);
    }

    pub fn pickup_node(mut self, node_type: &'static str) {
        self.drop_state.set(DropState::dragging(NodeType::from(node_type)));
    }

    pub fn clear_drop(mut self) {
        self.drop_state.set(DropState::Idle);
    }

    pub fn clear_pending_drop(self) {
        self.clear_drop();
    }

    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.drop_state.read().is_dragging()
    }

    #[must_use]
    pub fn has_pending_drop(&self) -> bool {
        self.is_dragging()
    }

    #[must_use]
    pub fn pending_drop(&self) -> Option<NodeType> {
        self.drop_state.read().node_type().cloned()
    }

    #[must_use]
    pub fn dragged_node_type(&self) -> Option<String> {
        self.drop_state.read().node_type().map(|t| t.as_str().to_string())
    }
}

pub fn provide_sidebar_context() -> SidebarState {
    let search = use_signal(SearchQuery::default);
    let drop_state = use_signal(DropState::default);

    let state = SidebarState { search, drop_state };
    provide_context(state)
}

#[must_use]
pub fn use_sidebar() -> SidebarState {
    use_context::<SidebarState>()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{DropState, NodeType, SearchQuery};

    #[test]
    fn given_idle_drop_state_when_is_dragging_then_false() {
        assert!(!DropState::Idle.is_dragging());
    }

    #[test]
    fn given_dragging_drop_state_when_is_dragging_then_true() {
        let state = DropState::dragging(NodeType::from("run"));
        assert!(state.is_dragging());
    }

    #[test]
    fn given_dragging_state_when_node_type_then_returns_type() {
        let state = DropState::dragging(NodeType::from("http-handler"));
        assert_eq!(state.node_type().map(|t| t.as_str()), Some("http-handler"));
    }

    #[test]
    fn given_idle_state_when_node_type_then_none() {
        assert!(DropState::Idle.node_type().is_none());
    }

    #[test]
    fn given_empty_search_query_when_is_empty_then_true() {
        assert!(SearchQuery::default().is_empty());
    }

    #[test]
    fn given_non_empty_search_query_when_is_empty_then_false() {
        let query = SearchQuery::new("test");
        assert!(!query.is_empty());
    }
}
