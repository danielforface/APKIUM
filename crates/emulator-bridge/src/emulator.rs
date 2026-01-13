//! Emulator Launcher
//!
//! Launches and manages Android emulator instances.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Command, Child};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{info, debug, warn, error};

/// Emulator errors
#[derive(Debug, thiserror::Error)]
pub enum EmulatorError {
    #[error("Emulator not found")]
    NotFound,
    #[error("AVD not found: {0}")]
    AvdNotFound(String),
    #[error("Failed to start emulator: {0}")]
    StartFailed(String),
    #[error("Emulator crashed: {0}")]
    Crashed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Emulator launch options
#[derive(Debug, Clone, Default)]
pub struct EmulatorOptions {
    /// GPU mode (auto, host, swiftshader_indirect, etc.)
    pub gpu: Option<String>,
    /// Enable/disable audio
    pub no_audio: bool,
    /// Enable/disable window
    pub no_window: bool,
    /// Enable/disable boot animation
    pub no_boot_anim: bool,
    /// Memory size in MB
    pub memory: Option<u32>,
    /// Number of cores
    pub cores: Option<u32>,
    /// Wipe data on launch
    pub wipe_data: bool,
    /// Enable cold boot
    pub cold_boot: bool,
    /// HTTP proxy
    pub http_proxy: Option<String>,
    /// DNS servers
    pub dns_servers: Option<String>,
    /// Port for console
    pub port: Option<u16>,
    /// Additional arguments
    pub extra_args: Vec<String>,
}

impl EmulatorOptions {
    /// Default options for development
    pub fn for_development() -> Self {
        Self {
            gpu: Some("auto".to_string()),
            no_boot_anim: true,
            ..Default::default()
        }
    }

    /// Options for headless/CI environments
    pub fn headless() -> Self {
        Self {
            gpu: Some("swiftshader_indirect".to_string()),
            no_audio: true,
            no_window: true,
            no_boot_anim: true,
            ..Default::default()
        }
    }

    /// Convert to command line arguments
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(ref gpu) = self.gpu {
            args.push("-gpu".to_string());
            args.push(gpu.clone());
        }

        if self.no_audio {
            args.push("-no-audio".to_string());
        }

        if self.no_window {
            args.push("-no-window".to_string());
        }

        if self.no_boot_anim {
            args.push("-no-boot-anim".to_string());
        }

        if let Some(memory) = self.memory {
            args.push("-memory".to_string());
            args.push(memory.to_string());
        }

        if let Some(cores) = self.cores {
            args.push("-cores".to_string());
            args.push(cores.to_string());
        }

        if self.wipe_data {
            args.push("-wipe-data".to_string());
        }

        if self.cold_boot {
            args.push("-no-snapshot-load".to_string());
        }

        if let Some(ref proxy) = self.http_proxy {
            args.push("-http-proxy".to_string());
            args.push(proxy.clone());
        }

        if let Some(ref dns) = self.dns_servers {
            args.push("-dns-server".to_string());
            args.push(dns.clone());
        }

        if let Some(port) = self.port {
            args.push("-port".to_string());
            args.push(port.to_string());
        }

        args.extend(self.extra_args.clone());

        args
    }
}

/// Running emulator instance
pub struct EmulatorInstance {
    pub avd_name: String,
    pub port: u16,
    pub adb_port: u16,
    process: Option<Child>,
    started: bool,
}

impl EmulatorInstance {
    /// Get the serial for ADB
    pub fn serial(&self) -> String {
        format!("emulator-{}", self.port)
    }

    /// Check if the emulator process is still running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.started = false;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Kill the emulator
    pub async fn kill(&mut self) -> Result<(), EmulatorError> {
        if let Some(ref mut child) = self.process {
            child.kill().await?;
            self.started = false;
            info!("Killed emulator: {}", self.avd_name);
        }
        Ok(())
    }

    /// Wait for emulator to exit
    pub async fn wait(&mut self) -> Result<i32, EmulatorError> {
        if let Some(ref mut child) = self.process {
            let status = child.wait().await?;
            self.started = false;
            Ok(status.code().unwrap_or(-1))
        } else {
            Ok(0)
        }
    }
}

/// Emulator launcher
pub struct EmulatorLauncher {
    sdk_path: PathBuf,
    running_instances: Vec<EmulatorInstance>,
}

impl EmulatorLauncher {
    /// Create a new emulator launcher
    pub fn new(sdk_path: PathBuf) -> Self {
        Self {
            sdk_path,
            running_instances: Vec::new(),
        }
    }

    /// Get the emulator executable path
    fn emulator_path(&self) -> PathBuf {
        let emulator_dir = self.sdk_path.join("emulator");
        if cfg!(windows) {
            emulator_dir.join("emulator.exe")
        } else {
            emulator_dir.join("emulator")
        }
    }

