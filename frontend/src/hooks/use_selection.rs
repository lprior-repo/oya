#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::graph::NodeId;
use dioxus::prelude::*;

fn toggle_selection_ids(current: &[NodeId], id: NodeId) -> (Vec<NodeId>, Option<NodeId>) {
    let next: Vec<NodeId> = if current.contains(&id) {
        current.iter().copied().filter(|&item| item != id).collect()
    } else {
        current.iter().copied().chain(std::iter::once(id)).collect()
    };

    let selected_id = if next.contains(&id) { Some(id) } else { next.first().copied() };

    (next, selected_id)
}

fn add_unique_selection(current: &[NodeId], id: NodeId) -> Vec<NodeId> {
    if current.contains(&id) {
        return current.to_vec();
    }
    current.iter().copied().chain(std::iter::once(id)).collect()
}

fn reconcile_primary_selection(
    current_primary: Option<NodeId>,
    selected_ids: &[NodeId],
) -> Option<NodeId> {
    if selected_ids.is_empty() {
        return None;
    }

    current_primary.filter(|id| selected_ids.contains(id)).or_else(|| selected_ids.first().copied())
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Selection {
    #[default]
    None,
    Single {
        node_id: NodeId,
    },
    Multiple {
        primary: NodeId,
        secondary: Vec<NodeId>,
    },
}

impl Selection {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Selection::None)
    }

    #[must_use]
    pub fn primary(&self) -> Option<NodeId> {
        match self {
            Selection::None => None,
            Selection::Single { node_id } => Some(*node_id),
            Selection::Multiple { primary, .. } => Some(*primary),
        }
    }

    #[must_use]
    pub fn all_ids(&self) -> Vec<NodeId> {
        match self {
            Selection::None => Vec::new(),
            Selection::Single { node_id } => vec![*node_id],
            Selection::Multiple { primary, secondary } => {
                std::iter::once(*primary).chain(secondary.iter().copied()).collect()
            }
        }
    }

    #[must_use]
    pub fn contains(&self, id: NodeId) -> bool {
        match self {
            Selection::None => false,
            Selection::Single { node_id } => *node_id == id,
            Selection::Multiple { primary, secondary } => *primary == id || secondary.contains(&id),
        }
    }

    #[must_use]
    pub fn count(&self) -> usize {
        match self {
            Selection::None => 0,
            Selection::Single { .. } => 1,
            Selection::Multiple { primary: _, secondary } => 1 + secondary.len(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum PendingDrag {
    #[default]
    None,
    Ready {
        node_ids: Vec<NodeId>,
    },
}

impl PendingDrag {
    // Test-only: convenient constructor for PendingDrag::None
    #[allow(dead_code)]
    #[must_use]
    pub fn none() -> Self {
        Self::None
    }

    #[must_use]
    pub fn ready(node_ids: Vec<NodeId>) -> Self {
        Self::Ready { node_ids }
    }

    // Test-only: check if drag state is Ready
    #[allow(dead_code)]
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self, PendingDrag::Ready { .. })
    }

    // Test-only: access node IDs from drag state
    #[allow(dead_code)]
    #[must_use]
    pub fn node_ids(&self) -> Option<&[NodeId]> {
        match self {
            PendingDrag::None => None,
            PendingDrag::Ready { node_ids } => Some(node_ids),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SelectionState {
    selection: Signal<Selection>,
    pending_drag: Signal<PendingDrag>,
    primary_memo: Memo<Option<NodeId>>,
    all_ids_memo: Memo<Vec<NodeId>>,
}

impl SelectionState {
    #[must_use]
    pub fn selection(&self) -> ReadSignal<Selection> {
        self.selection.into()
    }

    #[must_use]
    pub fn selected_id(&self) -> ReadSignal<Option<NodeId>> {
        self.primary_memo.into()
    }

    #[must_use]
    pub fn selected_ids(&self) -> ReadSignal<Vec<NodeId>> {
        self.all_ids_memo.into()
    }

    pub fn select_single(mut self, id: NodeId) {
        self.selection.set(Selection::Single { node_id: id });
    }

    pub fn toggle(mut self, id: NodeId) {
        let current_ids = self.selection.read().all_ids();
        let (new_ids, selected_id) = toggle_selection_ids(&current_ids, id);

        let new_selection = match new_ids.len() {
            0 => Selection::None,
            1 => Selection::Single { node_id: new_ids[0] },
            _ => Selection::Multiple {
                primary: match selected_id {
                    Some(id) => id,
                    None => match new_ids.first() {
                        Some(id) => *id,
                        None => return,
                    },
                },
                secondary: new_ids.into_iter().skip(1).collect(),
            },
        };
        self.selection.set(new_selection);
    }

    pub fn add_to_selection(mut self, id: NodeId) {
        let current_ids = self.selection.read().all_ids();
        let next_ids = add_unique_selection(&current_ids, id);

        let new_selection = match next_ids.len() {
            0 => Selection::None,
            1 => Selection::Single { node_id: next_ids[0] },
            _ => {
                let current_primary = self.selection.read().primary();
                let primary = match reconcile_primary_selection(current_primary, &next_ids) {
                    Some(id) => id,
                    None => match next_ids.first() {
                        Some(id) => *id,
                        None => return,
                    },
                };
                let secondary: Vec<NodeId> =
                    next_ids.into_iter().filter(|&n| n != primary).collect();
                Selection::Multiple { primary, secondary }
            }
        };
        self.selection.set(new_selection);
    }

    pub fn set_multiple(mut self, ids: Vec<NodeId>) {
        let new_selection = match ids.len() {
            0 => Selection::None,
            1 => Selection::Single { node_id: ids[0] },
            _ => Selection::Multiple {
                primary: ids[0],
                secondary: ids.into_iter().skip(1).collect(),
            },
        };
        self.selection.set(new_selection);
    }

    pub fn clear(mut self) {
        self.selection.set(Selection::None);
    }

    pub fn set_pending_drag(mut self, ids: Vec<NodeId>) {
        self.pending_drag.set(PendingDrag::ready(ids));
    }

    pub fn clear_pending_drag(mut self) {
        self.pending_drag.set(PendingDrag::None);
    }

    #[must_use]
    pub fn take_pending_drag(mut self) -> Option<Vec<NodeId>> {
        let result = match self.pending_drag.read().clone() {
            PendingDrag::None => None,
            PendingDrag::Ready { node_ids } => Some(node_ids),
        };
        if result.is_some() {
            self.pending_drag.set(PendingDrag::None);
        }
        result
    }

    #[must_use]
    pub fn pending_drag(&self) -> ReadSignal<PendingDrag> {
        self.pending_drag.into()
    }

    #[must_use]
    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selection.read().contains(id)
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.selection.read().count()
    }

    #[must_use]
    pub fn has_selection(&self) -> bool {
        !self.selection.read().is_empty()
    }
}

pub fn provide_selection_context() -> SelectionState {
    let selection = use_signal(Selection::default);
    let pending_drag = use_signal(PendingDrag::default);
    let primary_memo = use_memo(move || selection.read().primary());
    let all_ids_memo = use_memo(move || selection.read().all_ids());

    let state = SelectionState { selection, pending_drag, primary_memo, all_ids_memo };
    provide_context(state)
}

#[must_use]
pub fn use_selection() -> SelectionState {
    use_context::<SelectionState>()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{
        add_unique_selection, reconcile_primary_selection, toggle_selection_ids, PendingDrag,
        Selection,
    };
    use crate::graph::NodeId;

    #[test]
    fn given_selected_node_when_toggling_existing_then_node_is_removed_and_primary_updates() {
        let a = NodeId::new();
        let b = NodeId::new();
        let current = vec![a, b];

        let (next, selected) = toggle_selection_ids(&current, a);

        assert_eq!(next, vec![b]);
        assert_eq!(selected, Some(b));
    }

    #[test]
    fn given_duplicate_add_when_adding_to_selection_then_selection_remains_unique() {
        let a = NodeId::new();
        let current = vec![a];

        let next = add_unique_selection(&current, a);

        assert_eq!(next, vec![a]);
    }

    #[test]
    fn given_missing_node_when_toggling_then_node_is_added_and_selected() {
        let a = NodeId::new();
        let b = NodeId::new();
        let current = vec![a];

        let (next, selected) = toggle_selection_ids(&current, b);

        assert_eq!(next, vec![a, b]);
        assert_eq!(selected, Some(b));
    }

    #[test]
    fn given_current_primary_in_selection_when_reconciling_then_primary_is_preserved() {
        let a = NodeId::new();
        let b = NodeId::new();

        let primary = reconcile_primary_selection(Some(b), &[a, b]);

        assert_eq!(primary, Some(b));
    }

    #[test]
    fn given_missing_primary_when_reconciling_then_first_selected_becomes_primary() {
        let a = NodeId::new();
        let b = NodeId::new();

        let primary = reconcile_primary_selection(None, &[a, b]);

        assert_eq!(primary, Some(a));
    }

    #[test]
    fn given_empty_selection_when_reconciling_then_primary_is_none() {
        let a = NodeId::new();

        let primary = reconcile_primary_selection(Some(a), &[]);

        assert_eq!(primary, None);
    }

    #[test]
    fn given_selection_none_when_is_empty_then_true() {
        assert!(Selection::None.is_empty());
    }

    #[test]
    fn given_selection_single_when_is_empty_then_false() {
        let sel = Selection::Single { node_id: NodeId::new() };
        assert!(!sel.is_empty());
    }

    #[test]
    fn given_selection_multiple_when_count_then_returns_correct_count() {
        let a = NodeId::new();
        let b = NodeId::new();
        let c = NodeId::new();
        let sel = Selection::Multiple { primary: a, secondary: vec![b, c] };
        assert_eq!(sel.count(), 3);
    }

    #[test]
    fn given_pending_drag_ready_when_node_ids_then_some() {
        let ids = vec![NodeId::new()];
        let drag = PendingDrag::ready(ids.clone());
        assert_eq!(drag.node_ids(), Some(&ids[..]));
    }

    #[test]
    fn given_pending_drag_none_when_node_ids_then_none() {
        let drag = PendingDrag::none();
        assert!(drag.node_ids().is_none());
    }
}
