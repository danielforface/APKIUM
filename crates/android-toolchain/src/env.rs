//! Environment Manager
//! 
//! Manages environment variables for Android development within the IDE.

use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, debug, warn};

use crate::detector::{SdkInfo, NdkInfo, JdkInfo};

/// Environment configuration
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    /// ANDROID_HOME / ANDROID_SDK_ROOT
    pub android_home: Option<PathBuf>,
    /// ANDROID_NDK_HOME
    pub ndk_home: Option<PathBuf>,
    /// JAVA_HOME
    pub java_home: Option<PathBuf>,
    /// Additional PATH entries
    pub path_additions: Vec<PathBuf>,
    /// Custom environment variables
    pub custom_vars: HashMap<String, String>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            android_home: None,
            ndk_home: None,
            java_home: None,
            path_additions: Vec::new(),
            custom_vars: HashMap::new(),
        }
    }
}

impl EnvironmentConfig {
    /// Create from detected toolchain info
    pub fn from_detected(
        sdk: Option<&SdkInfo>,
        ndk: Option<&NdkInfo>,
        jdk: Option<&JdkInfo>,
    ) -> Self {
        let mut config = Self::default();

        if let Some(sdk_info) = sdk {
            config.android_home = Some(sdk_info.path.clone());
            
            // Add platform-tools and tools to PATH
            let platform_tools = sdk_info.path.join("platform-tools");
            let cmdline_tools = sdk_info.path.join("cmdline-tools").join("latest").join("bin");
            let tools = sdk_info.path.join("tools").join("bin");
            
            if platform_tools.exists() {
                config.path_additions.push(platform_tools);
            }
            if cmdline_tools.exists() {
                config.path_additions.push(cmdline_tools);
            }
            if tools.exists() {
                config.path_additions.push(tools);
            }
        }

        if let Some(ndk_info) = ndk {
            config.ndk_home = Some(ndk_info.path.clone());
        }

        if let Some(jdk_info) = jdk {
            config.java_home = Some(jdk_info.path.clone());
            
            // Add JDK bin to PATH
            let jdk_bin = jdk_info.path.join("bin");
            if jdk_bin.exists() {
                config.path_additions.push(jdk_bin);
            }
        }

        config
    }
}

/// Environment Manager
pub struct EnvManager {
    config: EnvironmentConfig,
    original_env: HashMap<String, String>,
}

impl EnvManager {
    /// Create a new environment manager
    pub fn new(config: EnvironmentConfig) -> Self {
        // Capture original environment
        let original_env: HashMap<String, String> = std::env::vars().collect();
        
        Self {
            config,
            original_env,
        }
    }

    /// Get environment variables to set
    pub fn get_env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        // Android SDK
        if let Some(ref path) = self.config.android_home {
            let path_str = path.to_string_lossy().to_string();
            vars.insert("ANDROID_HOME".to_string(), path_str.clone());
            vars.insert("ANDROID_SDK_ROOT".to_string(), path_str);
        }

        // Android NDK
        if let Some(ref path) = self.config.ndk_home {
            let path_str = path.to_string_lossy().to_string();
            vars.insert("ANDROID_NDK_HOME".to_string(), path_str.clone());
            vars.insert("NDK_HOME".to_string(), path_str);
        }

        // Java
        if let Some(ref path) = self.config.java_home {
            vars.insert("JAVA_HOME".to_string(), path.to_string_lossy().to_string());
        }

        // Custom variables
        for (key, value) in &self.config.custom_vars {
            vars.insert(key.clone(), value.clone());
        }

