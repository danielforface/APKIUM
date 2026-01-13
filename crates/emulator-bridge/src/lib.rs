//! Android Emulator Bridge
//!
//! Manages Android Virtual Devices (AVDs) and emulator instances.

pub mod avd;
pub mod emulator;
pub mod adb;
pub mod device;
pub mod logcat;

pub use avd::{AvdManager, AvdConfig, AvdInfo};
pub use emulator::{EmulatorLauncher, EmulatorInstance, EmulatorOptions};
pub use adb::{AdbClient, AdbDevice, AdbCommand};
pub use device::{Device, DeviceState, DeviceType};
pub use logcat::{LogcatReader, LogEntry, LogLevel};

/// Default emulator console port
pub const DEFAULT_CONSOLE_PORT: u16 = 5554;

/// Default ADB port
pub const DEFAULT_ADB_PORT: u16 = 5037;

/// Emulator port range
pub const EMULATOR_PORT_RANGE: std::ops::Range<u16> = 5554..5584;

/// Get next available emulator port
pub fn next_emulator_port(used_ports: &[u16]) -> Option<u16> {
    for port in (5554..5584).step_by(2) {
        if !used_ports.contains(&port) {
            return Some(port);
        }
    }
    None
}
