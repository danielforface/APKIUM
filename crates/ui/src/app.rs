//! Application UI Manager
//! 
//! Handles the main window lifecycle and UI state synchronization.

use std::sync::Arc;
use std::path::PathBuf;
use parking_lot::RwLock;
use tracing::{info, debug, error};

use r_droid_core::{
    Orchestrator, 
    AppConfig, 
    Workspace, 
    Event,
    events::EventBus,
};

use crate::{MainWindow, NewProjectDialog, SettingsDialog};

/// File tree item for UI binding
#[derive(Clone, Debug)]
pub struct FileTreeItem {
    pub name: String,
    pub is_folder: bool,
    pub indent: i32,
    pub expanded: bool,
}

/// Tab item for UI binding
#[derive(Clone, Debug)]
pub struct TabItem {
    pub filename: String,
    pub active: bool,
    pub dirty: bool,
}

/// Console line for UI binding
#[derive(Clone, Debug)]
pub struct ConsoleLine {
    pub text: String,
    pub level: String,
}

/// Main application state
pub struct AppState {
    pub file_tree: Vec<FileTreeItem>,
    pub open_tabs: Vec<TabItem>,
    pub console_output: Vec<ConsoleLine>,
    pub editor_content: String,
    pub building: bool,
    pub build_progress: f32,
    pub build_status: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            file_tree: vec![
                FileTreeItem { name: "src".into(), is_folder: true, indent: 0, expanded: true },
                FileTreeItem { name: "lib.rs".into(), is_folder: false, indent: 1, expanded: false },
            ],
            open_tabs: vec![
                TabItem { filename: "lib.rs".into(), active: true, dirty: false },
            ],
            console_output: vec![
                ConsoleLine { text: "R-Droid 2026 initialized".into(), level: "info".into() },
                ConsoleLine { text: "Welcome! Create or open a project to get started.".into(), level: "info".into() },
            ],
            editor_content: "// Welcome to R-Droid 2026!\n// Your Pure Rust Android IDE\n\nfn main() {\n    println!(\"Hello, Android!\");\n}".into(),
            building: false,
            build_progress: 0.0,
            build_status: "Ready".into(),
        }
    }
}

/// Main application controller
pub struct App {
    orchestrator: Arc<Orchestrator>,
    state: Arc<RwLock<AppState>>,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Self {
        let config = AppConfig::default();
        let orchestrator = Arc::new(Orchestrator::new(config));
        
        Self {
            orchestrator,
            state: Arc::new(RwLock::new(AppState::default())),
        }
    }

    /// Run the application
    pub fn run(&self) -> Result<(), slint::PlatformError> {
        info!("Starting R-Droid 2026...");
        
        let window = MainWindow::new()?;
        
        // Set up initial state
        self.sync_ui_state(&window);
        
        // Set up callbacks
        self.setup_callbacks(&window);
        
        // Run the event loop
        window.run()
    }

    /// Synchronize UI state with the window
    fn sync_ui_state(&self, window: &MainWindow) {
        let state = self.state.read();
        
        // Convert file tree to Slint model
        let file_tree: Vec<_> = state.file_tree.iter().map(|item| {
            slint::SharedString::from(format!(
                "{}|{}|{}|{}",
                item.name,
                item.is_folder,
                item.indent,
                item.expanded
            ))
        }).collect();
        
        // Set editor content
        window.set_editor_content(state.editor_content.clone().into());
        window.set_building(state.building);
        window.set_build_progress(state.build_progress);
        window.set_build_status(state.build_status.clone().into());
    }

    /// Set up UI callbacks
    fn setup_callbacks(&self, window: &MainWindow) {
        let state = Arc::clone(&self.state);
        let orchestrator = Arc::clone(&self.orchestrator);
        
        // Build button clicked
        let state_clone = Arc::clone(&state);
        window.on_build_clicked(move || {
            info!("Build clicked");
            let mut state = state_clone.write();
            state.building = true;
            state.build_status = "Compiling...".into();
            state.console_output.push(ConsoleLine {
                text: "Starting build...".into(),
                level: "info".into(),
            });
        });
        
        // Run button clicked
        let state_clone = Arc::clone(&state);
        window.on_run_clicked(move || {
            info!("Run clicked");
            let mut state = state_clone.write();
            state.console_output.push(ConsoleLine {
                text: "Launching emulator...".into(),
                level: "info".into(),
            });
        });
        
        // New project clicked
        window.on_new_project_clicked(move || {
            info!("New project clicked");
            // Would open NewProjectDialog
        });
        
        // Open project clicked
        window.on_open_project_clicked(move || {
            info!("Open project clicked");
            // Would open file dialog
        });
        
        // Settings clicked
        window.on_settings_clicked(move || {
            info!("Settings clicked");
            // Would open SettingsDialog
        });
        
        // File selected
        let state_clone = Arc::clone(&state);
        window.on_file_selected(move |index| {
            debug!("File selected: {}", index);
        });
        
        // Tab selected
        let state_clone = Arc::clone(&state);
        window.on_tab_selected(move |index| {
            debug!("Tab selected: {}", index);
            let mut state = state_clone.write();
            for (i, tab) in state.open_tabs.iter_mut().enumerate() {
                tab.active = i == index as usize;
            }
        });
        
        // Tab closed
        let state_clone = Arc::clone(&state);
        window.on_tab_closed(move |index| {
            debug!("Tab closed: {}", index);
            let mut state = state_clone.write();
            if (index as usize) < state.open_tabs.len() {
                state.open_tabs.remove(index as usize);
            }
        });
    }

    /// Add a log message to the console
    pub fn log(&self, message: &str, level: &str) {
        let mut state = self.state.write();
        state.console_output.push(ConsoleLine {
            text: message.into(),
            level: level.into(),
        });
    }

    /// Update build progress
    pub fn update_build_progress(&self, progress: f32, status: &str) {
        let mut state = self.state.write();
        state.build_progress = progress;
        state.build_status = status.into();
        if progress >= 1.0 {
            state.building = false;
        }
    }

    /// Open a file in the editor
    pub async fn open_file(&self, path: PathBuf) -> anyhow::Result<()> {
        let content = tokio::fs::read_to_string(&path).await?;
        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".into());
        
        let mut state = self.state.write();
        
        // Check if already open
        if !state.open_tabs.iter().any(|t| t.filename == filename) {
            // Deactivate all tabs
            for tab in state.open_tabs.iter_mut() {
                tab.active = false;
            }
            // Add new tab
            state.open_tabs.push(TabItem {
                filename,
                active: true,
                dirty: false,
            });
        }
        
        state.editor_content = content;
        
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
