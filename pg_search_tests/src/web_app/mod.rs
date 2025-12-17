// web_app/mod.rs - Root module for the Leptos web application
//
// This module contains all the components and logic for the full-stack
// Rust web application built with Leptos framework.

pub mod model;

#[cfg(feature = "web")]
pub mod api;

#[cfg(feature = "web")]
pub mod components;

#[cfg(feature = "web")]
pub mod pages;
