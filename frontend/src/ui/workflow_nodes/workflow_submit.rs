use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::WorkflowSubmitConfig;
use crate::ui::workflow_nodes::shared::{
    input_classes, json_to_display, parse_json_draft, textarea_classes, FormField, FormHint,
    NodeCard,
};
use dioxus::prelude::*;

const FOCUS_RING: &str = "blue";

#[component]
pub fn WorkflowSubmitForm(mut config: Signal<WorkflowSubmitConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    let textarea_cls = textarea_classes(FOCUS_RING);

    let pretty_input = json_to_display(&config.read().input);
    let mut json_draft = use_signal(|| pretty_input.clone());
    let mut json_error = use_signal(|| Option::<String>::None);
    let mut last_synced_input = use_signal(|| pretty_input.clone());

    use_effect(move || {
        let latest = json_to_display(&config.read().input);
        if latest != last_synced_input.read().clone() {
            last_synced_input.set(latest.clone());
            json_draft.set(latest);
            json_error.set(None);
        }
    });

    let key_value = config.read().key.as_ref().map(|k| k.as_str().to_string()).unwrap_or_default();

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-blue-50 border border-blue-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-blue-800",
                    "🚀 ",
                    strong { "Starts here" },
                    " - This workflow starts when it is submitted manually or by another service."
                }
            }

            FormField {
                label: "Workflow Name",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., process-order",
                    value: "{config.read().workflow_name.as_str()}",
                    oninput: move |e| {
                        config.write().workflow_name = e.value().into();
                    }
                }
            }

            FormField {
                label: "Workflow Key (Optional)",
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
                            Some(value.into())
                        };
                    }
                }
                FormHint { text: "ID to ensure exactly-once submission" }
            }

            FormField {
                label: "Initial Data (JSON)",
                textarea {
                    class: "{textarea_cls}",
                    rows: 4,
                    placeholder: r#"{{"order_id": "abc-123"}}"#,
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
                    FormHint { text: "The starting context for this workflow" }
                }
            }
        }
    }
}

#[component]
pub fn WorkflowSubmitNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-blue-100",
            icon: "📤",
            title: "Workflow Submit",
            subtitle: "Starts when submitted",
            service_kind: Some(ServiceKind::Workflow),
        }
    }
}
