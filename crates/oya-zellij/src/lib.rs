//! OYA Zellij Plugin - Pipeline Orchestration Dashboard
//!
//! Lints inherited from workspace - no local exceptions allowed.
//!
//! Real-time terminal UI for pipeline status, bead execution, and stage progress.

use im::{HashMap, Vector};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use zellij_tile::prelude::*;

// Constants for caching and timeouts
const CACHE_TTL: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

// Context keys for identifying web request responses
const CTX_REQUEST_TYPE: &str = "request_type";
const CTX_BEADS_LIST: &str = "beads_list";
const CTX_PIPELINE: &str = "pipeline";
const CTX_BEAD_ID: &str = "bead_id";
const CTX_AGENTS_LIST: &str = "agents_list";

// Plugin state
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

    // Tracking for timeouts
    last_request_sent: Option<Instant>,

    // Bead data
    beads: Vector<BeadInfo>,
    selected_index: usize,

    // Pipeline data for selected bead
    pipeline_stages: Vector<StageInfo>,

    // Agent data
    agents: Vector<AgentInfo>,
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
            last_request_sent: None,
            beads: Vector::new(),
            selected_index: 0,
            pipeline_stages: Vector::new(),
            agents: Vector::new(),
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    #[default]
    BeadList,
    BeadDetail,
    PipelineView,
    AgentList,
}

#[derive(Clone, Debug)]
struct BeadInfo {
    id: String,
    title: String,
    status: BeadStatus,
    current_stage: Option<String>,
    progress: f32, // 0.0 to 1.0
}

#[derive(Clone, Copy, Debug)]
enum BeadStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl BeadStatus {
    fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    fn color(&self) -> &str {
        match self {
            Self::Pending => "\x1b[90m",    // gray
            Self::InProgress => "\x1b[33m", // yellow
            Self::Completed => "\x1b[32m",  // green
            Self::Failed => "\x1b[31m",     // red
        }
    }
}

#[derive(Clone, Debug)]
struct StageInfo {
    name: String,
    status: StageStatus,
    duration_ms: Option<u64>,
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
struct AgentInfo {
    id: String,
    state: AgentState,
    current_bead: Option<String>,
    health_score: f64,
    uptime_secs: u64,
    capabilities: Vector<String>,
}

#[derive(Clone, Copy, Debug)]
enum AgentState {
    Idle,
    Working,
    Unhealthy,
    ShuttingDown,
    Terminated,
}

impl AgentState {
    fn as_str(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Unhealthy => "unhealthy",
            Self::ShuttingDown => "shutting_down",
            Self::Terminated => "terminated",
        }
    }

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
        self.server_url = configuration
            .get("server_url")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "http://localhost:3000".to_string());

        // Request permissions (WebAccess required for HTTP calls)
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::WebAccess,
        ]);

        // Subscribe to events (WebRequestResult for HTTP responses)
        subscribe(&[
            EventType::Key,
            EventType::Timer,
            EventType::WebRequestResult,
        ]);

        // Set timer for auto-refresh (every 2 seconds)
        set_timeout(2.0);

        // Initial data load will happen after permission is granted
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key_with_mod) => match key_with_mod.bare_key {
                BareKey::Char('q') | BareKey::Esc => {
                    close_focus();
                    false
                }
                BareKey::Char('j') | BareKey::Down => {
                    if self.selected_index < self.beads.len().saturating_sub(1) {
                        self.selected_index = self.selected_index.saturating_add(1);
                        if self.mode == ViewMode::PipelineView {
                            self.load_pipeline_for_selected();
                        }
                    }
                    true
                }
                BareKey::Char('k') | BareKey::Up => {
                    self.selected_index = self.selected_index.saturating_sub(1);
                    if self.mode == ViewMode::PipelineView {
                        self.load_pipeline_for_selected();
                    }
                    true
                }
                BareKey::Char('g') => {
                    self.selected_index = 0;
                    true
                }
                BareKey::Char('G') => {
                    self.selected_index = self.beads.len().saturating_sub(1);
                    true
                }
                BareKey::Char('1') => {
                    self.mode = ViewMode::BeadList;
                    true
                }
                BareKey::Char('2') => {
                    self.mode = ViewMode::BeadDetail;
                    true
                }
                BareKey::Char('3') => {
                    self.mode = ViewMode::PipelineView;
                    self.load_pipeline_for_selected();
                    true
                }
                BareKey::Char('4') => {
                    self.mode = ViewMode::AgentList;
                    self.load_agents();
                    true
                }
                BareKey::Enter => {
                    self.mode = match self.mode {
                        ViewMode::BeadList => ViewMode::BeadDetail,
                        ViewMode::BeadDetail => ViewMode::PipelineView,
                        ViewMode::PipelineView => ViewMode::AgentList,
                        ViewMode::AgentList => ViewMode::BeadList,
                    };
                    if self.mode == ViewMode::PipelineView {
                        self.load_pipeline_for_selected();
                    }
                    if self.mode == ViewMode::AgentList {
                        self.load_agents();
                    }
                    true
                }
                BareKey::Char('r') => {
                    self.beads_cache = None;
                    self.agents_cache = None;
                    self.pipeline_caches = HashMap::new();
                    self.load_beads();
                    if self.mode == ViewMode::PipelineView {
                        self.load_pipeline_for_selected();
                    }
                    true
                }
                _ => false,
            },
            Event::Timer(_) => {
                // Check for network timeouts
                if self.pending_requests > 0 {
                    if self
                        .last_request_sent
                        .is_some_and(|last| last.elapsed() > REQUEST_TIMEOUT)
                    {
                        self.api_connected = false;
                        self.last_error = Some("Network timeout".to_string());
                        self.pending_requests = 0;
                        self.last_request_sent = None;
                    }
                }

                self.load_beads();
                if self.mode == ViewMode::AgentList {
                    self.load_agents();
                }
                set_timeout(2.0);
                true
            }
            Event::PermissionRequestResult(_) => {
                self.load_beads();
                true
            }
            Event::WebRequestResult(status, _headers, body, context) => {
                self.handle_web_response(status, body, context);
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        print!("\x1b[2J\x1b[H");
        self.render_header(cols);
        let content_rows = rows.saturating_sub(4);
        match self.mode {
            ViewMode::BeadList => self.render_bead_list(content_rows, cols),
            ViewMode::BeadDetail => self.render_bead_detail(content_rows, cols),
            ViewMode::PipelineView => self.render_pipeline_view(content_rows, cols),
            ViewMode::AgentList => self.render_agent_list(content_rows, cols),
        }
        self.render_footer(rows, cols);
    }
}

