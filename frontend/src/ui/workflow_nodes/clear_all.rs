use crate::ui::workflow_nodes::schema::ClearAllConfig;
use crate::ui::workflow_nodes::shared::{FormHint, NodeCard};
use dioxus::prelude::*;

#[component]
pub fn ClearAllForm(config: Signal<ClearAllConfig>) -> Element {
    let _ = config; // Empty config — no fields to edit
    rsx! {
        div { class: "space-y-4",
            p { class: "text-sm text-gray-600",
                "This operation will clear ALL state keys for this Virtual Object or Workflow."
            }
            FormHint { text: "No configuration needed — ctx.clearAll() takes no arguments." }
        }
    }
}

#[component]
pub fn ClearAllNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-red-100",
            icon: "\u{1f9f9}",
            title: "Clear All",
            subtitle: "Remove all stored state values",
        }
    }
}
