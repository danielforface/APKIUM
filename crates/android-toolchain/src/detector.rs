//! Toolchain Detection
//! 
//! Detects existing installations of Android SDK, NDK, and JDK.

use std::path::PathBuf;
use std::env;
use tracing::{info, debug, warn};
use which::which;

/// Result of SDK detection
#[derive(Debug, Clone)]
pub struct SdkInfo {
    pub path: PathBuf,
    pub build_tools_versions: Vec<String>,
    pub platform_versions: Vec<u32>,
    pub has_platform_tools: bool,
    pub has_cmdline_tools: bool,
}

/// Result of NDK detection
#[derive(Debug, Clone)]
pub struct NdkInfo {
    pub path: PathBuf,
    pub version: String,
    pub supported_abis: Vec<String>,
}

/// Result of JDK detection
#[derive(Debug, Clone)]
pub struct JdkInfo {
    pub path: PathBuf,
    pub version: String,
    pub vendor: String,
    pub is_jdk: bool, // true for JDK, false for JRE only
}

/// Toolchain detection errors
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("SDK not found")]
    SdkNotFound,
    #[error("NDK not found")]
    NdkNotFound,
    #[error("JDK not found")]
    JdkNotFound,
    #[error("Invalid installation: {0}")]
    InvalidInstallation(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Toolchain detector
pub struct ToolchainDetector;

impl ToolchainDetector {
    /// Detect Android SDK installation
    pub async fn detect_sdk() -> Result<SdkInfo, DetectionError> {
        info!("Detecting Android SDK...");
        
        // Check common locations
        let candidates = Self::sdk_candidates();
        
        for path in candidates {
            if Self::is_valid_sdk(&path).await {
                let info = Self::analyze_sdk(&path).await?;
                info!("Found Android SDK at {:?}", path);
                return Ok(info);
            }
        }
        
        Err(DetectionError::SdkNotFound)
    }

    /// Get SDK path candidates
    fn sdk_candidates() -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        
        // Environment variable
        if let Ok(sdk_root) = env::var("ANDROID_SDK_ROOT") {
            candidates.push(PathBuf::from(sdk_root));
        }
        if let Ok(android_home) = env::var("ANDROID_HOME") {
            candidates.push(PathBuf::from(android_home));
        }
        
        // Common Windows paths
        if cfg!(windows) {
            if let Some(home) = dirs::home_dir() {
                candidates.push(home.join("AppData").join("Local").join("Android").join("Sdk"));
            }
            if let Some(local) = dirs::data_local_dir() {
                candidates.push(local.join("Android").join("Sdk"));
            }
            candidates.push(PathBuf::from(r"C:\Android\sdk"));
            candidates.push(PathBuf::from(r"C:\Program Files\Android\sdk"));
            candidates.push(PathBuf::from(r"C:\Program Files (x86)\Android\sdk"));
        }
        
        // Common Unix paths
        if cfg!(unix) {
            if let Some(home) = dirs::home_dir() {
                candidates.push(home.join("Android").join("Sdk"));
                candidates.push(home.join("android-sdk"));
            }
            candidates.push(PathBuf::from("/opt/android-sdk"));
            candidates.push(PathBuf::from("/usr/local/android-sdk"));
        }
        
        // R-Droid local installation
        if let Some(data_dir) = dirs::data_local_dir() {
            candidates.push(data_dir.join("R-Droid").join("sdk"));
        }
        
        candidates
    }

    /// Check if a path contains a valid SDK
    async fn is_valid_sdk(path: &PathBuf) -> bool {
        if !path.exists() {
            return false;
        }
        
        // Must have platforms directory
        let platforms = path.join("platforms");
        if !platforms.exists() {
            return false;
        }
        
        // Must have build-tools or cmdline-tools
        let build_tools = path.join("build-tools");
        let cmdline_tools = path.join("cmdline-tools");
        
        build_tools.exists() || cmdline_tools.exists()
    }

    /// Analyze SDK installation
    async fn analyze_sdk(path: &PathBuf) -> Result<SdkInfo, DetectionError> {
        let mut build_tools_versions = Vec::new();
        let mut platform_versions = Vec::new();
        
        // List build-tools versions
        let build_tools_dir = path.join("build-tools");
        if build_tools_dir.exists() {
            if let Ok(entries) = tokio::fs::read_dir(&build_tools_dir).await {
                let mut entries = entries;
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if entry.path().is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            build_tools_versions.push(name.to_string());
                        }
                    }
                }
            }
        }
        
        // List platform versions
        let platforms_dir = path.join("platforms");
        if platforms_dir.exists() {
            if let Ok(entries) = tokio::fs::read_dir(&platforms_dir).await {
                let mut entries = entries;
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if entry.path().is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            // Parse "android-XX" format
                            if let Some(version_str) = name.strip_prefix("android-") {
                                if let Ok(version) = version_str.parse::<u32>() {
                                    platform_versions.push(version);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        build_tools_versions.sort();
        platform_versions.sort();
        
        Ok(SdkInfo {
            path: path.clone(),
            build_tools_versions,
            platform_versions,
            has_platform_tools: path.join("platform-tools").exists(),
            has_cmdline_tools: path.join("cmdline-tools").exists(),
        })
    }

    /// Detect Android NDK installation
    pub async fn detect_ndk() -> Result<NdkInfo, DetectionError> {
        info!("Detecting Android NDK...");
        
        // Check environment variable
        if let Ok(ndk_home) = env::var("ANDROID_NDK_HOME") {
            let path = PathBuf::from(ndk_home);
            if Self::is_valid_ndk(&path).await {
                return Self::analyze_ndk(&path).await;
            }
        }
        
        if let Ok(ndk_root) = env::var("NDK_ROOT") {
            let path = PathBuf::from(ndk_root);
            if Self::is_valid_ndk(&path).await {
                return Self::analyze_ndk(&path).await;
            }
        }
        
        // Check inside SDK
        if let Ok(sdk) = Self::detect_sdk().await {
            let ndk_bundle = sdk.path.join("ndk-bundle");
            if Self::is_valid_ndk(&ndk_bundle).await {
                return Self::analyze_ndk(&ndk_bundle).await;
            }
            
            // Check ndk directory for versioned NDKs
            let ndk_dir = sdk.path.join("ndk");
            if ndk_dir.exists() {
                if let Ok(entries) = tokio::fs::read_dir(&ndk_dir).await {
                    let mut entries = entries;
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        if Self::is_valid_ndk(&path).await {
                            return Self::analyze_ndk(&path).await;
                        }
                    }
                }
            }
        }
        
        Err(DetectionError::NdkNotFound)
    }

    /// Check if a path contains a valid NDK
    async fn is_valid_ndk(path: &PathBuf) -> bool {
        if !path.exists() {
            return false;
        }
        
        // Must have source.properties or ndk-build
        let source_props = path.join("source.properties");
        let ndk_build = if cfg!(windows) {
            path.join("ndk-build.cmd")
        } else {
            path.join("ndk-build")
        };
        
        source_props.exists() || ndk_build.exists()
    }

    /// Analyze NDK installation
    async fn analyze_ndk(path: &PathBuf) -> Result<NdkInfo, DetectionError> {
        let mut version = "unknown".to_string();
        
        // Read source.properties
        let source_props = path.join("source.properties");
        if source_props.exists() {
            let content = tokio::fs::read_to_string(&source_props).await?;
            for line in content.lines() {
                if line.starts_with("Pkg.Revision") {
                    if let Some(v) = line.split('=').nth(1) {
                        version = v.trim().to_string();
                    }
                }
            }
        }
        
        // Detect supported ABIs by checking toolchains
        let mut supported_abis = Vec::new();
        let toolchains = path.join("toolchains").join("llvm").join("prebuilt");
        if toolchains.exists() {
            // Check for target directories
            for abi in crate::SUPPORTED_ABIS {
                supported_abis.push(abi.to_string());
            }
        }
        
        info!("Found Android NDK {} at {:?}", version, path);
        
        Ok(NdkInfo {
            path: path.clone(),
            version,
            supported_abis,
        })
    }

    /// Detect JDK installation
    pub async fn detect_jdk() -> Result<JdkInfo, DetectionError> {
        info!("Detecting JDK...");
        
        // Check JAVA_HOME
        if let Ok(java_home) = env::var("JAVA_HOME") {
            let path = PathBuf::from(java_home);
            if let Ok(info) = Self::analyze_jdk(&path).await {
                return Ok(info);
            }
        }
        
        // Try to find java executable
        if let Ok(java_path) = which("java") {
            // Navigate up from bin/java to JDK root
            if let Some(bin) = java_path.parent() {
                if let Some(jdk_root) = bin.parent() {
                    if let Ok(info) = Self::analyze_jdk(&jdk_root.to_path_buf()).await {
                        return Ok(info);
                    }
                }
            }
        }
        
        // Check common locations
        let candidates = Self::jdk_candidates();
        for path in candidates {
            if let Ok(info) = Self::analyze_jdk(&path).await {
                return Ok(info);
            }
        }
        
        Err(DetectionError::JdkNotFound)
    }

    /// Get JDK path candidates
    fn jdk_candidates() -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        
        if cfg!(windows) {
            // Common Windows JDK locations
            for base in &[
                r"C:\Program Files\Java",
                r"C:\Program Files\Eclipse Adoptium",
                r"C:\Program Files\Microsoft",
                r"C:\Program Files\Zulu",
            ] {
                let base_path = PathBuf::from(base);
                if base_path.exists() {
                    if let Ok(entries) = std::fs::read_dir(&base_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                let name = path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                if name.contains("jdk") || name.contains("java") {
                                    candidates.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if cfg!(unix) {
            candidates.push(PathBuf::from("/usr/lib/jvm/default-java"));
            candidates.push(PathBuf::from("/usr/lib/jvm/java-17-openjdk"));
            candidates.push(PathBuf::from("/usr/lib/jvm/java-21-openjdk"));
            
            if let Some(home) = dirs::home_dir() {
                candidates.push(home.join(".sdkman").join("candidates").join("java").join("current"));
            }
        }
        
        // R-Droid local installation
        if let Some(data_dir) = dirs::data_local_dir() {
            candidates.push(data_dir.join("R-Droid").join("jdk"));
        }
        
        candidates
    }

    /// Analyze JDK installation
    async fn analyze_jdk(path: &PathBuf) -> Result<JdkInfo, DetectionError> {
        if !path.exists() {
            return Err(DetectionError::JdkNotFound);
        }
        
        let java_exe = if cfg!(windows) {
            path.join("bin").join("java.exe")
        } else {
            path.join("bin").join("java")
        };
        
        if !java_exe.exists() {
            return Err(DetectionError::InvalidInstallation("java executable not found".into()));
        }
        
        // Check for javac (JDK vs JRE)
        let javac_exe = if cfg!(windows) {
            path.join("bin").join("javac.exe")
        } else {
            path.join("bin").join("javac")
        };
        let is_jdk = javac_exe.exists();
        
        // Get version info
        let output = tokio::process::Command::new(&java_exe)
            .arg("-version")
            .output()
            .await?;
        
        let version_output = String::from_utf8_lossy(&output.stderr);
        
        let mut version = "unknown".to_string();
        let mut vendor = "unknown".to_string();
        
        for line in version_output.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("version") {
                // Extract version string
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        version = line[start + 1..start + 1 + end].to_string();
                    }
                }
            }
            if line_lower.contains("openjdk") {
                vendor = "OpenJDK".to_string();
            } else if line_lower.contains("oracle") {
                vendor = "Oracle".to_string();
            } else if line_lower.contains("adoptium") || line_lower.contains("temurin") {
                vendor = "Eclipse Adoptium".to_string();
            } else if line_lower.contains("azul") || line_lower.contains("zulu") {
                vendor = "Azul Zulu".to_string();
            } else if line_lower.contains("microsoft") {
                vendor = "Microsoft".to_string();
            }
        }
        
        info!("Found {} JDK {} ({}) at {:?}", 
            if is_jdk { "full" } else { "JRE-only" },
            version, vendor, path);
        
        Ok(JdkInfo {
            path: path.clone(),
            version,
            vendor,
            is_jdk,
        })
    }

    /// Detect all toolchains
    pub async fn detect_all() -> ToolchainStatus {
        let sdk = Self::detect_sdk().await.ok();
        let ndk = Self::detect_ndk().await.ok();
        let jdk = Self::detect_jdk().await.ok();
        
        ToolchainStatus { sdk, ndk, jdk }
    }
}

/// Overall toolchain status
#[derive(Debug, Clone)]
pub struct ToolchainStatus {
    pub sdk: Option<SdkInfo>,
    pub ndk: Option<NdkInfo>,
    pub jdk: Option<JdkInfo>,
}

impl ToolchainStatus {
    /// Check if all required tools are available
    pub fn is_complete(&self) -> bool {
        self.sdk.is_some() && self.jdk.is_some()
    }

    /// Check if ready for Rust Android development
    pub fn is_rust_ready(&self) -> bool {
        self.sdk.is_some() && self.ndk.is_some()
    }

    /// Get missing components
    pub fn missing_components(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.sdk.is_none() {
            missing.push("Android SDK");
        }
        if self.ndk.is_none() {
            missing.push("Android NDK");
        }
        if self.jdk.is_none() {
            missing.push("JDK");
        }
        missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sdk_candidates() {
        let candidates = ToolchainDetector::sdk_candidates();
        assert!(!candidates.is_empty());
    }
}
