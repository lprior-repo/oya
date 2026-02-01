//! Color mapping for node states

use super::node::NodeState;

/// RGB color representation (0-255 per channel)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert to CSS rgb() string
    pub fn to_css(&self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    /// Convert to hex string (#RRGGBB)
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// Map NodeState to visual color
///
/// Pure function - always returns same color for same state.
/// Colors chosen for accessibility and semantic clarity:
/// - Idle: Light gray (neutral)
/// - Running: Blue (active)
/// - Blocked: Orange (warning)
/// - Completed: Green (success)
/// - Failed: Red (error)
pub fn get_node_color(state: &NodeState) -> RgbColor {
    match state {
        NodeState::Idle => RgbColor::new(200, 200, 200),      // Light gray
        NodeState::Running => RgbColor::new(52, 152, 219),    // Blue
        NodeState::Blocked => RgbColor::new(230, 126, 34),    // Orange
        NodeState::Completed => RgbColor::new(46, 204, 113),  // Green
        NodeState::Failed => RgbColor::new(231, 76, 60),      // Red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_color_deterministic() {
        let state = NodeState::Running;
        let color1 = get_node_color(&state);
        let color2 = get_node_color(&state);
        assert_eq!(color1, color2); // Pure function
    }

    #[test]
    fn test_all_states_have_unique_colors() {
        let idle = get_node_color(&NodeState::Idle);
        let running = get_node_color(&NodeState::Running);
        let blocked = get_node_color(&NodeState::Blocked);
        let completed = get_node_color(&NodeState::Completed);
        let failed = get_node_color(&NodeState::Failed);

        // All should be unique
        assert_ne!(idle, running);
        assert_ne!(idle, blocked);
        assert_ne!(idle, completed);
        assert_ne!(idle, failed);
        assert_ne!(running, blocked);
        assert_ne!(running, completed);
        assert_ne!(running, failed);
        assert_ne!(blocked, completed);
        assert_ne!(blocked, failed);
        assert_ne!(completed, failed);
    }

    #[test]
    fn test_css_output() {
        let color = RgbColor::new(52, 152, 219);
        assert_eq!(color.to_css(), "rgb(52, 152, 219)");
    }

    #[test]
    fn test_hex_output() {
        let color = RgbColor::new(52, 152, 219);
        assert_eq!(color.to_hex(), "#3498DB");
    }

    #[test]
    fn test_color_values_are_valid_rgb() {
        // Test all states produce valid RGB values (0-255)
        let states = [
            NodeState::Idle,
            NodeState::Running,
            NodeState::Blocked,
            NodeState::Completed,
            NodeState::Failed,
        ];

        for state in &states {
            let color = get_node_color(state);
            // All u8 values are valid (0-255), so just verify they exist
            assert!(color.r <= 255);
            assert!(color.g <= 255);
            assert!(color.b <= 255);
        }
    }

    #[test]
    fn test_exhaustive_match_coverage() {
        // Ensure all NodeState variants are covered
        let _: RgbColor = get_node_color(&NodeState::Idle);
        let _: RgbColor = get_node_color(&NodeState::Running);
        let _: RgbColor = get_node_color(&NodeState::Blocked);
        let _: RgbColor = get_node_color(&NodeState::Completed);
        let _: RgbColor = get_node_color(&NodeState::Failed);
    }

    #[test]
    fn test_rgb_color_construction() {
        let color = RgbColor::new(100, 150, 200);
        assert_eq!(color.r, 100);
        assert_eq!(color.g, 150);
        assert_eq!(color.b, 200);
    }

    #[test]
    fn test_css_output_all_states() {
        let idle = get_node_color(&NodeState::Idle);
        assert_eq!(idle.to_css(), "rgb(200, 200, 200)");

        let running = get_node_color(&NodeState::Running);
        assert_eq!(running.to_css(), "rgb(52, 152, 219)");

        let blocked = get_node_color(&NodeState::Blocked);
        assert_eq!(blocked.to_css(), "rgb(230, 126, 34)");

        let completed = get_node_color(&NodeState::Completed);
        assert_eq!(completed.to_css(), "rgb(46, 204, 113)");

        let failed = get_node_color(&NodeState::Failed);
        assert_eq!(failed.to_css(), "rgb(231, 76, 60)");
    }

    #[test]
    fn test_hex_output_all_states() {
        let idle = get_node_color(&NodeState::Idle);
        assert_eq!(idle.to_hex(), "#C8C8C8");

        let running = get_node_color(&NodeState::Running);
        assert_eq!(running.to_hex(), "#3498DB");

        let blocked = get_node_color(&NodeState::Blocked);
        assert_eq!(blocked.to_hex(), "#E67E22");

        let completed = get_node_color(&NodeState::Completed);
        assert_eq!(completed.to_hex(), "#2ECC71");

        let failed = get_node_color(&NodeState::Failed);
        assert_eq!(failed.to_hex(), "#E74C3C");
    }
}
