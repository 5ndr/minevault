@echo off
setlocal
cd /d "%~dp0"

:: Unblock downloaded files to remove Zone.Identifier (prevents SmartScreen warnings)
powershell -NoProfile -Command "Get-ChildItem -Path '%~dp0' -Recurse -File -ErrorAction SilentlyContinue | Unblock-File -ErrorAction SilentlyContinue" >nul 2>&1

if exist "Windows-x64\MineVault.exe" (
    start "" "Windows-x64\MineVault.exe" %*
) else if exist "Windows-x64\launch.bat" (
    call "Windows-x64\launch.bat" %*
) else (
    echo [Error] MineVault.exe not found in Windows-x64 folder.
    pause
)
