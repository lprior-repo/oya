use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::ObjectCallConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "cyan";

#[component]
pub fn ObjectCallForm(mut config: Signal<ObjectCallConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    let object_name_value =
        config.read().object_name.as_ref().map(|n| n.as_str()).unwrap_or("").to_string();

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-cyan-50 border border-cyan-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-cyan-800",
                    "📦 ",
                    strong { "Object Call" },
                    " - Call a method on a Restate Virtual Object."
                }
            }

            FormField {
                label: "Object Name",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., cart_service",
                    value: "{object_name_value}",
                    oninput: move |e| {
                        let value = e.value();
                        config.write().object_name = if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.into())
                        };
                    }
                }
                FormHint { text: "The name of the Virtual Object to call" }
            }

            div {
                class: "bg-yellow-50 p-3 rounded-lg",
                p {
                    class: "text-sm text-yellow-800",
                    "💡 Object calls are stateful and always routed to the same instance."
                }
            }
        }
    }
}

#[component]
pub fn ObjectCallNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-cyan-100",
            icon: "📦",
            title: "Object Call",
            subtitle: "Call Virtual Object method",
            service_kind: Some(ServiceKind::Actor),
        }
    }
}
