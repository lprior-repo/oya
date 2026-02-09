// Layout module - Terminal layout system for OYA UI
//
// Defines the 3-pane layout structure and pane types for the plugin.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Errors that can occur in layout operations
#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("Invalid pane dimensions: {0}")]
    InvalidDimensions(String),

    #[error("Pane not found: {0}")]
    PaneNotFound(String),

    #[error("Layout calculation error: {0}")]
    CalculationError(String),
}

/// Result type for layout operations
pub type LayoutResult<T> = Result<T, LayoutError>;

/// Types of panes in the OYA UI layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneType {
    /// Left pane showing list of beads
    BeadList,
    /// Right top pane showing bead details
    BeadDetail,
    /// Right top pane showing pipeline progress
    PipelineView,
    /// Bottom pane showing workflow graph
    WorkflowGraph,
}

impl fmt::Display for PaneType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaneType::BeadList => write!(f, "Bead List"),
            PaneType::BeadDetail => write!(f, "Bead Detail"),
            PaneType::PipelineView => write!(f, "Pipeline View"),
            PaneType::WorkflowGraph => write!(f, "Workflow Graph"),
        }
    }
}

/// A pane in the terminal layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pane {
    /// Pane type
    pub pane_type: PaneType,
    /// Row position (0-indexed)
    pub row: usize,
    /// Column position (0-indexed)
    pub col: usize,
    /// Height in rows
    pub height: usize,
    /// Width in columns
    pub width: usize,
    /// Pane title
    pub title: String,
}

impl Pane {
    /// Create a new pane
    ///
    /// # Arguments
    ///
    /// * `pane_type` - Type of pane
    /// * `row` - Row position
    /// * `col` - Column position
    /// * `height` - Height in rows
    /// * `width` - Width in columns
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are invalid
    pub fn new(
        pane_type: PaneType,
        row: usize,
        col: usize,
        height: usize,
        width: usize,
    ) -> LayoutResult<Self> {
        if height == 0 || width == 0 {
            return Err(LayoutError::InvalidDimensions(
                "Pane dimensions must be positive".to_string(),
            ));
        }

        let title = match pane_type {
            PaneType::BeadList => "Beads",
            PaneType::BeadDetail => "Details",
            PaneType::PipelineView => "Pipeline",
            PaneType::WorkflowGraph => "Workflow",
        }
        .to_string();

        Ok(Self {
            pane_type,
            row,
            col,
            height,
            width,
            title,
        })
    }

    /// Create a new pane with hardcoded default values (internal use only)
    ///
    /// # Panics
    ///
    /// Panics if the provided hardcoded dimensions are invalid (should never happen)
    #[expect(clippy::expect_used)]
    fn with_defaults(
        pane_type: PaneType,
        row: usize,
        col: usize,
        height: usize,
        width: usize,
    ) -> Self {
        Self::new(pane_type, row, col, height, width)
            .expect("Hardcoded default pane dimensions should be valid")
    }

    /// Get the right boundary column
    #[must_use]
    pub const fn right(&self) -> usize {
        self.col + self.width
    }

    /// Get the bottom boundary row
    #[must_use]
    pub const fn bottom(&self) -> usize {
        self.row + self.height
    }
}

/// Terminal layout configuration
///
/// Implements a 3-pane layout:
/// ┌─────────────────────────────────┐
/// │ BeadList      │ BeadDetail       │
/// │               ├─────────────────┤
/// │               │ PipelineView     │
/// ├───────────────┴─────────────────┤
/// │ WorkflowGraph                   │
/// └─────────────────────────────────┘
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    /// All panes in the layout
    panes: Vec<Pane>,
}

impl Layout {
    /// Create a new 3-pane layout
    ///
    /// # Returns
    ///
    /// A new layout with default 3-pane configuration
    #[must_use]
    pub fn new_3_pane() -> Self {
        let bead_list = Pane::with_defaults(PaneType::BeadList, 1, 1, 15, 32);
        let bead_detail = Pane::with_defaults(PaneType::BeadDetail, 1, 34, 8, 45);
        let pipeline_view = Pane::with_defaults(PaneType::PipelineView, 10, 34, 6, 45);
        let workflow_graph = Pane::with_defaults(PaneType::WorkflowGraph, 17, 1, 6, 78);

        Self {
            panes: vec![bead_list, bead_detail, pipeline_view, workflow_graph],
        }
    }

    /// Get all panes in the layout
    #[must_use]
    pub const fn panes(&self) -> &Vec<Pane> {
        &self.panes
    }

    /// Get a pane by type
    ///
    /// # Arguments
    ///
    /// * `pane_type` - Type of pane to find
    ///
    /// # Returns
    ///
    /// Reference to the pane if found
    pub fn get_pane(&self, pane_type: PaneType) -> Option<&Pane> {
        self.panes.iter().find(|p| p.pane_type == pane_type)
    }

