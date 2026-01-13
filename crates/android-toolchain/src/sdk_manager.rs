//! SDK Manager
//! 
//! Wraps the Android SDK manager to install and manage SDK components.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, debug, warn, error};

/// SDK component types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkComponent {
    Platform(u32),          // android-XX
    BuildTools(String),     // build-tools;XX.X.X
    PlatformTools,          // platform-tools
    CmdlineTools(String),   // cmdline-tools;XX.X
    Sources(u32),           // sources;android-XX
    SystemImage(u32, String, String), // system-images;android-XX;abi;tag
    Emulator,               // emulator
    Ndk(String),            // ndk;XX.X.XXXXX
}

impl SdkComponent {
    /// Get the SDK manager package name
    pub fn package_name(&self) -> String {
        match self {
            SdkComponent::Platform(api) => format!("platforms;android-{}", api),
            SdkComponent::BuildTools(version) => format!("build-tools;{}", version),
            SdkComponent::PlatformTools => "platform-tools".to_string(),
            SdkComponent::CmdlineTools(version) => format!("cmdline-tools;{}", version),
            SdkComponent::Sources(api) => format!("sources;android-{}", api),
            SdkComponent::SystemImage(api, abi, tag) => {
                format!("system-images;android-{};{};{}", api, tag, abi)
            }
            SdkComponent::Emulator => "emulator".to_string(),
            SdkComponent::Ndk(version) => format!("ndk;{}", version),
        }
    }
}

/// Installed component info
#[derive(Debug, Clone)]
pub struct InstalledComponent {
    pub package: String,
    pub version: String,
    pub description: String,
    pub location: PathBuf,
}

/// Available component info
#[derive(Debug, Clone)]
pub struct AvailableComponent {
    pub package: String,
    pub version: String,
    pub description: String,
}

/// SDK Manager errors
#[derive(Debug, thiserror::Error)]
pub enum SdkManagerError {
    #[error("SDK not found at {0}")]
    SdkNotFound(PathBuf),
    #[error("sdkmanager not found")]
    SdkManagerNotFound,
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Progress callback for SDK operations
pub type SdkProgressCallback = Box<dyn Fn(&str) + Send + Sync>;

/// Android SDK Manager wrapper
pub struct SdkManager {
    sdk_root: PathBuf,
    sdkmanager_path: PathBuf,
    java_home: Option<PathBuf>,
}

impl SdkManager {
    /// Create a new SDK manager
    pub fn new(sdk_root: PathBuf) -> Result<Self, SdkManagerError> {
        if !sdk_root.exists() {
            return Err(SdkManagerError::SdkNotFound(sdk_root));
        }

        // Find sdkmanager
        let sdkmanager_path = Self::find_sdkmanager(&sdk_root)?;

        Ok(Self {
            sdk_root,
            sdkmanager_path,
            java_home: None,
        })
    }

    /// Set the JAVA_HOME for SDK manager operations
    pub fn set_java_home(&mut self, java_home: PathBuf) {
        self.java_home = Some(java_home);
    }

    /// Find the sdkmanager executable
    fn find_sdkmanager(sdk_root: &PathBuf) -> Result<PathBuf, SdkManagerError> {
        let exe_name = if cfg!(windows) { "sdkmanager.bat" } else { "sdkmanager" };

        // Try cmdline-tools/latest
        let path = sdk_root.join("cmdline-tools").join("latest").join("bin").join(exe_name);
        if path.exists() {
            return Ok(path);
        }

        // Try cmdline-tools/X.X (versioned)
        let cmdline_tools = sdk_root.join("cmdline-tools");
        if cmdline_tools.exists() {
            if let Ok(entries) = std::fs::read_dir(&cmdline_tools) {
                for entry in entries.flatten() {
                    let path = entry.path().join("bin").join(exe_name);
                    if path.exists() {
                        return Ok(path);
                    }
                }
            }
        }

        // Try tools directory (legacy)
        let path = sdk_root.join("tools").join("bin").join(exe_name);
        if path.exists() {
            return Ok(path);
        }

        Err(SdkManagerError::SdkManagerNotFound)
    }

    /// Create the base command with environment variables
    fn create_command(&self) -> Command {
        let mut cmd = Command::new(&self.sdkmanager_path);
        
        cmd.env("ANDROID_SDK_ROOT", &self.sdk_root);
        cmd.env("ANDROID_HOME", &self.sdk_root);
        
        if let Some(java_home) = &self.java_home {
            cmd.env("JAVA_HOME", java_home);
        }

        cmd
    }

    /// Accept all licenses
    pub async fn accept_licenses(&self) -> Result<(), SdkManagerError> {
        info!("Accepting Android SDK licenses...");

        let mut child = self.create_command()
            .arg("--licenses")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Send 'y' repeatedly to accept all licenses
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            for _ in 0..20 {
                stdin.write_all(b"y\n").await?;
            }
        }

        let output = child.wait_with_output().await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("License acceptance may have failed: {}", stderr);
        }

