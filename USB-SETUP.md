# MineVault USB Drive Preparation & Multi-Platform Setup Guide

This guide explains how to properly prepare and format a USB flash drive for **MineVault**, ensuring maximum portability and reliability across macOS (Apple Silicon & Intel), Windows 10/11, and Linux (Ubuntu/Debian/Fedora/Arch).

---

## ⚠️ Important Warning Before Formatting

> [!CAUTION]
> **Formatting erases all data on the target drive.**
> Ensure you select the correct USB flash drive and backup any important files already stored on it before running any formatting command or utility.
>
> If your USB drive is already formatted as **exFAT**, **do not format it**. You can copy the MineVault distribution directly onto the drive.

---

## 💾 Recommended Filesystem: exFAT

**exFAT (Extensible File Allocation Table)** is the recommended filesystem for the MineVault USB kit.

### Why exFAT?
1. **Universal Cross-Platform Read & Write**:
   - Supported natively on macOS (10.6.5+), Windows (7, 10, 11), and Linux (kernel 5.7+ via the in-kernel `exfat` driver, or user-space `exfat-fuse`).
   - Requires zero third-party drivers or paid software (unlike NTFS on macOS or APFS/ext4 on Windows).
2. **No 4 GiB File Size Barrier**:
   - Older FAT32 filesystems fail on files larger than 4 GiB (4,294,967,295 bytes). Minecraft worlds, dense modpacks, and disk images routinely exceed this limit. exFAT supports theoretical file sizes up to 16 EiB.
3. **Optimized for Flash Media**:
   - Designed by Microsoft specifically for USB flash drives and SD cards with low write overhead.

### Partition Scheme: GUID Partition Table (GPT) or MBR
- Use **GUID Partition Table (GPT)** for modern UEFI computers (macOS, Windows 10/11, modern Linux).
- Alternatively, standard **Master Boot Record (MBR)** is acceptable for older legacy systems.
- **Important**: MineVault is a **standard data storage kit**, NOT a bootable operating system installer.
  - **Do NOT** flash the USB drive using bootable imaging tools like Rufus (DD mode), balenaEtcher, or raw disk dumping (`dd`). Doing so will destroy the filesystem and render the data partition inaccessible as normal storage.
  - MineVault contains no automatic disk partitioning or formatting code. You retain full control over your storage hardware.

---

## 🔍 How to Check Your Drive's Current Format

Before formatting, check if your drive is already exFAT:

### macOS
1. Insert the USB drive and open **Disk Utility** (`/Applications/Utilities/Disk Utility.app`).
2. Select your USB drive in the sidebar.
3. Look at the details under the drive name:
   - If it reads **"exFAT Volume"** or **"Type: exFAT"**, the drive is ready.
   - Or run in Terminal:
     ```bash
     diskutil info /Volumes/<YourDriveName> | grep "Type (Bundle):"
     ```

### Windows
1. Insert the USB drive and open **File Explorer** (`Win + E`).
2. Click **This PC**, right-click your USB flash drive, and select **Properties**.
3. Inspect the **File system** line:
   - If it says **exFAT**, the drive is ready.

### Linux
1. Insert the USB drive and open a terminal.
2. Run:
   ```bash
   lsblk -f
   ```
3. Locate your USB device (e.g. `/dev/sdb1` or `/dev/sdc1`) and check the `FSTYPE` column:
   - If it says `exfat`, the drive is ready.
   - If Linux fails to mount exFAT, install the exFAT utilities:
     ```bash
     # Ubuntu / Debian
     sudo apt-get install exfatprogs exfat-fuse
     # Fedora
     sudo dnf install exfatprogs
     # Arch Linux
     sudo pacman -S exfatprogs
     ```

---

## 🛠️ Safe Formatting Instructions

If your drive needs formatting:

### macOS (Disk Utility)
1. Open **Disk Utility** (`Cmd + Space` -> Disk Utility).
2. Click the **View** dropdown in the upper left corner and select **Show All Devices**.
3. Select the top-level USB disk (not just the indented partition).
4. Click the **Erase** button in the toolbar.
5. Set:
   - **Name**: `MINEVAULT` (or your preferred volume label)
   - **Format**: `ExFAT`
   - **Scheme**: `GUID Partition Table`
6. Click **Erase** and wait for completion.

### Windows (File Explorer)
1. Open **File Explorer** and navigate to **This PC**.
2. Right-click the USB flash drive and click **Format...**.
3. Set:
   - **File system**: `exFAT`
   - **Allocation unit size**: `Default allocation size` (usually 128KB or 256KB for flash drives)
   - **Volume label**: `MINEVAULT`
   - **Format options**: Check `Quick Format`
4. Click **Start** and confirm the data-loss warning.

### Linux (Terminal / GNOME Disks)
Using the terminal (replace `/dev/sdX1` with your actual partition identifier verified via `lsblk`):
```bash
# 1. Unmount the volume if mounted
sudo umount /dev/sdX1

# 2. Create the exFAT filesystem
sudo mkfs.exfat -n "MINEVAULT" /dev/sdX1
```
Or open **GNOME Disks** (`gnome-disks`), select the USB drive, click the gear icon, choose **Format Partition**, select **Other / exFAT**, name it `MINEVAULT`, and click **Next**.

---

## ⚡ Technical Realities & Platform Limitations of exFAT

