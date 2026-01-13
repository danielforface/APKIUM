//! Project management for R-Droid
//! 
//! Handles project creation, loading, and configuration.

use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tracing::info;

/// Project type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    /// Pure Rust Android app (using cargo-apk)
    RustAndroid,
    /// Kotlin Android app (using Gradle)
    KotlinAndroid,
    /// Java Android app (using Gradle)
    JavaAndroid,
    /// Flutter app with Rust components
    FlutterRust,
    /// React Native with Rust native modules
    ReactNativeRust,
}

impl Default for ProjectType {
    fn default() -> Self {
        Self::RustAndroid
    }
}

impl ProjectType {
    /// Get the display name for the project type
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::RustAndroid => "Rust Android App",
            Self::KotlinAndroid => "Kotlin Android App",
            Self::JavaAndroid => "Java Android App",
            Self::FlutterRust => "Flutter + Rust",
            Self::ReactNativeRust => "React Native + Rust",
        }
    }
    
    /// Get the template identifier
    pub fn template_id(&self) -> &'static str {
        match self {
            Self::RustAndroid => "rust-android",
            Self::KotlinAndroid => "kotlin-android",
            Self::JavaAndroid => "java-android",
            Self::FlutterRust => "flutter-rust",
            Self::ReactNativeRust => "react-native-rust",
        }
    }
}

/// R-Droid project configuration (rdroid.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Project type
    #[serde(default)]
    pub project_type: ProjectType,
    /// Package ID (e.g., com.example.myapp)
    pub package_id: String,
    /// Minimum SDK version
    #[serde(default = "default_min_sdk")]
    pub min_sdk: u32,
    /// Target SDK version
    #[serde(default = "default_target_sdk")]
    pub target_sdk: u32,
    /// Version code (Android internal version)
    #[serde(default = "default_version_code")]
    pub version_code: u32,
    /// Target ABIs
    #[serde(default)]
    pub target_abis: Vec<String>,
    /// Build configuration
    #[serde(default)]
    pub build: BuildConfig,
    /// Signing configuration
    #[serde(default)]
    pub signing: SigningConfig,
}

fn default_min_sdk() -> u32 { 24 }
fn default_target_sdk() -> u32 { 34 }
fn default_version_code() -> u32 { 1 }

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "my-app".to_string(),
            version: "0.1.0".to_string(),
            project_type: ProjectType::default(),
            package_id: "com.example.myapp".to_string(),
            min_sdk: default_min_sdk(),
            target_sdk: default_target_sdk(),
            version_code: default_version_code(),
            target_abis: vec![
                "arm64-v8a".to_string(),
                "armeabi-v7a".to_string(),
            ],
            build: BuildConfig::default(),
            signing: SigningConfig::default(),
        }
    }
}

/// Build configuration section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Enable release optimizations
    #[serde(default)]
    pub release_optimizations: bool,
    /// Strip debug symbols in release
    #[serde(default = "default_strip")]
    pub strip: bool,
    /// Enable LTO (Link Time Optimization)
    #[serde(default)]
    pub lto: bool,
    /// Custom build features
    #[serde(default)]
    pub features: Vec<String>,
}

fn default_strip() -> bool { true }

/// Signing configuration section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SigningConfig {
    /// Path to keystore file
    pub keystore: Option<PathBuf>,
    /// Key alias
    pub key_alias: Option<String>,
    /// Enable V2 signing
    #[serde(default = "default_true")]
    pub v2_signing: bool,
    /// Enable V3 signing
    #[serde(default)]
    pub v3_signing: bool,
}

fn default_true() -> bool { true }

/// Project manager for R-Droid
pub struct ProjectManager {
    config_filename: String,
}

impl ProjectManager {
    /// Create a new project manager
    pub fn new() -> Self {
        Self {
            config_filename: "rdroid.toml".to_string(),
        }
    }
    
    /// Load project configuration from directory
    pub fn load(&self, project_dir: &Path) -> Result<ProjectConfig> {
        let config_path = project_dir.join(&self.config_filename);
        
        if !config_path.exists() {
            return Err(anyhow::anyhow!(
                "No rdroid.toml found in {:?}. Is this an R-Droid project?",
                project_dir
            ));
        }
        
        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read rdroid.toml")?;
        
        let config: ProjectConfig = toml::from_str(&content)
            .context("Failed to parse rdroid.toml")?;
        
        Ok(config)
    }
    
