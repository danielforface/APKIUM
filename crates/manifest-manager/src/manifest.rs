//! Android Manifest Data Structures
//!
//! Represents the complete AndroidManifest.xml structure.

use serde::{Deserialize, Serialize};
use indexmap::IndexMap;

use crate::permissions::Permission;
use crate::components::{Activity, Service, Receiver, Provider};

/// Complete Android Manifest representation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AndroidManifest {
    /// Package name (e.g., "com.example.app")
    pub package: String,
    
    /// Version code (integer)
    pub version_code: Option<u32>,
    
    /// Version name (string, e.g., "1.0.0")
    pub version_name: Option<String>,
    
    /// Install location (auto, internalOnly, preferExternal)
    pub install_location: Option<String>,
    
    /// Shared user ID
    pub shared_user_id: Option<String>,
    
    /// Minimum SDK version
    pub min_sdk: Option<u32>,
    
    /// Target SDK version
    pub target_sdk: Option<u32>,
    
    /// Maximum SDK version
    pub max_sdk: Option<u32>,
    
    /// Required permissions
    pub permissions: Vec<Permission>,
    
    /// Permissions required only on SDK 23+
    pub permissions_sdk23: Vec<Permission>,
    
    /// Hardware/software features
    pub features: Vec<(String, bool)>, // (name, required)
    
    /// Used libraries
    pub uses_libraries: Vec<UseLibrary>,
    
    /// Application block
    pub application: Option<ManifestApplication>,
    
    /// Queries (for Android 11+ package visibility)
    pub queries: Vec<Query>,
}

/// uses-library element
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UseLibrary {
    pub name: String,
    pub required: bool,
}

/// Query element for package visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Query {
    Package(String),
    Intent(crate::intent_filters::IntentFilter),
    Provider(String),
}

/// Application element
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManifestApplication {
    /// Application class name
    pub name: Option<String>,
    
    /// Application label (@string/app_name or literal)
    pub label: Option<String>,
    
    /// Icon resource
    pub icon: Option<String>,
    
    /// Round icon resource (adaptive icons)
    pub round_icon: Option<String>,
    
    /// Theme
    pub theme: Option<String>,
    
    /// Banner (for TV)
    pub banner: Option<String>,
    
    /// Description
    pub description: Option<String>,
    
    /// Allow backup
    pub allow_backup: Option<bool>,
    
    /// Full backup content rules
    pub full_backup_content: Option<String>,
    
    /// Data extraction rules (Android 12+)
    pub data_extraction_rules: Option<String>,
    
    /// Supports RTL
    pub supports_rtl: Option<bool>,
    
    /// Uses cleartext traffic
    pub use_cleartext_traffic: Option<bool>,
    
    /// Network security config
    pub network_security_config: Option<String>,
    
    /// Hardware accelerated
    pub hardware_accelerated: Option<bool>,
    
    /// Large heap
    pub large_heap: Option<bool>,
    
    /// Request legacy external storage
    pub request_legacy_external_storage: Option<bool>,
    
    /// Preserve legacy external storage
    pub preserve_legacy_external_storage: Option<bool>,
    
    /// Activities
    pub activities: Vec<Activity>,
    
    /// Services
    pub services: Vec<Service>,
    
    /// Broadcast receivers
    pub receivers: Vec<Receiver>,
    
    /// Content providers
    pub providers: Vec<Provider>,
    
    /// Metadata
    pub metadata: Vec<ManifestMetadata>,
}

/// Meta-data element
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManifestMetadata {
    pub name: String,
    pub value: Option<String>,
    pub resource: Option<String>,
}

impl AndroidManifest {
    /// Create a new manifest with basic defaults
    pub fn new(package: &str) -> Self {
        Self {
            package: package.to_string(),
            version_code: Some(1),
            version_name: Some("1.0.0".to_string()),
            min_sdk: Some(24),
            target_sdk: Some(34),
            application: Some(ManifestApplication {
                label: Some("@string/app_name".to_string()),
                icon: Some("@mipmap/ic_launcher".to_string()),
                round_icon: Some("@mipmap/ic_launcher_round".to_string()),
                theme: Some("@style/Theme.App".to_string()),
                allow_backup: Some(true),
                supports_rtl: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Add a permission
    pub fn add_permission(&mut self, permission: &str) {
        let perm = Permission {
            name: permission.to_string(),
            max_sdk_version: None,
        };
        
        if !self.permissions.iter().any(|p| p.name == perm.name) {
            self.permissions.push(perm);
        }
    }

    /// Remove a permission
    pub fn remove_permission(&mut self, permission: &str) -> bool {
        let original_len = self.permissions.len();
        self.permissions.retain(|p| p.name != permission);
        self.permissions.len() != original_len
    }

    /// Check if permission is declared
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p.name == permission)
    }

    /// Add a feature requirement
    pub fn add_feature(&mut self, feature: &str, required: bool) {
        if !self.features.iter().any(|(f, _)| f == feature) {
            self.features.push((feature.to_string(), required));
        }
    }

    /// Get the main activity
    pub fn main_activity(&self) -> Option<&Activity> {
        self.application.as_ref()?.activities.iter().find(|a| {
            a.intent_filters.iter().any(|f| {
                f.actions.iter().any(|act| act.name == "android.intent.action.MAIN") &&
                f.categories.iter().any(|cat| cat.name == "android.intent.category.LAUNCHER")
            })
        })
    }

    /// Get all component names
    pub fn all_component_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        
        if let Some(ref app) = self.application {
            for activity in &app.activities {
                names.push(format!("Activity: {}", activity.name));
            }
            for service in &app.services {
                names.push(format!("Service: {}", service.name));
            }
            for receiver in &app.receivers {
                names.push(format!("Receiver: {}", receiver.name));
            }
            for provider in &app.providers {
                names.push(format!("Provider: {}", provider.name));
            }
        }
        
        names
    }

    /// Validate the manifest
    pub fn validate(&self) -> Vec<ManifestWarning> {
        let mut warnings = Vec::new();

        if self.package.is_empty() {
            warnings.push(ManifestWarning::Error("Package name is required".into()));
        }

        if self.min_sdk.is_none() {
            warnings.push(ManifestWarning::Warning("minSdkVersion not specified".into()));
        }

        if self.target_sdk.is_none() {
            warnings.push(ManifestWarning::Warning("targetSdkVersion not specified".into()));
        }

        // Check for exported components without intent filters (Android 12+)
        if let Some(ref app) = self.application {
            for activity in &app.activities {
                if activity.exported.is_none() && !activity.intent_filters.is_empty() {
                    warnings.push(ManifestWarning::Warning(
                        format!("Activity '{}' has intent-filter but android:exported not specified (required for Android 12+)", activity.name)
                    ));
                }
            }
            
            for service in &app.services {
                if service.exported.is_none() && !service.intent_filters.is_empty() {
                    warnings.push(ManifestWarning::Warning(
                        format!("Service '{}' has intent-filter but android:exported not specified", service.name)
                    ));
                }
            }
            
            for receiver in &app.receivers {
                if receiver.exported.is_none() && !receiver.intent_filters.is_empty() {
                    warnings.push(ManifestWarning::Warning(
                        format!("Receiver '{}' has intent-filter but android:exported not specified", receiver.name)
                    ));
                }
            }
        }

        warnings
    }
}

/// Manifest validation warning
#[derive(Debug, Clone)]
pub enum ManifestWarning {
    Error(String),
    Warning(String),
    Info(String),
}
