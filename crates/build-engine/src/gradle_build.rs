//! Gradle Build for Kotlin/Java Android Apps
//!
//! Wraps Gradle for traditional Android development.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{info, debug, warn};
use regex::Regex;

use crate::{BuildConfig, BuildError, BuildVariant, BuildType};
use crate::config::GradleConfig;
use crate::cargo_build::BuildMessage;

/// Gradle build for Android
pub struct GradleBuild {
    config: BuildConfig,
    gradle_config: GradleConfig,
    java_home: Option<PathBuf>,
    android_home: Option<PathBuf>,
}

impl GradleBuild {
    /// Create a new Gradle builder
    pub fn new(config: BuildConfig) -> Self {
        Self {
            config,
            gradle_config: GradleConfig::default(),
            java_home: None,
            android_home: None,
        }
    }

    /// Set Gradle-specific config
    pub fn with_gradle_config(mut self, config: GradleConfig) -> Self {
        self.gradle_config = config;
        self
    }

    /// Set JAVA_HOME
    pub fn with_java_home(mut self, path: PathBuf) -> Self {
        self.java_home = Some(path);
        self
    }

    /// Set ANDROID_HOME
    pub fn with_android_home(mut self, path: PathBuf) -> Self {
        self.android_home = Some(path);
        self
    }

    /// Get gradlew path
    fn gradlew_path(&self) -> PathBuf {
        let wrapper_name = if cfg!(windows) {
            "gradlew.bat"
        } else {
            "gradlew"
        };
        self.config.project_dir.join(wrapper_name)
    }

    /// Check if Gradle wrapper exists
    pub fn has_gradle_wrapper(&self) -> bool {
        self.gradlew_path().exists()
    }

    /// Build the project
    pub async fn build(&self) -> Result<PathBuf, BuildError> {
        if !self.has_gradle_wrapper() {
            return Err(BuildError::ToolchainNotFound("Gradle wrapper not found".into()));
        }

        info!("Building Android app with Gradle...");

        let task = self.gradle_config.task_name(self.config.build_type, self.config.variant);
        
        let mut args = vec![task.clone()];

        // Add properties
        for (key, value) in &self.gradle_config.properties {
            args.push(format!("-P{}={}", key, value));
        }

        // Add extra args
        for arg in &self.config.extra_args {
            args.push(arg.clone());
        }

        // Add parallel if jobs specified
        if let Some(jobs) = self.config.jobs {
            args.push(format!("--max-workers={}", jobs));
        }

        debug!("Running: gradlew {:?}", args);

        let mut cmd = Command::new(self.gradlew_path());
        cmd.current_dir(&self.config.project_dir);
        cmd.args(&args);

        // Set environment
        if let Some(ref java_home) = self.java_home {
            cmd.env("JAVA_HOME", java_home);
        }
        if let Some(ref android_home) = self.android_home {
            cmd.env("ANDROID_HOME", android_home);
            cmd.env("ANDROID_SDK_ROOT", android_home);
        }

        for (key, value) in &self.config.env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(BuildError::BuildFailed(format!("{}\n{}", stdout, stderr)));
        }

        info!("Gradle build completed successfully");

