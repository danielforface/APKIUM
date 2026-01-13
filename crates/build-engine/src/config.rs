//! Build Configuration
//!
//! Defines build settings and variants.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Build variant (debug/release)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BuildVariant {
    #[default]
    Debug,
    Release,
}

impl BuildVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildVariant::Debug => "debug",
            BuildVariant::Release => "release",
        }
    }

    pub fn cargo_flag(&self) -> Option<&'static str> {
        match self {
            BuildVariant::Debug => None,
            BuildVariant::Release => Some("--release"),
        }
    }

    pub fn gradle_task_suffix(&self) -> &'static str {
        match self {
            BuildVariant::Debug => "Debug",
            BuildVariant::Release => "Release",
        }
    }
}

/// Build type (APK or AAB)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BuildType {
    #[default]
    Apk,
    Bundle, // AAB
}

impl BuildType {
    pub fn extension(&self) -> &'static str {
        match self {
            BuildType::Apk => "apk",
            BuildType::Bundle => "aab",
        }
    }

    pub fn gradle_task(&self, variant: BuildVariant) -> String {
        let suffix = variant.gradle_task_suffix();
        match self {
            BuildType::Apk => format!("assemble{}", suffix),
            BuildType::Bundle => format!("bundle{}", suffix),
        }
    }
}

/// Target ABI for build
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbiTarget {
    Arm64V8a,
    ArmeabiV7a,
    X86,
    X86_64,
    All,
}

impl AbiTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            AbiTarget::Arm64V8a => "arm64-v8a",
            AbiTarget::ArmeabiV7a => "armeabi-v7a",
            AbiTarget::X86 => "x86",
            AbiTarget::X86_64 => "x86_64",
            AbiTarget::All => "all",
        }
    }

    pub fn rust_triple(&self) -> Option<&'static str> {
        match self {
            AbiTarget::Arm64V8a => Some("aarch64-linux-android"),
            AbiTarget::ArmeabiV7a => Some("armv7-linux-androideabi"),
            AbiTarget::X86 => Some("i686-linux-android"),
            AbiTarget::X86_64 => Some("x86_64-linux-android"),
            AbiTarget::All => None,
        }
    }

    pub fn all_targets() -> &'static [AbiTarget] {
        &[
            AbiTarget::Arm64V8a,
            AbiTarget::ArmeabiV7a,
            AbiTarget::X86,
            AbiTarget::X86_64,
        ]
    }
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Project root directory
    pub project_dir: PathBuf,
    
    /// Build variant
    pub variant: BuildVariant,
    
    /// Build type
    pub build_type: BuildType,
    
    /// Target ABIs
    pub abis: Vec<AbiTarget>,
    
    /// Minimum SDK version
    pub min_sdk: u32,
    
    /// Target SDK version
    pub target_sdk: u32,
    
    /// Output directory
    pub output_dir: Option<PathBuf>,
    
    /// Signing configuration
    pub signing: Option<SigningInfo>,
    
    /// Additional environment variables
    pub env_vars: std::collections::HashMap<String, String>,
    
    /// Extra build arguments
    pub extra_args: Vec<String>,
    
    /// Clean before build
    pub clean_first: bool,
    
    /// Parallel jobs
    pub jobs: Option<u32>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            project_dir: PathBuf::from("."),
            variant: BuildVariant::Debug,
            build_type: BuildType::Apk,
            abis: vec![AbiTarget::Arm64V8a],
            min_sdk: 24,
            target_sdk: 34,
            output_dir: None,
            signing: None,
            env_vars: std::collections::HashMap::new(),
            extra_args: Vec::new(),
            clean_first: false,
            jobs: None,
        }
    }
}

impl BuildConfig {
    /// Create for development (debug, single ABI)
    pub fn development(project_dir: PathBuf) -> Self {
        Self {
            project_dir,
            variant: BuildVariant::Debug,
            abis: vec![AbiTarget::Arm64V8a],
            ..Default::default()
        }
    }

    /// Create for release (all ABIs)
    pub fn release(project_dir: PathBuf) -> Self {
        Self {
            project_dir,
            variant: BuildVariant::Release,
            abis: AbiTarget::all_targets().to_vec(),
            ..Default::default()
        }
    }

    /// Get effective output directory
    pub fn effective_output_dir(&self) -> PathBuf {
        self.output_dir
            .clone()
            .unwrap_or_else(|| self.project_dir.join("target").join("android"))
    }

    /// Get expected APK/AAB output path
    pub fn expected_output_path(&self, name: &str) -> PathBuf {
        let filename = format!(
            "{}-{}.{}",
            name,
            self.variant.as_str(),
            self.build_type.extension()
        );
        self.effective_output_dir().join(filename)
    }
}

/// Signing information (for build config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningInfo {
    pub keystore_path: PathBuf,
    pub keystore_password: String,
    pub key_alias: String,
    pub key_password: String,
}

/// Gradle project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradleConfig {
    /// Path to gradlew/gradlew.bat
    pub gradle_wrapper: PathBuf,
    
    /// Module to build (default: app)
    pub module: String,
    
    /// Flavor (if using product flavors)
    pub flavor: Option<String>,
    
    /// Gradle properties
    pub properties: std::collections::HashMap<String, String>,
    
    /// Gradle JVM arguments
    pub jvm_args: Vec<String>,
}

impl Default for GradleConfig {
    fn default() -> Self {
        Self {
            gradle_wrapper: PathBuf::from("gradlew"),
            module: "app".to_string(),
            flavor: None,
            properties: std::collections::HashMap::new(),
            jvm_args: vec!["-Xmx2048m".to_string()],
        }
    }
}

impl GradleConfig {
    /// Get the full task name
    pub fn task_name(&self, build_type: BuildType, variant: BuildVariant) -> String {
        let base_task = build_type.gradle_task(variant);
        
        if let Some(ref flavor) = self.flavor {
            // Capitalize first letter of flavor
            let flavor_cap = format!(
                "{}{}",
                flavor.chars().next().unwrap().to_uppercase(),
                &flavor[1..]
            );
            format!(":{}:{}{}", self.module, flavor_cap, base_task)
        } else {
            format!(":{}:{}", self.module, base_task)
        }
    }
}

/// Cargo Android configuration (cargo-apk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoApkConfig {
    /// Package name override
    pub package_name: Option<String>,
    
    /// Version code override
    pub version_code: Option<u32>,
    
    /// Version name override
    pub version_name: Option<String>,
    
    /// Application label
    pub label: Option<String>,
    
    /// Icon resource path
    pub icon: Option<PathBuf>,
    
    /// Features to enable
    pub features: Vec<String>,
    
    /// Whether to strip debug symbols
    pub strip: bool,
}

impl Default for CargoApkConfig {
    fn default() -> Self {
        Self {
            package_name: None,
            version_code: None,
            version_name: None,
            label: None,
            icon: None,
            features: Vec::new(),
            strip: true,
        }
    }
}