impl State {
    fn load_beads(&mut self) {
        if let Some((cached_beads, timestamp)) = &self.beads_cache {
            if timestamp.elapsed() < CACHE_TTL {
                self.beads = cached_beads.clone();
                return;
            }
        }

        let url = format!("{}/api/beads", self.server_url);
        let mut context = BTreeMap::new();
        context.insert(CTX_REQUEST_TYPE.to_string(), CTX_BEADS_LIST.to_string());
        self.pending_requests = self.pending_requests.saturating_add(1);
        self.last_request_sent = Some(Instant::now());
        web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
    }

    fn load_pipeline_for_selected(&mut self) {
        let Some(bead) = self.beads.get(self.selected_index) else {
            return;
        };

        if let Some((cached_stages, timestamp)) = self.pipeline_caches.get(&bead.id) {
            if timestamp.elapsed() < CACHE_TTL {
                self.pipeline_stages = cached_stages.clone();
                return;
            }
        }

        let url = format!("{}/api/beads/{}/pipeline", self.server_url, bead.id);
        let mut context = BTreeMap::new();
        context.insert(CTX_REQUEST_TYPE.to_string(), CTX_PIPELINE.to_string());
        context.insert(CTX_BEAD_ID.to_string(), bead.id.clone());
        self.pending_requests = self.pending_requests.saturating_add(1);
        self.last_request_sent = Some(Instant::now());
        web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
    }

    fn load_agents(&mut self) {
        if let Some((cached_agents, timestamp)) = &self.agents_cache {
            if timestamp.elapsed() < CACHE_TTL {
                self.agents = cached_agents.clone();
                return;
            }
        }

        let url = format!("{}/api/agents", self.server_url);
        let mut context = BTreeMap::new();
        context.insert(CTX_REQUEST_TYPE.to_string(), CTX_AGENTS_LIST.to_string());
        self.pending_requests = self.pending_requests.saturating_add(1);
        self.last_request_sent = Some(Instant::now());
        web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
    }

    fn handle_web_response(
        &mut self,
        status: u16,
        body: Vec<u8>,
        context: BTreeMap<String, String>,
    ) {
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
            return;
        }

        self.api_connected = true;
        self.last_error = None;