        // Find output APK/AAB
        self.find_output()
    }

    /// Build with progress reporting
    pub async fn build_with_progress(&self, tx: mpsc::Sender<BuildMessage>) -> Result<PathBuf, BuildError> {
        if !self.has_gradle_wrapper() {
            return Err(BuildError::ToolchainNotFound("Gradle wrapper not found".into()));
        }

        let task = self.gradle_config.task_name(self.config.build_type, self.config.variant);
        
        let _ = tx.send(BuildMessage::Started).await;

        let mut child = Command::new(self.gradlew_path())
            .current_dir(&self.config.project_dir)
            .arg(&task)
            .arg("--console=plain")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Stream stdout
        if let Some(stdout) = child.stdout.take() {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    let msg = parse_gradle_output(&line);
                    let _ = tx_clone.send(msg).await;
                }
            });
        }

        // Stream stderr
        if let Some(stderr) = child.stderr.take() {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.is_empty() {
                        let _ = tx_clone.send(BuildMessage::Warning(line)).await;
                    }
                }
            });
        }

        let status = child.wait().await?;

        if !status.success() {
            let _ = tx.send(BuildMessage::Failed("Gradle build failed".into())).await;
            return Err(BuildError::BuildFailed("Gradle build failed".into()));
        }

        let output_path = self.find_output()?;
        let _ = tx.send(BuildMessage::Completed(output_path.clone())).await;

        Ok(output_path)
    }

    /// Find the output APK/AAB
    fn find_output(&self) -> Result<PathBuf, BuildError> {
        let variant = self.config.variant.as_str();
        let extension = self.config.build_type.extension();
        let module = &self.gradle_config.module;

        // Standard output paths
        let search_paths = vec![
            // Module outputs
            self.config.project_dir.join(module).join("build").join("outputs").join(extension).join(variant),
            // Flavor outputs
            self.config.project_dir.join(module).join("build").join("outputs").join(extension),
            // Root project
            self.config.project_dir.join("build").join("outputs").join(extension).join(variant),
        ];

        for dir in &search_paths {
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        let ext_match = path.extension()
                            .map(|e| e.to_string_lossy().to_string())
                            .map(|e| e == extension)
                            .unwrap_or(false);
                        
                        if ext_match {
                            // Prefer unsigned for debug, signed for release
                            let name = path.file_name().unwrap().to_string_lossy();
                            if self.config.variant == BuildVariant::Debug && name.contains("debug") {
                                return Ok(path);
                            }
                            if self.config.variant == BuildVariant::Release && name.contains("release") {
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }

        // Fallback: search recursively
        for dir in &search_paths {
            if let Some(path) = self.find_output_recursive(dir, extension) {
                return Ok(path);
            }
        }

        Err(BuildError::BuildFailed(format!("Could not find output {}", extension)))
    }

    fn find_output_recursive(&self, dir: &PathBuf, extension: &str) -> Option<PathBuf> {
        if !dir.exists() {
            return None;
        }

        walkdir::WalkDir::new(dir)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
            .find(|e| {
                e.path().extension()
                    .map(|ext| ext == extension)
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
    }

    /// Clean build
    pub async fn clean(&self) -> Result<(), BuildError> {
        info!("Cleaning Gradle build...");

        let output = Command::new(self.gradlew_path())
            .current_dir(&self.config.project_dir)
            .arg("clean")
            .output()
            .await?;

        if !output.status.success() {
            warn!("Gradle clean failed, but continuing...");
        }

        Ok(())
    }

    /// Run connected check (instrumented tests)
    pub async fn connected_check(&self, device_serial: Option<&str>) -> Result<(), BuildError> {
        info!("Running connected tests...");

        let mut cmd = Command::new(self.gradlew_path());
        cmd.current_dir(&self.config.project_dir);
        cmd.arg("connectedCheck");

        if let Some(serial) = device_serial {
            cmd.env("ANDROID_SERIAL", serial);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::BuildFailed(format!("Connected tests failed: {}", stderr)));
        }

        Ok(())
    }

    /// Install on device
    pub async fn install(&self, device_serial: Option<&str>) -> Result<(), BuildError> {
        let task = format!(
            ":{}:install{}",
            self.gradle_config.module,
            self.config.variant.gradle_task_suffix()
        );

        let mut cmd = Command::new(self.gradlew_path());
        cmd.current_dir(&self.config.project_dir);
        cmd.arg(&task);

        if let Some(serial) = device_serial {
            cmd.env("ANDROID_SERIAL", serial);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::BuildFailed(format!("Install failed: {}", stderr)));
        }

        Ok(())
    }
}

/// Parse Gradle output into build messages
fn parse_gradle_output(line: &str) -> BuildMessage {
    let line = line.trim();
    
    // Task execution
    if line.starts_with("> Task") {
        return BuildMessage::Info(line.to_string());
    }
    
    // Compilation
    if line.contains("Compiling") || line.contains("compiling") {
        return BuildMessage::Compiling(line.to_string(), String::new());
    }
    
    // Linking/Packaging
    if line.contains("Linking") || line.contains("packaging") {
        return BuildMessage::Packaging;
    }
    
    // Signing
    if line.contains("Signing") || line.contains("signing") {
        return BuildMessage::Signing;
    }
    
    // Warnings
    if line.contains("warning:") || line.contains("WARNING") {
        return BuildMessage::Warning(line.to_string());
    }
    
    // Errors
    if line.contains("error:") || line.contains("FAILED") {
        return BuildMessage::Failed(line.to_string());
    }
    
    // Build success
    if line.contains("BUILD SUCCESSFUL") {
        return BuildMessage::Info(line.to_string());
    }
    
    BuildMessage::Info(line.to_string())
}

/// Generate build.gradle.kts for a new project
pub fn generate_build_gradle_kts(
    package: &str,
    min_sdk: u32,
    target_sdk: u32,
    compile_sdk: u32,
) -> String {
    format!(r#"plugins {{
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}}

android {{
    namespace = "{package}"
    compileSdk = {compile_sdk}

    defaultConfig {{
        applicationId = "{package}"
        minSdk = {min_sdk}
        targetSdk = {target_sdk}
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
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

    buildFeatures {{
        viewBinding = true
    }}
}}

dependencies {{
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.11.0")
    implementation("androidx.constraintlayout:constraintlayout:2.1.4")
    
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
}}
"#)
}

/// Generate settings.gradle.kts
pub fn generate_settings_gradle_kts(project_name: &str) -> String {
    format!(r#"pluginManagement {{
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

rootProject.name = "{project_name}"
include(":app")
"#)
}
