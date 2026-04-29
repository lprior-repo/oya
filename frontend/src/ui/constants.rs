//! Shared UI constants for node geometry, viewport, and interaction parameters.
//!
//! All numeric literals that describe node dimensions, canvas defaults, zoom
//! behaviour, drag thresholds, and edge-auto-pan parameters live here so that
//! every module references the same canonical values.

// ---------------------------------------------------------------------------
// Node geometry
// ---------------------------------------------------------------------------

/// Visual width of a flow node in pixels.
pub const NODE_WIDTH: f32 = 220.0;

/// Visual height of a flow node in pixels.
pub const NODE_HEIGHT: f32 = 68.0;

/// Half the node width; horizontal offset from the node origin to its centre.
/// Equal to `NODE_WIDTH / 2.0`.
pub const NODE_CENTER_X_OFFSET: f32 = NODE_WIDTH / 2.0;

/// Half the node height; vertical offset from the node origin to the handle
/// centre-line.  Equal to `NODE_HEIGHT / 2.0`.
pub const NODE_HANDLE_Y_OFFSET: f32 = NODE_HEIGHT / 2.0;

/// Corner radius for smooth-step edge paths (pixels).
pub const EDGE_CORNER_RADIUS: f32 = 8.0;

// ---------------------------------------------------------------------------
// Default canvas / viewport dimensions
// ---------------------------------------------------------------------------

/// Assumed canvas width when the real size is unavailable.
pub const DEFAULT_CANVAS_WIDTH: f32 = 1280.0;

/// Assumed canvas height when the real size is unavailable.
pub const DEFAULT_CANVAS_HEIGHT: f32 = 760.0;

/// Horizontal centre of the default canvas (`DEFAULT_CANVAS_WIDTH / 2.0`).
pub const ZOOM_CENTER_X: f32 = DEFAULT_CANVAS_WIDTH / 2.0;

/// Vertical centre of the default canvas (`DEFAULT_CANVAS_HEIGHT / 2.0`).
pub const ZOOM_CENTER_Y: f32 = DEFAULT_CANVAS_HEIGHT / 2.0;

// ---------------------------------------------------------------------------
// Zoom / pan
// ---------------------------------------------------------------------------

/// Magnitude of a single zoom step (applied as +/- to the current zoom).
pub const ZOOM_DELTA: f32 = 0.12;

/// Padding added around node bounds when fitting the viewport.
pub const FIT_VIEW_PADDING: f32 = 200.0;

/// Distance an arrow-key press moves the selected node (pixels).
pub const ARROW_KEY_DELTA: f32 = 20.0;

// ---------------------------------------------------------------------------
// Drag / interaction
// ---------------------------------------------------------------------------

/// Minimum pixel movement before a click becomes a drag.
pub const DRAG_THRESHOLD_PX: f32 = 4.0;

/// Width of the edge zone (in pixels) that triggers auto-panning while
/// dragging a node towards the canvas boundary.
pub const EDGE_AUTO_PAN_ZONE: f32 = 56.0;

/// Maximum auto-pan speed (pixels per mouse-move event) inside the edge zone.
pub const EDGE_AUTO_PAN_MAX: f32 = 18.0;

/// Fallback canvas dimensions when `app_io::canvas_rect_size()` returns
/// `None` (non-WASM or element not yet mounted).
pub const FALLBACK_CANVAS_WIDTH: f32 = 960.0;
pub const FALLBACK_CANVAS_HEIGHT: f32 = 720.0;
