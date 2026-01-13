//! Logcat Reader
//!
//! Reads and parses Android logcat output.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::debug;

/// Logcat errors
#[derive(Debug, thiserror::Error)]
pub enum LogcatError {
    #[error("ADB not found")]
    AdbNotFound,
    #[error("Device not found")]
    DeviceNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
    Silent,
}

impl LogLevel {
    pub fn from_char(c: char) -> Self {
        match c {
            'V' => LogLevel::Verbose,
            'D' => LogLevel::Debug,
            'I' => LogLevel::Info,
            'W' => LogLevel::Warning,
            'E' => LogLevel::Error,
            'F' => LogLevel::Fatal,
            'S' => LogLevel::Silent,
            _ => LogLevel::Info,
        }
    }

    pub fn as_char(&self) -> char {
        match self {
            LogLevel::Verbose => 'V',
            LogLevel::Debug => 'D',
            LogLevel::Info => 'I',
            LogLevel::Warning => 'W',
            LogLevel::Error => 'E',
            LogLevel::Fatal => 'F',
            LogLevel::Silent => 'S',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::Verbose => "Verbose",
            LogLevel::Debug => "Debug",
            LogLevel::Info => "Info",
            LogLevel::Warning => "Warning",
            LogLevel::Error => "Error",
            LogLevel::Fatal => "Fatal",
            LogLevel::Silent => "Silent",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            LogLevel::Verbose => "#808080",
            LogLevel::Debug => "#0000FF",
            LogLevel::Info => "#00FF00",
            LogLevel::Warning => "#FFFF00",
            LogLevel::Error => "#FF0000",
            LogLevel::Fatal => "#FF00FF",
            LogLevel::Silent => "#FFFFFF",
        }
    }
}

/// Parsed log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp
    pub timestamp: String,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
    /// Log level
    pub level: LogLevel,
    /// Tag
    pub tag: String,
    /// Message
    pub message: String,
    /// Raw line
    pub raw: String,
}

impl LogEntry {
    /// Parse a logcat line (threadtime format)
    /// Format: MM-DD HH:MM:SS.mmm PID TID LEVEL TAG: MESSAGE
    pub fn parse(line: &str) -> Option<Self> {
        if line.len() < 30 {
            return None;
        }

        // Try to parse threadtime format
        let parts: Vec<&str> = line.splitn(7, ' ').collect();
        if parts.len() < 7 {
            return None;
        }

        let timestamp = format!("{} {}", parts[0], parts[1]);
        let pid: u32 = parts[2].trim().parse().ok()?;
        let tid: u32 = parts[3].trim().parse().ok()?;
        let level = LogLevel::from_char(parts[4].chars().next()?);
        
        // Tag and message are separated by ": "
        let rest = parts[5..].join(" ");
        let (tag, message) = if let Some(idx) = rest.find(": ") {
            (rest[..idx].to_string(), rest[idx + 2..].to_string())
        } else {
            (rest.clone(), String::new())
        };

        Some(LogEntry {
            timestamp,
            pid,
            tid,
            level,
            tag,
            message,
            raw: line.to_string(),
        })
    }

    /// Format as colored string
    pub fn formatted(&self) -> String {
        format!(
            "{} {} {} {}/{}: {}",
            self.timestamp,
            self.pid,
            self.tid,
            self.level.as_char(),
            self.tag,
            self.message
        )
    }
}

/// Logcat filter
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// Minimum log level
    pub min_level: Option<LogLevel>,
    /// Filter by tag (exact match)
    pub tags: Vec<String>,
    /// Filter by tag (contains)
    pub tag_contains: Option<String>,
    /// Filter by message (contains)
    pub message_contains: Option<String>,
    /// Filter by package/process name
    pub package: Option<String>,
    /// PIDs to filter
    pub pids: Vec<u32>,
}

impl LogFilter {
    /// Create a filter for a specific package
    pub fn for_package(package: &str) -> Self {
        Self {
            package: Some(package.to_string()),
            ..Default::default()
        }
    }

    /// Create a filter for a minimum log level
    pub fn min_level(level: LogLevel) -> Self {
        Self {
            min_level: Some(level),
            ..Default::default()
        }
    }

    /// Check if log entry matches filter
    pub fn matches(&self, entry: &LogEntry) -> bool {
        // Check minimum level
        if let Some(min) = self.min_level {
            if entry.level < min {
                return false;
            }
        }

        // Check exact tag match
        if !self.tags.is_empty() && !self.tags.iter().any(|t| t == &entry.tag) {
            return false;
        }

        // Check tag contains
        if let Some(ref pattern) = self.tag_contains {
            if !entry.tag.contains(pattern) {
                return false;
            }
        }

        // Check message contains
        if let Some(ref pattern) = self.message_contains {
            if !entry.message.contains(pattern) {
                return false;
            }
        }

        // Check PIDs
        if !self.pids.is_empty() && !self.pids.contains(&entry.pid) {
            return false;
        }

        true
    }

