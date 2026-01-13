# R-Droid 2026 - Pure Rust Android IDE

<p align="center">
  <img src="assets/logo.png" alt="R-Droid Logo" width="200">
</p>

<p align="center">
  <strong>A modern, blazing-fast Android IDE built entirely in Rust</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#building">Building</a> •
  <a href="#roadmap">Roadmap</a>
</p>

---

## 🚀 Features

### Core Capabilities

- **Pure Rust Performance** - Built from the ground up in Rust for maximum speed and memory safety
- **Modern UI** - Slint-based interface with glassmorphism aesthetics and fluid animations
- **Zero Configuration** - Automatic detection and download of Android SDK, NDK, and JDK
- **Multi-Platform Support** - Windows 10/11 primary, with Linux and macOS planned

### Development Tools

- **Smart Code Editor** - Syntax highlighting, code completion, and error detection
- **Manifest Editor** - Visual and code modes with bi-directional sync
- **Emulator Integration** - One-click AVD setup and device management
- **ADB Tools** - Logcat viewer, file explorer, and shell access

### Build System

- **One-Click Builds** - Build APK/AAB with automatic signing
- **Multi-ABI Support** - Build for ARM64, ARMv7, x86, and x86_64
- **Cargo-APK Integration** - First-class support for Rust Android apps
- **Gradle Support** - Full Kotlin/Java project compatibility

## 📦 Installation

### Windows (Recommended)

1. Download the latest MSI installer from [Releases](https://github.com/r-droid/r-droid/releases)
2. Run `R-Droid-x.x.x-x64.msi`
3. Follow the installation wizard
4. Launch R-Droid from the Start Menu

### Portable Version

1. Download `R-Droid-x.x.x-portable-x64.zip`
2. Extract to your preferred location
3. Run `r-droid.exe`

### From Source

```bash
# Clone the repository
git clone https://github.com/r-droid/r-droid.git
cd r-droid

# Build release version
cargo build --release

# Run
./target/release/r-droid
```

## 🏃 Quick Start

### Create a New Rust Android Project

1. Launch R-Droid
2. Click **File → New Project → Rust Android App**
3. Enter project name and package ID
4. Click **Create**

### Build and Run

1. Open your project
2. Connect an Android device or start an emulator
3. Click the **Run** button (or press F5)
4. Your app will be built, installed, and launched automatically

### Visual Manifest Editor

1. Open `AndroidManifest.xml` from the Project panel
2. Toggle between **Visual** and **Code** modes using the tabs
3. Changes sync automatically in both directions

## 🔧 Building from Source

### Prerequisites

- **Rust 1.75+** (install from https://rustup.rs/)
- **Android SDK** (auto-downloaded or manual installation)
- **Windows 10/11** (for full UI support)

### Build Steps

```bash
# Install Rust targets for Android
rustup target add aarch64-linux-android armv7-linux-androideabi

# Clone and build
git clone https://github.com/r-droid/r-droid.git
cd r-droid
cargo build --release

# Run tests
cargo test --workspace

# Create installer (requires WiX Toolset)
python installer/build_all.py
```

### Project Structure

```
r-droid/
├── src/                    # Main application
│   ├── main.rs            # Entry point
│   ├── commands.rs        # CLI commands
│   └── project.rs         # Project management
├── crates/
│   ├── core/              # Core orchestrator
│   ├── ui/                # Slint UI
│   ├── editor/            # Code editor
│   ├── android-toolchain/ # SDK/NDK management
│   ├── manifest-manager/  # AndroidManifest handling
│   ├── emulator-bridge/   # Emulator & ADB
│   └── build-engine/      # Build system
└── installer/             # Windows installer scripts
```

## 📋 Roadmap

See [ROADMAP.md](ROADMAP.md) for the detailed development roadmap.

### Version 0.1 (Current)
- ✅ Core UI framework
- ✅ Android toolchain detection/download
- ✅ Manifest editor (visual + code)
- ✅ Emulator management
- ✅ Build system with signing
- 🚧 Windows installer

### Version 0.2 (Planned)
- 🔲 Full Slint UI implementation
- 🔲 Code completion with rust-analyzer
- 🔲 Project templates
- 🔲 Settings UI

### Version 0.3 (Planned)
- 🔲 Debugger integration
- 🔲 Profiler tools
- 🔲 Linux support
- 🔲 Plugin system

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [Slint](https://slint-ui.com/) - Rust UI framework
- [cargo-apk](https://github.com/rust-mobile/cargo-apk) - Rust Android builds
- [Android Studio](https://developer.android.com/studio) - Inspiration for features

---

<p align="center">
  Made with ❤️ and 🦀 by the R-Droid Team
</p>
