#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use crate::graph::{Node, NodeCategory};
use dioxus::prelude::*;
use serde_json::Value;

mod common;
mod config_sections;
mod execution;

use common::CommonConfig;
use config_sections::{
    DurableConfig, EntryConfig, FlowConfig, SignalConfig, StateConfig, TimingConfig,
};
use execution::ExecutionTab;

const INPUT_CLASS: &str =
    "h-8 w-full rounded-md border border-slate-700 bg-slate-950 px-3 font-mono text-[12px] text-slate-100 outline-none transition-colors focus:border-indigo-500/50 focus:ring-1 focus:ring-indigo-500/30";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Config,
    Execution,
}

#[component]
pub fn NodeConfigEditor(
    node: Node,
    input_payloads: Vec<Value>,
    on_change: EventHandler<Value>,
) -> Element {
    let mut tab = use_signal(|| Tab::Config);
    let config = node.config.clone();

    let tab_val = *tab.read();

    let config_tab_class = if tab_val == Tab::Config {
        "flex-1 border-b-2 border-indigo-500 py-2 text-[11px] font-medium capitalize text-slate-100 transition-colors"
    } else {
        "flex-1 border-b-2 border-transparent py-2 text-[11px] font-medium capitalize text-slate-500 transition-colors hover:text-slate-300"
    };

    let exec_tab_class = if tab_val == Tab::Execution {
        "flex-1 border-b-2 border-indigo-500 py-2 text-[11px] font-medium capitalize text-slate-100 transition-colors"
    } else {
        "flex-1 border-b-2 border-transparent py-2 text-[11px] font-medium capitalize text-slate-500 transition-colors hover:text-slate-300"
    };

    rsx! {
        div { class: "flex flex-col gap-0",
            div { class: "flex border-b border-slate-800",
                button {
                    class: "{config_tab_class}",
                    onclick: move |_| tab.set(Tab::Config),
                    "Configuration"
                }
                button {
                    class: "{exec_tab_class}",
                    onclick: move |_| tab.set(Tab::Execution),
                    "Execution"
                }
            }

            div { class: "pt-4",
                match tab_val {
                    Tab::Config => rsx! {
                        ConfigTab { node: node.clone(), on_change: on_change }
                    },
                    Tab::Execution => rsx! {
                        ExecutionTab {
                            config: config.clone(),
                            execution_state: node.execution_state,
                            execution_data: node.execution_data.clone(),
                            last_output: node.last_output.clone(),
                            input_payloads,
                            on_pin_sample: EventHandler::new({
                                let config = config.clone();
                                move |payload: Option<Value>| {
                                    let mut new_config = config.clone();
                                    if let Some(obj) = new_config.as_object_mut() {
                                        match payload {
                                            Some(value) => {
                                                obj.insert("pinnedOutputSample".to_owned(), value);
                                            }
                                            None => {
                                                obj.remove("pinnedOutputSample");
                                            }
                                        }
                                        on_change.call(new_config);
                                    }
                                }
                            })
                        }
                    },
                }
            }
        }
    }
}

use crate::ui::workflow_nodes::schema::WorkflowNode as RichNode;
use crate::ui::workflow_nodes::{
    clear_all, clear_state, compensate, delay, delayed_message, durable_promise, http_trigger,
    kafka_consumer, load_from_memory, loop_iterate, parallel, peek_promise, resolve_promise,
    router, run_code, save_to_memory, schedule_trigger, send_message, service_call, timeout_guard,
    wait_for_signal, wait_for_webhook, workflow_submit,
};

