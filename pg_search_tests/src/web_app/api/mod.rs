// web_app/api/mod.rs - API module for server-side logic (SSR only)
//
// This module contains database query functions and helpers for the web application.
// These are only compiled on the server (not for WASM).
//
// Architecture:
// - queries.rs: Pure database query functions (no Leptos dependencies)
// - db.rs: Database connection pool setup
//
// NOTE: Server function declarations moved to ../server_fns.rs
// so they're available to both client and server.

pub mod queries;
pub mod db;
