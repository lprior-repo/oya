//! Canvas component and initialization

pub mod clear;
pub mod context;
pub mod coords;
pub mod init;
pub mod node_labels;
pub mod node_shapes;
pub mod resize;

pub use init::{CanvasConfig, create_canvas};
pub use node_labels::{calculate_label_position, render_node_label, truncate_text};
pub use node_shapes::{darken_color, render_node};
pub use resize::{ResizeConfig, calculate_canvas_size, get_window_size, resize_canvas};
