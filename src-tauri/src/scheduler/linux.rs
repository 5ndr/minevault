use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::{Profile, RepoConfig, RunResult, SchedulerStatus, SessionCredentials};
use crate::scheduler::{SchedulerAdapter, setup_persistent_local_files};
use crate::system::get_persistent_app_dir;

pub struct LinuxSystemdAdapter;

impl LinuxSystemdAdapter {
    pub fn new() -> Self {
        Self
    }

    fn systemd_user_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("systemd/user")
    }

    fn service_name(profile_id: &str) -> String {
        format!("resticdashboard-{}.service", profile_id)
    }

    fn timer_name(profile_id: &str) -> String {
        format!("resticdashboard-{}.timer", profile_id)
    }

    pub fn format_on_calendar(schedule: &crate::models::ScheduleSpec) -> String {
        match schedule.frequency.as_str() {
            "hourly" => "hourly".to_string(),
            "weekly" => {
                let day_name = match schedule.day_of_week.unwrap_or(1) {
                    0 | 7 => "Sun",
                    1 => "Mon",
                    2 => "Tue",
                    3 => "Wed",
                    4 => "Thu",
                    5 => "Fri",
                    6 => "Sat",
                    _ => "Mon",
                };
                format!(
                    "{} *-*-* {:02}:{:02}:00",
                    day_name, schedule.hour, schedule.minute
                )
            }
            _ => {
                // Default daily
                format!("*-*-* {:02}:{:02}:00", schedule.hour, schedule.minute)
            }
        }
    }

    pub fn generate_service_unit(runner_exe: &Path, profile_id: &str, base_dir: &Path) -> String {
        format!(
            r#"[Unit]
Description=ResticDashboard Persistent Runner for {}
After=network-online.target

[Service]
Type=oneshot
ExecStart={} runner --profile-id {} --base-dir {}
StandardOutput=journal
StandardError=journal
"#,
            profile_id,
            runner_exe.display(),
            profile_id,
            base_dir.display()
        )
    }

    pub fn generate_timer_unit(profile_id: &str, on_calendar: &str) -> String {
        format!(
            r#"[Unit]
Description=ResticDashboard Scheduled Backup Timer for {}

[Timer]
OnCalendar={}
Persistent=true

[Install]
WantedBy=timers.target
"#,
            profile_id, on_calendar
        )
    }
}

impl Default for LinuxSystemdAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulerAdapter for LinuxSystemdAdapter {
    fn platform_name(&self) -> &'static str {
        "linux"
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

        // 1. Setup persistent files
        let (runner_exe, config_path, _secret_path) =
            setup_persistent_local_files(profile, repo, creds, restic_bin)?;

        // 2. Write service & timer units
        let user_dir = Self::systemd_user_dir();
        fs::create_dir_all(&user_dir).map_err(|e| {
            format!(
                "Failed to create systemd user dir {}: {}",
                user_dir.display(),
                e
            )
        })?;

        let service_file = user_dir.join(Self::service_name(&profile.id));
        let timer_file = user_dir.join(Self::timer_name(&profile.id));

        let service_content = Self::generate_service_unit(&runner_exe, &profile.id, &base_dir);
        let on_calendar = Self::format_on_calendar(schedule);
        let timer_content = Self::generate_timer_unit(&profile.id, &on_calendar);

        fs::write(&service_file, service_content)
            .map_err(|e| format!("Failed to write {}: {}", service_file.display(), e))?;
        fs::write(&timer_file, timer_content)
            .map_err(|e| format!("Failed to write {}: {}", timer_file.display(), e))?;

        // 3. systemctl --user daemon-reload && systemctl --user enable --now <timer>
        let reload = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();

        if let Err(e) = reload {
            return Err(format!(
                "Failed to run systemctl --user: {}. Please ensure systemd user instance is active.",
                e
            ));
        }

        let _ = Command::new("systemctl")
            .args(["--user", "enable", "--now", &Self::timer_name(&profile.id)])
            .output();

        let notes = "Linux systemd user timer registered with Persistent=true. Missed backups (e.g. machine powered off) will trigger promptly upon login. Note: to run while logged out, ensure 'loginctl enable-linger $USER' is configured.".to_string();

        Ok(SchedulerStatus {
            platform: "linux".to_string(),
            installed: true,
            enabled: true,
            target_binary_path: runner_exe.to_string_lossy().to_string(),
            config_path: config_path.to_string_lossy().to_string(),
            schedule_file_path: timer_file.to_string_lossy().to_string(),
            last_run: None,
            notes,
        })
    }

    fn uninstall(&self, profile_id: &str) -> Result<(), String> {
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "--now", &Self::timer_name(profile_id)])
            .output();

        let user_dir = Self::systemd_user_dir();
        let service_file = user_dir.join(Self::service_name(profile_id));
        let timer_file = user_dir.join(Self::timer_name(profile_id));

        let _ = fs::remove_file(&service_file);
        let _ = fs::remove_file(&timer_file);

        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();

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
        let _ = Command::new("systemctl")
            .args(["--user", "stop", &Self::timer_name(profile_id)])
            .output();
        let _ = Command::new("systemctl")
            .args(["--user", "disable", &Self::timer_name(profile_id)])
            .output();
        Ok(())
    }

    fn enable(&self, profile_id: &str) -> Result<(), String> {
        let _ = Command::new("systemctl")
            .args(["--user", "enable", "--now", &Self::timer_name(profile_id)])
            .output();
        Ok(())
    }

    fn status(&self, profile_id: &str) -> Result<SchedulerStatus, String> {
        let user_dir = Self::systemd_user_dir();
        let timer_file = user_dir.join(Self::timer_name(profile_id));
        let installed = timer_file.is_file();

        let base_dir = get_persistent_app_dir();
        let config_path = base_dir
            .join("configs")
            .join(format!("{}.json", profile_id));
        let runner_path = base_dir.join("bin").join("resticdashboard-runner");

        let mut last_run = None;
        let status_file = base_dir.join("status").join(format!("{}.json", profile_id));
        if status_file.is_file()
            && let Ok(content) = fs::read_to_string(&status_file)
            && let Ok(run) = serde_json::from_str::<RunResult>(&content)
        {
            last_run = Some(run);
        }

        let notes = if installed {
            "systemd user timer is active with Persistent=true.".to_string()
        } else {
            "Automatic local backups are not installed on this machine.".to_string()
        };

        Ok(SchedulerStatus {
            platform: "linux".to_string(),
            installed,
            enabled: installed,
            target_binary_path: runner_path.to_string_lossy().to_string(),
            config_path: config_path.to_string_lossy().to_string(),
            schedule_file_path: timer_file.to_string_lossy().to_string(),
            last_run,
            notes,
        })
    }
}
