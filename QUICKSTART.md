# R-Droid 2026 - Build Quick Start Guide

This guide will help you build R-Droid from source.

## Prerequisites

### 1. Install Rust

```powershell
# Using winget (Windows 10/11)
winget install Rustlang.Rust.MSVC

# Or download from https://rustup.rs/
```

### 2. Add Android Targets

```powershell
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

### 3. Install Android SDK (Optional)

R-Droid can auto-download the Android SDK, or you can install Android Studio
and set the `ANDROID_HOME` environment variable.

## Quick Build

### Development Build

```powershell
cd c:\Users\danie\Documents\code\APKIUM

# Build all crates
cargo build

# Run the application
cargo run
```

### Release Build

```powershell
# Build optimized release
cargo build --release

# Run release version
cargo run --release
```

### Build with specific features

```powershell
# Build without UI (CLI only)
cargo build --no-default-features

# Build with UI
cargo build --features ui
```

## Testing

```powershell
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p r-droid-manifest-manager

# Run with output
cargo test -- --nocapture
```

## Creating Distribution

### Windows MSI Installer

Prerequisites:
- Python 3.10+
- WiX Toolset v4 (https://wixtoolset.org/)

```powershell
# Full build and package
python installer/build_all.py

# Just create installer (after cargo build --release)
python installer/build_installer.py --version 0.1.0 --portable
```

### Portable ZIP

```powershell
python installer/build_installer.py --no-msi --portable
```

## Project Structure

```
APKIUM/
├── src/                     # Main binary
│   ├── main.rs             # Entry point
│   ├── lib.rs              # Library exports
│   ├── commands.rs         # CLI commands
│   └── project.rs          # Project management
│
├── crates/                  # Workspace crates
│   ├── core/               # Orchestration & config
│   ├── ui/                 # Slint-based UI
│   ├── editor/             # Code editor
│   ├── android-toolchain/  # SDK/NDK/JDK management
│   ├── manifest-manager/   # AndroidManifest handling
│   ├── emulator-bridge/    # Emulator & ADB
│   └── build-engine/       # Build system
│
├── installer/              # Windows installer scripts
│   ├── build_installer.py  # MSI generator
│   ├── build_all.py        # Full build script
│   └── requirements.txt
│
├── assets/                 # Static assets
│   ├── logo.svg
│   └── theme/
│
├── Cargo.toml              # Workspace manifest
├── README.md               # Project readme
├── ROADMAP.md              # Development roadmap
├── CONTRIBUTING.md         # Contribution guide
├── LICENSE                 # MIT License
└── CHANGELOG.md            # Version history
```

## Crate Dependencies

```
r-droid (main binary)
├── r-droid-core
├── r-droid-ui
├── r-droid-editor
├── r-droid-android-toolchain
├── r-droid-manifest-manager
├── r-droid-emulator-bridge
└── r-droid-build-engine
```

## Troubleshooting

### "Can't find crate" errors

Make sure all workspace members are present:

```powershell
cargo check --workspace
```

### Android target issues

Verify targets are installed:

```powershell
rustup target list --installed
```

### Slint compilation issues

Ensure you have the required Slint dependencies:

```powershell
# Windows: Visual Studio Build Tools with C++ workload
# Install from: https://visualstudio.microsoft.com/visual-cpp-build-tools/
```

## Next Steps

1. **Implement UI**: Add Slint components in `crates/ui/`
2. **Add Editor**: Implement code editing in `crates/editor/`
3. **Test Builds**: Try building a sample Android app
4. **Create Installer**: Use the Python scripts to package

---

For more information, see [ROADMAP.md](ROADMAP.md)
