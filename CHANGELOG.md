# R-Droid 2026 Changelog

All notable changes to R-Droid will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure with 7 modular crates
- Android SDK/NDK/JDK auto-detection and download
- AndroidManifest.xml bi-directional parser/writer
- AVD management and emulator launcher
- ADB client implementation with full command set
- Logcat streaming with filtering
- Build engine with cargo-apk and Gradle support
- APK signing with v1-v4 signature schemes
- One-click build and run functionality
- Windows MSI installer generator
- Portable ZIP distribution

### Technical
- Rust workspace with shared dependencies
- Tokio async runtime throughout
- Comprehensive error handling with anyhow/thiserror
- Tracing-based logging infrastructure

## [0.1.0] - 2025-XX-XX

### Added
- Initial public release
- Core functionality for Android development in Rust
- Windows 10/11 support

---

## Version History

| Version | Release Date | Status |
|---------|-------------|--------|
| 0.1.0   | TBD         | Development |
| 0.2.0   | TBD         | Planned |
| 0.3.0   | TBD         | Planned |
| 1.0.0   | Q2 2026     | Planned |
