//! APK Signing
//!
//! Sign APKs using keystore and apksigner.

use std::path::PathBuf;
use tokio::process::Command;
use tracing::{info, debug};

use crate::BuildError;

/// Keystore information
#[derive(Debug, Clone)]
pub struct KeyStore {
    /// Path to keystore file
    pub path: PathBuf,
    /// Keystore password
    pub password: String,
    /// Key alias
    pub alias: String,
    /// Key password (if different from keystore password)
    pub key_password: Option<String>,
    /// Keystore type (JKS, PKCS12)
    pub store_type: KeyStoreType,
}

/// Keystore type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStoreType {
    Jks,
    Pkcs12,
}

impl KeyStoreType {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyStoreType::Jks => "JKS",
            KeyStoreType::Pkcs12 => "PKCS12",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            KeyStoreType::Jks => "jks",
            KeyStoreType::Pkcs12 => "p12",
        }
    }
}

impl KeyStore {
    /// Create a new keystore reference
    pub fn new(path: PathBuf, password: &str, alias: &str) -> Self {
        Self {
            path,
            password: password.to_string(),
            alias: alias.to_string(),
            key_password: None,
            store_type: KeyStoreType::Jks,
        }
    }

    /// Get the effective key password
    pub fn effective_key_password(&self) -> &str {
        self.key_password.as_ref().unwrap_or(&self.password)
    }

    /// Check if keystore exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Signing configuration
#[derive(Debug, Clone)]
pub struct SigningConfig {
    /// Keystore to use
    pub keystore: KeyStore,
    /// Enable v1 signature (JAR signing)
    pub v1_signing_enabled: bool,
    /// Enable v2 signature
    pub v2_signing_enabled: bool,
    /// Enable v3 signature
    pub v3_signing_enabled: bool,
    /// Enable v4 signature
    pub v4_signing_enabled: bool,
    /// Minimum SDK for signing config
    pub min_sdk_version: Option<u32>,
}

impl SigningConfig {
    /// Create default signing config
    pub fn new(keystore: KeyStore) -> Self {
        Self {
            keystore,
            v1_signing_enabled: true,
            v2_signing_enabled: true,
            v3_signing_enabled: true,
            v4_signing_enabled: false,
            min_sdk_version: None,
        }
    }

    /// Create for debug signing
    pub fn debug(debug_keystore: PathBuf) -> Self {
        Self::new(KeyStore::new(debug_keystore, "android", "androiddebugkey"))
    }

    /// Enable all signature versions
    pub fn with_all_signatures(mut self) -> Self {
        self.v1_signing_enabled = true;
        self.v2_signing_enabled = true;
        self.v3_signing_enabled = true;
        self.v4_signing_enabled = true;
        self
    }
}

/// APK Signer
pub struct ApkSigner {
    sdk_path: PathBuf,
}

impl ApkSigner {
    /// Create a new APK signer
    pub fn new(sdk_path: PathBuf) -> Self {
        Self { sdk_path }
    }

    /// Get apksigner path
    fn apksigner_path(&self) -> Option<PathBuf> {
        let build_tools = self.sdk_path.join("build-tools");
        
        if !build_tools.exists() {
            return None;
        }

        // Find the latest version
        let mut versions: Vec<_> = std::fs::read_dir(&build_tools)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        
        versions.sort();
        
        let latest = versions.last()?;
        let apksigner_name = if cfg!(windows) {
            "apksigner.bat"
        } else {
            "apksigner"
        };

        Some(build_tools.join(latest).join(apksigner_name))
    }

    /// Get zipalign path
    fn zipalign_path(&self) -> Option<PathBuf> {
        let build_tools = self.sdk_path.join("build-tools");
        
        if !build_tools.exists() {
            return None;
        }

        let mut versions: Vec<_> = std::fs::read_dir(&build_tools)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        
        versions.sort();
        
        let latest = versions.last()?;
        let zipalign_name = if cfg!(windows) {
            "zipalign.exe"
        } else {
            "zipalign"
        };

        Some(build_tools.join(latest).join(zipalign_name))
    }

