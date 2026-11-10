========================================================================
ResticDashboard - USB Portable Distribution Layout
========================================================================

This USB drive contains platform-specific builds of ResticDashboard,
allowing you to plug into macOS, Linux, or Windows and inspect, back up,
restore, or install unattended automatic backups.

DIRECTORY LAYOUT:
-----------------
USB_ROOT/
├── launch_macos.command       (Double-clickable launcher for macOS)
├── launch_linux.sh            (Safe launcher for Linux with noexec fallback)
├── launch_windows.bat         (Launcher for Windows)
├── macos/
│   ├── ResticDashboard.app    (macOS universal or Apple Silicon application)
│   └── bin/restic             (Bundled restic engine for macOS)
├── linux/
│   ├── resticdashboard        (Linux x86_64 executable)
│   └── bin/restic             (Bundled restic engine for Linux x86_64)
├── windows/
│   ├── resticdashboard.exe    (Windows x86_64 executable)
│   └── bin/restic.exe         (Bundled restic engine for Windows)
├── profiles/                  (Optional reusable profile templates)
└── docs/                      (User manuals, recovery procedures, and guides)

EXECUTION INSTRUCTIONS PER OS:
------------------------------

1. macOS:
   - Double-click `launch_macos.command` or `macos/ResticDashboard.app`.
   - Gatekeeper Note: macOS flags apps downloaded or run from removable drives.
     If prompted with "unidentified developer", right-click ResticDashboard.app
     and select "Open", or run in Terminal:
     xattr -cr "/Volumes/<USB_NAME>/macos/ResticDashboard.app"

2. Linux:
   - Run `bash launch_linux.sh` in terminal.
   - USB `noexec` Mount Workaround: Many Linux desktop environments mount USB drives
     with the `noexec` flag. `launch_linux.sh` automatically detects if execution
     is blocked on the USB drive, copies the self-contained binary to `~/.cache/resticdashboard`,
     and launches it without requiring root privileges.

3. Windows:
   - Double-click `launch_windows.bat` or `windows/resticdashboard.exe`.
   - Windows SmartScreen may show an unrecognized app warning. Click "More info" -> "Run anyway".

UNATTENDED BACKGROUND BACKUPS & USB INDEPENDENCE:
------------------------------------------------
When you enable "Install Automatic Backups" in the dashboard, the application:
1. Copies the runner and restic engine to persistent OS local paths
   (macOS: ~/Library/Application Support/resticdashboard/bin)
   (Linux: ~/.local/share/resticdashboard/bin)
   (Windows: %LOCALAPPDATA%\resticdashboard\bin)
2. Registers the native OS scheduler (launchd / systemd / Task Scheduler)
   using ABSOLUTE local paths.
3. Saves credentials in a restricted local file (chmod 0600).

Once installed, YOU CAN UNPLUG THIS USB DRIVE. Automatic backups will continue
running silently in the background according to your schedule.
