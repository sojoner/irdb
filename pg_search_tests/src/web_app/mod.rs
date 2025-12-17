// web_app/mod.rs - Root module for the Leptos web application
//
// This module contains all the components and logic for the full-stack
// Rust web application built with Leptos framework.
//
// Architecture:
// - model/: Shared data types (used by both client and server)
// - server_fns/: Server function declarations (both client and server)
// - api/: Database queries and server-side logic (SSR only)
// - components/: Reusable UI components (both SSR and hydrate)
// - pages/: Page-level components (both SSR and hydrate)
// - app.rs: Root application component with routing (both SSR and hydrate)

pub mod model;

// Server function declarations - must be available to both client and server
// The #[server] macro generates client stubs that call the server via HTTP
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod server_fns;

// API module for database queries and server-side logic (SSR only)
#[cfg(feature = "ssr")]
pub mod api;

// Components, pages, and app are used by both server and client
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod components;

#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod pages;

#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod app;

// Re-export main app component for convenience
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub use app::App;