    /// Check if emulator is available
    pub fn is_available(&self) -> bool {
        self.emulator_path().exists()
    }

    /// Launch an emulator
    pub async fn launch(
        &mut self,
        avd_name: &str,
        options: EmulatorOptions,
    ) -> Result<&EmulatorInstance, EmulatorError> {
        let emulator = self.emulator_path();
        
        if !emulator.exists() {
            return Err(EmulatorError::NotFound);
        }

        // Determine port
        let port = options.port.unwrap_or_else(|| {
            let used: Vec<u16> = self.running_instances.iter().map(|i| i.port).collect();
            crate::next_emulator_port(&used).unwrap_or(5554)
        });

        info!("Launching emulator {} on port {}", avd_name, port);

        let mut args = vec!["-avd".to_string(), avd_name.to_string()];
        
        // Add port if not in options
        if options.port.is_none() {
            args.push("-port".to_string());
            args.push(port.to_string());
        }
        
        args.extend(options.to_args());

        debug!("Emulator args: {:?}", args);

        let child = Command::new(&emulator)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let instance = EmulatorInstance {
            avd_name: avd_name.to_string(),
            port,
            adb_port: port + 1,
            process: Some(child),
            started: true,
        };

        self.running_instances.push(instance);
        
        Ok(self.running_instances.last().unwrap())
    }

    /// Launch and wait for boot
    pub async fn launch_and_wait(
        &mut self,
        avd_name: &str,
        options: EmulatorOptions,
        timeout_secs: u64,
    ) -> Result<&EmulatorInstance, EmulatorError> {
        let instance = self.launch(avd_name, options).await?;
        let serial = instance.serial();
        
        info!("Waiting for emulator to boot...");

        // Wait for device to be ready
        let adb = crate::adb::AdbClient::new(self.sdk_path.clone());
        
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);
        
        while tokio::time::Instant::now() < deadline {
            if let Ok(devices) = adb.list_devices().await {
                if devices.iter().any(|d| d.serial == serial && d.state == crate::device::DeviceState::Online) {
                    // Device is online, now wait for boot completed
                    if let Ok(output) = adb.shell(&serial, "getprop sys.boot_completed").await {
                        if output.trim() == "1" {
                            info!("Emulator booted successfully");
                            return Ok(self.running_instances.last().unwrap());
                        }
                    }
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        Err(EmulatorError::StartFailed("Boot timeout".into()))
    }

    /// Get all running emulator instances
    pub fn running_instances(&mut self) -> &mut Vec<EmulatorInstance> {
        // Clean up dead instances
        self.running_instances.retain_mut(|i| i.is_running());
        &mut self.running_instances
    }

    /// Stop all running emulators
    pub async fn stop_all(&mut self) -> Result<(), EmulatorError> {
        for instance in &mut self.running_instances {
            let _ = instance.kill().await;
        }
        self.running_instances.clear();
        Ok(())
    }

    /// Find instance by AVD name
    pub fn find_by_avd(&mut self, avd_name: &str) -> Option<&mut EmulatorInstance> {
        self.running_instances.iter_mut().find(|i| i.avd_name == avd_name)
    }

    /// Find instance by port
    pub fn find_by_port(&mut self, port: u16) -> Option<&mut EmulatorInstance> {
        self.running_instances.iter_mut().find(|i| i.port == port)
    }
}

/// Emulator output event
#[derive(Debug, Clone)]
pub enum EmulatorEvent {
    Stdout(String),
    Stderr(String),
    Started,
    Crashed(String),
    Exited(i32),
}

/// Spawn emulator with event channel
pub async fn spawn_with_events(
    sdk_path: PathBuf,
    avd_name: &str,
    options: EmulatorOptions,
) -> Result<(Child, mpsc::Receiver<EmulatorEvent>), EmulatorError> {
    let emulator_dir = sdk_path.join("emulator");
    let emulator = if cfg!(windows) {
        emulator_dir.join("emulator.exe")
    } else {
        emulator_dir.join("emulator")
    };

    if !emulator.exists() {
        return Err(EmulatorError::NotFound);
    }

    let mut args = vec!["-avd".to_string(), avd_name.to_string()];
    args.extend(options.to_args());

    let mut child = Command::new(&emulator)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let (tx, rx) = mpsc::channel(100);
    let tx_clone = tx.clone();

    // Spawn stdout reader
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(EmulatorEvent::Stdout(line)).await;
            }
        });
    }

    // Spawn stderr reader
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx_clone.send(EmulatorEvent::Stderr(line)).await;
            }
        });
    }

    Ok((child, rx))
}
