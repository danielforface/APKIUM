#!/usr/bin/env python3
"""
R-Droid 2026 - Windows MSI Installer Generator

This script generates a professional Windows MSI installer for R-Droid IDE.
Uses WiX Toolset v4 for MSI generation with modern Windows 11 aesthetics.

Requirements:
- Python 3.10+
- WiX Toolset v4 (https://wixtoolset.org/)
- Rust build artifacts in target/release/

Usage:
    python installer/build_installer.py [--version VERSION] [--output OUTPUT_DIR]
"""

import os
import sys
import json
import shutil
import hashlib
import tempfile
import argparse
import subprocess
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional, List, Dict
from xml.etree import ElementTree as ET
from xml.dom import minidom

# Version constants
DEFAULT_VERSION = "0.1.0"
PRODUCT_NAME = "R-Droid 2026"
MANUFACTURER = "R-Droid Team"
UPGRADE_CODE = "12345678-1234-1234-1234-123456789012"  # Fixed GUID for upgrades

# Directory structure
SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
BUILD_DIR = PROJECT_ROOT / "target" / "release"
DIST_DIR = PROJECT_ROOT / "dist"


@dataclass
class InstallerConfig:
    """Configuration for the installer."""
    version: str = DEFAULT_VERSION
    product_name: str = PRODUCT_NAME
    manufacturer: str = MANUFACTURER
    upgrade_code: str = UPGRADE_CODE
    output_dir: Path = field(default_factory=lambda: DIST_DIR)
    license_file: Optional[Path] = None
    icon_file: Optional[Path] = None
    banner_file: Optional[Path] = None
    dialog_file: Optional[Path] = None
    
    # Feature flags
    include_sdk_manager: bool = True
    include_emulator_support: bool = True
    create_desktop_shortcut: bool = True
    create_start_menu_shortcut: bool = True
    add_to_path: bool = True


@dataclass
class FileEntry:
    """Represents a file to be included in the installer."""
    source_path: Path
    target_name: str
    component_id: str
    directory: str = "INSTALLFOLDER"
    is_executable: bool = False
    
    def __post_init__(self):
        # Generate component ID if not provided
        if not self.component_id:
            self.component_id = f"Component_{self.target_name.replace('.', '_').replace('-', '_')}"


