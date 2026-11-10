@echo off
set "DIR=%~dp0"
if exist "%DIR%windows\resticdashboard.exe" (
    start "" "%DIR%windows\resticdashboard.exe"
) else (
    echo [Error] resticdashboard.exe was not found in %DIR%windows
    pause
)
