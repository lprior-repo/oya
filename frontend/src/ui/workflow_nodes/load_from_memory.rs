use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::{GetStateConfig, ObjectKey};
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "cyan";

#[component]
pub fn GetStateForm(mut config: Signal<GetStateConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",
            div {
                class: "bg-cyan-50 border border-cyan-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-cyan-800",
                    "📂 ",
                    strong { "Get State" },
                    " - Get a value from Restate state."
                }
            }
            FormField {
                label: "State Key",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., cart_items",
                    value: "{config.read().key.as_str()}",
                    oninput: move |e| config.write().key = ObjectKey::new(e.value()),
                }
                FormHint { text: "The key to load the value from" }
            }
        }
    }
}

#[component]
pub fn GetStateNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-cyan-100",
            icon: "📂",
            title: "Get State",
            subtitle: "Get a saved value",
            service_kind: Some(ServiceKind::Actor),
        }
    }
}

pub use GetStateForm as LoadFromMemoryForm;
pub use GetStateNodeCard as LoadFromMemoryNodeCard;
