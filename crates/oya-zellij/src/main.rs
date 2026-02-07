//! OYA Zellij Plugin - Pipeline Orchestration Dashboard
//!
//! Lints inherited from workspace - no local exceptions allowed.
//!
//! Real-time terminal UI for pipeline status, bead execution, and stage progress.
//!
//! # Functional State Management Architecture
//!
//! This plugin uses a functional state management pattern with exterior mutability
//! to bridge between pure functional state transformations and the Zellij trait's
//! `&mut self` requirement.
//!
//! ## Why Exterior Mut?
//!
//! The `zellij_tile::Plugin` trait requires methods with `&mut self` signatures:
//! ```rust
//! fn update(&mut self, event: Event) -> bool;
//! fn render(&mut self, rows: usize, cols: usize);
//! ```
//!
//! To maintain functional purity internally while satisfying this interface, we use
//! the **exterior mut pattern**:
//!
//! ```rust
//! // Pure functional handler - returns new state
//! fn handle_event(self, event: Event) -> (Self, bool) {
//!     match event {
//!         Event::Timer(_) => self.handle_timer_event(),
//!         // ...
//!     }
//! }
//!
//! // Zellij trait implementation uses exterior mut
//! impl zellij_tile::Plugin for State {
//!     fn update(&mut self, event: Event) -> bool {
//!         let (new_state, should_render) = std::mem::replace(self, State::default())
//!             .handle_event(event);
//!         *self = new_state;
//!         should_render
//!     }
//! }
//! ```
//!
//! ## Benefits
//!
//! - **Explicit state transformations**: Every state change returns a new state
//! - **No hidden mutations**: All transformations are visible in function signatures
//! - **Easy to test**: Pure functions are trivial to unit test
//! - **Structural sharing**: `im::Vector` and `im::HashMap` provide efficient cloning
//!
//! ## Side Effects
//!
//! Note that `web_request()` calls are I/O side effects. While we can't make these
//! pure, we structure the code to separate state transformation from I/O:
//!
//! ```rust
//! fn load_beads(self) -> (Self, Result<()>) {
//!     // Check cache, return early if valid (pure)
//!     if let Some((cached, _)) = &self.beads_cache {
//!         // ...
//!     }
//!
//!     // Perform web request (side effect)
//!     web_request(&url, HttpVerb::Get, ...);
//!
//!     // Return new state (pure transformation)
//!     (self, Ok(()))
//! }
//! ```

mod command_pane;
mod graph;
mod log_stream;
mod ui;

use im::{HashMap, Vector};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use zellij_tile::prelude::*;

// Constants for caching and timeouts
const CACHE_TTL: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const AGENT_EVENT_LIMIT: usize = 50;

// Log streaming backpressure constants
#[allow(dead_code)]
const MAX_LOG_MESSAGES: usize = 1000; // Maximum messages in buffer (backpressure limit)
#[allow(dead_code)]
const LOG_EVENT_NAME: &str = "log"; // Custom message name for log streaming

// Context keys for identifying web request responses
const CTX_REQUEST_TYPE: &str = "request_type";
const CTX_BEADS_LIST: &str = "beads_list";
const CTX_PIPELINE: &str = "pipeline";
const CTX_BEAD_ID: &str = "bead_id";
const CTX_AGENTS_LIST: &str = "agents_list";
const CTX_GRAPH: &str = "graph";

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct GraphNode {
    id: String,
    label: String,
    is_on_critical_path: bool,
    state: NodeState,
}

#[derive(Clone, Debug)]
struct GraphEdge {
    from: String,
    to: String,
    is_on_critical_path: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeState {
    Idle,
    Running,
    Blocked,
    Completed,
    Failed,
}

impl NodeState {
    fn as_str(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    fn color(&self) -> &str {
        match self {
            Self::Idle => "\x1b[90m",
            Self::Running => "\x1b[33m",
            Self::Blocked => "\x1b[31m",
            Self::Completed => "\x1b[32m",
            Self::Failed => "\x1b[31m",
        }
    }

    fn symbol(&self) -> &str {
        match self {
            Self::Idle => "○",
            Self::Running => "◐",
            Self::Blocked => "⊘",
            Self::Completed => "●",
            Self::Failed => "✗",
        }
    }
}

// Plugin state
#[derive(Clone)]
struct State {
    // Current view mode
    mode: ViewMode,

    // API connection
    server_url: String,
    api_connected: bool,
    last_error: Option<String>,
    pending_requests: u8,

    // Cache with TTL (Using im types for structural sharing)
    beads_cache: Option<(Vector<BeadInfo>, Instant)>,
    agents_cache: Option<(Vector<AgentInfo>, Instant)>,
    pipeline_caches: HashMap<String, (Vector<StageInfo>, Instant)>,
    #[allow(clippy::type_complexity)]
    graph_cache: Option<(
        Vector<GraphNode>,
        Vector<GraphEdge>,
        Vector<String>,
        Instant,
    )>,

    // Tracking for timeouts
    last_request_sent: Option<Instant>,

    // Bead data
    beads: Vector<BeadInfo>,
    selected_index: usize,

    // Pipeline data for selected bead
    pipeline_stages: Vector<StageInfo>,
    selected_stage_index: usize,

    // Agent data
    agents: Vector<AgentInfo>,
    agent_events: VecDeque<AgentEvent>,

    // Graph data
    graph_nodes: Vector<GraphNode>,
    graph_edges: Vector<GraphEdge>,
    critical_path: Vector<String>,

    // Command pane tracking
    command_panes: HashMap<String, command_pane::CommandPane>,

    // Log streaming with backpressure
    #[allow(dead_code)]
    log_buffer: log_stream::LogBuffer,
}

#[allow(clippy::derivable_impls)]
impl Default for State {
    fn default() -> Self {
        Self {
            mode: ViewMode::default(),
            server_url: String::new(),
            api_connected: false,
            last_error: None,
            pending_requests: 0,
            beads_cache: None,
            agents_cache: None,
            pipeline_caches: HashMap::new(),
            graph_cache: None,
            last_request_sent: None,
            beads: Vector::new(),
            selected_index: 0,
            pipeline_stages: Vector::new(),
            selected_stage_index: 0,
            agents: Vector::new(),
            agent_events: VecDeque::new(),
            graph_nodes: Vector::new(),
            graph_edges: Vector::new(),
            critical_path: Vector::new(),
            command_panes: HashMap::new(),
            log_buffer: log_stream::LogBuffer::new(),
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum ViewMode {
    #[default]
    BeadList,
    BeadDetail,
    PipelineView,
    AgentView,
    GraphView,
    SystemHealth,
    LogAggregator,
}

#[derive(Clone, Debug)]
struct BeadInfo {
    id: String,
    title: String,
    status: BeadStatus,
    current_stage: Option<String>,
    progress: f32, // 0.0 to 1.0
    history: Vector<ui::bead_detail::HistoryEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeadStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl BeadStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Self::Pending => "\x1b[90m",    // gray
            Self::InProgress => "\x1b[33m", // yellow
            Self::Completed => "\x1b[32m",  // green
            Self::Failed => "\x1b[31m",     // red
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Self::Pending => "○",
            Self::InProgress => "◐",
            Self::Completed => "●",
            Self::Failed => "✗",
        }
    }
}

#[derive(Clone, Debug)]
struct StageInfo {
    name: String,
    status: StageStatus,
    duration_ms: Option<u64>,
    exit_code: Option<i32>,
}

#[derive(Clone, Copy, Debug)]
enum StageStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct AgentInfo {
    id: String,
    state: AgentState,
    current_bead: Option<String>,
    health_score: f64,
    uptime_secs: u64,
    capabilities: Vector<String>,
    workload_history: WorkloadHistory,
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
struct WorkloadHistory {
    beads_completed: u64,
    operations_executed: u64,
    avg_execution_secs: Option<f64>,
}

#[derive(Clone, Debug)]
struct AgentEvent {
    message: String,
    level: EventLevel,
    occurred_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventLevel {
    Info,
    Warning,
    Error,
}

impl EventLevel {
    fn color(&self) -> &str {
        match self {
            Self::Info => "\x1b[36m",
            Self::Warning => "\x1b[33m",
            Self::Error => "\x1b[31m",
        }
    }

