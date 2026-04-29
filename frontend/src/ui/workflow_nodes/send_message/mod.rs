use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::{
    HandlerName, ObjectKey, SendMessageConfig, ServiceName, TargetType,
};
use crate::ui::workflow_nodes::shared::{
    input_classes, json_to_display, parse_json_draft, textarea_classes, FormField, NodeCard,
};
use dioxus::prelude::*;

const FOCUS_RING: &str = "yellow";

#[component]
pub fn SendMessageForm(mut config: Signal<SendMessageConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    let textarea_cls = textarea_classes(FOCUS_RING);

    let pretty_input = json_to_display(&config.read().input);
    let mut draft = use_signal(move || pretty_input.clone());
    let mut parse_error = use_signal(|| None::<String>);

    let key_value = config.read().key.as_ref().map(|k| k.as_str().to_string()).unwrap_or_default();

    rsx! {
        div {
            class: "space-y-4",
            div {
                class: "bg-yellow-50 border border-yellow-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-yellow-800",
                    "📤 ",
                    strong { "Fire & Forget" },
                    " - Send a message and continue without waiting."
                }
            }
            FormField {
                label: "Send to",
                select {
                    class: "{input_cls}",
                    value: match config.read().target_type {
                        TargetType::Service => "Service",
                        TargetType::VirtualObject => "Virtual Object",
                        TargetType::Workflow => "Workflow",
                    },
                    onchange: move |e| {
                        let next_target_type = match e.value().as_str() {
                            "Service" => TargetType::Service,
                            "Virtual Object" => TargetType::VirtualObject,
                            "Workflow" => TargetType::Workflow,
                            _ => TargetType::Service,
                        };
                        let mut cfg = config.write();
                        cfg.target_type = next_target_type;
                        if matches!(next_target_type, TargetType::Service) {
                            cfg.key = None;
                        }
                    },
                    option { value: "Service", "Service" }
                    option { value: "Virtual Object", "Virtual Object" }
                    option { value: "Workflow", "Workflow" }
                }
            }
            FormField {
                label: "Service Name",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., notification_service",
                    value: "{config.read().service_name.as_str()}",
                    oninput: move |e| config.write().service_name = ServiceName::new(e.value()),
                }
            }
            FormField {
                label: "Key (for objects/workflows)",
                div {
                    class: if matches!(config.read().target_type, TargetType::VirtualObject | TargetType::Workflow) { "" } else { "hidden" },
                    input {
                        r#type: "text",
                        class: "{input_cls}",
                        placeholder: "e.g., user-456",
                        value: "{key_value}",
                        oninput: move |e| {
                            let value = e.value();
                            config.write().key = if value.trim().is_empty() {
                                None
                            } else {
                                Some(ObjectKey::new(value))
                            };
                        }
                    }
                }
            }
            FormField {
                label: "Handler",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., send_email",
                    value: "{config.read().handler_name.as_str()}",
                    oninput: move |e| config.write().handler_name = HandlerName::new(e.value()),
                }
            }
            FormField {
                label: "Message (JSON)",
                textarea {
                    class: "{textarea_cls}",
                    rows: 3,
                    value: "{draft}",
                    oninput: move |e| {
                        let next_value = e.value();
                        draft.set(next_value.clone());
                        match parse_json_draft(next_value.as_str()) {
                            Ok(value) => {
                                parse_error.set(None);
                                config.write().input = value;
                            }
                            Err(error_text) => parse_error.set(Some(error_text)),
                        }
                    },
                }
                if let Some(error_text) = parse_error.read().as_ref() {
                    p { class: "text-xs text-red-600 mt-1", "{error_text}" }
                }
            }
        }
    }
}

#[component]
pub fn SendMessageNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-yellow-100",
            icon: "📤",
            title: "Send Message",
            subtitle: "Send without waiting for response",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}
