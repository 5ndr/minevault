@echo off
setlocal enabledelayedexpansion
set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%"

if exist "MineVault.exe" (
    start "" "MineVault.exe" %*
) else (
    echo [Error] MineVault.exe was not found in "%SCRIPT_DIR%".
    echo Please make sure all files were extracted properly.
    pause
)