    fn symbol(&self) -> &str {
        match self {
            Self::Info => "i",
            Self::Warning => "!",
            Self::Error => "x",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HealthBand {
    Healthy,
    Warning,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AgentState {
    Idle,
    Working,
    Unhealthy,
    ShuttingDown,
    Terminated,
}

impl AgentState {
    #[allow(dead_code)]
    fn as_str(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Unhealthy => "unhealthy",
            Self::ShuttingDown => "shutting_down",
            Self::Terminated => "terminated",
        }
    }

    #[allow(dead_code)]
    fn color(&self) -> &str {
        match self {
            Self::Idle => "\x1b[36m",
            Self::Working => "\x1b[32m",
            Self::Unhealthy => "\x1b[31m",
            Self::ShuttingDown => "\x1b[33m",
            Self::Terminated => "\x1b[90m",
        }
    }

    #[allow(dead_code)]
    fn symbol(&self) -> &str {
        match self {
            Self::Idle => "○",
            Self::Working => "●",
            Self::Unhealthy => "✗",
            Self::ShuttingDown => "◌",
            Self::Terminated => "⊘",
        }
    }
}

impl StageStatus {
    fn symbol(&self) -> &str {
        match self {
            Self::Pending => "○",
            Self::Running => "◐",
            Self::Passed => "●",
            Self::Failed => "✗",
            Self::Skipped => "⊘",
        }
    }

    fn color(&self) -> &str {
        match self {
            Self::Pending => "\x1b[90m",
            Self::Running => "\x1b[33m",
            Self::Passed => "\x1b[32m",
            Self::Failed => "\x1b[31m",
            Self::Skipped => "\x1b[90m",
        }
    }
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // Get server URL from config
        let server_url = configuration
            .get("server_url")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "http://localhost:3000".to_string());

        // Create new state with configuration loaded
        let new_state = self.clone().with_config(server_url);
        *self = new_state;

        // Request permissions (WebAccess required for HTTP calls)
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::WebAccess,
        ]);

        // Subscribe to events (WebRequestResult for HTTP responses, command pane events)
        subscribe(&[
            EventType::Key,
            EventType::Timer,
            EventType::WebRequestResult,
            EventType::CommandPaneOpened,
            EventType::CommandPaneExited,
            EventType::CommandPaneReRun,
        ]);

        // Set timer for auto-refresh (every 2 seconds)
        set_timeout(2.0);

        // Initial data load will happen after permission is granted
    }

    fn update(&mut self, event: Event) -> bool {
        let (new_state, should_render) = std::mem::take(self).handle_event(event);
        *self = new_state;
        should_render
    }

    fn render(&mut self, rows: usize, cols: usize) {
        print!("\x1b[2J\x1b[H");
        self.render_header(cols);
        let content_rows = rows.saturating_sub(4);
        match self.mode {
            ViewMode::BeadList => self.render_bead_list(content_rows, cols),
            ViewMode::BeadDetail => self.render_bead_detail(content_rows, cols),
            ViewMode::PipelineView => self.render_pipeline_view(content_rows, cols),
            ViewMode::AgentView => self.render_agent_list(content_rows, cols),
            ViewMode::GraphView => self.render_graph_view(content_rows, cols),
            ViewMode::SystemHealth => self.render_system_health(content_rows, cols),
            ViewMode::LogAggregator => self.render_log_aggregator(content_rows, cols),
        }
        self.render_footer(rows, cols);
    }
}
impl State {
    // Functional constructor for loading config
    fn with_config(mut self, server_url: String) -> Self {
        self.server_url = server_url;
        self
    }

    // Functional event handler - returns new state
    fn handle_event(self, event: Event) -> (Self, bool) {
        match event {
            Event::Key(key_with_mod) => self.handle_key_event(key_with_mod),
            Event::Timer(_) => self.handle_timer_event(),
            Event::PermissionRequestResult(_) => self.handle_permission_result(),
            Event::WebRequestResult(status, headers, body, context) => {
                self.handle_web_response(status, headers, body, context)
            }
            Event::CommandPaneOpened(pane_id, context) => {
                self.handle_command_pane_opened(pane_id, context)
            }
            Event::CommandPaneExited(pane_id, exit_code, context) => {
                self.handle_command_pane_exited(pane_id, exit_code, context)
            }
            Event::CommandPaneReRun(pane_id, context) => {
                self.handle_command_pane_rerun(pane_id, context)
            }
            _ => (self, false),
        }
    }

    fn handle_key_event(self, key_with_mod: KeyWithModifier) -> (Self, bool) {
        // Handle Ctrl-d (page down) and Ctrl-u (page up) first
        if key_with_mod.key_modifiers.contains(&KeyModifier::Ctrl) {
            const PAGE_SIZE: usize = 20;

            return match key_with_mod.bare_key {
                BareKey::Char('d') => {
                    let beads_len = self.beads.len();
                    let new_idx = self
                        .selected_index
                        .saturating_add(PAGE_SIZE)
                        .min(beads_len.saturating_sub(1));
                    let new_state = self.with_selected_index(new_idx);
                    let new_state = if new_state.mode == ViewMode::PipelineView {
                        new_state.trigger_pipeline_load()
                    } else {
                        new_state
                    };
                    (new_state, true)
                }
                BareKey::Char('u') => {
                    let new_idx = self.selected_index.saturating_sub(PAGE_SIZE);
                    let new_state = self.with_selected_index(new_idx);
                    let new_state = if new_state.mode == ViewMode::PipelineView {
                        new_state.trigger_pipeline_load()
                    } else {
                        new_state
                    };
                    (new_state, true)
                }
                _ => (self, false),
            };
        }

        // Regular key handling
        match key_with_mod.bare_key {
            BareKey::Char('q') | BareKey::Esc => {
                close_focus();
                (self, false)
            }
            BareKey::Char('j') | BareKey::Down => self.handle_nav_down(),
            BareKey::Char('k') | BareKey::Up => self.handle_nav_up(),
            BareKey::Char('g') => (self.with_selected_index(0), true),
            BareKey::Char('G') => {
                let beads_len = self.beads.len();
                (self.with_selected_index(beads_len.saturating_sub(1)), true)
            }
            BareKey::Char('1') => (self.with_mode(ViewMode::BeadList), true),
            BareKey::Char('2') => (self.with_mode(ViewMode::BeadDetail), true),
            BareKey::Char('3') => self.switch_to_pipeline_view(),
            BareKey::Char('4') => self.switch_to_agent_view(),
            BareKey::Char('5') => self.switch_to_graph_view(),
            BareKey::Char('6') => self.switch_to_system_health_view(),
            BareKey::Char('7') => self.switch_to_log_aggregator_view(),
            BareKey::Enter => self.handle_enter_key(),
            BareKey::Char('r') => self.handle_refresh(),
            _ => (self, false),
        }
    }

