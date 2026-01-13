//! JDK Manager
//! 
//! Manages JDK installations for Android development.

use std::path::PathBuf;
use tokio::process::Command;
use tracing::{info, debug};

use crate::detector::JdkInfo;

/// JDK Manager errors
#[derive(Debug, thiserror::Error)]
pub enum JdkError {
    #[error("JDK not found")]
    NotFound,
    #[error("Invalid JDK: {0}")]
    Invalid(String),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// JDK Manager
pub struct JdkManager {
    jdk_path: PathBuf,
    info: JdkInfo,
}

impl JdkManager {
    /// Create a new JDK manager from a path
    pub async fn from_path(path: PathBuf) -> Result<Self, JdkError> {
        if !path.exists() {
            return Err(JdkError::NotFound);
        }

        let info = Self::analyze_jdk(&path).await?;
        
        if !info.is_jdk {
            return Err(JdkError::Invalid("Path contains JRE only, not full JDK".into()));
        }

        Ok(Self {
            jdk_path: path,
            info,
        })
    }

    /// Analyze a JDK installation
    async fn analyze_jdk(path: &PathBuf) -> Result<JdkInfo, JdkError> {
        let java_exe = if cfg!(windows) {
            path.join("bin").join("java.exe")
        } else {
            path.join("bin").join("java")
        };

        if !java_exe.exists() {
            return Err(JdkError::Invalid("java executable not found".into()));
        }

        let javac_exe = if cfg!(windows) {
            path.join("bin").join("javac.exe")
        } else {
            path.join("bin").join("javac")
        };

        let output = Command::new(&java_exe)
            .arg("-version")
            .output()
            .await?;

        let version_output = String::from_utf8_lossy(&output.stderr);
        
        let mut version = "unknown".to_string();
        let mut vendor = "unknown".to_string();

        for line in version_output.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("version") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        version = line[start + 1..start + 1 + end].to_string();
                    }
                }
            }
            if line_lower.contains("openjdk") {
                vendor = "OpenJDK".to_string();
            } else if line_lower.contains("oracle") {
                vendor = "Oracle".to_string();
            } else if line_lower.contains("temurin") || line_lower.contains("adoptium") {
                vendor = "Eclipse Temurin".to_string();
            }
        }

        Ok(JdkInfo {
            path: path.clone(),
            version,
            vendor,
            is_jdk: javac_exe.exists(),
        })
    }

    /// Get the JDK path
    pub fn path(&self) -> &PathBuf {
        &self.jdk_path
    }

    /// Get the JDK info
    pub fn info(&self) -> &JdkInfo {
        &self.info
    }

    /// Get the java executable path
    pub fn java_path(&self) -> PathBuf {
        if cfg!(windows) {
            self.jdk_path.join("bin").join("java.exe")
        } else {
            self.jdk_path.join("bin").join("java")
        }
    }

    /// Get the javac executable path
    pub fn javac_path(&self) -> PathBuf {
        if cfg!(windows) {
            self.jdk_path.join("bin").join("javac.exe")
        } else {
            self.jdk_path.join("bin").join("javac")
        }
    }

    /// Get the keytool executable path
    pub fn keytool_path(&self) -> PathBuf {
        if cfg!(windows) {
            self.jdk_path.join("bin").join("keytool.exe")
        } else {
            self.jdk_path.join("bin").join("keytool")
        }
    }

    /// Get the jarsigner executable path
    pub fn jarsigner_path(&self) -> PathBuf {
        if cfg!(windows) {
            self.jdk_path.join("bin").join("jarsigner.exe")
        } else {
            self.jdk_path.join("bin").join("jarsigner")
        }
    }

    /// Check if Java version is compatible with Android development
    pub fn is_android_compatible(&self) -> bool {
        // Android requires JDK 17 or later
        if let Some(major_version) = self.parse_major_version() {
            major_version >= 17
        } else {
            false
        }
    }

    /// Parse the major version number
    fn parse_major_version(&self) -> Option<u32> {
        let version = &self.info.version;
        
        // Handle formats like "17.0.2", "21.0.1", "1.8.0_xxx"
        if version.starts_with("1.") {
            // Old format (1.8.x)
            version.split('.').nth(1)?.parse().ok()
        } else {
            // New format (17.x, 21.x)
            version.split('.').next()?.parse().ok()
        }
    }

    /// Get the major version
    pub fn major_version(&self) -> Option<u32> {
        self.parse_major_version()
    }

    /// Run a java command
    pub async fn run_java(&self, args: &[&str]) -> Result<std::process::Output, JdkError> {
        let output = Command::new(self.java_path())
            .args(args)
            .output()
            .await?;

        Ok(output)
    }

    /// Run javac
    pub async fn run_javac(&self, args: &[&str]) -> Result<std::process::Output, JdkError> {
        let output = Command::new(self.javac_path())
            .args(args)
            .output()
            .await?;

        Ok(output)
    }

    /// Generate a debug keystore
    pub async fn generate_debug_keystore(&self, output_path: &PathBuf) -> Result<(), JdkError> {
        info!("Generating debug keystore at {:?}", output_path);

        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let output = Command::new(self.keytool_path())
            .args(&[
                "-genkey",
                "-v",
                "-keystore",
                &output_path.to_string_lossy(),
                "-storepass",
                "android",
                "-alias",
                "androiddebugkey",
                "-keypass",
                "android",
                "-keyalg",
                "RSA",
                "-keysize",
                "2048",
                "-validity",
                "10000",
                "-dname",
                "CN=Android Debug,O=Android,C=US",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(JdkError::CommandFailed(format!("keytool failed: {}", stderr)));
        }

        info!("Debug keystore generated successfully");
        Ok(())
    }

    /// Get environment variables for this JDK
    pub fn env_vars(&self) -> Vec<(String, String)> {
        vec![
            ("JAVA_HOME".to_string(), self.jdk_path.to_string_lossy().to_string()),
            ("PATH".to_string(), self.jdk_path.join("bin").to_string_lossy().to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        // This would need a mock JdkManager for testing
    }
}