class InstallerBuilder:
    """Builds the MSI installer using WiX Toolset."""
    
    def __init__(self, config: InstallerConfig):
        self.config = config
        self.files: List[FileEntry] = []
        self.directories: Dict[str, str] = {}
        
    def collect_files(self) -> None:
        """Collect all files to be included in the installer."""
        print("Collecting files...")
        
        # Main executable
        exe_path = BUILD_DIR / "r-droid.exe"
        if exe_path.exists():
            self.files.append(FileEntry(
                source_path=exe_path,
                target_name="r-droid.exe",
                component_id="Component_MainExecutable",
                is_executable=True
            ))
        else:
            print(f"Warning: Main executable not found at {exe_path}")
        
        # DLL dependencies (if any)
        for dll in BUILD_DIR.glob("*.dll"):
            self.files.append(FileEntry(
                source_path=dll,
                target_name=dll.name,
                component_id=f"Component_{dll.stem.replace('-', '_')}"
            ))
        
        # Resources
        resources_dir = PROJECT_ROOT / "resources"
        if resources_dir.exists():
            self._collect_directory(resources_dir, "resources")
        
        # Assets
        assets_dir = PROJECT_ROOT / "assets"
        if assets_dir.exists():
            self._collect_directory(assets_dir, "assets")
            
        print(f"Collected {len(self.files)} files")
    
    def _collect_directory(self, dir_path: Path, target_subdir: str) -> None:
        """Recursively collect files from a directory."""
        for item in dir_path.rglob("*"):
            if item.is_file():
                rel_path = item.relative_to(dir_path)
                target_dir = f"INSTALLFOLDER\\{target_subdir}\\{rel_path.parent}".rstrip("\\.")
                
                # Register directory
                dir_id = target_dir.replace("\\", "_").replace(".", "_")
                self.directories[target_dir] = dir_id
                
                self.files.append(FileEntry(
                    source_path=item,
                    target_name=item.name,
                    component_id=f"Component_{item.stem.replace('-', '_').replace('.', '_')}_{hashlib.md5(str(item).encode()).hexdigest()[:8]}",
                    directory=dir_id
                ))
    
    def generate_wix_source(self) -> str:
        """Generate WiX source XML."""
        print("Generating WiX source...")
        
        # Create root element with namespaces
        wix = ET.Element("Wix")
        wix.set("xmlns", "http://wixtoolset.org/schemas/v4/wxs")
        wix.set("xmlns:ui", "http://wixtoolset.org/schemas/v4/wxs/ui")
        
        # Package element
        package = ET.SubElement(wix, "Package")
        package.set("Name", self.config.product_name)
        package.set("Manufacturer", self.config.manufacturer)
        package.set("Version", self.config.version)
        package.set("UpgradeCode", self.config.upgrade_code)
        package.set("Scope", "perMachine")
        package.set("Compressed", "yes")
        
        # Upgrade handling (allows upgrades and prevents downgrades)
        major_upgrade = ET.SubElement(package, "MajorUpgrade")
        major_upgrade.set("DowngradeErrorMessage", 
                         "A newer version of [ProductName] is already installed.")
        
        # Media template (embedded cabinet)
        media = ET.SubElement(package, "MediaTemplate")
        media.set("EmbedCab", "yes")
        
        # Standard directories
        standard_dir = ET.SubElement(package, "StandardDirectory")
        standard_dir.set("Id", "ProgramFiles64Folder")
        
        install_dir = ET.SubElement(standard_dir, "Directory")
        install_dir.set("Id", "INSTALLFOLDER")
        install_dir.set("Name", "R-Droid")
        
        # Add subdirectories
        for dir_path, dir_id in sorted(self.directories.items()):
            if dir_id != "INSTALLFOLDER":
                parts = dir_path.split("\\")
                if len(parts) > 1:
                    self._add_directory_tree(install_dir, parts[1:], dir_id)
        
        # Add components and files
        component_group = ET.SubElement(package, "ComponentGroup")
        component_group.set("Id", "ProductComponents")
        component_group.set("Directory", "INSTALLFOLDER")
        
        for file_entry in self.files:
            component = ET.SubElement(component_group, "Component")
            component.set("Id", file_entry.component_id)
            if file_entry.directory != "INSTALLFOLDER":
                component.set("Directory", file_entry.directory)
            
            file_elem = ET.SubElement(component, "File")
            file_elem.set("Source", str(file_entry.source_path))
            file_elem.set("Name", file_entry.target_name)
            
            if file_entry.is_executable:
                file_elem.set("Id", "MainExecutable")
        
        # Add shortcuts feature
        if self.config.create_desktop_shortcut or self.config.create_start_menu_shortcut:
            self._add_shortcuts(package)
        
        # Add PATH environment variable
        if self.config.add_to_path:
            self._add_path_component(component_group)
        
        # Feature element
        feature = ET.SubElement(package, "Feature")
        feature.set("Id", "ProductFeature")
        feature.set("Title", "R-Droid IDE")
        feature.set("Level", "1")
        
        component_ref = ET.SubElement(feature, "ComponentGroupRef")
        component_ref.set("Id", "ProductComponents")
        
        if self.config.create_start_menu_shortcut:
            shortcut_ref = ET.SubElement(feature, "ComponentRef")
            shortcut_ref.set("Id", "StartMenuShortcut")
        
        if self.config.create_desktop_shortcut:
            desktop_ref = ET.SubElement(feature, "ComponentRef")
            desktop_ref.set("Id", "DesktopShortcut")
        
        if self.config.add_to_path:
            path_ref = ET.SubElement(feature, "ComponentRef")
            path_ref.set("Id", "PathComponent")
        
        # UI configuration
        ui_ref = ET.SubElement(package, "ui:WixUI")
        ui_ref.set("Id", "WixUI_InstallDir")
        
        ui_property = ET.SubElement(package, "Property")
        ui_property.set("Id", "WIXUI_INSTALLDIR")
        ui_property.set("Value", "INSTALLFOLDER")
        
        # Format and return
        return self._format_xml(wix)
    
    def _add_directory_tree(self, parent: ET.Element, parts: List[str], final_id: str) -> None:
        """Recursively add directory elements."""
        if not parts:
            return
        
        dir_name = parts[0]
        dir_id = f"Dir_{dir_name.replace('-', '_').replace('.', '_')}"
        
        # Check if directory already exists
        existing = parent.find(f"./Directory[@Name='{dir_name}']")
        if existing is not None:
            dir_elem = existing
        else:
            dir_elem = ET.SubElement(parent, "Directory")
            dir_elem.set("Id", dir_id if len(parts) == 1 else f"{dir_id}_{id(parts)}")
            dir_elem.set("Name", dir_name)
        
        if len(parts) > 1:
            self._add_directory_tree(dir_elem, parts[1:], final_id)
        else:
            dir_elem.set("Id", final_id)
    
    def _add_shortcuts(self, package: ET.Element) -> None:
        """Add shortcuts to Start Menu and Desktop."""
        # Start Menu
        if self.config.create_start_menu_shortcut:
            start_dir = ET.SubElement(package, "StandardDirectory")
            start_dir.set("Id", "ProgramMenuFolder")
            
            start_component = ET.SubElement(start_dir, "Component")
            start_component.set("Id", "StartMenuShortcut")
            
            shortcut = ET.SubElement(start_component, "Shortcut")
            shortcut.set("Id", "StartMenuShortcut")
            shortcut.set("Name", self.config.product_name)
            shortcut.set("Target", "[INSTALLFOLDER]r-droid.exe")
            shortcut.set("WorkingDirectory", "INSTALLFOLDER")
            
            reg_key = ET.SubElement(start_component, "RegistryValue")
            reg_key.set("Root", "HKCU")
            reg_key.set("Key", f"Software\\{self.config.manufacturer}\\{self.config.product_name}")
            reg_key.set("Name", "StartMenuShortcut")
            reg_key.set("Type", "integer")
            reg_key.set("Value", "1")
            reg_key.set("KeyPath", "yes")
        
        # Desktop
        if self.config.create_desktop_shortcut:
            desktop_dir = ET.SubElement(package, "StandardDirectory")
            desktop_dir.set("Id", "DesktopFolder")
            
            desktop_component = ET.SubElement(desktop_dir, "Component")
            desktop_component.set("Id", "DesktopShortcut")
            
            shortcut = ET.SubElement(desktop_component, "Shortcut")
            shortcut.set("Id", "DesktopShortcut")
            shortcut.set("Name", self.config.product_name)
            shortcut.set("Target", "[INSTALLFOLDER]r-droid.exe")
            shortcut.set("WorkingDirectory", "INSTALLFOLDER")
            
            reg_key = ET.SubElement(desktop_component, "RegistryValue")
            reg_key.set("Root", "HKCU")
            reg_key.set("Key", f"Software\\{self.config.manufacturer}\\{self.config.product_name}")
            reg_key.set("Name", "DesktopShortcut")
            reg_key.set("Type", "integer")
            reg_key.set("Value", "1")
            reg_key.set("KeyPath", "yes")
    
    def _add_path_component(self, component_group: ET.Element) -> None:
        """Add component to add installation directory to PATH."""
        component = ET.SubElement(component_group, "Component")
        component.set("Id", "PathComponent")
        component.set("Guid", "*")
        
        env = ET.SubElement(component, "Environment")
        env.set("Id", "PATH")
        env.set("Name", "PATH")
        env.set("Value", "[INSTALLFOLDER]")
        env.set("Permanent", "no")
        env.set("Part", "last")
        env.set("Action", "set")
        env.set("System", "yes")
    
    def _format_xml(self, root: ET.Element) -> str:
        """Format XML with proper indentation."""
        rough_string = ET.tostring(root, encoding="unicode")
        reparsed = minidom.parseString(rough_string)
        return reparsed.toprettyxml(indent="  ")
    
    def build(self) -> Path:
        """Build the MSI installer."""
        print(f"\n{'='*60}")
        print(f"Building {self.config.product_name} v{self.config.version} Installer")
        print(f"{'='*60}\n")
        
        # Collect files
        self.collect_files()
        
        if not self.files:
            raise RuntimeError("No files collected for installer")
        
        # Create output directory
        self.config.output_dir.mkdir(parents=True, exist_ok=True)
        
        # Generate WiX source
        wxs_content = self.generate_wix_source()
        
        # Write WiX source file
        wxs_path = self.config.output_dir / "r-droid.wxs"
        wxs_path.write_text(wxs_content, encoding="utf-8")
        print(f"Generated: {wxs_path}")
        
        # Build MSI using WiX
        msi_path = self.config.output_dir / f"R-Droid-{self.config.version}-x64.msi"
        
        try:
            # Check for WiX
            wix_path = self._find_wix()
            
            # Run WiX build
            print("\nRunning WiX build...")
            result = subprocess.run(
                [wix_path, "build", "-o", str(msi_path), str(wxs_path)],
                capture_output=True,
                text=True
            )
            
            if result.returncode != 0:
                print(f"WiX build failed:\n{result.stderr}")
                raise RuntimeError("WiX build failed")
            
            print(f"\nSuccess! Installer created: {msi_path}")
            print(f"Size: {msi_path.stat().st_size / 1024 / 1024:.2f} MB")
            
            return msi_path
            
        except FileNotFoundError:
            print("\nWarning: WiX Toolset not found. WXS file generated but MSI not built.")
            print("Install WiX Toolset from: https://wixtoolset.org/")
            print(f"\nTo build manually, run:")
            print(f"  wix build -o {msi_path} {wxs_path}")
            return wxs_path
    
    def _find_wix(self) -> str:
        """Find WiX executable."""
        # Check common locations
        locations = [
            "wix",  # In PATH
            r"C:\Program Files (x86)\WiX Toolset v4\bin\wix.exe",
            r"C:\Program Files\WiX Toolset v4\bin\wix.exe",
        ]
        
        for loc in locations:
            try:
                result = subprocess.run(
                    [loc, "--version"],
                    capture_output=True,
                    text=True
                )
                if result.returncode == 0:
                    print(f"Found WiX: {loc}")
                    return loc
            except FileNotFoundError:
                continue
        
        raise FileNotFoundError("WiX Toolset not found")


