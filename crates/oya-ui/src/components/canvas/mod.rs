//! Canvas component and initialization

pub mod clear;
pub mod context;
pub mod coords;
pub mod dpi;
pub mod edge_arrows;
pub mod init;
pub mod node_labels;
pub mod node_shapes;
pub mod raf;
pub mod resize;

pub use dpi::{apply_dpi_scaling, detect_device_pixel_ratio, setup_dpi_aware_canvas};
pub use edge_arrows::{ArrowConfig, ArrowError, ArrowPath, ArrowStyle, calculate_arrow_head, edge_direction};
pub use init::{CanvasConfig, create_canvas};
pub use node_labels::{calculate_label_position, render_node_label, truncate_text};
pub use node_shapes::{darken_color, render_node};
pub use raf::{
    AnimationHandle, AnimationState, FrameTiming, RafError, start_animation_loop,
    start_canvas_animation_loop,
};
pub use resize::{ResizeConfig, calculate_canvas_size, get_window_size, resize_canvas};
