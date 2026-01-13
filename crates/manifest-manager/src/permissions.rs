//! Permission Management
//!
//! Handles Android permissions with metadata for UI display.

use serde::{Deserialize, Serialize};

/// Android permission
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Permission {
    /// Full permission name (e.g., "android.permission.INTERNET")
    pub name: String,
    
    /// Maximum SDK version (for compatibility)
    pub max_sdk_version: Option<u32>,
}

impl Permission {
    /// Create a new permission
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            max_sdk_version: None,
        }
    }

    /// Get the short name without android.permission prefix
    pub fn short_name(&self) -> &str {
        self.name
            .strip_prefix("android.permission.")
            .unwrap_or(&self.name)
    }
}

/// Permission group with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGroup {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub icon: String,
    pub permissions: Vec<PermissionInfo>,
}

/// Permission info with UI metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub protection_level: ProtectionLevel,
    pub added_in_api: u32,
    pub deprecated_in_api: Option<u32>,
    pub max_sdk_version: Option<u32>,
}

/// Permission protection level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectionLevel {
    Normal,
    Dangerous,
    Signature,
    SignatureOrSystem,
}

impl ProtectionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtectionLevel::Normal => "normal",
            ProtectionLevel::Dangerous => "dangerous",
            ProtectionLevel::Signature => "signature",
            ProtectionLevel::SignatureOrSystem => "signatureOrSystem",
        }
    }
}

/// Permission Manager for querying and managing permissions
pub struct PermissionManager {
    groups: Vec<PermissionGroup>,
}

impl PermissionManager {
    /// Create a new permission manager with built-in permission data
    pub fn new() -> Self {
        Self {
            groups: Self::load_permission_groups(),
        }
    }

    /// Get all permission groups
    pub fn groups(&self) -> &[PermissionGroup] {
        &self.groups
    }

    /// Get permission info by name
    pub fn get_permission(&self, name: &str) -> Option<&PermissionInfo> {
        for group in &self.groups {
            for perm in &group.permissions {
                if perm.name == name {
                    return Some(perm);
                }
            }
        }
        None
    }

    /// Get all dangerous permissions
    pub fn dangerous_permissions(&self) -> Vec<&PermissionInfo> {
        self.groups
            .iter()
            .flat_map(|g| g.permissions.iter())
            .filter(|p| p.protection_level == ProtectionLevel::Dangerous)
            .collect()
    }

