// Render module - Terminal rendering with ANSI box-drawing characters
//
// This module provides rendering functionality for the OYA UI plugin,
// including:
// - ANSI box-drawing characters for pane borders
// - Text layout and wrapping
// - Color and styling support
// - Focused pane highlighting
// - Help overlay rendering
// - DAG visualization for workflow graphs
// - Agent list view for pool monitoring

use crate::layout::{Layout, Pane, PaneType};
use crate::metrics::{AgentMetrics, PoolMetrics};
use crate::plugin::{StageInfo, StageState, TaskRow};
use oya_ipc::BeadDetail;
use rpds::Vector;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

// Style helper functions using functional patterns
mod style_helpers {
    use crate::components;

    #[must_use]
    pub const fn selected() -> &'static str {
        components::selected()
    }

    #[must_use]
    pub const fn header() -> &'static str {
        components::header()
    }

    #[must_use]
    pub const fn label() -> &'static str {
        components::label()
    }

    #[must_use]
    pub const fn text() -> &'static str {
        components::text()
    }

    #[must_use]
    pub const fn overlay() -> &'static str {
        components::overlay()
    }

    #[must_use]
    pub const fn border_normal() -> &'static str {
        components::border_normal()
    }

    #[must_use]
    pub const fn border_focused() -> &'static str {
        components::border_focused()
    }

    #[must_use]
    pub fn status_color(status: &str) -> &'static str {
        components::status_color(status)
    }
}

/// Errors that can occur during help overlay rendering
#[derive(Debug, Error, Clone, PartialEq)]
pub enum HelpOverlayError {
    /// Terminal too small to render overlay
    #[error("Terminal too small: {rows}x{cols}, minimum 10x40 required")]
    TerminalTooSmall { rows: usize, cols: usize },
}

/// Result type for help overlay rendering
pub type HelpOverlayResult<T> = Result<T, HelpOverlayError>;

/// Column widths for agent view rendering
struct AgentColumnWidths {
    id: usize,
    state: usize,
    health: usize,
    beads: usize,
}

/// Node status for DAG visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeStatus {
    /// Node is pending execution
    #[default]
    Pending,
    /// Node is currently in progress
    InProgress,
    /// Node completed successfully
    Completed,
    /// Node failed
    Failed,
    /// Node is blocked by dependencies
    Blocked,
}

/// A node in the DAG for visualization
#[derive(Debug, Clone, PartialEq)]
pub struct DagNode {
    /// Unique identifier for the node
    pub id: String,
    /// Display name for the node
    pub name: String,
    /// List of dependency node IDs
    pub dependencies: Vec<String>,
    /// Current status of the node
    pub status: NodeStatus,
}

impl DagNode {
    /// Create a new DAG node with default status
    #[must_use]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            dependencies: Vec::new(),
            status: NodeStatus::default(),
        }
    }
}

/// Rendered DAG output
#[derive(Debug, Clone)]
pub struct RenderedDag {
    /// Lines of rendered output
    pub lines: Vec<String>,
}

/// Errors during DAG rendering
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DagRenderError {
    /// Cycle detected in the graph
    #[error("Cycle detected in DAG: {cycle:?}")]
    CycleDetected { cycle: Vec<String> },
}

/// Errors that can occur during workflow graph operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum WorkflowGraphError {
    /// Node with the given ID already exists
    #[error("Node already exists: {id}")]
    DuplicateNode { id: String },
    /// Node with the given ID was not found
    #[error("Node not found: {id}")]
    NodeNotFound { id: String },
}

/// Result type for workflow graph operations
pub type WorkflowGraphResult<T> = Result<T, WorkflowGraphError>;

/// The type/kind of relationship represented by a graph edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeKind {
    /// Standard dependency relationship (default)
    #[default]
    Dependency,
    /// Blocking relationship - this edge blocks the target
    Blocks,
    /// Required relationship - hard requirement
    Requires,
    /// Soft/optional relationship
    Soft,
}

/// An edge in the workflow graph representing a relationship between two nodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    source: String,
    target: String,
    kind: EdgeKind,
}

impl GraphEdge {
    /// Create a new edge with default kind (Dependency)
    #[must_use]
    pub fn new(source: &str, target: &str) -> Self {
        Self {
            source: source.to_string(),
            target: target.to_string(),
            kind: EdgeKind::default(),
        }
    }

    /// Create a new edge with a specific kind
    #[must_use]
    pub fn new_with_kind(source: &str, target: &str, kind: EdgeKind) -> Self {
        Self {
            source: source.to_string(),
            target: target.to_string(),
            kind,
        }
    }

    /// Get the source node ID
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the target node ID
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Get the edge kind
    #[must_use]
    pub const fn kind(&self) -> EdgeKind {
        self.kind
    }

    /// Check if this edge connects the given source to target
    #[must_use]
    pub fn connects(&self, source: &str, target: &str) -> bool {
        self.source == source && self.target == target
    }
}

/// A workflow graph that manages nodes and dependencies for DAG visualization.
///
/// This struct provides a higher-level API for building and managing
/// workflow graphs that can be rendered using `DagRenderer`.
#[derive(Debug, Clone)]
pub struct WorkflowGraph {
    nodes: Vec<DagNode>,
    renderer: DagRenderer,
}