    /// Convert to logcat filter spec
    pub fn to_filter_spec(&self) -> Vec<String> {
        let mut specs = Vec::new();

        if self.tags.is_empty() {
            if let Some(level) = self.min_level {
                specs.push(format!("*:{}", level.as_char()));
            }
        } else {
            for tag in &self.tags {
                let level = self.min_level.unwrap_or(LogLevel::Verbose);
                specs.push(format!("{}:{}", tag, level.as_char()));
            }
            specs.push("*:S".to_string()); // Silence everything else
        }

        specs
    }
}

/// Logcat reader
pub struct LogcatReader {
    sdk_path: PathBuf,
    serial: String,
}

impl LogcatReader {
    /// Create a new logcat reader
    pub fn new(sdk_path: PathBuf, serial: &str) -> Self {
        Self {
            sdk_path,
            serial: serial.to_string(),
        }
    }

    /// Get ADB path
    fn adb_path(&self) -> PathBuf {
        let platform_tools = self.sdk_path.join("platform-tools");
        if cfg!(windows) {
            platform_tools.join("adb.exe")
        } else {
            platform_tools.join("adb")
        }
    }

    /// Clear logcat buffer
    pub async fn clear(&self) -> Result<(), LogcatError> {
        let adb = self.adb_path();
        
        Command::new(&adb)
            .args(["-s", &self.serial, "logcat", "-c"])
            .output()
            .await?;
        
        Ok(())
    }

    /// Read logcat dump (one-shot)
    pub async fn dump(&self, filter: Option<&LogFilter>) -> Result<Vec<LogEntry>, LogcatError> {
        let adb = self.adb_path();
        
        if !adb.exists() {
            return Err(LogcatError::AdbNotFound);
        }

        let mut args = vec!["-s", &self.serial, "logcat", "-d", "-v", "threadtime"];
        
        let filter_specs: Vec<String>;
        if let Some(f) = filter {
            filter_specs = f.to_filter_spec();
            for spec in &filter_specs {
                args.push(spec);
            }
        }

        let output = Command::new(&adb)
            .args(&args)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries: Vec<LogEntry> = stdout
            .lines()
            .filter_map(LogEntry::parse)
            .filter(|e| filter.map(|f| f.matches(e)).unwrap_or(true))
            .collect();

        Ok(entries)
    }

    /// Start streaming logcat
    pub async fn stream(&self, filter: Option<LogFilter>) -> Result<mpsc::Receiver<LogEntry>, LogcatError> {
        let adb = self.adb_path();
        
        if !adb.exists() {
            return Err(LogcatError::AdbNotFound);
        }

        let mut args = vec![
            "-s".to_string(),
            self.serial.clone(),
            "logcat".to_string(),
            "-v".to_string(),
            "threadtime".to_string(),
        ];
        
        if let Some(ref f) = filter {
            for spec in f.to_filter_spec() {
                args.push(spec);
            }
        }

        let mut child = Command::new(&adb)
            .args(&args)
            .stdout(Stdio::piped())
            .spawn()?;

        let (tx, rx) = mpsc::channel(1000);
        let filter = filter.clone();

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(entry) = LogEntry::parse(&line) {
                        let should_send = filter.as_ref()
                            .map(|f| f.matches(&entry))
                            .unwrap_or(true);
                        
                        if should_send {
                            if tx.send(entry).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            });
        }

        Ok(rx)
    }

    /// Get logcat for a specific package by finding its PID
    pub async fn stream_for_package(&self, package: &str) -> Result<mpsc::Receiver<LogEntry>, LogcatError> {
        let adb = self.adb_path();
        
        // Get PID of the package
        let output = Command::new(&adb)
            .args(["-s", &self.serial, "shell", "pidof", "-s", package])
            .output()
            .await?;

        let pid: Option<u32> = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .ok();

        let filter = if let Some(p) = pid {
            Some(LogFilter {
                pids: vec![p],
                ..Default::default()
            })
        } else {
            None
        };

        self.stream(filter).await
    }
}

/// Logcat buffer types
#[derive(Debug, Clone, Copy)]
pub enum LogBuffer {
    Main,
    System,
    Radio,
    Events,
    Crash,
    All,
}

impl LogBuffer {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogBuffer::Main => "main",
            LogBuffer::System => "system",
            LogBuffer::Radio => "radio",
            LogBuffer::Events => "events",
            LogBuffer::Crash => "crash",
            LogBuffer::All => "all",
        }
    }
}
