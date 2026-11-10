use crate::models::LauncherDescriptor;
use std::path::{Path, PathBuf};

/// Returns a stable machine identifier for filtering backups per machine.
pub fn get_machine_id() -> String {
    let hostname = get_machine_name();
    let user = whoami_user();
    format!("{}-{}", hostname, user)
}

/// Returns a readable machine name (hostname).
pub fn get_machine_name() -> String {
    // Try reading hostname from env or libc
    if let Ok(host) = std::env::var("HOSTNAME")
        && !host.trim().is_empty()
    {
        return host.trim().to_string();
    }

    #[cfg(unix)]
    {
        let mut buf = [0u8; 256];
        let res = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
        if res == 0
            && let Ok(s) = std::ffi::CStr::from_bytes_until_nul(&buf)
            && let Ok(str_slice) = s.to_str()
        {
            return str_slice.to_string();
        }
    }

    #[cfg(windows)]
    {
        if let Ok(computer_name) = std::env::var("COMPUTERNAME") {
            return computer_name;
        }
    }

    "unknown-host".to_string()
}

fn whoami_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

/// Get free disk space in bytes for a given path (destination).
pub fn get_disk_free_space(path: &Path) -> Result<u64, String> {
    // If the path doesn't exist yet, check its closest existing parent
    let mut check_path = path.to_path_buf();
    while !check_path.exists() {
        match check_path.parent() {
            Some(parent) => check_path = parent.to_path_buf(),
            None => break,
        }
    }

    #[cfg(unix)]
    {
        let path_str = check_path.to_str().ok_or("Invalid UTF-8 in path")?;
        let c_path = std::ffi::CString::new(path_str).map_err(|e| e.to_string())?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        let res = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
        if res == 0 {
            // f_bavail is free blocks available to non-root user
            let free_bytes = (stat.f_bavail as u64) * (stat.f_frsize as u64);
            Ok(free_bytes)
        } else {
            Err(format!("libc::statvfs failed with code {}", res))
        }
    }

    #[cfg(windows)]
    {
        let root = check_path
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| "C:\\".to_string());
        let drive = root.trim_end_matches('\\');
        let ps_cmd = format!(
            "(Get-PSDrive -PSProvider FileSystem | Where-Object {{ $_.Root -like '{}*' }} | Select-Object -First 1).Free",
            drive
        );
        if let Ok(out) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
            .output()
        {
            if out.status.success() {
                if let Ok(s) = String::from_utf8(out.stdout) {
                    if let Ok(free_bytes) = s.trim().parse::<u64>() {
                        return Ok(free_bytes);
                    }
                }
            }
        }
        Ok(100 * 1024 * 1024 * 1024) // 100 GiB safe fallback if powershell fails
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(100 * 1024 * 1024 * 1024) // 100 GiB placeholder if unqueried
    }
}

