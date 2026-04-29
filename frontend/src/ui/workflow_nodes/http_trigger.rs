use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::{HttpHandlerConfig, HttpMethod, HttpPath};
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "blue";

#[component]
pub fn HttpHandlerForm(mut config: Signal<HttpHandlerConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-blue-50 border border-blue-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-blue-800",
                    "🚀 ",
                    strong { "Starts here" },
                    " - This workflow runs when an HTTP request comes in."
                }
            }

            FormField {
                label: "Path",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "/orders/{{order_id}}",
                    value: "{config.read().path.as_str()}",
                    oninput: move |e| {
                        config.write().path = HttpPath::new(e.value());
                    }
                }
                FormHint { text: "The HTTP route that starts this workflow" }
            }

            FormField {
                label: "HTTP Method",
                select {
                    class: "{input_cls}",
                    value: match config.read().method {
                        HttpMethod::GET => "GET",
                        HttpMethod::POST => "POST",
                        HttpMethod::PUT => "PUT",
                        HttpMethod::DELETE => "DELETE",
                        HttpMethod::PATCH => "PATCH",
                    },
                    onchange: move |e| {
                        config.write().method = match e.value().as_str() {
                            "GET" => HttpMethod::GET,
                            "POST" => HttpMethod::POST,
                            "PUT" => HttpMethod::PUT,
                            "DELETE" => HttpMethod::DELETE,
                            "PATCH" => HttpMethod::PATCH,
                            _ => HttpMethod::POST,
                        };
                    },
                    option { value: "GET", "GET" }
                    option { value: "POST", "POST" }
                    option { value: "PUT", "PUT" }
                    option { value: "DELETE", "DELETE" }
                    option { value: "PATCH", "PATCH" }
                }
                FormHint { text: "What kind of request this handler accepts" }
            }
        }
    }
}

#[component]
pub fn HttpHandlerNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-blue-100",
            icon: "🌐",
            title: "HTTP Handler",
            subtitle: "Starts when someone calls this URL",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}
