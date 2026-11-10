# MineVault ⛏️📦

**MineVault** is a portable, cross-platform Minecraft backup management and disaster recovery system built with **Tauri 2**, **Rust**, and **React + TypeScript**.

Carried on a USB flash drive, MineVault lets you plug into any computer running Minecraft, inspect existing Backblaze B2 restic repositories, execute safe manual backups and restorations, and optionally install persistent unattended background backup schedules that run locally on the host machine even after the USB drive is unplugged.

---

## 🌟 Key Features

### 1. 🛡️ Live Minecraft Process Guard (World Corruption Defense)
- **Automatic Pre-Flight Check**: Detects if Minecraft launchers or active game instances are running before initiating any backup to prevent chunk tearing and world save corruption.
- **Supported Launchers**: Prism Launcher, Modrinth App, CurseForge, ATLauncher, MultiMC, Official Minecraft (Vanilla), Lunar Client, and Badlion Client.
- **Supported Game Engines**: Vanilla Java, Fabric, Forge, NeoForge, Quilt (`net.fabricmc.*`, `net.neoforged.*`, `org.quiltmc.*`, `net.minecraftforge.*`, `net.minecraft.*`), and Minecraft Bedrock Edition (`Minecraft.Windows.exe`, `bedrock_server`).
- **One-Click Graceful Shutdown**: Close games cleanly via `SIGTERM` / `WM_CLOSE` so chunks save before backup.
- **Unattended Safety Delay**: Scheduled background backup tasks automatically postpone if the game is actively running, logging a safe skip record (`status: "postponed"`).

### 2. ☁️ Direct Backblaze B2 Native API & Obsolete Version Purge
- **Physical vs Deduplicated Storage**: Measures true physical Backblaze B2 bucket storage alongside restic deduplicated chunk bytes.
- **Automatic Lifecycle Management**: Automatically configures Backblaze 1-day auto-delete lifecycle rules (`daysFromHidingToDeleting: 1`) to eliminate perpetual duplication.
- **Mathematical Active Whitelist Purge**:
  - Whitelists all active, live files via `b2_list_file_names`.
  - Scans `b2_list_file_versions` and permanently purges only dead hide markers and superseded rewritten versions.
  - **Zero Risk**: Active restic blobs and the `config` anchor file are mathematically protected and can never be deleted.
- **~Rough Cost Estimator**: Calculates real-time estimated monthly/yearly Backblaze B2 storage costs (10 GB free tier + $0.006/GB/mo).

### 3. 🔐 AES-256-GCM Encrypted USB Key Vault
- **Encrypted Portable Vault**: Store Backblaze B2 Master/Management Key IDs, Secret Application Keys, and repository passwords encrypted directly on the USB drive.
- **Hardware-Grade Cryptography**: Secured using **PBKDF2-HMAC-SHA256** (100,000 rounds) key derivation and **AES-256-GCM** authenticated encryption with random salts and nonces.
- **Optional Passphrase**: Protect keys with an optional master passphrase or device encryption seed.

### 4. 🗂️ Minecraft Launcher & Scope Selector
- **Automatic Launcher Detection**:
  - **macOS**: `~/Library/Application Support/PrismLauncher`, `~/Library/Application Support/minecraft`
  - **Linux**: `~/.local/share/PrismLauncher`, `~/.minecraft`
  - **Windows & Windows on ARM**: `%APPDATA%\PrismLauncher`, `%APPDATA%\.minecraft`
- **Scope Differentiation**: Distinguishes between **Full Launcher Backups** (`scope:launcher`) and **Individual Instance Backups** (`scope:instance`, `instance:<name>`).
- **Targeted Restore**: Restore entire launchers or specific instances directly back into the launcher or into a safe isolated folder.
- **Snapshot File-Size Breakdown**: Analyzes snapshot internals into intuitive categories (Worlds & Saves, Mods & Plugins, Resource Packs, Shader Packs, Configs, Libraries, Assets).