/// Returns the platform-specific default Prism Launcher directory.
pub fn detect_prism_launcher_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let p = home.join("Library/Application Support/PrismLauncher");
            if p.exists() {
                return Some(p);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(data_dir) = dirs::data_dir() {
            let p = data_dir.join("PrismLauncher");
            if p.exists() {
                return Some(p);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = dirs::config_dir() {
            let p = app_data.join("PrismLauncher");
            if p.exists() {
                return Some(p);
            }
        }
    }

    None
}

/// Lists all supported Minecraft launchers, checking their common paths on this OS.
pub fn detect_all_launchers() -> Vec<LauncherDescriptor> {
    let mut launchers = Vec::new();
    let home = dirs::home_dir();
    let app_data = dirs::config_dir();
    let data_dir = dirs::data_dir();

    // 1. Prism Launcher
    let prism_path = if cfg!(target_os = "macos") {
        home.as_ref()
            .map(|h| h.join("Library/Application Support/PrismLauncher"))
    } else if cfg!(target_os = "windows") {
        app_data.as_ref().map(|a| a.join("PrismLauncher"))
    } else {
        data_dir.as_ref().map(|d| d.join("PrismLauncher"))
    };
    if let Some(p) = prism_path {
        let detected = p.exists();
        let instances = if detected {
            scan_instances_in_path(&p)
        } else {
            Vec::new()
        };
        launchers.push(LauncherDescriptor {
            id: "prism".to_string(),
            name: "Prism Launcher".to_string(),
            default_path: p.to_string_lossy().to_string(),
            detected,
            instances,
        });
    }

    // 2. Modrinth App (Theseus)
    let modrinth_path = if cfg!(target_os = "macos") {
        home.as_ref()
            .map(|h| h.join("Library/Application Support/com.modrinth.theseus"))
    } else if cfg!(target_os = "windows") {
        app_data.as_ref().map(|a| a.join("com.modrinth.theseus"))
    } else {
        data_dir.as_ref().map(|d| d.join("com.modrinth.theseus"))
    };
    if let Some(p) = modrinth_path {
        let detected = p.exists();
        let instances = if detected {
            scan_instances_in_path(&p)
        } else {
            Vec::new()
        };
        launchers.push(LauncherDescriptor {
            id: "modrinth".to_string(),
            name: "Modrinth App".to_string(),
            default_path: p.to_string_lossy().to_string(),
            detected,
            instances,
        });
    }

    // 3. CurseForge
    let curseforge_path = if cfg!(target_os = "macos") {
        home.as_ref()
            .map(|h| h.join("Documents/curseforge/minecraft"))
    } else if cfg!(target_os = "windows") {
        home.as_ref().map(|h| h.join("curseforge/minecraft"))
    } else {
        home.as_ref().map(|h| h.join("curseforge/minecraft"))
    };
    if let Some(p) = curseforge_path {
        let detected = p.exists();
        let instances = if detected {
            scan_instances_in_path(&p)
        } else {
            Vec::new()
        };
        launchers.push(LauncherDescriptor {
            id: "curseforge".to_string(),
            name: "CurseForge".to_string(),
            default_path: p.to_string_lossy().to_string(),
            detected,
            instances,
        });
    }

    // 4. Official Minecraft (Vanilla)
    let vanilla_path = if cfg!(target_os = "macos") {
        home.as_ref()
            .map(|h| h.join("Library/Application Support/minecraft"))
    } else if cfg!(target_os = "windows") {
        app_data.as_ref().map(|a| a.join(".minecraft"))
    } else {
        home.as_ref().map(|h| h.join(".minecraft"))
    };
    if let Some(p) = vanilla_path {
        let detected = p.exists();
        let instances = if detected {
            scan_instances_in_path(&p)
        } else {
            Vec::new()
        };
        launchers.push(LauncherDescriptor {
            id: "vanilla".to_string(),
            name: "Official Minecraft (Vanilla)".to_string(),
            default_path: p.to_string_lossy().to_string(),
            detected,
            instances,
        });
    }

    // 5. ATLauncher
    let atlauncher_path = if cfg!(target_os = "macos") {
        home.as_ref()
            .map(|h| h.join("Library/Application Support/ATLauncher"))
    } else if cfg!(target_os = "windows") {
        app_data.as_ref().map(|a| a.join("ATLauncher"))
    } else {
        home.as_ref().map(|h| h.join(".atlauncher"))
    };
    if let Some(p) = atlauncher_path {
        let detected = p.exists();
        let instances = if detected {
            scan_instances_in_path(&p)
        } else {
            Vec::new()
        };
        launchers.push(LauncherDescriptor {
            id: "atlauncher".to_string(),
            name: "ATLauncher".to_string(),
            default_path: p.to_string_lossy().to_string(),
            detected,
            instances,
        });
    }

    // 6. MultiMC
    let multimc_path = if cfg!(target_os = "macos") {
        home.as_ref()
            .map(|h| h.join("Library/Application Support/MultiMC"))
    } else if cfg!(target_os = "windows") {
        app_data.as_ref().map(|a| a.join("MultiMC"))
    } else {
        data_dir.as_ref().map(|d| d.join("multimc"))
    };
    if let Some(p) = multimc_path {
        let detected = p.exists();
        let instances = if detected {
            scan_instances_in_path(&p)
        } else {
            Vec::new()
        };
        launchers.push(LauncherDescriptor {
            id: "multimc".to_string(),
            name: "MultiMC".to_string(),
            default_path: p.to_string_lossy().to_string(),
            detected,
            instances,
        });
    }

    launchers
}

/// Scans for instance or world folders in a directory
pub fn scan_instances_in_path(root: &Path) -> Vec<String> {
    let mut instances = Vec::new();
    let candidates = [
        root.join("instances"),
        root.join("Instances"),
        root.join("profiles"),
        root.join("saves"), // for vanilla worlds
    ];

    for dir in &candidates {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type()
                    && ft.is_dir()
                {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with('.') && !instances.contains(&name) {
                        instances.push(name);
                    }
                }
            }
        }
    }

    instances.sort();
    instances
}

