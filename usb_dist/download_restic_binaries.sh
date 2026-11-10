#!/bin/bash
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
RESTIC_VERSION="0.19.1"

echo "=== Restic Multi-Platform USB Bundler ==="
echo "Target version: v$RESTIC_VERSION"
echo ""

mkdir -p "$DIR/macos/bin"
mkdir -p "$DIR/linux/bin"
mkdir -p "$DIR/windows/bin"

# 1. macOS binary
if [ -f "/opt/homebrew/bin/restic" ] && [ ! -f "$DIR/macos/bin/restic" ]; then
    echo "[1/3] Copying local macOS restic binary..."
    cp /opt/homebrew/bin/restic "$DIR/macos/bin/restic"
    chmod +x "$DIR/macos/bin/restic"
elif [ -f "$DIR/macos/bin/restic" ]; then
    echo "[1/3] macOS restic binary already present in $DIR/macos/bin/restic"
else
    echo "[1/3] Downloading macOS restic v$RESTIC_VERSION..."
    curl -sL "https://github.com/restic/restic/releases/download/v${RESTIC_VERSION}/restic_${RESTIC_VERSION}_darwin_arm64.bz2" | bunzip2 > "$DIR/macos/bin/restic"
    chmod +x "$DIR/macos/bin/restic"
fi

# 2. Linux x86_64 binary
if [ ! -f "$DIR/linux/bin/restic" ]; then
    echo "[2/3] Downloading Linux x86_64 restic v$RESTIC_VERSION..."
    curl -sL "https://github.com/restic/restic/releases/download/v${RESTIC_VERSION}/restic_${RESTIC_VERSION}_linux_amd64.bz2" | bunzip2 > "$DIR/linux/bin/restic"
    chmod +x "$DIR/linux/bin/restic"
else
    echo "[2/3] Linux restic binary already present in $DIR/linux/bin/restic"
fi

# 3. Windows x86_64 binary
if [ ! -f "$DIR/windows/bin/restic.exe" ]; then
    echo "[3/3] Downloading Windows x86_64 restic v$RESTIC_VERSION..."
    TMP_ZIP="/tmp/restic_win.zip"
    curl -sL -o "$TMP_ZIP" "https://github.com/restic/restic/releases/download/v${RESTIC_VERSION}/restic_${RESTIC_VERSION}_windows_amd64.zip"
    unzip -p "$TMP_ZIP" "restic_${RESTIC_VERSION}_windows_amd64.exe" > "$DIR/windows/bin/restic.exe"
    rm -f "$TMP_ZIP"
else
    echo "[3/3] Windows restic.exe binary already present in $DIR/windows/bin/restic.exe"
fi

echo ""
echo "=== Summary of Bundled Binaries ==="
ls -lh "$DIR/macos/bin/restic" 2>/dev/null || echo "macOS: missing"
ls -lh "$DIR/linux/bin/restic" 2>/dev/null || echo "Linux: missing"
ls -lh "$DIR/windows/bin/restic.exe" 2>/dev/null || echo "Windows: missing"
echo ""
echo "Your USB drive is now prepared with bundled restic engines for all platforms!"
