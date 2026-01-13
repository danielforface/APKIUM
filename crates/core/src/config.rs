//! Application Configuration
//! 
//! Manages all IDE settings including:
//! - UI preferences (theme, layout)
//! - Android SDK paths
//! - Editor settings
//! - Build configurations

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use directories::ProjectDirs;
use tracing::{info, debug};

use crate::error::Result;

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Theme {
    DarkNeon,
    LightMinimal,
    Midnight,
    Custom(String),
}

impl Default for Theme {
    fn default() -> Self {
        Theme::DarkNeon
    }
}

/// UI Layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Show file tree sidebar
    pub show_sidebar: bool,
    /// Sidebar width in pixels
    pub sidebar_width: u32,
    /// Show preview panel
    pub show_preview: bool,
    /// Preview panel width
    pub preview_width: u32,
    /// Show console panel
    pub show_console: bool,
    /// Console panel height
    pub console_height: u32,
    /// Enable glassmorphism effects
    pub glassmorphism: bool,
    /// Animation speed (0.0 - 1.0)
    pub animation_speed: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            show_sidebar: true,
            sidebar_width: 280,
            show_preview: true,
            preview_width: 400,
            show_console: true,
            console_height: 200,
            glassmorphism: true,
            animation_speed: 0.8,
        }
    }
}

/// Editor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Font family
    pub font_family: String,
    /// Font size in pixels
    pub font_size: u32,
    /// Tab size
    pub tab_size: u32,
    /// Use spaces instead of tabs
    pub use_spaces: bool,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Enable word wrap
    pub word_wrap: bool,
    /// Enable minimap
    pub show_minimap: bool,
    /// Auto-save delay in milliseconds (0 to disable)
    pub auto_save_delay: u32,
    /// Enable code lens
    pub code_lens: bool,
    /// Enable bracket pair colorization
    pub bracket_colorization: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            tab_size: 4,
            use_spaces: true,
            show_line_numbers: true,
            word_wrap: false,
            show_minimap: true,
            auto_save_delay: 1000,
            code_lens: true,
            bracket_colorization: true,
        }
    }
}

/// Android SDK configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidConfig {
    /// Path to Android SDK
    pub sdk_path: Option<PathBuf>,
    /// Path to Android NDK
    pub ndk_path: Option<PathBuf>,
    /// Path to JDK
    pub jdk_path: Option<PathBuf>,
    /// Default target SDK version
    pub target_sdk: u32,
    /// Minimum SDK version
    pub min_sdk: u32,
    /// Preferred ABI targets
    pub abi_targets: Vec<String>,
    /// Auto-download missing components
    pub auto_download: bool,
}

impl Default for AndroidConfig {
    fn default() -> Self {
        Self {
            sdk_path: None,
            ndk_path: None,
            jdk_path: None,
            target_sdk: 34,
            min_sdk: 24,
            abi_targets: vec![
                "arm64-v8a".to_string(),
                "armeabi-v7a".to_string(),
                "x86_64".to_string(),
            ],
            auto_download: true,
        }
    }
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Number of parallel jobs
    pub parallel_jobs: u32,
    /// Enable incremental builds
    pub incremental: bool,
    /// Default build variant
    pub default_variant: String,
    /// Auto-generate keystore for debug builds
    pub auto_keystore: bool,
    /// Keystore path for release builds
    pub release_keystore: Option<PathBuf>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            parallel_jobs: num_cpus::get() as u32,
            incremental: true,
            default_variant: "debug".to_string(),
            auto_keystore: true,
            release_keystore: None,
        }
    }
}

/// Emulator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorConfig {
    /// Embed emulator in IDE window
    pub embed_in_ide: bool,
    /// Default RAM size in MB
    pub default_ram_mb: u32,
    /// Enable GPU acceleration
    pub gpu_acceleration: bool,
    /// Quick boot enabled
    pub quick_boot: bool,
    /// Default skin
    pub default_skin: String,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            embed_in_ide: true,
            default_ram_mb: 2048,
            gpu_acceleration: true,
            quick_boot: true,
            default_skin: "pixel_6".to_string(),
        }
    }
}

