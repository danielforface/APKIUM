//! Android Component Types
//!
//! Activity, Service, BroadcastReceiver, and ContentProvider definitions.

use serde::{Deserialize, Serialize};

use crate::manifest::ManifestMetadata;
use crate::intent_filters::IntentFilter;

/// Component type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentType {
    Activity,
    Service,
    Receiver,
    Provider,
}

impl ComponentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComponentType::Activity => "activity",
            ComponentType::Service => "service",
            ComponentType::Receiver => "receiver",
            ComponentType::Provider => "provider",
        }
    }
}

/// Activity component
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Activity {
    /// Class name (.MainActivity or full package.Class)
    pub name: String,
    
    /// Display label
    pub label: Option<String>,
    
    /// Whether the activity is exported
    pub exported: Option<bool>,
    
    /// Enabled state
    pub enabled: Option<bool>,
    
    /// Theme
    pub theme: Option<String>,
    
    /// Screen orientation
    pub screen_orientation: Option<String>,
    
    /// Launch mode
    pub launch_mode: Option<String>,
    
    /// Config changes to handle
    pub config_changes: Option<String>,
    
    /// Window soft input mode
    pub window_soft_input_mode: Option<String>,
    
    /// Hardware accelerated
    pub hardware_accelerated: Option<bool>,
    
    /// Task affinity
    pub task_affinity: Option<String>,
    
    /// Document launch mode
    pub document_launch_mode: Option<String>,
    
    /// Parent activity
    pub parent_activity_name: Option<String>,
    
    /// Intent filters
    pub intent_filters: Vec<IntentFilter>,
    
    /// Metadata
    pub metadata: Vec<ManifestMetadata>,
}

impl Activity {
    /// Create a new activity
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Create a launcher activity (main entry point)
    pub fn launcher(name: &str) -> Self {
        let mut activity = Self::new(name);
        activity.exported = Some(true);
        activity.intent_filters.push(IntentFilter::launcher());
        activity
    }

    /// Check if this is the main launcher activity
    pub fn is_launcher(&self) -> bool {
        self.intent_filters.iter().any(|f| f.is_launcher())
    }

    /// Get the full class name
    pub fn full_name(&self, package: &str) -> String {
        if self.name.starts_with('.') {
            format!("{}{}", package, self.name)
        } else if self.name.contains('.') {
            self.name.clone()
        } else {
            format!("{}.{}", package, self.name)
        }
    }
}

/// Screen orientation options
pub mod orientation {
    pub const UNSPECIFIED: &str = "unspecified";
    pub const BEHIND: &str = "behind";
    pub const LANDSCAPE: &str = "landscape";
    pub const PORTRAIT: &str = "portrait";
    pub const REVERSE_LANDSCAPE: &str = "reverseLandscape";
    pub const REVERSE_PORTRAIT: &str = "reversePortrait";
    pub const SENSOR_LANDSCAPE: &str = "sensorLandscape";
    pub const SENSOR_PORTRAIT: &str = "sensorPortrait";
    pub const USER_LANDSCAPE: &str = "userLandscape";
    pub const USER_PORTRAIT: &str = "userPortrait";
    pub const SENSOR: &str = "sensor";
    pub const FULL_SENSOR: &str = "fullSensor";
    pub const NO_SENSOR: &str = "nosensor";
    pub const USER: &str = "user";
    pub const FULL_USER: &str = "fullUser";
    pub const LOCKED: &str = "locked";
}

/// Launch mode options
pub mod launch_mode {
    pub const STANDARD: &str = "standard";
    pub const SINGLE_TOP: &str = "singleTop";
    pub const SINGLE_TASK: &str = "singleTask";
    pub const SINGLE_INSTANCE: &str = "singleInstance";
    pub const SINGLE_INSTANCE_PER_TASK: &str = "singleInstancePerTask";
}

/// Config changes flags
pub mod config_changes {
    pub const MCC: &str = "mcc";
    pub const MNC: &str = "mnc";
    pub const LOCALE: &str = "locale";
    pub const TOUCHSCREEN: &str = "touchscreen";
    pub const KEYBOARD: &str = "keyboard";
    pub const KEYBOARD_HIDDEN: &str = "keyboardHidden";
    pub const NAVIGATION: &str = "navigation";
    pub const SCREEN_LAYOUT: &str = "screenLayout";
    pub const FONT_SCALE: &str = "fontScale";
    pub const UI_MODE: &str = "uiMode";
    pub const ORIENTATION: &str = "orientation";
    pub const DENSITY: &str = "density";
    pub const SCREEN_SIZE: &str = "screenSize";
    pub const SMALLEST_SCREEN_SIZE: &str = "smallestScreenSize";
    pub const COLOR_MODE: &str = "colorMode";
}

