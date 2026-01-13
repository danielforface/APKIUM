//! NDK Manager
//! 
//! Manages Android NDK for native Rust development.

use std::path::PathBuf;
use tokio::process::Command;
use tracing::{info, debug};

use crate::detector::NdkInfo;

/// NDK Manager errors
#[derive(Debug, thiserror::Error)]
pub enum NdkError {
    #[error("NDK not found")]
    NotFound,
    #[error("Invalid NDK: {0}")]
    Invalid(String),
    #[error("Toolchain not found for {0}")]
    ToolchainNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Target ABI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abi {
    Arm64V8a,
    ArmeabiV7a,
    X86,
    X86_64,
}

impl Abi {
    /// Get the NDK triple for this ABI
    pub fn ndk_triple(&self) -> &'static str {
        match self {
            Abi::Arm64V8a => "aarch64-linux-android",
            Abi::ArmeabiV7a => "armv7a-linux-androideabi",
            Abi::X86 => "i686-linux-android",
            Abi::X86_64 => "x86_64-linux-android",
        }
    }

    /// Get the Rust target triple for this ABI
    pub fn rust_triple(&self) -> &'static str {
        match self {
            Abi::Arm64V8a => "aarch64-linux-android",
            Abi::ArmeabiV7a => "armv7-linux-androideabi",
            Abi::X86 => "i686-linux-android",
            Abi::X86_64 => "x86_64-linux-android",
        }
    }

    /// Get the ABI name as used in APK lib directory
    pub fn abi_name(&self) -> &'static str {
        match self {
            Abi::Arm64V8a => "arm64-v8a",
            Abi::ArmeabiV7a => "armeabi-v7a",
            Abi::X86 => "x86",
            Abi::X86_64 => "x86_64",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "arm64-v8a" | "aarch64-linux-android" => Some(Abi::Arm64V8a),
            "armeabi-v7a" | "armv7-linux-androideabi" => Some(Abi::ArmeabiV7a),
            "x86" | "i686-linux-android" => Some(Abi::X86),
            "x86_64" | "x86_64-linux-android" => Some(Abi::X86_64),
            _ => None,
        }
    }

    /// Get all supported ABIs
    pub fn all() -> &'static [Abi] {
        &[Abi::Arm64V8a, Abi::ArmeabiV7a, Abi::X86, Abi::X86_64]
    }
}

/// NDK Toolchain for a specific ABI
pub struct Toolchain {
    pub abi: Abi,
    pub clang: PathBuf,
    pub clangxx: PathBuf,
    pub ar: PathBuf,
    pub linker: PathBuf,
    pub strip: PathBuf,
}

/// NDK Manager
pub struct NdkManager {
    ndk_path: PathBuf,
    info: NdkInfo,
    host_tag: String,
}

impl NdkManager {
    /// Create a new NDK manager from a path
    pub async fn from_path(path: PathBuf) -> Result<Self, NdkError> {
        if !path.exists() {
            return Err(NdkError::NotFound);
        }

        let info = Self::analyze_ndk(&path).await?;
        let host_tag = Self::detect_host_tag();

        Ok(Self {
            ndk_path: path,
            info,
            host_tag,
        })
    }

    /// Analyze NDK installation
    async fn analyze_ndk(path: &PathBuf) -> Result<NdkInfo, NdkError> {
        let source_props = path.join("source.properties");
        
        if !source_props.exists() {
            return Err(NdkError::Invalid("source.properties not found".into()));
        }

        let content = tokio::fs::read_to_string(&source_props).await?;
        let mut version = "unknown".to_string();

        for line in content.lines() {
            if line.starts_with("Pkg.Revision") {
                if let Some(v) = line.split('=').nth(1) {
                    version = v.trim().to_string();
                }
            }
        }

        Ok(NdkInfo {
            path: path.clone(),
            version,
            supported_abis: Abi::all().iter().map(|a| a.abi_name().to_string()).collect(),
        })
    }

    /// Detect the host platform tag
    fn detect_host_tag() -> String {
        if cfg!(windows) {
            "windows-x86_64".to_string()
        } else if cfg!(target_os = "macos") {
            "darwin-x86_64".to_string()
        } else {
            "linux-x86_64".to_string()
        }
    }

    /// Get the NDK path
    pub fn path(&self) -> &PathBuf {
        &self.ndk_path
    }

    /// Get NDK info
    pub fn info(&self) -> &NdkInfo {
        &self.info
    }