    /// Save project configuration
    pub fn save(&self, project_dir: &Path, config: &ProjectConfig) -> Result<()> {
        let config_path = project_dir.join(&self.config_filename);
        
        let content = toml::to_string_pretty(config)
            .context("Failed to serialize configuration")?;
        
        std::fs::write(&config_path, content)
            .context("Failed to write rdroid.toml")?;
        
        Ok(())
    }
    
    /// Create a new project
    pub fn create(&self, parent_dir: &Path, config: &ProjectConfig) -> Result<PathBuf> {
        let project_dir = parent_dir.join(&config.name);
        
        if project_dir.exists() {
            return Err(anyhow::anyhow!(
                "Directory already exists: {:?}",
                project_dir
            ));
        }
        
        info!("Creating project: {:?}", project_dir);
        
        // Create project directory
        std::fs::create_dir_all(&project_dir)
            .context("Failed to create project directory")?;
        
        // Generate project structure based on type
        match config.project_type {
            ProjectType::RustAndroid => self.create_rust_project(&project_dir, config)?,
            ProjectType::KotlinAndroid => self.create_kotlin_project(&project_dir, config)?,
            ProjectType::JavaAndroid => self.create_java_project(&project_dir, config)?,
            ProjectType::FlutterRust => self.create_flutter_rust_project(&project_dir, config)?,
            ProjectType::ReactNativeRust => self.create_react_native_rust_project(&project_dir, config)?,
        }
        
        // Save rdroid.toml
        self.save(&project_dir, config)?;
        
        info!("Project created successfully!");
        Ok(project_dir)
    }
    
    fn create_rust_project(&self, dir: &Path, config: &ProjectConfig) -> Result<()> {
        // Create directory structure
        std::fs::create_dir_all(dir.join("src"))?;
        std::fs::create_dir_all(dir.join("res/values"))?;
        std::fs::create_dir_all(dir.join("res/layout"))?;
        std::fs::create_dir_all(dir.join("res/mipmap-xxxhdpi"))?;
        
        // Create Cargo.toml
        let cargo_toml = format!(r#"[package]
name = "{}"
version = "{}"
edition = "2021"

[dependencies]
ndk = "0.8"
ndk-glue = "0.7"
log = "0.4"
android_logger = "0.13"

[lib]
crate-type = ["cdylib"]

[package.metadata.android]
package = "{}"
min_sdk_version = {}
target_sdk_version = {}
label = "{}"
"#, config.name, config.version, config.package_id, config.min_sdk, config.target_sdk, config.name);
        
        std::fs::write(dir.join("Cargo.toml"), cargo_toml)?;
        
        // Create lib.rs
        let lib_rs = r#"//! Android application entry point

use ndk_glue::Event;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
    );
    
    log::info!("Application started!");
    
    loop {
        match ndk_glue::poll_events() {
            Some(Event::Start) => {
                log::info!("App started");
            }
            Some(Event::Resume) => {
                log::info!("App resumed");
            }
            Some(Event::Pause) => {
                log::info!("App paused");
            }
            Some(Event::Stop) => {
                log::info!("App stopped");
                break;
            }
            _ => {}
        }
    }
}
"#;
        std::fs::write(dir.join("src/lib.rs"), lib_rs)?;
        
