//! R-Droid Core - Orchestrator and shared types
//! 
//! This crate provides the central coordination for the R-Droid IDE,
//! managing application lifecycle, background tasks, and inter-component communication.

pub mod orchestrator;
pub mod config;
pub mod project;
pub mod events;
pub mod error;
pub mod workspace;

pub use orchestrator::Orchestrator;
pub use config::AppConfig;
pub use project::Project;
pub use events::{Event, EventBus};
pub use error::{RDroidError, Result};
pub use workspace::Workspace;

/// R-Droid version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = "R-Droid 2026";
