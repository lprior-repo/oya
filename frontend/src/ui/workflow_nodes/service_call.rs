use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::{
    Condition, HandlerName, ObjectKey, ServiceCallConfig, ServiceName, TargetType,
};
use crate::ui::workflow_nodes::shared::{
    input_classes, json_to_display, parse_json_draft, textarea_classes, FormField, FormHint,
    NodeCard,
};
use dioxus::prelude::*;

const FOCUS_RING: &str = "green";

#[component]
pub fn ServiceCallForm(mut config: Signal<ServiceCallConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    let textarea_cls = textarea_classes(FOCUS_RING);

    let pretty_input = json_to_display(&config.read().input);
    let mut json_draft = use_signal(|| pretty_input.clone());
    let mut json_error = use_signal(|| Option::<String>::None);
    let mut last_synced_input = use_signal(|| pretty_input.clone());

    use_effect(move || {
        let latest = json_to_display(&config.read().input);
        let synced = last_synced_input.read().clone();
        if latest != synced {
            last_synced_input.set(latest.clone());
            json_draft.set(latest);
            json_error.set(None);
        }
    });

    let key_value = config.read().key.as_ref().map(|k| k.as_str().to_string()).unwrap_or_default();
    let condition_value =
        config.read().condition.as_ref().map(|c| c.as_str().to_string()).unwrap_or_default();

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-green-50 border border-green-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-green-800",
                    "🔗 ",
                    strong { "Action" },
                    " - Call another service and wait for the response."
                }
            }

            FormField {
                label: "What type of thing are you calling?",
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
                    option { value: "Service", "Service (stateless)" }
                    option { value: "Virtual Object", "Virtual Object (stateful)" }
                    option { value: "Workflow", "Workflow (long-running)" }
                }
            }

            FormField {
                label: "Service Name",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "/orders/{{order_id}}",
                    value: "{config.read().service_name.as_str()}",
                    oninput: move |e| {
                        config.write().service_name = ServiceName::new(e.value());
                    }
                }
            }

            FormField {
                label: "Object/Workflow Key",
                div {
                    class: if matches!(config.read().target_type, TargetType::VirtualObject | TargetType::Workflow) { "" } else { "hidden" },
                    input {
                        r#type: "text",
                        class: "{input_cls}",
                        placeholder: "e.g., order-123",
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
                    FormHint { text: "Which specific object/workflow to call" }
                }
            }

            FormField {
                label: "What to do",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., charge_card",
                    value: "{config.read().handler_name.as_str()}",
                    oninput: move |e| {
                        config.write().handler_name = HandlerName::new(e.value());
                    }
                }
            }

            FormField {
                label: "What to send (JSON)",
                textarea {
                    class: "{textarea_cls}",
                    rows: 4,
                    placeholder: r#"{{"amount": 100, "currency": "USD"}}"#,
                    value: "{json_draft}",
                    oninput: move |e| {
                        let draft = e.value();
                        json_draft.set(draft.clone());

                        match parse_json_draft(&draft) {
                            Ok(value) => {
                                config.write().input = value;
                                last_synced_input.set(draft);
                                json_error.set(None);
                            }
                            Err(error) => {
                                json_error.set(Some(error));
                            }
                        }
                    },
                }
                if let Some(error) = json_error() {
                    p {
                        class: "text-xs text-red-600 mt-1",
                        "{error}"
                    }
                } else {
                    FormHint { text: "Enter valid JSON to update this field" }
                }
            }

            FormField {
                label: "Only run if...",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., {{{{ steps.validate.valid }}}} == true",
                    value: "{condition_value}",
                    oninput: move |e| {
                        let value = e.value();
                        config.write().condition = if value.trim().is_empty() {
                            None
                        } else {
                            Some(Condition::new(value))
                        };
                    }
                }
                FormHint { text: "Leave empty to always run this step" }
            }
        }
    }
}

#[component]
pub fn ServiceCallNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-green-100",
            icon: "🔗",
            title: "Call Service",
            subtitle: "Call another service and wait for result",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}