        info!("Licenses accepted");
        Ok(())
    }

    /// List installed packages
    pub async fn list_installed(&self) -> Result<Vec<InstalledComponent>, SdkManagerError> {
        debug!("Listing installed SDK packages...");

        let output = self.create_command()
            .arg("--list_installed")
            .output()
            .await?;

        if !output.status.success() {
            return Err(SdkManagerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let components = Self::parse_installed_output(&stdout, &self.sdk_root)?;

        Ok(components)
    }

    /// Parse the --list_installed output
    fn parse_installed_output(output: &str, sdk_root: &PathBuf) -> Result<Vec<InstalledComponent>, SdkManagerError> {
        let mut components = Vec::new();
        let mut in_packages = false;

        for line in output.lines() {
            let line = line.trim();
            
            if line.starts_with("Installed packages:") {
                in_packages = true;
                continue;
            }
            
            if in_packages && !line.is_empty() && !line.starts_with("---") && !line.starts_with("Path") {
                let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    components.push(InstalledComponent {
                        package: parts[0].to_string(),
                        version: parts[1].to_string(),
                        description: parts[2].to_string(),
                        location: sdk_root.join(parts[0].replace(';', std::path::MAIN_SEPARATOR_STR)),
                    });
                }
            }
        }

        Ok(components)
    }

    /// List available packages
    pub async fn list_available(&self) -> Result<Vec<AvailableComponent>, SdkManagerError> {
        debug!("Listing available SDK packages...");

        let output = self.create_command()
            .arg("--list")
            .output()
            .await?;

        if !output.status.success() {
            return Err(SdkManagerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let components = Self::parse_available_output(&stdout)?;

        Ok(components)
    }

    /// Parse the --list output
    fn parse_available_output(output: &str) -> Result<Vec<AvailableComponent>, SdkManagerError> {
        let mut components = Vec::new();
        let mut in_packages = false;

        for line in output.lines() {
            let line = line.trim();
            
            if line.starts_with("Available Packages:") {
                in_packages = true;
                continue;
            }
            
            if in_packages && !line.is_empty() && !line.starts_with("---") && !line.starts_with("Path") {
                let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    components.push(AvailableComponent {
                        package: parts[0].to_string(),
                        version: parts[1].to_string(),
                        description: parts[2].to_string(),
                    });
                }
            }
        }

        Ok(components)
    }

    /// Install SDK components
    pub async fn install(
        &self,
        components: &[SdkComponent],
        progress: Option<SdkProgressCallback>,
    ) -> Result<(), SdkManagerError> {
        let packages: Vec<String> = components.iter().map(|c| c.package_name()).collect();
        
        info!("Installing SDK packages: {:?}", packages);

        let mut cmd = self.create_command();
        for package in &packages {
            cmd.arg(package);
        }

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Accept any prompts
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            for _ in 0..10 {
                stdin.write_all(b"y\n").await?;
            }
        }

        // Read output for progress
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("sdkmanager: {}", line);
                if let Some(ref callback) = progress {
                    callback(&line);
                }
            }
        }

        let status = child.wait().await?;
        
        if !status.success() {
            return Err(SdkManagerError::CommandFailed(
                format!("Installation failed with exit code: {:?}", status.code())
            ));
        }

        info!("SDK packages installed successfully");
        Ok(())
    }

    /// Uninstall SDK components
    pub async fn uninstall(&self, components: &[SdkComponent]) -> Result<(), SdkManagerError> {
        let packages: Vec<String> = components.iter().map(|c| c.package_name()).collect();
        
        info!("Uninstalling SDK packages: {:?}", packages);

        let mut cmd = self.create_command();
        cmd.arg("--uninstall");
        for package in &packages {
            cmd.arg(package);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            return Err(SdkManagerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        info!("SDK packages uninstalled successfully");
        Ok(())
    }

    /// Update all installed packages
    pub async fn update_all(&self, progress: Option<SdkProgressCallback>) -> Result<(), SdkManagerError> {
        info!("Updating all SDK packages...");

        let mut child = self.create_command()
            .arg("--update")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Accept any prompts
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            for _ in 0..10 {
                stdin.write_all(b"y\n").await?;
            }
        }

        // Read output for progress
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("sdkmanager: {}", line);
                if let Some(ref callback) = progress {
                    callback(&line);
                }
            }
        }

        let status = child.wait().await?;
        
        if !status.success() {
            return Err(SdkManagerError::CommandFailed("Update failed".to_string()));
        }

        info!("SDK packages updated successfully");
        Ok(())
    }

    /// Get the SDK root path
    pub fn sdk_root(&self) -> &PathBuf {
        &self.sdk_root
    }

    /// Check if a component is installed
    pub async fn is_installed(&self, component: &SdkComponent) -> bool {
        if let Ok(installed) = self.list_installed().await {
            let package_name = component.package_name();
            installed.iter().any(|c| c.package == package_name)
        } else {
            false
        }
    }

    /// Install essential components for Android development
    pub async fn install_essentials(
        &self,
        target_api: u32,
        progress: Option<SdkProgressCallback>,
    ) -> Result<(), SdkManagerError> {
        let components = vec![
            SdkComponent::PlatformTools,
            SdkComponent::BuildTools("34.0.0".to_string()),
            SdkComponent::Platform(target_api),
            SdkComponent::Emulator,
        ];

        self.install(&components, progress).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_package_name() {
        assert_eq!(
            SdkComponent::Platform(34).package_name(),
            "platforms;android-34"
        );
        assert_eq!(
            SdkComponent::BuildTools("34.0.0".into()).package_name(),
            "build-tools;34.0.0"
        );
        assert_eq!(
            SdkComponent::SystemImage(34, "x86_64".into(), "google_apis".into()).package_name(),
            "system-images;android-34;google_apis;x86_64"
        );
    }
}