    /// Get the toolchain for a specific ABI and API level
    pub fn toolchain(&self, abi: Abi, api_level: u32) -> Result<Toolchain, NdkError> {
        let llvm_prebuilt = self.ndk_path
            .join("toolchains")
            .join("llvm")
            .join("prebuilt")
            .join(&self.host_tag);

        if !llvm_prebuilt.exists() {
            return Err(NdkError::ToolchainNotFound(abi.abi_name().to_string()));
        }

        let bin_dir = llvm_prebuilt.join("bin");
        let triple = abi.ndk_triple();
        
        let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
        let cmd_suffix = if cfg!(windows) { ".cmd" } else { "" };

        // Clang uses target-api format
        let clang_name = format!("{}{}clang{}", triple, api_level, exe_suffix);
        let clangxx_name = format!("{}{}clang++{}", triple, api_level, exe_suffix);
        
        // AR and other tools use llvm- prefix
        let ar_name = format!("llvm-ar{}", exe_suffix);
        let strip_name = format!("llvm-strip{}", exe_suffix);

        Ok(Toolchain {
            abi,
            clang: bin_dir.join(&clang_name),
            clangxx: bin_dir.join(&clangxx_name),
            ar: bin_dir.join(&ar_name),
            linker: bin_dir.join(&clang_name), // Use clang as linker
            strip: bin_dir.join(&strip_name),
        })
    }

    /// Get cargo configuration for cross-compilation
    pub fn cargo_config(&self, api_level: u32) -> String {
        let mut config = String::new();
        
        for abi in Abi::all() {
            if let Ok(toolchain) = self.toolchain(*abi, api_level) {
                let target = abi.rust_triple();
                config.push_str(&format!(
                    r#"[target.{}]
linker = "{}"
ar = "{}"

"#,
                    target,
                    toolchain.linker.to_string_lossy().replace('\\', "/"),
                    toolchain.ar.to_string_lossy().replace('\\', "/"),
                ));
            }
        }

        config
    }

    /// Write cargo config to a project
    pub async fn write_cargo_config(&self, project_dir: &PathBuf, api_level: u32) -> Result<(), NdkError> {
        let cargo_dir = project_dir.join(".cargo");
        tokio::fs::create_dir_all(&cargo_dir).await?;

        let config_path = cargo_dir.join("config.toml");
        let config = self.cargo_config(api_level);
        
        tokio::fs::write(&config_path, config).await?;
        
        info!("Wrote cargo config to {:?}", config_path);
        Ok(())
    }

    /// Get environment variables for NDK
    pub fn env_vars(&self) -> Vec<(String, String)> {
        vec![
            ("ANDROID_NDK_HOME".to_string(), self.ndk_path.to_string_lossy().to_string()),
            ("NDK_HOME".to_string(), self.ndk_path.to_string_lossy().to_string()),
        ]
    }

    /// Get environment variables for a specific target
    pub fn target_env_vars(&self, abi: Abi, api_level: u32) -> Result<Vec<(String, String)>, NdkError> {
        let toolchain = self.toolchain(abi, api_level)?;
        let target_upper = abi.rust_triple().to_uppercase().replace('-', "_");

        Ok(vec![
            (format!("CC_{}", abi.rust_triple()), toolchain.clang.to_string_lossy().to_string()),
            (format!("CXX_{}", abi.rust_triple()), toolchain.clangxx.to_string_lossy().to_string()),
            (format!("AR_{}", abi.rust_triple()), toolchain.ar.to_string_lossy().to_string()),
            (format!("CARGO_TARGET_{}_LINKER", target_upper), toolchain.linker.to_string_lossy().to_string()),
        ])
    }

    /// Install Rust targets for Android
    pub async fn install_rust_targets() -> Result<(), NdkError> {
        info!("Installing Rust Android targets...");

        for abi in Abi::all() {
            let target = abi.rust_triple();
            debug!("Installing target: {}", target);

            let output = Command::new("rustup")
                .args(&["target", "add", target])
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                debug!("Warning: Failed to install target {}: {}", target, stderr);
            }
        }

        info!("Rust Android targets installed");
        Ok(())
    }

    /// Check if Rust targets are installed
    pub async fn check_rust_targets() -> Vec<(Abi, bool)> {
        let output = Command::new("rustup")
            .args(&["target", "list", "--installed"])
            .output()
            .await;

        let installed: Vec<String> = if let Ok(output) = output {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        Abi::all()
            .iter()
            .map(|abi| (*abi, installed.iter().any(|t| t == abi.rust_triple())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_triples() {
        assert_eq!(Abi::Arm64V8a.ndk_triple(), "aarch64-linux-android");
        assert_eq!(Abi::Arm64V8a.rust_triple(), "aarch64-linux-android");
        assert_eq!(Abi::ArmeabiV7a.abi_name(), "armeabi-v7a");
    }

    #[test]
    fn test_abi_from_str() {
        assert_eq!(Abi::from_str("arm64-v8a"), Some(Abi::Arm64V8a));
        assert_eq!(Abi::from_str("x86_64"), Some(Abi::X86_64));
        assert_eq!(Abi::from_str("unknown"), None);
    }
}
