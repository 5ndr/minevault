@echo off
setlocal enabledelayedexpansion
set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%"

:: Unblock downloaded files to remove Zone.Identifier (prevents SmartScreen warnings)
powershell -NoProfile -Command "Get-ChildItem -Path '%SCRIPT_DIR%' -Recurse -File -ErrorAction SilentlyContinue | Unblock-File -ErrorAction SilentlyContinue" >nul 2>&1

if exist "MineVault.exe" (
    start "" "MineVault.exe" %*
) else (
    echo [Error] MineVault.exe was not found in "%SCRIPT_DIR%".
    echo Please make sure all files were extracted properly.
    pause
)
