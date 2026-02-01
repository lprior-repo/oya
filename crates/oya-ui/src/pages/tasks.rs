//! Tasks page component

use leptos::prelude::*;

/// Tasks page component
#[component]
pub fn Tasks() -> impl IntoView {
    view! {
        <div class="tasks-page">
            <h1>"Tasks"</h1>
            <p>"Task list and management"</p>
            <div class="tasks-content">
                <p>"Task list coming soon"</p>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tasks_component_exists() {
        let _component = Tasks;
    }
}
