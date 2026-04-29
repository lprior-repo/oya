use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::ConditionConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "green";

#[component]
pub fn ConditionForm(mut config: Signal<ConditionConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    let expression_value =
        config.read().expression.as_ref().map(|e| e.as_str()).unwrap_or("").to_string();

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-green-50 border border-green-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-green-800",
                    "🌳 ",
                    strong { "Condition" },
                    " - Branch based on a true/false condition."
                }
            }

            FormField {
                label: "Expression",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., steps.validate.valid == true",
                    value: "{expression_value}",
                    oninput: move |e| {
                        let value = e.value();
                        config.write().expression = if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.into())
                        };
                    }
                }
                FormHint { text: "The condition to evaluate" }
            }

            div {
                class: "bg-yellow-50 p-3 rounded-lg",
                p {
                    class: "text-sm text-yellow-800",
                    "⚠️ This node has two output ports: 'true' and 'false'."
                }
            }
        }
    }
}

#[component]
pub fn ConditionNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-green-100",
            icon: "🌳",
            title: "Condition",
            subtitle: "Branch on a condition",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}
