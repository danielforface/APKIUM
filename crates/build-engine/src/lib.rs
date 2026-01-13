//! Android Build Engine
//!
//! Handles building Android apps using Cargo (Rust) or Gradle (Kotlin/Java).

pub mod config;
pub mod cargo_build;
pub mod gradle_build;
pub mod apk;
pub mod signing;
pub mod runner;

pub use config::{BuildConfig, BuildVariant, BuildType, AbiTarget};
pub use cargo_build::CargoBuild;
pub use gradle_build::GradleBuild;
pub use apk::{ApkInfo, ApkAnalyzer};
pub use signing::{KeyStore, SigningConfig, ApkSigner};
pub use runner::{BuildRunner, BuildOutput, BuildProgress};

use std::path::PathBuf;

/// Build errors
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Build failed: {0}")]
    BuildFailed(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Toolchain not found: {0}")]
    ToolchainNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Signing error: {0}")]
    SigningError(String),
}

/// Build system type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildSystem {
    /// Cargo with cargo-apk for Rust Android apps
    CargoApk,
    /// Gradle for traditional Android (Kotlin/Java)
    Gradle,
    /// Manual NDK build
    NdkBuild,
}

/// Detect build system from project
pub fn detect_build_system(project_dir: &PathBuf) -> Option<BuildSystem> {
    // Check for Cargo.toml (Rust project)
    if project_dir.join("Cargo.toml").exists() {
        return Some(BuildSystem::CargoApk);
    }

    // Check for build.gradle or build.gradle.kts (Gradle project)
    if project_dir.join("build.gradle").exists() || 
       project_dir.join("build.gradle.kts").exists() {
        return Some(BuildSystem::Gradle);
    }

    // Check for Android.mk (NDK build)
    if project_dir.join("jni").join("Android.mk").exists() {
        return Some(BuildSystem::NdkBuild);
    }

    None
}
