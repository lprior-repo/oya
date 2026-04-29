use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::AwakeableConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "amber";

#[component]
pub fn AwakeableForm(mut config: Signal<AwakeableConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-amber-50 border border-amber-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-amber-800",
                    "📡 ",
                    strong { "Awakeable" },
                    " - Create a callback that can be awakened later."
                }
            }

            FormField {
                label: "Awakeable ID",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "auto-generated if empty",
                    value: "{config.read().awakeable_id}",
                    oninput: move |e| {
                        config.write().awakeable_id = e.value();
                    }
                }
                FormHint { text: "Unique identifier for this awakeable callback" }
            }

            div {
                class: "bg-blue-50 p-3 rounded-lg",
                p {
                    class: "text-sm text-blue-800",
                    "💡 Use this to create a suspension point that can be resumed later by another workflow or service."
                }
            }
        }
    }
}

#[component]
pub fn AwakeableNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-amber-100",
            icon: "📡",
            title: "Awakeable",
            subtitle: "Create a resumption point",
            service_kind: Some(ServiceKind::Workflow),
        }
    }
}
