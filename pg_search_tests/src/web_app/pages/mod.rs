// web_app/pages/mod.rs - Page components module
//
// This module contains page-level Leptos components:
// - SearchPage: Main product search interface
// - ImportPage: Product import functionality (future)
// - AnalyticsPage: Analytics dashboard (future)

pub mod search;

// Re-export page components
pub use search::SearchPage;
