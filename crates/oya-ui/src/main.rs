//! WASM entry point for Leptos CSR app
//!
//! This is the main entry point that Trunk compiles to WASM.
//! It mounts the Leptos App component to the document body.

use leptos::prelude::*;
use oya_ui::App;

fn main() {
    // Set up panic hook for better error messages in browser console
    console_error_panic_hook::set_once();

    // Mount the Leptos app to the body
    mount_to_body(|| {
        view! {
            <App />
        }
    })
}
