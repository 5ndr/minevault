use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::{Profile, RepoConfig, RunResult, SchedulerStatus, SessionCredentials};
use crate::scheduler::{SchedulerAdapter, setup_persistent_local_files};
use crate::system::get_persistent_app_dir;

pub struct MacOsLaunchdAdapter;

impl MacOsLaunchdAdapter {
    pub fn new() -> Self {
        Self
    }

    fn plist_path(profile_id: &str) -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("Library/LaunchAgents")
            .join(format!("com.resticdashboard.{}.plist", profile_id))
    }

    fn service_label(profile_id: &str) -> String {
        format!("com.resticdashboard.{}", profile_id)
    }

    pub fn generate_plist_xml(
        profile_id: &str,
        runner_exe: &Path,
        base_dir: &Path,
        schedule: &crate::models::ScheduleSpec,
    ) -> String {
        let label = Self::service_label(profile_id);
        let stdout_log = base_dir
            .join("logs")
            .join(format!("{}-launchd.out.log", profile_id));
        let stderr_log = base_dir
            .join("logs")
            .join(format!("{}-launchd.err.log", profile_id));

        let schedule_xml = match schedule.frequency.as_str() {
            "hourly" => "    <key>StartInterval</key>\n    <integer>3600</integer>".to_string(),
            "weekly" => {
                let day = schedule.day_of_week.unwrap_or(1);
                format!(
                    "    <key>StartCalendarInterval</key>\n    <dict>\n        <key>Weekday</key>\n        <integer>{}</integer>\n        <key>Hour</key>\n        <integer>{}</integer>\n        <key>Minute</key>\n        <integer>{}</integer>\n    </dict>",
                    day, schedule.hour, schedule.minute
                )
            }
            _ => {
                // Default daily
                format!(
                    "    <key>StartCalendarInterval</key>\n    <dict>\n        <key>Hour</key>\n        <integer>{}</integer>\n        <key>Minute</key>\n        <integer>{}</integer>\n    </dict>",
                    schedule.hour, schedule.minute
                )
            }
        };

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>runner</string>
        <string>--profile-id</string>
        <string>{}</string>
        <string>--base-dir</string>
        <string>{}</string>
    </array>
{}
    <key>StandardOutPath</key>
    <string>{}</string>
    <key>StandardErrorPath</key>
    <string>{}</string>
    <key>RunAtLoad</key>
    <false/>
</dict>
</plist>
"#,
            label,
            runner_exe.display(),
            profile_id,
            base_dir.display(),
            schedule_xml,
            stdout_log.display(),
            stderr_log.display()
        )
    }
}