/// Service component
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Service {
    /// Class name
    pub name: String,
    
    /// Whether the service is exported
    pub exported: Option<bool>,
    
    /// Enabled state
    pub enabled: bool,
    
    /// Permission required to start/bind
    pub permission: Option<String>,
    
    /// Process to run in
    pub process: Option<String>,
    
    /// Foreground service type (Android 10+)
    pub foreground_service_type: Option<String>,
    
    /// Is isolated process
    pub isolated_process: Option<bool>,
    
    /// Intent filters
    pub intent_filters: Vec<IntentFilter>,
    
    /// Metadata
    pub metadata: Vec<ManifestMetadata>,
}

impl Service {
    /// Create a new service
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            ..Default::default()
        }
    }
}

/// Foreground service types
pub mod foreground_service_type {
    pub const CAMERA: &str = "camera";
    pub const CONNECTED_DEVICE: &str = "connectedDevice";
    pub const DATA_SYNC: &str = "dataSync";
    pub const HEALTH: &str = "health";
    pub const LOCATION: &str = "location";
    pub const MEDIA_PLAYBACK: &str = "mediaPlayback";
    pub const MEDIA_PROJECTION: &str = "mediaProjection";
    pub const MICROPHONE: &str = "microphone";
    pub const PHONE_CALL: &str = "phoneCall";
    pub const REMOTE_MESSAGING: &str = "remoteMessaging";
    pub const SHORT_SERVICE: &str = "shortService";
    pub const SPECIAL_USE: &str = "specialUse";
    pub const SYSTEM_EXEMPTED: &str = "systemExempted";
}

/// Broadcast Receiver component
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Receiver {
    /// Class name
    pub name: String,
    
    /// Whether the receiver is exported
    pub exported: Option<bool>,
    
    /// Enabled state
    pub enabled: bool,
    
    /// Permission required to send broadcasts
    pub permission: Option<String>,
    
    /// Process to run in
    pub process: Option<String>,
    
    /// Direct boot aware
    pub direct_boot_aware: Option<bool>,
    
    /// Intent filters
    pub intent_filters: Vec<IntentFilter>,
    
    /// Metadata
    pub metadata: Vec<ManifestMetadata>,
}

impl Receiver {
    /// Create a new receiver
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            ..Default::default()
        }
    }

    /// Create a boot receiver
    pub fn boot_receiver(name: &str) -> Self {
        let mut receiver = Self::new(name);
        receiver.exported = Some(true);
        receiver.intent_filters.push(IntentFilter::boot_completed());
        receiver
    }
}

/// Content Provider component
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provider {
    /// Class name
    pub name: String,
    
    /// Authorities (unique identifier)
    pub authorities: String,
    
    /// Whether the provider is exported
    pub exported: Option<bool>,
    
    /// Enabled state
    pub enabled: bool,
    
    /// Read permission
    pub read_permission: Option<String>,
    
    /// Write permission
    pub write_permission: Option<String>,
    
    /// Grant URI permissions
    pub grant_uri_permissions: Option<bool>,
    
    /// Multiprocess
    pub multiprocess: Option<bool>,
    
    /// Init order
    pub init_order: Option<i32>,
    
    /// Sync adapter
    pub sync_adapter: Option<bool>,
    
    /// Path permissions
    pub path_permissions: Vec<PathPermission>,
    
    /// Metadata
    pub metadata: Vec<ManifestMetadata>,
}

impl Provider {
    /// Create a new provider
    pub fn new(name: &str, authorities: &str) -> Self {
        Self {
            name: name.to_string(),
            authorities: authorities.to_string(),
            enabled: true,
            ..Default::default()
        }
    }

    /// Create a FileProvider
    pub fn file_provider(package: &str) -> Self {
        let mut provider = Self::new(
            "androidx.core.content.FileProvider",
            &format!("{}.fileprovider", package),
        );
        provider.exported = Some(false);
        provider.grant_uri_permissions = Some(true);
        provider.metadata.push(ManifestMetadata {
            name: "android.support.FILE_PROVIDER_PATHS".to_string(),
            value: None,
            resource: Some("@xml/file_paths".to_string()),
        });
        provider
    }
}

/// Path permission for ContentProvider
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathPermission {
    pub path: Option<String>,
    pub path_prefix: Option<String>,
    pub path_pattern: Option<String>,
    pub read_permission: Option<String>,
    pub write_permission: Option<String>,
}