        vars
    }

    /// Get PATH value including additions
    pub fn get_path(&self) -> String {
        let path_sep = if cfg!(windows) { ";" } else { ":" };
        
        let original_path = self.original_env
            .get("PATH")
            .or_else(|| self.original_env.get("Path"))
            .cloned()
            .unwrap_or_default();

        if self.config.path_additions.is_empty() {
            return original_path;
        }

        let additions: Vec<String> = self.config.path_additions
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        format!("{}{}{}", additions.join(path_sep), path_sep, original_path)
    }

    /// Apply environment to the current process (for child processes)
    pub fn apply_to_process(&self) {
        for (key, value) in self.get_env_vars() {
            std::env::set_var(&key, &value);
        }

        let path_key = if cfg!(windows) { "Path" } else { "PATH" };
        std::env::set_var(path_key, self.get_path());
        
        info!("Applied Android development environment");
    }

    /// Get command with environment
    pub fn command_env(&self) -> HashMap<String, String> {
        let mut env = self.get_env_vars();
        let path_key = if cfg!(windows) { "Path" } else { "PATH" };
        env.insert(path_key.to_string(), self.get_path());
        env
    }

    /// Check if environment is properly configured
    pub fn validate(&self) -> EnvironmentValidation {
        let mut validation = EnvironmentValidation::default();

        // Check Android SDK
        if let Some(ref path) = self.config.android_home {
            if path.exists() {
                validation.sdk_valid = true;
                
                // Check for required components
                let platform_tools = path.join("platform-tools");
                let adb = platform_tools.join(if cfg!(windows) { "adb.exe" } else { "adb" });
                validation.adb_available = adb.exists();
            }
        }

        // Check NDK
        if let Some(ref path) = self.config.ndk_home {
            if path.exists() {
                validation.ndk_valid = true;
            }
        }

        // Check JDK
        if let Some(ref path) = self.config.java_home {
            if path.exists() {
                let java = path.join("bin").join(if cfg!(windows) { "java.exe" } else { "java" });
                validation.jdk_valid = java.exists();
            }
        }

        validation
    }

    /// Get shell export commands (for terminal display)
    pub fn shell_exports(&self) -> String {
        let mut exports = String::new();
        let export_cmd = if cfg!(windows) { "set" } else { "export" };

        for (key, value) in self.get_env_vars() {
            if cfg!(windows) {
                exports.push_str(&format!("{}={}={}\n", export_cmd, key, value));
            } else {
                exports.push_str(&format!("{} {}=\"{}\"\n", export_cmd, key, value));
            }
        }

        exports
    }

    /// Add custom environment variable
    pub fn set_var(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.config.custom_vars.insert(key.into(), value.into());
    }

    /// Add path to PATH
    pub fn add_to_path(&mut self, path: PathBuf) {
        if !self.config.path_additions.contains(&path) {
            self.config.path_additions.push(path);
        }
    }
}

/// Environment validation result
#[derive(Debug, Default)]
pub struct EnvironmentValidation {
    pub sdk_valid: bool,
    pub ndk_valid: bool,
    pub jdk_valid: bool,
    pub adb_available: bool,
}

impl EnvironmentValidation {
    /// Check if environment is ready for Android development
    pub fn is_ready(&self) -> bool {
        self.sdk_valid && self.jdk_valid && self.adb_available
    }

    /// Check if environment is ready for native development
    pub fn is_native_ready(&self) -> bool {
        self.is_ready() && self.ndk_valid
    }

    /// Get list of missing components
    pub fn missing_components(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        
        if !self.sdk_valid {
            missing.push("Android SDK");
        }
        if !self.adb_available {
            missing.push("ADB (platform-tools)");
        }
        if !self.jdk_valid {
            missing.push("JDK");
        }
        if !self.ndk_valid {
            missing.push("Android NDK");
        }

        missing
    }
}

/// Environment file writer
pub struct EnvFileWriter;

impl EnvFileWriter {
    /// Write a .env file
    pub async fn write_dotenv(path: &PathBuf, env: &EnvManager) -> std::io::Result<()> {
        let mut content = String::new();
        content.push_str("# R-Droid Android Development Environment\n\n");

        for (key, value) in env.get_env_vars() {
            content.push_str(&format!("{}={}\n", key, value));
        }

        tokio::fs::write(path, content).await?;
        info!("Wrote environment to {:?}", path);
        Ok(())
    }

    /// Write a shell script for environment setup
    pub async fn write_shell_script(path: &PathBuf, env: &EnvManager) -> std::io::Result<()> {
        let mut content = if cfg!(windows) {
            "@echo off\nREM R-Droid Android Development Environment\n\n".to_string()
        } else {
            "#!/bin/bash\n# R-Droid Android Development Environment\n\n".to_string()
        };

        content.push_str(&env.shell_exports());

        tokio::fs::write(path, content).await?;
        
        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(path, perms).await?;
        }

        info!("Wrote shell script to {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_config() {
        let config = EnvironmentConfig {
            android_home: Some(PathBuf::from("/android/sdk")),
            java_home: Some(PathBuf::from("/java/jdk")),
            ..Default::default()
        };

        let env = EnvManager::new(config);
        let vars = env.get_env_vars();

        assert!(vars.contains_key("ANDROID_HOME"));
        assert!(vars.contains_key("JAVA_HOME"));
    }

    #[test]
    fn test_validation() {
        let validation = EnvironmentValidation {
            sdk_valid: true,
            ndk_valid: true,
            jdk_valid: true,
            adb_available: true,
        };

        assert!(validation.is_ready());
        assert!(validation.is_native_ready());
        assert!(validation.missing_components().is_empty());
    }
}
