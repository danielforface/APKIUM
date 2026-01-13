//! Project Management
//! 
//! Handles Android project structure, metadata, and operations.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{Result, RDroidError};

/// Project type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectType {
    /// Native Rust Android project (using cargo-apk)
    RustNative,
    /// Standard Kotlin/Java Android project
    KotlinJava,
    /// Flutter project
    Flutter,
    /// React Native project
    ReactNative,
    /// Unknown project type
    Unknown,
}

/// Build variant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuildVariant {
    Debug,
    Release,
    Custom(String),
}

impl Default for BuildVariant {
    fn default() -> Self {
        BuildVariant::Debug
    }
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,
    /// Package name (e.g., com.example.app)
    pub package_name: String,
    /// Version name
    pub version_name: String,
    /// Version code
    pub version_code: u32,
    /// Minimum SDK version
    pub min_sdk: u32,
    /// Target SDK version
    pub target_sdk: u32,
    /// Compile SDK version
    pub compile_sdk: u32,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        Self {
            name: "MyApp".to_string(),
            package_name: "com.example.myapp".to_string(),
            version_name: "1.0.0".to_string(),
            version_code: 1,
            min_sdk: 24,
            target_sdk: 34,
            compile_sdk: 34,
        }
    }
}

/// Android project representation
#[derive(Debug, Clone)]
pub struct Project {
    /// Project root directory
    pub root: PathBuf,
    /// Project type
    pub project_type: ProjectType,
    /// Project metadata
    pub metadata: ProjectMetadata,
    /// Current build variant
    pub build_variant: BuildVariant,
    /// Whether the project has been modified
    pub is_dirty: bool,
}

impl Project {
    /// Open an existing project
    pub async fn open(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(RDroidError::NotFound(format!("Project path not found: {:?}", path)));
        }

        let project_type = Self::detect_project_type(&path).await?;
        let metadata = Self::load_metadata(&path, &project_type).await?;

        info!("Opened project: {} ({:?})", metadata.name, project_type);

