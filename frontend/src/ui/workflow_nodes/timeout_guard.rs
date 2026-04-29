use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::TimeoutConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "red";

#[component]
pub fn TimeoutGuardForm(mut config: Signal<TimeoutConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    rsx! {
        div { class: "space-y-4",
            FormField {
                label: "Timeout (ms)",
                input {
                    r#type: "number",
                    class: "{input_cls}",
                    value: "{config.read().timeout_ms}",
                    oninput: move |e| {
                        if let Ok(val) = e.value().parse::<u64>() {
                            config.write().timeout_ms = val;
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn TimeoutGuardNodeCard() -> Element {
    rsx! {
        NodeCard { icon_bg: "bg-red-100", icon: "⏱️", title: "Timeout", subtitle: "Add execution timeout", service_kind: Some(ServiceKind::Handler) }
    }
}

pub use TimeoutGuardNodeCard as TimeoutNodeCard;
