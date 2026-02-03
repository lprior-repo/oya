//! Pan state management for DAG viewport
//!
//! This module provides functional, panic-free pan state management with proper
//! delta tracking and bounds validation. All operations follow the Railway-Oriented
//! Programming pattern with Result types.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

/// Type-safe pan offset in pixels
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanOffset {
    x: f64,
    y: f64,
}

impl PanOffset {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn x(&self) -> f64 {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> f64 {
        self.y
    }

    #[must_use]
    pub const fn add_delta(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    #[must_use]
    pub fn clamp(self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            x: self.x.clamp(min_x, max_x),
            y: self.y.clamp(min_y, max_y),
        }
    }
}

impl Default for PanOffset {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// Drag state for pan operations
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DragState {
    #[default]
    Idle,
    Dragging {
        start_x: f64,
        start_y: f64,
        current_x: f64,
        current_y: f64,
    },
}

#[must_use]
pub const fn start_drag(_state: DragState, x: f64, y: f64) -> DragState {
    DragState::Dragging {
        start_x: x,
        start_y: y,
        current_x: x,
        current_y: y,
    }
}

#[must_use]
pub const fn update_drag(state: DragState, x: f64, y: f64) -> DragState {
    match state {
        DragState::Idle => DragState::Idle,
        DragState::Dragging {
            start_x, start_y, ..
        } => DragState::Dragging {
            start_x,
            start_y,
            current_x: x,
            current_y: y,
        },
    }
}

#[must_use]
pub const fn end_drag(_state: DragState) -> DragState {
    DragState::Idle
}

#[must_use]
pub const fn get_drag_delta(state: DragState) -> Option<(f64, f64)> {
    match state {
        DragState::Idle => None,
        DragState::Dragging {
            start_x,
            start_y,
            current_x,
            current_y,
        } => Some((current_x - start_x, current_y - start_y)),
    }
}

/// Applies a pan delta to the current offset with optional bounds checking
///
/// # Errors
/// Returns an error if the provided bounds are invalid (min > max for either axis)
pub fn apply_pan_delta(
    offset: PanOffset,
    dx: f64,
    dy: f64,
    bounds: Option<(f64, f64, f64, f64)>,
) -> Result<PanOffset, String> {
    let new_offset = offset.add_delta(dx, dy);

    match bounds {
        Some((min_x, min_y, max_x, max_y)) => {
            if min_x > max_x {
                return Err(format!(
                    "Invalid X bounds: min ({min_x}) > max ({max_x})"
                ));
            }
            if min_y > max_y {
                return Err(format!(
                    "Invalid Y bounds: min ({min_y}) > max ({max_y})"
                ));
            }

            Ok(new_offset.clamp(min_x, min_y, max_x, max_y))
        }
        None => Ok(new_offset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pan_offset_new() {
        let offset = PanOffset::new(100.0, 200.0);
        assert!((offset.x() - 100.0).abs() < 1e-10);
        assert!((offset.y() - 200.0).abs() < 1e-10);
    }

    #[test]
    fn test_drag_state_default() {
        let state = DragState::default();
        assert!(matches!(state, DragState::Idle));
    }

    #[test]
    fn test_start_drag() {
        let state = DragState::Idle;
        let dragging = start_drag(state, 150.0, 250.0);

        if let DragState::Dragging {
            start_x,
            start_y,
            current_x,
            current_y,
        } = dragging
        {
            assert!((start_x - 150.0).abs() < 1e-10);
            assert!((start_y - 250.0).abs() < 1e-10);
            assert!((current_x - 150.0).abs() < 1e-10);
            assert!((current_y - 250.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_get_drag_delta() {
        let state = DragState::Dragging {
            start_x: 100.0,
            start_y: 200.0,
            current_x: 150.0,
            current_y: 250.0,
        };

        let delta = get_drag_delta(state);
        assert!(delta.is_some());
        if let Some((dx, dy)) = delta {
            assert!((dx - 50.0).abs() < 1e-10);
            assert!((dy - 50.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_apply_pan_delta() {
        let offset = PanOffset::new(100.0, 200.0);
        let result = apply_pan_delta(offset, 50.0, -30.0, None);

        assert!(result.is_ok());
        if let Ok(new_offset) = result {
            assert!((new_offset.x() - 150.0).abs() < 1e-10);
            assert!((new_offset.y() - 170.0).abs() < 1e-10);
        }
    }
}
