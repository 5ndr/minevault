#!/usr/bin/env bash
set -e
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Strip quarantine attribute so Gatekeeper opens without admin prompts
xattr -cr "$DIR" 2>/dev/null || true

# 1. Look for existing .app bundle
if [ -d "$DIR/MineVault.app" ]; then
    xattr -cr "$DIR/MineVault.app" 2>/dev/null || true
    open "$DIR/MineVault.app"
    exit 0
fi

# 2. Look for .dmg to mount and open
DMG_FILE="$(find "$DIR" -maxdepth 1 -name "*.dmg" | head -n 1)"
if [ -n "$DMG_FILE" ] && [ -f "$DMG_FILE" ]; then
    xattr -d com.apple.quarantine "$DMG_FILE" 2>/dev/null || true
    echo "Opening MineVault disk image..."
    open "$DMG_FILE"
    exit 0
fi

# 3. Look for .app.tar.gz archive and stage to native APFS/HFS+ cache
TAR_FILE="$(find "$DIR" -maxdepth 1 -name "*MineVault*.tar.gz" | head -n 1)"
if [ -n "$TAR_FILE" ] && [ -f "$TAR_FILE" ]; then
    STAGE_DIR="$HOME/.cache/minevault_staging"
    mkdir -p "$STAGE_DIR"
    echo "Staging MineVault to local application cache ($STAGE_DIR)..."
    tar -xzf "$TAR_FILE" -C "$STAGE_DIR"
    xattr -cr "$STAGE_DIR/MineVault.app" 2>/dev/null || true
    open "$STAGE_DIR/MineVault.app"
    exit 0
fi

echo "Error: No MineVault application bundle, DMG, or archive found in $DIR"
read -p "Press Enter to exit..."
exit 1