        // Create AndroidManifest.xml
        let manifest = format!(r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{}">
    
    <application
        android:label="{}"
        android:hasCode="false">
        
        <activity
            android:name="android.app.NativeActivity"
            android:label="{}"
            android:configChanges="orientation|screenSize|keyboardHidden"
            android:exported="true">
            
            <meta-data
                android:name="android.app.lib_name"
                android:value="{}" />
            
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#, config.package_id, config.name, config.name, config.name.replace('-', "_"));
        
        std::fs::write(dir.join("AndroidManifest.xml"), manifest)?;
        
        // Create strings.xml
        let strings_xml = format!(r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">{}</string>
</resources>
"#, config.name);
        
        std::fs::write(dir.join("res/values/strings.xml"), strings_xml)?;
        
        // Create .gitignore
        let gitignore = r#"/target
/Cargo.lock
*.apk
*.aab
*.keystore
.idea/
*.iml
"#;
        std::fs::write(dir.join(".gitignore"), gitignore)?;
        
        Ok(())
    }
    
    fn create_kotlin_project(&self, dir: &Path, config: &ProjectConfig) -> Result<()> {
        let package_path = config.package_id.replace('.', "/");
        
        // Create directory structure
        std::fs::create_dir_all(dir.join(format!("app/src/main/java/{}", package_path)))?;
        std::fs::create_dir_all(dir.join("app/src/main/res/values"))?;
        std::fs::create_dir_all(dir.join("app/src/main/res/layout"))?;
        std::fs::create_dir_all(dir.join("gradle/wrapper"))?;
        
        // Create settings.gradle.kts
        let settings_gradle = format!(r#"rootProject.name = "{}"
include(":app")
"#, config.name);
        std::fs::write(dir.join("settings.gradle.kts"), settings_gradle)?;
        
        // Create build.gradle.kts (root)
        let root_build_gradle = r#"plugins {
    id("com.android.application") version "8.2.0" apply false
    id("org.jetbrains.kotlin.android") version "1.9.22" apply false
}
"#;
        std::fs::write(dir.join("build.gradle.kts"), root_build_gradle)?;
        
        // Create app/build.gradle.kts
        let app_build_gradle = format!(r#"plugins {{
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}}

android {{
    namespace = "{}"
    compileSdk = {}
    
    defaultConfig {{
        applicationId = "{}"
        minSdk = {}
        targetSdk = {}
        versionCode = {}
        versionName = "{}"
    }}
    
    buildTypes {{
        release {{
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }}
    }}
    
    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }}
    
    kotlinOptions {{
        jvmTarget = "17"
    }}
}}

