#!/usr/bin/env bash
set -euo pipefail

# Script: package-usb.sh
# Purpose: Assembles the multi-platform portable MineVault-USB distribution and individual platform archives.

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

BUILD_DIR="${1:-$REPO_ROOT/dist-build}"
OUTPUT_DIR="${2:-$REPO_ROOT/dist-release}"

VERSION="${MINEVAULT_VERSION:-$(node -p "require('$REPO_ROOT/src-tauri/tauri.conf.json').version" 2>/dev/null || echo "0.1.0")}"
COMMIT_SHA="${GITHUB_SHA:-$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo "unknown")}"
BUILD_DATE="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
RESTIC_VERSION="0.19.1"

echo "============================================================"
echo " Packaging MineVault USB Distribution v${VERSION}"
echo " Commit: ${COMMIT_SHA}"
echo " Timestamp: ${BUILD_DATE}"
echo "============================================================"

USB_ROOT="$OUTPUT_DIR/MineVault-USB"
rm -rf "$USB_ROOT"
mkdir -p "$USB_ROOT"
mkdir -p "$OUTPUT_DIR/archives"

# 1. Copy Documentation and Top-Level Files
echo "--> Copying documentation and licenses..."
cp "$REPO_ROOT/START-HERE.txt" "$USB_ROOT/"
cp "$REPO_ROOT/USB-SETUP.md" "$USB_ROOT/"

mkdir -p "$USB_ROOT/licenses"
cp "$REPO_ROOT/licenses/LICENSE" "$USB_ROOT/licenses/"
cp "$REPO_ROOT/licenses/RESTIC-LICENSE.txt" "$USB_ROOT/licenses/"
cp "$REPO_ROOT/licenses/THIRD-PARTY-LICENSES.md" "$USB_ROOT/licenses/"

# Copy Root Launchers
cp "$SCRIPT_DIR/launchers/Launch-Windows.bat" "$USB_ROOT/"
cp "$SCRIPT_DIR/launchers/Launch-macOS.command" "$USB_ROOT/"
cp "$SCRIPT_DIR/launchers/Launch-Linux.sh" "$USB_ROOT/"
chmod +x "$USB_ROOT/Launch-macOS.command" "$USB_ROOT/Launch-Linux.sh"

# 2. Windows x86_64
echo "--> Assembling Windows-x64 distribution..."
WIN_DIR="$USB_ROOT/Windows-x64"
mkdir -p "$WIN_DIR/bin"
cp "$SCRIPT_DIR/launchers/windows-launch.bat" "$WIN_DIR/launch.bat"

# Find Windows executable
if [ -f "$BUILD_DIR/windows-x64/MineVault.exe" ]; then
    cp "$BUILD_DIR/windows-x64/MineVault.exe" "$WIN_DIR/MineVault.exe"
elif [ -f "$BUILD_DIR/windows-x64/resticdashboard.exe" ]; then
    cp "$BUILD_DIR/windows-x64/resticdashboard.exe" "$WIN_DIR/MineVault.exe"
elif [ -f "$REPO_ROOT/src-tauri/target/release/MineVault.exe" ]; then
    cp "$REPO_ROOT/src-tauri/target/release/MineVault.exe" "$WIN_DIR/MineVault.exe"
elif [ -f "$REPO_ROOT/src-tauri/target/release/resticdashboard.exe" ]; then
    cp "$REPO_ROOT/src-tauri/target/release/resticdashboard.exe" "$WIN_DIR/MineVault.exe"
else
    echo "Warning: Windows MineVault.exe not found in $BUILD_DIR/windows-x64"
fi

# Windows restic sidecar
if [ -f "$BUILD_DIR/restic/windows-x64/restic.exe" ]; then
    cp "$BUILD_DIR/restic/windows-x64/restic.exe" "$WIN_DIR/bin/restic.exe"
fi

cat << 'EOF' > "$WIN_DIR/README.txt"
MineVault Portable for Windows (x86_64)
=======================================
Requirements:
- Windows 10 (1809+) or Windows 11 (64-bit).
- Microsoft Edge WebView2 runtime (pre-installed on Windows 10/11).
  If missing on a stripped or offline Windows install, install WebView2 from Microsoft:
  https://developer.microsoft.com/en-us/microsoft-edge/webview2/

