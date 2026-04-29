// ═══════════════════════════════════════════════════════════════════════
// DIOXUS PORT REFERENCE -- Restate DAG Builder
// ═══════════════════════════════════════════════════════════════════════
//
// This file is a compilation reference for porting the TypeScript/React
// flow builder into Dioxus (Rust). It is NOT meant to compile as-is,
// but gives you the exact Rust equivalents for every pattern used.
//
// File mapping:
//   lib/flow-types.ts           -> src/types.rs
//   components/flow/icons.tsx   -> src/components/icons.rs
//   components/flow/flow-node.tsx       -> src/components/flow_node.rs
//   components/flow/flow-edges.tsx      -> src/components/flow_edges.rs
//   components/flow/node-sidebar.tsx    -> src/components/node_sidebar.rs
//   components/flow/flow-toolbar.tsx    -> src/components/flow_toolbar.rs
//   components/flow/node-config-panel.tsx -> src/components/node_config_panel.rs
//   components/flow/flow-canvas.tsx     -> src/components/flow_canvas.rs
//   app/page.tsx                        -> src/main.rs

// ═══════════════════════════════════════════════════════════════════════
// 1. TYPES (src/types.rs)
// ═══════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use dioxus::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum NodeCategory {
    Entry,
    Durable,
    State,
    Flow,
    Timing,
    Signal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ServiceType {
    Service,
    VirtualObject,
    Workflow,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum InvocationStatus {
    Pending,
    Running,
    Suspended,
    Completed,
    Failed,
    Retrying,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct NodeData {
    pub label: String,
    pub description: String,
    pub icon: String,
    pub category: NodeCategory,
    pub configured: bool,
    pub status: Option<InvocationStatus>,
    pub durable_step_name: Option<String>,
    pub retry_count: Option<u32>,
    pub journal_index: Option<u32>,
    pub state_key: Option<String>,
    pub target_service: Option<String>,
    pub target_handler: Option<String>,
    pub promise_name: Option<String>,
    pub sleep_duration: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FlowNode {
    pub id: String,
    pub position: Position,
    pub data: NodeData,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub animated: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub struct NodeTemplate {
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub category: NodeCategory,
}

pub const NODE_WIDTH: f64 = 240.0;
pub const NODE_HEIGHT: f64 = 72.0;

pub const CATEGORY_ORDER: &[NodeCategory] = &[
    NodeCategory::Entry,
    NodeCategory::Durable,
    NodeCategory::State,
    NodeCategory::Flow,
    NodeCategory::Timing,
    NodeCategory::Signal,
];

pub fn category_label(cat: NodeCategory) -> &'static str {
    match cat {
        NodeCategory::Entry => "Entry Points",
        NodeCategory::Durable => "Durable Steps",
        NodeCategory::State => "State",
        NodeCategory::Flow => "Control Flow",
        NodeCategory::Timing => "Timing & Events",
        NodeCategory::Signal => "Signals & Promises",
    }
}

pub fn node_templates() -> Vec<NodeTemplate> {
    vec![
        NodeTemplate { node_type: "http-handler".into(), label: "HTTP Handler".into(), description: "Handle HTTP/gRPC invocation".into(), icon: "globe".into(), category: NodeCategory::Entry },
        NodeTemplate { node_type: "kafka-handler".into(), label: "Kafka Consumer".into(), description: "Consume events from Kafka topic".into(), icon: "kafka".into(), category: NodeCategory::Entry },
        // ... continue for all 24 templates
    ]
}

// ═══════════════════════════════════════════════════════════════════════
// 2. ICON RENDERING (src/components/icons.rs)
// ═══════════════════════════════════════════════════════════════════════

// Single match-based icon renderer instead of a HashMap of components:

pub fn render_icon(name: &str, class: &str) -> Element {
    match name {
        "globe" => rsx! {
            svg {
                xmlns: "http://www.w3.org/2000/svg",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                class: "{class}",
                circle { cx: "12", cy: "12", r: "10" }
                path { d: "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" }
                path { d: "M2 12h20" }
            }
        },
        "shield" => rsx! {
            svg {
                xmlns: "http://www.w3.org/2000/svg",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                class: "{class}",
                path { d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" }
                path { d: "m9 12 2 2 4-4" }
            }
        },
        // ... all other icons follow the same pattern
        _ => rsx! {
            svg {
                class: "{class}",
                // fallback empty icon
            }
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 3. PATTERN MAPPING CHEATSHEET
// ═══════════════════════════════════════════════════════════════════════
//
// ┌─────────────────────────────────┬──────────────────────────────────────────────┐
// │ TypeScript / React              │ Rust / Dioxus                                │
// ├─────────────────────────────────┼──────────────────────────────────────────────┤
// │ useState(value)                 │ use_signal(|| value)                         │
// │ useRef(value)                   │ use_signal(|| value) // non-rendering ref    │
// │ useCallback(fn, [deps])         │ move |args| { ... } // closures auto-capture│
// │ useEffect(() => {...}, [])      │ use_effect(move || { ... })                  │
// │ e.stopPropagation()             │ e.stop_propagation()                         │
// │ e.preventDefault()              │ e.prevent_default()                          │
// │ e.clientX                       │ e.client_coordinates().x                     │
// │ e.target                        │ e.data().target()                            │
// │ ref.current?.getBoundingRect()  │ mounted.get_client_rect().await              │
// │ className="..."                 │ class: "..."                                 │
// │ style={{ prop: val }}           │ style: "prop: val;"                          │
// │ onClick={() => fn(x)}           │ onclick: move |_| fn(x)                     │
// │ onChange={(e) => set(e.target.value)} │ oninput: move |e| sig.set(e.value())   │
// │ onMouseDown                     │ onmousedown                                  │
// │ onMouseMove                     │ onmousemove                                  │
// │ onMouseUp                       │ onmouseup                                    │
// │ onDragOver                      │ ondragover                                   │
// │ onDrop                          │ ondrop                                       │
// │ {condition && <Elem />}         │ if condition { rsx! { Elem {} } }            │
// │ {arr.map(x => <Comp key={x.id} />)} │ for x in arr { Comp { key: "{x.id}" } }│
// │ ?.                              │ .as_ref().map(|x| ...) or if let Some(x)     │
// │ JSON.parse / JSON.stringify     │ serde_json::from_str / serde_json::to_string │
// │ `template literal ${var}`       │ format!("... {var}")                         │
// │ .join(" ")                      │ format!("{} {}", a, b) or [a, b].join(" ")   │
// │ .filter() / .map()             │ .iter().filter().map().collect()              │
// │ .find()                         │ .iter().find(|x| ...)                        │
// │ .some()                         │ .iter().any(|x| ...)                         │
// │ .reduce()                       │ .iter().fold(init, |acc, x| ...)             │
// │ Math.min(...arr)                │ arr.iter().copied().reduce(f64::min)         │
// │ Math.max(...arr)                │ arr.iter().copied().reduce(f64::max)         │
// │ Math.round(x)                   │ x.round()                                   │
// │ Math.abs(x)                     │ x.abs()                                     │
// │ let nextId = 20 (module mut)    │ static NEXT_ID: AtomicU32 = ...             │
// │ setInterval(fn, ms)             │ spawn_local(async { loop { sleep(ms).await } │
// │ setTimeout(fn, ms)              │ spawn_local(async { sleep(ms).await; fn() }) │
// │ clearInterval(id)               │ Drop the spawned task / use CancellationToken│
// │ Record<string, string>          │ HashMap<String, &'static str> or match       │
// │ type X = "a" | "b"             │ enum X { A, B }                              │
// │ interface Props { ... }         │ #[derive(Props, Clone, PartialEq)]           │
// │ export function Comp(props)     │ #[component] fn Comp(...) -> Element         │
// └─────────────────────────────────┴──────────────────────────────────────────────┘
//
// ═══════════════════════════════════════════════════════════════════════
// 4. WHEEL EVENT (needs non-passive listener)
// ═══════════════════════════════════════════════════════════════════════
//
// Dioxus doesn't have a native non-passive wheel handler. Use web_sys:
//
//   use_effect(move || {
//       let el = canvas_ref.read().as_ref().unwrap().downcast::<web_sys::HtmlElement>().unwrap();
//       let closure = Closure::wrap(Box::new(move |e: web_sys::WheelEvent| {
//           e.prevent_default();
//           let delta_y = e.delta_y();
//           let factor = if delta_y > 0.0 { 0.92 } else { 1.08 };
//           // ... zoom logic using signals
//       }) as Box<dyn FnMut(_)>);
//       let opts = web_sys::AddEventListenerOptions::new();
//       opts.set_passive(false);
//       el.add_event_listener_with_callback_and_add_event_listener_options(
//           "wheel", closure.as_ref().unchecked_ref(), &opts
//       ).unwrap();
//       closure.forget(); // or store for cleanup
//   });
//
// ═══════════════════════════════════════════════════════════════════════
// 5. DRAG & DROP (use shared signal instead of dataTransfer)
// ═══════════════════════════════════════════════════════════════════════
//
//   // In a parent or context:
//   let dragging_template: Signal<Option<NodeTemplate>> = use_signal(|| None);
//
//   // In sidebar item:
//   onmousedown: move |_| {
//       dragging_template.set(Some(template.clone()));
//   }
//
//   // In canvas:
//   ondrop: move |e: DragEvent| {
//       e.prevent_default();
//       if let Some(template) = dragging_template.take() {
//           // Create node at drop position
//       }
//   }
//
// ═══════════════════════════════════════════════════════════════════════
// 6. EXECUTION SIMULATION
// ═══════════════════════════════════════════════════════════════════════
//
//   fn start_execution(nodes: Signal<Vec<FlowNode>>, edges: Signal<Vec<FlowEdge>>) {
//       spawn_local(async move {
//           let order = vec!["1", "2", "3", "4", "6"];
//           for (i, id) in order.iter().enumerate() {
//               // Mark current as running, previous as completed
//               nodes.write().iter_mut().for_each(|n| {
//                   if n.id == *id { n.data.status = Some(InvocationStatus::Running); }
//                   if i > 0 && n.id == order[i - 1] { n.data.status = Some(InvocationStatus::Completed); }
//               });
//               gloo_timers::future::sleep(Duration::from_millis(800)).await;
//           }
//           // Final: mark last as suspended
//       });
//   }
//
// ═══════════════════════════════════════════════════════════════════════
// 7. TAILWIND CSS SETUP FOR DIOXUS
// ═══════════════════════════════════════════════════════════════════════
//
// In your Dioxus project's Dioxus.toml:
//   [web.resource]
//   style = ["tailwind.css"]
//
// Use a build script or `npx tailwindcss` to compile.
// Copy the globals.css :root variables into your base CSS.
// The custom --node-trigger, --node-action etc. tokens work as-is.
// The @theme inline {} block is Tailwind v4 syntax -- use it or convert
// to tailwind.config.js for v3.
//
// ═══════════════════════════════════════════════════════════════════════
// 8. SMOOTH STEP PATH (pure math, direct port)
// ═══════════════════════════════════════════════════════════════════════

fn create_smooth_step_path(from: Position, to: Position) -> String {
    let dx = to.x - from.x;
    let mid_y = (from.y + to.y) / 2.0;
    let radius = 8.0;

    if dx.abs() < 2.0 {
        return format!("M {} {} L {} {}", from.x, from.y, to.x, to.y);
    }

    let sign_x: f64 = if dx > 0.0 { 1.0 } else { -1.0 };
    let r = radius.min(dx.abs() / 2.0).min((to.y - from.y).abs() / 4.0);

    format!(
        "M {} {} L {} {} Q {} {} {} {} L {} {} Q {} {} {} {} L {} {}",
        from.x, from.y,
        from.x, mid_y - r,
        from.x, mid_y, from.x + sign_x * r, mid_y,
        to.x - sign_x * r, mid_y,
        to.x, mid_y, to.x, mid_y + r,
        to.x, to.y,
    )
}

// ═══════════════════════════════════════════════════════════════════════
// 9. MAIN ENTRY POINT (src/main.rs)
// ═══════════════════════════════════════════════════════════════════════

fn main() {
    dioxus::launch(App);
}

fn App() -> Element {
    rsx! {
        FlowCanvas {}
    }
}