    /// Search permissions by keyword
    pub fn search(&self, query: &str) -> Vec<&PermissionInfo> {
        let query_lower = query.to_lowercase();
        
        self.groups
            .iter()
            .flat_map(|g| g.permissions.iter())
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.display_name.to_lowercase().contains(&query_lower) ||
                p.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Check if permission requires runtime request (API 23+)
    pub fn requires_runtime_request(&self, name: &str) -> bool {
        self.get_permission(name)
            .map(|p| p.protection_level == ProtectionLevel::Dangerous)
            .unwrap_or(false)
    }

    /// Get commonly used permissions
    pub fn common_permissions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("android.permission.INTERNET", "Internet access"),
            ("android.permission.ACCESS_NETWORK_STATE", "Network state"),
            ("android.permission.ACCESS_WIFI_STATE", "WiFi state"),
            ("android.permission.CAMERA", "Camera"),
            ("android.permission.RECORD_AUDIO", "Microphone"),
            ("android.permission.ACCESS_FINE_LOCATION", "Fine location"),
            ("android.permission.ACCESS_COARSE_LOCATION", "Coarse location"),
            ("android.permission.READ_EXTERNAL_STORAGE", "Read storage"),
            ("android.permission.WRITE_EXTERNAL_STORAGE", "Write storage"),
            ("android.permission.READ_CONTACTS", "Read contacts"),
            ("android.permission.VIBRATE", "Vibrate"),
            ("android.permission.WAKE_LOCK", "Wake lock"),
            ("android.permission.POST_NOTIFICATIONS", "Notifications"),
            ("android.permission.FOREGROUND_SERVICE", "Foreground service"),
        ]
    }

    fn load_permission_groups() -> Vec<PermissionGroup> {
        vec![
            PermissionGroup {
                name: "NETWORK".to_string(),
                display_name: "Network".to_string(),
                description: "Network access and state".to_string(),
                icon: "network".to_string(),
                permissions: vec![
                    PermissionInfo {
                        name: "android.permission.INTERNET".to_string(),
                        display_name: "Internet".to_string(),
                        description: "Allows the app to open network sockets".to_string(),
                        protection_level: ProtectionLevel::Normal,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.ACCESS_NETWORK_STATE".to_string(),
                        display_name: "Network State".to_string(),
                        description: "Allows the app to access information about networks".to_string(),
                        protection_level: ProtectionLevel::Normal,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.ACCESS_WIFI_STATE".to_string(),
                        display_name: "WiFi State".to_string(),
                        description: "Allows the app to access information about Wi-Fi networks".to_string(),
                        protection_level: ProtectionLevel::Normal,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                ],
            },
            PermissionGroup {
                name: "LOCATION".to_string(),
                display_name: "Location".to_string(),
                description: "Device location access".to_string(),
                icon: "location".to_string(),
                permissions: vec![
                    PermissionInfo {
                        name: "android.permission.ACCESS_FINE_LOCATION".to_string(),
                        display_name: "Fine Location".to_string(),
                        description: "Allows the app to access precise location from GPS".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.ACCESS_COARSE_LOCATION".to_string(),
                        display_name: "Coarse Location".to_string(),
                        description: "Allows the app to access approximate location".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.ACCESS_BACKGROUND_LOCATION".to_string(),
                        display_name: "Background Location".to_string(),
                        description: "Allows the app to access location in the background".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 29,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                ],
            },
            PermissionGroup {
                name: "CAMERA".to_string(),
                display_name: "Camera".to_string(),
                description: "Camera and sensors".to_string(),
                icon: "camera".to_string(),
                permissions: vec![
                    PermissionInfo {
                        name: "android.permission.CAMERA".to_string(),
                        display_name: "Camera".to_string(),
                        description: "Allows the app to access the camera".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.RECORD_AUDIO".to_string(),
                        display_name: "Microphone".to_string(),
                        description: "Allows the app to record audio".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                ],
            },
            PermissionGroup {
                name: "STORAGE".to_string(),
                display_name: "Storage".to_string(),
                description: "External storage access".to_string(),
                icon: "storage".to_string(),
                permissions: vec![
                    PermissionInfo {
                        name: "android.permission.READ_EXTERNAL_STORAGE".to_string(),
                        display_name: "Read Storage".to_string(),
                        description: "Allows the app to read from external storage".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 16,
                        deprecated_in_api: Some(33),
                        max_sdk_version: Some(32),
                    },
                    PermissionInfo {
                        name: "android.permission.WRITE_EXTERNAL_STORAGE".to_string(),
                        display_name: "Write Storage".to_string(),
                        description: "Allows the app to write to external storage".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 4,
                        deprecated_in_api: Some(33),
                        max_sdk_version: Some(32),
                    },
                    PermissionInfo {
                        name: "android.permission.READ_MEDIA_IMAGES".to_string(),
                        display_name: "Read Images".to_string(),
                        description: "Allows the app to read image files".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 33,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.READ_MEDIA_VIDEO".to_string(),
                        display_name: "Read Video".to_string(),
                        description: "Allows the app to read video files".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 33,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.READ_MEDIA_AUDIO".to_string(),
                        display_name: "Read Audio".to_string(),
                        description: "Allows the app to read audio files".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 33,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                ],
            },
            PermissionGroup {
                name: "CONTACTS".to_string(),
                display_name: "Contacts".to_string(),
                description: "Contact information access".to_string(),
                icon: "contacts".to_string(),
                permissions: vec![
                    PermissionInfo {
                        name: "android.permission.READ_CONTACTS".to_string(),
                        display_name: "Read Contacts".to_string(),
                        description: "Allows the app to read contact data".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.WRITE_CONTACTS".to_string(),
                        display_name: "Write Contacts".to_string(),
                        description: "Allows the app to modify contact data".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                ],
            },
            PermissionGroup {
                name: "SYSTEM".to_string(),
                display_name: "System".to_string(),
                description: "System features".to_string(),
                icon: "system".to_string(),
                permissions: vec![
                    PermissionInfo {
                        name: "android.permission.VIBRATE".to_string(),
                        display_name: "Vibrate".to_string(),
                        description: "Allows the app to control the vibrator".to_string(),
                        protection_level: ProtectionLevel::Normal,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.WAKE_LOCK".to_string(),
                        display_name: "Wake Lock".to_string(),
                        description: "Allows the app to prevent the device from sleeping".to_string(),
                        protection_level: ProtectionLevel::Normal,
                        added_in_api: 1,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.FOREGROUND_SERVICE".to_string(),
                        display_name: "Foreground Service".to_string(),
                        description: "Allows the app to use foreground services".to_string(),
                        protection_level: ProtectionLevel::Normal,
                        added_in_api: 28,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.POST_NOTIFICATIONS".to_string(),
                        display_name: "Post Notifications".to_string(),
                        description: "Allows the app to post notifications".to_string(),
                        protection_level: ProtectionLevel::Dangerous,
                        added_in_api: 33,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                    PermissionInfo {
                        name: "android.permission.RECEIVE_BOOT_COMPLETED".to_string(),
                        display_name: "Boot Completed".to_string(),
                        description: "Allows the app to receive boot completed broadcast".to_string(),
                        protection_level: ProtectionLevel::Normal,
                        added_in_api: 3,
                        deprecated_in_api: None,
                        max_sdk_version: None,
                    },
                ],
            },
        ]
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}
