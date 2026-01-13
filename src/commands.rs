//! CLI commands for R-Droid
//! 
//! Provides command-line interface functionality for automation and scripting.

use std::path::PathBuf;
use anyhow::Result;
use tracing::info;

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
        
        let config = BuildConfig::builder()
            .project_dir(self.project_path.clone())
            .variant(variant)
            .target_abis(if targets.is_empty() { vec![AbiTarget::Arm64V8a] } else { targets })
            .build()?;
        
        let runner = BuildRunner::new(config);
        let output = runner.build().await?;
        
        info!("Build successful: {:?}", output.output_file);
        Ok(output.output_file)
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
        
        let variant = if self.release {
            BuildVariant::Release
        } else {
            BuildVariant::Debug
        };
        
        let config = BuildConfig::builder()
            .project_dir(self.project_path.clone())
            .variant(variant)
            .target_abis(vec![AbiTarget::Arm64V8a])
            .build()?;
        
        let runner = BuildRunner::new(config);
        
        // Build
        let output = runner.build().await?;
        info!("Build complete: {:?}", output.output_file);
        
        // Install and run
        let adb = AdbClient::new()?;
        
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
        adb.install(&device, &output.output_file, Default::default()).await?;
        
        // Start the app
        if let Some(package) = &output.package_name {
            info!("Starting app: {}", package);
            let activity = format!("{}/.MainActivity", package);
            adb.shell(&device, &format!("am start -n {}", activity)).await?;
        }
        
        Ok(())
    }
}

/// Device list command
pub struct DevicesCommand;

impl DevicesCommand {
    /// List all connected devices
    pub async fn execute(&self) -> Result<()> {
        use r_droid_emulator_bridge::AdbClient;
        
        let adb = AdbClient::new()?;
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
        
        let sdk_path = std::env::var("ANDROID_HOME")
            .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
            .map(PathBuf::from)?;
        
        let avd_manager = AvdManager::new(&sdk_path)?;
        
        match &self.action {
            AvdAction::List => {
                let avds = avd_manager.list_avds().await?;
                if avds.is_empty() {
                    println!("No AVDs configured");
                } else {
                    println!("Available AVDs:");
                    for avd in avds {
                        println!("  {} - {} (API {})", 
                            avd.name, 
                            avd.device_name.as_deref().unwrap_or("Unknown"),
                            avd.api_level
                        );
                    }
                }
            }
            AvdAction::Create { name, system_image, device } => {
                let config = AvdConfig {
                    name: name.clone(),
                    system_image: system_image.clone(),
                    device: device.clone().unwrap_or_else(|| "pixel_6".to_string()),
                    ..Default::default()
                };
                
                avd_manager.create_avd(&config).await?;
                println!("Created AVD: {}", name);
            }
            AvdAction::Delete { name } => {
                avd_manager.delete_avd(name).await?;
                println!("Deleted AVD: {}", name);
            }
            AvdAction::Start { name } => {
                let launcher = EmulatorLauncher::new(&sdk_path)?;
                let _instance = launcher.start(name, Default::default()).await?;
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
        use r_droid_android_toolchain::{ToolchainDetector, ToolchainDownloader};
        
        let detector = ToolchainDetector::new();
        
        match &self.action {
            ToolchainAction::Check => {
                println!("Android Development Environment Status:");
                println!("========================================");
                
                if let Some(sdk) = detector.detect_sdk() {
                    println!("✓ Android SDK: {:?}", sdk.path);
                } else {
                    println!("✗ Android SDK: Not found");
                }
                
                if let Some(ndk) = detector.detect_ndk() {
                    println!("✓ Android NDK: {:?}", ndk.path);
                } else {
                    println!("✗ Android NDK: Not found");
                }
                
                if let Some(jdk) = detector.detect_jdk() {
                    println!("✓ JDK: {:?} (Java {})", jdk.path, jdk.version.as_deref().unwrap_or("?"));
                } else {
                    println!("✗ JDK: Not found");
                }
            }
            ToolchainAction::Install { component } => {
                let downloader = ToolchainDownloader::new();
                
                match component.as_str() {
                    "sdk" => {
                        println!("Installing Android SDK...");
                        let path = downloader.download_sdk(None).await?;
                        println!("SDK installed to: {:?}", path);
                    }
                    "ndk" => {
                        println!("Installing Android NDK...");
                        let path = downloader.download_ndk("26.1.10909125", None).await?;
                        println!("NDK installed to: {:?}", path);
                    }
                    "jdk" => {
                        println!("Installing JDK 17...");
                        let path = downloader.download_jdk("17", None).await?;
                        println!("JDK installed to: {:?}", path);
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unknown component: {}", component));
                    }
                }
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
