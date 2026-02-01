//! Demonstration of TaskList component
//!
//! This example shows how to use the TaskList component with filtering and search.
//! To run in browser (requires trunk): `trunk serve examples/task_list_demo.rs`

use leptos::prelude::*;
use oya_ui::components::task_list::TaskList;

/// Demo app showcasing TaskList component
#[component]
pub fn App() -> impl IntoView {
    view! {
        <div style="max-width: 1200px; margin: 0 auto; padding: 20px;">
            <TaskList />
        </div>
    }
}

/// WASM entry point
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

fn main() {
    // This is just a placeholder for cargo to compile
    // The actual app runs in WASM via trunk
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("This example should be run with trunk serve");
        println!("Install trunk: cargo install trunk");
        println!("Run: trunk serve examples/task_list_demo.rs");
    }
}
