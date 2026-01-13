//! Android Manifest Manager
//! 
//! Bi-directional parsing and editing of AndroidManifest.xml files.
//! Provides both XML mode and visual/form mode editing.

pub mod parser;
pub mod manifest;
pub mod permissions;
pub mod components;
pub mod intent_filters;
pub mod writer;

pub use parser::ManifestParser;
pub use manifest::{AndroidManifest, ManifestApplication, ManifestMetadata};
pub use permissions::{Permission, PermissionGroup, PermissionManager};
pub use components::{Activity, Service, Receiver, Provider, ComponentType};
pub use intent_filters::{IntentFilter, IntentAction, IntentCategory, IntentData};
pub use writer::ManifestWriter;

/// Common Android permissions
pub mod common_permissions {
    /// Network permissions
    pub const INTERNET: &str = "android.permission.INTERNET";
    pub const ACCESS_NETWORK_STATE: &str = "android.permission.ACCESS_NETWORK_STATE";
    pub const ACCESS_WIFI_STATE: &str = "android.permission.ACCESS_WIFI_STATE";
    
    /// Location permissions
    pub const ACCESS_FINE_LOCATION: &str = "android.permission.ACCESS_FINE_LOCATION";
    pub const ACCESS_COARSE_LOCATION: &str = "android.permission.ACCESS_COARSE_LOCATION";
    pub const ACCESS_BACKGROUND_LOCATION: &str = "android.permission.ACCESS_BACKGROUND_LOCATION";
    
    /// Camera and sensors
    pub const CAMERA: &str = "android.permission.CAMERA";
    pub const RECORD_AUDIO: &str = "android.permission.RECORD_AUDIO";
    pub const BODY_SENSORS: &str = "android.permission.BODY_SENSORS";
    
    /// Storage
    pub const READ_EXTERNAL_STORAGE: &str = "android.permission.READ_EXTERNAL_STORAGE";
    pub const WRITE_EXTERNAL_STORAGE: &str = "android.permission.WRITE_EXTERNAL_STORAGE";
    pub const READ_MEDIA_IMAGES: &str = "android.permission.READ_MEDIA_IMAGES";
    pub const READ_MEDIA_VIDEO: &str = "android.permission.READ_MEDIA_VIDEO";
    pub const READ_MEDIA_AUDIO: &str = "android.permission.READ_MEDIA_AUDIO";
    
    /// Phone
    pub const READ_PHONE_STATE: &str = "android.permission.READ_PHONE_STATE";
    pub const CALL_PHONE: &str = "android.permission.CALL_PHONE";
    pub const READ_CALL_LOG: &str = "android.permission.READ_CALL_LOG";
    
    /// Contacts
    pub const READ_CONTACTS: &str = "android.permission.READ_CONTACTS";
    pub const WRITE_CONTACTS: &str = "android.permission.WRITE_CONTACTS";
    
    /// Calendar
    pub const READ_CALENDAR: &str = "android.permission.READ_CALENDAR";
    pub const WRITE_CALENDAR: &str = "android.permission.WRITE_CALENDAR";
    
    /// SMS
    pub const SEND_SMS: &str = "android.permission.SEND_SMS";
    pub const RECEIVE_SMS: &str = "android.permission.RECEIVE_SMS";
    pub const READ_SMS: &str = "android.permission.READ_SMS";
    
    /// Notifications
    pub const POST_NOTIFICATIONS: &str = "android.permission.POST_NOTIFICATIONS";
    
    /// System
    pub const VIBRATE: &str = "android.permission.VIBRATE";
    pub const WAKE_LOCK: &str = "android.permission.WAKE_LOCK";
    pub const FOREGROUND_SERVICE: &str = "android.permission.FOREGROUND_SERVICE";
    pub const RECEIVE_BOOT_COMPLETED: &str = "android.permission.RECEIVE_BOOT_COMPLETED";
    
    /// Bluetooth
    pub const BLUETOOTH: &str = "android.permission.BLUETOOTH";
    pub const BLUETOOTH_ADMIN: &str = "android.permission.BLUETOOTH_ADMIN";
    pub const BLUETOOTH_CONNECT: &str = "android.permission.BLUETOOTH_CONNECT";
    pub const BLUETOOTH_SCAN: &str = "android.permission.BLUETOOTH_SCAN";
}
