use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::SwitchConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "pink";

#[component]
pub fn SwitchForm(mut config: Signal<SwitchConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",
            div {
                class: "bg-pink-50 border border-pink-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-pink-800",
                    "🔀 ",
                    strong { "Switch" },
                    " - Go to different steps based on an expression."
                }
            }

            FormField {
                label: "Expression",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., {{{{ steps.total.amount }}}} > 100",
                    value: "{config.read().expression.as_deref().unwrap_or(\"\")}",
                    oninput: move |e| {
                        let value = e.value();
                        config.write().expression = if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.into())
                        };
                    }
                }
                FormHint { text: "The condition to evaluate for branching" }
            }
        }
    }
}

#[component]
pub fn SwitchNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-pink-100",
            icon: "🔀",
            title: "Switch",
            subtitle: "Go different ways based on conditions",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}

pub use SwitchForm as RouterForm;
pub use SwitchNodeCard as RouterNodeCard;
