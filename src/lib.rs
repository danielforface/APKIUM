//! R-Droid 2026 - Pure Rust Android IDE
//! 
//! A modern, fast, and feature-rich Android development environment
//! built entirely in Rust.
//! 
//! ## Features
//! 
//! - **Modern UI**: Slint-based interface with glassmorphism and fluid animations
//! - **Pure Rust**: Fast, safe, and memory-efficient
//! - **Auto Toolchain**: Automatic detection and download of Android SDK, NDK, and JDK
//! - **Manifest Editor**: Dual-mode visual and code editor with bi-directional sync
//! - **Emulator Integration**: One-click AVD setup and management
//! - **Build System**: One-click APK/AAB builds with signing support
//! 
//! ## Architecture
//! 
//! R-Droid is organized into specialized crates:
//! 
//! - `rdroid-core`: Core orchestration and configuration
//! - `rdroid-ui`: Slint-based user interface
//! - `rdroid-editor`: Code editor with syntax highlighting
//! - `rdroid-android-toolchain`: SDK/NDK/JDK management
//! - `rdroid-manifest-manager`: AndroidManifest.xml parsing and editing
//! - `rdroid-emulator-bridge`: Android emulator and device management
//! - `rdroid-build-engine`: Build system for APK/AAB generation

#![doc(html_root_url = "https://docs.rdroid.dev/")]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod commands;
pub mod project;

// Re-export main components for library usage
pub use rdroid_core as core;
pub use rdroid_android_toolchain as toolchain;
pub use rdroid_manifest_manager as manifest;
pub use rdroid_emulator_bridge as emulator;
pub use rdroid_build_engine as build;

/// Prelude module for convenient imports
pub mod prelude {
    pub use rdroid_core::config::AppConfig;
    pub use rdroid_android_toolchain::{ToolchainDetector, SdkManager};
    pub use rdroid_manifest_manager::{ManifestParser, ManifestWriter};
    pub use rdroid_emulator_bridge::{AdbClient, AvdManager, EmulatorLauncher};
    pub use rdroid_build_engine::{BuildRunner, BuildConfig};
}
