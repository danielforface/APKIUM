//! ADB (Android Debug Bridge) Client
//!
//! Communicates with devices via ADB.

use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, warn};

use crate::device::{Device, DeviceState, DeviceType};

/// ADB errors
#[derive(Debug, thiserror::Error)]
pub enum AdbError {
    #[error("ADB not found")]
    NotFound,
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("ADB command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// ADB Client
pub struct AdbClient {
    sdk_path: PathBuf,
}

impl AdbClient {
    /// Create a new ADB client
    pub fn new(sdk_path: PathBuf) -> Self {
        Self { sdk_path }
    }

    /// Get the ADB executable path
    fn adb_path(&self) -> PathBuf {
        let platform_tools = self.sdk_path.join("platform-tools");
        if cfg!(windows) {
            platform_tools.join("adb.exe")
        } else {
            platform_tools.join("adb")
        }
    }

    /// Check if ADB is available
    pub fn is_available(&self) -> bool {
        self.adb_path().exists()
    }

    /// Run an ADB command
    async fn run(&self, args: &[&str]) -> Result<String, AdbError> {
        let adb = self.adb_path();
        
        if !adb.exists() {
            return Err(AdbError::NotFound);
        }

        debug!("adb {:?}", args);

        let output = Command::new(&adb)
            .args(args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdbError::CommandFailed(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run an ADB command for a specific device
    async fn run_for_device(&self, serial: &str, args: &[&str]) -> Result<String, AdbError> {
        let mut full_args = vec!["-s", serial];
        full_args.extend(args);
        self.run(&full_args).await
    }

    /// Start the ADB server
    pub async fn start_server(&self) -> Result<(), AdbError> {
        self.run(&["start-server"]).await?;
        Ok(())
    }

    /// Kill the ADB server
    pub async fn kill_server(&self) -> Result<(), AdbError> {
        self.run(&["kill-server"]).await?;
        Ok(())
    }

    /// List connected devices
    pub async fn list_devices(&self) -> Result<Vec<Device>, AdbError> {
        let output = self.run(&["devices", "-l"]).await?;
        let mut devices = Vec::new();

        for line in output.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let serial = parts[0].to_string();
                let state = match parts[1] {
                    "device" => DeviceState::Online,
                    "offline" => DeviceState::Offline,
                    "unauthorized" => DeviceState::Unauthorized,
                    "bootloader" => DeviceState::Bootloader,
                    "recovery" => DeviceState::Recovery,
                    _ => DeviceState::Unknown,
                };

                // Parse additional properties
                let mut model = None;
                let mut product = None;
                let mut transport_id = None;

                for part in parts.iter().skip(2) {
                    if let Some(value) = part.strip_prefix("model:") {
                        model = Some(value.to_string());
                    } else if let Some(value) = part.strip_prefix("product:") {
                        product = Some(value.to_string());
                    } else if let Some(value) = part.strip_prefix("transport_id:") {
                        transport_id = value.parse().ok();
                    }
                }

                let device_type = if serial.starts_with("emulator-") {
                    DeviceType::Emulator
                } else {
                    DeviceType::Physical
                };

                devices.push(Device {
                    serial,
                    state,
                    device_type,
                    model,
                    product,
                    transport_id,
                });
            }
        }

        Ok(devices)
    }

    /// Get a specific device
    pub async fn get_device(&self, serial: &str) -> Result<Device, AdbError> {
        let devices = self.list_devices().await?;
        devices
            .into_iter()
            .find(|d| d.serial == serial)
            .ok_or_else(|| AdbError::DeviceNotFound(serial.to_string()))
    }

    /// Wait for a device to be online
    pub async fn wait_for_device(&self, serial: &str, timeout_secs: u64) -> Result<Device, AdbError> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);
        
        while tokio::time::Instant::now() < deadline {
            if let Ok(device) = self.get_device(serial).await {
                if device.state == DeviceState::Online {
                    return Ok(device);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Err(AdbError::DeviceNotFound(format!("{} (timeout)", serial)))
    }

    /// Run a shell command on device
    pub async fn shell(&self, serial: &str, command: &str) -> Result<String, AdbError> {
        self.run_for_device(serial, &["shell", command]).await
    }

    /// Run a shell command with multiple arguments
    pub async fn shell_args(&self, serial: &str, args: &[&str]) -> Result<String, AdbError> {
        let mut full_args = vec!["shell"];
        full_args.extend(args);
        self.run_for_device(serial, &full_args).await
    }

    /// Install an APK
    pub async fn install(&self, serial: &str, apk_path: &PathBuf, replace: bool) -> Result<(), AdbError> {
        let path_str = apk_path.to_string_lossy();
        let args = if replace {
            vec!["install", "-r", &path_str]
        } else {
            vec!["install", &path_str]
        };

        self.run_for_device(serial, &args).await?;
        Ok(())
    }

    /// Install an APK with additional options
    pub async fn install_with_options(
        &self,
        serial: &str,
        apk_path: &PathBuf,
        options: &InstallOptions,
    ) -> Result<(), AdbError> {
        let path_str = apk_path.to_string_lossy();
        let mut args = vec!["install"];

        if options.replace {
            args.push("-r");
        }
        if options.allow_downgrade {
            args.push("-d");
        }
        if options.grant_permissions {
            args.push("-g");
        }
        if options.instant {
            args.push("--instant");
        }

        args.push(&path_str);
        
        self.run_for_device(serial, &args).await?;
        Ok(())
    }

    /// Uninstall a package
    pub async fn uninstall(&self, serial: &str, package: &str, keep_data: bool) -> Result<(), AdbError> {
        let args = if keep_data {
            vec!["uninstall", "-k", package]
        } else {
            vec!["uninstall", package]
        };

        self.run_for_device(serial, &args).await?;
        Ok(())
    }

    /// Push a file to device
    pub async fn push(&self, serial: &str, local: &PathBuf, remote: &str) -> Result<(), AdbError> {
        let local_str = local.to_string_lossy();
        self.run_for_device(serial, &["push", &local_str, remote]).await?;
        Ok(())
    }

    /// Pull a file from device
    pub async fn pull(&self, serial: &str, remote: &str, local: &PathBuf) -> Result<(), AdbError> {
        let local_str = local.to_string_lossy();
        self.run_for_device(serial, &["pull", remote, &local_str]).await?;
        Ok(())
    }

    /// Forward a port
    pub async fn forward(&self, serial: &str, local_port: u16, remote_port: u16) -> Result<(), AdbError> {
        let local = format!("tcp:{}", local_port);
        let remote = format!("tcp:{}", remote_port);
        self.run_for_device(serial, &["forward", &local, &remote]).await?;
        Ok(())
    }

    /// Reverse a port (device connects to host)
    pub async fn reverse(&self, serial: &str, remote_port: u16, local_port: u16) -> Result<(), AdbError> {
        let remote = format!("tcp:{}", remote_port);
        let local = format!("tcp:{}", local_port);
        self.run_for_device(serial, &["reverse", &remote, &local]).await?;
        Ok(())
    }

    /// Get device property
    pub async fn get_prop(&self, serial: &str, prop: &str) -> Result<String, AdbError> {
        let output = self.shell(serial, &format!("getprop {}", prop)).await?;
        Ok(output.trim().to_string())
    }

    /// Get Android version
    pub async fn get_android_version(&self, serial: &str) -> Result<String, AdbError> {
        self.get_prop(serial, "ro.build.version.release").await
    }

    /// Get SDK version
    pub async fn get_sdk_version(&self, serial: &str) -> Result<u32, AdbError> {
        let version = self.get_prop(serial, "ro.build.version.sdk").await?;
        version.parse().map_err(|_| AdbError::CommandFailed("Invalid SDK version".into()))
    }

    /// Launch an activity
    pub async fn start_activity(&self, serial: &str, component: &str) -> Result<(), AdbError> {
        self.shell(serial, &format!("am start -n {}", component)).await?;
        Ok(())
    }

    /// Launch an activity with intent
    pub async fn start_activity_with_intent(&self, serial: &str, action: &str, data: Option<&str>) -> Result<(), AdbError> {
        let cmd = if let Some(d) = data {
            format!("am start -a {} -d {}", action, d)
        } else {
            format!("am start -a {}", action)
        };
        self.shell(serial, &cmd).await?;
        Ok(())
    }

    /// Force stop a package
    pub async fn force_stop(&self, serial: &str, package: &str) -> Result<(), AdbError> {
        self.shell(serial, &format!("am force-stop {}", package)).await?;
        Ok(())
    }

    /// Clear app data
    pub async fn clear_data(&self, serial: &str, package: &str) -> Result<(), AdbError> {
        self.shell(serial, &format!("pm clear {}", package)).await?;
        Ok(())
    }

    /// Take a screenshot
    pub async fn screenshot(&self, serial: &str, output: &PathBuf) -> Result<(), AdbError> {
        let remote_path = "/sdcard/screenshot.png";
        self.shell(serial, &format!("screencap -p {}", remote_path)).await?;
        self.pull(serial, remote_path, output).await?;
        self.shell(serial, &format!("rm {}", remote_path)).await?;
        Ok(())
    }

    /// Record screen
    pub async fn screen_record(&self, serial: &str, remote_path: &str, duration_secs: u32) -> Result<(), AdbError> {
        let cmd = format!("screenrecord --time-limit {} {}", duration_secs, remote_path);
        self.shell(serial, &cmd).await?;
        Ok(())
    }

    /// Reboot device
    pub async fn reboot(&self, serial: &str) -> Result<(), AdbError> {
        self.run_for_device(serial, &["reboot"]).await?;
        Ok(())
    }

    /// Reboot to bootloader
    pub async fn reboot_bootloader(&self, serial: &str) -> Result<(), AdbError> {
        self.run_for_device(serial, &["reboot", "bootloader"]).await?;
        Ok(())
    }

    /// Reboot to recovery
    pub async fn reboot_recovery(&self, serial: &str) -> Result<(), AdbError> {
        self.run_for_device(serial, &["reboot", "recovery"]).await?;
        Ok(())
    }
}

/// APK install options
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// Replace existing app
    pub replace: bool,
    /// Allow version downgrade
    pub allow_downgrade: bool,
    /// Grant all permissions
    pub grant_permissions: bool,
    /// Install as instant app
    pub instant: bool,
}

/// ADB command builder
pub struct AdbCommand {
    serial: Option<String>,
    args: Vec<String>,
}

impl AdbCommand {
    pub fn new() -> Self {
        Self {
            serial: None,
            args: Vec::new(),
        }
    }

    pub fn device(mut self, serial: &str) -> Self {
        self.serial = Some(serial.to_string());
        self
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    pub fn shell(mut self, command: &str) -> Self {
        self.args.push("shell".to_string());
        self.args.push(command.to_string());
        self
    }

    pub async fn run(self, client: &AdbClient) -> Result<String, AdbError> {
        let args: Vec<&str> = self.args.iter().map(|s| s.as_str()).collect();
        
        if let Some(serial) = &self.serial {
            client.run_for_device(serial, &args).await
        } else {
            client.run(&args).await
        }
    }
}

impl Default for AdbCommand {
    fn default() -> Self {
        Self::new()
    }
}