    fn handle_timer_event(mut self) -> (Self, bool) {
        // Check for network timeouts
        let has_timeout = self.pending_requests > 0
            && self
                .last_request_sent
                .is_some_and(|last| last.elapsed() > REQUEST_TIMEOUT);

        if has_timeout {
            self.api_connected = false;
            self.last_error = Some("Network timeout".to_string());
            self.pending_requests = 0;
            self.last_request_sent = None;
        }

        // Trigger data loads - each returns updated state
        self = self.trigger_beads_load();

        if self.mode == ViewMode::AgentView {
            self = self.trigger_agents_load();
        }
        if self.mode == ViewMode::GraphView {
            self = self.trigger_graph_load();
        }
        if self.mode == ViewMode::SystemHealth {
            self = self.trigger_system_health_load();
        }
        if self.mode == ViewMode::LogAggregator {
            self = self.trigger_log_aggregator_load();
        }

        set_timeout(2.0);
        (self, true)
    }

    fn handle_permission_result(mut self) -> (Self, bool) {
        self = self.trigger_beads_load();

        if should_fetch_agents_on_view_load(self.mode) {
            self = self.trigger_agents_load();
        }
        if should_fetch_graph_on_view_load(self.mode) {
            self = self.trigger_graph_load();
        }
        if should_fetch_system_health_on_view_load(self.mode) {
            self = self.trigger_system_health_load();
        }
        if should_fetch_log_aggregator_on_view_load(self.mode) {
            self = self.trigger_log_aggregator_load();
        }

        (self, true)
    }

    fn handle_nav_down(mut self) -> (Self, bool) {
        if self.mode == ViewMode::PipelineView {
            // Navigate pipeline stages
            if self.selected_stage_index < self.pipeline_stages.len().saturating_sub(1) {
                self.selected_stage_index = self.selected_stage_index.saturating_add(1);
            }
        } else {
            // Navigate beads
            if self.selected_index < self.beads.len().saturating_sub(1) {
                self.selected_index = self.selected_index.saturating_add(1);
                if self.mode == ViewMode::PipelineView {
                    self = self.trigger_pipeline_load();
                }
            }
        }
        (self, true)
    }

    fn handle_nav_up(mut self) -> (Self, bool) {
        if self.mode == ViewMode::PipelineView {
            // Navigate pipeline stages
            self.selected_stage_index = self.selected_stage_index.saturating_sub(1);
        } else {
            // Navigate beads
            self.selected_index = self.selected_index.saturating_sub(1);
            if self.mode == ViewMode::PipelineView {
                self = self.trigger_pipeline_load();
            }
        }
        (self, true)
    }

    fn switch_to_pipeline_view(mut self) -> (Self, bool) {
        self.mode = ViewMode::PipelineView;
        self.selected_stage_index = 0;
        self = self.trigger_pipeline_load();
        (self, true)
    }

    fn switch_to_agent_view(mut self) -> (Self, bool) {
        self.mode = ViewMode::AgentView;
        self = self.trigger_agents_load();
        (self, true)
    }

    fn switch_to_graph_view(mut self) -> (Self, bool) {
        self.mode = ViewMode::GraphView;
        self = self.trigger_graph_load();
        (self, true)
    }

    fn switch_to_system_health_view(mut self) -> (Self, bool) {
        self.mode = ViewMode::SystemHealth;
        self = self.trigger_system_health_load();
        (self, true)
    }

    fn switch_to_log_aggregator_view(mut self) -> (Self, bool) {
        self.mode = ViewMode::LogAggregator;
        self = self.trigger_log_aggregator_load();
        (self, true)
    }

    fn handle_enter_key(mut self) -> (Self, bool) {
        if self.mode == ViewMode::PipelineView {
            // In PipelineView: open command pane to rerun selected stage
            if let Some(bead) = self.beads.get(self.selected_index) {
                if let Some(stage) = self.pipeline_stages.get(self.selected_stage_index) {
                    self.open_command_pane_for_stage(&bead.id, &stage.name);
                }
            }
            (self, true)
        } else {
            // Other modes: stay in current mode and reload data
            if self.mode == ViewMode::PipelineView {
                self = self.trigger_pipeline_load();
            }
            if self.mode == ViewMode::AgentView {
                self = self.trigger_agents_load();
            }
            if self.mode == ViewMode::GraphView {
                self = self.trigger_graph_load();
            }
            if self.mode == ViewMode::SystemHealth {
                self = self.trigger_system_health_load();
            }
            if self.mode == ViewMode::LogAggregator {
                self = self.trigger_log_aggregator_load();
            }
            (self, true)
        }
    }

    fn handle_refresh(mut self) -> (Self, bool) {
        self.beads_cache = None;
        self.agents_cache = None;
        self.pipeline_caches = HashMap::new();
        self = self.trigger_beads_load();

        if self.mode == ViewMode::PipelineView {
            self = self.trigger_pipeline_load();
        }
        if self.mode == ViewMode::GraphView {
            self = self.trigger_graph_load();
        }
        if self.mode == ViewMode::SystemHealth {
            self = self.trigger_system_health_load();
        }
        if self.mode == ViewMode::LogAggregator {
            self = self.trigger_log_aggregator_load();
        }

        (self, true)
    }

    // State update helpers
    fn with_mode(mut self, mode: ViewMode) -> Self {
        self.mode = mode;
        self
    }

    fn with_selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    #[allow(dead_code)]
    fn with_selected_stage_index(mut self, index: usize) -> Self {
        self.selected_stage_index = index;
        self
    }

    #[allow(dead_code)]
    fn with_network_timeout(mut self) -> Self {
        self.api_connected = false;
        self.last_error = Some("Network timeout".to_string());
        self.pending_requests = 0;
        self.last_request_sent = None;
        self
    }

    #[allow(dead_code)]
    fn with_cleared_caches(mut self) -> Self {
        self.beads_cache = None;
        self.agents_cache = None;
        self.pipeline_caches = HashMap::new();
        self
    }

    // Trigger methods that perform side effects and return new state
    // These methods bridge between the functional state model and the Zellij trait's
    // requirement for &mut self by using the exterior mut pattern.
    #[must_use]
    fn trigger_beads_load(self) -> Self {
        self.load_beads().0
    }

    #[must_use]
    fn trigger_pipeline_load(self) -> Self {
        self.load_pipeline_for_selected().0
    }

    #[must_use]
    fn trigger_agents_load(self) -> Self {
        self.load_agents().0
    }

    #[must_use]
    fn trigger_graph_load(self) -> Self {
        self.load_graph().0
    }

    #[must_use]
    fn trigger_system_health_load(self) -> Self {
        self.load_system_health().0
    }

    #[must_use]
    fn trigger_log_aggregator_load(self) -> Self {
        self.load_log_aggregator().0
    }

