//! R-Droid 2026 - Pure Rust Android IDE
//! 
//! Main application entry point that initializes all subsystems
//! and launches the UI.

use std::path::PathBuf;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;
use anyhow::Result;

// Re-export crates
pub use rdroid_core as core;
pub use rdroid_ui as ui;
pub use rdroid_editor as editor;
pub use rdroid_android_toolchain as android_toolchain;
pub use rdroid_manifest_manager as manifest_manager;
pub use rdroid_emulator_bridge as emulator_bridge;
pub use rdroid_build_engine as build_engine;

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = "R-Droid 2026";

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("{} v{} starting...", APP_NAME, VERSION);

    // Initialize the core orchestrator
    let config = load_or_create_config().await?;
    let orchestrator = core::orchestrator::Orchestrator::new(config);
    
    // Auto-detect Android toolchain
    info!("Detecting Android development environment...");
    let detector = android_toolchain::ToolchainDetector::new();
    
    if let Some(sdk) = detector.detect_sdk() {
        info!("Found Android SDK: {:?}", sdk.path);
    } else {
        info!("Android SDK not found. Use the toolchain manager to download.");
    }
    
    if let Some(ndk) = detector.detect_ndk() {
        info!("Found Android NDK: {:?}", ndk.path);
    }
    
    if let Some(jdk) = detector.detect_jdk() {
        info!("Found JDK: {:?}", jdk.path);
    }

    // Start the UI
    info!("Launching UI...");
    
    #[cfg(feature = "ui")]
    {
        ui::run()?;
    }
    
    #[cfg(not(feature = "ui"))]
    {
        // CLI mode for testing
        info!("Running in CLI mode (UI feature disabled)");
        info!("R-Droid initialized successfully!");
        info!("Available commands:");
        info!("  - build: Build the current project");
        info!("  - run: Build and run on connected device");
        info!("  - devices: List connected devices");
        info!("  - avd: Manage Android Virtual Devices");
    }

    Ok(())
}

/// Load or create application configuration
async fn load_or_create_config() -> Result<core::config::AppConfig> {
    let config_dir = get_config_dir();
    let config_path = config_dir.join("config.toml");
    
    if config_path.exists() {
        info!("Loading configuration from {:?}", config_path);
        let content = tokio::fs::read_to_string(&config_path).await?;
        let config: core::config::AppConfig = toml::from_str(&content)?;
        Ok(config)
    } else {
        info!("Creating default configuration");
        let config = core::config::AppConfig::default();
        
        // Ensure config directory exists
        tokio::fs::create_dir_all(&config_dir).await?;
        
        // Save default config
        let content = toml::to_string_pretty(&config)?;
        tokio::fs::write(&config_path, content).await?;
        
        Ok(config)
    }
}

/// Get the configuration directory
fn get_config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("R-Droid")
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".config")
            .join("rdroid")
    }
}

/// Get the data directory (for SDK, cache, etc.)
fn get_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("R-Droid")
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".local")
            .join("share")
            .join("rdroid")
    }
}

/// Application state for sharing across components
pub struct AppState {
    pub config: core::config::AppConfig,
    pub sdk_path: Option<PathBuf>,
    pub ndk_path: Option<PathBuf>,
    pub jdk_path: Option<PathBuf>,
    pub current_project: Option<PathBuf>,
}

impl AppState {
    pub fn new(config: core::config::AppConfig) -> Self {
        Self {
            config,
            sdk_path: None,
            ndk_path: None,
            jdk_path: None,
            current_project: None,
        }
    }
}
