use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::KafkaHandlerConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "blue";

#[component]
pub fn KafkaConsumerForm(mut config: Signal<KafkaHandlerConfig>) -> Element {
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
                    " - This workflow runs when a message is received from Kafka."
                }
            }

            FormField {
                label: "Topic",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "orders",
                    value: "{config.read().topic}",
                    oninput: move |e| {
                        config.write().topic = e.value();
                    }
                }
                FormHint { text: "The Kafka topic to listen to" }
            }
        }
    }
}

#[component]
pub fn KafkaConsumerNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-blue-100",
            icon: "📥",
            title: "Kafka Consumer",
            subtitle: "Starts when a Kafka message arrives",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}

pub use KafkaConsumerNodeCard as KafkaHandlerNodeCard;
