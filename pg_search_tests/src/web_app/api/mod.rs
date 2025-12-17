// web_app/api/mod.rs - API module for server-side logic
//
// This module contains database query functions and helpers
// for the web application.

#[cfg(feature = "web")]
pub mod queries;

#[cfg(feature = "web")]
pub mod db;
