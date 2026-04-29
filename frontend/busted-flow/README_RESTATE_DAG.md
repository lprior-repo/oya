# Restate DAG Builder - Comprehensive Visual Workflow Orchestration

A professional, minimalist DAG (Directed Acyclic Graph) tracker built specifically for **Restate** durable execution orchestration. This builder provides a highly interactive, slick visualization for designing, managing, and tracking complex workflows on top of Restate's SDK.

## Architecture Overview

### Pure HTML/Tailwind + Minimal React → Easy Dioxus Port

- **Zero UI library dependencies**: No Radix, shadcn, or lucide-react in flow components
- **Inline SVG icons**: 40+ icons as plain SVG functions (see `components/flow/icons.tsx`)
- **HTML + Tailwind only**: Every component is `<div>`, `<button>`, `<input>`, `<svg>` with Tailwind classes
- **4 React imports total**: Only in files with `useState` (sidebar, toolbar, config panel, canvas)
- **Rust DAG layout algorithm included**: `lib/dag_layout.rs` - Sugiyama layered graph layout

### File Structure

```
lib/
  flow-types.ts          - Type definitions (65 lines, 0 React)
  dag-layout.ts          - TypeScript layout algorithm (56 lines)
  dag_layout.rs          - Rust layout algorithm (318 lines) - PORT THIS TO DIOXUS
components/flow/
  icons.tsx              - Inline SVG icon library (68 lines, 0 React imports)
  flow-node.tsx          - Node rendering (80 lines, 0 React imports)
  flow-edges.tsx         - Edge/connection rendering (55 lines, 0 React imports)
  node-sidebar.tsx       - Left sidebar with node templates (80 lines, 1 React import)
  flow-toolbar.tsx       - Top toolbar with controls (103 lines, 1 React import)
  node-config-panel.tsx  - Right config panel (279 lines, 1 React import)
  flow-canvas.tsx        - Main orchestrator (239 lines, 1 React import)
```

## Restate Integration - Full SDK Coverage

### 24 Node Types Mapped to Restate Primitives

#### Entry Points (4 nodes)
- **HTTP Handler**: `@handler` for HTTP/gRPC invocations
- **Kafka Consumer**: Event-driven triggers from Kafka topics
- **Cron Trigger**: Scheduled periodic execution
- **Workflow Submit**: Submit workflow with unique key

#### Durable Steps (6 nodes)
- **Durable Step**: `ctx.run("name", () => ...)` - Persisted side effect
- **Service Call**: `ctx.serviceClient<T>("name").method(req)` - Request-response
- **Object Call**: `ctx.objectClient<T>("key").method(req)` - Virtual object handler
- **Workflow Call**: `ctx.workflowClient<T>("key").submit(req)` - Workflow invocation
- **Send Message**: `ctx.objectSendClient<T>("key").method(req)` - Fire-and-forget
- **Delayed Message**: Scheduled future handler call with delay

#### State Management (3 nodes)
- **Get State**: `await ctx.get<T>("key")` - Read persisted state
- **Set State**: `ctx.set("key", value)` - Write persisted state
- **Clear State**: `ctx.clear("key")` / `ctx.clearAll()` - Delete state

#### Control Flow (5 nodes)
- **If / Else**: Conditional branching
- **Switch**: Multi-path routing
- **Loop / Iterate**: `for (const x of items)` iteration
- **Parallel**: `Promise.all([...])` concurrent execution
- **Compensate**: Saga compensation / rollback logic

#### Timing & Events (2 nodes)
- **Sleep / Timer**: `await ctx.sleep(duration)` - Durable pause
- **Timeout**: `promise.orTimeout(ms)` - Deadline guard

#### Signals & Promises (4 nodes)
- **Durable Promise**: `await ctx.promise<T>("name")` - Await external event
- **Awakeable**: `ctx.awakeable<T>()` - Pause for external completion
- **Resolve Promise**: `ctx.promiseManager().resolve("name", val)` - Resolve externally
- **Signal Handler**: Shared handler for signal resolution

### Configuration Fields Per Node Type

Each node type exposes relevant Restate SDK fields in the config panel:

**Entry Points:**
- Cron Expression for scheduled workflows
- Kafka topic for event consumption
- Workflow key for idempotent workflow submission

**Durable Steps:**
- Step name for `ctx.run("name", ...)`
- Target service name for SDK clients
- Handler/method name for invocation
- Delay duration for delayed sends
- Idempotency keys for exactly-once semantics

**State:**
- State key for get/set/clear operations
- Visual code examples: `ctx.get("cart")`

**Control Flow:**
- Condition expressions for if/else
- Loop iterators for collections
- Compensation handlers for saga rollback

**Timing:**
- Sleep duration strings: "5m", "1h", "30s"
- Timeout milliseconds for deadline guards

**Signals:**
- Promise names for durable promises
- Awakeable IDs for external resolution
- Visual code examples showing SDK usage

### Execution Tracking

