#!/usr/bin/env python3
"""
R-Droid 2026 - Complete Build and Package Script

This script orchestrates the full build process:
1. Build Rust project in release mode
2. Generate Windows MSI installer
3. Create portable ZIP distribution
4. Generate checksums

Usage:
    python installer/build_all.py [--release] [--skip-tests]
"""

import os
import sys
import shutil
import argparse
import subprocess
from pathlib import Path
from datetime import datetime

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
DIST_DIR = PROJECT_ROOT / "dist"


def run_command(cmd: list, cwd: Path = None, env: dict = None) -> bool:
    """Run a command and return success status."""
    print(f"\n>> {' '.join(cmd)}")
    
    full_env = os.environ.copy()
    if env:
        full_env.update(env)
    
    result = subprocess.run(
        cmd,
        cwd=cwd or PROJECT_ROOT,
        env=full_env
    )
    
    return result.returncode == 0


def check_rust_toolchain() -> bool:
    """Check if Rust toolchain is available."""
    print("Checking Rust toolchain...")
    
    try:
        result = subprocess.run(
            ["rustc", "--version"],
            capture_output=True,
            text=True
        )
        print(f"  Found: {result.stdout.strip()}")
        
        result = subprocess.run(
            ["cargo", "--version"],
            capture_output=True,
            text=True
        )
        print(f"  Found: {result.stdout.strip()}")
        
        return True
    except FileNotFoundError:
        print("ERROR: Rust toolchain not found!")
        print("Install from: https://rustup.rs/")
        return False


def check_android_targets() -> bool:
    """Check and install Android Rust targets."""
    print("\nChecking Android targets...")
    
    targets = [
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "x86_64-linux-android",
        "i686-linux-android",
    ]
    
    # Get installed targets
    result = subprocess.run(
        ["rustup", "target", "list", "--installed"],
        capture_output=True,
        text=True
    )
    installed = result.stdout.strip().split("\n")
    
    missing = [t for t in targets if t not in installed]
    
    if missing:
        print(f"Installing missing targets: {missing}")
        for target in missing:
            run_command(["rustup", "target", "add", target])
    else:
        print("  All Android targets installed")
    
    return True


def build_rust_project(release: bool = True, skip_tests: bool = False) -> bool:
    """Build the Rust project."""
    print("\n" + "="*60)
    print("Building Rust Project")
    print("="*60)
    
    # Run tests first (unless skipped)
    if not skip_tests:
        print("\nRunning tests...")
        if not run_command(["cargo", "test", "--workspace"]):
            print("Tests failed!")
            return False
    
    # Build release or debug
    build_args = ["cargo", "build", "--workspace"]
    if release:
        build_args.append("--release")
    
    print(f"\nBuilding {'release' if release else 'debug'} version...")
    if not run_command(build_args):
        print("Build failed!")
        return False
    
    print("\nBuild successful!")
    return True


def build_installer(version: str) -> bool:
    """Build the Windows installer."""
    print("\n" + "="*60)
    print("Building Windows Installer")
    print("="*60)
    
    # Ensure dist directory exists
    DIST_DIR.mkdir(parents=True, exist_ok=True)
    
    # Run installer builder
    return run_command([
        sys.executable,
        str(SCRIPT_DIR / "build_installer.py"),
        "--version", version,
        "--portable"
    ])


def generate_version() -> str:
    """Generate version string from Cargo.toml."""
    cargo_toml = PROJECT_ROOT / "Cargo.toml"
    
    if cargo_toml.exists():
        content = cargo_toml.read_text()
        for line in content.split("\n"):
            if line.startswith("version = "):
                version = line.split('"')[1]
                print(f"Version from Cargo.toml: {version}")
                return version
    
    # Fallback to date-based version
    return datetime.now().strftime("0.%Y.%m%d")


def clean_build() -> None:
    """Clean previous build artifacts."""
    print("Cleaning previous build...")
    
    # Clean cargo
    run_command(["cargo", "clean"])
    
    # Clean dist
    if DIST_DIR.exists():
        shutil.rmtree(DIST_DIR)


def generate_release_notes(version: str) -> Path:
    """Generate release notes template."""
    notes_path = DIST_DIR / f"RELEASE_NOTES_{version}.md"
    
    content = f"""# R-Droid 2026 v{version} Release Notes

Released: {datetime.now().strftime("%Y-%m-%d")}

## What's New

### Features
- [Add your features here]

### Bug Fixes
- [Add your bug fixes here]

### Improvements
- [Add your improvements here]

## Installation

### Windows MSI Installer
1. Download `R-Droid-{version}-x64.msi`
2. Run the installer
3. Follow the installation wizard

### Portable Version
1. Download `R-Droid-{version}-portable-x64.zip`
2. Extract to your preferred location
3. Run `r-droid.exe`

## System Requirements

- Windows 10 (1903+) or Windows 11
- 8 GB RAM minimum (16 GB recommended)
- 4 GB disk space
- Internet connection for SDK downloads

## Known Issues

- [List any known issues]

## Checksums

See `SHA256SUMS.txt` for file checksums.
"""
    
    notes_path.write_text(content)
    print(f"\nGenerated release notes: {notes_path}")
    return notes_path


def main():
    parser = argparse.ArgumentParser(
        description="Build R-Droid and create distribution packages"
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Build debug version (default: release)"
    )
    parser.add_argument(
        "--skip-tests",
        action="store_true",
        help="Skip running tests"
    )
    parser.add_argument(
        "--skip-installer",
        action="store_true",
        help="Skip creating installer"
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Clean before building"
    )
    parser.add_argument(
        "--version", "-v",
        help="Override version string"
    )
    
    args = parser.parse_args()
    
    print("="*60)
    print("R-Droid 2026 - Complete Build Script")
    print("="*60)
    
    # Check prerequisites
    if not check_rust_toolchain():
        return 1
    
    check_android_targets()
    
    # Get version
    version = args.version or generate_version()
    print(f"\nBuilding version: {version}")
    
    # Clean if requested
    if args.clean:
        clean_build()
    
    # Build Rust project
    if not build_rust_project(
        release=not args.debug,
        skip_tests=args.skip_tests
    ):
        return 1
    
    # Build installer
    if not args.skip_installer:
        if not build_installer(version):
            print("Warning: Installer build failed (WiX may not be installed)")
    
    # Generate release notes
    generate_release_notes(version)
    
    print("\n" + "="*60)
    print("BUILD COMPLETE!")
    print("="*60)
    print(f"\nOutput directory: {DIST_DIR}")
    
    if DIST_DIR.exists():
        print("\nGenerated files:")
        for f in DIST_DIR.iterdir():
            size = f.stat().st_size / 1024 / 1024
            print(f"  - {f.name} ({size:.2f} MB)")
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