impl WorkflowGraph {
    /// Create a new empty workflow graph
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            renderer: DagRenderer::new(),
        }
    }

    /// Set custom dimensions for the renderer
    #[must_use]
    pub fn with_dimensions(mut self, width: usize, height: usize) -> Self {
        self.renderer = self.renderer.with_dimensions(width, height);
        self
    }

    /// Add a node to the graph
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the node
    /// * `name` - Display name for the node
    ///
    /// # Errors
    ///
    /// Returns `WorkflowGraphError::DuplicateNode` if a node with the same ID already exists
    pub fn add_node(&mut self, id: &str, name: &str) -> WorkflowGraphResult<()> {
        if self.nodes.iter().any(|n| n.id == id) {
            return Err(WorkflowGraphError::DuplicateNode { id: id.to_string() });
        }
        self.nodes.push(DagNode::new(id, name));
        Ok(())
    }

    /// Add a dependency relationship between two nodes
    ///
    /// # Arguments
    ///
    /// * `dependent` - ID of the node that has the dependency
    /// * `dependency` - ID of the node that is depended upon
    ///
    /// # Errors
    ///
    /// Returns `WorkflowGraphError::NodeNotFound` if either node doesn't exist
    pub fn add_dependency(&mut self, dependent: &str, dependency: &str) -> WorkflowGraphResult<()> {
        let dependent_exists = self.nodes.iter().any(|n| n.id == dependent);
        if !dependent_exists {
            return Err(WorkflowGraphError::NodeNotFound {
                id: dependent.to_string(),
            });
        }

        let dependency_exists = self.nodes.iter().any(|n| n.id == dependency);
        if !dependency_exists {
            return Err(WorkflowGraphError::NodeNotFound {
                id: dependency.to_string(),
            });
        }

        for node in &mut self.nodes {
            if node.id == dependent && !node.dependencies.contains(&dependency.to_string()) {
                node.dependencies.push(dependency.to_string());
            }
        }

        Ok(())
    }

    /// Set the status of a node
    ///
    /// # Arguments
    ///
    /// * `id` - ID of the node to update
    /// * `status` - New status for the node
    ///
    /// # Errors
    ///
    /// Returns `WorkflowGraphError::NodeNotFound` if the node doesn't exist
    pub fn set_node_status(&mut self, id: &str, status: NodeStatus) -> WorkflowGraphResult<()> {
        for node in &mut self.nodes {
            if node.id == id {
                node.status = status;
                return Ok(());
            }
        }
        Err(WorkflowGraphError::NodeNotFound { id: id.to_string() })
    }

    /// Get a reference to a node by ID
    #[must_use]
    pub fn get_node(&self, id: &str) -> Option<&DagNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all nodes in the graph
    #[must_use]
    pub fn nodes(&self) -> &[DagNode] {
        &self.nodes
    }

    /// Get the number of nodes in the graph
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Render the workflow graph to ASCII art
    #[must_use]
    pub fn render(&self) -> RenderedDag {
        self.renderer.render(&self.nodes)
    }

    /// Clear all nodes from the graph
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

impl Default for WorkflowGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// DAG renderer for workflow visualization
#[derive(Debug, Clone)]
pub struct DagRenderer {
    width: usize,
    height: usize,
}

impl DagRenderer {
    /// Create a new DAG renderer with default dimensions
    #[must_use]
    pub const fn new() -> Self {
        Self {
            width: 78,
            height: 6,
        }
    }

    /// Set custom dimensions
    #[must_use]
    pub const fn with_dimensions(mut self, width: usize, height: usize) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Get the configured width
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Get the configured height
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Render a DAG to ASCII art
    #[must_use]
    pub fn render(&self, nodes: &[DagNode]) -> RenderedDag {
        if nodes.is_empty() {
            return RenderedDag {
                lines: vec!["No workflow data available".to_string()],
            };
        }

        // Check for cycles first
        if let Some(cycle) = self.detect_cycle(nodes) {
            return RenderedDag {
                lines: vec![format!("Error: Cycle detected in graph: {:?}", cycle)],
            };
        }

        // Calculate levels for topological layout
        let levels = self.calculate_levels(nodes);

        // Render each level as a row
        let lines = self.render_levels(nodes, &levels);

        RenderedDag { lines }
    }

    /// Calculate topological levels for nodes
    #[must_use]
    pub fn calculate_levels(&self, nodes: &[DagNode]) -> Vec<Vec<String>> {
        let node_map: HashMap<&str, &DagNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut assigned: HashSet<String> = HashSet::new();

        loop {
            // Find nodes whose dependencies are all assigned
            let ready: Vec<String> = nodes
                .iter()
                .filter(|n| !assigned.contains(&n.id))
                .filter(|n| n.dependencies.iter().all(|dep| assigned.contains(dep)))
                .map(|n| n.id.clone())
                .collect();

            if ready.is_empty() {
                break;
            }

            for id in &ready {
                assigned.insert(id.clone());
            }

            levels.push(ready);
        }

        levels
    }

    /// Detect cycle in the graph using DFS
    fn detect_cycle(&self, nodes: &[DagNode]) -> Option<Vec<String>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        for node in nodes {
            if !visited.contains(&node.id) {
                if self.dfs_cycle(node, nodes, &mut visited, &mut rec_stack, &mut path) {
                    return Some(path);
                }
            }
        }

