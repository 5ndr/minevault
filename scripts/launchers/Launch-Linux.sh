#!/usr/bin/env bash
set -e
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

run_app() {
    local bin="$1"
    shift
    if "$bin" "$@" 2>/dev/null; then
        return 0
    else
        # If FUSE is absent (e.g. Ubuntu 24.04), fallback to built-in userspace extraction mode without root
        exec "$bin" --appimage-extract-and-run "$@"
    fi
}

if [ -f "$DIR/Linux-x64/launch.sh" ]; then
    exec "$DIR/Linux-x64/launch.sh" "$@"
elif [ -f "$DIR/Linux-x64/MineVault.AppImage" ]; then
    APPIMAGE="$DIR/Linux-x64/MineVault.AppImage"
    if ! [ -x "$APPIMAGE" ] || ! "$APPIMAGE" --version >/dev/null 2>&1; then
        echo "[Info] Direct execution from USB drive blocked (noexec mount detected)."
        echo "[Info] Staging to local user cache..."
        CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/minevault"
        mkdir -p "$CACHE_DIR/bin"
        cp -p "$APPIMAGE" "$CACHE_DIR/MineVault.AppImage"
        chmod +x "$CACHE_DIR/MineVault.AppImage"
        if [ -f "$DIR/Linux-x64/bin/restic" ]; then
            cp -p "$DIR/Linux-x64/bin/restic" "$CACHE_DIR/bin/restic"
            chmod +x "$CACHE_DIR/bin/restic"
        fi
        run_app "$CACHE_DIR/MineVault.AppImage" "$@"
    else
        run_app "$APPIMAGE" "$@"
    fi
else
    echo "Error: MineVault.AppImage not found in $DIR/Linux-x64."
    exit 1
fi
