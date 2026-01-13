//! AVD (Android Virtual Device) Manager
//!
//! Creates, lists, and manages Android Virtual Devices.

use std::path::PathBuf;
use std::collections::HashMap;
use tokio::process::Command;
use tracing::{info, debug, warn};
use configparser::ini::Ini;

/// AVD Manager errors
#[derive(Debug, thiserror::Error)]
pub enum AvdError {
    #[error("AVD not found: {0}")]
    NotFound(String),
    #[error("AVD Manager not found. Is Android SDK installed?")]
    ManagerNotFound,
    #[error("Failed to create AVD: {0}")]
    CreateFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

/// AVD information
#[derive(Debug, Clone)]
pub struct AvdInfo {
    pub name: String,
    pub path: PathBuf,
    pub target: String,
    pub abi: String,
    pub device_name: Option<String>,
    pub skin: Option<String>,
    pub sdcard_size: Option<String>,
    pub ram_size: Option<u32>,
    pub vm_heap: Option<u32>,
    pub data_partition_size: Option<String>,
}

/// AVD configuration for creation
#[derive(Debug, Clone)]
pub struct AvdConfig {
    pub name: String,
    pub package: String, // system image package
    pub device: Option<String>, // device profile
    pub sdcard: Option<String>, // e.g., "512M"
    pub force: bool,
}

impl AvdConfig {
    /// Create a new AVD config with defaults
    pub fn new(name: &str, api_level: u32, abi: &str) -> Self {
        Self {
            name: name.to_string(),
            package: format!("system-images;android-{};google_apis;{}", api_level, abi),
            device: Some("pixel_4".to_string()),
            sdcard: Some("512M".to_string()),
            force: false,
        }
    }

    /// Create for latest Pixel device
    pub fn pixel(name: &str, api_level: u32) -> Self {
        Self {
            name: name.to_string(),
            package: format!("system-images;android-{};google_apis_playstore;x86_64", api_level),
            device: Some("pixel_6".to_string()),
            sdcard: Some("1G".to_string()),
            force: false,
        }
    }
}

/// AVD Manager
pub struct AvdManager {
    sdk_path: PathBuf,
    avd_home: PathBuf,
}

impl AvdManager {
    /// Create a new AVD manager
    pub fn new(sdk_path: PathBuf) -> Self {
        let avd_home = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".android")
            .join("avd");

        Self { sdk_path, avd_home }
    }

    /// Get avdmanager path
    fn avdmanager_path(&self) -> PathBuf {
        let cmdline_tools = self.sdk_path.join("cmdline-tools").join("latest").join("bin");
        if cfg!(windows) {
            cmdline_tools.join("avdmanager.bat")
        } else {
            cmdline_tools.join("avdmanager")
        }
    }

