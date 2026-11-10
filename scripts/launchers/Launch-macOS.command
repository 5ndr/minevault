#!/usr/bin/env bash
set -e
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ARCH="$(uname -m)"

if [ "$ARCH" = "arm64" ]; then
    PLATFORM_DIR="$DIR/macOS-arm64"
else
    PLATFORM_DIR="$DIR/macOS-x64"
fi

# Automatically strip web download quarantine attribute so Gatekeeper opens without admin prompts
xattr -cr "$PLATFORM_DIR" 2>/dev/null || true

if [ -f "$PLATFORM_DIR/launch_macos.command" ]; then
    exec "$PLATFORM_DIR/launch_macos.command" "$@"
elif [ -f "$PLATFORM_DIR/MineVault.dmg" ]; then
    xattr -d com.apple.quarantine "$PLATFORM_DIR/MineVault.dmg" 2>/dev/null || true
    open "$PLATFORM_DIR/MineVault.dmg"
else
    echo "Error: MineVault distribution for $ARCH was not found in $PLATFORM_DIR."
    read -p "Press Enter to exit..."
fi