- **Journal Index**: Position in Restate's durable journal
- **Invocation Status**: `pending | running | suspended | completed | failed | retrying`
- **Retry Count**: Number of automatic retry attempts
- **Status Badges**: Color-coded with animated indicators
- **Execution Simulation**: Click "Invoke" to see workflow execution with animated edges

### Visual Features

- **Node Status Colors**:
  - Running: Blue pulse animation
  - Completed: Green
  - Suspended: Pink (awaiting promise)
  - Failed: Red
  - Retrying: Yellow with spin

- **Edge Animations**: Animated dashed lines showing active execution flow
- **Minimap**: Top-right overview of entire DAG
- **Dot Grid Background**: Professional canvas appearance
- **Service Type Badge**: Workflow / Virtual Object / Service indicator

## DAG Layout Algorithm

### Sugiyama Layered Graph Layout (`lib/dag_layout.rs`)

The Rust implementation provides automatic hierarchical layout:

1. **Layer Assignment**: Topological sort with longest-path layering
2. **Crossing Minimization**: Barycenter heuristic (4 passes)
3. **Position Assignment**: Layer-based coordinate calculation
4. **Configurable Spacing**: Node spacing (60px) and layer spacing (140px)

**Usage in Dioxus:**
```rust
use dag_layout::{LayeredLayout, Node, Edge, Position};

let layout = LayeredLayout::default();
let positions = layout.layout(&nodes, &edges);

// Apply positions to your node state
for (id, pos) in positions {
    // Update node at `id` to position (pos.x, pos.y)
}
```

### Force-Directed Layout (Optional)

Also included: simpler spring-based physics layout with:
- Repulsion between all nodes
- Spring attraction along edges
- 50 iterations with damping

## Canvas Interactions

### Mouse Controls
- **Pan**: Click-drag on empty canvas
- **Zoom**: Mouse wheel (0.15x - 3x range)
- **Select Node**: Click on node
- **Drag Node**: Click-drag node handle
- **Connect Nodes**: Click-drag from node handle → target node

### Toolbar Controls
- **Zoom In/Out**: +/- buttons
- **Fit View**: Auto-center and zoom to fit all nodes
- **Auto Layout DAG**: Run Sugiyama algorithm to reorganize nodes
- **Invoke**: Start execution simulation with animated status updates
- **Service Type Picker**: Switch between Workflow / Virtual Object / Service

### Keyboard Shortcuts (Future)
- `Cmd/Ctrl + Z`: Undo
- `Cmd/Ctrl + Y`: Redo
- `Delete`: Remove selected node
- `Cmd/Ctrl + D`: Duplicate selected node

## Dioxus Port Guide

### 1. Type System

All TypeScript types in `lib/flow-types.ts` map directly to Rust structs:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum NodeCategory {
    Entry, Durable, State, Flow, Timing, Signal
}

#[derive(Clone, Debug, PartialEq)]
pub struct Position { pub x: f64, pub y: f64 }

#[derive(Clone, Debug)]
pub struct NodeData {
    pub label: String,
    pub description: String,
    pub icon: String,
    pub category: NodeCategory,
    pub configured: bool,
    pub status: Option<InvocationStatus>,
    // ... rest of fields
}

#[derive(Clone, Debug)]
pub struct FlowNode {
    pub id: String,
    pub position: Position,
    pub data: NodeData,
}
```

### 2. Icons

Convert `components/flow/icons.tsx` to Dioxus:

```rust
// icons.tsx: Icon name → inline SVG
pub fn render_icon(name: &str, class: &str) -> Element {
    match name {
        "shield" => rsx! {
            svg { class: "{class}", xmlns: "http://www.w3.org/2000/svg", 
                  view_box: "0 0 24 24", fill: "none", 
                  stroke: "currentColor", stroke_width: "2",
                path { d: "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" }
            }
        },
        "globe" => rsx! { /* ... */ },
        // ... 40+ more icons
        _ => rsx! { span { "?" } }
    }
}
```

### 3. Component Conversion

**FlowNode** (`flow-node.tsx` → Dioxus component):

```rust
#[component]
fn FlowNode(
    node: FlowNode,
    selected: bool,
    scale: f64,
    on_mouse_down: EventHandler<(MouseEvent, String)>,
    on_click: EventHandler<String>,
    on_handle_mouse_down: EventHandler<(MouseEvent, String, String)>,
) -> Element {
    let border = match node.data.category {
        NodeCategory::Entry => "border-node-trigger/40",
        NodeCategory::Durable => "border-node-action/40",
        // ... rest
    };
    
    let selected_ring = if selected { "ring-2 ring-primary" } else { "" };
    
    rsx! {
        div {
            class: "absolute cursor-move {border} {selected_ring}",
            style: "left: {node.position.x}px; top: {node.position.y}px; width: 240px;",
            data_node_id: "{node.id}",
            onmousedown: move |e| on_mouse_down.call((e, node.id.clone())),
            onclick: move |_| on_click.call(node.id.clone()),
            
            div { class: "flex items-center gap-2 p-3",
                // Icon, label, status badge...
            }
            
            // Source handle (bottom)
            div {
                class: "absolute left-1/2 -translate-x-1/2 -bottom-1 w-3 h-3 rounded-full",
                onmousedown: move |e| {
                    e.stop_propagation();
                    on_handle_mouse_down.call((e, node.id.clone(), "source".into()));
                }
            }
        }
    }
}
```

### 4. State Management

Replace React hooks with Dioxus signals:

```rust
// useState → use_signal
let mut nodes = use_signal(|| vec![/* initial nodes */]);
let mut edges = use_signal(|| vec![/* initial edges */]);
let mut selected_node_id = use_signal(|| Option::<String>::None);
let mut zoom = use_signal(|| 0.85f64);
let mut pan = use_signal(|| Position { x: 0.0, y: 0.0 });

