use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::SignalHandlerConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "rose";

#[component]
pub fn SignalHandlerForm(mut config: Signal<SignalHandlerConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-rose-50 border border-rose-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-rose-800",
                    "📡 ",
                    strong { "Signal Handler" },
                    " - Pause until another workflow sends a signal."
                }
            }

            FormField {
                label: "Signal Name",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., payment_complete, order_shipped",
                    value: "{config.read().signal_name}",
                    oninput: move |e| {
                        config.write().signal_name = e.value();
                    }
                }
                FormHint { text: "Another workflow sends this signal by name" }
            }
        }
    }
}

#[component]
pub fn SignalHandlerNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-rose-100",
            icon: "📡",
            title: "Signal Handler",
            subtitle: "Wait for another workflow",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}

pub use SignalHandlerForm as WaitForSignalForm;
pub use SignalHandlerNodeCard as WaitForSignalNodeCard;
