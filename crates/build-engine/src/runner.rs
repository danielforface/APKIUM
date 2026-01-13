//! Build Runner
//!
//! Coordinates the entire build process.

use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, debug, error};

use crate::{
    BuildConfig, BuildError, BuildSystem, detect_build_system,
    CargoBuild, GradleBuild,
    signing::{SigningConfig, ApkSigner},
    apk::ApkInfo,
    cargo_build::BuildMessage,
};

/// Build output
#[derive(Debug, Clone)]
pub struct BuildOutput {
    /// Path to the output APK/AAB
    pub path: PathBuf,
    /// Build duration in seconds
    pub duration_secs: f64,
    /// APK/AAB size in bytes
    pub size: u64,
    /// Was signed
    pub signed: bool,
    /// Build variant used
    pub variant: String,
    /// ABIs included
    pub abis: Vec<String>,
}

/// Build progress
#[derive(Debug, Clone)]
pub enum BuildProgress {
    Started,
    Cleaning,
    Compiling { current: u32, total: u32, target: String },
    Linking,
    Packaging,
    Signing,
    Completed { output: BuildOutput },
    Failed { error: String },
}

/// Build runner that coordinates the build process
pub struct BuildRunner {
    config: BuildConfig,
    sdk_path: PathBuf,
    ndk_path: Option<PathBuf>,
    java_home: Option<PathBuf>,
}

impl BuildRunner {
    /// Create a new build runner
    pub fn new(config: BuildConfig, sdk_path: PathBuf) -> Self {
        Self {
            config,
            sdk_path,
            ndk_path: None,
            java_home: None,
        }
    }

    /// Set NDK path
    pub fn with_ndk(mut self, path: PathBuf) -> Self {
        self.ndk_path = Some(path);
        self
    }

    /// Set JAVA_HOME
    pub fn with_java_home(mut self, path: PathBuf) -> Self {
        self.java_home = Some(path);
        self
    }

    /// Run the build
    pub async fn build(&self) -> Result<BuildOutput, BuildError> {
        let start = std::time::Instant::now();
        
        info!("Starting build for {:?}", self.config.project_dir);

        // Detect build system
        let build_system = detect_build_system(&self.config.project_dir)
            .ok_or_else(|| BuildError::ConfigError("Could not detect build system".into()))?;

        // Clean if requested
        if self.config.clean_first {
            self.clean(build_system).await?;
        }

        // Run the build
        let apk_path = match build_system {
            BuildSystem::CargoApk => {
                let mut builder = CargoBuild::new(self.config.clone());
                if let Some(ref ndk) = self.ndk_path {
                    builder = builder.with_ndk(ndk.clone());
                }
                builder.build().await?
            }
            BuildSystem::Gradle => {
                let mut builder = GradleBuild::new(self.config.clone());
                if let Some(ref java) = self.java_home {
                    builder = builder.with_java_home(java.clone());
                }
                builder = builder.with_android_home(self.sdk_path.clone());
                builder.build().await?
            }
            BuildSystem::NdkBuild => {
                return Err(BuildError::ConfigError("NDK-build not yet supported".into()));
            }
        };

        // Sign if signing config provided and this is a release build
        let signed = if let Some(ref signing_info) = self.config.signing {
            let keystore = crate::signing::KeyStore::new(
                signing_info.keystore_path.clone(),
                &signing_info.keystore_password,
                &signing_info.key_alias,
            );
            let signing_config = SigningConfig::new(keystore);
            let signer = ApkSigner::new(self.sdk_path.clone());
            signer.sign_in_place(&apk_path, &signing_config).await?;
            true
        } else {
            false
        };

        let duration = start.elapsed().as_secs_f64();
        let metadata = std::fs::metadata(&apk_path)?;

        let output = BuildOutput {
            path: apk_path,
            duration_secs: duration,
            size: metadata.len(),
            signed,
            variant: self.config.variant.as_str().to_string(),
            abis: self.config.abis.iter().map(|a| a.as_str().to_string()).collect(),
        };

        info!("Build completed in {:.2}s", duration);
        Ok(output)
    }

