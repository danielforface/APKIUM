//! R-Droid UI Module
//! 
//! Provides the main UI components and window management for the IDE.

pub mod app;
pub mod theme;

pub use app::App;

slint::include_modules!();