// useRef → use_signal (for non-reactive mutable state)
let mut is_panning = use_signal(|| false);
let mut drag_node_id = use_signal(|| Option::<String>::None);
```

### 5. Event Handlers

Mouse events map directly:

```rust
// Pan canvas
let on_canvas_mouse_down = move |e: MouseEvent| {
    if e.button() != 0 { return; }
    selected_node_id.set(None);
    is_panning.set(true);
    // Store start position...
};

// Wheel zoom (requires web_sys for passive: false)
use_effect(move || {
    let canvas_ref = /* ... */;
    let handler = EventListener::new(&canvas_ref, "wheel", move |e: &web_sys::WheelEvent| {
        e.prevent_default();
        let delta = e.delta_y();
        let zoom_factor = if delta > 0.0 { 0.92 } else { 1.08 };
        // Update zoom and pan...
    });
    move || drop(handler)
});
```

### 6. Drag & Drop

**Recommended**: Use shared signal instead of DataTransfer:

```rust
// Shared state for dragging
let dragging_template = use_context::<Signal<Option<NodeTemplate>>>();

// In sidebar - start drag
ondragstart: move |_| {
    dragging_template.set(Some(template.clone()));
},

// In canvas - handle drop
ondrop: move |e: DragEvent| {
    e.prevent_default();
    if let Some(template) = dragging_template() {
        let pos = to_canvas_coords(e.client_x(), e.client_y());
        // Create node at position...
        dragging_template.set(None);
    }
}
```

### 7. Execution Simulation

Replace `setInterval` with `spawn_local` + `gloo_timers`:

```rust
use gloo_timers::future::TimeoutFuture;

let toggle_execution = move |_| {
    if *is_executing.read() {
        is_executing.set(false);
        return;
    }
    
    is_executing.set(true);
    spawn_local(async move {
        let order = vec!["1", "2", "3", "4", "6"];
        
        for (i, node_id) in order.iter().enumerate() {
            TimeoutFuture::new(800).await;
            
            // Update node status to "running"
            nodes.write().iter_mut()
                .find(|n| n.id == *node_id)
                .map(|n| n.data.status = Some(InvocationStatus::Running));
            
            // Update edges to animate
            edges.write().iter_mut()
                .filter(|e| e.target == *node_id)
                .for_each(|e| e.animated = true);
        }
        
        TimeoutFuture::new(800).await;
        // Final suspended state...
        is_executing.set(false);
    });
};
```

## Performance Considerations

### Current Implementation
- ~850 total lines of code
- ~25 React-specific lines (hooks + imports)
- Zero external UI libraries
- Pure CSS transforms for pan/zoom (GPU-accelerated)
- Inline styles for dynamic positioning (no style recalculation)

### Dioxus Optimizations
- Use `memo` for expensive layout calculations
- Batch node position updates in single write
- Use `use_resource` for async layout computation
- Consider virtual scrolling for 500+ nodes
- WebAssembly SIMD for layout algorithm

## Future Enhancements

### Restate SDK Deep Integration
- [ ] Live journal inspection via Restate Admin API
- [ ] Real-time execution status from Restate runtime
- [ ] Deploy workflows directly to Restate endpoint
- [ ] Import existing workflows from Restate introspection
- [ ] Replay journal entries with step-through debugger

### Advanced Layout
- [ ] Hierarchical edge bundling for complex graphs
- [ ] Orthogonal edge routing (fewer crossings)
- [ ] Circular layout for cyclic workflows
- [ ] Collapsible subgraphs / nested workflows

### Collaboration
- [ ] Multi-user real-time editing (CRDT-based)
- [ ] Version control / Git integration
- [ ] Workflow templates library
- [ ] Export to Restate TypeScript/Rust SDK code

### Visual Enhancements
- [ ] Dark/light theme toggle
- [ ] Custom color schemes per service type
- [ ] Animated transitions for layout changes
- [ ] 3D visualization mode (Three.js/Bevy)

## License

MIT - Built for Restate community

## Credits

Built with ❤️ for visual workflow orchestration on Restate's durable execution platform.