### 5. 🔌 Portable Zero-Dependency Architecture
- **No Homebrew Required**: Operates completely out of the box on macOS without Homebrew or Xcode command line tools.
- **No Package Managers Required**: Operates on Ubuntu Linux without requiring `apt` installations, and on Windows without Chocolatey or Scoop.
- **One-Click Native Engine Installer**: Built-in engine downloader fetches official standalone restic binaries for your exact OS and CPU architecture.
- **Self-Contained Unattended Runner**: Automatically copies the runner and restic engine into persistent host directories (`~/.minevault/bin/` or `%LOCALAPPDATA%\resticdashboard\bin\`) so scheduled backups continue without the USB drive.

---

## 💻 Cross-Platform & Architecture Support

| Operating System | Architectures | Scheduler Engine | Process Inspection | Graphics & Acceleration |
| :--- | :--- | :--- | :--- | :--- |
| **macOS** | **Apple Silicon (ARM64 / M1-M4)**<br>Intel (x86_64) | Built-in Apple `launchd`<br>(`~/Library/LaunchAgents`) | Native POSIX `ps -A` | **Apple Metal** GPU acceleration via `WKWebView` |
| **Linux (Ubuntu / Debian)** | **x86_64**<br>ARM64 | Systemd user timers<br>(`~/.config/systemd/user`) | Native POSIX `ps -A` | Native WebKitGTK / XDG Desktop Portal |
| **Windows & Windows on ARM** | **x86_64**<br>**ARM64 (WoA / Copilot+ PCs)** | Task Scheduler (`schtasks.exe`) | Native ARM64 PowerShell (`Get-CimInstance`) & `wmic` fallback | Microsoft WebView2 |

---

## 🍎 macOS Quick Start & Troubleshooting ("Why macOS likes to be hard")

macOS defaults to aggressive security checks on open-source web downloads (Gatekeeper, quarantine tags, and admin password prompts in System Settings). 

If macOS reports **"cannot be opened because Apple cannot check it for malicious software"** or prompts for an administrator password:

> [!TIP]
> **The 1-Second Zero-Admin Fix**:
> Open Terminal and run:
> ```bash
> xattr -cr /Applications/MineVault.app
> ```
> *(Or `xattr -d com.apple.quarantine ~/Downloads/MineVault-*.dmg`)*
> 
> Because you own your downloaded files, this strips the browser quarantine flag in user space with **zero passwords, zero admin rights, and zero sudo**. Double-click `MineVault.app` and it opens immediately.

For complete details on Gatekeeper, Apple Silicon vs Intel architectures, hidden `~/Library` Minecraft save paths, and native `launchd` background schedules, see the dedicated [**macOS Guide (MACOS-GUIDE.md)**](MACOS-GUIDE.md).

---

## 🏗️ Architecture Overview

```
                               +-----------------------------+
                               |     USB DRIVE (Portable)    |
                               |  - MineVault GUI Controller |
                               |  - Bundled restic engines   |
                               |  - AES-256-GCM Key Vault    |
                               |  - Portable Launch Scripts  |
                               +--------------+--------------+
                                              |
                                              | 1-Click "Install Schedule"
                                              v
+-----------------------------------------------------------------------------------------+
|                                HOST COMPUTER (Persistent)                              |
|                                                                                         |
|  [Persistent App Directory]                                                             |
|   macOS:   ~/Library/Application Support/resticdashboard/                              |
|   Linux:   ~/.local/share/resticdashboard/                                              |
|   Windows: %LOCALAPPDATA%\resticdashboard\                                              |
|                                                                                         |
|   ├── bin/                                                                              |
|   │   ├── resticdashboard-runner (Copied binary: runs headless via --runner flag)      |
|   │   └── restic                 (Copied restic engine)                                 |
|   ├── configs/<profile-id>.json  (Non-secret profile & repository settings)             |
|   ├── secrets/<profile-id>.env   (Credentials file with restrictive 0600 permissions)  |
|   ├── logs/<profile-id>.log      (Rotated run logs)                                     |
|   └── status/<profile-id>.json   (Machine-readable latest run execution results)        |
|                                                                                         |
|  [Native OS Schedulers - Zero Dependencies]                                             |
|   macOS:   ~/Library/LaunchAgents/com.resticdashboard.<profile-id>.plist                |
|   Linux:   ~/.config/systemd/user/resticdashboard-<profile-id>.timer                    |
|   Windows: Task Scheduler ("ResticDashboard_<profile-id>")                              |
+-----------------------------------------------------------------------------------------+
```

---

## 🚀 Getting Started

### Prerequisites
- **Node.js**: v20+ (`node --version`)
- **Rust**: 1.80+ (`rustc --version`, `cargo --version`)
- **restic**: Optional (MineVault automatically bundles or downloads standalone restic on demand).

### Installation & Build

```bash
# 1. Clone the repository
git clone https://github.com/5ndr/minevault.git
cd minevault

# 2. Install frontend dependencies
npm install

# 3. Run automated tests (Rust backend & integration tests)
cd src-tauri
cargo test
cd ..

# 4. Compile frontend production bundle
npm run build

# 5. Launch in development mode
npm run tauri dev
```

---

## 📦 Building the Portable USB Distribution

To package MineVault to run directly from a USB flash drive:

```
USB_DRIVE/
├── launch_macos.command       # Double-click launcher for macOS
├── launch_linux.sh            # Auto-bypasses USB noexec mounts on Linux
├── launch_windows.bat         # Double-click launcher for Windows
├── download_restic_binaries.sh# Fetches official multi-platform restic engines
├── README.txt
├── macos/
│   ├── ResticDashboard.app
│   └── bin/restic             # Bundled macOS Apple Silicon / Intel restic
├── linux/
│   ├── resticdashboard
│   └── bin/restic             # Bundled Linux restic
└── windows/
    ├── resticdashboard.exe
    └── bin/restic.exe         # Bundled Windows restic
```

Run `./usb_dist/download_restic_binaries.sh` to download and verify multi-platform restic binaries into the distribution folders.

---

## 🧪 Automated Test Suite

The Rust backend includes **18 unit and integration tests** verifying:
- Path traversal protection on restoration
- launchd and systemd scheduler generation with persistent paths
- Secret redaction and log sanitization
- Timeout enforcement (preventing process hangs)
- Real local scratch repository backup, stats, and byte-for-byte restore fidelity
- Minecraft path categorization (instances, worlds, saves, mods, resourcepacks)
- B2 target parsing and v3 authorization response handling
- AES-256-GCM USB vault encryption, decryption, and wrong-password rejection
- Process guard classification

```bash
cd src-tauri
cargo test -- --nocapture
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
