//! Error types for R-Droid
//! 
//! Centralized error handling using thiserror.

use thiserror::Error;

/// Main error type for R-Droid
#[derive(Error, Debug)]
pub enum RDroidError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("XML parse error: {0}")]
    XmlParse(String),

    #[error("Project error: {0}")]
    Project(String),

    #[error("Workspace error: {0}")]
    Workspace(String),

    #[error("Android SDK error: {0}")]
    AndroidSdk(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Emulator error: {0}")]
    Emulator(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("LSP error: {0}")]
    Lsp(String),

    #[error("Editor error: {0}")]
    Editor(String),

    #[error("UI error: {0}")]
    Ui(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Cancelled")]
    Cancelled,
}

/// Result type alias for R-Droid operations
pub type Result<T> = std::result::Result<T, RDroidError>;

impl RDroidError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            RDroidError::Network(_)
                | RDroidError::Timeout(_)
                | RDroidError::Cancelled
        )
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            RDroidError::Io(e) => format!("File operation failed: {}", e),
            RDroidError::Config(msg) => format!("Configuration error: {}", msg),
            RDroidError::AndroidSdk(msg) => format!("Android SDK issue: {}", msg),
            RDroidError::Build(msg) => format!("Build failed: {}", msg),
            RDroidError::Network(msg) => format!("Network error: {}. Please check your connection.", msg),
            RDroidError::Download(msg) => format!("Download failed: {}", msg),
            RDroidError::NotFound(msg) => format!("Not found: {}", msg),
            RDroidError::PermissionDenied(msg) => format!("Permission denied: {}", msg),
            RDroidError::Cancelled => "Operation was cancelled".to_string(),
            _ => self.to_string(),
        }
    }
}