        None
    }

    /// DFS helper for cycle detection
    fn dfs_cycle(
        &self,
        node: &DagNode,
        nodes: &[DagNode],
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        visited.insert(node.id.clone());
        rec_stack.insert(node.id.clone());
        path.push(node.id.clone());

        // Find nodes that depend on this node
        for other in nodes {
            if other.dependencies.contains(&node.id) {
                if !visited.contains(&other.id) {
                    if self.dfs_cycle(other, nodes, visited, rec_stack, path) {
                        return true;
                    }
                } else if rec_stack.contains(&other.id) {
                    // Found cycle
                    let start_pos = path.iter().position(|id| id == &other.id);
                    if let Some(start) = start_pos {
                        if let Some(slice) = path.get(start..) {
                            *path = slice.to_vec();
                            path.push(other.id.clone());
                        }
                    }
                    return true;
                }
            }
        }

        rec_stack.remove(&node.id);
        path.pop();
        false
    }

    /// Render levels to ASCII lines
    fn render_levels(&self, nodes: &[DagNode], levels: &[Vec<String>]) -> Vec<String> {
        let node_map: HashMap<&str, &DagNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let mut lines = Vec::new();

        for (level_idx, level) in levels.iter().enumerate() {
            // Render boxes for this level side by side
            let box_lines = self.render_level_boxes(level, &node_map, level_idx);
            lines.extend(box_lines);

            // Add connector lines between levels (except after last level)
            if level_idx < levels.len().saturating_sub(1) {
                lines.push(String::new());
            }
        }

        lines
    }

    /// Render boxes for a single level
    fn render_level_boxes(
        &self,
        level: &[String],
        node_map: &HashMap<&str, &DagNode>,
        _level_idx: usize,
    ) -> Vec<String> {
        if level.is_empty() {
            return vec![];
        }

        // Max box width (accounting for spacing)
        let box_width = ((self.width.saturating_sub(2)) / level.len().max(1))
            .min(12)
            .max(6);

        // Render each node as a box
        let mut top_line = String::new();
        let mut mid_line = String::new();
        let mut bot_line = String::new();

        for (i, node_id) in level.iter().enumerate() {
            let node = node_map.get(node_id.as_str());
            let (name, status_color, status_symbol) = match node {
                Some(n) => {
                    let truncated = truncate(&n.name, box_width.saturating_sub(2));
                    let (color, symbol) = status_style(n.status);
                    (truncated, color, symbol)
                }
                None => (
                    truncate(node_id, box_width.saturating_sub(2)),
                    "\x1b[0m",
                    "○",
                ),
            };

            let padding = box_width
                .saturating_sub(name.chars().count())
                .saturating_sub(2);
            let left_pad = padding / 2;
            let right_pad = padding.saturating_sub(left_pad);

            // Add spacing between boxes
            if i > 0 {
                top_line.push_str("   ");
                mid_line.push_str("──▶");
                bot_line.push_str("   ");
            }

            top_line.push_str(&format!("┌{}┐", "─".repeat(box_width.saturating_sub(2))));
            mid_line.push_str(&format!(
                "{}│{}{}{}│\x1b[0m",
                status_color,
                " ".repeat(left_pad),
                name,
                " ".repeat(right_pad)
            ));
            bot_line.push_str(&format!("└{}┘", "─".repeat(box_width.saturating_sub(2))));
        }

        vec![top_line, mid_line, bot_line]
    }
}

impl Default for DagRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Get status color and symbol
fn status_style(status: NodeStatus) -> (&'static str, &'static str) {
    match status {
        NodeStatus::Pending => ("\x1b[33m", "○"),    // Yellow
        NodeStatus::InProgress => ("\x1b[34m", "◐"), // Blue
        NodeStatus::Completed => ("\x1b[32m", "✓"),  // Green
        NodeStatus::Failed => ("\x1b[31m", "✗"),     // Red
        NodeStatus::Blocked => ("\x1b[90m", "⊗"),    // Gray
    }
}

/// Terminal renderer for OYA UI
pub struct Renderer {
    /// Use colors (can be disabled for non-color terminals)
    use_colors: bool,
}

impl Renderer {
    /// Create a new renderer
    #[must_use]
    pub const fn new() -> Self {
        Self { use_colors: true }
    }

    /// Disable color output
    pub fn disable_colors(&mut self) {
        self.use_colors = false;
    }

    /// Enable color output
    pub fn enable_colors(&mut self) {
        self.use_colors = true;
    }

    /// Render the complete layout
    ///
    /// # Arguments
    ///
    /// * `layout` - Layout configuration
    /// * `beads` - List of beads to display
    /// * `selected_index` - Index of selected bead
    /// * `focused_pane` - Currently focused pane type
    ///
    /// # Returns
    ///
    /// Complete rendered output as a string
    #[must_use]
    pub fn render_layout(
        &self,
        layout: &Layout,
        tasks: &[TaskRow],
        selected_index: usize,
        focused_pane: PaneType,
        status_message: Option<&str>,
    ) -> String {
        // Render panes using functional fold pattern
        let panes_rendered = layout
            .panes()
            .iter()
            .fold(String::new(), |mut acc: String, pane| {
                let pane_content = match pane.pane_type {
                    PaneType::BeadList => self.render_bead_list(layout, tasks, selected_index),
                    PaneType::BeadDetail => tasks
                        .get(selected_index)
                        .map_or_else(String::new, |task| self.render_bead_detail(layout, task)),
                    PaneType::WorkflowGraph => self.render_workflow_graph(pane, focused_pane),
                    PaneType::PipelineView => tasks
                        .get(selected_index)
                        .map_or_else(String::new, |task| self.render_pipeline_view(pane, task)),
                    PaneType::AgentView => String::new(),
                };

                let rendered = self.render_pane(pane, &pane_content, focused_pane);
                acc.push_str(&rendered);
                acc
            });

        // Add status bar
        let status = self.render_status_bar(focused_pane, status_message);
        format!("{panes_rendered}{status}")
    }

