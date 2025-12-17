// web_app/components/mod.rs - UI components module
//
// This module contains all Leptos UI components for the application.
//
// Structure:
// - common.rs: Reusable atomic components (Button, Modal, Loading, etc.)
// - search.rs: Search-related components (SearchBar, FilterPanel, etc.)
// - product.rs: Product display components (ProductCard, ProductDetail)

pub mod common;
pub mod search;
pub mod product;

// Re-export commonly used components for convenience
pub use common::*;
pub use search::*;
pub use product::*;