    fn handle_web_response(
        mut self,
        status: u16,
        _headers: BTreeMap<String, String>,
        body: Vec<u8>,
        context: BTreeMap<String, String>,
    ) -> (Self, bool) {
        self.pending_requests = self.pending_requests.saturating_sub(1);
        if self.pending_requests == 0 {
            self.last_request_sent = None;
        }

        if !(200..300).contains(&status) {
            self.api_connected = false;
            self.last_error = Some(if (500..600).contains(&status) {
                format!("Server Error: HTTP {}", status)
            } else {
                format!("HTTP {}", status)
            });
            return (self, true);
        }

        self.api_connected = true;
        self.last_error = None;

        match context.get(CTX_REQUEST_TYPE).map(|s| s.as_str()) {
            Some(CTX_BEADS_LIST) => {
                self = self.parse_beads_response(&body);
                self.beads_cache = Some((self.beads.clone(), Instant::now()));
            }
            Some(CTX_PIPELINE) => {
                self = self.parse_pipeline_response(&body);
                if let Some(bead_id) = context.get(CTX_BEAD_ID) {
                    self.pipeline_caches.insert(
                        bead_id.clone(),
                        (self.pipeline_stages.clone(), Instant::now()),
                    );
                }
            }
            Some(CTX_AGENTS_LIST) => {
                self = self.parse_agents_response(&body);
                self.agents_cache = Some((self.agents.clone(), Instant::now()));
            }
            Some(CTX_GRAPH) => {
                self = self.parse_graph_response(&body);
                self.graph_cache = Some((
                    self.graph_nodes.clone(),
                    self.graph_edges.clone(),
                    self.critical_path.clone(),
                    Instant::now(),
                ));
            }
            _ => (),
        }

        (self, true)
    }

    fn handle_command_pane_opened(
        self,
        _pane_id: u32,
        _context: BTreeMap<String, String>,
    ) -> (Self, bool) {
        // Command pane opened tracking to be implemented when needed
        (self, true)
    }

    fn handle_command_pane_exited(
        mut self,
        _pane_id: u32,
        exit_code: Option<i32>,
        _context: BTreeMap<String, String>,
    ) -> (Self, bool) {
        // Command pane exited - refresh pipeline to show updated stage status
        if self.mode == ViewMode::PipelineView {
            self.pipeline_caches = HashMap::new();
            self = self.load_pipeline_for_selected().0;
        }

        // Track command pane completion
        let pane_id_str = _pane_id.to_string();
        if let Some(pane) = self.command_panes.get_mut(&pane_id_str) {
            let code = exit_code.map_or(-1, |c| c);
            pane.mark_completed(code);

            // Update the pipeline stage status if this was a stage run
            if pane.action == "run_stage" {
                if let Some(stage_name) = pane.stage_name.clone() {
                    let _bead_id = pane.bead_id.clone();
                    // Update stage status functionally
                    let new_status = if code == 0 {
                        StageStatus::Passed
                    } else {
                        StageStatus::Failed
                    };

                    self.pipeline_stages = self
                        .pipeline_stages
                        .iter()
                        .map(|stage| {
                            if stage.name == stage_name {
                                StageInfo {
                                    status: new_status,
                                    exit_code: Some(code),
                                    ..stage.clone()
                                }
                            } else {
                                stage.clone()
                            }
                        })
                        .collect();
                }
            }
        }

        (self, true)
    }

    fn handle_command_pane_rerun(
        self,
        _pane_id: u32,
        context: BTreeMap<String, String>,
    ) -> (Self, bool) {
        // Handle CommandPaneReRun event - rerun the stage
        let bead_id = context.get("bead_id");
        let stage_name = context.get("stage_name");

        if let (Some(bead_id), Some(stage_name)) = (bead_id, stage_name) {
            self.open_command_pane_for_stage(bead_id, stage_name);
        }

        (self, true)
    }

    // Load methods - functional pattern: consume self, perform side effect, return new state
    // Note: web_request() is a side effect (I/O), but state transformation is pure.
    // This bridges the gap between functional state management and Zellij's I/O requirements.
    fn load_beads(mut self) -> (Self, Result<()>) {
        if let Some((cached_beads, timestamp)) = &self.beads_cache {
            if timestamp.elapsed() < CACHE_TTL {
                self.beads = cached_beads.clone();
                return (self, Ok(()));
            }
        }

        let url = format!("{}/api/beads", self.server_url);
        let mut context = BTreeMap::new();
        context.insert(CTX_REQUEST_TYPE.to_string(), CTX_BEADS_LIST.to_string());
        self.pending_requests = self.pending_requests.saturating_add(1);
        self.last_request_sent = Some(Instant::now());
        web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
        (self, Ok(()))
    }

    fn load_pipeline_for_selected(mut self) -> (Self, Result<()>) {
        let Some(bead) = self.beads.get(self.selected_index) else {
            return (self, Ok(()));
        };

        if let Some((cached_stages, timestamp)) = self.pipeline_caches.get(&bead.id) {
            if timestamp.elapsed() < CACHE_TTL {
                self.pipeline_stages = cached_stages.clone();
                return (self, Ok(()));
            }
        }

        let url = format!("{}/api/beads/{}/pipeline", self.server_url, bead.id);
        let mut context = BTreeMap::new();
        context.insert(CTX_REQUEST_TYPE.to_string(), CTX_PIPELINE.to_string());
        context.insert(CTX_BEAD_ID.to_string(), bead.id.clone());
        self.pending_requests = self.pending_requests.saturating_add(1);
        self.last_request_sent = Some(Instant::now());
        web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
        (self, Ok(()))
    }

    fn load_agents(mut self) -> (Self, Result<()>) {
        if let Some((cached_agents, timestamp)) = &self.agents_cache {
            if timestamp.elapsed() < CACHE_TTL {
                self.agents = cached_agents.clone();
                return (self, Ok(()));
            }
        }

        let url = format!("{}/api/agents", self.server_url);
        let mut context = BTreeMap::new();
        context.insert(CTX_REQUEST_TYPE.to_string(), CTX_AGENTS_LIST.to_string());
        self.pending_requests = self.pending_requests.saturating_add(1);
        self.last_request_sent = Some(Instant::now());
        web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
        (self, Ok(()))
    }

    fn load_graph(mut self) -> (Self, Result<()>) {
        if let Some((cached_nodes, cached_edges, cached_path, timestamp)) = &self.graph_cache {
            if timestamp.elapsed() < CACHE_TTL {
                self.graph_nodes = cached_nodes.clone();
                self.graph_edges = cached_edges.clone();
                self.critical_path = cached_path.clone();
                return (self, Ok(()));
            }
        }

        let url = format!("{}/api/graph", self.server_url);
        let mut context = BTreeMap::new();
        context.insert(CTX_REQUEST_TYPE.to_string(), CTX_GRAPH.to_string());
        self.pending_requests = self.pending_requests.saturating_add(1);
        self.last_request_sent = Some(Instant::now());
        web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
        (self, Ok(()))
    }

    fn load_system_health(self) -> (Self, Result<()>) {
        // System health loading to be implemented when backend API is ready
        (self, Ok(()))
    }

    fn load_log_aggregator(self) -> (Self, Result<()>) {
        // Log aggregator loading to be implemented when backend API is ready
        (self, Ok(()))
    }

    fn open_command_pane_for_stage(&self, bead_id: &str, stage_name: &str) {
        let command = format!("oya stage -s {} --stage {}", bead_id, stage_name);

        // Create context for the command pane
        let mut context = BTreeMap::new();
        context.insert("bead_id".to_string(), bead_id.to_string());
        context.insert("stage_name".to_string(), stage_name.to_string());
        context.insert("action".to_string(), "run_stage".to_string());

        open_command_pane(
            CommandToRun::new_with_args("/bin/sh", vec!["-c", &command]),
            context,
        );
    }

    // Old imperative methods removed - replaced with functional versions above