While exFAT provides cross-platform file access, it does not support POSIX permission semantics. MineVault handles these platform quirks automatically:

### 1. macOS: The `.app` Bundle Integrity Problem
- **Limitation**: exFAT lacks Unix permissions, execute bits (`+x`), POSIX symlinks, and macOS extended attributes.
- If a `.app` bundle is extracted directly onto an exFAT drive, macOS flags code signing seals as broken and refuses to launch the application.
- **MineVault Solution**:
  - The macOS distribution is packaged inside a native Apple Disk Image (`MineVault.dmg`) or compressed `.tar.gz` archive.
  - Double-clicking `MineVault.dmg` mounts a virtual HFS+/APFS volume where all POSIX permissions, internal symlinks, and code signatures remain 100% intact.
  - The provided `launch_macos.command` script mounts the image automatically or stages the app bundle to a local temporary cache (`~/.cache/minevault_staging/`) on the internal APFS drive if necessary.
  - **Quarantine & Admin Prompts**: When downloaded from the web, macOS attaches `com.apple.quarantine`. If macOS asks for an administrator password in System Settings, you can bypass this completely without admin rights by running `Launch-macOS.command` (which strips quarantine automatically) or by running:
    ```bash
    xattr -cr /path/to/MineVault.app
    ```
    This immediately marks the app as trusted and allows it to launch on double-click with zero prompts. See [MACOS-GUIDE.md](MACOS-GUIDE.md) for full details.

### 2. Linux: The `noexec` Mount Problem
- **Limitation**: Almost all modern Linux desktop environments (GNOME, KDE, systemd udisks2) automatically mount FAT32 and exFAT removable media with the `noexec` security mount flag.
- Regular files on exFAT do not store execute mode bits, so running `chmod +x /media/user/USB/MineVault.AppImage` has zero effect on exFAT.
- **MineVault Solution**:
  - Double-clicking or executing `./Linux-x64/launch.sh` tests whether direct execution is allowed.
  - If blocked by `noexec`, `launch.sh` transparently stages `MineVault.AppImage` and `bin/restic` to `~/.cache/minevault/` on the native filesystem (ext4/btrfs/xfs), sets `chmod +x` there, and executes it immediately.
  - **No root (`sudo`) required**: The user never needs to edit `/etc/fstab` or modify system mount policies.

### 3. Windows: Standalone Execution & WebView2
- **Limitation**: Windows executes `.exe` files directly from exFAT without permission issues. However, modern Tauri applications require the **Microsoft Edge WebView2 Runtime**.
- **Runtime Environment**:
  - Windows 10 (version 1809 and later with updates) and all editions of Windows 11 include Evergreen WebView2 pre-installed out-of-the-box.
  - On pristine, stripped, or offline Windows systems where WebView2 was uninstalled, download the official Evergreen WebView2 Standalone/Bootstrapper from Microsoft (`https://developer.microsoft.com/en-us/microsoft-edge/webview2/`).
  - MineVault's Windows package is an actual standalone application folder containing `MineVault.exe`, sidecar restic binaries, and dependencies—**not** an installer renamed as a portable executable.

### 4. Why Not a Single Universal Executable?
- Different operating systems use incompatible binary executable formats:
  - macOS uses **Mach-O** (Fat 64-bit binaries).
  - Linux uses **ELF** (Executable and Linkable Format).
  - Windows uses **PE32+** (Portable Executable 64-bit).
- Different processors use distinct machine instruction sets:
  - Intel/AMD processors use **x86_64 / amd64**.
  - Apple Silicon and Copilot+ PCs use **ARM64 (aarch64)**.
- MineVault accommodates this by organizing platform-native executables into dedicated directories:
  - `Windows-x64/`
  - `macOS-arm64/`
  - `macOS-x64/`
  - `Linux-x64/`

---

## 🧹 Local Footprint & Privacy Honesty

MineVault runs completely without an installation wizard or installer registry keys. However, for total technical honesty:
- **Web Engine Caches**: The OS webview engine creates standard runtime rendering and GPU caches in the active user profile:
  - Linux: `~/.cache/minevault`, `~/.config/minevault`
  - macOS: `~/Library/Caches/com.minevault.app`, `~/Library/Application Support/com.minevault.app`
  - Windows: `%LOCALAPPDATA%\com.minevault.app`
- **Background Tasks**: Launching MineVault from the USB drive **never** silently installs background services or scheduled tasks. Persistent automated background backups are only created when you explicitly click "Install Background Schedule" in the wizard.

---

## ⏏️ Safe Ejection Instructions

Before unplugging your USB drive from any computer, always eject it properly to avoid corrupted data:

- **macOS**: Drag the USB drive icon from your desktop to the Trash (which turns into an Eject icon), or select the drive in Finder and press `Cmd + E`.
- **Windows**: Click the arrow in the bottom-right taskbar (system tray), click the **Safely Remove Hardware and Eject Media** USB icon, and choose your drive. Wait for the "Safe to Remove Hardware" notification.
- **Linux**: In your file manager (Nautilus, Dolphin, Thunar), click the **⏏ (Eject)** button next to the USB drive. Alternatively, run in terminal:
  ```bash
  udisksctl unmount -b /dev/sdX1 && udisksctl power-off -b /dev/sdX
  ```
