//! Device Types and State
//!
//! Represents Android devices (physical and emulated).

use serde::{Deserialize, Serialize};

/// Device state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceState {
    /// Device is online and ready
    Online,
    /// Device is offline
    Offline,
    /// Device is not authorized (need to accept on device)
    Unauthorized,
    /// Device is in bootloader mode
    Bootloader,
    /// Device is in recovery mode
    Recovery,
    /// Device is in sideload mode
    Sideload,
    /// Unknown state
    Unknown,
}

impl DeviceState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceState::Online => "device",
            DeviceState::Offline => "offline",
            DeviceState::Unauthorized => "unauthorized",
            DeviceState::Bootloader => "bootloader",
            DeviceState::Recovery => "recovery",
            DeviceState::Sideload => "sideload",
            DeviceState::Unknown => "unknown",
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(self, DeviceState::Online)
    }
}

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    /// Physical device connected via USB/WiFi
    Physical,
    /// Android emulator
    Emulator,
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Device serial number
    pub serial: String,
    /// Device state
    pub state: DeviceState,
    /// Device type
    pub device_type: DeviceType,
    /// Device model (e.g., "Pixel 4")
    pub model: Option<String>,
    /// Device product name
    pub product: Option<String>,
    /// Transport ID
    pub transport_id: Option<u32>,
}

impl Device {
    /// Check if device is online and usable
    pub fn is_usable(&self) -> bool {
        self.state.is_usable()
    }

    /// Check if this is an emulator
    pub fn is_emulator(&self) -> bool {
        self.device_type == DeviceType::Emulator
    }

    /// Get display name
    pub fn display_name(&self) -> String {
        if let Some(ref model) = self.model {
            format!("{} ({})", model.replace('_', " "), self.serial)
        } else {
            self.serial.clone()
        }
    }

    /// Get short display name
    pub fn short_name(&self) -> String {
        self.model.as_ref()
            .map(|m| m.replace('_', " "))
            .unwrap_or_else(|| self.serial.clone())
    }
}

/// Device info with extended properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Basic device info
    pub device: Device,
    /// Android version (e.g., "13")
    pub android_version: Option<String>,
    /// SDK/API level
    pub sdk_level: Option<u32>,
    /// Build ID
    pub build_id: Option<String>,
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Brand
    pub brand: Option<String>,
    /// CPU ABI
    pub abi: Option<String>,
    /// Screen resolution
    pub screen_resolution: Option<String>,
    /// Screen density
    pub screen_density: Option<u32>,
    /// Battery level (0-100)
    pub battery_level: Option<u32>,
    /// Is charging
    pub is_charging: Option<bool>,
}

impl DeviceInfo {
    /// Create from basic device
    pub fn new(device: Device) -> Self {
        Self {
            device,
            android_version: None,
            sdk_level: None,
            build_id: None,
            manufacturer: None,
            brand: None,
            abi: None,
            screen_resolution: None,
            screen_density: None,
            battery_level: None,
            is_charging: None,
        }
    }

    /// Fetch extended info via ADB
    pub async fn fetch_extended_info(&mut self, adb: &super::adb::AdbClient) -> Result<(), super::adb::AdbError> {
        let serial = &self.device.serial;

        // Android version
        if let Ok(version) = adb.get_prop(serial, "ro.build.version.release").await {
            self.android_version = Some(version);
        }

        // SDK level
        if let Ok(sdk) = adb.get_sdk_version(serial).await {
            self.sdk_level = Some(sdk);
        }

        // Build ID
        if let Ok(build) = adb.get_prop(serial, "ro.build.id").await {
            self.build_id = Some(build);
        }

        // Manufacturer
        if let Ok(mfr) = adb.get_prop(serial, "ro.product.manufacturer").await {
            self.manufacturer = Some(mfr);
        }

        // Brand
        if let Ok(brand) = adb.get_prop(serial, "ro.product.brand").await {
            self.brand = Some(brand);
        }

        // ABI
        if let Ok(abi) = adb.get_prop(serial, "ro.product.cpu.abi").await {
            self.abi = Some(abi);
        }

        // Screen resolution
        if let Ok(output) = adb.shell(serial, "wm size").await {
            if let Some(size) = output.strip_prefix("Physical size: ") {
                self.screen_resolution = Some(size.trim().to_string());
            }
        }

        // Screen density
        if let Ok(output) = adb.shell(serial, "wm density").await {
            if let Some(density) = output.strip_prefix("Physical density: ") {
                self.screen_density = density.trim().parse().ok();
            }
        }

        // Battery info
        if let Ok(output) = adb.shell(serial, "dumpsys battery").await {
            for line in output.lines() {
                let line = line.trim();
                if line.starts_with("level:") {
                    self.battery_level = line[6..].trim().parse().ok();
                } else if line.starts_with("status:") {
                    // 2 = Charging, 5 = Full
                    if let Ok(status) = line[7..].trim().parse::<u32>() {
                        self.is_charging = Some(status == 2 || status == 5);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Device filter
#[derive(Debug, Clone, Default)]
pub struct DeviceFilter {
    /// Only online devices
    pub online_only: bool,
    /// Only physical devices
    pub physical_only: bool,
    /// Only emulators
    pub emulators_only: bool,
    /// Minimum API level
    pub min_api: Option<u32>,
    /// Maximum API level
    pub max_api: Option<u32>,
}

impl DeviceFilter {
    /// Filter for online devices only
    pub fn online() -> Self {
        Self {
            online_only: true,
            ..Default::default()
        }
    }

    /// Filter for emulators only
    pub fn emulators() -> Self {
        Self {
            emulators_only: true,
            ..Default::default()
        }
    }

    /// Filter for physical devices only
    pub fn physical() -> Self {
        Self {
            physical_only: true,
            ..Default::default()
        }
    }

    /// Check if device matches filter
    pub fn matches(&self, device: &Device) -> bool {
        if self.online_only && !device.is_usable() {
            return false;
        }
        if self.physical_only && device.is_emulator() {
            return false;
        }
        if self.emulators_only && !device.is_emulator() {
            return false;
        }
        true
    }

    /// Check if device info matches filter (with API check)
    pub fn matches_info(&self, info: &DeviceInfo) -> bool {
        if !self.matches(&info.device) {
            return false;
        }
        
        if let Some(min_api) = self.min_api {
            if let Some(sdk) = info.sdk_level {
                if sdk < min_api {
                    return false;
                }
            }
        }

        if let Some(max_api) = self.max_api {
            if let Some(sdk) = info.sdk_level {
                if sdk > max_api {
                    return false;
                }
            }
        }

        true
    }
}