def create_portable_zip(config: InstallerConfig) -> Path:
    """Create a portable ZIP distribution."""
    print("\nCreating portable ZIP distribution...")
    
    zip_name = f"R-Droid-{config.version}-portable-x64.zip"
    zip_path = config.output_dir / zip_name
    
    # Create temp directory for archive contents
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir) / "R-Droid"
        tmp_path.mkdir()
        
        # Copy executable
        exe_src = BUILD_DIR / "r-droid.exe"
        if exe_src.exists():
            shutil.copy2(exe_src, tmp_path / "r-droid.exe")
        
        # Copy DLLs
        for dll in BUILD_DIR.glob("*.dll"):
            shutil.copy2(dll, tmp_path / dll.name)
        
        # Copy resources
        for subdir in ["resources", "assets"]:
            src = PROJECT_ROOT / subdir
            if src.exists():
                shutil.copytree(src, tmp_path / subdir)
        
        # Create portable marker
        (tmp_path / ".portable").touch()
        
        # Create ZIP
        shutil.make_archive(
            str(zip_path.with_suffix("")),
            "zip",
            tmpdir
        )
    
    print(f"Created: {zip_path}")
    print(f"Size: {zip_path.stat().st_size / 1024 / 1024:.2f} MB")
    
    return zip_path


