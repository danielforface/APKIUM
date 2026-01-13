//! Android Toolchain Management
//! 
//! Handles detection, download, and management of:
//! - Android SDK
//! - Android NDK
//! - OpenJDK
//! - Build tools and platform tools

pub mod detector;
pub mod downloader;
pub mod sdk_manager;
pub mod jdk;
pub mod ndk;
pub mod env;

pub use detector::{ToolchainDetector, SdkInfo, NdkInfo, JdkInfo};
pub use downloader::ToolchainDownloader;
pub use sdk_manager::SdkManager;
pub use jdk::JdkManager;
pub use ndk::{NdkManager, Abi, Toolchain};
pub use env::{EnvManager, EnvironmentConfig, EnvironmentValidation};

/// Supported Android API levels
pub const SUPPORTED_API_LEVELS: &[u32] = &[24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35];

/// Default target API level
pub const DEFAULT_TARGET_API: u32 = 34;

/// Default minimum API level
pub const DEFAULT_MIN_API: u32 = 24;

/// Supported ABIs
pub const SUPPORTED_ABIS: &[&str] = &["arm64-v8a", "armeabi-v7a", "x86", "x86_64"];
