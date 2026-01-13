//! APK Analysis and Manipulation
//!
//! Parse and analyze APK files.

use std::path::PathBuf;
use std::io::{Read, Seek};
use zip::ZipArchive;
use tracing::{debug, warn};

use crate::BuildError;

/// APK information
#[derive(Debug, Clone)]
pub struct ApkInfo {
    /// APK file path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Package name
    pub package: Option<String>,
    /// Version code
    pub version_code: Option<u32>,
    /// Version name
    pub version_name: Option<String>,
    /// Minimum SDK version
    pub min_sdk: Option<u32>,
    /// Target SDK version
    pub target_sdk: Option<u32>,
    /// List of permissions
    pub permissions: Vec<String>,
    /// Native libraries by ABI
    pub native_libs: Vec<NativeLib>,
    /// Is debuggable
    pub debuggable: bool,
    /// Is signed
    pub signed: bool,
    /// Signature scheme version
    pub signature_version: Option<u32>,
    /// Activities
    pub activities: Vec<String>,
    /// Main activity
    pub main_activity: Option<String>,
}

/// Native library in APK
#[derive(Debug, Clone)]
pub struct NativeLib {
    pub abi: String,
    pub name: String,
    pub size: u64,
}

/// APK Analyzer
pub struct ApkAnalyzer;

impl ApkAnalyzer {
    /// Analyze an APK file
    pub fn analyze(path: &PathBuf) -> Result<ApkInfo, BuildError> {
        let file = std::fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut archive = ZipArchive::new(file)
            .map_err(|e| BuildError::BuildFailed(format!("Invalid APK: {}", e)))?;

        let mut info = ApkInfo {
            path: path.clone(),
            size,
            package: None,
            version_code: None,
            version_name: None,
            min_sdk: None,
            target_sdk: None,
            permissions: Vec::new(),
            native_libs: Vec::new(),
            debuggable: false,
            signed: false,
            signature_version: None,
            activities: Vec::new(),
            main_activity: None,
        };

        // Check for native libraries
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                
                // Check for native libs
                if name.starts_with("lib/") && name.ends_with(".so") {
                    let parts: Vec<&str> = name.split('/').collect();
                    if parts.len() >= 3 {
                        info.native_libs.push(NativeLib {
                            abi: parts[1].to_string(),
                            name: parts[2].to_string(),
                            size: file.size(),
                        });
                    }
                }

                // Check for signatures
                if name.starts_with("META-INF/") {
                    if name.ends_with(".RSA") || name.ends_with(".DSA") || name.ends_with(".EC") {
                        info.signed = true;
                        info.signature_version = Some(1);
                    }
                    if name == "META-INF/CERT.SF" {
                        info.signed = true;
                    }
                }
            }
        }

        // Check for v2/v3/v4 signatures (in APK signing block)
        if Self::has_v2_signature(path) {
            info.signed = true;
            info.signature_version = Some(2);
        }

        Ok(info)
    }

    /// Check if APK has v2+ signature
    fn has_v2_signature(path: &PathBuf) -> bool {
        // V2 signature is stored in a special block before the central directory
        // This is a simplified check
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };

        let mut reader = std::io::BufReader::new(file);
        
        // APK Signature Scheme v2 magic: "APK Sig Block 42"
        let magic = b"APK Sig Block 42";
        let mut buffer = vec![0u8; 4096];
        
        // Read from end of file
        if reader.seek(std::io::SeekFrom::End(-4096)).is_err() {
            return false;
        }
        
        if reader.read(&mut buffer).is_err() {
            return false;
        }

        // Search for magic bytes
        buffer.windows(magic.len()).any(|w| w == magic)
    }

    /// Get APK size in human-readable format
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }

    /// List files in APK
    pub fn list_files(path: &PathBuf) -> Result<Vec<ApkEntry>, BuildError> {
        let file = std::fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| BuildError::BuildFailed(format!("Invalid APK: {}", e)))?;

        let mut entries = Vec::new();
        
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                entries.push(ApkEntry {
                    name: file.name().to_string(),
                    size: file.size(),
                    compressed_size: file.compressed_size(),
                    is_directory: file.is_dir(),
                });
            }
        }

        Ok(entries)
    }

    /// Extract a file from APK
    pub fn extract_file(apk_path: &PathBuf, file_name: &str, output: &PathBuf) -> Result<(), BuildError> {
        let file = std::fs::File::open(apk_path)?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| BuildError::BuildFailed(format!("Invalid APK: {}", e)))?;

        let mut zip_file = archive.by_name(file_name)
            .map_err(|_| BuildError::BuildFailed(format!("File not found in APK: {}", file_name)))?;

        let mut content = Vec::new();
        zip_file.read_to_end(&mut content)?;
        
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(output, content)?;

        Ok(())
    }

    /// Get ABIs supported by APK
    pub fn supported_abis(info: &ApkInfo) -> Vec<String> {
        let mut abis: Vec<String> = info.native_libs
            .iter()
            .map(|l| l.abi.clone())
            .collect();
        
        abis.sort();
        abis.dedup();
        abis
    }
}

/// APK entry information
#[derive(Debug, Clone)]
pub struct ApkEntry {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_directory: bool,
}

impl ApkEntry {
    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.size == 0 {
            0.0
        } else {
            1.0 - (self.compressed_size as f64 / self.size as f64)
        }
    }
}

/// APK size breakdown
#[derive(Debug, Clone, Default)]
pub struct ApkSizeBreakdown {
    pub dex: u64,
    pub resources: u64,
    pub native_libs: u64,
    pub assets: u64,
    pub other: u64,
    pub total: u64,
}

impl ApkSizeBreakdown {
    /// Calculate size breakdown for APK
    pub fn calculate(path: &PathBuf) -> Result<Self, BuildError> {
        let entries = ApkAnalyzer::list_files(path)?;
        
        let mut breakdown = Self::default();
        
        for entry in &entries {
            let size = entry.compressed_size;
            
            if entry.name.ends_with(".dex") {
                breakdown.dex += size;
            } else if entry.name.starts_with("res/") || entry.name == "resources.arsc" {
                breakdown.resources += size;
            } else if entry.name.starts_with("lib/") {
                breakdown.native_libs += size;
            } else if entry.name.starts_with("assets/") {
                breakdown.assets += size;
            } else {
                breakdown.other += size;
            }
            
            breakdown.total += size;
        }
        
        Ok(breakdown)
    }

    /// Get as percentages
    pub fn percentages(&self) -> Vec<(&'static str, f64)> {
        let total = self.total as f64;
        if total == 0.0 {
            return Vec::new();
        }
        
        vec![
            ("DEX", self.dex as f64 / total * 100.0),
            ("Resources", self.resources as f64 / total * 100.0),
            ("Native Libs", self.native_libs as f64 / total * 100.0),
            ("Assets", self.assets as f64 / total * 100.0),
            ("Other", self.other as f64 / total * 100.0),
        ]
    }
}