    fn parse_beads_response(mut self, body: &[u8]) -> Self {
        #[derive(serde::Deserialize)]
        struct ApiBeadInfo {
            id: String,
            title: String,
            status: String,
            #[serde(default)]
            current_stage: Option<String>,
            #[serde(default)]
            progress: Option<f32>,
        }

        let parsed = std::str::from_utf8(body)
            .map_err(|_| "Invalid UTF-8 in response".to_string())
            .and_then(|body_str| {
                serde_json::from_str::<Vec<ApiBeadInfo>>(body_str)
                    .map_err(|e| format!("Parse error: {}", e))
            });

        match parsed {
            Ok(api_beads) => {
                self.beads = api_beads
                    .into_iter()
                    .map(|b| BeadInfo {
                        id: b.id,
                        title: b.title,
                        status: match b.status.as_str() {
                            "in_progress" => BeadStatus::InProgress,
                            "completed" | "closed" => BeadStatus::Completed,
                            "failed" => BeadStatus::Failed,
                            _ => BeadStatus::Pending,
                        },
                        current_stage: b.current_stage,
                        progress: b.progress.map_or(0.0, |p| p),
                        history: Vector::new(),
                    })
                    .collect::<Vector<_>>();
                if self.selected_index >= self.beads.len() {
                    self.selected_index = self.beads.len().saturating_sub(1);
                }
                self
            }
            Err(e) => {
                self.last_error = Some(e);
                self
            }
        }
    }

    fn parse_pipeline_response(mut self, body: &[u8]) -> Self {
        #[derive(serde::Deserialize)]
        struct ApiStageInfo {
            name: String,
            status: String,
            #[serde(default)]
            duration_ms: Option<u64>,
            #[serde(default)]
            exit_code: Option<i32>,
        }

        let parsed = std::str::from_utf8(body)
            .map_err(|_| "Invalid UTF-8 in response".to_string())
            .and_then(|body_str| {
                serde_json::from_str::<Vec<ApiStageInfo>>(body_str)
                    .map_err(|e| format!("Parse error: {}", e))
            });

        match parsed {
            Ok(api_stages) => {
                self.pipeline_stages = api_stages
                    .into_iter()
                    .map(|s| StageInfo {
                        name: s.name,
                        status: match s.status.as_str() {
                            "running" => StageStatus::Running,
                            "passed" => StageStatus::Passed,
                            "failed" => StageStatus::Failed,
                            "skipped" => StageStatus::Skipped,
                            _ => StageStatus::Pending,
                        },
                        duration_ms: s.duration_ms,
                        exit_code: s.exit_code,
                    })
                    .collect::<Vector<_>>();
                self
            }
            Err(e) => {
                self.last_error = Some(e);
                self
            }
        }
    }

    fn parse_agents_response(mut self, body: &[u8]) -> Self {
        #[derive(serde::Deserialize)]
        struct ApiAgentInfo {
            id: String,
            #[serde(alias = "status")]
            state: String,
            #[serde(default)]
            current_bead: Option<String>,
            health_score: f64,
            uptime_secs: u64,
            #[serde(default)]
            capabilities: Vec<String>,
            #[serde(default)]
            beads_completed: u64,
            #[serde(default)]
            operations_executed: u64,
            #[serde(default)]
            avg_execution_secs: Option<f64>,
        }

        #[derive(serde::Deserialize)]
        struct ApiAgentsResponse {
            agents: Vec<ApiAgentInfo>,
        }

        let parsed = std::str::from_utf8(body)
            .map_err(|_| "Invalid UTF-8 in response".to_string())
            .and_then(|body_str| {
                serde_json::from_str::<ApiAgentsResponse>(body_str)
                    .map(|response| response.agents)
                    .or_else(|_| serde_json::from_str::<Vec<ApiAgentInfo>>(body_str))
                    .map_err(|e| format!("Parse error: {}", e))
            });

        match parsed {
            Ok(api_agents) => {
                let next_agents = api_agents
                    .into_iter()
                    .map(|a| AgentInfo {
                        id: a.id,
                        state: match a.state.as_str() {
                            "working" => AgentState::Working,
                            "unhealthy" => AgentState::Unhealthy,
                            "shutting_down" => AgentState::ShuttingDown,
                            "terminated" => AgentState::Terminated,
                            _ => AgentState::Idle,
                        },
                        current_bead: a.current_bead,
                        health_score: a.health_score,
                        uptime_secs: a.uptime_secs,
                        capabilities: a.capabilities.into_iter().collect::<Vector<_>>(),
                        workload_history: WorkloadHistory {
                            beads_completed: a.beads_completed,
                            operations_executed: a.operations_executed,
                            avg_execution_secs: a.avg_execution_secs,
                        },
                    })
                    .collect::<Vector<_>>();
                self = self.update_agent_events(&next_agents);
                self.agents = next_agents;
                self
            }
            Err(e) => {
                self.last_error = Some(e);
                self
            }
        }
    }

    fn parse_graph_response(mut self, body: &[u8]) -> Self {
        #[derive(serde::Deserialize)]
        struct ApiGraphNode {
            id: String,
            label: String,
            state: String,
        }

        #[derive(serde::Deserialize)]
        struct ApiGraphEdge {
            from: String,
            to: String,
        }

        #[derive(serde::Deserialize)]
        struct ApiGraphResponse {
            nodes: Vec<ApiGraphNode>,
            edges: Vec<ApiGraphEdge>,
            critical_path: Vec<String>,
        }

        let parsed = std::str::from_utf8(body)
            .map_err(|_| "Invalid UTF-8 in response".to_string())
            .and_then(|body_str| {
                serde_json::from_str::<ApiGraphResponse>(body_str)
                    .map_err(|e| format!("Parse error: {}", e))
            });

        match parsed {
            Ok(api_graph) => {
                let critical_path_set: HashSet<String> =
                    api_graph.critical_path.into_iter().collect();

                self.graph_nodes = api_graph
                    .nodes
                    .into_iter()
                    .map(|n| GraphNode {
                        id: n.id.clone(),
                        label: n.label,
                        is_on_critical_path: critical_path_set.contains(&n.id),
                        state: match n.state.as_str() {
                            "running" => NodeState::Running,
                            "blocked" => NodeState::Blocked,
                            "completed" => NodeState::Completed,
                            "failed" => NodeState::Failed,
                            _ => NodeState::Idle,
                        },
                    })
                    .collect::<Vector<_>>();

                self.graph_edges = api_graph
                    .edges
                    .into_iter()
                    .map(|e| {
                        let is_critical = critical_path_set.contains(&e.from)
                            && critical_path_set.contains(&e.to);
                        GraphEdge {
                            from: e.from,
                            to: e.to,
                            is_on_critical_path: is_critical,
                        }
                    })
                    .collect::<Vector<_>>();

                self.critical_path = critical_path_set.into_iter().collect::<Vector<_>>();
                self
            }
            Err(e) => {
                self.last_error = Some(e);
                self
            }
        }
    }

    fn render_header(&self, cols: usize) {
        let title = "OYA Pipeline Dashboard";
        let status_symbol = if self.api_connected { "●" } else { "○" };
        let status_color = if self.api_connected {
            "\x1b[32m"
        } else {
            "\x1b[31m"
        };

        println!(
            "\x1b[1m{}\x1b[0m{}{}{}\x1b[0m",
            title,
            " ".repeat(cols.saturating_sub(title.len().saturating_add(3))),
            status_color,
            status_symbol
        );
        println!("{}", "─".repeat(cols));
    }

