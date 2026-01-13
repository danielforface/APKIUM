//! Cargo Build for Rust Android Apps
//!
//! Uses cargo-apk or xbuild for building Rust Android apps.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{info, debug, warn};

use crate::{BuildConfig, BuildError, BuildVariant, AbiTarget};
use crate::config::CargoApkConfig;

/// Cargo build for Android
pub struct CargoBuild {
    config: BuildConfig,
    cargo_config: CargoApkConfig,
    ndk_path: Option<PathBuf>,
}

impl CargoBuild {
    /// Create a new cargo builder
    pub fn new(config: BuildConfig) -> Self {
        Self {
            config,
            cargo_config: CargoApkConfig::default(),
            ndk_path: None,
        }
    }

    /// Set cargo-apk specific config
    pub fn with_cargo_config(mut self, config: CargoApkConfig) -> Self {
        self.cargo_config = config;
        self
    }

    /// Set NDK path
    pub fn with_ndk(mut self, path: PathBuf) -> Self {
        self.ndk_path = Some(path);
        self
    }

    /// Check if cargo-apk is installed
    pub async fn is_cargo_apk_available() -> bool {
        Command::new("cargo")
            .args(["apk", "--version"])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Install cargo-apk
    pub async fn install_cargo_apk() -> Result<(), BuildError> {
        info!("Installing cargo-apk...");
        
        let output = Command::new("cargo")
            .args(["install", "cargo-apk"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::BuildFailed(format!("Failed to install cargo-apk: {}", stderr)));
        }

        info!("cargo-apk installed successfully");
        Ok(())
    }

    /// Build the project
    pub async fn build(&self) -> Result<PathBuf, BuildError> {
        if !Self::is_cargo_apk_available().await {
            return Err(BuildError::ToolchainNotFound("cargo-apk not installed. Run `cargo install cargo-apk`".into()));
        }

        info!("Building Rust Android app...");

        let mut args = vec!["apk", "build"];

        // Add release flag if needed
        if self.config.variant == BuildVariant::Release {
            args.push("--release");
        }

        // Add target triple for each ABI
        for abi in &self.config.abis {
            if let Some(triple) = abi.rust_triple() {
                args.push("--target");
                args.push(triple);
            }
        }

        // Add features
        for feature in &self.cargo_config.features {
            args.push("--features");
            args.push(feature);
        }

        // Add extra args
        for arg in &self.config.extra_args {
            args.push(arg);
        }

        debug!("Running: cargo {:?}", args);

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&self.config.project_dir);
        cmd.args(&args);

        // Set environment variables
        if let Some(ref ndk_path) = self.ndk_path {
            cmd.env("ANDROID_NDK_HOME", ndk_path);
        }

        for (key, value) in &self.config.env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(BuildError::BuildFailed(format!("Build failed:\n{}\n{}", stdout, stderr)));
        }

        info!("Build completed successfully");

        // Find the output APK
        self.find_output_apk()
    }

    /// Build with progress reporting
    pub async fn build_with_progress(&self, tx: mpsc::Sender<BuildMessage>) -> Result<PathBuf, BuildError> {
        if !Self::is_cargo_apk_available().await {
            return Err(BuildError::ToolchainNotFound("cargo-apk not installed".into()));
        }

        let mut args = vec!["apk".to_string(), "build".to_string()];

        if self.config.variant == BuildVariant::Release {
            args.push("--release".to_string());
        }

        for abi in &self.config.abis {
            if let Some(triple) = abi.rust_triple() {
                args.push("--target".to_string());
                args.push(triple.to_string());
            }
        }

        let _ = tx.send(BuildMessage::Started).await;

        let mut child = Command::new("cargo")
            .current_dir(&self.config.project_dir)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Stream stderr (cargo writes to stderr)
        if let Some(stderr) = child.stderr.take() {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    let msg = parse_cargo_output(&line);
                    let _ = tx_clone.send(msg).await;
                }
            });
        }

        let status = child.wait().await?;

        if !status.success() {
            let _ = tx.send(BuildMessage::Failed("Build failed".into())).await;
            return Err(BuildError::BuildFailed("Cargo build failed".into()));
        }

        let apk_path = self.find_output_apk()?;
        let _ = tx.send(BuildMessage::Completed(apk_path.clone())).await;

        Ok(apk_path)
    }

    /// Find the output APK
    fn find_output_apk(&self) -> Result<PathBuf, BuildError> {
        let target_dir = self.config.project_dir.join("target");
        
        // Look for APK in target directory
        let search_dirs = vec![
            target_dir.join("debug").join("apk"),
            target_dir.join("release").join("apk"),
        ];

        for dir in search_dirs {
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.extension().map(|e| e == "apk").unwrap_or(false) {
                            return Ok(path);
                        }
                    }
                }
            }
        }

        Err(BuildError::BuildFailed("Could not find output APK".into()))
    }

    /// Clean build artifacts
    pub async fn clean(&self) -> Result<(), BuildError> {
        info!("Cleaning build artifacts...");
        
        let output = Command::new("cargo")
            .current_dir(&self.config.project_dir)
            .args(["clean"])
            .output()
            .await?;

        if !output.status.success() {
            warn!("Clean failed, but continuing...");
        }

        Ok(())
    }
}

/// Build message for progress reporting
#[derive(Debug, Clone)]
pub enum BuildMessage {
    Started,
    Compiling(String, String), // crate name, version
    Compiled(String),
    Linking,
    Packaging,
    Signing,
    Completed(PathBuf),
    Failed(String),
    Warning(String),
    Info(String),
}

/// Parse cargo output into build messages
fn parse_cargo_output(line: &str) -> BuildMessage {
    let line = line.trim();
    
    if line.starts_with("Compiling") {
        // "Compiling crate v0.1.0 (/path)"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            return BuildMessage::Compiling(
                parts[1].to_string(),
                parts[2].to_string(),
            );
        }
    }
    
    if line.starts_with("Finished") {
        return BuildMessage::Info(line.to_string());
    }
    
    if line.starts_with("warning:") {
        return BuildMessage::Warning(line.to_string());
    }
    
    if line.starts_with("error") {
        return BuildMessage::Failed(line.to_string());
    }
    
    BuildMessage::Info(line.to_string())
}

/// Create Cargo.toml for Android app
pub fn generate_cargo_toml(
    name: &str,
    package: &str,
    min_sdk: u32,
    target_sdk: u32,
) -> String {
    format!(r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# Android bindings
ndk = "0.8"
ndk-glue = "0.7"
log = "0.4"
android_logger = "0.13"

[package.metadata.android]
package = "{package}"
build_targets = ["aarch64-linux-android", "armv7-linux-androideabi"]

[package.metadata.android.sdk]
min_sdk_version = {min_sdk}
target_sdk_version = {target_sdk}

[[package.metadata.android.uses_permission]]
name = "android.permission.INTERNET"

[profile.release]
lto = true
opt-level = "z"
codegen-units = 1
panic = "abort"
"#)
}