Usage:
- Double-click "launch.bat" or "MineVault.exe".
- If Windows SmartScreen prompts "Windows protected your PC", click "More info" -> "Run anyway".
EOF

# 3. macOS ARM64 (Apple Silicon)
echo "--> Assembling macOS-arm64 distribution..."
MAC_ARM_DIR="$USB_ROOT/macOS-arm64"
mkdir -p "$MAC_ARM_DIR/bin"
cp "$SCRIPT_DIR/launchers/macos-launch.command" "$MAC_ARM_DIR/launch_macos.command"
chmod +x "$MAC_ARM_DIR/launch_macos.command"

# Find macOS ARM64 DMG / app bundle
if [ -f "$BUILD_DIR/macos-arm64/MineVault.dmg" ]; then
    cp "$BUILD_DIR/macos-arm64/MineVault.dmg" "$MAC_ARM_DIR/MineVault.dmg"
elif [ -f "$BUILD_DIR/macos-arm64/MineVault_${VERSION}_aarch64.dmg" ]; then
    cp "$BUILD_DIR/macos-arm64/MineVault_${VERSION}_aarch64.dmg" "$MAC_ARM_DIR/MineVault.dmg"
fi

if [ -f "$BUILD_DIR/macos-arm64/MineVault.app.tar.gz" ]; then
    cp "$BUILD_DIR/macos-arm64/MineVault.app.tar.gz" "$MAC_ARM_DIR/MineVault.app.tar.gz"
fi

if [ -d "$BUILD_DIR/macos-arm64/MineVault.app" ]; then
    tar -czf "$MAC_ARM_DIR/MineVault.app.tar.gz" -C "$BUILD_DIR/macos-arm64" "MineVault.app"
fi

# macOS ARM64 restic sidecar
if [ -f "$BUILD_DIR/restic/macos-arm64/restic" ]; then
    cp "$BUILD_DIR/restic/macos-arm64/restic" "$MAC_ARM_DIR/bin/restic"
    chmod +x "$MAC_ARM_DIR/bin/restic"
fi

cat << 'EOF' > "$MAC_ARM_DIR/README.txt"
MineVault Portable for macOS Apple Silicon (M1/M2/M3/M4)
=======================================================
Requirements:
- macOS 11.0 (Big Sur) or later on Apple Silicon.

Gatekeeper & Security:
- Because this community release is distributed directly without an Apple Developer ID notarization ticket:
  1. Open "MineVault.dmg" or double-click "launch_macos.command".
  2. If macOS warns "developer cannot be verified":
     Right-click (Control-click) "MineVault.app" -> Open -> click "Open".
  3. Alternatively, in Terminal remove quarantine:
     xattr -d com.apple.quarantine /path/to/MineVault.app
  4. Note: NEVER disable Gatekeeper globally.
EOF

# 4. macOS x86_64 (Intel)
echo "--> Assembling macOS-x64 distribution..."
MAC_X64_DIR="$USB_ROOT/macOS-x64"
mkdir -p "$MAC_X64_DIR/bin"
cp "$SCRIPT_DIR/launchers/macos-launch.command" "$MAC_X64_DIR/launch_macos.command"
chmod +x "$MAC_X64_DIR/launch_macos.command"

# Find macOS x86_64 DMG / app bundle
if [ -f "$BUILD_DIR/macos-x64/MineVault.dmg" ]; then
    cp "$BUILD_DIR/macos-x64/MineVault.dmg" "$MAC_X64_DIR/MineVault.dmg"
elif [ -f "$BUILD_DIR/macos-x64/MineVault_${VERSION}_x64.dmg" ]; then
    cp "$BUILD_DIR/macos-x64/MineVault_${VERSION}_x64.dmg" "$MAC_X64_DIR/MineVault.dmg"
fi

if [ -f "$BUILD_DIR/macos-x64/MineVault.app.tar.gz" ]; then
    cp "$BUILD_DIR/macos-x64/MineVault.app.tar.gz" "$MAC_X64_DIR/MineVault.app.tar.gz"
fi

if [ -d "$BUILD_DIR/macos-x64/MineVault.app" ]; then
    tar -czf "$MAC_X64_DIR/MineVault.app.tar.gz" -C "$BUILD_DIR/macos-x64" "MineVault.app"