dependencies {{
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.11.0")
}}
"#, config.package_id, config.target_sdk, config.package_id, 
    config.min_sdk, config.target_sdk, config.version_code, config.version);
        
        std::fs::write(dir.join("app/build.gradle.kts"), app_build_gradle)?;
        
        // Create MainActivity.kt
        let main_activity = format!(r#"package {}

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {{
    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
    }}
}}
"#, config.package_id);
        
        std::fs::write(
            dir.join(format!("app/src/main/java/{}/MainActivity.kt", package_path)),
            main_activity
        )?;
        
        // Create AndroidManifest.xml
        let manifest = format!(r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    
    <application
        android:label="@string/app_name"
        android:theme="@style/Theme.AppCompat.Light.DarkActionBar">
        
        <activity
            android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#);
        
        std::fs::write(dir.join("app/src/main/AndroidManifest.xml"), manifest)?;
        
        // Create layout
        let activity_main = r#"<?xml version="1.0" encoding="utf-8"?>
<androidx.constraintlayout.widget.ConstraintLayout
    xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:app="http://schemas.android.com/apk/res-auto"
    android:layout_width="match_parent"
    android:layout_height="match_parent">
    
    <TextView
        android:layout_width="wrap_content"
        android:layout_height="wrap_content"
        android:text="Hello, R-Droid!"
        app:layout_constraintBottom_toBottomOf="parent"
        app:layout_constraintEnd_toEndOf="parent"
        app:layout_constraintStart_toStartOf="parent"
        app:layout_constraintTop_toTopOf="parent" />

</androidx.constraintlayout.widget.ConstraintLayout>
"#;
        
        std::fs::write(dir.join("app/src/main/res/layout/activity_main.xml"), activity_main)?;
        
        // Create strings.xml
        let strings_xml = format!(r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">{}</string>
</resources>
"#, config.name);
        
        std::fs::write(dir.join("app/src/main/res/values/strings.xml"), strings_xml)?;
        
        // Create .gitignore
        let gitignore = r#".gradle/
build/
*.apk
*.aab
*.keystore
.idea/
*.iml
local.properties
"#;
        std::fs::write(dir.join(".gitignore"), gitignore)?;
        
        Ok(())
    }
    
    fn create_java_project(&self, dir: &Path, config: &ProjectConfig) -> Result<()> {
        // Similar to Kotlin but with Java files
        // For brevity, delegate to Kotlin and convert
        self.create_kotlin_project(dir, config)?;
        
        // Convert MainActivity.kt to MainActivity.java
        let package_path = config.package_id.replace('.', "/");
        let kt_path = dir.join(format!("app/src/main/java/{}/MainActivity.kt", package_path));
        
        if kt_path.exists() {
            std::fs::remove_file(&kt_path)?;
        }
        
        let main_activity = format!(r#"package {};

import android.os.Bundle;
import androidx.appcompat.app.AppCompatActivity;

public class MainActivity extends AppCompatActivity {{
    @Override
    protected void onCreate(Bundle savedInstanceState) {{
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
    }}
}}
"#, config.package_id);
        
        std::fs::write(
            dir.join(format!("app/src/main/java/{}/MainActivity.java", package_path)),
            main_activity
        )?;
        
        Ok(())
    }
    
    fn create_flutter_rust_project(&self, dir: &Path, _config: &ProjectConfig) -> Result<()> {
        // Create placeholder structure
        std::fs::create_dir_all(dir.join("lib"))?;
        std::fs::create_dir_all(dir.join("native/src"))?;
        
        // Create README with instructions
        let readme = r#"# Flutter + Rust Project

This project combines Flutter for UI with Rust for performance-critical code.

## Setup

1. Install Flutter: https://flutter.dev/docs/get-started/install
2. Install `flutter_rust_bridge`: `cargo install flutter_rust_bridge_codegen`
3. Run `flutter pub get`
4. Generate bindings: `flutter_rust_bridge_codegen generate`

## Build

```bash
flutter build apk
```
"#;
        std::fs::write(dir.join("README.md"), readme)?;
        
        Ok(())
    }
    
    fn create_react_native_rust_project(&self, dir: &Path, _config: &ProjectConfig) -> Result<()> {
        // Create placeholder structure
        std::fs::create_dir_all(dir.join("src"))?;
        std::fs::create_dir_all(dir.join("native/src"))?;
        
        // Create README with instructions
        let readme = r#"# React Native + Rust Project

This project combines React Native for UI with Rust native modules.

## Setup

1. Install Node.js and npm
2. Install React Native CLI: `npm install -g react-native-cli`
3. Run `npm install`

## Build

```bash
npx react-native run-android
```
"#;
        std::fs::write(dir.join("README.md"), readme)?;
        
        Ok(())
    }
    
    /// Check if a directory contains an R-Droid project
    pub fn is_project(&self, dir: &Path) -> bool {
        dir.join(&self.config_filename).exists()
    }
    
    /// Detect project type from directory contents
    pub fn detect_type(&self, dir: &Path) -> Option<ProjectType> {
        // Check for rdroid.toml first
        if let Ok(config) = self.load(dir) {
            return Some(config.project_type);
        }
        
        // Check for Cargo.toml with android metadata
        if dir.join("Cargo.toml").exists() {
            if let Ok(content) = std::fs::read_to_string(dir.join("Cargo.toml")) {
                if content.contains("[package.metadata.android]") {
                    return Some(ProjectType::RustAndroid);
                }
            }
        }
        
        // Check for Gradle projects
        if dir.join("build.gradle.kts").exists() || dir.join("build.gradle").exists() {
            // Check for Kotlin sources
            if dir.join("app/src/main/java").exists() || dir.join("app/src/main/kotlin").exists() {
                // Would need to check file extensions to distinguish
                return Some(ProjectType::KotlinAndroid);
            }
        }
        
        // Check for Flutter
        if dir.join("pubspec.yaml").exists() {
            return Some(ProjectType::FlutterRust);
        }
        
        // Check for React Native
        if dir.join("package.json").exists() {
            if let Ok(content) = std::fs::read_to_string(dir.join("package.json")) {
                if content.contains("react-native") {
                    return Some(ProjectType::ReactNativeRust);
                }
            }
        }
        
        None
    }
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}