        Ok(Self {
            root: path,
            project_type,
            metadata,
            build_variant: BuildVariant::default(),
            is_dirty: false,
        })
    }

    /// Create a new project
    pub async fn create(path: PathBuf, project_type: ProjectType, metadata: ProjectMetadata) -> Result<Self> {
        if path.exists() {
            return Err(RDroidError::Project(format!("Path already exists: {:?}", path)));
        }

        tokio::fs::create_dir_all(&path).await?;

        let project = Self {
            root: path.clone(),
            project_type: project_type.clone(),
            metadata: metadata.clone(),
            build_variant: BuildVariant::default(),
            is_dirty: false,
        };

        // Generate project structure based on type
        match project_type {
            ProjectType::RustNative => project.create_rust_native_structure().await?,
            ProjectType::KotlinJava => project.create_kotlin_java_structure().await?,
            _ => return Err(RDroidError::Project("Unsupported project type for creation".into())),
        }

        info!("Created new project: {} at {:?}", metadata.name, path);

        Ok(project)
    }

    /// Detect project type from directory structure
    async fn detect_project_type(path: &PathBuf) -> Result<ProjectType> {
        // Check for Cargo.toml (Rust project)
        if path.join("Cargo.toml").exists() {
            return Ok(ProjectType::RustNative);
        }

        // Check for build.gradle or build.gradle.kts (Kotlin/Java project)
        if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
            return Ok(ProjectType::KotlinJava);
        }

        // Check for pubspec.yaml (Flutter project)
        if path.join("pubspec.yaml").exists() {
            return Ok(ProjectType::Flutter);
        }

        // Check for package.json with react-native (React Native project)
        if path.join("package.json").exists() {
            let package_json = tokio::fs::read_to_string(path.join("package.json")).await?;
            if package_json.contains("react-native") {
                return Ok(ProjectType::ReactNative);
            }
        }

        Ok(ProjectType::Unknown)
    }

    /// Load project metadata
    async fn load_metadata(path: &PathBuf, project_type: &ProjectType) -> Result<ProjectMetadata> {
        match project_type {
            ProjectType::RustNative => Self::load_rust_metadata(path).await,
            ProjectType::KotlinJava => Self::load_gradle_metadata(path).await,
            _ => Ok(ProjectMetadata::default()),
        }
    }

    async fn load_rust_metadata(path: &PathBuf) -> Result<ProjectMetadata> {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let contents = tokio::fs::read_to_string(&cargo_toml).await?;
            let parsed: toml::Value = toml::from_str(&contents)?;
            
            let package = parsed.get("package");
            let name = package
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("MyApp")
                .to_string();
            let version = package
                .and_then(|p| p.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string();

            // Try to load Android-specific metadata from Cargo.toml [package.metadata.android]
            let android_meta = package.and_then(|p| p.get("metadata")).and_then(|m| m.get("android"));
            
            let package_name = android_meta
                .and_then(|a| a.get("package"))
                .and_then(|p| p.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("com.example.{}", name.to_lowercase().replace('-', "_")));

            let min_sdk = android_meta
                .and_then(|a| a.get("min_sdk_version"))
                .and_then(|v| v.as_integer())
                .unwrap_or(24) as u32;

            let target_sdk = android_meta
                .and_then(|a| a.get("target_sdk_version"))
                .and_then(|v| v.as_integer())
                .unwrap_or(34) as u32;

            return Ok(ProjectMetadata {
                name,
                package_name,
                version_name: version,
                version_code: 1,
                min_sdk,
                target_sdk,
                compile_sdk: target_sdk,
            });
        }

        Ok(ProjectMetadata::default())
    }

    async fn load_gradle_metadata(_path: &PathBuf) -> Result<ProjectMetadata> {
        // This would parse build.gradle files to extract metadata
        // Simplified implementation for now
        Ok(ProjectMetadata::default())
    }

    /// Create Rust native project structure
    async fn create_rust_native_structure(&self) -> Result<()> {
        // Create Cargo.toml
        let cargo_toml = format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
ndk = "0.8"
ndk-glue = "0.7"
log = "0.4"
android_logger = "0.13"

[package.metadata.android]
package = "{}"
min_sdk_version = {}
target_sdk_version = {}
build_targets = ["aarch64-linux-android", "armv7-linux-androideabi"]

[package.metadata.android.application]
label = "{}"
"#,
            self.metadata.name.to_lowercase().replace(' ', "-"),
            self.metadata.version_name,
            self.metadata.package_name,
            self.metadata.min_sdk,
            self.metadata.target_sdk,
            self.metadata.name,
        );
        tokio::fs::write(self.root.join("Cargo.toml"), cargo_toml).await?;

        // Create src directory and lib.rs
        let src_dir = self.root.join("src");
        tokio::fs::create_dir_all(&src_dir).await?;

        let lib_rs = r#"use ndk_glue::Event;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
    );

    log::info!("R-Droid app started!");

    loop {
        match ndk_glue::poll_events() {
            Some(Event::WindowCreated) => {
                log::info!("Window created");
            }
            Some(Event::WindowDestroyed) => {
                log::info!("Window destroyed");
                break;
            }
            _ => {}
        }
    }
}
"#;
        tokio::fs::write(src_dir.join("lib.rs"), lib_rs).await?;

        // Create res directory
        let res_dir = self.root.join("res");
        tokio::fs::create_dir_all(&res_dir).await?;

        Ok(())
    }

    /// Create Kotlin/Java project structure
    async fn create_kotlin_java_structure(&self) -> Result<()> {
        // Create app module structure
        let app_dir = self.root.join("app");
        let src_main = app_dir.join("src").join("main");
        let java_dir = src_main.join("java").join(self.metadata.package_name.replace('.', std::path::MAIN_SEPARATOR_STR));
        let res_dir = src_main.join("res");

        tokio::fs::create_dir_all(&java_dir).await?;
        tokio::fs::create_dir_all(res_dir.join("layout")).await?;
        tokio::fs::create_dir_all(res_dir.join("values")).await?;
        tokio::fs::create_dir_all(res_dir.join("drawable")).await?;

        // Create settings.gradle.kts
        let settings_gradle = format!(
            r#"pluginManagement {{
    repositories {{
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}
dependencyResolutionManagement {{
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {{
        google()
        mavenCentral()
    }}
}}

rootProject.name = "{}"
include(":app")
"#,
            self.metadata.name
        );
        tokio::fs::write(self.root.join("settings.gradle.kts"), settings_gradle).await?;

        // Create root build.gradle.kts
        let root_build_gradle = r#"plugins {
    id("com.android.application") version "8.2.0" apply false
    id("org.jetbrains.kotlin.android") version "1.9.20" apply false
}
"#;
        tokio::fs::write(self.root.join("build.gradle.kts"), root_build_gradle).await?;

        // Create app/build.gradle.kts
        let app_build_gradle = format!(
            r#"plugins {{
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
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
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
"#,
            self.metadata.package_name,
            self.metadata.compile_sdk,
            self.metadata.package_name,
            self.metadata.min_sdk,
            self.metadata.target_sdk,
            self.metadata.version_code,
            self.metadata.version_name,
        );
        tokio::fs::write(app_dir.join("build.gradle.kts"), app_build_gradle).await?;

        // Create AndroidManifest.xml
        let manifest = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <application
        android:allowBackup="true"
        android:icon="@mipmap/ic_launcher"
        android:label="{}"
        android:roundIcon="@mipmap/ic_launcher_round"
        android:supportsRtl="true"
        android:theme="@style/Theme.Material3.DayNight">
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
"#,
            self.metadata.name
        );
        tokio::fs::write(src_main.join("AndroidManifest.xml"), manifest).await?;

        // Create MainActivity.kt
        let main_activity = format!(
            r#"package {}

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {{
    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
    }}
}}
"#,
            self.metadata.package_name
        );
        tokio::fs::write(java_dir.join("MainActivity.kt"), main_activity).await?;

        // Create activity_main.xml
        let activity_main_layout = r#"<?xml version="1.0" encoding="utf-8"?>