fi

# macOS x86_64 restic sidecar
if [ -f "$BUILD_DIR/restic/macos-x64/restic" ]; then
    cp "$BUILD_DIR/restic/macos-x64/restic" "$MAC_X64_DIR/bin/restic"
    chmod +x "$MAC_X64_DIR/bin/restic"
fi

cat << 'EOF' > "$MAC_X64_DIR/README.txt"
MineVault Portable for macOS Intel (x86_64)
===========================================
Requirements:
- macOS 10.15 (Catalina) or later on 64-bit Intel Macs.

Gatekeeper & Security:
- Right-click "MineVault.app" -> Open -> click "Open".
EOF

# 5. Linux x86_64
echo "--> Assembling Linux-x64 distribution..."
LINUX_DIR="$USB_ROOT/Linux-x64"
mkdir -p "$LINUX_DIR/bin"
cp "$SCRIPT_DIR/launchers/linux-launch.sh" "$LINUX_DIR/launch.sh"
chmod +x "$LINUX_DIR/launch.sh"

# Find Linux AppImage
if [ -f "$BUILD_DIR/linux-x64/MineVault.AppImage" ]; then
    cp "$BUILD_DIR/linux-x64/MineVault.AppImage" "$LINUX_DIR/MineVault.AppImage"
    chmod +x "$LINUX_DIR/MineVault.AppImage"
