#!/usr/bin/env bash
set -euo pipefail

# Script: verify-package.sh
# Purpose: Rigorously verifies the MineVault USB distribution for completeness, architecture accuracy,
#          checksum integrity, freedom from developer paths or leaked secrets, and spaces-in-path safety.

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

TARGET_DIR="${1:-$REPO_ROOT/dist-release/MineVault-USB}"

echo "============================================================"
echo " MineVault Package Verification"
echo " Target Directory: $TARGET_DIR"
echo "============================================================"

if [ ! -d "$TARGET_DIR" ]; then
    echo "Error: Target directory $TARGET_DIR does not exist!" >&2
    exit 1
fi

ERRORS=0
log_err() {
    echo "❌ [FAIL] $1" >&2
    ERRORS=$((ERRORS + 1))
}

log_ok() {
    echo "✅ [PASS] $1"
}

# 1. Verify Core Documentation and Metadata
echo ""
echo "--- 1. Core Documentation & Metadata ---"
for file in "START-HERE.txt" "USB-SETUP.md" "SHA256SUMS" "build-manifest.json" "Launch-Windows.bat" "Launch-macOS.command" "Launch-Linux.sh"; do
    if [ -f "$TARGET_DIR/$file" ] && [ -s "$TARGET_DIR/$file" ]; then
        log_ok "Found non-empty $file"
    else
        log_err "Missing or empty $file"
    fi
done

for lic in "licenses/LICENSE" "licenses/RESTIC-LICENSE.txt" "licenses/THIRD-PARTY-LICENSES.md"; do
    if [ -f "$TARGET_DIR/$lic" ] && [ -s "$TARGET_DIR/$lic" ]; then
        log_ok "Found non-empty $lic"
    else
        log_err "Missing or empty $lic"
    fi
done

# 2. Verify Platform Directories & Required Files
echo ""
echo "--- 2. Platform Deliverables ---"

# 2.1 Windows x64
if [ -d "$TARGET_DIR/Windows-x64" ]; then
    log_ok "Found Windows-x64 directory"
    [ -f "$TARGET_DIR/Windows-x64/MineVault.exe" ] && [ -s "$TARGET_DIR/Windows-x64/MineVault.exe" ] && log_ok "Found Windows-x64/MineVault.exe" || log_err "Missing or empty Windows-x64/MineVault.exe"
    [ -f "$TARGET_DIR/Windows-x64/bin/restic.exe" ] && [ -s "$TARGET_DIR/Windows-x64/bin/restic.exe" ] && log_ok "Found Windows-x64/bin/restic.exe" || log_err "Missing or empty Windows-x64/bin/restic.exe"
    [ -f "$TARGET_DIR/Windows-x64/launch.bat" ] && [ -s "$TARGET_DIR/Windows-x64/launch.bat" ] && log_ok "Found Windows-x64/launch.bat" || log_err "Missing or empty Windows-x64/launch.bat"
    [ -f "$TARGET_DIR/Windows-x64/README.txt" ] && log_ok "Found Windows-x64/README.txt" || log_err "Missing Windows-x64/README.txt"
else
    log_err "Missing Windows-x64 directory"
fi

# 2.2 macOS ARM64
if [ -d "$TARGET_DIR/macOS-arm64" ]; then
    log_ok "Found macOS-arm64 directory"
    if [ -f "$TARGET_DIR/macOS-arm64/MineVault.dmg" ] || [ -f "$TARGET_DIR/macOS-arm64/MineVault.app.tar.gz" ]; then
        log_ok "Found macOS-arm64 distribution archive or disk image"
    else
        log_err "Missing macOS-arm64 DMG or app tar.gz"
    fi
    [ -f "$TARGET_DIR/macOS-arm64/bin/restic" ] && [ -s "$TARGET_DIR/macOS-arm64/bin/restic" ] && log_ok "Found macOS-arm64/bin/restic" || log_err "Missing or empty macOS-arm64/bin/restic"
    [ -f "$TARGET_DIR/macOS-arm64/launch_macos.command" ] && log_ok "Found macOS-arm64/launch_macos.command" || log_err "Missing macOS-arm64/launch_macos.command"
    [ -f "$TARGET_DIR/macOS-arm64/README.txt" ] && log_ok "Found macOS-arm64/README.txt" || log_err "Missing macOS-arm64/README.txt"
else
    log_err "Missing macOS-arm64 directory"
fi

