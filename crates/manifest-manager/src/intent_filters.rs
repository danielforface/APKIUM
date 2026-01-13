//! Intent Filters
//!
//! Handles intent-filter, action, category, and data elements.

use serde::{Deserialize, Serialize};

/// Intent filter for components
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentFilter {
    /// Actions
    pub actions: Vec<IntentAction>,
    
    /// Categories
    pub categories: Vec<IntentCategory>,
    
    /// Data specifications
    pub data: Vec<IntentData>,
    
    /// Priority
    pub priority: Option<i32>,
    
    /// Auto verify (for App Links)
    pub auto_verify: Option<bool>,
}

impl IntentFilter {
    /// Create a launcher intent filter (MAIN + LAUNCHER)
    pub fn launcher() -> Self {
        Self {
            actions: vec![IntentAction::main()],
            categories: vec![IntentCategory::launcher()],
            ..Default::default()
        }
    }

    /// Create a VIEW intent filter for deep links
    pub fn deep_link(scheme: &str, host: &str) -> Self {
        Self {
            actions: vec![IntentAction::view()],
            categories: vec![
                IntentCategory::default_category(),
                IntentCategory::browsable(),
            ],
            data: vec![IntentData {
                scheme: Some(scheme.to_string()),
                host: Some(host.to_string()),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// Create an App Link intent filter (verified)
    pub fn app_link(host: &str, path_prefix: Option<&str>) -> Self {
        Self {
            actions: vec![IntentAction::view()],
            categories: vec![
                IntentCategory::default_category(),
                IntentCategory::browsable(),
            ],
            data: vec![IntentData {
                scheme: Some("https".to_string()),
                host: Some(host.to_string()),
                path_prefix: path_prefix.map(|s| s.to_string()),
                ..Default::default()
            }],
            auto_verify: Some(true),
            ..Default::default()
        }
    }

    /// Create a boot completed intent filter
    pub fn boot_completed() -> Self {
        Self {
            actions: vec![IntentAction::new("android.intent.action.BOOT_COMPLETED")],
            ..Default::default()
        }
    }

    /// Check if this is a launcher intent filter
    pub fn is_launcher(&self) -> bool {
        self.actions.iter().any(|a| a.name == "android.intent.action.MAIN") &&
        self.categories.iter().any(|c| c.name == "android.intent.category.LAUNCHER")
    }

    /// Check if this is a deep link / app link filter
    pub fn is_deep_link(&self) -> bool {
        self.actions.iter().any(|a| a.name == "android.intent.action.VIEW") &&
        self.categories.iter().any(|c| c.name == "android.intent.category.BROWSABLE")
    }
}

/// Intent action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAction {
    pub name: String,
}

impl IntentAction {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    pub fn main() -> Self {
        Self::new("android.intent.action.MAIN")
    }

    pub fn view() -> Self {
        Self::new("android.intent.action.VIEW")
    }

    pub fn send() -> Self {
        Self::new("android.intent.action.SEND")
    }

    pub fn send_multiple() -> Self {
        Self::new("android.intent.action.SEND_MULTIPLE")
    }
}

/// Common intent actions
pub mod actions {
    pub const MAIN: &str = "android.intent.action.MAIN";
    pub const VIEW: &str = "android.intent.action.VIEW";
    pub const EDIT: &str = "android.intent.action.EDIT";
    pub const PICK: &str = "android.intent.action.PICK";
    pub const SEND: &str = "android.intent.action.SEND";
    pub const SEND_MULTIPLE: &str = "android.intent.action.SEND_MULTIPLE";
    pub const SENDTO: &str = "android.intent.action.SENDTO";
    pub const ANSWER: &str = "android.intent.action.ANSWER";
    pub const INSERT: &str = "android.intent.action.INSERT";
    pub const DELETE: &str = "android.intent.action.DELETE";
    pub const SEARCH: &str = "android.intent.action.SEARCH";
    pub const WEB_SEARCH: &str = "android.intent.action.WEB_SEARCH";
    pub const SYNC: &str = "android.intent.action.SYNC";
    pub const BOOT_COMPLETED: &str = "android.intent.action.BOOT_COMPLETED";
    pub const SCREEN_ON: &str = "android.intent.action.SCREEN_ON";
    pub const SCREEN_OFF: &str = "android.intent.action.SCREEN_OFF";
    pub const BATTERY_LOW: &str = "android.intent.action.BATTERY_LOW";
    pub const BATTERY_OKAY: &str = "android.intent.action.BATTERY_OKAY";
    pub const POWER_CONNECTED: &str = "android.intent.action.ACTION_POWER_CONNECTED";
    pub const POWER_DISCONNECTED: &str = "android.intent.action.ACTION_POWER_DISCONNECTED";
}

/// Intent category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentCategory {
    pub name: String,
}

impl IntentCategory {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    pub fn launcher() -> Self {
        Self::new("android.intent.category.LAUNCHER")
    }

    pub fn default_category() -> Self {
        Self::new("android.intent.category.DEFAULT")
    }

    pub fn browsable() -> Self {
        Self::new("android.intent.category.BROWSABLE")
    }

    pub fn home() -> Self {
        Self::new("android.intent.category.HOME")
    }
}

/// Common intent categories
pub mod categories {
    pub const DEFAULT: &str = "android.intent.category.DEFAULT";
    pub const BROWSABLE: &str = "android.intent.category.BROWSABLE";
    pub const LAUNCHER: &str = "android.intent.category.LAUNCHER";
    pub const HOME: &str = "android.intent.category.HOME";
    pub const PREFERENCE: &str = "android.intent.category.PREFERENCE";
    pub const ALTERNATIVE: &str = "android.intent.category.ALTERNATIVE";
    pub const SELECTED_ALTERNATIVE: &str = "android.intent.category.SELECTED_ALTERNATIVE";
    pub const TAB: &str = "android.intent.category.TAB";
    pub const INFO: &str = "android.intent.category.INFO";
    pub const LEANBACK_LAUNCHER: &str = "android.intent.category.LEANBACK_LAUNCHER";
}

/// Intent data specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentData {
    /// URI scheme (http, https, custom)
    pub scheme: Option<String>,
    
    /// Host
    pub host: Option<String>,
    
    /// Port
    pub port: Option<String>,
    
    /// Exact path
    pub path: Option<String>,
    
    /// Path prefix
    pub path_prefix: Option<String>,
    
    /// Path pattern (with wildcards)
    pub path_pattern: Option<String>,
    
    /// MIME type
    pub mime_type: Option<String>,
}

impl IntentData {
    /// Create for a URL scheme
    pub fn url(scheme: &str, host: &str) -> Self {
        Self {
            scheme: Some(scheme.to_string()),
            host: Some(host.to_string()),
            ..Default::default()
        }
    }

    /// Create for HTTPS
    pub fn https(host: &str) -> Self {
        Self::url("https", host)
    }

    /// Create for a MIME type
    pub fn mime(mime_type: &str) -> Self {
        Self {
            mime_type: Some(mime_type.to_string()),
            ..Default::default()
        }
    }

    /// Create for image sharing
    pub fn image() -> Self {
        Self::mime("image/*")
    }

    /// Create for text sharing
    pub fn text_plain() -> Self {
        Self::mime("text/plain")
    }

    /// Build URI string
    pub fn to_uri(&self) -> Option<String> {
        let scheme = self.scheme.as_ref()?;
        let host = self.host.as_ref().map(|h| h.as_str()).unwrap_or("");
        
        let path = self.path.as_ref()
            .or(self.path_prefix.as_ref())
            .or(self.path_pattern.as_ref())
            .map(|p| p.as_str())
            .unwrap_or("");

        Some(format!("{}://{}{}", scheme, host, path))
    }
}