/// AI Assistant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// Enable AI assistant
    pub enabled: bool,
    /// AI provider (local/cloud)
    pub provider: String,
    /// Show context bar
    pub show_context_bar: bool,
    /// Auto-suggest completions
    pub auto_suggest: bool,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "local".to_string(),
            show_context_bar: true,
            auto_suggest: true,
        }
    }
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Configuration version for migrations
    pub version: u32,
    /// Selected theme
    pub theme: Theme,
    /// Layout settings
    pub layout: LayoutConfig,
    /// Editor settings
    pub editor: EditorConfig,
    /// Android SDK settings
    pub android: AndroidConfig,
    /// Build settings
    pub build: BuildConfig,
    /// Emulator settings
    pub emulator: EmulatorConfig,
    /// AI assistant settings
    pub ai: AIConfig,
    /// Recent projects
    pub recent_projects: Vec<PathBuf>,
    /// Maximum recent projects to store
    pub max_recent_projects: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            theme: Theme::default(),
            layout: LayoutConfig::default(),
            editor: EditorConfig::default(),
            android: AndroidConfig::default(),
            build: BuildConfig::default(),
            emulator: EmulatorConfig::default(),
            ai: AIConfig::default(),
            recent_projects: Vec::new(),
            max_recent_projects: 10,
        }
    }
}

impl AppConfig {
    /// Get the configuration directory path
    pub fn config_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "rdroid", "R-Droid")
            .map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Get the configuration file path
    pub fn config_file() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.toml"))
    }

    /// Get the data directory path
    pub fn data_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "rdroid", "R-Droid")
            .map(|dirs| dirs.data_dir().to_path_buf())
    }

    /// Load configuration from file
    pub async fn load() -> Result<Self> {
        let config_file = Self::config_file()
            .ok_or_else(|| crate::error::RDroidError::Config("Cannot determine config path".into()))?;

        if config_file.exists() {
            debug!("Loading config from {:?}", config_file);
            let contents = tokio::fs::read_to_string(&config_file).await?;
            let config: AppConfig = toml::from_str(&contents)?;
            Ok(config)
        } else {
            info!("Config file not found, using defaults");
            let config = AppConfig::default();
            config.save().await?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub async fn save(&self) -> Result<()> {
        let config_file = Self::config_file()
            .ok_or_else(|| crate::error::RDroidError::Config("Cannot determine config path".into()))?;

        // Ensure directory exists
        if let Some(parent) = config_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let contents = toml::to_string_pretty(self)?;
        tokio::fs::write(&config_file, contents).await?;
        
        debug!("Config saved to {:?}", config_file);
        Ok(())
    }

    /// Add a recent project
    pub fn add_recent_project(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_projects.retain(|p| p != &path);
        
        // Add to front
        self.recent_projects.insert(0, path);
        
        // Trim to max size
        self.recent_projects.truncate(self.max_recent_projects);
    }

    /// Get Android SDK path, with auto-detection
    pub fn get_sdk_path(&self) -> Option<PathBuf> {
        self.android.sdk_path.clone().or_else(|| {
            // Auto-detect common paths
            let candidates = if cfg!(windows) {
                vec![
                    dirs::config_local_dir().map(|d| d.join("Android").join("Sdk")),
                    Some(PathBuf::from("C:\\Android\\sdk")),
                ]
            } else {
                vec![
                    dirs::home_dir().map(|h: PathBuf| h.join("Android").join("Sdk")),
                    Some(PathBuf::from("/usr/local/android-sdk")),
                ]
            };

            candidates.into_iter()
                .flatten()
                .find(|p: &PathBuf| p.exists())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.theme, Theme::DarkNeon);
        assert!(config.layout.show_sidebar);
        assert_eq!(config.editor.font_size, 14);
    }

    #[test]
    fn test_recent_projects() {
        let mut config = AppConfig::default();
        config.max_recent_projects = 3;
        
        config.add_recent_project(PathBuf::from("/project1"));
        config.add_recent_project(PathBuf::from("/project2"));
        config.add_recent_project(PathBuf::from("/project3"));
        config.add_recent_project(PathBuf::from("/project4"));
        
        assert_eq!(config.recent_projects.len(), 3);
        assert_eq!(config.recent_projects[0], PathBuf::from("/project4"));
    }
}
