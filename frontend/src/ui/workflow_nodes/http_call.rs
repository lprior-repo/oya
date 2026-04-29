use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::HttpCallConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "blue";

#[component]
pub fn HttpCallForm(mut config: Signal<HttpCallConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-blue-50 border border-blue-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-blue-800",
                    "🌐 ",
                    strong { "HTTP Call" },
                    " - Call an external HTTP API and wait for response."
                }
            }

            FormField {
                label: "URL",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "https://api.example.com/data",
                    value: "{config.read().url}",
                    oninput: move |e| {
                        config.write().url = e.value();
                    }
                }
                FormHint { text: "The full URL to call" }
            }

            div {
                class: "bg-yellow-50 p-3 rounded-lg",
                p {
                    class: "text-sm text-yellow-800",
                    "⚠️ This makes an external HTTP request. Configure timeout handling for production use."
                }
            }
        }
    }
}

#[component]
pub fn HttpCallNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-blue-100",
            icon: "🌐",
            title: "HTTP Call",
            subtitle: "Call external HTTP API",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}
