// lib.rs - Root module for the pg_search_tests library
//
// This file defines the structure of our test library.
// In Rust, when you have a lib.rs, you can organize code into modules
// and then use those modules in your test binaries.

/// The fixtures module contains reusable test data and database setup
/// Only available when database dependencies are present (not in WASM)
#[cfg(feature = "db-tools")]
pub mod fixtures;

/// The web_app module contains the Leptos-based web application
pub mod web_app;

// WASM-specific setup
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::web_app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