    /// Render a single pane with border
    fn render_pane(&self, pane: &Pane, content: &str, focused_pane: PaneType) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let border_color = if is_focused {
            style_helpers::border_focused()
        } else {
            style_helpers::border_normal()
        };
        let width = pane.width;
        let title = format!(" {} ", pane.title);

        // Render top border
        let top_border = self.render_top_border(width, &title);

        // Render content lines using functional fold pattern
        let content_lines = content.lines().fold(String::new(), |mut acc, line| {
            let line_len = line.chars().count();
            let padding = " ".repeat(width.saturating_sub(2).saturating_sub(line_len));
            acc.push_str(border_color);
            acc.push('│');
            acc.push_str(line);
            acc.push_str(&padding);
            acc.push_str("│\n");
            acc
        });

        // Assemble complete pane
        let bottom_border = self.render_bottom_border(width);
        format!("{border_color}{top_border}{content_lines}{bottom_border}\x1b[0m")
    }

    /// Render top border with title
    fn render_top_border(&self, width: usize, title: &str) -> String {
        let title_len = title.chars().count();
        let remaining = width.saturating_sub(2).saturating_sub(title_len);

        // Functional pattern: use repeat instead of loop
        let border_line = "─".repeat(remaining);

        format!("┌{title}{border_line}┐\n")
    }

    /// Render bottom border
    fn render_bottom_border(&self, width: usize) -> String {
        let border_line = "─".repeat(width.saturating_sub(2));
        format!("└{border_line}┘")
    }

    /// Render task list pane
    fn render_bead_list(
        &self,
        layout: &Layout,
        tasks: &[TaskRow],
        selected_index: usize,
    ) -> String {
        let pane_width = layout.get_pane(PaneType::BeadList).map_or(40, |p| p.width);

        let mut output = String::new();

        let header = format!(
            "{}{:<1} {:<13} {:<6} Title\x1b[0m\n",
            style_helpers::header(),
            "",
            "Slug",
            "Stage"
        );
        output.push_str(&header);

        let separator = format!("{}\n", "─".repeat(pane_width.saturating_sub(2)));
        output.push_str(&separator);

        tasks.iter().enumerate().fold(output, |mut acc, (i, task)| {
            let is_selected = i == selected_index;

            if is_selected {
                acc.push_str(style_helpers::selected());
            }

            let indicator = if is_selected { "►" } else { " " };
            acc.push_str(indicator);

            if !is_selected {
                let status_color = style_helpers::status_color(&task.status);
                acc.push_str(status_color);
            }

            let slug = truncate(&task.slug, 12);
            let slug_padding = " ".repeat(14_usize.saturating_sub(slug.chars().count()));
            acc.push_str(&slug);
            acc.push_str(&slug_padding);

            let symbol = task
                .stage
                .as_ref()
                .and_then(|s| s.split(':').next())
                .map_or("○", stage_symbol);
            let symbol_padding = " ".repeat(16_usize.saturating_sub(symbol.chars().count()));
            acc.push_str(symbol);
            acc.push_str(&symbol_padding);

            let title = truncate(&task.slug, pane_width.saturating_sub(40));
            acc.push_str(&title);

            acc.push_str("\x1b[0m");

            if task.status == "in_progress" {
                let (running_stage, failed_stage, completed) = get_stage_info(task);
                let progress =
                    calculate_stage_progress(&task.stages, running_stage, failed_stage, completed);
                let bar = render_progress_bar(progress, 10);
                acc.push_str(&bar);
            }

            acc.push('\n');
            acc
        })
    }

    /// Render task detail pane
    fn render_bead_detail(&self, _layout: &Layout, task: &TaskRow) -> String {
        // Header with slug
        let header = format!("{}{}\n\n", style_helpers::header(), task.slug);

        // Build field lines using functional pattern
        let fields = [
            ("Status", &task.status),
            ("Priority", &task.priority),
            ("Language", &task.language),
            ("Branch", &task.branch),
        ];

        let field_lines = fields
            .iter()
            .fold(String::new(), |mut acc: String, (label, value)| {
                acc.push_str(style_helpers::label());
                acc.push_str(&format!("{label:<9} "));
                acc.push_str(style_helpers::text());
                acc.push_str(value);
                acc.push('\n');
                acc
            });

        // Stage line if present
        let stage_line = task.stage.as_ref().map_or_else(String::new, |stage| {
            format!(
                "{}Stage:    {}{}\n",
                style_helpers::label(),
                style_helpers::text(),
                stage
            )
        });

        // Pipeline header
        let pipeline_header = format!("\n{}Pipeline:\n", style_helpers::header());

        // Calculate stage info once
        let (running_stage, failed_stage, completed) = get_stage_info(task);

        // Render pipeline stages using functional fold
        let pipeline_stages =
            task.stages
                .iter()
                .fold(String::new(), |mut acc: String, stage_info: &StageInfo| {
                    let progress = calculate_stage_progress(
                        &task.stages,
                        running_stage,
                        failed_stage,
                        completed,
                    );
                    let bar = render_progress_bar(progress, 15);
                    acc.push_str(style_helpers::text());
                    acc.push_str("  ");
                    acc.push_str(stage_info.symbol());
                    acc.push(' ');
                    acc.push_str(&stage_info.name);
                    acc.push_str(&bar);
                    acc.push('\n');
                    acc
                });

        // Assemble complete detail view
        format!("{header}{field_lines}{stage_line}{pipeline_header}{pipeline_stages}")
    }

    /// Render pipeline view pane
    pub fn render_pipeline_view(&self, pane: &Pane, task: &TaskRow) -> String {
        let header = format!("{}Pipeline: {}\n\n", style_helpers::header(), task.slug);

        // Calculate stage info once
        let (running_stage, failed_stage, completed) = get_stage_info(task);

        // Render pipeline stages using functional fold
        let stages =
            task.stages
                .iter()
                .fold(String::new(), |mut acc: String, stage_info: &StageInfo| {
                    let progress = calculate_stage_progress(
                        &task.stages,
                        running_stage,
                        failed_stage,
                        completed,
                    );
                    let bar = render_progress_bar(progress, pane.width.saturating_sub(20));
                    acc.push_str(style_helpers::text());
                    acc.push_str(stage_info.symbol());
                    acc.push(' ');
                    acc.push_str(&stage_info.name);
                    acc.push_str(&bar);
                    acc.push('\n');

                    // Render substeps if any exist
                    for substep in &stage_info.substeps {
                        acc.push_str(style_helpers::text());
                        acc.push_str("  ");
                        acc.push_str(substep.symbol());
                        acc.push(' ');
                        acc.push_str(&substep.name);
                        acc.push('\n');
                    }

                    acc
                });

        format!("{header}{stages}")
    }

    /// Render workflow graph pane (DAG visualization)
    fn render_workflow_graph(&self, pane: &Pane, focused_pane: PaneType) -> String {
        let is_focused = pane.pane_type == focused_pane;

        if is_focused {
            let dag = self.render_horizontal_dag();
            format!(
                "{}Workflow Dependency Graph\n{}(Horizontal DAG visualization)\n\n{dag}",
                style_helpers::header(),
                style_helpers::text()
            )
        } else {
            "Press Enter to view graph".to_string()
        }
    }

    /// Render a horizontal DAG (left-to-right flow) using the DAG renderer
    fn render_horizontal_dag(&self) -> String {
        // Create sample workflow nodes for demonstration
        // In production, this data would come from the actual task list
        let nodes = vec![
            DagNode::new("src-abc", "Setup Project"),
            DagNode {
                id: "src-def".to_string(),
                name: "Core Module".to_string(),
                dependencies: vec!["src-abc".to_string()],
                status: NodeStatus::InProgress,
            },
            DagNode {
                id: "src-ghi".to_string(),
                name: "API Layer".to_string(),
                dependencies: vec!["src-def".to_string()],
                status: NodeStatus::Pending,
            },
            DagNode {
                id: "src-jkl".to_string(),
                name: "Tests".to_string(),
                dependencies: vec!["src-def".to_string()],
                status: NodeStatus::Pending,
            },
        ];

        let renderer = DagRenderer::new().with_dimensions(78, 6);
        let result = renderer.render(&nodes);
        result.lines.join("\n")
    }

    /// Render agent list view for pool monitoring
    ///
    /// Renders a list of agents with their status, health, and metrics.
    /// Includes pool-wide summary at the top.
    ///
    /// # Arguments
    ///
    /// * `pane` - Pane configuration for width calculation
    /// * `agents` - Vector of agent metrics
    /// * `pool` - Pool-wide metrics summary
    ///
    /// # Returns
    ///
    /// Formatted string with agent list view
    #[must_use]
    pub fn render_agent_view(
        &self,
        pane: &Pane,
        agents: &Vector<AgentMetrics>,
        pool: &PoolMetrics,
    ) -> String {
        let max_width = pane.width.saturating_sub(2);

        let header = format!("{}Agent Pool Status\n\x1b[0m", style_helpers::header());

        let pool_summary = format!(
            "{}Total: {}  Idle: {}  Working: {}  Unhealthy: {}\n\n",
            style_helpers::label(),
            pool.total,
            pool.idle,
            pool.working,
            pool.unhealthy
        );

        if agents.is_empty() {
            let empty_msg = format!("{}No agents connected\n", style_helpers::text());
            return format!("{header}{pool_summary}{empty_msg}");
        }

        let col_widths = self.calculate_agent_column_widths(max_width);
        let headers = format!(
            "{}{:<width_id$} {:<width_state$} {:>width_health$} {:>width_beads$}\n\x1b[0m",
            style_helpers::label(),
            "ID",
            "State",
            "Health",
            "Beads",
            width_id = col_widths.id,
            width_state = col_widths.state,
            width_health = col_widths.health,
            width_beads = col_widths.beads,
        );

        let agent_rows = agents.iter().fold(String::new(), |mut acc, agent| {
            let health_display = format!("{:.0}%", agent.health_score);
            let health_color = self.health_score_color(agent.health_score);

            acc.push_str(style_helpers::text());
            acc.push_str(&format!(
                "{:<width_id$} ",
                truncate(&agent.id, col_widths.id.saturating_sub(1)),
                width_id = col_widths.id
            ));
            acc.push_str(&format!(
                "{:<width_state$} ",
                truncate(&agent.state, col_widths.state.saturating_sub(1)),
                width_state = col_widths.state
            ));
            acc.push_str(health_color);
            acc.push_str(&format!(
                "{:>width_health$} ",
                health_display,
                width_health = col_widths.health
            ));
            acc.push_str(style_helpers::text());
            acc.push_str(&format!(
                "{:>width_beads$}\n",
                agent.beads_completed,
                width_beads = col_widths.beads
            ));
            acc
        });

        format!("{header}{pool_summary}{headers}{agent_rows}")
    }

    /// Calculate column widths for agent view based on pane width
    fn calculate_agent_column_widths(&self, max_width: usize) -> AgentColumnWidths {
        let state_width = 8;
        let health_width = 7;
        let beads_width = 6;
        let spacing = 3;

        let fixed_width = state_width + health_width + beads_width + spacing;
        let id_width = max_width.saturating_sub(fixed_width).max(8);

        AgentColumnWidths {
            id: id_width,
            state: state_width,
            health: health_width,
            beads: beads_width,
        }
    }

    /// Get color code for health score
    fn health_score_color(&self, score: f64) -> &'static str {
        match score {
            s if s >= 90.0 => "\x1b[32m",
            s if s >= 70.0 => "\x1b[33m",
            s if s >= 50.0 => "\x1b[35m",
            _ => "\x1b[31m",
        }
    }

    /// Render BeadDetail metadata section for displaying detailed bead information
    ///
    /// Renders the metadata section of a BeadDetail including:
    /// - ID, title, description
    /// - State, priority, issue type, workflow
    /// - Labels and dependencies
    ///
    /// # Arguments
    ///
    /// * `pane` - Pane configuration for width calculation
    /// * `bead` - BeadDetail containing the metadata to render
    ///
    /// # Returns
    ///
    /// Formatted string with bead metadata section
    #[must_use]
    pub fn render_bead_detail_metadata(&self, pane: &Pane, bead: &BeadDetail) -> String {
        let max_width = pane.width.saturating_sub(4);

        let header = format!(
            "{}{}\n\x1b[0m",
            style_helpers::header(),
            truncate(&bead.title, max_width)
        );

        let priority_label = format!("P{}", bead.priority);

        let fields = [
            ("ID", bead.id.as_str()),
            ("State", bead.state.as_str()),
            ("Priority", priority_label.as_str()),
            ("Type", bead.issue_type.as_str()),
            ("Workflow", bead.workflow_id.as_str()),
        ];

        let field_lines = fields
            .iter()
            .fold(String::new(), |mut acc, (label, value)| {
                acc.push_str(style_helpers::label());
                acc.push_str(&format!("{:<9} ", label));
                acc.push_str(style_helpers::text());
                acc.push_str(value);
                acc.push('\n');
                acc
            });

        let description_line = if !bead.description.is_empty() {
            let truncated_desc = truncate(&bead.description, max_width.saturating_sub(12));
            format!(
                "{}Description: {}{}\n",
                style_helpers::label(),
                style_helpers::text(),
                truncated_desc
            )
        } else {
            String::new()
        };

        let labels_line = if bead.labels.is_empty() {
            format!(
                "{}Labels:    {}none\n",
                style_helpers::label(),
                style_helpers::text()
            )
        } else {
            let labels_str = bead.labels.join(", ");
            format!(
                "{}Labels:    {}{}\n",
                style_helpers::label(),
                style_helpers::text(),
                truncate(&labels_str, max_width.saturating_sub(12))
            )
        };

        let deps_line = if bead.dependencies.is_empty() {
            format!(
                "{}Deps:      {}none\n",
                style_helpers::label(),
                style_helpers::text()
            )
        } else {
            let deps_str = bead.dependencies.join(", ");
            format!(
                "{}Deps:      {}{}\n",
                style_helpers::label(),
                style_helpers::text(),
                truncate(&deps_str, max_width.saturating_sub(12))
            )
        };

        format!(
            "{}\n{}{}{}{}\x1b[0m",
            header, field_lines, description_line, labels_line, deps_line
        )
    }

    /// Render status bar at bottom of screen
    fn render_status_bar(&self, focused_pane: PaneType, status_message: Option<&str>) -> String {
        let border_line = "─".repeat(78);

        // Focus indicator
        let focus_text = match focused_pane {
            PaneType::BeadList => "Beads",
            PaneType::BeadDetail => "Details",
            PaneType::WorkflowGraph => "Graph",
            PaneType::PipelineView => "Pipeline",
            PaneType::AgentView => "Agents",
        };

        // Status message using map_or for default
        let msg = status_message.map_or("↑↓: navigate | Enter: focus | ?: help", |s| s);

        // Build content line
        let content = format!(" Focus: {:<8} {msg}", focus_text);
        let content_len = content.chars().count();
        let padding = " ".repeat(80_usize.saturating_sub(content_len).saturating_sub(1));

        // Assemble complete status bar
        format!(
            "{}┌{border_line}┐\n│{content}{padding}│\n└{border_line}┘\x1b[0m\n",
            style_helpers::border_normal()
        )
    }

    /// Render help overlay
    ///
    /// # Arguments
    ///
    /// * `rows` - Terminal rows
    /// * `cols` - Terminal columns
    /// * `keybindings` - Keybindings for current pane
    /// * `focused_pane` - Currently focused pane type
    ///
    /// # Returns
    ///
    /// Help overlay content or error if terminal too small
    ///
    /// # Errors
    ///
    /// Returns `Err(HelpOverlayError::TerminalTooSmall)` if terminal is below minimum size
    pub fn render_help_overlay(
        &self,
        rows: usize,
        cols: usize,
        keybindings: &[(char, &str)],
        focused_pane: PaneType,
    ) -> HelpOverlayResult<String> {
        const MIN_ROWS: usize = 10;
        const MIN_COLS: usize = 40;

        if rows < MIN_ROWS || cols < MIN_COLS {
            return Err(HelpOverlayError::TerminalTooSmall { rows, cols });
        }

        let width = cols.min(80);
        let height = rows.min(25);

        // Render top border
        let top_border = self.render_overlay_top_border(width, " Help ");

        // Focus indicator
        let focus_text = match focused_pane {
            PaneType::BeadList => "Bead List",
            PaneType::BeadDetail => "Bead Details",
            PaneType::WorkflowGraph => "Workflow Graph",
            PaneType::PipelineView => "Pipeline View",
            PaneType::AgentView => "Agent View",
        };

        // Build all content lines using functional patterns
        // Create iterator of static content lines
        let static_lines = [
            "Global Keys:",
            "  ?      Show/hide this help",
            "  q      Quit",
            "  Tab    Switch focus between panes",
            "",
        ]
        .iter()
        .map(|s| (*s).to_string());

        // Create iterator of focus section
        let focus_lines = [
            format!("Current Focus: {focus_text}"),
            "".to_string(),
            "Keybindings for current pane:".to_string(),
        ]
        .into_iter();

        // Combine all lines: static + focus + keybindings
        let all_lines: Vec<String> = static_lines
            .chain(focus_lines)
            .chain(
                keybindings
                    .iter()
                    .map(|(key, desc)| format!("  {key}      {desc}")),
            )
            .collect();

        let content_height = all_lines.len();
        let padding_top = height.saturating_sub(content_height.saturating_add(4)) / 2;
        let padding_bottom =
            height.saturating_sub(content_height.saturating_add(4).saturating_add(padding_top));
        let inner_width = width.saturating_sub(2);
        let overlay_style = style_helpers::overlay();

        // Helper to render padding lines using repeat
        let padding_line = format!("{overlay_style}│{}│\n", " ".repeat(inner_width));
        let top_padding = padding_line.repeat(padding_top);

        // Render content lines using functional fold pattern
        let content_lines = all_lines.iter().fold(String::new(), |mut acc, line| {
            let padding =
                " ".repeat(width.saturating_sub(2_usize.saturating_add(line.chars().count())));
            acc.push_str(overlay_style);
            acc.push_str("│ ");
            acc.push_str(line);
            acc.push_str(&padding);
            acc.push_str("│\n");
            acc
        });

        let bottom_padding = padding_line.repeat(padding_bottom);

        // Assemble complete overlay
        let bottom_border = self.render_overlay_bottom_border(width);
        Ok(format!(
            "{overlay_style}{top_border}{top_padding}{content_lines}{bottom_padding}{bottom_border}\x1b[0m"
        ))
    }

    /// Render overlay top border
    fn render_overlay_top_border(&self, width: usize, title: &str) -> String {
        let title_len = title.chars().count();
        let remaining = width.saturating_sub(2).saturating_sub(title_len);
        let border_line = "═".repeat(remaining);

        format!("╔{title}{border_line}╗\n")
    }

    /// Render overlay bottom border
    fn render_overlay_bottom_border(&self, width: usize) -> String {
        let border_line = "═".repeat(width.saturating_sub(2));
        format!("╚{border_line}╝")
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate text to fit width using functional patterns.
///
/// Uses char_indices for byte-efficient truncation without collecting into Vec.
#[must_use]
fn truncate(text: &str, width: usize) -> String {
    // Handle edge case where text already fits
    if text.chars().count() <= width {
        return text.to_string();
    }

    // If width is too small for meaningful truncation with ellipsis
    if width <= 3 {
        return "...".to_string();
    }

    // Find the byte position at which to truncate (leave room for "...")
    let target_chars = width.saturating_sub(3);
    let byte_pos = text.char_indices().map(|(pos, _)| pos).nth(target_chars);

    match byte_pos {
        Some(pos) => format!("{}...", &text[..pos]),
        None => text.to_string(), // Shouldn't happen if count > width
    }
}

fn stage_symbol(stage: &str) -> &'static str {
    match stage {
        "research" => "🔍",
        "plan" => "📋",
        "implement" => "◇",
        "review" => "◌",
        "validate" => "◎",
        "accept" => "✓",
        _ => "•",
    }
}