# 2.3 macOS x64 (Intel)
if [ -d "$TARGET_DIR/macOS-x64" ]; then
    log_ok "Found macOS-x64 directory"
    if [ -f "$TARGET_DIR/macOS-x64/MineVault.dmg" ] || [ -f "$TARGET_DIR/macOS-x64/MineVault.app.tar.gz" ]; then
        log_ok "Found macOS-x64 distribution archive or disk image"
    else
        log_err "Missing macOS-x64 DMG or app tar.gz"
    fi
    [ -f "$TARGET_DIR/macOS-x64/bin/restic" ] && [ -s "$TARGET_DIR/macOS-x64/bin/restic" ] && log_ok "Found macOS-x64/bin/restic" || log_err "Missing or empty macOS-x64/bin/restic"
    [ -f "$TARGET_DIR/macOS-x64/launch_macos.command" ] && log_ok "Found macOS-x64/launch_macos.command" || log_err "Missing macOS-x64/launch_macos.command"
    [ -f "$TARGET_DIR/macOS-x64/README.txt" ] && log_ok "Found macOS-x64/README.txt" || log_err "Missing macOS-x64/README.txt"
else
    log_err "Missing macOS-x64 directory"
fi

# 2.4 Linux x64
if [ -d "$TARGET_DIR/Linux-x64" ]; then
    log_ok "Found Linux-x64 directory"
    [ -f "$TARGET_DIR/Linux-x64/MineVault.AppImage" ] && [ -s "$TARGET_DIR/Linux-x64/MineVault.AppImage" ] && log_ok "Found Linux-x64/MineVault.AppImage" || log_err "Missing or empty Linux-x64/MineVault.AppImage"
    [ -f "$TARGET_DIR/Linux-x64/bin/restic" ] && [ -s "$TARGET_DIR/Linux-x64/bin/restic" ] && log_ok "Found Linux-x64/bin/restic" || log_err "Missing or empty Linux-x64/bin/restic"
    [ -f "$TARGET_DIR/Linux-x64/launch.sh" ] && log_ok "Found Linux-x64/launch.sh" || log_err "Missing Linux-x64/launch.sh"
    [ -f "$TARGET_DIR/Linux-x64/README.txt" ] && log_ok "Found Linux-x64/README.txt" || log_err "Missing Linux-x64/README.txt"
else
    log_err "Missing Linux-x64 directory"
fi

# 3. Architecture Validation
echo ""
echo "--- 3. Architecture Verification ---"
if command -v file >/dev/null 2>&1; then
    if [ -f "$TARGET_DIR/Windows-x64/bin/restic.exe" ]; then
        F_OUT="$(file "$TARGET_DIR/Windows-x64/bin/restic.exe")"
        if echo "$F_OUT" | grep -qi "PE32+\|x86-64\|MS Windows"; then
            log_ok "Windows restic.exe architecture verified: $F_OUT"
        else
            log_err "Windows restic.exe architecture mismatch: $F_OUT"
        fi
    fi

    if [ -f "$TARGET_DIR/macOS-arm64/bin/restic" ]; then
        F_OUT="$(file "$TARGET_DIR/macOS-arm64/bin/restic")"
        if echo "$F_OUT" | grep -qi "arm64"; then
            log_ok "macOS-arm64 restic architecture verified: $F_OUT"
        else
            log_err "macOS-arm64 restic architecture mismatch: $F_OUT"
        fi
    fi

    if [ -f "$TARGET_DIR/macOS-x64/bin/restic" ]; then
        F_OUT="$(file "$TARGET_DIR/macOS-x64/bin/restic")"
        if echo "$F_OUT" | grep -qi "x86_64"; then
            log_ok "macOS-x64 restic architecture verified: $F_OUT"
        else
            log_err "macOS-x64 restic architecture mismatch: $F_OUT"
        fi
    fi

    if [ -f "$TARGET_DIR/Linux-x64/bin/restic" ]; then
        F_OUT="$(file "$TARGET_DIR/Linux-x64/bin/restic")"
        if echo "$F_OUT" | grep -qi "ELF 64-bit.*x86-64"; then
            log_ok "Linux-x64 restic architecture verified: $F_OUT"
        else
            log_err "Linux-x64 restic architecture mismatch: $F_OUT"
        fi
    fi
else
    echo "Notice: 'file' utility not present, skipping deep binary header inspection"
fi

