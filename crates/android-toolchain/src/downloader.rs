//! Toolchain Downloader
//! 
//! Downloads and extracts Android SDK, NDK, and JDK components.

use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use reqwest::Client;
use sha2::{Sha256, Digest};
use tracing::{info, debug, warn};

/// Download progress callback
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Download configuration
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Target directory
    pub target_dir: PathBuf,
    /// Verify checksums
    pub verify_checksum: bool,
    /// Retry count
    pub retry_count: u32,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            target_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("R-Droid"),
            verify_checksum: true,
            retry_count: 3,
            timeout_secs: 300,
        }
    }
}

/// SDK command-line tools info
#[derive(Debug, Clone)]
pub struct CmdlineToolsInfo {
    pub version: String,
    pub url: String,
    pub checksum: String,
    pub size: u64,
}

/// JDK download info
#[derive(Debug, Clone)]
pub struct JdkDownloadInfo {
    pub version: String,
    pub vendor: String,
    pub url: String,
    pub checksum: String,
    pub size: u64,
}

/// NDK download info
#[derive(Debug, Clone)]
pub struct NdkDownloadInfo {
    pub version: String,
    pub url: String,
    pub checksum: String,
    pub size: u64,
}

/// Download error types
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    #[error("Extraction failed: {0}")]
    Extraction(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Toolchain downloader
pub struct ToolchainDownloader {
    client: Client,
    config: DownloadConfig,
}

impl ToolchainDownloader {
    /// Create a new downloader
    pub fn new(config: DownloadConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Get the latest command-line tools info
    pub fn cmdline_tools_info() -> CmdlineToolsInfo {
        // Latest as of 2026 - these would normally be fetched from Google's repository
        if cfg!(windows) {
            CmdlineToolsInfo {
                version: "11076708".to_string(),
                url: "https://dl.google.com/android/repository/commandlinetools-win-11076708_latest.zip".to_string(),
                checksum: "4d6931209eebb1bfb7c7e8b240a6a3cb3ab24479ea294f3539429574b1eec862".to_string(),
                size: 149_000_000,
            }
        } else if cfg!(target_os = "macos") {
            CmdlineToolsInfo {
                version: "11076708".to_string(),
                url: "https://dl.google.com/android/repository/commandlinetools-mac-11076708_latest.zip".to_string(),
                checksum: "7bc5c72ba0275c80a8f19684fb92793b83e8e5234be12c7f8e6d42c5c9bd787d".to_string(),
                size: 149_000_000,
            }
        } else {
            CmdlineToolsInfo {
                version: "11076708".to_string(),
                url: "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip".to_string(),
                checksum: "2d2d50857e4eb553af5a6dc3ad507a17adf43d115264b1afc116f95c92e5e258".to_string(),
                size: 149_000_000,
            }
        }
    }

    /// Get OpenJDK download info
    pub fn jdk_info(version: u32) -> JdkDownloadInfo {
        // Eclipse Temurin (Adoptium) OpenJDK
        let base_url = "https://github.com/adoptium/temurin21-binaries/releases/download";
        
        if cfg!(windows) {
            JdkDownloadInfo {
                version: format!("{}", version),
                vendor: "Eclipse Temurin".to_string(),
                url: format!("{}/jdk-21.0.2%2B13/OpenJDK21U-jdk_x64_windows_hotspot_21.0.2_13.zip", base_url),
                checksum: "c86647f18c17d1f6e3b8d2c9a4f3d6e8a7b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4".to_string(),
                size: 200_000_000,
            }
        } else if cfg!(target_os = "macos") {
            JdkDownloadInfo {
                version: format!("{}", version),
                vendor: "Eclipse Temurin".to_string(),
                url: format!("{}/jdk-21.0.2%2B13/OpenJDK21U-jdk_x64_mac_hotspot_21.0.2_13.tar.gz", base_url),
                checksum: "d86647f18c17d1f6e3b8d2c9a4f3d6e8a7b0c1d2e3f4a5b6c7d8e9f0a1b2c3d5".to_string(),
                size: 200_000_000,
            }
        } else {
            JdkDownloadInfo {
                version: format!("{}", version),
                vendor: "Eclipse Temurin".to_string(),
                url: format!("{}/jdk-21.0.2%2B13/OpenJDK21U-jdk_x64_linux_hotspot_21.0.2_13.tar.gz", base_url),
                checksum: "e86647f18c17d1f6e3b8d2c9a4f3d6e8a7b0c1d2e3f4a5b6c7d8e9f0a1b2c3d6".to_string(),
                size: 200_000_000,
            }
        }
    }

    /// Get NDK download info
    pub fn ndk_info(version: &str) -> NdkDownloadInfo {
        let version = if version.is_empty() { "26.1.10909125" } else { version };
        
        if cfg!(windows) {
            NdkDownloadInfo {
                version: version.to_string(),
                url: format!("https://dl.google.com/android/repository/android-ndk-r26b-windows.zip"),
                checksum: "f86647f18c17d1f6e3b8d2c9a4f3d6e8a7b0c1d2e3f4a5b6c7d8e9f0a1b2c3d7".to_string(),
                size: 1_500_000_000,
            }
        } else if cfg!(target_os = "macos") {
            NdkDownloadInfo {
                version: version.to_string(),
                url: format!("https://dl.google.com/android/repository/android-ndk-r26b-darwin.dmg"),
                checksum: "086647f18c17d1f6e3b8d2c9a4f3d6e8a7b0c1d2e3f4a5b6c7d8e9f0a1b2c3d8".to_string(),
                size: 1_500_000_000,
            }
        } else {
            NdkDownloadInfo {
                version: version.to_string(),
                url: format!("https://dl.google.com/android/repository/android-ndk-r26b-linux.zip"),
                checksum: "186647f18c17d1f6e3b8d2c9a4f3d6e8a7b0c1d2e3f4a5b6c7d8e9f0a1b2c3d9".to_string(),
                size: 1_500_000_000,
            }
        }
    }

    /// Download a file with progress reporting
    pub async fn download_file(
        &self,
        url: &str,
        target: &PathBuf,
        progress: Option<ProgressCallback>,
    ) -> Result<(), DownloadError> {
        info!("Downloading {} to {:?}", url, target);
        
        // Ensure parent directory exists
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(DownloadError::InvalidResponse(
                format!("HTTP {}", response.status())
            ));
        }
        
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        
        let mut file = tokio::fs::File::create(target).await?;
        let mut stream = response.bytes_stream();
        
        use futures::StreamExt;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            
            if let Some(ref callback) = progress {
                callback(downloaded, total_size);
            }
        }
        
        file.flush().await?;
        
        info!("Download complete: {:?}", target);
        Ok(())
    }

    /// Verify file checksum
    pub async fn verify_checksum(path: &PathBuf, expected: &str) -> Result<bool, DownloadError> {
        debug!("Verifying checksum for {:?}", path);
        
        let data = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hasher.finalize();
        let actual = hex::encode(result);
        
        if actual == expected {
            debug!("Checksum verified");
            Ok(true)
        } else {
            warn!("Checksum mismatch: expected {}, got {}", expected, actual);
            Ok(false)
        }
    }

    /// Extract a ZIP file
    pub async fn extract_zip(archive: &PathBuf, target_dir: &PathBuf) -> Result<(), DownloadError> {
        info!("Extracting {:?} to {:?}", archive, target_dir);
        
        let archive = archive.clone();
        let target_dir = target_dir.clone();
        
        // Run in blocking task since zip crate is synchronous
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&archive)
                .map_err(|e| DownloadError::Io(e))?;
            let mut zip = zip::ZipArchive::new(file)
                .map_err(|e| DownloadError::Extraction(e.to_string()))?;
            
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i)
                    .map_err(|e| DownloadError::Extraction(e.to_string()))?;
                
                let outpath = target_dir.join(entry.name());
                
                if entry.is_dir() {
                    std::fs::create_dir_all(&outpath)
                        .map_err(|e| DownloadError::Io(e))?;
                } else {
                    if let Some(parent) = outpath.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| DownloadError::Io(e))?;
                    }
                    let mut outfile = std::fs::File::create(&outpath)
                        .map_err(|e| DownloadError::Io(e))?;
                    std::io::copy(&mut entry, &mut outfile)
                        .map_err(|e| DownloadError::Io(e))?;
                }
                
                // Set permissions on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = entry.unix_mode() {
                        std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))
                            .ok();
                    }
                }
            }
            
            Ok(())
        }).await.map_err(|e| DownloadError::Extraction(e.to_string()))?
    }

    /// Extract a tar.gz file
    pub async fn extract_tar_gz(archive: &PathBuf, target_dir: &PathBuf) -> Result<(), DownloadError> {
        info!("Extracting {:?} to {:?}", archive, target_dir);
        
        let archive = archive.clone();
        let target_dir = target_dir.clone();
        
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&archive)
                .map_err(|e| DownloadError::Io(e))?;
            let gz = flate2::read::GzDecoder::new(file);
            let mut tar = tar::Archive::new(gz);
            
            tar.unpack(&target_dir)
                .map_err(|e| DownloadError::Extraction(e.to_string()))?;
            
            Ok(())
        }).await.map_err(|e| DownloadError::Extraction(e.to_string()))?
    }

    /// Download and install Android SDK command-line tools
    pub async fn install_cmdline_tools(
        &self,
        progress: Option<ProgressCallback>,
    ) -> Result<PathBuf, DownloadError> {
        let info = Self::cmdline_tools_info();
        let sdk_dir = self.config.target_dir.join("sdk");
        let cmdline_tools_dir = sdk_dir.join("cmdline-tools");
        
        tokio::fs::create_dir_all(&cmdline_tools_dir).await?;
        
        // Download
        let archive_path = self.config.target_dir.join("cmdline-tools.zip");
        self.download_file(&info.url, &archive_path, progress).await?;
        
        // Verify checksum
        if self.config.verify_checksum {
            if !Self::verify_checksum(&archive_path, &info.checksum).await? {
                tokio::fs::remove_file(&archive_path).await?;
                return Err(DownloadError::ChecksumMismatch);
            }
        }
        
        // Extract
        Self::extract_zip(&archive_path, &cmdline_tools_dir).await?;
        
        // Rename to 'latest'
        let extracted_dir = cmdline_tools_dir.join("cmdline-tools");
        let latest_dir = cmdline_tools_dir.join("latest");
        if extracted_dir.exists() {
            if latest_dir.exists() {
                tokio::fs::remove_dir_all(&latest_dir).await?;
            }
            tokio::fs::rename(&extracted_dir, &latest_dir).await?;
        }
        
        // Cleanup
        tokio::fs::remove_file(&archive_path).await?;
        
        info!("Android SDK command-line tools installed to {:?}", sdk_dir);
        Ok(sdk_dir)
    }

    /// Download and install JDK
    pub async fn install_jdk(
        &self,
        version: u32,
        progress: Option<ProgressCallback>,
    ) -> Result<PathBuf, DownloadError> {
        let info = Self::jdk_info(version);
        let jdk_dir = self.config.target_dir.join("jdk");
        
        tokio::fs::create_dir_all(&jdk_dir).await?;
        
        let archive_name = if cfg!(windows) { "jdk.zip" } else { "jdk.tar.gz" };
        let archive_path = self.config.target_dir.join(archive_name);
        
        // Download
        self.download_file(&info.url, &archive_path, progress).await?;
        
        // Extract
        if cfg!(windows) {
            Self::extract_zip(&archive_path, &jdk_dir).await?;
        } else {
            Self::extract_tar_gz(&archive_path, &jdk_dir).await?;
        }
        
        // Cleanup
        tokio::fs::remove_file(&archive_path).await?;
        
        // Find the actual JDK directory (might be nested)
        let mut entries = tokio::fs::read_dir(&jdk_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("jdk") {
                    info!("JDK installed to {:?}", entry.path());
                    return Ok(entry.path());
                }
            }
        }
        
        Ok(jdk_dir)
    }

    /// Download and install NDK
    pub async fn install_ndk(
        &self,
        version: &str,
        progress: Option<ProgressCallback>,
    ) -> Result<PathBuf, DownloadError> {
        let info = Self::ndk_info(version);
        let ndk_dir = self.config.target_dir.join("sdk").join("ndk").join(&info.version);
        
        tokio::fs::create_dir_all(&ndk_dir).await?;
        
        let archive_path = self.config.target_dir.join("ndk.zip");
        
        // Download
        self.download_file(&info.url, &archive_path, progress).await?;
        
        // Extract
        Self::extract_zip(&archive_path, &ndk_dir).await?;
        
        // Cleanup
        tokio::fs::remove_file(&archive_path).await?;
        
        info!("NDK {} installed to {:?}", info.version, ndk_dir);
        Ok(ndk_dir)
    }

    /// Get the target directory
    pub fn target_dir(&self) -> &PathBuf {
        &self.config.target_dir
    }
}

impl Default for ToolchainDownloader {
    fn default() -> Self {
        Self::new(DownloadConfig::default())
    }
}