    fn render_bead_list(&self, rows: usize, cols: usize) {
        if self.beads.is_empty() {
            println!("\n  \x1b[2mNo beads found. Create one with: oya new -s <slug>\x1b[0m");
            return;
        }

        println!(
            "\n  \x1b[1m{:<12} {:<45} {:<12} {:<15} Progress\x1b[0m",
            "ID", "Title", "Status", "Stage"
        );
        println!("  {}", "─".repeat(cols.saturating_sub(2)));

        self.beads
            .iter()
            .take(rows.saturating_sub(3))
            .enumerate()
            .for_each(|(idx, bead)| {
                let selected = idx == self.selected_index;
                let prefix = if selected { "\x1b[7m> " } else { "  " };
                let suffix = if selected { "\x1b[0m" } else { "" };

                let title = truncate(&bead.title, 45);
                let stage = bead.current_stage.as_deref().map_or("-", |s| s);
                let progress_bar = render_progress_bar(bead.progress, 15);

                println!(
                    "{}{:<12} {:<45} {}{:<12}\x1b[0m {:<15} {}{}",
                    prefix,
                    bead.id,
                    title,
                    bead.status.color(),
                    bead.status.as_str(),
                    stage,
                    progress_bar,
                    suffix
                );
            });

        let total = self.beads.len();
        let completed = self
            .beads
            .iter()
            .filter(|b| matches!(b.status, BeadStatus::Completed))
            .count();
        let in_progress = self
            .beads
            .iter()
            .filter(|b| matches!(b.status, BeadStatus::InProgress))
            .count();
        let failed = self
            .beads
            .iter()
            .filter(|b| matches!(b.status, BeadStatus::Failed))
            .count();

        println!(
            "\n  \x1b[2m{} total | {} completed | {} in progress | {} failed\x1b[0m",
            total, completed, in_progress, failed
        );
    }

    fn render_bead_detail(&self, rows: usize, cols: usize) {
        let Some(bead) = self.beads.get(self.selected_index) else {
            println!("\n  \x1b[2mNo bead selected\x1b[0m");
            return;
        };

        // Convert BeadInfo to BeadDetail for rendering
        let bead_detail = ui::bead_detail::BeadDetail::new(
            bead.id.clone(),
            bead.title.clone(),
            bead.status,
            bead.progress,
        );

        // Add stage if present
        let bead_detail = if let Some(stage) = bead.current_stage.as_ref() {
            bead_detail.with_stage(stage.clone())
        } else {
            bead_detail
        };

        // Add history entries
        let bead_detail = bead.history.iter().fold(bead_detail, |detail, entry| {
            detail.with_history_entry(entry.clone())
        });

        // Render the bead detail with history section
        bead_detail.render(rows, cols);
    }

    fn render_pipeline_view(&self, rows: usize, cols: usize) {
        let Some(bead) = self.beads.get(self.selected_index) else {
            println!("\n  \x1b[2mNo bead selected\x1b[0m");
            return;
        };

        println!("\n  \x1b[1mPipeline Stages: {}\x1b[0m", bead.id);
        println!("  {}", "─".repeat(cols.saturating_sub(2)));
        println!();

        if self.pipeline_stages.is_empty() {
            println!("  \x1b[2mNo pipeline stages yet\x1b[0m");
            return;
        }

        println!("  Pipeline Flow:");
        self.pipeline_stages
            .iter()
            .take(rows.saturating_sub(8))
            .enumerate()
            .for_each(|(idx, stage)| {
                let symbol = stage.status.symbol();
                let color = stage.status.color();
                let connector = if idx < self.pipeline_stages.len().saturating_sub(1) {
                    "│"
                } else {
                    " "
                };

                let duration_str = stage
                    .duration_ms
                    .map(|ms| format!("({:.1}s)", ms as f64 / 1000.0));
                let exit_code_str = stage.exit_code.map(|code| format!("(exit {code})"));
                let details = match (duration_str, exit_code_str) {
                    (Some(duration), Some(exit_code)) => format!("{duration} {exit_code}"),
                    (Some(duration), None) => duration,
                    (None, Some(exit_code)) => exit_code,
                    (None, None) => String::new(),
                };

                // Highlight selected stage
                let is_selected = idx == self.selected_stage_index;
                let prefix = if is_selected { "\x1b[7m> " } else { "  " };
                let suffix = if is_selected { "\x1b[0m" } else { "" };

                println!(
                    "{}{} {}{}\x1b[0m {:<15} {}{}",
                    prefix, connector, color, symbol, stage.name, details, suffix
                );
            });

        let passed = self
            .pipeline_stages
            .iter()
            .filter(|s| matches!(s.status, StageStatus::Passed))
            .count();
        let total = self.pipeline_stages.len();
        let progress = if total > 0 {
            passed as f32 / total as f32
        } else {
            0.0
        };

        println!();
        println!(
            "  Overall: {}/{} stages passed {}",
            passed,
            total,
            render_progress_bar(progress, 20)
        );
    }

    fn render_agent_list(&self, rows: usize, cols: usize) {
        if self.agents.is_empty() {
            println!("\n  \x1b[2mNo agents found\x1b[0m");
            return;
        }

        let show_events = rows >= 12 && !self.agent_events.is_empty();
        let event_lines = if show_events {
            rows.saturating_div(3).max(4)
        } else {
            0
        };
        let reserved = 3 + 1 + if show_events { 3 + event_lines } else { 0 };
        let list_capacity = rows.saturating_sub(reserved);

        println!(
            "\n  \x1b[1m{:<15} {:<12} {:<20} {:<12} Health\x1b[0m",
            "Agent ID", "State", "Current Bead", "Uptime"
        );
        println!("  {}", "─".repeat(cols.saturating_sub(2)));

        self.agents.iter().take(list_capacity).for_each(|agent| {
            let bead_str = agent.current_bead.as_deref().map_or("-", |s| s);
            let uptime_str = format_uptime(agent.uptime_secs);
            let health_color = if agent.health_score >= 0.8 {
                "\x1b[32m"
            } else if agent.health_score >= 0.5 {
                "\x1b[33m"
            } else {
                "\x1b[31m"
            };

            let health_percent = (agent.health_score * 100.0).clamp(0.0, 100.0);

            println!(
                "  {:<15} {}{:<12}\x1b[0m {:<20} {:<12} {}{:.1}%\x1b[0m",
                agent.id,
                agent.state.color(),
                agent.state.as_str(),
                truncate(bead_str, 20),
                uptime_str,
                health_color,
                health_percent
            );

            if !agent.capabilities.is_empty() {
                let caps_str = agent
                    .capabilities
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "    \x1b[2mCapabilities: {}\x1b[0m",
                    truncate(&caps_str, cols.saturating_sub(6))
                );
            }
        });

        let total = self.agents.len();
        let idle = self
            .agents
            .iter()
            .filter(|a| matches!(a.state, AgentState::Idle))
            .count();
        let working = self
            .agents
            .iter()
            .filter(|a| matches!(a.state, AgentState::Working))
            .count();
        let unhealthy = self
            .agents
            .iter()
            .filter(|a| matches!(a.state, AgentState::Unhealthy))
            .count();

        println!(
            "\n  \x1b[2m{} total | {} idle | {} working | {} unhealthy\x1b[0m",
            total, idle, working, unhealthy
        );

        if show_events {
            println!();
            println!("  \x1b[1mEvent Stream\x1b[0m");
            println!("  {}", "─".repeat(cols.saturating_sub(2)));
            self.render_agent_events(event_lines, cols);
        }
    }