        match context.get(CTX_REQUEST_TYPE).map(|s| s.as_str()) {
            Some(CTX_BEADS_LIST) => {
                self.parse_beads_response(&body);
                self.beads_cache = Some((self.beads.clone(), Instant::now()));
            }
            Some(CTX_PIPELINE) => {
                self.parse_pipeline_response(&body);
                if let Some(bead_id) = context.get(CTX_BEAD_ID) {
                    self.pipeline_caches.insert(
                        bead_id.clone(),
                        (self.pipeline_stages.clone(), Instant::now()),
                    );
                }
            }
            Some(CTX_AGENTS_LIST) => {
                self.parse_agents_response(&body);
                self.agents_cache = Some((self.agents.clone(), Instant::now()));
            }
            _ => {}
        }
    }

    fn parse_beads_response(&mut self, body: &[u8]) {
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
                        progress: b.progress.unwrap_or(0.0),
                    })
                    .collect::<Vector<_>>();
                if self.selected_index >= self.beads.len() {
                    self.selected_index = self.beads.len().saturating_sub(1);
                }
            }
            Err(e) => self.last_error = Some(e),
        }
    }

    fn parse_pipeline_response(&mut self, body: &[u8]) {
        #[derive(serde::Deserialize)]
        struct ApiStageInfo {
            name: String,
            status: String,
            #[serde(default)]
            duration_ms: Option<u64>,
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
                    })
                    .collect::<Vector<_>>();
            }
            Err(e) => self.last_error = Some(e),
        }
    }

    fn parse_agents_response(&mut self, body: &[u8]) {
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
                self.agents = api_agents
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
                    })
                    .collect::<Vector<_>>();
            }
            Err(e) => self.last_error = Some(e),
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
                let stage = bead.current_stage.as_deref().unwrap_or("-");
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

    fn render_bead_detail(&self, _rows: usize, cols: usize) {
        let Some(bead) = self.beads.get(self.selected_index) else {
            println!("\n  \x1b[2mNo bead selected\x1b[0m");
            return;
        };

        println!("\n  \x1b[1mBead Details\x1b[0m");
        println!("  {}", "─".repeat(cols.saturating_sub(2)));
        println!();
        println!("  \x1b[1mID:\x1b[0m       {}", bead.id);
        println!("  \x1b[1mTitle:\x1b[0m    {}", bead.title);
        println!(
            "  \x1b[1mStatus:\x1b[0m   {}{}\x1b[0m",
            bead.status.color(),
            bead.status.as_str()
        );

        if let Some(stage) = bead.current_stage.as_ref() {
            println!("  \x1b[1mStage:\x1b[0m    {}", stage);
        }

        println!(
            "  \x1b[1mProgress:\x1b[0m {}",
            render_progress_bar(bead.progress, 30)
        );

        println!();
        println!("  \x1b[1mWorkspace:\x1b[0m");
        println!("    Path:   ~/.local/share/jj/repos/oya/{}", bead.id);
        println!("    Branch: {}", bead.id);

        println!();
        println!("  \x1b[1mQuick Actions:\x1b[0m");
        println!(
            "    \x1b[2mzjj spawn {}  # Open in isolated workspace\x1b[0m",
            bead.id
        );
        println!(
            "    \x1b[2moya stage -s {} --stage <name>  # Run stage\x1b[0m",
            bead.id
        );
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
                    .map(|ms| format!("({:.1}s)", ms as f64 / 1000.0))
                    .unwrap_or_default();

                println!(
                    "  {} {}{}\x1b[0m {:<15} {}",
                    connector, color, symbol, stage.name, duration_str
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

        println!(
            "\n  \x1b[1m{:<15} {:<12} {:<20} {:<12} Health\x1b[0m",
            "Agent ID", "State", "Current Bead", "Uptime"
        );
        println!("  {}", "─".repeat(cols.saturating_sub(2)));

        self.agents
            .iter()
            .take(rows.saturating_sub(3))
            .for_each(|agent| {
                let bead_str = agent.current_bead.as_deref().unwrap_or("-");
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
    }

    fn render_footer(&self, rows: usize, cols: usize) {
        print!("\x1b[{};1H", rows.saturating_sub(1));

        let view_mode = match self.mode {
            ViewMode::BeadList => "List",
            ViewMode::BeadDetail => "Detail",
            ViewMode::PipelineView => "Pipeline",
            ViewMode::AgentList => "Agents",
        };

        println!("{}", "─".repeat(cols));

        let help = format!(
            "\x1b[2m[{}] 1:List 2:Detail 3:Pipeline | j/k:Navigate g/G:Top/Bottom Enter:Cycle r:Refresh q:Quit\x1b[0m",
            view_mode
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
}

// Helper functions
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
