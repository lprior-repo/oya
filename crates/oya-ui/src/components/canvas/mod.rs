//! Canvas component and initialization

pub mod clear;
pub mod context;
pub mod coords;
pub mod init;
pub mod resize;

pub use init::{create_canvas, CanvasConfig};
pub use resize::{calculate_canvas_size, get_window_size, resize_canvas, ResizeConfig};
