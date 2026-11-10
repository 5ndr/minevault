# 🍎 MineVault macOS Guide & Troubleshooting

> *"Why does macOS have to be so hard sometimes?"*

macOS includes multiple overlapping security layers: **Gatekeeper**, **Quarantine flags**, **App Translocation**, **TCC (Transparency, Consent, and Control)**, and **Architecture Enforcements**. Because MineVault is a free, open-source project not enrolled in Apple's paid ($99/year) Developer Program, macOS defaults to aggressive warnings.

This guide explains **why** macOS acts this way and how to get MineVault running smoothly with **zero admin rights, zero password prompts, and zero headaches**.

---

## ⚡ Quick Fix (TL;DR)

If macOS tells you:
> *"MineVault can't be opened because Apple cannot check it for malicious software."*  
> Or prompts for an **Administrator Password** when you click *"Open Anyway"* in System Settings:

### 🚀 The 1-Second Terminal Fix (No Admin / No Password Required)

Open **Terminal** (press `Cmd + Space`, type `Terminal`, hit Enter) and paste:

```bash
# If you dragged MineVault to Applications:
xattr -cr /Applications/MineVault.app

# OR if you are running it from Downloads or your Desktop:
xattr -cr ~/Downloads/MineVault.app
```

Or if you haven't opened the `.dmg` yet:
```bash
xattr -d com.apple.quarantine ~/Downloads/MineVault-*.dmg
```

**That's it!** Double-click `MineVault.app` and it will open immediately on your Mac without any dialogs, errors, or password requests.

---

## 🔍 Why Does macOS Do This?

### 1. The `com.apple.quarantine` Attribute
When you download any file through a web browser (Safari, Chrome, Firefox, Brave), macOS automatically tags the file with a hidden extended filesystem attribute called `com.apple.quarantine`.

When you try to run an application with this tag:
1. macOS checks if the app is notarized by Apple's automated cloud servers.
2. Because MineVault is open-source and not signed with an Apple Developer ID, macOS refuses to open it.
3. If you go to **System Settings > Privacy & Security > Security** and click **"Open Anyway"**, macOS treats modifying the system-wide security exception database as a system change, which triggers an **Administrator Password / Touch ID** prompt.

**Why `xattr -cr` works without Admin:**
Because you downloaded and own the file, you have full permission to clear extended attributes on your own files. Running `xattr -cr` deletes the quarantine tag in user space. Once deleted, Gatekeeper ignores the app and treats it as a locally built, trusted application.

---

### 2. Apple Silicon (ARM64 / M1–M4) vs Intel (x86_64)

Apple Macs use two different processor types. Make sure you use the build matching your hardware:

| Your Mac | Recommended Release Asset |
| :--- | :--- |
| **M1, M2, M3, M4, M-Pro, M-Max** (2020+) | `MineVault-v0.1.1-macos-arm64.dmg` |
| **Intel Core i5 / i7 / i9** (2019 and older) | `MineVault-v0.1.1-macos-x64.dmg` |

*To check which you have: Click  (top-left) > **About This Mac** > look at **Chip** or **Processor**.*

- The **ARM64** build features native **Apple Metal GPU acceleration** via WebKit and uses significantly less battery and CPU.
- Both builds are ad-hoc signed so they meet macOS ARM64 kernel execution requirements out of the box.

---

### 3. App Translocation (Running Directly from the DMG)

If you double-click `MineVault.app` while it is still inside the mounted `.dmg` disk image:
- macOS mounts the disk image as **Read-Only**.
- macOS places the app into a randomized temporary sandbox called **App Translocation** (`/private/var/folders/.../AppTranslocation/...`).
- In this sandbox, helper tools (like the bundled `restic` engine) may struggle to discover neighboring files.

**Best Practice:**
Always drag `MineVault.app` from the DMG into your `/Applications` folder (or your user's `~/Applications` or Desktop) before running it.

---

### 4. Finding Your Minecraft Saves on macOS (Hidden `~/Library`)

macOS hides the user `Library` folder by default to prevent accidental modifications.

Your Minecraft saves and launcher directories are located at:
- **Official Vanilla Launcher**: `~/Library/Application Support/minecraft`
- **Prism Launcher**: `~/Library/Application Support/PrismLauncher`
- **CurseForge**: `~/Library/Application Support/curseforge`
- **Modrinth**: `~/Library/Application Support/com.modrinth.theseus`

**To see your `~/Library` folder in Finder:**
1. Open **Finder**.
2. Press **`Cmd + Shift + .`** (Period) to reveal all hidden files and folders.
3. Or press **`Cmd + Shift + G`**, paste `~/Library/Application Support/minecraft`, and press Enter.

---

### 5. Automated Background Backups & macOS `launchd`

MineVault's "Automatic Backup Wizard" lets you schedule recurring backups (hourly, daily, weekly) that continue running even when MineVault is closed and the USB drive is disconnected.

- On macOS, this uses Apple's native scheduler: **`launchd`**.
- It creates a standard user agent at:
  `~/Library/LaunchAgents/com.resticdashboard.<profile-id>.plist`
- The background runner and restic engine are copied to:
  `~/Library/Application Support/resticdashboard/bin/`

**Key Advantages:**
- **Zero Admin / Zero Sudo**: Uses standard per-user `launchd` (`gui/$UID`).
- **Energy-Efficient**: Respects macOS Power Nap and wakes on schedule.
- **Process Guard**: If you are playing Minecraft when a scheduled backup fires, the runner detects the active game process and safely postpones until the game is closed, preventing save corruption.
