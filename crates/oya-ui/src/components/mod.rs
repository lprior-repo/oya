//! Leptos UI components for graph visualization

use leptos::prelude::*;

/// Canvas component for rendering the graph
#[component]
pub fn GraphCanvas() -> impl IntoView {
    view! {
        <canvas
            id="graph-canvas"
            width="800"
            height="600"
            style="border: 1px solid #ccc;"
        >
            "Your browser does not support the canvas element."
        </canvas>
    }
}

/// Control panel component for graph interactions
#[component]
pub fn ControlPanel() -> impl IntoView {
    view! {
        <div class="control-panel">
            <h2>"Controls"</h2>
            <button>"Reset View"</button>
            <button>"Export Graph"</button>
        </div>
    }
}

/// Info panel component for displaying node details
#[component]
pub fn InfoPanel() -> impl IntoView {
    view! {
        <div class="info-panel">
            <h3>"Node Information"</h3>
            <p>"Select a node to view details"</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_components_compile() {
        // This test verifies that all components compile correctly
        // Actual rendering tests would require a DOM environment
        let _ = GraphCanvas;
        let _ = ControlPanel;
        let _ = InfoPanel;
    }
}
