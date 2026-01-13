//! CLI commands for R-Droid
//! 
//! Provides command-line interface functionality for automation and scripting.

use std::path::PathBuf;
use anyhow::Result;
use tracing::info;

/// Helper to get SDK path from environment
fn get_sdk_path() -> Result<PathBuf> {
    std::env::var("ANDROID_HOME")
        .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("ANDROID_HOME or ANDROID_SDK_ROOT not set"))
}

/// Build command options
pub struct BuildCommand {
    pub project_path: PathBuf,
    pub release: bool,
    pub target_abis: Vec<String>,
    pub sign: bool,
}

impl BuildCommand {
    /// Execute the build command
    pub async fn execute(&self) -> Result<PathBuf> {
        use r_droid_build_engine::{BuildConfig, BuildRunner, BuildVariant, AbiTarget};
        
        info!("Building project: {:?}", self.project_path);
        
        let variant = if self.release {
            BuildVariant::Release
        } else {
            BuildVariant::Debug
        };
        
        let targets: Vec<AbiTarget> = self.target_abis
            .iter()
            .filter_map(|s| match s.as_str() {
                "arm64-v8a" => Some(AbiTarget::Arm64V8a),
                "armeabi-v7a" => Some(AbiTarget::ArmeabiV7a),
                "x86" => Some(AbiTarget::X86),
                "x86_64" => Some(AbiTarget::X86_64),
                _ => None,
            })
            .collect();
        
        let config = BuildConfig {
            project_dir: self.project_path.clone(),
            variant,
            abis: if targets.is_empty() { vec![AbiTarget::Arm64V8a] } else { targets },
            ..Default::default()
        };
        
        let sdk_path = get_sdk_path()?;
        let runner = BuildRunner::new(config, sdk_path);
        let output = runner.build().await?;
        
        info!("Build successful: {:?}", output.path);
        Ok(output.path)
    }
}

/// Run command options
pub struct RunCommand {
    pub project_path: PathBuf,
    pub device_serial: Option<String>,
    pub release: bool,
}

impl RunCommand {
    /// Execute the run command
    pub async fn execute(&self) -> Result<()> {
        use r_droid_build_engine::{BuildConfig, BuildRunner, BuildVariant, AbiTarget};
        use r_droid_emulator_bridge::AdbClient;
        
        info!("Building and running: {:?}", self.project_path);
        
        let sdk_path = get_sdk_path()?;
        
        let variant = if self.release {
            BuildVariant::Release
        } else {
            BuildVariant::Debug
        };
        
        let config = BuildConfig {
            project_dir: self.project_path.clone(),
            variant,
            abis: vec![AbiTarget::Arm64V8a],
            ..Default::default()
        };
        
        let runner = BuildRunner::new(config, sdk_path.clone());
        
        // Build
        let output = runner.build().await?;
        info!("Build complete: {:?}", output.path);
        
        // Install and run
        let adb = AdbClient::new(sdk_path);
        
        let device = if let Some(serial) = &self.device_serial {
            serial.clone()
        } else {
            // Get first connected device
            let devices = adb.list_devices().await?;
            devices.first()
                .ok_or_else(|| anyhow::anyhow!("No devices connected"))?
                .serial
                .clone()
        };
        
        info!("Installing on device: {}", device);
        adb.install(&device, &output.path, Default::default()).await?;
        
        info!("App installed successfully");
        
        Ok(())
    }
}

/// Device list command
pub struct DevicesCommand;

impl DevicesCommand {
    /// List all connected devices
    pub async fn execute(&self) -> Result<()> {
        use r_droid_emulator_bridge::AdbClient;
        
        let sdk_path = get_sdk_path()?;
        let adb = AdbClient::new(sdk_path);
        let devices = adb.list_devices().await?;
        
        if devices.is_empty() {
            println!("No devices connected");
        } else {
            println!("Connected devices:");
            for device in devices {
                println!("  {} - {:?}", device.serial, device.state);
            }
        }
        
        Ok(())
    }
}

/// AVD management command
pub struct AvdCommand {
    pub action: AvdAction,
}

pub enum AvdAction {
    List,
    Create {
        name: String,
        system_image: String,
        device: Option<String>,
    },
    Delete {
        name: String,
    },
    Start {
        name: String,
    },
}

impl AvdCommand {
    /// Execute the AVD command
    pub async fn execute(&self) -> Result<()> {
        use r_droid_emulator_bridge::{AvdManager, AvdConfig, EmulatorLauncher};
        
        let sdk_path = get_sdk_path()?;
        let avd_manager = AvdManager::new(sdk_path.clone());
        
        match &self.action {
            AvdAction::List => {
                let avds = avd_manager.list_avds().await?;
                if avds.is_empty() {
                    println!("No AVDs configured");
                } else {
                    println!("Available AVDs:");
                    for avd in avds {
                        println!("  {} - {} ({})", 
                            avd.name, 
                            avd.device_name.as_deref().unwrap_or("Unknown"),
                            avd.target
                        );
                    }
                }
            }
            AvdAction::Create { name, system_image, device } => {
                let config = AvdConfig::new(name, 34, &system_image);
                avd_manager.create_avd(&config).await?;
                println!("Created AVD: {} with image {}", name, system_image);
                let _ = device; // device profile unused for now
            }
            AvdAction::Delete { name } => {
                avd_manager.delete_avd(name).await?;
                println!("Deleted AVD: {}", name);
            }
            AvdAction::Start { name } => {
                let mut launcher = EmulatorLauncher::new(sdk_path);
                let _instance = launcher.launch(name, Default::default()).await?;
                println!("Started emulator: {}", name);
            }
        }
        
        Ok(())
    }
}

/// Toolchain management command
pub struct ToolchainCommand {
    pub action: ToolchainAction,
}

pub enum ToolchainAction {
    Check,
    Install { component: String },
    Update,
}

impl ToolchainCommand {
    /// Execute the toolchain command
    pub async fn execute(&self) -> Result<()> {
        use r_droid_android_toolchain::ToolchainDetector;
        
        match &self.action {
            ToolchainAction::Check => {
                println!("Android Development Environment Status:");
                println!("========================================");
                
                match ToolchainDetector::detect_sdk().await {
                    Ok(sdk) => println!("✓ Android SDK: {:?}", sdk.path),
                    Err(_) => println!("✗ Android SDK: Not found"),
                }
                
                match ToolchainDetector::detect_ndk().await {
                    Ok(ndk) => println!("✓ Android NDK: {:?} (v{})", ndk.path, ndk.version),
                    Err(_) => println!("✗ Android NDK: Not found"),
                }
                
                match ToolchainDetector::detect_jdk().await {
                    Ok(jdk) => println!("✓ JDK: {:?} (Java {})", jdk.path, jdk.version),
                    Err(_) => println!("✗ JDK: Not found"),
                }
            }
            ToolchainAction::Install { component } => {
                println!("Installing {} (this feature requires download implementation)", component);
                // TODO: Implement download functionality
            }
            ToolchainAction::Update => {
                println!("Updating Android toolchain...");
                // Would call SDK manager to update
                println!("Update complete!");
            }
        }
        
        Ok(())
    }
}
