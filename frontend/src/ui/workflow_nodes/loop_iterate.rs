use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::LoopConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "purple";

#[component]
pub fn LoopIterateForm(mut config: Signal<LoopConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    rsx! {
        div { class: "space-y-4",
            FormField {
                label: "Iterator Variable",
                input { r#type: "text", class: "{input_cls}", value: "{config.read().iterator}", oninput: move |e| config.write().iterator = e.value().clone() }
            }
            FormField {
                label: "Collection",
                input { r#type: "text", class: "{input_cls}", value: "{config.read().collection}", oninput: move |e| config.write().collection = e.value().clone() }
            }
        }
    }
}

#[component]
pub fn LoopIterateNodeCard() -> Element {
    rsx! {
        NodeCard { icon_bg: "bg-purple-100", icon: "🔁", title: "Loop Iterate", subtitle: "Iterate over a collection", service_kind: Some(ServiceKind::Handler) }
    }
}