def create_checksums(files: List[Path]) -> Path:
    """Create SHA256 checksums file."""
    checksums = []
    
    for file_path in files:
        if file_path.exists():
            sha256 = hashlib.sha256()
            with open(file_path, "rb") as f:
                for chunk in iter(lambda: f.read(8192), b""):
                    sha256.update(chunk)
            checksums.append(f"{sha256.hexdigest()}  {file_path.name}")
    
    checksum_file = files[0].parent / "SHA256SUMS.txt"
    checksum_file.write_text("\n".join(checksums))
    
    print(f"\nCreated checksums: {checksum_file}")
    return checksum_file


def main():
    parser = argparse.ArgumentParser(
        description="Build R-Droid Windows MSI Installer"
    )
    parser.add_argument(
        "--version", "-v",
        default=DEFAULT_VERSION,
        help=f"Version string (default: {DEFAULT_VERSION})"
    )
    parser.add_argument(
        "--output", "-o",
        type=Path,
        default=DIST_DIR,
        help=f"Output directory (default: {DIST_DIR})"
    )
    parser.add_argument(
        "--portable",
        action="store_true",
        help="Also create portable ZIP distribution"
    )
    parser.add_argument(
        "--no-msi",
        action="store_true",
        help="Skip MSI creation (only create portable if --portable)"
    )
    
    args = parser.parse_args()
    
    config = InstallerConfig(
        version=args.version,
        output_dir=args.output
    )
    
    created_files = []
    
    # Build MSI
    if not args.no_msi:
        builder = InstallerBuilder(config)
        msi_path = builder.build()
        created_files.append(msi_path)
    
    # Create portable ZIP
    if args.portable:
        zip_path = create_portable_zip(config)
        created_files.append(zip_path)
    
    # Create checksums
    if created_files:
        create_checksums(created_files)
    
    print("\n" + "="*60)
    print("Build complete!")
    print("="*60)
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
