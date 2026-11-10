#!/usr/bin/env bash
set -e
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
APPIMAGE="$DIR/MineVault.AppImage"

if [ ! -f "$APPIMAGE" ]; then
    echo "Error: MineVault.AppImage not found in $DIR"
    exit 1
fi

# Test if execution from USB media is permitted or blocked by noexec mount
if ! [ -x "$APPIMAGE" ] || ! "$APPIMAGE" --version >/dev/null 2>&1; then
    echo "[Info] Direct execution from USB drive blocked (noexec mount detected)."
    echo "[Info] Staging to local user cache (~/.cache/minevault)..."
    CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/minevault"
    mkdir -p "$CACHE_DIR/bin"
    cp -p "$APPIMAGE" "$CACHE_DIR/MineVault.AppImage"
    chmod +x "$CACHE_DIR/MineVault.AppImage"
    if [ -f "$DIR/bin/restic" ]; then
        cp -p "$DIR/bin/restic" "$CACHE_DIR/bin/restic"
        chmod +x "$CACHE_DIR/bin/restic"
    fi
    exec "$CACHE_DIR/MineVault.AppImage" "$@"
else
    exec "$APPIMAGE" "$@"
fi
