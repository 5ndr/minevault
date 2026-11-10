use std::fs;
use std::path::Path;
use std::process::Command;

use crate::models::{Profile, RepoConfig, RunResult, SchedulerStatus, SessionCredentials};
use crate::scheduler::{SchedulerAdapter, setup_persistent_local_files};
use crate::system::get_persistent_app_dir;

pub struct WindowsTaskSchedulerAdapter;

impl WindowsTaskSchedulerAdapter {
    pub fn new() -> Self {
        Self
    }

    fn task_name(profile_id: &str) -> String {
        format!("ResticDashboard_{}", profile_id.replace('-', "_"))
    }
}

impl Default for WindowsTaskSchedulerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulerAdapter for WindowsTaskSchedulerAdapter {
    fn platform_name(&self) -> &'static str {
        "windows"
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

        let tn = Self::task_name(&profile.id);
        let tr = format!(
            "\"{}\" runner --profile-id {} --base-dir \"{}\"",
            runner_exe.display(),
            profile.id,
            base_dir.display()
        );

        let st = format!("{:02}:{:02}", schedule.hour, schedule.minute);
        let sc = match schedule.frequency.as_str() {
            "hourly" => "HOURLY",
            "weekly" => "WEEKLY",
            _ => "DAILY",
        };

        // Run schtasks.exe /create
        let output = Command::new("schtasks.exe")
            .args([
                "/create", "/tn", &tn, "/tr", &tr, "/sc", sc, "/st", &st, "/f",
            ])
            .output()
            .map_err(|e| format!("Failed to execute schtasks.exe: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("schtasks failed: {}", stderr.trim()));
        }

        let notes = "Windows Task Scheduler task registered. Unattended backups run in your user account without requiring administrator elevation. If the computer was asleep or shut down, missed tasks are triggered when available.".to_string();

        Ok(SchedulerStatus {
            platform: "windows".to_string(),
            installed: true,
            enabled: true,
            target_binary_path: runner_exe.to_string_lossy().to_string(),
            config_path: config_path.to_string_lossy().to_string(),
            schedule_file_path: format!("TaskScheduler:{}", tn),
            last_run: None,
            notes,
        })
    }

    fn uninstall(&self, profile_id: &str) -> Result<(), String> {
        let tn = Self::task_name(profile_id);
        let _ = Command::new("schtasks.exe")
            .args(["/delete", "/tn", &tn, "/f"])
            .output();

        // Clean up persistent configs/secrets
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
        let tn = Self::task_name(profile_id);
        let _ = Command::new("schtasks.exe")
            .args(["/change", "/tn", &tn, "/disable"])
            .output();
        Ok(())
    }

    fn enable(&self, profile_id: &str) -> Result<(), String> {
        let tn = Self::task_name(profile_id);
        let _ = Command::new("schtasks.exe")
            .args(["/change", "/tn", &tn, "/enable"])
            .output();
        Ok(())
    }

    fn status(&self, profile_id: &str) -> Result<SchedulerStatus, String> {
        let tn = Self::task_name(profile_id);
        let output = Command::new("schtasks.exe")
            .args(["/query", "/tn", &tn, "/fo", "CSV"])
            .output();

        let installed = match output {
            Ok(ref out) => out.status.success(),
            Err(_) => false,
        };

        let base_dir = get_persistent_app_dir();
        let config_path = base_dir
            .join("configs")
            .join(format!("{}.json", profile_id));
        let runner_path = base_dir.join("bin").join("resticdashboard-runner.exe");

        let mut last_run = None;
        let status_file = base_dir.join("status").join(format!("{}.json", profile_id));
        if status_file.is_file()
            && let Ok(content) = fs::read_to_string(&status_file)
            && let Ok(run) = serde_json::from_str::<RunResult>(&content)
        {
            last_run = Some(run);
        }

        let notes = if installed {
            "Windows Task Scheduler task is active with local persistent binaries.".to_string()
        } else {
            "Automatic local backups are not installed on this machine.".to_string()
        };

        Ok(SchedulerStatus {
            platform: "windows".to_string(),
            installed,
            enabled: installed,
            target_binary_path: runner_path.to_string_lossy().to_string(),
            config_path: config_path.to_string_lossy().to_string(),
            schedule_file_path: format!("TaskScheduler:{}", tn),
            last_run,
            notes,
        })
    }
}
