#!/usr/bin/env bash
set -euo pipefail

# Script: download-restic.sh
# Purpose: Downloads official multi-platform restic v0.19.1 binaries and verifies SHA256 checksums.

RESTIC_VERSION="0.19.1"
OUTPUT_DIR="${1:-vendor/restic}"
mkdir -p "$OUTPUT_DIR"

verify_sha256() {
    local file="$1"
    local expected="$2"
    local actual
    if command -v sha256sum >/dev/null 2>&1; then
        echo "$expected  $file" | sha256sum -c -
    elif command -v shasum >/dev/null 2>&1; then
        echo "$expected  $file" | shasum -a 256 -c -
    else
        echo "Error: Neither sha256sum nor shasum is available for checksum verification" >&2
        return 1
    fi
}

echo "=== MineVault: Fetching Official Restic v${RESTIC_VERSION} Binaries ==="

# Checksums from official restic release v0.19.1:
# https://github.com/restic/restic/releases/download/v0.19.1/SHA256SUMS
DARWIN_ARM64_HASH="7be0a144ccc377880f294204aa271d76e4b79554b42a751151d425ce6ebac143"
DARWIN_AMD64_HASH="c38d579622cf602f665234c5a8c315030b6cf70656028fe6dc29a786b60e5f35"
LINUX_AMD64_HASH="f415415624dcc452f2a02b8c33641791a8c6d6d3b65bbb3543fcf9a25151585c"
WINDOWS_AMD64_HASH="da948ad707ed690426473aaba2046cd61f8f90f6f0e7dab6be0d5796531de67d"

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'restic_dl')"
trap 'rm -rf "$TMP_DIR"' EXIT

# 1. macOS ARM64
echo "--> Downloading macOS ARM64 (Apple Silicon)..."
MAC_ARM64_ARCHIVE="$TMP_DIR/restic_${RESTIC_VERSION}_darwin_arm64.bz2"
curl -sSL -o "$MAC_ARM64_ARCHIVE" "https://github.com/restic/restic/releases/download/v${RESTIC_VERSION}/restic_${RESTIC_VERSION}_darwin_arm64.bz2"
verify_sha256 "$MAC_ARM64_ARCHIVE" "$DARWIN_ARM64_HASH"
mkdir -p "$OUTPUT_DIR/macos-arm64"
bunzip2 -c "$MAC_ARM64_ARCHIVE" > "$OUTPUT_DIR/macos-arm64/restic"
chmod +x "$OUTPUT_DIR/macos-arm64/restic"

# 2. macOS x86_64
echo "--> Downloading macOS x86_64 (Intel)..."
MAC_X64_ARCHIVE="$TMP_DIR/restic_${RESTIC_VERSION}_darwin_amd64.bz2"
curl -sSL -o "$MAC_X64_ARCHIVE" "https://github.com/restic/restic/releases/download/v${RESTIC_VERSION}/restic_${RESTIC_VERSION}_darwin_amd64.bz2"
verify_sha256 "$MAC_X64_ARCHIVE" "$DARWIN_AMD64_HASH"
mkdir -p "$OUTPUT_DIR/macos-x64"
bunzip2 -c "$MAC_X64_ARCHIVE" > "$OUTPUT_DIR/macos-x64/restic"
chmod +x "$OUTPUT_DIR/macos-x64/restic"

# 3. Linux x86_64
echo "--> Downloading Linux x86_64..."
LINUX_X64_ARCHIVE="$TMP_DIR/restic_${RESTIC_VERSION}_linux_amd64.bz2"
curl -sSL -o "$LINUX_X64_ARCHIVE" "https://github.com/restic/restic/releases/download/v${RESTIC_VERSION}/restic_${RESTIC_VERSION}_linux_amd64.bz2"
verify_sha256 "$LINUX_X64_ARCHIVE" "$LINUX_AMD64_HASH"
mkdir -p "$OUTPUT_DIR/linux-x64"
bunzip2 -c "$LINUX_X64_ARCHIVE" > "$OUTPUT_DIR/linux-x64/restic"
chmod +x "$OUTPUT_DIR/linux-x64/restic"

# 4. Windows x86_64
echo "--> Downloading Windows x86_64..."
WIN_X64_ARCHIVE="$TMP_DIR/restic_${RESTIC_VERSION}_windows_amd64.zip"
curl -sSL -o "$WIN_X64_ARCHIVE" "https://github.com/restic/restic/releases/download/v${RESTIC_VERSION}/restic_${RESTIC_VERSION}_windows_amd64.zip"
verify_sha256 "$WIN_X64_ARCHIVE" "$WINDOWS_AMD64_HASH"
mkdir -p "$OUTPUT_DIR/windows-x64"
unzip -p "$WIN_X64_ARCHIVE" "restic_${RESTIC_VERSION}_windows_amd64.exe" > "$OUTPUT_DIR/windows-x64/restic.exe"

# 5. Fetch restic license notice
echo "--> Fetching restic LICENSE..."
mkdir -p "$OUTPUT_DIR/licenses"
curl -sSL -o "$OUTPUT_DIR/licenses/RESTIC-LICENSE.txt" "https://raw.githubusercontent.com/restic/restic/v${RESTIC_VERSION}/LICENSE"

echo "=== All official restic v${RESTIC_VERSION} binaries downloaded and verified! ==="
ls -lh "$OUTPUT_DIR"/*/*