#[component]
fn ConfigTab(node: Node, on_change: EventHandler<Value>) -> Element {
    let config = node.config.clone();

    // Try to parse into rich node for the specialized forms
    let rich_node = serde_json::from_value::<RichNode>(config.clone()).ok();

    let update_str = EventHandler::new({
        let config = config.clone();
        move |(key, val): (String, String)| {
            let mut new_config = config.clone();
            if let Some(obj) = new_config.as_object_mut() {
                obj.insert(key, Value::String(val));
                on_change.call(new_config);
            }
        }
    });

    let update_u64 = EventHandler::new({
        let config = config.clone();
        move |(key, val): (String, u64)| {
            let mut new_config = config.clone();
            if let Some(obj) = new_config.as_object_mut() {
                obj.insert(key, Value::Number(val.into()));
                on_change.call(new_config);
            }
        }
    });

    rsx! {
        div { class: "flex flex-col gap-4",
            if let Some(rich) = rich_node {
                // Use the specialized forms I built
                {
                    // Bridge: Convert Value to Signal for the forms
                    let rich_signal = use_signal(|| rich.clone());

                    // Sync back changes to the main workflow
                    let effect_config = config.clone();
                    use_effect(move || {
                        if let Ok(val) = serde_json::to_value(&*rich_signal.read()) {
                            if val != effect_config {
                                on_change.call(val);
                            }
                        }
                    });

                    let rich_val = rich_signal.read();
                    let x = match &*rich_val {
                        RichNode::HttpHandler(cfg) => rsx! { http_trigger::HttpHandlerForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::KafkaConsumer(cfg) => rsx! { kafka_consumer::KafkaConsumerForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::CronTrigger(cfg) => rsx! { schedule_trigger::CronTriggerForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::Run(cfg) => rsx! { run_code::RunForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::ServiceCall(cfg) => rsx! { service_call::ServiceCallForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::SendMessage(cfg) => rsx! { send_message::SendMessageForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::DelayedSend(cfg) => rsx! { delayed_message::DelayedSendForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::SetState(cfg) => rsx! { save_to_memory::SetStateForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::GetState(cfg) => rsx! { load_from_memory::GetStateForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::Sleep(cfg) => rsx! { delay::SleepForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::Switch(cfg) => rsx! { router::SwitchForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::Awakeable(cfg) => rsx! { wait_for_webhook::AwakeableForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::SignalHandler(cfg) => rsx! { wait_for_signal { config: use_signal(|| cfg.clone()) } },
                        RichNode::ClearState(cfg) => rsx! { clear_state::ClearStateForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::ClearAll(cfg) => rsx! { clear_all::ClearAllForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::Loop(cfg) | RichNode::LoopIterate(cfg) => rsx! { loop_iterate::LoopIterateForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::Parallel(cfg) => rsx! { parallel::ParallelForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::Compensate(cfg) => rsx! { compensate::CompensateForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::Timeout(cfg) | RichNode::TimeoutGuard(cfg) => rsx! { timeout_guard::TimeoutGuardForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::DurablePromise(cfg) => rsx! { durable_promise::DurablePromiseForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::PeekPromise(cfg) => rsx! { peek_promise::PeekPromiseForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::ResolvePromise(cfg) => rsx! { resolve_promise::ResolvePromiseForm { config: use_signal(|| cfg.clone()) } },
                        RichNode::WorkflowSubmit(cfg) => rsx! { workflow_submit::WorkflowSubmitForm { config: use_signal(|| cfg.clone()) } },
                        _ => rsx! {
                            // Fallback to basic editor for other types
                            match node.category {
                                NodeCategory::Entry => rsx! { EntryConfig { node: node.node.clone(), config: config.clone(), update_str, input_cls: INPUT_CLASS } },
                                NodeCategory::Durable => rsx! { DurableConfig { node: node.node.clone(), config: config.clone(), update_str, update_u64, input_cls: INPUT_CLASS } },
                                NodeCategory::State => rsx! { StateConfig { node: node.node.clone(), config: config.clone(), update_str, input_cls: INPUT_CLASS } },
                                NodeCategory::Flow => rsx! { FlowConfig { node: node.node.clone(), config: config.clone(), update_str, input_cls: INPUT_CLASS } },
                                NodeCategory::Timing => rsx! { TimingConfig { node: node.node.clone(), config: config.clone(), update_u64, input_cls: INPUT_CLASS } },
                                NodeCategory::Signal => rsx! { SignalConfig { node: node.node.clone(), config: config.clone(), update_str, input_cls: INPUT_CLASS } },
                            }
                        }
                    };
                    drop(rich_val);
                    x
                }
            } else {
                // Full Fallback to basic editor
                match node.category {
                    NodeCategory::Entry => rsx! { EntryConfig { node: node.node.clone(), config: config.clone(), update_str, input_cls: INPUT_CLASS } },
                    NodeCategory::Durable => rsx! { DurableConfig { node: node.node.clone(), config: config.clone(), update_str, update_u64, input_cls: INPUT_CLASS } },
                    NodeCategory::State => rsx! { StateConfig { node: node.node.clone(), config: config.clone(), update_str, input_cls: INPUT_CLASS } },
                    NodeCategory::Flow => rsx! { FlowConfig { node: node.node.clone(), config: config.clone(), update_str, input_cls: INPUT_CLASS } },
                    NodeCategory::Timing => rsx! { TimingConfig { node: node.node.clone(), config: config.clone(), update_u64, input_cls: INPUT_CLASS } },
                    NodeCategory::Signal => rsx! { SignalConfig { node: node.node.clone(), config: config.clone(), update_str, input_cls: INPUT_CLASS } },
                }
            }

            div { class: "h-px bg-slate-800" }

            CommonConfig { config: config, update_u64: update_u64, update_str: update_str, input_cls: INPUT_CLASS }
        }
    }
}

pub(crate) fn get_str_val(config: &Value, key: &str) -> String {
    config.get(key).and_then(Value::as_str).map_or("", |value| value).to_string()
}

pub(crate) fn get_u64_val(config: &Value, key: &str) -> Option<u64> {
    config.get(key).and_then(Value::as_u64)
}
