use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::ClearStateConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "amber";

#[component]
pub fn ClearStateForm(config: Signal<ClearStateConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    rsx! {
        div { class: "space-y-4",
            FormField {
                label: "State Key",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., cart_items",
                    value: "{config.read().key.as_str()}",
                    oninput: move |e| config.write().key = crate::ui::workflow_nodes::schema::ObjectKey::new(e.value().clone()),
                }
            }
        }
    }
}

#[component]
pub fn ClearStateNodeCard() -> Element {
    rsx! {
        NodeCard { icon_bg: "bg-amber-100", icon: "🗑", title: "Clear State", subtitle: "Remove a stored state value", service_kind: Some(ServiceKind::Actor) }
    }
}