    /// Check if apksigner is available
    pub fn is_available(&self) -> bool {
        self.apksigner_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Sign an APK
    pub async fn sign(&self, apk: &PathBuf, config: &SigningConfig, output: &PathBuf) -> Result<(), BuildError> {
        let apksigner = self.apksigner_path()
            .ok_or_else(|| BuildError::ToolchainNotFound("apksigner not found".into()))?;

        info!("Signing APK: {:?}", apk);

        // First, zipalign if it's not already aligned
        let aligned_apk = if self.needs_alignment(apk).await {
            let temp = apk.with_extension("aligned.apk");
            self.zipalign(apk, &temp).await?;
            temp
        } else {
            apk.clone()
        };

        let mut args = vec![
            "sign".to_string(),
            "--ks".to_string(),
            config.keystore.path.to_string_lossy().to_string(),
            "--ks-pass".to_string(),
            format!("pass:{}", config.keystore.password),
            "--ks-key-alias".to_string(),
            config.keystore.alias.clone(),
            "--key-pass".to_string(),
            format!("pass:{}", config.keystore.effective_key_password()),
        ];

        // Signature versions
        if !config.v1_signing_enabled {
            args.push("--v1-signing-enabled".to_string());
            args.push("false".to_string());
        }
        if !config.v2_signing_enabled {
            args.push("--v2-signing-enabled".to_string());
            args.push("false".to_string());
        }
        if !config.v3_signing_enabled {
            args.push("--v3-signing-enabled".to_string());
            args.push("false".to_string());
        }
        if config.v4_signing_enabled {
            args.push("--v4-signing-enabled".to_string());
            args.push("true".to_string());
        }

        if let Some(min_sdk) = config.min_sdk_version {
            args.push("--min-sdk-version".to_string());
            args.push(min_sdk.to_string());
        }

        // Output
        args.push("--out".to_string());
        args.push(output.to_string_lossy().to_string());
        args.push(aligned_apk.to_string_lossy().to_string());

        debug!("apksigner {:?}", args);

        let cmd_output = Command::new(&apksigner)
            .args(&args)
            .output()
            .await?;

        // Clean up temp file
        if aligned_apk != *apk {
            let _ = std::fs::remove_file(&aligned_apk);
        }

        if !cmd_output.status.success() {
            let stderr = String::from_utf8_lossy(&cmd_output.stderr);
            return Err(BuildError::SigningError(stderr.to_string()));
        }

        info!("APK signed successfully: {:?}", output);
        Ok(())
    }

    /// Sign APK in place
    pub async fn sign_in_place(&self, apk: &PathBuf, config: &SigningConfig) -> Result<(), BuildError> {
        let temp = apk.with_extension("signed.apk");
        self.sign(apk, config, &temp).await?;
        
        // Replace original with signed
        std::fs::rename(&temp, apk)?;
        Ok(())
    }

    /// Verify APK signature
    pub async fn verify(&self, apk: &PathBuf) -> Result<SignatureVerification, BuildError> {
        let apksigner = self.apksigner_path()
            .ok_or_else(|| BuildError::ToolchainNotFound("apksigner not found".into()))?;

        let output = Command::new(&apksigner)
            .args(["verify", "--verbose", "--print-certs"])
            .arg(apk.to_string_lossy().to_string())
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let verified = output.status.success();
        
        let mut verification = SignatureVerification {
            verified,
            v1_signed: stdout.contains("v1 scheme"),
            v2_signed: stdout.contains("v2 scheme"),
            v3_signed: stdout.contains("v3 scheme"),
            v4_signed: stdout.contains("v4 scheme"),
            signer_certs: Vec::new(),
            errors: if verified { Vec::new() } else { vec![stderr.to_string()] },
        };

        // Parse certificate info
        for line in stdout.lines() {
            if line.contains("Signer #") || line.contains("Subject:") {
                verification.signer_certs.push(line.trim().to_string());
            }
        }

        Ok(verification)
    }

    /// Zipalign an APK
    pub async fn zipalign(&self, input: &PathBuf, output: &PathBuf) -> Result<(), BuildError> {
        let zipalign = self.zipalign_path()
            .ok_or_else(|| BuildError::ToolchainNotFound("zipalign not found".into()))?;

        info!("Zipaligning APK: {:?}", input);

        let cmd_output = Command::new(&zipalign)
            .args(["-f", "4"])
            .arg(input.to_string_lossy().to_string())
            .arg(output.to_string_lossy().to_string())
            .output()
            .await?;

        if !cmd_output.status.success() {
            let stderr = String::from_utf8_lossy(&cmd_output.stderr);
            return Err(BuildError::BuildFailed(format!("zipalign failed: {}", stderr)));
        }

        Ok(())
    }

    /// Check if APK needs alignment
    async fn needs_alignment(&self, apk: &PathBuf) -> bool {
        let zipalign = match self.zipalign_path() {
            Some(p) => p,
            None => return false,
        };

        let output = Command::new(&zipalign)
            .args(["-c", "4"])
            .arg(apk.to_string_lossy().to_string())
            .output()
            .await;

        match output {
            Ok(o) => !o.status.success(),
            Err(_) => true,
        }
    }
}

/// Signature verification result
#[derive(Debug, Clone)]
pub struct SignatureVerification {
    pub verified: bool,
    pub v1_signed: bool,
    pub v2_signed: bool,
    pub v3_signed: bool,
    pub v4_signed: bool,
    pub signer_certs: Vec<String>,
    pub errors: Vec<String>,
}

/// Generate a new keystore
pub async fn generate_keystore(
    path: &PathBuf,
    password: &str,
    alias: &str,
    key_password: &str,
    validity_days: u32,
    dn: &str, // Distinguished Name (e.g., "CN=Name, OU=Unit, O=Org, L=City, ST=State, C=US")
) -> Result<(), BuildError> {
    info!("Generating keystore: {:?}", path);

    let output = Command::new("keytool")
        .args([
            "-genkeypair",
            "-keystore", &path.to_string_lossy(),
            "-storepass", password,
            "-alias", alias,
            "-keypass", key_password,
            "-keyalg", "RSA",
            "-keysize", "2048",
            "-validity", &validity_days.to_string(),
            "-dname", dn,
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BuildError::SigningError(format!("keytool failed: {}", stderr)));
    }

    info!("Keystore generated successfully");
    Ok(())
}

/// Get or create debug keystore
pub fn get_debug_keystore() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".android")
            .join("debug.keystore")
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".android")
            .join("debug.keystore")
    }
}
