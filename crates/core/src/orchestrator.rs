//! Application Orchestrator
//! 
//! Central coordinator for the R-Droid IDE, managing:
//! - Application lifecycle
//! - Background task scheduling
//! - Component communication
//! - Resource management

use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{info, debug};

use crate::{
    config::AppConfig,
    events::{Event, EventBus},
    workspace::Workspace,
    error::Result,
};

/// Application state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    /// Initial startup
    Initializing,
    /// Loading workspace
    LoadingWorkspace,
    /// Ready for user interaction
    Ready,
    /// Building project
    Building,
    /// Running emulator
    RunningEmulator,
    /// Shutting down
    ShuttingDown,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Background task representation
pub struct BackgroundTask {
    pub id: uuid::Uuid,
    pub name: String,
    pub priority: TaskPriority,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

/// Main orchestrator for the IDE
pub struct Orchestrator {
    /// Current application state
    state: Arc<RwLock<AppState>>,
    /// Application configuration
    config: Arc<RwLock<AppConfig>>,
    /// Event bus for inter-component communication
    event_bus: Arc<EventBus>,
    /// Current workspace
    workspace: Arc<RwLock<Option<Workspace>>>,
    /// Background task manager
    task_sender: mpsc::Sender<BackgroundTask>,
    /// Task receiver (held by the orchestrator)
    task_receiver: Arc<RwLock<Option<mpsc::Receiver<BackgroundTask>>>>,
    /// Active background tasks
    active_tasks: Arc<RwLock<Vec<BackgroundTask>>>,
}

impl Orchestrator {
    /// Create a new orchestrator instance
    pub fn new(config: AppConfig) -> Self {
        let (task_sender, task_receiver) = mpsc::channel(100);
        
        Self {
            state: Arc::new(RwLock::new(AppState::Initializing)),
            config: Arc::new(RwLock::new(config)),
            event_bus: Arc::new(EventBus::new()),
            workspace: Arc::new(RwLock::new(None)),
            task_sender,
            task_receiver: Arc::new(RwLock::new(Some(task_receiver))),
            active_tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize the orchestrator and start background services
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing R-Droid Orchestrator...");
        
        // Initialize subsystems sequentially for reliability
        self.init_config_watcher().await?;
        self.init_file_watcher().await?;
        self.init_lsp_manager().await?;
        
        self.set_state(AppState::Ready);
        info!("Orchestrator initialized successfully");
        
        Ok(())
    }

    /// Get current application state
    pub fn state(&self) -> AppState {
        self.state.read().clone()
    }

    /// Set application state
    pub fn set_state(&self, state: AppState) {
        let mut current = self.state.write();
        debug!("State transition: {:?} -> {:?}", *current, state);
        *current = state.clone();
        
        // Emit state change event
        let _ = self.event_bus.emit(Event::StateChanged(state));
    }

    /// Get the event bus for subscribing to events
    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    /// Get current configuration
    pub fn config(&self) -> AppConfig {
        self.config.read().clone()
    }

    /// Update configuration
    pub fn update_config<F>(&self, updater: F)
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write();
        updater(&mut config);
        let _ = self.event_bus.emit(Event::ConfigChanged);
    }

    /// Open a workspace
    pub async fn open_workspace(&self, path: std::path::PathBuf) -> Result<()> {
        info!("Opening workspace: {:?}", path);
        self.set_state(AppState::LoadingWorkspace);
        
        let workspace = Workspace::open(path).await?;
        
        {
            let mut ws = self.workspace.write();
            *ws = Some(workspace);
        }
        
        let _ = self.event_bus.emit(Event::WorkspaceOpened);
        self.set_state(AppState::Ready);
        
        Ok(())
    }

    /// Get current workspace
    pub fn workspace(&self) -> Option<Workspace> {
        self.workspace.read().clone()
    }

    /// Schedule a background task
    pub async fn schedule_task(&self, name: String, priority: TaskPriority) -> Result<uuid::Uuid> {
        let task = BackgroundTask {
            id: uuid::Uuid::new_v4(),
            name,
            priority,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        };
        
        let id = task.id;
        self.task_sender.send(task).await
            .map_err(|e| crate::error::RDroidError::Internal(e.to_string()))?;
        
        Ok(id)
    }

    /// Cancel a background task
    pub fn cancel_task(&self, task_id: uuid::Uuid) {
        let tasks = self.active_tasks.read();
        if let Some(task) = tasks.iter().find(|t| t.id == task_id) {
            task.cancellation_token.cancel();
        }
    }

    /// Shutdown the orchestrator
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down orchestrator...");
        self.set_state(AppState::ShuttingDown);
        
        // Cancel all active tasks
        {
            let tasks = self.active_tasks.read();
            for task in tasks.iter() {
                task.cancellation_token.cancel();
            }
        }
        
        // Save configuration
        self.save_config().await?;
        
        let _ = self.event_bus.emit(Event::Shutdown);
        info!("Orchestrator shutdown complete");
        
        Ok(())
    }

    // Private initialization methods
    
    async fn init_config_watcher(&self) -> Result<()> {
        debug!("Initializing config watcher...");
        // Config file watcher would be implemented here
        Ok(())
    }

    async fn init_file_watcher(&self) -> Result<()> {
        debug!("Initializing file watcher...");
        // File system watcher would be implemented here
        Ok(())
    }

    async fn init_lsp_manager(&self) -> Result<()> {
        debug!("Initializing LSP manager...");
        // LSP manager would be initialized here
        Ok(())
    }

    async fn save_config(&self) -> Result<()> {
        let config = self.config.read().clone();
        config.save().await
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new(AppConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_lifecycle() {
        let orchestrator = Orchestrator::default();
        assert_eq!(orchestrator.state(), AppState::Initializing);
        
        orchestrator.set_state(AppState::Ready);
        assert_eq!(orchestrator.state(), AppState::Ready);
    }
}
