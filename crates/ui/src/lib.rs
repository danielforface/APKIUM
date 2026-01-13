//! R-Droid UI Module
//! 
//! Provides the main UI components and window management for the IDE.

pub mod app;
pub mod theme;

pub use app::App;

use anyhow::Result;

slint::include_modules!();

/// Run the R-Droid UI application
pub fn run() -> Result<()> {
    let app = App::new()?;
    app.run()?;
    Ok(())
}
