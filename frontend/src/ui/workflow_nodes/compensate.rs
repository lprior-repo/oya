use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::CompensateConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "rose";

#[component]
pub fn CompensateForm(config: Signal<CompensateConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    rsx! {
        div { class: "space-y-4",
            FormField {
                label: "Compensation Handler",
                input { r#type: "text", class: "{input_cls}", value: "{config.read().handler_name.as_str()}", oninput: move |e| config.write().handler_name = crate::ui::workflow_nodes::schema::HandlerName::new(e.value().clone()) }
            }
        }
    }
}

#[component]
pub fn CompensateNodeCard() -> Element {
    rsx! {
        NodeCard { icon_bg: "bg-rose-100", icon: "↩️", title: "Compensate", subtitle: "Saga compensation path", service_kind: Some(ServiceKind::Handler) }
    }
}
