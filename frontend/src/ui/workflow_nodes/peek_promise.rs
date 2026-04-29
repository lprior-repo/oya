use crate::ui::workflow_nodes::schema::PeekPromiseConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "cyan";

#[component]
pub fn PeekPromiseForm(config: Signal<PeekPromiseConfig>) -> Element {
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
pub fn PeekPromiseNodeCard() -> Element {
    rsx! {
        NodeCard { icon_bg: "bg-cyan-100", icon: "\u{1F441}", title: "Peek", subtitle: "Non-blocking promise inspection" }
    }
}
