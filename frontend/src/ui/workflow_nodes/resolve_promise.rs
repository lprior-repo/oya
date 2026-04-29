use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::ResolvePromiseConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "cyan";

#[component]
pub fn ResolvePromiseForm(config: Signal<ResolvePromiseConfig>) -> Element {
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
pub fn ResolvePromiseNodeCard() -> Element {
    rsx! {
        NodeCard { icon_bg: "bg-cyan-200", icon: "✅", title: "Resolve", subtitle: "Resolve a promise", service_kind: Some(ServiceKind::Workflow) }
    }
}