fn render_progress_bar(progress: f32, width: usize) -> String {
    let clamped = progress.clamp(0.0, 1.0);
    let filled = ((clamped * width as f32).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    let percentage = (clamped * 100.0).round() as usize;

    // Build bar using functional patterns: repeat chars and collect
    let filled_part = "█".repeat(filled);
    let empty_part = "░".repeat(empty);

    format!("[{}{}] {}%", filled_part, empty_part, percentage)
}

/// Extract stage lifecycle information from a task.
///
/// Returns (running_stage_index, failed_stage_index, is_completed) tuple.
/// All indices are Option<usize> representing positions in the stages vector.
/// Uses functional patterns: find_position with iterator combinators.
#[must_use]
fn get_stage_info(task: &TaskRow) -> (Option<usize>, Option<usize>, bool) {
    let is_completed = matches!(task.status.as_str(), "passed" | "integrated");

    // Use iterator find_position for running stage
    let running_stage = task
        .stages
        .iter()
        .position(|s| matches!(s.state, StageState::Running));

    // Use iterator find_position for failed stage
    let failed_stage = task
        .stages
        .iter()
        .position(|s| matches!(s.state, StageState::Failed));

    (running_stage, failed_stage, is_completed)
}

/// Calculate progress for a specific stage in the pipeline.
///
/// Uses functional patterns with match expressions instead of imperative logic.
/// Returns f32 between 0.0 and 1.0 representing completion percentage.
#[must_use]
fn calculate_stage_progress(
    stages: &[crate::plugin::StageInfo],
    running_stage: Option<usize>,
    failed_stage: Option<usize>,
    is_completed: bool,
) -> f32 {
    // Total number of stages for percentage calculation
    let total_stages = stages.len();

    // Calculate completed count using iterator combinators
    let completed_count = stages
        .iter()
        .filter(|s| matches!(s.state, StageState::Completed))
        .count();

    // Functional pattern: nested map_or_else for clean composition
    let base_progress = if is_completed {
        1.0
    } else if let Some(failed_idx) = failed_stage {
        // All stages up to and including failed are "done" (even if failed)
        failed_idx.saturating_add(1) as f32 / total_stages.max(1) as f32
    } else if running_stage.is_some() {
        // Completed stages + 0.5 for the running stage
        (completed_count as f32 + 0.5) / total_stages.max(1) as f32
    } else {
        // Only completed stages contribute
        completed_count as f32 / total_stages.max(1) as f32
    };

    base_progress.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{AgentMetrics, PoolMetrics};
    use rpds::Vector;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new();
        assert!(renderer.use_colors);
    }

    #[test]
    fn test_disable_colors() {
        let mut renderer = Renderer::new();
        renderer.disable_colors();
        assert!(!renderer.use_colors);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello world", 5), "he...");
        assert_eq!(truncate("hi", 10), "hi");
        assert_eq!(truncate("hello", 3), "...");
    }

    #[test]
    fn test_render_top_border() {
        let renderer = Renderer::new();
        let border = renderer.render_top_border(20, "Test");
        assert!(border.starts_with('┌'));
        assert!(border.ends_with("┐\n"));
    }

    #[test]
    fn test_render_bottom_border() {
        let renderer = Renderer::new();
        let border = renderer.render_bottom_border(20);
        assert!(border.starts_with('└'));
        assert!(border.ends_with("┘"));
    }

    #[test]
    fn test_render_agent_view_empty() {
        let renderer = Renderer::new();
        let pane = Pane::new(PaneType::AgentView, 1, 1, 10, 40).expect("Failed to create pane");
        let agents = Vector::new();
        let pool = PoolMetrics {
            total: 0,
            idle: 0,
            working: 0,
            unhealthy: 0,
            shutting_down: 0,
            terminated: 0,
        };

        let output = renderer.render_agent_view(&pane, &agents, &pool);

        assert!(output.contains("Agent Pool Status"));
        assert!(output.contains("Total: 0"));
        assert!(output.contains("No agents connected"));
    }

    #[test]
    fn test_render_agent_view_with_agents() {
        let renderer = Renderer::new();
        let pane = Pane::new(PaneType::AgentView, 1, 1, 15, 60).expect("Failed to create pane");

        let agents = Vector::from_iter(vec![
            AgentMetrics {
                id: "agent-001".to_string(),
                state: "working".to_string(),
                uptime_secs: 3600,
                beads_completed: 10,
                operations_executed: 50,
                avg_execution_secs: Some(1.5),
                health_score: 95.0,
            },
            AgentMetrics {
                id: "agent-002".to_string(),
                state: "idle".to_string(),
                uptime_secs: 7200,
                beads_completed: 25,
                operations_executed: 100,
                avg_execution_secs: Some(2.0),
                health_score: 100.0,
            },
        ]);

        let pool = PoolMetrics {
            total: 2,
            idle: 1,
            working: 1,
            unhealthy: 0,
            shutting_down: 0,
            terminated: 0,
        };

        let output = renderer.render_agent_view(&pane, &agents, &pool);

        assert!(output.contains("agent-001"));
        assert!(output.contains("agent-002"));
        assert!(output.contains("working"));
        assert!(output.contains("idle"));
        assert!(output.contains("Total: 2"));
        assert!(output.contains("Idle: 1"));
    }

    #[test]
    fn test_render_agent_view_health_status() {
        let renderer = Renderer::new();
        let pane = Pane::new(PaneType::AgentView, 1, 1, 10, 50).expect("Failed to create pane");

        let agents = Vector::from_iter(vec![
            AgentMetrics {
                id: "healthy-agent".to_string(),
                state: "working".to_string(),
                uptime_secs: 3600,
                beads_completed: 10,
                operations_executed: 50,
                avg_execution_secs: Some(1.5),
                health_score: 100.0,
            },
            AgentMetrics {
                id: "degraded-agent".to_string(),
                state: "working".to_string(),
                uptime_secs: 3600,
                beads_completed: 5,
                operations_executed: 20,
                avg_execution_secs: Some(1.5),
                health_score: 50.0,
            },
        ]);

        let pool = PoolMetrics {
            total: 2,
            idle: 0,
            working: 2,
            unhealthy: 1,
            shutting_down: 0,
            terminated: 0,
        };

        let output = renderer.render_agent_view(&pane, &agents, &pool);

        assert!(output.contains("healthy-agent"));
        assert!(output.contains("degraded-agent"));
        assert!(output.contains("100%"));
        assert!(output.contains("50%"));
    }

    #[test]
    fn test_render_agent_view_pool_summary() {
        let renderer = Renderer::new();
        let pane = Pane::new(PaneType::AgentView, 1, 1, 10, 70).expect("Failed to create pane");

        let agents = Vector::new();
        let pool = PoolMetrics {
            total: 8,
            idle: 3,
            working: 4,
            unhealthy: 1,
            shutting_down: 0,
            terminated: 0,
        };

        let output = renderer.render_agent_view(&pane, &agents, &pool);

        assert!(output.contains("Total: 8"));
        assert!(output.contains("Idle: 3"));
        assert!(output.contains("Working: 4"));
        assert!(output.contains("Unhealthy: 1"));
    }
}