    fn render_graph_view(&self, rows: usize, cols: usize) {
        if self.graph_nodes.is_empty() {
            println!("\n  \x1b[2mNo graph data available\x1b[0m");
            println!("  \x1b[2mPress 'r' to refresh from server\x1b[0m");
            return;
        }

        println!("\n  \x1b[1mDependency Graph (Critical Path Highlighted)\x1b[0m");
        println!("  {}", "─".repeat(cols.saturating_sub(2)));
        println!();

        // Count critical path items
        let critical_count = self
            .graph_nodes
            .iter()
            .filter(|n| n.is_on_critical_path)
            .count();

        let total_nodes = self.graph_nodes.len();
        let total_edges = self.graph_edges.len();

        // Display legend
        println!(
            "  \x1b[1mLegend:\x1b[0m \x1b[33m★\x1b[0m Critical Path | \x1b[90m○\x1b[0m Normal"
        );
        println!(
            "  \x1b[1mNodes:\x1b[0m {} total | {} on critical path",
            total_nodes, critical_count
        );
        println!("  \x1b[1mEdges:\x1b[0m {} total", total_edges);
        println!();

        // Display nodes with critical path highlighting
        let max_rows = rows.saturating_sub(12);
        println!("  \x1b[1mNodes:\x1b[0m");
        self.graph_nodes.iter().take(max_rows).for_each(|node| {
            let critical_marker = if node.is_on_critical_path {
                "\x1b[33m★\x1b[0m"
            } else {
                "\x1b[90m○\x1b[0m"
            };
            let node_color = if node.is_on_critical_path {
                "\x1b[33m" // Yellow for critical path
            } else {
                "\x1b[90m" // Gray for normal
            };

            println!(
                "  {} {}{}\x1b[0m {} {}{}",
                critical_marker,
                node_color,
                node.state.symbol(),
                truncate(&node.label, 30),
                node.state.color(),
                node.state.as_str()
            );
        });

        if self.graph_nodes.len() > max_rows {
            println!(
                "  \x1b[2m... and {} more nodes\x1b[0m",
                self.graph_nodes.len().saturating_sub(max_rows)
            );
        }

        println!();

        // Display edges with critical path highlighting
        let edge_max_rows = max_rows.saturating_sub(2);
        println!("  \x1b[1mEdges:\x1b[0m");
        self.graph_edges
            .iter()
            .take(edge_max_rows)
            .for_each(|edge| {
                let edge_color = if edge.is_on_critical_path {
                    "\x1b[33m" // Yellow for critical path
                } else {
                    "\x1b[90m" // Gray for normal
                };

                let critical_marker = if edge.is_on_critical_path {
                    "★"
                } else {
                    "○"
                };

                println!(
                    "  {} {}{} → {}\x1b[0m",
                    critical_marker,
                    edge_color,
                    truncate(&edge.from, 20),
                    truncate(&edge.to, 20)
                );
            });

        if self.graph_edges.len() > edge_max_rows {
            println!(
                "  \x1b[2m... and {} more edges\x1b[0m",
                self.graph_edges.len().saturating_sub(edge_max_rows)
            );
        }
    }

    fn render_system_health(&self, _rows: usize, _cols: usize) {
        println!("\n  \x1b[2mSystem Health view coming soon\x1b[0m");
        println!("  \x1b[2mPress 'r' to refresh from server\x1b[0m");
    }

    fn render_log_aggregator(&self, _rows: usize, _cols: usize) {
        println!("\n  \x1b[2mLog Aggregator view coming soon\x1b[0m");
        println!("  \x1b[2mPress 'r' to refresh from server\x1b[0m");
    }
    fn render_footer(&self, rows: usize, cols: usize) {
        print!("\x1b[{};1H", rows.saturating_sub(1));

        let view_mode = match self.mode {
            ViewMode::BeadList => "List",
            ViewMode::BeadDetail => "Detail",
            ViewMode::PipelineView => "Pipeline",
            ViewMode::AgentView => "Agents",
            ViewMode::GraphView => "Graph",
            ViewMode::SystemHealth => "Health",
            ViewMode::LogAggregator => "Logs",
        };

        println!("{}", "─".repeat(cols));

        let enter_hint = if self.mode == ViewMode::PipelineView {
            "Enter:Rerun"
        } else {
            "Enter:Cycle"
        };

        let help = format!(
            "\x1b[2m[{}] 1:List 2:Detail 3:Pipeline 4:Agents 5:Graph 6:Health 7:Logs | j/k:Navigate g/G:Top/Bottom {} r:Refresh q:Quit\x1b[0m",
            view_mode, enter_hint
        );

        self.last_error.as_ref().map_or_else(
            || println!("{}", help),
            |err| {
                println!(
                    "\x1b[31mError: {}\x1b[0m",
                    truncate(err, cols.saturating_sub(7))
                )
            },
        );
    }

    fn update_agent_events(mut self, next_agents: &Vector<AgentInfo>) -> Self {
        let mut previous_by_id: BTreeMap<String, AgentInfo> = self
            .agents
            .iter()
            .cloned()
            .map(|agent| (agent.id.clone(), agent))
            .collect();
        let next_by_id: BTreeMap<String, AgentInfo> = next_agents
            .iter()
            .cloned()
            .map(|agent| (agent.id.clone(), agent))
            .collect();

        for (agent_id, next_agent) in next_by_id.iter() {
            match previous_by_id.remove(agent_id) {
                None => {
                    self = self.push_agent_event(
                        EventLevel::Info,
                        format!("Agent {} registered", agent_id),
                    );
                }
                Some(previous) => {
                    if previous.state != next_agent.state {
                        let level = match next_agent.state {
                            AgentState::Unhealthy | AgentState::Terminated => EventLevel::Error,
                            AgentState::ShuttingDown => EventLevel::Warning,
                            _ => EventLevel::Info,
                        };
                        self = self.push_agent_event(
                            level,
                            format!(
                                "Agent {} state {} → {}",
                                agent_id,
                                previous.state.as_str(),
                                next_agent.state.as_str()
                            ),
                        );
                    }

                    if previous.current_bead != next_agent.current_bead {
                        match (&previous.current_bead, &next_agent.current_bead) {
                            (None, Some(bead)) => {
                                self = self.push_agent_event(
                                    EventLevel::Info,
                                    format!("Agent {} assigned bead {}", agent_id, bead),
                                );
                            }
                            (Some(bead), None) => {
                                self = self.push_agent_event(
                                    EventLevel::Info,
                                    format!("Agent {} released bead {}", agent_id, bead),
                                );
                            }
                            (Some(previous_bead), Some(next_bead)) => {
                                self = self.push_agent_event(
                                    EventLevel::Info,
                                    format!(
                                        "Agent {} switched bead {} → {}",
                                        agent_id, previous_bead, next_bead
                                    ),
                                );
                            }
                            (None, None) => {}
                        }
                    }

                    let previous_band = health_band(previous.health_score);
                    let next_band = health_band(next_agent.health_score);
                    if previous_band != next_band {
                        let level = match next_band {
                            HealthBand::Healthy => EventLevel::Info,
                            HealthBand::Warning => EventLevel::Warning,
                            HealthBand::Critical => EventLevel::Error,
                        };
                        self = self.push_agent_event(
                            level,
                            format!(
                                "Agent {} health {:.0}% → {:.0}%",
                                agent_id,
                                previous.health_score * 100.0,
                                next_agent.health_score * 100.0
                            ),
                        );
                    }
                }
            }
        }

        for (agent_id, _) in previous_by_id.iter() {
            self =
                self.push_agent_event(EventLevel::Warning, format!("Agent {} removed", agent_id));
        }

        self
    }

