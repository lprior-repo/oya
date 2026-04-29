use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::AwakeableConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "teal";

#[component]
pub fn AwakeableForm(mut config: Signal<AwakeableConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-teal-50 border border-teal-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-teal-800",
                    "🪝 ",
                    strong { "Awakeable" },
                    " - Pause until an external service calls back."
                }
            }

            FormField {
                label: "Awakeable ID",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., payment_callback, approval_response",
                    value: "{config.read().awakeable_id}",
                    oninput: move |e| {
                        config.write().awakeable_id = e.value();
                    }
                }
                FormHint { text: "Use the same awakeable ID in your callback resolver" }
            }
        }
    }
}

#[component]
pub fn AwakeableNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-teal-100",
            icon: "🪝",
            title: "Awakeable",
            subtitle: "Pause until external callback",
            service_kind: Some(ServiceKind::Workflow),
        }
    }
}

pub use AwakeableForm as WaitForWebhookForm;
pub use AwakeableNodeCard as WaitForWebhookNodeCard;
