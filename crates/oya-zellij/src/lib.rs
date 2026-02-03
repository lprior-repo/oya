#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! OYA Zellij Plugin - Pipeline Orchestration Dashboard
//!
//! This plugin provides a real-time view of OYA pipeline status, bead execution,
//! and stage progress directly in your Zellij terminal.

use std::collections::BTreeMap;
use zellij_tile::prelude::*;

// Plugin state
#[derive(Default)]
struct State {
    // Mock bead data for now
    beads: Vec<String>,
    selected: usize,
}

// Required Zellij plugin trait
register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        // Initialize with mock data
        self.beads = vec![
            "src-1234: Implement user auth".to_string(),
            "src-5678: Add database migration".to_string(),
        ];

        // Subscribe to key events
        subscribe(&[EventType::Key]);
    }

    fn update(&mut self, event: Event) -> bool {
        // Handle events - for now just return false (no re-render)
        if matches!(event, Event::Key(_)) {
            // Trigger re-render on any key press
            return true;
        }
        false
    }

    fn render(&mut self, rows: usize, cols: usize) {
        // Simple header
        println!("{}", "\x1b[1mOYA Pipeline Dashboard\x1b[0m");
        println!("{}", "─".repeat(cols));

        // List beads
        println!("\nBeads:");
        for (idx, bead) in self.beads.iter().enumerate() {
            let prefix = if idx == self.selected { "> " } else { "  " };
            println!("{}{}", prefix, bead);
        }

        // Footer
        println!("\n{}", "─".repeat(cols));
        println!("Press 'q' to quit | {} rows x {} cols", rows, cols);
    }
}