    /// Calculate layout for a given terminal size
    ///
    /// # Arguments
    ///
    /// * `rows` - Total rows in terminal
    /// * `cols` - Total columns in terminal
    ///
    /// # Returns
    ///
    /// A new layout adjusted for the terminal size
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal is too small or pane creation fails
    pub fn calculate_for_terminal(rows: usize, cols: usize) -> LayoutResult<Self> {
        if rows < 20 || cols < 40 {
            return Err(LayoutError::InvalidDimensions(
                "Terminal must be at least 20 rows x 40 columns".to_string(),
            ));
        }

        // Calculate dimensions based on terminal size
        // Left pane: 40% width
        let left_width = cols.saturating_mul(40).saturating_div(100);
        // Right panes: 60% width
        let right_width = cols.saturating_sub(left_width).saturating_sub(3); // -3 for borders
        // Top panes: 60% height
        let top_height = rows.saturating_mul(60).saturating_div(100);
        // Bottom pane: 40% height
        let bottom_height = rows.saturating_sub(top_height).saturating_sub(3); // -3 for borders

        // Split right top into bead detail (60%) and pipeline view (40%)
        let bead_detail_height = top_height.saturating_mul(60).saturating_div(100);
        let pipeline_height = top_height
            .saturating_sub(bead_detail_height)
            .saturating_sub(2);

        // Validate calculated dimensions before creating panes
        let bead_list_height = top_height.saturating_sub(1);
        if bead_list_height == 0 || left_width == 0 {
            return Err(LayoutError::CalculationError(
                "Calculated bead list dimensions are invalid".to_string(),
            ));
        }

        let bead_list = Pane::new(
            PaneType::BeadList,
            1,
            1,
            bead_list_height,
            left_width,
        )?;

        if bead_detail_height == 0 || right_width == 0 {
            return Err(LayoutError::CalculationError(
                "Calculated bead detail dimensions are invalid".to_string(),
            ));
        }

        let bead_detail = Pane::new(
            PaneType::BeadDetail,
            1,
            left_width.saturating_add(3),
            bead_detail_height,
            right_width,
        )?;

        if pipeline_height == 0 {
            return Err(LayoutError::CalculationError(
                "Calculated pipeline view dimensions are invalid".to_string(),
            ));
        }

        let pipeline_view = Pane::new(
            PaneType::PipelineView,
            bead_detail_height.saturating_add(3),
            left_width.saturating_add(3),
            pipeline_height,
            right_width,
        )?;

        if bottom_height == 0 {
            return Err(LayoutError::CalculationError(
                "Calculated workflow graph dimensions are invalid".to_string(),
            ));
        }

        let workflow_graph = Pane::new(
            PaneType::WorkflowGraph,
            top_height.saturating_add(3),
            1,
            bottom_height,
            cols,
        )?;

        Ok(Self {
            panes: vec![bead_list, bead_detail, pipeline_view, workflow_graph],
        })
    }

    /// Validate the layout
    ///
    /// # Returns
    ///
    /// Ok if the layout is valid, Err otherwise
    pub fn validate(&self) -> LayoutResult<()> {
        for pane in &self.panes {
            if pane.height == 0 || pane.width == 0 {
                return Err(LayoutError::InvalidDimensions(format!(
                    "Pane {} has invalid dimensions",
                    pane.pane_type
                )));
            }
        }
        Ok(())
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new_3_pane()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn test_layout_creation() {
        let layout = Layout::new_3_pane();
        assert_eq!(layout.panes.len(), 4);
    }

    #[test]
    fn test_get_pane() {
        let layout = Layout::new_3_pane();
        let bead_list = layout.get_pane(PaneType::BeadList);
        assert!(bead_list.is_some());
        assert_eq!(bead_list.unwrap().pane_type, PaneType::BeadList);
    }

    #[test]
    fn test_calculate_for_terminal() {
        let layout = Layout::calculate_for_terminal(24, 80).expect("Failed to calculate layout");
        assert_eq!(layout.panes.len(), 4);

        let bead_list = layout
            .get_pane(PaneType::BeadList)
            .expect("BeadList not found");
        assert_eq!(bead_list.width, 32); // 40% of 80

        let workflow_graph = layout
            .get_pane(PaneType::WorkflowGraph)
            .expect("WorkflowGraph not found");
        assert!(workflow_graph.height > 0);
    }

    #[test]
    fn test_calculate_too_small() {
        let result = Layout::calculate_for_terminal(10, 20);
        assert!(result.is_err());
    }

    #[test]
    fn test_pane_boundaries() {
        let pane = Pane::new(PaneType::BeadList, 5, 10, 15, 30).expect("Failed to create pane");
        assert_eq!(pane.right(), 40);
        assert_eq!(pane.bottom(), 20);
    }

    #[test]
    fn test_validate_layout() {
        let layout = Layout::new_3_pane();
        assert!(layout.validate().is_ok());
    }

    #[test]
    fn test_pane_display() {
        assert_eq!(PaneType::BeadList.to_string(), "Bead List");
        assert_eq!(PaneType::BeadDetail.to_string(), "Bead Detail");
        assert_eq!(PaneType::PipelineView.to_string(), "Pipeline View");
        assert_eq!(PaneType::WorkflowGraph.to_string(), "Workflow Graph");
    }

    #[test]
    fn test_invalid_pane_dimensions() {
        let result = Pane::new(PaneType::BeadList, 0, 0, 0, 10);
        assert!(result.is_err());
    }
}
