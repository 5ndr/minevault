@echo off
setlocal
cd /d "%~dp0"

:: Unblock downloaded files to remove Zone.Identifier (prevents SmartScreen warnings)
powershell -NoProfile -Command "Get-ChildItem -Path '%~dp0' -Recurse -File -ErrorAction SilentlyContinue | Unblock-File -ErrorAction SilentlyContinue" >nul 2>&1

if exist "Windows-x64\MineVault.exe" (
    start "" "Windows-x64\MineVault.exe" %*
) else if exist "Windows-x64\launch.bat" (
    call "Windows-x64\launch.bat" %*
) else if exist "src-tauri\target\release\MineVault.exe" (
    start "" "src-tauri\target\release\MineVault.exe" %*
) else if exist "src-tauri\target\release\resticdashboard.exe" (
    start "" "src-tauri\target\release\resticdashboard.exe" %*
) else if exist "package.json" (
    echo [MineVault] Standalone executable not found. Starting development server on host PC...
    call npm run tauri dev
) else (
    echo [Error] MineVault.exe not found.
    echo Please make sure all files were extracted properly.
    pause
)