    /// List all available AVDs
    pub async fn list_avds(&self) -> Result<Vec<AvdInfo>, AvdError> {
        let mut avds = Vec::new();

        if !self.avd_home.exists() {
            return Ok(avds);
        }

        let mut entries = tokio::fs::read_dir(&self.avd_home).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().map(|e| e == "ini").unwrap_or(false) {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let avd_dir = self.avd_home.join(format!("{}.avd", name));
                    
                    if let Ok(info) = self.parse_avd_info(name, &path, &avd_dir).await {
                        avds.push(info);
                    }
                }
            }
        }

        Ok(avds)
    }

    /// Parse AVD info from ini files
    async fn parse_avd_info(&self, name: &str, ini_path: &PathBuf, avd_dir: &PathBuf) -> Result<AvdInfo, AvdError> {
        let content = tokio::fs::read_to_string(ini_path).await?;
        let mut ini = Ini::new();
        ini.read(content).map_err(|e| AvdError::Parse(e))?;

        let path_str = ini.get("default", "path").unwrap_or_default();
        let avd_path = if path_str.is_empty() {
            avd_dir.clone()
        } else {
            PathBuf::from(path_str)
        };

        // Parse config.ini from AVD directory
        let config_path = avd_path.join("config.ini");
        let (target, abi, device_name, skin, sdcard, ram, vm_heap, data_partition) = 
            if config_path.exists() {
                let config_content = tokio::fs::read_to_string(&config_path).await?;
                let mut config = Ini::new();
                config.read(config_content).map_err(|e| AvdError::Parse(e))?;

                (
                    config.get("default", "image.sysdir.1").unwrap_or_default(),
                    config.get("default", "abi.type").unwrap_or_default(),
                    config.get("default", "hw.device.name"),
                    config.get("default", "skin.name"),
                    config.get("default", "sdcard.size"),
                    config.get("default", "hw.ramSize").and_then(|s| s.parse().ok()),
                    config.get("default", "vm.heapSize").and_then(|s| s.parse().ok()),
                    config.get("default", "disk.dataPartition.size"),
                )
            } else {
                (String::new(), String::new(), None, None, None, None, None, None)
            };

        Ok(AvdInfo {
            name: name.to_string(),
            path: avd_path,
            target,
            abi,
            device_name,
            skin,
            sdcard_size: sdcard,
            ram_size: ram,
            vm_heap,
            data_partition_size: data_partition,
        })
    }

    /// Create a new AVD
    pub async fn create_avd(&self, config: &AvdConfig) -> Result<AvdInfo, AvdError> {
        let avdmanager = self.avdmanager_path();
        
        if !avdmanager.exists() {
            return Err(AvdError::ManagerNotFound);
        }

        info!("Creating AVD: {}", config.name);

        let mut cmd = Command::new(&avdmanager);
        cmd.arg("create")
            .arg("avd")
            .arg("-n").arg(&config.name)
            .arg("-k").arg(&config.package);

        if let Some(ref device) = config.device {
            cmd.arg("-d").arg(device);
        }

        if config.force {
            cmd.arg("--force");
        }

        // Pipe "no" to the custom hardware profile question
        cmd.stdin(std::process::Stdio::piped());
        
        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AvdError::CreateFailed(stderr.to_string()));
        }

        // Update config with sdcard if specified
        if let Some(ref sdcard) = config.sdcard {
            let config_path = self.avd_home.join(format!("{}.avd", config.name)).join("config.ini");
            if config_path.exists() {
                let mut content = tokio::fs::read_to_string(&config_path).await?;
                content.push_str(&format!("\nsdcard.size={}\n", sdcard));
                tokio::fs::write(&config_path, content).await?;
            }
        }

        info!("AVD created successfully: {}", config.name);
        
        // Return the new AVD info
        let ini_path = self.avd_home.join(format!("{}.ini", config.name));
        let avd_dir = self.avd_home.join(format!("{}.avd", config.name));
        self.parse_avd_info(&config.name, &ini_path, &avd_dir).await
    }

    /// Delete an AVD
    pub async fn delete_avd(&self, name: &str) -> Result<(), AvdError> {
        let avdmanager = self.avdmanager_path();
        
        if !avdmanager.exists() {
            return Err(AvdError::ManagerNotFound);
        }

        info!("Deleting AVD: {}", name);

        let output = Command::new(&avdmanager)
            .arg("delete")
            .arg("avd")
            .arg("-n").arg(name)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AvdError::NotFound(stderr.to_string()));
        }

        info!("AVD deleted: {}", name);
        Ok(())
    }

    /// List available system images
    pub async fn list_system_images(&self) -> Result<Vec<SystemImage>, AvdError> {
        let mut images = Vec::new();
        let system_images_dir = self.sdk_path.join("system-images");

        if !system_images_dir.exists() {
            return Ok(images);
        }

        // Walk through system-images directory structure
        // system-images/android-{api}/{variant}/{abi}/
        let mut api_entries = tokio::fs::read_dir(&system_images_dir).await?;
        
        while let Some(api_entry) = api_entries.next_entry().await? {
            let api_path = api_entry.path();
            if !api_path.is_dir() { continue; }

            let api_name = api_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let api_level = api_name.strip_prefix("android-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            if api_level == 0 { continue; }

            let mut variant_entries = tokio::fs::read_dir(&api_path).await?;
            
            while let Some(variant_entry) = variant_entries.next_entry().await? {
                let variant_path = variant_entry.path();
                if !variant_path.is_dir() { continue; }

                let variant = variant_path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let mut abi_entries = tokio::fs::read_dir(&variant_path).await?;
                
                while let Some(abi_entry) = abi_entries.next_entry().await? {
                    let abi_path = abi_entry.path();
                    if !abi_path.is_dir() { continue; }

                    let abi = abi_path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();

                    images.push(SystemImage {
                        api_level,
                        variant: variant.clone(),
                        abi,
                        path: abi_path,
                    });
                }
            }
        }

        Ok(images)
    }

    /// List available device definitions
    pub async fn list_devices(&self) -> Result<Vec<DeviceDefinition>, AvdError> {
        let avdmanager = self.avdmanager_path();
        
        if !avdmanager.exists() {
            return Ok(Self::builtin_devices());
        }

        let output = Command::new(&avdmanager)
            .arg("list")
            .arg("device")
            .arg("-c")
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Self::builtin_devices());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices: Vec<DeviceDefinition> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| DeviceDefinition {
                id: l.trim().to_string(),
                name: l.trim().to_string(),
                manufacturer: None,
            })
            .collect();

        if devices.is_empty() {
            Ok(Self::builtin_devices())
        } else {
            Ok(devices)
        }
    }

    fn builtin_devices() -> Vec<DeviceDefinition> {
        vec![
            DeviceDefinition { id: "pixel_6".to_string(), name: "Pixel 6".to_string(), manufacturer: Some("Google".to_string()) },
            DeviceDefinition { id: "pixel_6_pro".to_string(), name: "Pixel 6 Pro".to_string(), manufacturer: Some("Google".to_string()) },
            DeviceDefinition { id: "pixel_5".to_string(), name: "Pixel 5".to_string(), manufacturer: Some("Google".to_string()) },
            DeviceDefinition { id: "pixel_4".to_string(), name: "Pixel 4".to_string(), manufacturer: Some("Google".to_string()) },
            DeviceDefinition { id: "pixel_4_xl".to_string(), name: "Pixel 4 XL".to_string(), manufacturer: Some("Google".to_string()) },
            DeviceDefinition { id: "pixel_3a".to_string(), name: "Pixel 3a".to_string(), manufacturer: Some("Google".to_string()) },
            DeviceDefinition { id: "Nexus 5X".to_string(), name: "Nexus 5X".to_string(), manufacturer: Some("LG".to_string()) },
            DeviceDefinition { id: "Nexus 6P".to_string(), name: "Nexus 6P".to_string(), manufacturer: Some("Huawei".to_string()) },
        ]
    }
}

/// System image info
#[derive(Debug, Clone)]
pub struct SystemImage {
    pub api_level: u32,
    pub variant: String, // google_apis, google_apis_playstore, default
    pub abi: String,     // x86_64, arm64-v8a, etc.
    pub path: PathBuf,
}

impl SystemImage {
    /// Get the package string for avdmanager
    pub fn package(&self) -> String {
        format!("system-images;android-{};{};{}", self.api_level, self.variant, self.abi)
    }

    /// Check if this has Google Play
    pub fn has_play_store(&self) -> bool {
        self.variant.contains("playstore")
    }
}

/// Device definition
#[derive(Debug, Clone)]
pub struct DeviceDefinition {
    pub id: String,
    pub name: String,
    pub manufacturer: Option<String>,
}

fn dirs_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

// Use local implementation instead of dirs crate
mod dirs {
    use std::path::PathBuf;
    
    pub fn home_dir() -> Option<PathBuf> {
        super::dirs_home_dir()
    }
}