impl Default for MacOsLaunchdAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulerAdapter for MacOsLaunchdAdapter {
    fn platform_name(&self) -> &'static str {
        "macos"
    }

    fn install(
        &self,
        profile: &Profile,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        restic_bin: &Path,
    ) -> Result<SchedulerStatus, String> {
        let schedule = profile
            .schedule
            .as_ref()
            .ok_or("No schedule configured on profile")?;
        let base_dir = get_persistent_app_dir();

        // 1. Set up persistent local directories, copy runner & restic binaries, write configs
        let (runner_exe, config_path, _secret_path) =
            setup_persistent_local_files(profile, repo, creds, restic_bin)?;

        // 2. Generate LaunchAgent plist file
        let plist_path = Self::plist_path(&profile.id);
        if let Some(parent) = plist_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let xml = Self::generate_plist_xml(&profile.id, &runner_exe, &base_dir, schedule);
        fs::write(&plist_path, xml).map_err(|e| {
            format!(
                "Failed to write LaunchAgent plist to {}: {}",
                plist_path.display(),
                e
            )
        })?;

        // 3. Register with launchctl
        let uid = unsafe { libc::getuid() };
        let domain_target = format!("gui/{}", uid);

        // Try modern launchctl bootstrap
        let boot_res = Command::new("launchctl")
            .args(["bootstrap", &domain_target, plist_path.to_str().unwrap()])
            .output();

        // Fallback to launchctl load
        if boot_res.is_err() || !boot_res.as_ref().unwrap().status.success() {
            let _ = Command::new("launchctl")
                .args(["load", "-w", plist_path.to_str().unwrap()])
                .output();
        }

        let notes = "macOS launchd user agent registered. Backups execute in your active user session. If the Mac is sleeping at the scheduled time, launchd triggers the backup upon system wake.".to_string();

        Ok(SchedulerStatus {
            platform: "macos".to_string(),
            installed: true,
            enabled: true,
            target_binary_path: runner_exe.to_string_lossy().to_string(),
            config_path: config_path.to_string_lossy().to_string(),
            schedule_file_path: plist_path.to_string_lossy().to_string(),
            last_run: None,
            notes,
        })
    }

    fn uninstall(&self, profile_id: &str) -> Result<(), String> {
        let plist_path = Self::plist_path(profile_id);
        let uid = unsafe { libc::getuid() };
        let domain_target = format!("gui/{}", uid);
        let service_target = format!("gui/{}/{}", uid, Self::service_label(profile_id));

        // Try bootout or unload
        let _ = Command::new("launchctl")
            .args(["bootout", &service_target])
            .output();

        let _ = Command::new("launchctl")
            .args(["bootout", &domain_target, plist_path.to_str().unwrap_or("")])
            .output();

        let _ = Command::new("launchctl")
            .args(["unload", plist_path.to_str().unwrap_or("")])
            .output();

        if plist_path.exists() {
            let _ = fs::remove_file(&plist_path);
        }

        // Clean up persistent config & secrets (preserving backup repository data)
        let base_dir = get_persistent_app_dir();
        let _ = fs::remove_file(
            base_dir
                .join("configs")
                .join(format!("{}.json", profile_id)),
        );
        let _ = fs::remove_file(base_dir.join("secrets").join(format!("{}.env", profile_id)));

        Ok(())
    }

    fn disable(&self, profile_id: &str) -> Result<(), String> {
        let plist_path = Self::plist_path(profile_id);
        let uid = unsafe { libc::getuid() };
        let service_target = format!("gui/{}/{}", uid, Self::service_label(profile_id));

        let _ = Command::new("launchctl")
            .args(["bootout", &service_target])
            .output();

        let _ = Command::new("launchctl")
            .args(["unload", plist_path.to_str().unwrap_or("")])
            .output();

        Ok(())
    }

    fn enable(&self, profile_id: &str) -> Result<(), String> {
        let plist_path = Self::plist_path(profile_id);
        if !plist_path.exists() {
            return Err(
                "LaunchAgent plist file does not exist. Please reinstall schedule.".to_string(),
            );
        }

        let uid = unsafe { libc::getuid() };
        let domain_target = format!("gui/{}", uid);

        let _ = Command::new("launchctl")
            .args(["bootstrap", &domain_target, plist_path.to_str().unwrap()])
            .output();

        let _ = Command::new("launchctl")
            .args(["load", "-w", plist_path.to_str().unwrap()])
            .output();

        Ok(())
    }

    fn status(&self, profile_id: &str) -> Result<SchedulerStatus, String> {
        let plist_path = Self::plist_path(profile_id);
        let base_dir = get_persistent_app_dir();
        let config_path = base_dir
            .join("configs")
            .join(format!("{}.json", profile_id));
        let runner_path = base_dir.join("bin").join("resticdashboard-runner");

        let installed = plist_path.is_file();
        let mut last_run = None;

        let status_file = base_dir.join("status").join(format!("{}.json", profile_id));
        if status_file.is_file()
            && let Ok(content) = fs::read_to_string(&status_file)
            && let Ok(run) = serde_json::from_str::<RunResult>(&content)
        {
            last_run = Some(run);
        }

        let notes = if installed {
            "launchd user agent is installed and configured to run using persistent local binaries without USB.".to_string()
        } else {
            "Automatic local backups are not installed on this machine.".to_string()
        };

        Ok(SchedulerStatus {
            platform: "macos".to_string(),
            installed,
            enabled: installed,
            target_binary_path: runner_path.to_string_lossy().to_string(),
            config_path: config_path.to_string_lossy().to_string(),
            schedule_file_path: plist_path.to_string_lossy().to_string(),
            last_run,
            notes,
        })
    }
}
