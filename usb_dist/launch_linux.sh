#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
EXE="$DIR/linux/resticdashboard"

# Test if execution from USB media is permitted or blocked by noexec mount
if ! [ -x "$EXE" ] || ! "$EXE" --version >/dev/null 2>&1; then
    echo "[Info] Direct execution from USB drive blocked (noexec mount detected)."
    echo "[Info] Launching via safe user cache fallback..."
    CACHE_DIR="$HOME/.cache/resticdashboard_launcher"
    mkdir -p "$CACHE_DIR"
    cp -r "$DIR/linux/"* "$CACHE_DIR/"
    chmod +x "$CACHE_DIR/resticdashboard"
    if [ -f "$CACHE_DIR/bin/restic" ]; then
        chmod +x "$CACHE_DIR/bin/restic"
    fi
    exec "$CACHE_DIR/resticdashboard" "$@"
else
    exec "$EXE" "$@"
fi