elif compgen -G "$BUILD_DIR/linux-x64/*.AppImage" > /dev/null; then
    FIRST_APPIMAGE="$(compgen -G "$BUILD_DIR/linux-x64/*.AppImage" | head -n 1)"
    cp "$FIRST_APPIMAGE" "$LINUX_DIR/MineVault.AppImage"
    chmod +x "$LINUX_DIR/MineVault.AppImage"
else
    echo "Warning: Linux MineVault.AppImage not found in $BUILD_DIR/linux-x64"
fi

# Linux restic sidecar
if [ -f "$BUILD_DIR/restic/linux-x64/restic" ]; then
    cp "$BUILD_DIR/restic/linux-x64/restic" "$LINUX_DIR/bin/restic"
    chmod +x "$LINUX_DIR/bin/restic"
fi

cat << 'EOF' > "$LINUX_DIR/README.txt"
MineVault Portable for Linux (x86_64)
====================================
Compatibility:
- Linux distributions with glibc 2.35+ (Ubuntu 22.04+, Debian 12+, Fedora 37+, Arch Linux).
- WebKit2GTK 4.1 (libwebkit2gtk-4.1-0).

Launch:
- Run "./launch.sh" or "./MineVault.AppImage".
- If the USB drive is mounted with "noexec", "./launch.sh" automatically stages
  the AppImage to ~/.cache/minevault and executes it safely without sudo.
EOF

# 6. Generate build-manifest.json
echo "--> Generating build-manifest.json..."
python3 -c "
import json, os, datetime

manifest = {
    'name': 'MineVault',
    'version': '${VERSION}',
    'commit': '${COMMIT_SHA}',
    'build_timestamp': '${BUILD_DATE}',
    'restic': {
        'version': '${RESTIC_VERSION}',
        'darwin_arm64_sha256': '7be0a144ccc377880f294204aa271d76e4b79554b42a751151d425ce6ebac143',
        'darwin_amd64_sha256': 'c38d579622cf602f665234c5a8c315030b6cf70656028fe6dc29a786b60e5f35',
        'linux_amd64_sha256':  'f415415624dcc452f2a02b8c33641791a8c6d6d3b65bbb3543fcf9a25151585c',
        'windows_amd64_sha256':'da948ad707ed690426473aaba2046cd61f8f90f6f0e7dab6be0d5796531de67d'
    },
    'platforms': {
        'windows-x64': {'supported': True, 'runner': 'windows-latest', 'format': 'portable-dir'},
        'macos-arm64': {'supported': True, 'runner': 'macos-14', 'format': 'dmg/tar.gz'},
        'macos-x64':   {'supported': True, 'runner': 'macos-13', 'format': 'dmg/tar.gz'},
        'linux-x64':   {'supported': True, 'runner': 'ubuntu-22.04', 'format': 'AppImage'},
        'linux-arm64': {'supported': False, 'status': 'planned', 'notes': 'Awaiting standard GitHub-hosted Linux ARM64 runner availability'}
    }
}

with open('${USB_ROOT}/build-manifest.json', 'w') as f:
    json.dump(manifest, f, indent=2)
"

# 7. Generate SHA256SUMS for the USB distribution
# IMPORTANT: SHA256SUMS must NOT contain an entry for SHA256SUMS itself (avoids self-reference)
echo "--> Generating SHA256SUMS for MineVault-USB..."
(
    cd "$USB_ROOT"
    rm -f SHA256SUMS
    if command -v sha256sum >/dev/null 2>&1; then
        find . -type f ! -name "SHA256SUMS" ! -name ".*" | LC_ALL=C sort | xargs sha256sum > SHA256SUMS
    elif command -v shasum >/dev/null 2>&1; then
        find . -type f ! -name "SHA256SUMS" ! -name ".*" | LC_ALL=C sort | xargs shasum -a 256 > SHA256SUMS
    fi
)

# 8. Create compressed release archives
echo "--> Packaging archives in $OUTPUT_DIR/archives..."

# 8.1 Combined USB Kit
(
    cd "$OUTPUT_DIR"
    rm -f "archives/MineVault-USB-v${VERSION}.zip"
    zip -q -r -9 "archives/MineVault-USB-v${VERSION}.zip" "MineVault-USB"
)

# 8.2 Windows-x64 Archive
if [ -d "$USB_ROOT/Windows-x64" ]; then
    (
        cd "$USB_ROOT"
        zip -q -r -9 "$OUTPUT_DIR/archives/MineVault-v${VERSION}-windows-x64.zip" "Windows-x64" "START-HERE.txt" "USB-SETUP.md" "licenses" "build-manifest.json"
    )
fi

# 8.3 macOS-arm64 Archive
if [ -f "$USB_ROOT/macOS-arm64/MineVault.dmg" ]; then
    cp "$USB_ROOT/macOS-arm64/MineVault.dmg" "$OUTPUT_DIR/archives/MineVault-v${VERSION}-macos-arm64.dmg"
elif [ -f "$USB_ROOT/macOS-arm64/MineVault.app.tar.gz" ]; then
    cp "$USB_ROOT/macOS-arm64/MineVault.app.tar.gz" "$OUTPUT_DIR/archives/MineVault-v${VERSION}-macos-arm64.app.tar.gz"
fi

# 8.4 macOS-x64 Archive
if [ -f "$USB_ROOT/macOS-x64/MineVault.dmg" ]; then
    cp "$USB_ROOT/macOS-x64/MineVault.dmg" "$OUTPUT_DIR/archives/MineVault-v${VERSION}-macos-x64.dmg"
elif [ -f "$USB_ROOT/macOS-x64/MineVault.app.tar.gz" ]; then
    cp "$USB_ROOT/macOS-x64/MineVault.app.tar.gz" "$OUTPUT_DIR/archives/MineVault-v${VERSION}-macos-x64.app.tar.gz"
fi

# 8.5 Linux-x64 Archive
if [ -f "$USB_ROOT/Linux-x64/MineVault.AppImage" ]; then
    cp "$USB_ROOT/Linux-x64/MineVault.AppImage" "$OUTPUT_DIR/archives/MineVault-v${VERSION}-linux-x64.AppImage"
fi

# 8.6 Generate Top-Level Release SHA256SUMS covering all deliverables
echo "--> Generating top-level SHA256SUMS for release assets..."
(
    cd "$OUTPUT_DIR/archives"
    rm -f SHA256SUMS
    if command -v sha256sum >/dev/null 2>&1; then
        find . -maxdepth 1 -type f ! -name "SHA256SUMS" ! -name ".*" | sed 's|^\./||' | LC_ALL=C sort | xargs sha256sum > SHA256SUMS
    elif command -v shasum >/dev/null 2>&1; then
        find . -maxdepth 1 -type f ! -name "SHA256SUMS" ! -name ".*" | sed 's|^\./||' | LC_ALL=C sort | xargs shasum -a 256 > SHA256SUMS
    fi
)

echo "============================================================"
echo " MineVault USB Packaging Complete!"
echo " Distribution Folder: $USB_ROOT"
echo " Release Archives:    $OUTPUT_DIR/archives"
echo "============================================================"
