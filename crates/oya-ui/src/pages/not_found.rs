//! 404 Not Found page component

use leptos::prelude::*;

/// 404 Not Found page component
#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="not-found-page">
            <h1>"404 - Page Not Found"</h1>
            <p>"The page you're looking for doesn't exist."</p>
            <a href="/">"Go to Home"</a>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_component_exists() {
        let _component = NotFound;
    }
}
