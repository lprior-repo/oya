use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::DurablePromiseConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "cyan";

#[component]
pub fn DurablePromiseForm(config: Signal<DurablePromiseConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    rsx! {
        div { class: "space-y-4",
            FormField {
                label: "Promise Name",
                input { r#type: "text", class: "{input_cls}", value: "{config.read().promise_name.as_str()}", oninput: move |e| config.write().promise_name = crate::ui::workflow_nodes::schema::PromiseName::new(e.value().clone()) }
            }
        }
    }
}

#[component]
pub fn DurablePromiseNodeCard() -> Element {
    rsx! {
        NodeCard { icon_bg: "bg-cyan-100", icon: "🤝", title: "Promise", subtitle: "Durable awaitable promise", service_kind: Some(ServiceKind::Workflow) }
    }
}