/// Lists instance names in a Prism Launcher directory.
pub fn list_prism_launcher_instances(root: &Path) -> Vec<String> {
    scan_instances_in_path(root)
}

/// Returns the persistent local directory used for unattended runner, local configs, and logs.
/// macOS: ~/Library/Application Support/resticdashboard
/// Linux: ~/.local/share/resticdashboard
/// Windows: %LOCALAPPDATA%\resticdashboard
pub fn get_persistent_app_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join("Library/Application Support/resticdashboard");
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(data) = dirs::data_local_dir() {
            return data.join("resticdashboard");
        }
    }

    // Default Linux & generic Unix
    if let Some(data) = dirs::data_dir() {
        data.join("resticdashboard")
    } else {
        PathBuf::from("/tmp/resticdashboard")
    }
}

/// Downloads and installs the native restic binary directly into the persistent bin directory
/// without requiring Homebrew, package managers, or administrator privileges.
pub fn download_native_restic_engine() -> Result<PathBuf, String> {
    let base_dir = get_persistent_app_dir();
    let bin_dir = base_dir.join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin dir: {}", e))?;

    let target_bin = if cfg!(windows) {
        "restic.exe"
    } else {
        "restic"
    };
    let dest_path = bin_dir.join(target_bin);

    let version = "0.19.1";
    #[cfg(target_os = "macos")]
    {
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "amd64"
        };
        let url = format!(
            "https://github.com/restic/restic/releases/download/v{}/restic_{}_darwin_{}.bz2",
            version, version, arch
        );
        let cmd = format!(
            "curl -sL '{}' | bunzip2 > '{}' && chmod +x '{}'",
            url,
            dest_path.display(),
            dest_path.display()
        );
        let out = std::process::Command::new("/bin/sh")
            .args(["-c", &cmd])
            .output()
            .map_err(|e| format!("Download command failed: {}", e))?;
        if !out.status.success() || !dest_path.is_file() {
            return Err(format!(
                "Failed to download and extract restic: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "amd64"
        };
        let url = format!(
            "https://github.com/restic/restic/releases/download/v{}/restic_{}_linux_{}.bz2",
            version, version, arch
        );
        let cmd = format!(
            "curl -sL '{}' | bunzip2 > '{}' && chmod +x '{}'",
            url,
            dest_path.display(),
            dest_path.display()
        );
        let out = std::process::Command::new("/bin/sh")
            .args(["-c", &cmd])
            .output()
            .map_err(|e| format!("Download command failed: {}", e))?;
        if !out.status.success() || !dest_path.is_file() {
            return Err(format!(
                "Failed to download and extract restic: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let url = format!(
            "https://github.com/restic/restic/releases/download/v{}/restic_{}_windows_amd64.zip",
            version, version
        );
        let ps_cmd = format!(
            "$zip = Join-Path $env:TEMP 'restic_win.zip'; $extr = Join-Path $env:TEMP 'restic_ext'; \
             Invoke-WebRequest -Uri '{}' -OutFile $zip; \
             Expand-Archive -Path $zip -DestinationPath $extr -Force; \
             $exe = Get-ChildItem -Path $extr -Filter '*.exe' -Recurse | Select-Object -First 1; \
             Copy-Item -Path $exe.FullName -Destination '{}' -Force; \
             Remove-Item -Path $zip, $extr -Recurse -Force -ErrorAction SilentlyContinue",
            url,
            dest_path.display()
        );
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
            .output()
            .map_err(|e| format!("Download command failed: {}", e))?;
        if !out.status.success() || !dest_path.is_file() {
            return Err(format!(
                "Failed to download restic on Windows: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest_path, std::fs::Permissions::from_mode(0o755));
    }

    Ok(dest_path)
}