    fn push_agent_event(mut self, level: EventLevel, message: String) -> Self {
        self.agent_events.push_back(AgentEvent {
            message,
            level,
            occurred_at: Instant::now(),
        });
        while self.agent_events.len() > AGENT_EVENT_LIMIT {
            self.agent_events.pop_front();
        }
        self
    }

    fn render_agent_events(&self, rows: usize, cols: usize) {
        let message_width = cols.saturating_sub(12);
        self.agent_events.iter().rev().take(rows).for_each(|event| {
            let age = format_event_age(event.occurred_at);
            let message = truncate(&event.message, message_width);
            println!(
                "  {}{}\x1b[0m {:>4} {}",
                event.level.color(),
                event.level.symbol(),
                age,
                message
            );
        });
    }
}

// Helper functions
fn should_fetch_agents_on_view_load(mode: ViewMode) -> bool {
    matches!(mode, ViewMode::AgentView)
}

fn should_fetch_graph_on_view_load(mode: ViewMode) -> bool {
    matches!(mode, ViewMode::GraphView)
}

fn should_fetch_system_health_on_view_load(mode: ViewMode) -> bool {
    matches!(mode, ViewMode::SystemHealth)
}

fn should_fetch_log_aggregator_on_view_load(mode: ViewMode) -> bool {
    matches!(mode, ViewMode::LogAggregator)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn render_progress_bar(progress: f32, width: usize) -> String {
    let clamped = progress.clamp(0.0, 1.0);
    let filled = (clamped * width as f32).round() as usize;
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);
    let percent = (clamped * 100.0).round() as u8;

    format!(
        "\x1b[32m{}\x1b[90m{}\x1b[0m {}%",
        "█".repeat(filled),
        "░".repeat(empty),
        percent
    )
}

fn health_band(score: f64) -> HealthBand {
    if score >= 0.8 {
        HealthBand::Healthy
    } else if score >= 0.5 {
        HealthBand::Warning
    } else {
        HealthBand::Critical
    }
}

fn format_event_age(occurred_at: Instant) -> String {
    let elapsed = occurred_at.elapsed();
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs.saturating_div(60))
    } else if secs < 86400 {
        format!("{}h", secs.saturating_div(3600))
    } else {
        format!("{}d", secs.saturating_div(86400))
    }
}

fn format_uptime(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs.saturating_div(60))
    } else if secs < 86400 {
        format!(
            "{}h {}m",
            secs.saturating_div(3600),
            secs.saturating_sub(secs.saturating_div(3600).saturating_mul(3600))
                .saturating_div(60)
        )
    } else {
        format!("{}d", secs.saturating_div(86400))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn build_agent(id: &str, state: AgentState, bead: Option<&str>, health: f64) -> AgentInfo {
        AgentInfo {
            id: id.to_string(),
            state,
            current_bead: bead.map(|value| value.to_string()),
            health_score: health,
            uptime_secs: 42,
            capabilities: Vector::new(),
            workload_history: WorkloadHistory::default(),
        }
    }

    fn to_vector(agents: Vec<AgentInfo>) -> Vector<AgentInfo> {
        agents.into_iter().collect::<Vector<_>>()
    }

    fn to_vector_stages(stages: Vec<StageInfo>) -> Vector<StageInfo> {
        stages.into_iter().collect::<Vector<_>>()
    }

    #[test]
    fn agent_view_fetches_agents_on_load() {
        assert!(should_fetch_agents_on_view_load(ViewMode::AgentView));
    }

    #[test]
    fn non_agent_views_do_not_fetch_agents_on_load() {
        let modes = [
            ViewMode::BeadList,
            ViewMode::BeadDetail,
            ViewMode::PipelineView,
        ];

        for mode in modes {
            assert!(!should_fetch_agents_on_view_load(mode));
        }
    }

    #[test]
    fn test_agent_event_registered() {
        let state = State::default();
        let agents = to_vector(vec![build_agent("agent-1", AgentState::Idle, None, 0.95)]);

        let state = state.update_agent_events(&agents);

        assert_eq!(state.agent_events.len(), 1);
        assert!(state
            .agent_events
            .iter()
            .any(|event| event.message.contains("registered")));
    }

    #[test]
    fn test_agent_event_state_and_bead_change() {
        let state = {
            let mut state = State::default();
            let initial = to_vector(vec![build_agent("agent-7", AgentState::Idle, None, 0.9)]);
            state.agents = initial;
            state
        };

        let updated = to_vector(vec![build_agent(
            "agent-7",
            AgentState::Working,
            Some("bead-1"),
            0.9,
        )]);

        let state = state.update_agent_events(&updated);

        assert!(state
            .agent_events
            .iter()
            .any(|event| event.message.contains("state")));
        assert!(state
            .agent_events
            .iter()
            .any(|event| event.message.contains("assigned")));
    }

    #[test]
    fn test_agent_workload_history_default() {
        let agent = build_agent("agent-1", AgentState::Idle, None, 0.95);

        assert_eq!(agent.workload_history.beads_completed, 0);
        assert_eq!(agent.workload_history.operations_executed, 0);
        assert!(agent.workload_history.avg_execution_secs.is_none());
    }

    #[test]
    fn test_stage_selection_navigation() {
        let state = State {
            pipeline_stages: to_vector_stages(vec![
                StageInfo {
                    name: "stage-1".to_string(),
                    status: StageStatus::Passed,
                    duration_ms: Some(100),
                    exit_code: Some(0),
                },
                StageInfo {
                    name: "stage-2".to_string(),
                    status: StageStatus::Failed,
                    duration_ms: Some(200),
                    exit_code: Some(1),
                },
            ]),
            selected_stage_index: 0,
            ..Default::default()
        };

        let mut state = state;

        // Simulate Down key
        if state.selected_stage_index < state.pipeline_stages.len().saturating_sub(1) {
            state.selected_stage_index = state.selected_stage_index.saturating_add(1);
        }
        assert_eq!(state.selected_stage_index, 1);

        // Simulate Up key
        state.selected_stage_index = state.selected_stage_index.saturating_sub(1);
        assert_eq!(state.selected_stage_index, 0);
    }

    #[test]
    fn test_stage_selection_bounds() {
        let state = State {
            pipeline_stages: to_vector_stages(vec![StageInfo {
                name: "stage-1".to_string(),
                status: StageStatus::Pending,
                duration_ms: None,
                exit_code: None,
            }]),
            selected_stage_index: 0,
            ..Default::default()
        };

        let mut state = state;

        // Try to go down when at the last stage
        if state.selected_stage_index < state.pipeline_stages.len().saturating_sub(1) {
            state.selected_stage_index = state.selected_stage_index.saturating_add(1);
        }
        assert_eq!(state.selected_stage_index, 0);

        // Try to go up when at the first stage
        state.selected_stage_index = state.selected_stage_index.saturating_sub(1);
        assert_eq!(state.selected_stage_index, 0);
    }

    #[test]
    fn test_view_mode_has_seven_variants() {
        // Verify all 7 ViewMode variants are present and usable
        let _ = ViewMode::BeadList;
        let _ = ViewMode::BeadDetail;
        let _ = ViewMode::PipelineView;
        let _ = ViewMode::AgentView;
        let _ = ViewMode::GraphView;
        let _ = ViewMode::SystemHealth;
        let _ = ViewMode::LogAggregator;

        // Verify default is BeadList
        let default_mode = ViewMode::default();
        assert_eq!(default_mode, ViewMode::BeadList);
    }
}