# 4. Checksums & Self-Reference Check
echo ""
echo "--- 4. Checksum Verification & Self-Reference Audit ---"
if [ -f "$TARGET_DIR/SHA256SUMS" ]; then
    # Check self-reference
    if grep -q "SHA256SUMS" "$TARGET_DIR/SHA256SUMS"; then
        log_err "SHA256SUMS contains a self-referencing entry for SHA256SUMS!"
    else
        log_ok "SHA256SUMS contains NO self-referencing entry"
    fi

    # Verify checksums
    echo "--> Running checksum verification of all files against SHA256SUMS..."
    (
        cd "$TARGET_DIR"
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum -c SHA256SUMS --status && log_ok "All files match SHA256SUMS" || log_err "One or more files failed SHA256SUMS verification"
        elif command -v shasum >/dev/null 2>&1; then
            shasum -a 256 -c SHA256SUMS --status && log_ok "All files match SHA256SUMS" || log_err "One or more files failed SHA256SUMS verification"
        fi
    )
fi

# 5. Leaked Secrets, Keys, and Developer Paths Audit
echo ""
echo "--- 5. Security & Privacy Audit ---"

# Check for sensitive extensions
SENSITIVE_FILES="$(find "$TARGET_DIR" -type f \( -name "*.env" -o -name "*.enc" -o -name "*.pem" -o -name "*.key" -o -name "*.p12" -o -name "minevault_keys.enc" -o -name "*.log" \) || true)"
if [ -n "$SENSITIVE_FILES" ]; then
    log_err "Found sensitive / secret files in distribution package:\n$SENSITIVE_FILES"
else
    log_ok "No .env, .enc, keys, certificates, or logs found in distribution"
fi

# Check for git metadata
if [ -d "$TARGET_DIR/.git" ] || find "$TARGET_DIR" -name ".git" | grep -q .; then
    log_err "Found .git metadata in distribution package!"
else
    log_ok "No .git directory or VCS metadata in distribution"
fi

# Check for developer home paths in packaged text / json
DEV_PATH_MATCHES="$(grep -rnE "(/Users/[a-zA-Z0-9_-]+|/home/(runner|[a-zA-Z0-9_-]+)/work)" "$TARGET_DIR" --include="*.json" --include="*.txt" --include="*.md" --include="*.bat" --include="*.sh" --include="*.command" 2>/dev/null || true)"
if [ -n "$DEV_PATH_MATCHES" ]; then
    log_err "Distribution contains developer absolute paths:\n$DEV_PATH_MATCHES"
else
    log_ok "No developer absolute paths found in packaged text/config files"
fi

# 6. Portability from Paths with Spaces & Non-ASCII Characters
echo ""
echo "--- 6. Paths with Spaces & Non-ASCII Validation ---"
TEST_SPACE_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'mv_space_test')/MineVault Test Space & Ûnicøde"
mkdir -p "$TEST_SPACE_DIR"

cp -R "$TARGET_DIR/"* "$TEST_SPACE_DIR/"

# Verify launcher scripts run/parse properly in path with spaces
if [ -f "$TEST_SPACE_DIR/Launch-Linux.sh" ]; then
    bash -n "$TEST_SPACE_DIR/Launch-Linux.sh" && log_ok "Launch-Linux.sh syntax valid in path with spaces" || log_err "Launch-Linux.sh failed syntax check"
fi
if [ -f "$TEST_SPACE_DIR/Launch-macOS.command" ]; then
    bash -n "$TEST_SPACE_DIR/Launch-macOS.command" && log_ok "Launch-macOS.command syntax valid in path with spaces" || log_err "Launch-macOS.command failed syntax check"
fi
if [ -f "$TEST_SPACE_DIR/Linux-x64/launch.sh" ]; then
    bash -n "$TEST_SPACE_DIR/Linux-x64/launch.sh" && log_ok "Linux-x64/launch.sh syntax valid in path with spaces" || log_err "Linux-x64/launch.sh failed syntax check"
fi
if [ -f "$TEST_SPACE_DIR/macOS-arm64/launch_macos.command" ]; then
    bash -n "$TEST_SPACE_DIR/macOS-arm64/launch_macos.command" && log_ok "macOS-arm64/launch_macos.command syntax valid in path with spaces" || log_err "macOS-arm64/launch_macos.command failed syntax check"
fi

rm -rf "$(dirname "$TEST_SPACE_DIR")"

echo ""
echo "============================================================"
if [ $ERRORS -eq 0 ]; then
    echo "🎉 ALL PACKAGE VERIFICATION CHECKS PASSED WITH 0 ERRORS!"
    echo "============================================================"
    exit 0
else
    echo "💥 PACKAGE VERIFICATION FAILED WITH $ERRORS ERRORS!"
    echo "============================================================"
    exit 1
fi
