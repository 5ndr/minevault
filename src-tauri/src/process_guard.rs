use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningMinecraftProcess {
    pub pid: u32,
    pub name: String,
    pub kind: String, // "launcher" | "game_instance"
    pub command_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessCheckResult {
    pub is_running: bool,
    pub processes: Vec<RunningMinecraftProcess>,
}

/// Classifies a process command line as a Minecraft launcher, game instance, or unrelated.
pub fn classify_process(pid: u32, args_str: &str) -> Option<RunningMinecraftProcess> {
    let low = args_str.to_lowercase();

    // Skip ourselves, grep, ps, or test runners
    if low.contains("resticdashboard")
        || low.contains("ps -a")
        || low.contains("grep")
        || low.contains("target/debug")
        || low.contains("target/release")
    {
        return None;
    }

    // 1. Check Known Minecraft Launchers
    if low.contains("prismlauncher") || low.contains("prism launcher") {
        return Some(RunningMinecraftProcess {
            pid,
            name: "Prism Launcher".to_string(),
            kind: "launcher".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    if low.contains("modrinthapp") || low.contains("modrinth app") || low.contains("theseus") {
        return Some(RunningMinecraftProcess {
            pid,
            name: "Modrinth App".to_string(),
            kind: "launcher".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    if low.contains("curseforge") || low.contains("curse client") {
        return Some(RunningMinecraftProcess {
            pid,
            name: "CurseForge".to_string(),
            kind: "launcher".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    if low.contains("atlauncher") {
        return Some(RunningMinecraftProcess {
            pid,
            name: "ATLauncher".to_string(),
            kind: "launcher".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    if low.contains("multimc") {
        return Some(RunningMinecraftProcess {
            pid,
            name: "MultiMC".to_string(),
            kind: "launcher".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    if low.contains("minecraftlauncher")
        || low.contains("minecraft.app/contents/macos/launcher")
        || low.contains("minecraft.exe")
    {
        return Some(RunningMinecraftProcess {
            pid,
            name: "Minecraft (Vanilla) Launcher".to_string(),
            kind: "launcher".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    if low.contains("lunar client") || low.contains("lunar-client") {
        return Some(RunningMinecraftProcess {
            pid,
            name: "Lunar Client".to_string(),
            kind: "launcher".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    if low.contains("badlion") {
        return Some(RunningMinecraftProcess {
            pid,
            name: "Badlion Client".to_string(),
            kind: "launcher".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    // 2. Check Minecraft Game Instances (Java / Bedrock)
    let is_java = low.contains("java") || low.contains("javaw");
    if is_java {
        let minecraft_indicators = [
            "net.minecraft.client.main.main",
            "net.fabricmc.loader",
            "org.quiltmc.loader",
            "net.minecraftforge",
            "net.neoforged",
            "cpw.mods.bootstraplauncher",
            "net.minecraft.launchwrapper",
            "minecraft.applet",
            "/instances/",
            "\\instances\\",
            "/.minecraft",
            "\\.minecraft",
        ];

        for ind in &minecraft_indicators {
            if low.contains(ind) {
                // Try extracting instance name if inside an instances directory
                let mut instance_label = "Minecraft Game Instance (Java)".to_string();
                if let Some(idx) = low.find("instances/") {
                    let sub = &args_str[idx + 10..];
                    if let Some(slash_idx) = sub.find('/') {
                        let name = &sub[..slash_idx];
                        if !name.is_empty() {
                            instance_label = format!("Minecraft Instance: {}", name);
                        }
                    }
                } else if let Some(idx) = low.find("instances\\") {
                    let sub = &args_str[idx + 10..];
                    if let Some(slash_idx) = sub.find('\\') {
                        let name = &sub[..slash_idx];
                        if !name.is_empty() {
                            instance_label = format!("Minecraft Instance: {}", name);
                        }
                    }
                }

                return Some(RunningMinecraftProcess {
                    pid,
                    name: instance_label,
                    kind: "game_instance".to_string(),
                    command_line: args_str.trim().to_string(),
                });
            }
        }
    }

    if low.contains("minecraft.windows.exe") || low.contains("bedrock_server") {
        return Some(RunningMinecraftProcess {
            pid,
            name: "Minecraft Bedrock Edition".to_string(),
            kind: "game_instance".to_string(),
            command_line: args_str.trim().to_string(),
        });
    }

    None
}

/// Detects all running Minecraft launcher and game instance processes.
pub fn detect_minecraft_processes() -> ProcessCheckResult {
    let mut detected = Vec::new();

    #[cfg(unix)]
    {
        let out = Command::new("ps")
            .arg("-A")
            .arg("-o")
            .arg("pid=,args=")
            .output();

        if let Ok(out) = out {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // First token is PID, rest is full args
                let mut parts = trimmed.split_whitespace();
                if let Some(pid_str) = parts.next() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        let args_str = &trimmed[pid_str.len()..].trim();
                        if let Some(proc_info) = classify_process(pid, args_str) {
                            detected.push(proc_info);
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // 1. Try wmic first (standard on Windows 10 and earlier Windows 11)
        let out = Command::new("wmic")
            .arg("process")
            .arg("get")
            .arg("ProcessId,CommandLine")
            .arg("/format:csv")
            .output();

        let mut wmic_succeeded = false;
        if let Ok(out) = out {
            if out.status.success() {
                wmic_succeeded = true;
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        // CSV format: Node, CommandLine, ProcessId
                        let cmdline = parts[1].trim();
                        let pid_str = parts[2].trim();
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            if let Some(proc_info) = classify_process(pid, cmdline) {
                                detected.push(proc_info);
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback to PowerShell Get-CimInstance (standard on Windows 11 24H2 & Windows on ARM where wmic is removed)
        if !wmic_succeeded {
            let ps_script = "Get-CimInstance Win32_Process | ForEach-Object { \"$($_.ProcessId)::$($_.CommandLine)\" }";
            let ps_out = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
                .output();

            if let Ok(ps_out) = ps_out {
                let stdout = String::from_utf8_lossy(&ps_out.stdout);
                for line in stdout.lines() {
                    if let Some((pid_str, cmdline)) = line.split_once("::") {
                        if let Ok(pid) = pid_str.trim().parse::<u32>() {
                            if let Some(proc_info) = classify_process(pid, cmdline) {
                                detected.push(proc_info);
                            }
                        }
                    }
                }
            }
        }
    }

    let is_running = !detected.is_empty();
    ProcessCheckResult {
        is_running,
        processes: detected,
    }
}

/// Closes the specified process IDs. If force is false, requests graceful shutdown (SIGTERM / taskkill).
pub fn close_processes(pids: &[u32], force: bool) -> Result<usize, String> {
    let mut closed_count = 0;

    for &pid in pids {
        #[cfg(unix)]
        {
            let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
            let res = unsafe { libc::kill(pid as libc::pid_t, sig) };
            if res == 0 {
                closed_count += 1;
            }
        }

        #[cfg(windows)]
        {
            let mut cmd = Command::new("taskkill");
            if force {
                cmd.arg("/F");
            }
            cmd.arg("/PID").arg(pid.to_string());
            if let Ok(status) = cmd.status() {
                if status.success() {
                    closed_count += 1;
                }
            }
        }
    }

    Ok(closed_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_prism_launcher() {
        let cmd = "/Applications/Prism Launcher.app/Contents/MacOS/prismlauncher";
        let proc = classify_process(1234, cmd).unwrap();
        assert_eq!(proc.name, "Prism Launcher");
        assert_eq!(proc.kind, "launcher");
    }

    #[test]
    fn test_classify_minecraft_fabric_instance() {
        let cmd = "/usr/bin/java -Xmx4G -cp ... net.fabricmc.loader.impl.launch.knot.KnotClient -instances/1.20.4/run";
        let proc = classify_process(5678, cmd).unwrap();
        assert_eq!(proc.kind, "game_instance");
        assert!(proc.name.contains("Minecraft"));
    }

    #[test]
    fn test_classify_unrelated_process() {
        let cmd = "/usr/bin/python3 -m http.server 8000";
        assert!(classify_process(9999, cmd).is_none());
    }
}