    /// Build with progress reporting
    pub async fn build_with_progress(&self, tx: mpsc::Sender<BuildProgress>) -> Result<BuildOutput, BuildError> {
        let start = std::time::Instant::now();
        
        let _ = tx.send(BuildProgress::Started).await;

        let build_system = detect_build_system(&self.config.project_dir)
            .ok_or_else(|| BuildError::ConfigError("Could not detect build system".into()))?;

        if self.config.clean_first {
            let _ = tx.send(BuildProgress::Cleaning).await;
            self.clean(build_system).await?;
        }

        // Create a channel for build messages
        let (build_tx, mut build_rx) = mpsc::channel::<BuildMessage>(100);
        let tx_clone = tx.clone();
        
        // Forward build messages to progress
        tokio::spawn(async move {
            let mut compiled_count = 0u32;
            while let Some(msg) = build_rx.recv().await {
                let progress = match msg {
                    BuildMessage::Compiling(name, _) => {
                        compiled_count += 1;
                        BuildProgress::Compiling {
                            current: compiled_count,
                            total: 0, // Unknown total
                            target: name,
                        }
                    }
                    BuildMessage::Linking => BuildProgress::Linking,
                    BuildMessage::Packaging => BuildProgress::Packaging,
                    BuildMessage::Signing => BuildProgress::Signing,
                    BuildMessage::Failed(err) => BuildProgress::Failed { error: err },
                    _ => continue,
                };
                let _ = tx_clone.send(progress).await;
            }
        });

        let apk_path = match build_system {
            BuildSystem::CargoApk => {
                let mut builder = CargoBuild::new(self.config.clone());
                if let Some(ref ndk) = self.ndk_path {
                    builder = builder.with_ndk(ndk.clone());
                }
                builder.build_with_progress(build_tx).await?
            }
            BuildSystem::Gradle => {
                let mut builder = GradleBuild::new(self.config.clone());
                if let Some(ref java) = self.java_home {
                    builder = builder.with_java_home(java.clone());
                }
                builder = builder.with_android_home(self.sdk_path.clone());
                builder.build_with_progress(build_tx).await?
            }
            BuildSystem::NdkBuild => {
                return Err(BuildError::ConfigError("NDK-build not yet supported".into()));
            }
        };

        // Sign if needed
        let signed = if let Some(ref signing_info) = self.config.signing {
            let _ = tx.send(BuildProgress::Signing).await;
            let keystore = crate::signing::KeyStore::new(
                signing_info.keystore_path.clone(),
                &signing_info.keystore_password,
                &signing_info.key_alias,
            );
            let signing_config = SigningConfig::new(keystore);
            let signer = ApkSigner::new(self.sdk_path.clone());
            signer.sign_in_place(&apk_path, &signing_config).await?;
            true
        } else {
            false
        };

        let duration = start.elapsed().as_secs_f64();
        let metadata = std::fs::metadata(&apk_path)?;

        let output = BuildOutput {
            path: apk_path,
            duration_secs: duration,
            size: metadata.len(),
            signed,
            variant: self.config.variant.as_str().to_string(),
            abis: self.config.abis.iter().map(|a| a.as_str().to_string()).collect(),
        };

        let _ = tx.send(BuildProgress::Completed { output: output.clone() }).await;
        Ok(output)
    }

    /// Clean build artifacts
    async fn clean(&self, build_system: BuildSystem) -> Result<(), BuildError> {
        match build_system {
            BuildSystem::CargoApk => {
                CargoBuild::new(self.config.clone()).clean().await
            }
            BuildSystem::Gradle => {
                GradleBuild::new(self.config.clone()).clean().await
            }
            BuildSystem::NdkBuild => Ok(()),
        }
    }

    /// Install on device
    pub async fn install(&self, device_serial: &str) -> Result<(), BuildError> {
        let build_system = detect_build_system(&self.config.project_dir)
            .ok_or_else(|| BuildError::ConfigError("Could not detect build system".into()))?;

        match build_system {
            BuildSystem::Gradle => {
                let mut builder = GradleBuild::new(self.config.clone());
                if let Some(ref java) = self.java_home {
                    builder = builder.with_java_home(java.clone());
                }
                builder = builder.with_android_home(self.sdk_path.clone());
                builder.install(Some(device_serial)).await
            }
            _ => {
                // Use ADB for other build systems
                let output = self.build().await?;
                let adb_path = self.sdk_path.join("platform-tools")
                    .join(if cfg!(windows) { "adb.exe" } else { "adb" });
                
                let cmd_output = tokio::process::Command::new(&adb_path)
                    .args(["-s", device_serial, "install", "-r"])
                    .arg(&output.path)
                    .output()
                    .await?;

                if !cmd_output.status.success() {
                    let stderr = String::from_utf8_lossy(&cmd_output.stderr);
                    return Err(BuildError::BuildFailed(format!("Install failed: {}", stderr)));
                }

                Ok(())
            }
        }
    }

    /// Build and run on device
    pub async fn run(&self, device_serial: &str, activity: Option<&str>) -> Result<(), BuildError> {
        // Build
        let output = self.build().await?;
        
        // Install
        let adb_path = self.sdk_path.join("platform-tools")
            .join(if cfg!(windows) { "adb.exe" } else { "adb" });

        let install_output = tokio::process::Command::new(&adb_path)
            .args(["-s", device_serial, "install", "-r"])
            .arg(&output.path)
            .output()
            .await?;

        if !install_output.status.success() {
            let stderr = String::from_utf8_lossy(&install_output.stderr);
            return Err(BuildError::BuildFailed(format!("Install failed: {}", stderr)));
        }

        // Launch if activity specified
        if let Some(activity) = activity {
            let launch_output = tokio::process::Command::new(&adb_path)
                .args(["-s", device_serial, "shell", "am", "start", "-n"])
                .arg(activity)
                .output()
                .await?;

            if !launch_output.status.success() {
                let stderr = String::from_utf8_lossy(&launch_output.stderr);
                return Err(BuildError::BuildFailed(format!("Launch failed: {}", stderr)));
            }
        }

        Ok(())
    }
}

/// One-click build helper
pub async fn one_click_build(
    project_dir: PathBuf,
    sdk_path: PathBuf,
    variant: crate::BuildVariant,
) -> Result<BuildOutput, BuildError> {
    let config = BuildConfig {
        project_dir,
        variant,
        ..Default::default()
    };

    let runner = BuildRunner::new(config, sdk_path);
    runner.build().await
}

/// One-click build and install
pub async fn one_click_run(
    project_dir: PathBuf,
    sdk_path: PathBuf,
    device_serial: &str,
    main_activity: &str,
) -> Result<(), BuildError> {
    let config = BuildConfig {
        project_dir,
        variant: crate::BuildVariant::Debug,
        ..Default::default()
    };

    let runner = BuildRunner::new(config, sdk_path);
    runner.run(device_serial, Some(main_activity)).await
}
