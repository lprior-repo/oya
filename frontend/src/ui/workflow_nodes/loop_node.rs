use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::LoopConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "purple";

#[component]
pub fn LoopForm(mut config: Signal<LoopConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-purple-50 border border-purple-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-purple-800",
                    "🔁 ",
                    strong { "Loop" },
                    " - Iterate over a collection of items."
                }
            }

            FormField {
                label: "Iterator Variable",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., item",
                    value: "{config.read().iterator}",
                    oninput: move |e| {
                        config.write().iterator = e.value();
                    }
                }
                FormHint { text: "The variable name for each item" }
            }

            FormField {
                label: "Collection",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., steps.get_items.result",
                    value: "{config.read().collection}",
                    oninput: move |e| {
                        config.write().collection = e.value();
                    }
                }
                FormHint { text: "The collection to iterate over" }
            }
        }
    }
}

#[component]
pub fn LoopNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-purple-100",
            icon: "🔁",
            title: "Loop",
            subtitle: "Iterate over collection",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}
