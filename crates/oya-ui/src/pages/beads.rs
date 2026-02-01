//! Beads page component

use leptos::prelude::*;

/// Beads page component for issue tracking
#[component]
pub fn Beads() -> impl IntoView {
    view! {
        <div class="beads-page">
            <h1>"Beads"</h1>
            <p>"Issue tracking and management"</p>
            <div class="beads-content">
                <p>"Beads integration coming soon"</p>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beads_component_exists() {
        let _component = Beads;
    }
}