<androidx.constraintlayout.widget.ConstraintLayout 
    xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:app="http://schemas.android.com/apk/res-auto"
    android:layout_width="match_parent"
    android:layout_height="match_parent">

    <TextView
        android:layout_width="wrap_content"
        android:layout_height="wrap_content"
        android:text="Hello, R-Droid!"
        android:textSize="24sp"
        app:layout_constraintBottom_toBottomOf="parent"
        app:layout_constraintEnd_toEndOf="parent"
        app:layout_constraintStart_toStartOf="parent"
        app:layout_constraintTop_toTopOf="parent" />

</androidx.constraintlayout.widget.ConstraintLayout>
"#;
        tokio::fs::write(res_dir.join("layout").join("activity_main.xml"), activity_main_layout).await?;

        // Create strings.xml
        let strings_xml = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">{}</string>
</resources>
"#,
            self.metadata.name
        );
        tokio::fs::write(res_dir.join("values").join("strings.xml"), strings_xml).await?;

        // Create gradle.properties
        let gradle_properties = r#"org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8
android.useAndroidX=true
kotlin.code.style=official
android.nonTransitiveRClass=true
"#;
        tokio::fs::write(self.root.join("gradle.properties"), gradle_properties).await?;

        // Create gradle-wrapper.properties
        let wrapper_dir = self.root.join("gradle").join("wrapper");
        tokio::fs::create_dir_all(&wrapper_dir).await?;
        let wrapper_properties = r#"distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-8.4-bin.zip
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
"#;
        tokio::fs::write(wrapper_dir.join("gradle-wrapper.properties"), wrapper_properties).await?;

        Ok(())
    }

    /// Get the manifest file path
    pub fn manifest_path(&self) -> PathBuf {
        match self.project_type {
            ProjectType::RustNative => self.root.join("AndroidManifest.xml"),
            ProjectType::KotlinJava => self.root.join("app").join("src").join("main").join("AndroidManifest.xml"),
            _ => self.root.join("AndroidManifest.xml"),
        }
    }

    /// Get the build output directory
    pub fn build_output_dir(&self) -> PathBuf {
        match self.project_type {
            ProjectType::RustNative => self.root.join("target").join("android-artifacts"),
            ProjectType::KotlinJava => self.root.join("app").join("build").join("outputs").join("apk"),
            _ => self.root.join("build"),
        }
    }

    /// Mark project as dirty (modified)
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Save project changes
    pub async fn save(&mut self) -> Result<()> {
        // Save metadata and other project files
        self.is_dirty = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_metadata_default() {
        let metadata = ProjectMetadata::default();
        assert_eq!(metadata.name, "MyApp");
        assert_eq!(metadata.min_sdk, 24);
    }
}
